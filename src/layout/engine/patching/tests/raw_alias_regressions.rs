use super::{EngineFingerprint, LayoutEngine, width_props};
use crate::core::{Dimension, Props, VNode};
use crate::layout::{
    DirectPatchError, DirectPatchPreflightCause, IncrementalPatchKind, TransactionalLayoutError,
};
use crate::reconciler::Patch;

fn expect_preflight(error: TransactionalLayoutError) -> crate::layout::DirectPatchPreflightError {
    match error {
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error)) => error,
        other => panic!("expected preflight rejection, got {other:?}"),
    }
}

#[test]
fn positional_remove_then_current_no_op_reorder_is_applied() {
    let initial = VNode::root().children([
        VNode::box_node().with_props(width_props(1.0)),
        VNode::box_node().with_props(width_props(2.0)),
        VNode::box_node().with_props(width_props(3.0)),
    ]);
    let target = VNode::root().children([
        VNode::box_node().with_props(width_props(2.0)),
        VNode::box_node().with_props(width_props(3.0)),
    ]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&initial, 20, 4);

    engine
        .try_apply_patches_transactional(&[
            Patch::remove(initial.children[0].key),
            Patch::reorder(
                initial.key,
                target.children.iter().map(|child| child.key).collect(),
            ),
        ])
        .expect("the post-remove current order is already exact");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
    assert_eq!(
        target
            .children
            .iter()
            .map(|child| engine
                .get_vnode_layout(child.key)
                .map(|layout| layout.width))
            .collect::<Vec<_>>(),
        vec![Some(2.0), Some(3.0)]
    );
}

#[test]
fn positional_batch_alias_precedes_normalized_current_target_and_parent() {
    let mut existing_props = width_props(1.0);
    existing_props.key = Some("existing".into());
    let existing = VNode::box_node().with_props(existing_props);
    let initial = VNode::root().child(existing);
    let created_parent = VNode::box_node().with_index(1);
    let lower_sibling = VNode::box_node().with_index(0).with_props(width_props(3.0));
    let mut updated_props = created_parent.props.clone();
    updated_props.style.width = Dimension::Points(9.0);
    let child = VNode::text("child").with_key("child");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&initial, 20, 4);

    engine
        .try_apply_patches_transactional(&[
            Patch::create(created_parent.clone(), initial.key),
            Patch::create(lower_sibling, initial.key),
            Patch::update(
                created_parent.key,
                created_parent.props.clone(),
                updated_props,
            ),
            Patch::create(child, created_parent.key),
        ])
        .expect("the batch-created positional generation owns its raw alias");

    let committed = engine.committed_vnode.as_ref().expect("committed target");
    assert_eq!(committed.children.len(), 3);
    assert_eq!(
        committed.children[1].props.style.width,
        Dimension::Points(1.0)
    );
    assert_eq!(
        committed.children[2].props.style.width,
        Dimension::Points(9.0)
    );
    assert_eq!(committed.children[2].children.len(), 1);
}

#[test]
fn batch_local_target_alias_does_not_hide_an_existing_scope() {
    let existing = VNode::box_node()
        .with_key("left")
        .child(VNode::text("left").with_key("dup"));
    let tree = VNode::root().child(existing);
    let created = VNode::box_node()
        .with_key("right")
        .child(VNode::text("right").with_key("dup"));
    let raw_duplicate = VNode::text("raw").with_key("dup").key;
    let mut new_props = Props::new();
    new_props.style.width = Dimension::Points(5.0);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = expect_preflight(
        engine
            .try_apply_patches_transactional(&[
                Patch::create(created, tree.key),
                Patch::update(raw_duplicate, Props::new(), new_props),
            ])
            .expect_err("two current scopes make the raw target ambiguous"),
    );

    assert_eq!(error.patch_index, 1);
    assert_eq!(error.kind, IncrementalPatchKind::Update);
    assert!(matches!(
        *error.source,
        DirectPatchPreflightCause::AmbiguousTarget { match_count: 2 }
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

fn parent_alias_fixture() -> (VNode, VNode, crate::core::NodeKey) {
    let duplicate = VNode::box_node().with_key("dup-parent");
    let left = VNode::box_node().with_key("left").child(duplicate);
    let right = VNode::box_node().with_key("right");
    let right_key = right.key;
    (
        VNode::root().children([left, right]),
        VNode::box_node().with_key("dup-parent"),
        right_key,
    )
}

#[test]
fn batch_local_create_parent_alias_does_not_hide_an_existing_scope() {
    let (tree, duplicate_parent, right_key) = parent_alias_fixture();
    let raw_parent = duplicate_parent.key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = expect_preflight(
        engine
            .try_apply_patches_transactional(&[
                Patch::create(duplicate_parent, right_key),
                Patch::create(VNode::box_node().with_key("child"), raw_parent),
            ])
            .expect_err("two current scopes make the raw parent ambiguous"),
    );

    assert_eq!(error.patch_index, 1);
    assert_eq!(error.kind, IncrementalPatchKind::Create);
    assert!(matches!(
        *error.source,
        DirectPatchPreflightCause::AmbiguousParent { match_count: 2 }
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn batch_local_reorder_parent_alias_does_not_hide_an_existing_scope() {
    let (tree, duplicate_parent, right_key) = parent_alias_fixture();
    let raw_parent = duplicate_parent.key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = expect_preflight(
        engine
            .try_apply_patches_transactional(&[
                Patch::create(duplicate_parent, right_key),
                Patch::reorder(raw_parent, Vec::new()),
            ])
            .expect_err("two current scopes make the reorder parent ambiguous"),
    );

    assert_eq!(error.patch_index, 1);
    assert_eq!(error.kind, IncrementalPatchKind::Reorder);
    assert!(matches!(
        *error.source,
        DirectPatchPreflightCause::AmbiguousParent { match_count: 2 }
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn positional_batch_alias_does_not_hide_an_existing_scope() {
    let left = VNode::box_node().with_key("left").child(VNode::box_node());
    let right = VNode::box_node().with_key("right");
    let right_key = right.key;
    let tree = VNode::root().children([left, right]);
    let created = VNode::box_node();
    let raw_positional = created.key;
    let mut updated_props = Props::new();
    updated_props.style.width = Dimension::Points(5.0);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = expect_preflight(
        engine
            .try_apply_patches_transactional(&[
                Patch::create(created, right_key),
                Patch::update(raw_positional, Props::new(), updated_props),
            ])
            .expect_err("positional matches in two scopes are ambiguous"),
    );

    assert_eq!(error.patch_index, 1);
    assert_eq!(error.kind, IncrementalPatchKind::Update);
    assert!(matches!(
        *error.source,
        DirectPatchPreflightCause::AmbiguousTarget { match_count: 2 }
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}
