//! GH-59: keyed children keep their identity, and Taffy order follows VNode order.
//!
//! Incremental layout used to reach a different tree than a full rebuild of the
//! same final UI: a keyed child that moved was matched under one identity rule
//! and looked up under another, so it was destroyed and recreated, and the
//! resulting Taffy child order depended on a heuristic rather than on a stated
//! target order.
//!
//! These drive only the public `Element` / `LayoutEngine` surface.

use rnk::core::VNode;
use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::reconciler::{Patch, diff};

/// A row of keyed children, each a fixed height so order is readable from `y`.
fn row(keys: &[&str]) -> Element {
    let mut root = Box::new().width(40.0).flex_direction(FlexDirection::Column);
    for key in keys {
        root = root.child(
            Box::new()
                .key(*key)
                .height(1.0)
                .child(Text::new((*key).to_string())),
        );
    }
    root.into_element()
}

/// Vertical positions of each child after layout, in child order.
fn child_offsets(engine: &LayoutEngine, root: &Element) -> Vec<(String, i32)> {
    root.children
        .iter()
        .map(|child| {
            let y = engine
                .get_layout(child.id)
                .map(|layout| layout.y as i32)
                .unwrap_or(-1);
            (child.key.clone().unwrap_or_default(), y)
        })
        .collect()
}

/// Lay `before` out, then reach `after` incrementally.
fn incremental(before: &[&str], after: &[&str]) -> Vec<(String, i32)> {
    let mut engine = LayoutEngine::new();
    let first = row(before);
    let (vnode, _) = engine.compute_element_incremental(&first, None, 40, 20);

    let second = row(after);
    engine.compute_element_incremental(&second, Some(&vnode), 40, 20);
    child_offsets(&engine, &second)
}

/// Lay `after` out from scratch.
fn full_rebuild(after: &[&str]) -> Vec<(String, i32)> {
    let mut engine = LayoutEngine::new();
    let root = row(after);
    engine.compute(&root, 40, 20);
    child_offsets(&engine, &root)
}

/// Every edit shape the acceptance criteria name.
const EDITS: &[(&[&str], &[&str])] = &[
    (&["b", "c"], &["a", "b", "c"]),
    (&["a", "c"], &["a", "b", "c"]),
    (&["a", "b"], &["a", "b", "c"]),
    (&["a", "b", "c"], &["a", "c"]),
    (&["a", "b"], &["b", "a"]),
    (&["a", "b", "c"], &["c", "b", "a"]),
    (&["a", "b", "c", "d"], &["d", "b", "c", "a"]),
    (&["a", "b", "c"], &["b", "c", "a"]),
];

#[test]
fn incremental_and_full_rebuild_agree_on_every_edit() {
    for (before, after) in EDITS {
        assert_eq!(
            incremental(before, after),
            full_rebuild(after),
            "{before:?} -> {after:?}"
        );
    }
}

#[test]
fn children_land_in_the_order_the_tree_declares() {
    for (before, after) in EDITS {
        let offsets = incremental(before, after);
        let expected: Vec<(String, i32)> = after
            .iter()
            .enumerate()
            .map(|(i, key)| ((*key).to_string(), i as i32))
            .collect();
        assert_eq!(offsets, expected, "{before:?} -> {after:?}");
    }
}

#[test]
fn consecutive_frames_match_full_rebuild() {
    // One frame's result is the next frame's input, so a stale mapping would
    // compound rather than wash out.
    let frames = [
        vec!["a", "b", "c", "d"],
        vec!["d", "a", "b", "c"],
        vec!["b", "d", "c", "a"],
        vec!["a", "b", "c", "d"],
        vec!["c", "a", "d"],
        vec!["e", "c", "a", "d"],
    ];

    let mut engine = LayoutEngine::new();
    let first = row(&frames[0]);
    let (mut vnode, _) = engine.compute_element_incremental(&first, None, 40, 20);

    for frame in &frames[1..] {
        let root = row(frame);
        let (next, _) = engine.compute_element_incremental(&root, Some(&vnode), 40, 20);
        vnode = next;

        assert_eq!(
            child_offsets(&engine, &root),
            full_rebuild(frame),
            "frame {frame:?}"
        );
    }
}

#[test]
fn a_keyed_subtree_survives_its_parent_moving() {
    // A descendant's identity used to be built from its ancestors' positions,
    // so reordering a keyed parent silently renamed everything beneath it.
    fn tree(order: &[&str]) -> Element {
        let mut root = Box::new().width(40.0).flex_direction(FlexDirection::Column);
        for key in order {
            root = root.child(
                Box::new()
                    .key(*key)
                    .height(2.0)
                    .child(Box::new().key("inner").child(Text::new("leaf"))),
            );
        }
        root.into_element()
    }

    let mut engine = LayoutEngine::new();
    let first = tree(&["a", "b"]);
    let (vnode, _) = engine.compute_element_incremental(&first, None, 40, 20);

    let second = tree(&["b", "a"]);
    let (_, outcome) = engine.compute_element_incremental(&second, Some(&vnode), 40, 20);

    assert!(
        !outcome.fallback_full_rebuild,
        "swapping keyed parents should stay on the incremental path"
    );
    // Swapping two keyed siblings is one order change. Rebuilding their
    // subtrees as well means the descendants were renamed by the move, which
    // is exactly the ancestor-path defect: the layout still came out right,
    // so only the patch count shows it.
    assert_eq!(
        outcome.patch_count, 1,
        "moving a keyed parent must not rebuild what is under it"
    );

    let mut rebuilt = LayoutEngine::new();
    let expected = tree(&["b", "a"]);
    rebuilt.compute(&expected, 40, 20);

    for (moved, fresh) in second.children.iter().zip(expected.children.iter()) {
        let leaf = moved.children.iter().next().expect("one child");
        let fresh_leaf = fresh.children.iter().next().expect("one child");
        assert_eq!(
            engine
                .get_layout(leaf.id)
                .map(|l| (l.y as i32, l.height as i32)),
            rebuilt
                .get_layout(fresh_leaf.id)
                .map(|l| (l.y as i32, l.height as i32)),
            "nested subtree under key {:?}",
            moved.key
        );
    }
}

#[test]
fn mixed_keyed_unkeyed_keeps_public_behavior() {
    fn mixed(keys: &[Option<&str>]) -> Element {
        let mut root = Box::new().width(40.0).flex_direction(FlexDirection::Column);
        for key in keys {
            let child = Box::new().height(1.0);
            root = root.child(match key {
                Some(k) => child.key(*k).child(Text::new((*k).to_string())),
                None => child.child(Text::new("-")),
            });
        }
        root.into_element()
    }

    let mut engine = LayoutEngine::new();
    let before = mixed(&[Some("a"), None, Some("b")]);
    let (vnode, _) = engine.compute_element_incremental(&before, None, 40, 20);

    let after = mixed(&[Some("b"), None, Some("a")]);
    engine.compute_element_incremental(&after, Some(&vnode), 40, 20);

    let mut rebuilt = LayoutEngine::new();
    let expected = mixed(&[Some("b"), None, Some("a")]);
    rebuilt.compute(&expected, 40, 20);

    assert_eq!(
        child_offsets(&engine, &after),
        child_offsets(&rebuilt, &expected)
    );
}

#[test]
fn same_user_key_with_a_different_type_is_one_replace() {
    let old = VNode::box_node().child(VNode::box_node().with_key("stable"));
    let new = VNode::box_node().child(VNode::text("replacement").with_key("stable"));

    let patches = diff(&old, &new);

    assert_eq!(
        patches
            .iter()
            .filter(|patch| matches!(patch, Patch::Replace { .. }))
            .count(),
        1,
        "{patches:?}"
    );
    assert!(
        !patches
            .iter()
            .any(|patch| matches!(patch, Patch::Create { .. } | Patch::Remove { .. })),
        "same-key type changes must be represented as replace: {patches:?}"
    );
}

#[test]
fn delimiter_payloads_do_not_alias_distinct_scopes() {
    fn keyed_leaf(key: &str, width: f32) -> Element {
        Box::new().key(key).width(width).height(1.0).into_element()
    }

    let slash_leaf = keyed_leaf("leaf", 3.0);
    let slash_leaf_id = slash_leaf.id;
    let slash_branch = Box::new().key("a/key:b").child(slash_leaf);

    let nested_leaf = keyed_leaf("leaf", 7.0);
    let nested_leaf_id = nested_leaf.id;
    let nested_branch = Box::new()
        .key("a")
        .child(Box::new().key("b").child(nested_leaf));

    let hash_leaf = keyed_leaf("b#key:c", 5.0);
    let hash_leaf_id = hash_leaf.id;
    let hash_branch = Box::new().key("hash-a").child(hash_leaf);

    let other_hash_leaf = keyed_leaf("c", 9.0);
    let other_hash_leaf_id = other_hash_leaf.id;
    let other_hash_branch = Box::new().key("hash-a#key:b").child(other_hash_leaf);

    let root = Box::new()
        .child(slash_branch)
        .child(nested_branch)
        .child(hash_branch)
        .child(other_hash_branch)
        .into_element();
    let mut engine = LayoutEngine::new();
    engine.compute_element_incremental(&root, None, 80, 20);

    let widths = [
        engine.get_layout(slash_leaf_id).map(|layout| layout.width),
        engine.get_layout(nested_leaf_id).map(|layout| layout.width),
        engine.get_layout(hash_leaf_id).map(|layout| layout.width),
        engine
            .get_layout(other_hash_leaf_id)
            .map(|layout| layout.width),
    ];
    assert_eq!(widths, [Some(3.0), Some(7.0), Some(5.0), Some(9.0)]);
}

#[test]
fn keyed_reorder_must_not_create_or_remove_survivors() {
    let old = VNode::box_node().children([
        VNode::text("a").with_key("a"),
        VNode::text("b").with_key("b"),
        VNode::text("c").with_key("c"),
    ]);
    let new = VNode::box_node().children([
        VNode::text("c").with_key("c"),
        VNode::text("a").with_key("a"),
        VNode::text("b").with_key("b"),
    ]);
    let patches = diff(&old, &new);

    assert!(
        patches
            .iter()
            .all(|patch| !matches!(patch, Patch::Create { .. } | Patch::Remove { .. })),
        "{patches:?}"
    );
    assert!(
        patches
            .iter()
            .any(|patch| matches!(patch, Patch::Reorder { .. }))
    );
}

#[test]
fn public_node_key_and_patch_surface_compiles() {
    let root = VNode::box_node();
    let child = VNode::text("child").with_key("child");
    let key: rnk::core::NodeKey = child.key;
    let patches = [
        Patch::create(child.clone(), root.key),
        Patch::update(key, child.props.clone(), child.props.clone()),
        Patch::remove(key),
        Patch::replace(key, child.clone()),
        Patch::reorder(root.key, vec![key]),
    ];
    assert_eq!(patches.len(), 5);
}

#[test]
fn same_key_in_distinct_parents_has_layouts() {
    let left_leaf = Box::new()
        .key("shared")
        .width(3.0)
        .height(1.0)
        .into_element();
    let left_id = left_leaf.id;
    let right_leaf = Box::new()
        .key("shared")
        .width(7.0)
        .height(1.0)
        .into_element();
    let right_id = right_leaf.id;
    let root = Box::new()
        .child(Box::new().key("left").child(left_leaf))
        .child(Box::new().key("right").child(right_leaf))
        .into_element();
    let mut engine = LayoutEngine::new();
    engine.compute_element_incremental(&root, None, 20, 4);

    assert_eq!(
        (
            engine.get_layout(left_id).expect("left layout").width,
            engine.get_layout(right_id).expect("right layout").width,
        ),
        (3.0, 7.0)
    );
    assert_eq!(engine.get_all_vnode_layouts().len(), 5);
}
