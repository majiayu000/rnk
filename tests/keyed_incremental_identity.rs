//! GH-59: keyed children keep their identity, and Taffy order follows VNode order.
//!
//! Incremental layout used to reach a different tree than a full rebuild of the
//! same final UI: a keyed child that moved was matched under one identity rule
//! and looked up under another, so it was destroyed and recreated, and the
//! resulting Taffy child order depended on a heuristic rather than on a stated
//! target order.
//!
//! These drive only the public `Element` / `LayoutEngine` surface.

use rnk::layout::LayoutEngine;
use rnk::prelude::*;

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
fn consecutive_reorders_stay_correct() {
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
fn mixed_keyed_and_unkeyed_children_reorder_correctly() {
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
