//! GH-59 checked identity and direct-patch boundary regressions.

use std::panic::{AssertUnwindSafe, catch_unwind};

use rnk::core::{Dimension, Props, Style, VNode};
use rnk::layout::{IncrementalLayoutError, LayoutEngine, PatchFailure};
use rnk::prelude::*;
use rnk::reconciler::{Patch, ReconcilePlanError, try_diff, try_diff_children};

fn duplicate_boxes() -> Element {
    Box::new()
        .child(Box::new().key("duplicate").width(2.0))
        .child(Box::new().key("duplicate").width(4.0))
        .into_element()
}

#[test]
fn duplicate_key_reaches_checked_layout_boundary() {
    let mut engine = LayoutEngine::new();
    let failure = engine
        .try_compute_element_incremental_checked(&duplicate_boxes(), None, 20, 4)
        .expect_err("duplicate sibling keys must be a checked error");

    assert!(matches!(
        failure,
        IncrementalLayoutError::Identity(ReconcilePlanError::DuplicateSiblingKey {
            first_index: 0,
            second_index: 1,
            ..
        })
    ));
    assert!(!engine.has_tree());
}

#[test]
fn dynamic_frame_error_is_publicly_exported() {
    fn accepts_public_error(_: Option<rnk::renderer::DynamicFrameError>) {}
    accepts_public_error(None);
}

fn duplicate_subtree() -> VNode {
    VNode::box_node().with_key("new").children([
        VNode::text("first").with_key("duplicate"),
        VNode::text("second").with_key("duplicate"),
    ])
}

fn duplicate_subtree_patch(parent: rnk::core::NodeKey) -> Patch {
    Patch::create(duplicate_subtree(), parent)
}

#[test]
fn direct_create_duplicate_subtree_is_an_identity_rejection() {
    let root = VNode::box_node();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    let failure = engine
        .try_apply_patches(&[duplicate_subtree_patch(root.key)])
        .expect_err("a direct patch must preflight duplicate subtree identity");

    assert_eq!(failure.failure, PatchFailure::IdentityRejected);
}

#[test]
fn direct_replace_duplicate_subtree_is_an_identity_rejection() {
    let root = VNode::box_node().child(VNode::box_node().with_key("old"));
    let old_key = root.children[0].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before_count = engine.node_count();

    let failure = engine
        .try_apply_patches(&[Patch::replace(old_key, duplicate_subtree())])
        .expect_err("a replacement subtree must pass identity preflight");

    assert_eq!(failure.failure, PatchFailure::IdentityRejected);
    assert_eq!(engine.node_count(), before_count);
    assert!(engine.get_vnode_layout(old_key).is_some());
}

#[test]
fn duplicate_create_roots_are_rejected_by_batch_preflight() {
    let root = VNode::box_node();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before_count = engine.node_count();
    let patches = [
        Patch::create(VNode::box_node().with_key("duplicate"), root.key),
        Patch::create(VNode::text("duplicate").with_key("duplicate"), root.key),
    ];

    let failure = engine
        .try_apply_patches(&patches)
        .expect_err("the whole batch must be identity-valid before mutation");

    assert_eq!(failure.failure, PatchFailure::IdentityRejected);
    assert_eq!(engine.node_count(), before_count);
}

#[test]
fn legacy_apply_patches_fails_loudly_on_identity_error() {
    let root = VNode::box_node();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.apply_patches(&[duplicate_subtree_patch(root.key)])
    }));

    assert!(
        result.is_err(),
        "legacy bool API must not silently turn an identity error into false"
    );
}

#[test]
fn legacy_apply_patches_fails_loudly_on_unknown_identity() {
    let root = VNode::box_node();
    let missing = VNode::box_node().with_key("missing").key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.apply_patches(&[Patch::remove(missing)])
    }));

    assert!(
        result.is_err(),
        "legacy bool API must not silently turn an unknown identity into false"
    );
}

fn styled_branch(parent_key: &str, width: f32) -> VNode {
    let mut style = Style::new();
    style.width = Dimension::Points(width);
    VNode::box_node().with_key(parent_key).child(
        VNode::box_node()
            .with_key("shared")
            .with_props(Props::with_style(style)),
    )
}

#[test]
fn checked_diff_apply_handles_same_raw_key_in_distinct_parents() {
    let old = VNode::box_node().children([styled_branch("left", 2.0), styled_branch("right", 4.0)]);
    let new = VNode::box_node().children([styled_branch("left", 3.0), styled_branch("right", 5.0)]);
    let patches = try_diff(&old, &new).expect("parent-scoped diff is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);

    assert!(
        engine
            .try_apply_patches(&patches)
            .expect("checked diff output must be directly applicable")
    );
    let widths: Vec<_> = engine
        .get_all_vnode_layouts()
        .values()
        .map(|layout| layout.width)
        .collect();
    assert!(widths.contains(&3.0));
    assert!(widths.contains(&5.0));
    assert!(!widths.contains(&2.0));
    assert!(!widths.contains(&4.0));
}

#[test]
fn child_only_diff_reports_global_raw_ambiguity_without_mutation() {
    let old = VNode::box_node().children([styled_branch("left", 2.0), styled_branch("right", 4.0)]);
    let new_left = styled_branch("left", 3.0);
    let patches = try_diff_children(
        &old.children[0].children,
        &new_left.children,
        old.children[0].key,
    )
    .expect("the sibling-local child diff itself is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let mut before_widths: Vec<_> = engine
        .get_all_vnode_layouts()
        .values()
        .map(|layout| layout.width as i32)
        .collect();
    before_widths.sort_unstable();

    let failure = engine
        .try_apply_patches(&patches)
        .expect_err("a partial raw diff must not guess among global scopes");
    let mut after_widths: Vec<_> = engine
        .get_all_vnode_layouts()
        .values()
        .map(|layout| layout.width as i32)
        .collect();
    after_widths.sort_unstable();

    assert_eq!(failure.failure, PatchFailure::AmbiguousNode);
    assert_eq!(after_widths, before_widths);
}

#[test]
fn stale_public_indices_do_not_reject_the_committed_previous_tree() {
    let root = Box::new()
        .child(Box::new().key("stable").width(3.0))
        .into_element();
    let mut engine = LayoutEngine::new();
    let (mut previous, _) = engine.compute_element_incremental(&root, None, 20, 4);
    previous.children[0].key.index = 99;

    engine
        .try_compute_element_incremental_checked(&root, Some(&previous), 20, 4)
        .expect("actual vector position, not stale public index, is index truth");
}
