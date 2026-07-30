//! GH-60: a patch batch applies completely or not at all.
//!
//! A batch used to be accepted whenever any single patch in it succeeded. A
//! patch naming a node that no longer existed was skipped in silence while its
//! siblings applied, leaving the tree describing neither the old VNode nor the
//! new one — and `apply_patches` reported success.

use rnk::core::{Dimension, NodeKey, Props, VNode};
use rnk::layout::{LayoutEngine, PatchFailure, PatchKind};
use rnk::prelude::*;
use rnk::reconciler::Patch;

/// A root with three keyed text children.
fn tree() -> VNode {
    VNode::box_node().children([
        VNode::text("a").with_key("a"),
        VNode::text("b").with_key("b"),
        VNode::text("c").with_key("c"),
    ])
}

/// A key that is not in the tree.
fn absent_key() -> NodeKey {
    VNode::text("gone").with_key("gone").key
}

/// Widths of the root's children, in order.
fn child_widths(engine: &LayoutEngine, root: &VNode) -> Vec<Option<i32>> {
    root.children
        .iter()
        .map(|child| {
            engine
                .get_vnode_layout(child.key)
                .map(|layout| layout.width as i32)
        })
        .collect()
}

fn wide_props() -> Props {
    let mut props = Props::new();
    props.style.min_width = Dimension::Points(11.0);
    props
}

#[test]
fn a_batch_with_one_bad_patch_changes_nothing() {
    let root = tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 40, 10);
    let before = child_widths(&engine, &root);

    let first = root.children[0].key;
    let error = engine
        .try_apply_patches(&[
            // This one would succeed on its own.
            Patch::update(first, Props::new(), wide_props()),
            // This one cannot.
            Patch::reorder(root.key, vec![absent_key()]),
        ])
        .expect_err("a batch naming a missing node must be rejected");

    assert_eq!(error.kind, PatchKind::Reorder);
    assert_eq!(error.failure, PatchFailure::UnknownNode);
    assert_eq!(
        child_widths(&engine, &root),
        before,
        "the successful patch in the batch must not have been kept"
    );
}

#[test]
fn every_patch_kind_reports_its_own_rejection() {
    let root = tree();
    let missing = absent_key();

    let cases: Vec<(Patch, PatchKind)> = vec![
        (
            Patch::create(VNode::text("new").with_key("new"), missing),
            PatchKind::Create,
        ),
        (
            Patch::update(missing, Props::new(), wide_props()),
            PatchKind::Update,
        ),
        (Patch::remove(missing), PatchKind::Remove),
        (
            Patch::replace(missing, VNode::text("x").with_key("x")),
            PatchKind::Replace,
        ),
        (
            Patch::reorder(missing, vec![root.children[0].key]),
            PatchKind::Reorder,
        ),
    ];

    for (patch, expected_kind) in cases {
        let mut engine = LayoutEngine::new();
        engine.compute_vnode(&root, 40, 10);

        let error = engine
            .try_apply_patches(&[patch])
            .expect_err("a patch naming a missing node must be rejected");

        assert_eq!(error.kind, expected_kind);
        assert_eq!(error.failure, PatchFailure::UnknownNode);
        assert_eq!(
            error.key, missing,
            "the error must name the node it failed on"
        );
    }
}

#[test]
fn a_rejected_batch_is_reported_and_recovered_by_one_full_rebuild() {
    // Through the public element path: an incremental frame that cannot be
    // patched still produces the right layout, and says why.
    fn row(count: usize) -> Element {
        let mut root = Box::new().width(40.0).flex_direction(FlexDirection::Column);
        for i in 0..count {
            root = root.child(
                Box::new()
                    .key(format!("k{i}"))
                    .height(1.0)
                    .child(Text::new(format!("row {i}"))),
            );
        }
        root.into_element()
    }

    let mut engine = LayoutEngine::new();
    let first = row(3);
    let (vnode, _) = engine.compute_element_incremental(&first, None, 40, 20);

    let second = row(5);
    let (_, outcome) = engine.compute_element_incremental(&second, Some(&vnode), 40, 20);

    // A normal frame patches cleanly and reports no error.
    assert!(outcome.patch_error.is_none(), "{outcome:?}");
    assert!(!outcome.fallback_full_rebuild);

    let mut rebuilt = LayoutEngine::new();
    let expected = row(5);
    rebuilt.compute(&expected, 40, 20);

    for (patched, fresh) in second.children.iter().zip(expected.children.iter()) {
        assert_eq!(
            engine.get_layout(patched.id).map(|l| l.y as i32),
            rebuilt.get_layout(fresh.id).map(|l| l.y as i32),
        );
    }
}

#[test]
fn removing_a_subtree_leaves_no_descendant_mappings() {
    let leaf = VNode::text("leaf").with_key("leaf");
    let leaf_key = leaf.key;
    let middle = VNode::box_node().with_key("middle").child(leaf);
    let middle_key = middle.key;
    let root = VNode::box_node().children([middle, VNode::text("keep").with_key("keep")]);
    let keep_key = root.children[1].key;

    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 40, 10);
    assert!(engine.get_vnode_layout(leaf_key).is_some());

    engine
        .try_apply_patches(&[Patch::remove(middle_key)])
        .expect("removing an existing node must succeed");

    assert!(
        engine.get_vnode_layout(middle_key).is_none(),
        "the removed node is still mapped"
    );
    assert!(
        engine.get_vnode_layout(leaf_key).is_none(),
        "a descendant of the removed node is still mapped"
    );
    assert!(
        engine.get_vnode_layout(keep_key).is_some(),
        "an unrelated sibling must survive"
    );
}

#[test]
fn replacing_a_subtree_leaves_no_descendant_mappings() {
    let leaf = VNode::text("old-leaf").with_key("old-leaf");
    let leaf_key = leaf.key;
    let branch = VNode::box_node().with_key("branch").child(leaf);
    let branch_key = branch.key;
    let root = VNode::box_node().child(branch);

    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 40, 10);
    assert!(engine.get_vnode_layout(leaf_key).is_some());

    let new_leaf = VNode::text("new-leaf").with_key("new-leaf");
    let new_leaf_key = new_leaf.key;
    let replacement = VNode::box_node().with_key("branch").child(new_leaf);

    engine
        .try_apply_patches(&[Patch::replace(branch_key, replacement)])
        .expect("replacing an existing node must succeed");

    assert!(
        engine.get_vnode_layout(leaf_key).is_none(),
        "the replaced subtree's descendant is still mapped"
    );
    assert!(engine.get_vnode_layout(new_leaf_key).is_some());
}

#[test]
fn an_empty_batch_is_not_a_change() {
    let root = tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 40, 10);

    assert_eq!(engine.try_apply_patches(&[]), Ok(false));
}

#[test]
fn a_successful_batch_leaves_the_stated_child_order() {
    let root = tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 40, 10);

    let order = vec![
        root.children[2].key,
        root.children[0].key,
        root.children[1].key,
    ];
    engine
        .try_apply_patches(&[Patch::reorder(root.key, order)])
        .expect("reordering existing children must succeed");

    // Vertical position follows child order in a column layout.
    let positions: Vec<i32> = ["c", "a", "b"]
        .iter()
        .map(|name| {
            let key = root
                .children
                .iter()
                .find(|child| child.get_text() == Some(name))
                .expect("child")
                .key;
            engine.get_vnode_layout(key).expect("layout").y as i32
        })
        .collect();

    assert!(
        positions.windows(2).all(|pair| pair[0] <= pair[1]),
        "children are not in the stated order: {positions:?}"
    );
}
