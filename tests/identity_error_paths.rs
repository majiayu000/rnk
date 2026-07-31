//! GH-59 corrective regressions for checked identity failures.

use std::any::TypeId;
use std::panic::{AssertUnwindSafe, catch_unwind};

use rnk::core::{VNode, VNodeType};
use rnk::layout::{IncrementalLayoutError, LayoutEngine};
use rnk::prelude::*;
use rnk::reconciler::{ReconcilePlanError, diff, try_diff};

#[test]
fn dynamic_frame_error_is_publicly_exported() {
    fn accepts_public_error(_: Option<rnk::renderer::DynamicFrameError>) {}

    accepts_public_error(None);
}

fn duplicate_boxes() -> Element {
    Box::new()
        .child(Box::new().key("duplicate").width(2.0))
        .child(Box::new().key("duplicate").width(4.0))
        .into_element()
}

#[test]
fn duplicate_key_on_first_frame_fails_loudly() {
    let mut engine = LayoutEngine::new();
    let invalid = duplicate_boxes();

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.compute_element_incremental(&invalid, None, 20, 4)
    }));

    assert!(result.is_err(), "duplicate sibling keys must be rejected");
    assert!(!engine.has_tree(), "a rejected first frame must not commit");
}

#[test]
fn duplicate_key_reaches_the_checked_layout_boundary() {
    let mut engine = LayoutEngine::new();
    let invalid = duplicate_boxes();

    let failure = engine
        .try_compute_element_incremental_checked(&invalid, None, 20, 4)
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
fn duplicate_key_with_different_types_is_still_rejected() {
    let mut engine = LayoutEngine::new();
    let invalid = Box::new()
        .child(Box::new().key("duplicate"))
        .child(Text::new("duplicate").key("duplicate"))
        .into_element();

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.compute_element_incremental(&invalid, None, 20, 4)
    }));

    assert!(
        result.is_err(),
        "node type must not exempt a duplicate user key"
    );
    assert!(!engine.has_tree(), "a rejected first frame must not commit");
}

#[test]
fn duplicate_incremental_target_changes_no_engine_state() {
    let mut engine = LayoutEngine::new();
    let stable = Box::new()
        .child(Box::new().key("left").width(2.0))
        .child(Box::new().key("right").width(4.0))
        .into_element();
    let mut stable_children = stable.children.iter();
    let stable_ids = [
        stable_children.next().expect("left child").id,
        stable_children.next().expect("right child").id,
    ];
    let (previous, _) = engine.compute_element_incremental(&stable, None, 20, 4);
    let before_count = engine.node_count();
    let before_widths = stable_ids.map(|id| engine.get_layout(id).map(|layout| layout.width));

    let invalid = duplicate_boxes();
    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.compute_element_incremental(&invalid, Some(&previous), 20, 4)
    }));

    assert!(result.is_err(), "duplicate sibling keys must be rejected");
    assert_eq!(engine.node_count(), before_count);
    assert_eq!(
        stable_ids.map(|id| engine.get_layout(id).map(|layout| layout.width)),
        before_widths,
        "identity failure must not replace the committed maps/tree"
    );
}

#[test]
fn legacy_diff_rejects_duplicate_keys_instead_of_creating() {
    let old = VNode::box_node().child(VNode::text("old").with_key("duplicate"));
    let new = VNode::box_node()
        .child(VNode::text("first").with_key("duplicate"))
        .child(VNode::text("second").with_key("duplicate"));

    let result = catch_unwind(AssertUnwindSafe(|| diff(&old, &new)));

    assert!(
        result.is_err(),
        "legacy diff must fail loudly on an invalid target"
    );
}

#[test]
fn checked_diff_returns_no_partial_patches_for_a_nested_duplicate() {
    let old = VNode::box_node().child(VNode::box_node().with_key("branch"));
    let new = VNode::box_node().child(
        VNode::box_node()
            .with_key("branch")
            .child(VNode::text("first").with_key("duplicate"))
            .child(VNode::text("second").with_key("duplicate")),
    );

    let failure = try_diff(&old, &new).expect_err("nested duplicate must fail");

    assert!(matches!(
        failure,
        ReconcilePlanError::DuplicateSiblingKey {
            first_index: 0,
            second_index: 1,
            ..
        }
    ));
}

#[test]
fn no_op_plan_rejects_a_drifted_previous_tree_without_committing_it() {
    fn element_with_child_width(width: f32) -> Element {
        Box::new()
            .child(Box::new().key("stable").width(width).height(1.0))
            .into_element()
    }

    let mut engine = LayoutEngine::new();
    let committed = element_with_child_width(2.0);
    let committed_id = committed.children.iter().next().expect("one child").id;
    engine.compute_element_incremental(&committed, None, 20, 4);

    let target = element_with_child_width(9.0);
    let mut unrelated_engine = LayoutEngine::new();
    let (drifted_previous, _) = unrelated_engine.compute_element_incremental(&target, None, 20, 4);

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.compute_element_incremental(&target, Some(&drifted_previous), 20, 4)
    }));

    assert!(
        result.is_err(),
        "a no-op diff must not authorize a caller-supplied previous tree"
    );
    assert_eq!(
        engine.get_layout(committed_id).map(|layout| layout.width),
        Some(2.0),
        "the committed engine must remain on its real previous tree"
    );
}

fn accepts_root_metadata_mutation(mutate: impl FnOnce(&mut VNode)) {
    let root = Box::new()
        .key("root-source")
        .child(Box::new().key("stable").width(3.0))
        .into_element();
    let mut engine = LayoutEngine::new();
    let (mut previous, _) = engine.compute_element_incremental(&root, None, 20, 4);
    mutate(&mut previous);

    let (_, outcome) = engine
        .try_compute_element_incremental_checked(&root, Some(&previous), 20, 4)
        .expect("root identity metadata is not sibling identity");

    assert!(outcome.used_reconciler);
    assert!(!outcome.fallback_full_rebuild);
}

#[test]
fn previous_root_key_token_is_ignored_as_identity_metadata() {
    accepts_root_metadata_mutation(|previous| previous.key.user_key = Some(u64::MAX));
}

#[test]
fn previous_root_key_type_is_ignored_as_identity_metadata() {
    accepts_root_metadata_mutation(|previous| previous.key.type_id = TypeId::of::<u128>());
}

#[test]
fn previous_root_props_key_is_ignored_as_identity_metadata() {
    accepts_root_metadata_mutation(|previous| previous.props.key = Some("caller-copy".into()));
}

#[test]
fn previous_root_real_props_drift_is_rejected() {
    let root = Box::new().width(3.0).into_element();
    let mut engine = LayoutEngine::new();
    let (mut previous, _) = engine.compute_element_incremental(&root, None, 20, 4);
    previous.props.style.width = 9.0.into();

    let failure = engine
        .try_compute_element_incremental_checked(&root, Some(&previous), 20, 4)
        .expect_err("non-key root props are committed semantics");

    assert!(matches!(
        failure,
        IncrementalLayoutError::Identity(ReconcilePlanError::PreviousTreeMismatch)
    ));
}

#[test]
fn previous_root_content_and_node_type_drift_are_rejected() {
    for node_type in [VNodeType::Text("changed".into()), VNodeType::Box] {
        let root = Text::new("committed").into_element();
        let mut engine = LayoutEngine::new();
        let (mut previous, _) = engine.compute_element_incremental(&root, None, 20, 4);
        previous.node_type = node_type;

        let failure = engine
            .try_compute_element_incremental_checked(&root, Some(&previous), 20, 4)
            .expect_err("root content and node type are committed semantics");

        assert!(matches!(
            failure,
            IncrementalLayoutError::Identity(ReconcilePlanError::PreviousTreeMismatch)
        ));
    }
}
