//! GH-60 public transaction, recovery, and compatibility acceptance ledger.

use std::panic::{AssertUnwindSafe, catch_unwind};

use rnk::core::{Dimension, Element, ElementId, NodeKey, Props, VNode};
use rnk::layout::{
    CheckedIncrementalLayoutReport, DirectPatchApplyReport, DirectPatchError,
    DirectPatchPreflightCause, DirectPatchPreflightError, IncrementalInvariantError,
    IncrementalLayoutError, IncrementalLayoutOutcome, IncrementalPatchKind, Layout, LayoutEngine,
    PatchStage, PatchTransactionCause, RebuildFailure, TransactionalLayoutError,
};
use rnk::reconciler::{Patch, ReconcilePlanError, try_diff};
use rnk::renderer::Output;

fn keyed_tree() -> VNode {
    VNode::box_node().children([
        VNode::text("a").with_key("a"),
        VNode::text("b").with_key("b"),
        VNode::text("c").with_key("c"),
    ])
}

fn missing_key() -> NodeKey {
    VNode::text("missing").with_key("missing").key
}

fn wide_props() -> Props {
    let mut props = Props::new();
    props.style.min_width = Dimension::Points(11.0);
    props
}

fn width_props(width: f32) -> Props {
    let mut props = Props::new();
    props.style.width = Dimension::Points(width);
    props
}

fn widths(engine: &LayoutEngine, tree: &VNode) -> Vec<Option<i32>> {
    tree.children
        .iter()
        .map(|child| {
            engine
                .get_vnode_layout(child.key)
                .map(|layout| layout.width as i32)
        })
        .collect()
}

fn expect_preflight(error: TransactionalLayoutError) -> DirectPatchPreflightError {
    match error {
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(source)) => source,
        other => panic!("expected direct preflight failure, got {other}"),
    }
}

fn element_frame(text: &str) -> (Element, ElementId) {
    let mut root = Element::root();
    let child = Element::text(text).with_key("message");
    let child_id = child.id;
    root.add_child(child);
    (root, child_id)
}

fn committed_then_rebuild_failure() -> (LayoutEngine, ElementId, ElementId, TransactionalLayoutError)
{
    let mut engine = LayoutEngine::new();
    let (before, before_id) = element_frame("before");
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 4)
        .expect("initial target commits");
    let mut after = Element::root();
    let first = Element::text("after").with_key("message");
    let after_id = first.id;
    let mut second = Element::text("duplicate ElementId").with_key("second");
    second.id = after_id;
    after.add_child(first);
    after.add_child(second);
    let failure = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 4)
        .expect_err("invalid policy must fail candidate and rebuild");
    (engine, before_id, after_id, failure)
}

#[test]
fn mixed_batch_failure_commits_no_partial_state() {
    let tree = keyed_tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 8);
    let before = widths(&engine, &tree);
    let failure = engine
        .try_apply_patches_transactional(&[
            Patch::update(
                tree.children[0].key,
                tree.children[0].props.clone(),
                wide_props(),
            ),
            Patch::remove(missing_key()),
        ])
        .expect_err("missing second target rejects the whole batch");
    let failure = expect_preflight(failure);

    assert_eq!(failure.patch_index, 1);
    assert_eq!(failure.kind, IncrementalPatchKind::Remove);
    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::MissingTarget
    ));
    assert_eq!(widths(&engine, &tree), before);
}

#[test]
fn invalid_plan_returns_without_rebuild_or_mutation() {
    let mut engine = LayoutEngine::new();
    let (before, before_id) = element_frame("before");
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 4)
        .expect("initial frame");
    let mut invalid = Element::root();
    let first = Element::text("first").with_key("duplicate");
    let first_id = first.id;
    invalid.add_child(first);
    invalid.add_child(Element::text("second").with_key("duplicate"));

    let failure = engine
        .try_compute_element_incremental_transactional(&invalid, Some(&previous), 20, 4)
        .expect_err("invalid plan must fail before recovery");
    assert!(matches!(
        failure,
        TransactionalLayoutError::Upstream(IncrementalLayoutError::Identity(
            ReconcilePlanError::DuplicateSiblingKey { .. }
        ))
    ));
    assert!(engine.get_layout(before_id).is_some());
    assert!(engine.get_layout(first_id).is_none());
}

#[test]
fn public_transaction_compatibility_surface_compiles() {
    let layout = Layout {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    let outcome = IncrementalLayoutOutcome {
        used_reconciler: true,
        patch_count: 0,
        fallback_full_rebuild: false,
        patch_error: None,
    };
    assert_eq!(layout.width, 1.0);
    assert!(outcome.used_reconciler);

    let (element, _) = element_frame("surface");
    let mut engine = LayoutEngine::new();
    let prepared = engine
        .prepare_element_incremental(&element, None, 20, 4)
        .expect("prepared surface");
    assert!(matches!(
        prepared.report(),
        CheckedIncrementalLayoutReport::InitialFullBuild
    ));
    let (previous, _) = prepared.commit(&mut engine);
    let next = element_frame("surface").0;
    let (_, legacy) = engine.compute_element_incremental(&next, Some(&previous), 20, 4);
    assert!(legacy.used_reconciler);

    let tree = keyed_tree();
    let mut raw = LayoutEngine::new();
    raw.compute_vnode(&tree, 40, 8);
    assert_eq!(raw.try_apply_patches(&[]), Ok(false));
    assert!(!raw.apply_patches(&[]));
}

#[test]
fn direct_patch_per_kind_cardinality_is_checked_before_mutation() {
    let tree = keyed_tree();
    let missing = missing_key();
    let created = VNode::text("new").with_key("new");
    let created_key = created.key;
    let cases = [
        (
            Patch::create(created, missing),
            IncrementalPatchKind::Create,
            Some(created_key),
            Some(missing),
            true,
        ),
        (
            Patch::update(missing, Props::new(), wide_props()),
            IncrementalPatchKind::Update,
            Some(missing),
            None,
            false,
        ),
        (
            Patch::remove(missing),
            IncrementalPatchKind::Remove,
            Some(missing),
            None,
            false,
        ),
        (
            Patch::replace(missing, VNode::text("replacement").with_key("replacement")),
            IncrementalPatchKind::Replace,
            Some(missing),
            None,
            false,
        ),
        (
            Patch::reorder(missing, vec![]),
            IncrementalPatchKind::Reorder,
            None,
            Some(missing),
            true,
        ),
    ];

    for (patch, kind, key, parent, missing_parent) in cases {
        let mut engine = LayoutEngine::new();
        engine.compute_vnode(&tree, 40, 8);
        let before = widths(&engine, &tree);
        let failure = engine
            .try_apply_patches_transactional(&[patch])
            .err()
            .map(expect_preflight)
            .expect("cardinality failure");
        assert_eq!(failure.patch_index, 0);
        assert_eq!(failure.kind, kind);
        assert_eq!(failure.key, key);
        assert_eq!(failure.parent, parent);
        assert!(matches!(
            (failure.source.as_ref(), missing_parent),
            (DirectPatchPreflightCause::MissingParent, true)
                | (DirectPatchPreflightCause::MissingTarget, false)
        ));
        assert_eq!(widths(&engine, &tree), before);
    }
}

#[test]
fn direct_create_and_subtree_collisions_report_exact_ordinal_and_kind() {
    let tree = keyed_tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 8);
    let free = VNode::text("free").with_key("free");
    let existing = VNode::text("duplicate").with_key("a");
    let existing_key = existing.key;
    let failure = engine
        .try_apply_patches_transactional(&[
            Patch::create(free.clone(), tree.key),
            Patch::create(existing, tree.key),
        ])
        .err()
        .map(expect_preflight)
        .expect("existing identity must reject the batch");
    assert_eq!(failure.patch_index, 1);
    assert_eq!(failure.kind, IncrementalPatchKind::Create);
    assert_eq!(failure.key, Some(existing_key));
    assert_eq!(failure.parent, Some(tree.key));
    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::AlreadyExists
    ));
    assert!(engine.get_vnode_layout(free.key).is_none());

    let duplicate_a = VNode::text("a").with_key("inner");
    let duplicate_b = VNode::text("b").with_key("inner");
    let collision_key = duplicate_b.key;
    let subtree = VNode::box_node()
        .with_key("subtree")
        .children([duplicate_a, duplicate_b]);
    let failure = engine
        .try_apply_patches_transactional(&[Patch::create(subtree, tree.key)])
        .err()
        .map(expect_preflight)
        .expect("duplicate subtree identity must fail");
    assert_eq!(failure.patch_index, 0);
    assert_eq!(failure.kind, IncrementalPatchKind::Create);
    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::SubtreeCollision { conflicting_key }
            if conflicting_key.matches(&collision_key)
    ));
}

#[test]
fn direct_batch_dependencies_are_preflighted_in_order() {
    let tree = keyed_tree();
    let target = tree.children[0].key;
    let batches = [
        (
            vec![
                Patch::remove(target),
                Patch::update(target, Props::new(), wide_props()),
            ],
            false,
        ),
        (
            vec![
                Patch::replace(target, VNode::text("replacement").with_key("replacement")),
                Patch::update(target, Props::new(), wide_props()),
            ],
            true,
        ),
    ];

    for (patches, replaced) in batches {
        let mut engine = LayoutEngine::new();
        engine.compute_vnode(&tree, 40, 8);
        let before = widths(&engine, &tree);
        let failure = engine
            .try_apply_patches_transactional(&patches)
            .err()
            .map(expect_preflight)
            .expect("batch dependency must fail in preflight");
        assert_eq!(failure.patch_index, 1);
        assert_eq!(failure.kind, IncrementalPatchKind::Update);
        assert!(matches!(
            (failure.source.as_ref(), replaced),
            (
                DirectPatchPreflightCause::DependencyRemoved {
                    prior_patch_index: 0
                },
                false
            ) | (
                DirectPatchPreflightCause::DependencyReplaced {
                    prior_patch_index: 0
                },
                true
            )
        ));
        assert_eq!(widths(&engine, &tree), before);
    }
}

fn tree_with_ambiguous_target() -> (VNode, NodeKey) {
    let duplicate = VNode::text("left").with_key("duplicate");
    let duplicate_key = duplicate.key;
    let left = VNode::box_node().with_key("left").child(duplicate);
    let right = VNode::box_node()
        .with_key("right")
        .child(VNode::text("right").with_key("duplicate"));
    (VNode::box_node().children([left, right]), duplicate_key)
}

fn tree_with_ambiguous_parent() -> (VNode, NodeKey) {
    let shared = VNode::box_node().with_key("shared-parent");
    let shared_key = shared.key;
    let left = VNode::box_node().with_key("left").child(shared);
    let right = VNode::box_node()
        .with_key("right")
        .child(VNode::box_node().with_key("shared-parent"));
    (VNode::box_node().children([left, right]), shared_key)
}

#[test]
fn direct_patch_ambiguous_target_fails_before_mutation() {
    let (tree, target) = tree_with_ambiguous_target();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 8);
    let failure = engine
        .try_apply_patches_transactional(&[Patch::update(target, Props::new(), wide_props())])
        .err()
        .map(expect_preflight)
        .expect("unscoped target must remain ambiguous");
    assert_eq!(failure.patch_index, 0);
    assert_eq!(failure.kind, IncrementalPatchKind::Update);
    assert_eq!(failure.key, Some(target));
    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::AmbiguousTarget { match_count: 2 }
    ));
}

#[test]
fn direct_patch_ambiguous_parent_fails_before_mutation() {
    let (tree, parent) = tree_with_ambiguous_parent();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 8);
    let node = VNode::text("new").with_key("new");
    let failure = engine
        .try_apply_patches_transactional(&[Patch::create(node.clone(), parent)])
        .err()
        .map(expect_preflight)
        .expect("unscoped parent must remain ambiguous");
    assert_eq!(failure.patch_index, 0);
    assert_eq!(failure.kind, IncrementalPatchKind::Create);
    assert_eq!(failure.key, Some(node.key));
    assert_eq!(failure.parent, Some(parent));
    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::AmbiguousParent { match_count: 2 }
    ));
}

#[test]
fn direct_patch_apply_report_is_concrete_and_exact() {
    let tree = keyed_tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 8);
    assert!(matches!(
        engine
            .try_apply_patches_transactional(&[])
            .expect("empty batch"),
        DirectPatchApplyReport::NoChange
    ));
    let report = engine
        .try_apply_patches_transactional(&[Patch::update(
            tree.children[0].key,
            tree.children[0].props.clone(),
            wide_props(),
        )])
        .expect("one update commits");
    assert!(matches!(
        report,
        DirectPatchApplyReport::Applied { patch_count: 1 }
    ));
}

#[test]
fn legacy_wrappers_delegate_to_checked_core() {
    let tree = keyed_tree();
    let patch = Patch::update(
        tree.children[0].key,
        tree.children[0].props.clone(),
        wide_props(),
    );
    let mut checked = LayoutEngine::new();
    checked.compute_vnode(&tree, 40, 8);
    assert!(
        checked
            .try_apply_patches_checked(std::slice::from_ref(&patch))
            .expect("checked compatibility wrapper")
    );

    let mut legacy = LayoutEngine::new();
    legacy.compute_vnode(&tree, 40, 8);
    assert_eq!(
        legacy.try_apply_patches(std::slice::from_ref(&patch)),
        Ok(true)
    );
    let mut non_try = LayoutEngine::new();
    non_try.compute_vnode(&tree, 40, 8);
    assert!(non_try.apply_patches(&[patch]));
    assert_eq!(widths(&checked, &tree), widths(&legacy, &tree));
    assert_eq!(widths(&legacy, &tree), widths(&non_try, &tree));
}

#[test]
fn legacy_apply_patches_ambiguity_fails_loudly_without_mutation() {
    let (tree, target) = tree_with_ambiguous_target();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 8);
    let before = engine
        .try_get_vnode_layout(target)
        .expect_err("legacy target is ambiguous");
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        engine.apply_patches(&[Patch::update(target, Props::new(), wide_props())]);
    }));
    assert!(panicked.is_err());
    assert_eq!(
        engine
            .try_get_vnode_layout(target)
            .expect_err("ambiguity must remain after panic"),
        before
    );
}

#[test]
fn raw_patch_reindexes_positional_identities_in_current_virtual_state() {
    let positional = VNode::box_node();
    let keep = VNode::box_node().with_key("keep");
    let tree = VNode::root().children([positional, keep.clone()]);
    let inserted = VNode::box_node().with_key("inserted");
    let expected = VNode::root().children([
        inserted.clone(),
        VNode::box_node(),
        keep.clone().with_props(wide_props()),
    ]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    engine
        .try_apply_patches_transactional(&[
            Patch::create(inserted, tree.key),
            Patch::update(keep.key, keep.props.clone(), wide_props()),
        ])
        .expect("a valid insertion reindexes the current positional identity");

    assert!(engine.get_vnode_layout(expected.children[0].key).is_some());
    assert!(engine.get_vnode_layout(expected.children[1].key).is_some());
    assert_eq!(
        engine
            .get_vnode_layout(keep.key)
            .expect("keyed survivor remains addressable")
            .width,
        11.0
    );
}

#[test]
fn raw_patch_uses_current_positional_state_at_each_ordinal() {
    let tree = VNode::root().children([
        VNode::box_node().with_props(width_props(1.0)),
        VNode::box_node().with_props(width_props(2.0)),
        VNode::box_node().with_props(width_props(3.0)),
    ]);
    let first = tree.children[0].key;
    let current_second = tree.children[1].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    engine
        .try_apply_patches_transactional(&[Patch::remove(first), Patch::remove(current_second)])
        .expect("the second ordinal addresses current slot one after normalization");

    let remaining = engine
        .try_get_vnode_layout(first)
        .expect("current positional key is unambiguous")
        .expect("one positional child remains");
    assert_eq!(remaining.width, 2.0);
}

#[test]
fn raw_patch_tombstones_preserve_cross_scope_ambiguity() {
    let (tree, duplicate) = tree_with_ambiguous_target();
    let left_removed =
        VNode::box_node().children([VNode::box_node().with_key("left"), tree.children[1].clone()]);
    let both_removed = VNode::box_node().children([
        VNode::box_node().with_key("left"),
        VNode::box_node().with_key("right"),
    ]);
    let first_remove = try_diff(&tree, &left_removed)
        .expect("first scoped removal plan")
        .into_iter()
        .find(|patch| matches!(patch, Patch::Remove { .. }))
        .expect("first scoped remove");
    let second_remove = try_diff(&left_removed, &both_removed)
        .expect("second scoped removal plan")
        .into_iter()
        .find(|patch| matches!(patch, Patch::Remove { .. }))
        .expect("second scoped remove");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    let failure = expect_preflight(
        engine
            .try_apply_patches_transactional(&[
                first_remove,
                second_remove,
                Patch::update(duplicate, Props::new(), wide_props()),
            ])
            .expect_err("two removed scopes remain ambiguous to an unscoped lookup"),
    );

    assert_eq!(failure.patch_index, 2);
    assert_eq!(failure.kind, IncrementalPatchKind::Update);
    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::AmbiguousTarget { match_count: 2 }
    ));
}

#[test]
fn raw_patch_failures_keep_available_child_and_parent_locators() {
    let a = VNode::box_node().with_key("a");
    let b = VNode::box_node().with_key("b");
    let tree = VNode::root().children([a.clone(), b.clone()]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let stale = expect_preflight(
        engine
            .try_apply_patches_transactional(&[Patch::update(a.key, wide_props(), a.props.clone())])
            .expect_err("stale update props"),
    );
    assert!(stale.parent.is_some());

    let dependency = expect_preflight(
        engine
            .try_apply_patches_transactional(&[
                Patch::remove(a.key),
                Patch::update(a.key, a.props.clone(), wide_props()),
            ])
            .expect_err("update references a removed child"),
    );
    assert_eq!(dependency.key, Some(a.key));
    assert!(dependency.parent.is_some());
    assert!(matches!(
        dependency.source.as_ref(),
        DirectPatchPreflightCause::DependencyRemoved {
            prior_patch_index: 0
        }
    ));
}

#[test]
fn raw_patch_rejects_cross_type_duplicate_key_tokens_as_subtree_collision() {
    let tree = VNode::root();
    let duplicate_text = VNode::text("duplicate").with_key("duplicate");
    let expected_key = duplicate_text.key;
    let subtree = VNode::box_node()
        .with_key("subtree")
        .children([VNode::box_node().with_key("duplicate"), duplicate_text]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    let failure = expect_preflight(
        engine
            .try_apply_patches_transactional(&[Patch::create(subtree, tree.key)])
            .expect_err("same exact key token is duplicate across element types"),
    );

    assert!(matches!(
        failure.source.as_ref(),
        DirectPatchPreflightCause::SubtreeCollision { conflicting_key }
            if conflicting_key.user_key == expected_key.user_key
    ));
}

#[test]
fn raw_replace_uses_the_resolved_slot_after_prior_reorder() {
    let a = VNode::box_node().with_key("a");
    let b = VNode::box_node().with_key("b");
    let tree = VNode::root().children([a.clone(), b.clone()]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    engine
        .try_apply_patches_transactional(&[
            Patch::reorder(tree.key, vec![b.key, a.key]),
            Patch::replace(a.key, VNode::box_node()),
        ])
        .expect("replace uses the target's current slot after reorder");

    let expected = VNode::root().children([b.clone(), VNode::box_node()]);
    assert!(engine.get_vnode_layout(b.key).is_some());
    assert!(engine.get_vnode_layout(a.key).is_none());
    assert!(engine.get_vnode_layout(expected.children[1].key).is_some());
}

#[test]
fn recovered_rebuild_preserves_incremental_cause() {
    let (target, text_id) = element_frame("recovered");
    let previous = VNode::root().child(
        VNode::text("recovered")
            .with_key("message")
            .with_props(Props::new().key("message")),
    );
    let mut engine = LayoutEngine::new();
    engine
        .build_vnode_tree(&previous)
        .expect("public build creates a committed but not-yet-computed tree");

    let (current, report) = engine
        .try_compute_element_incremental_transactional(&target, Some(&previous), 0, 0)
        .expect("one fresh target rebuild recovers the incomplete committed layout");

    assert_eq!(current.children, previous.children);
    let CheckedIncrementalLayoutReport::RecoveredFullRebuild {
        patch_count,
        incremental_failure,
    } = report
    else {
        panic!("expected a real recovery report, got {report:?}");
    };
    assert_eq!(patch_count, 0);
    assert_eq!(incremental_failure.patch_index, None);
    assert_eq!(incremental_failure.kind, IncrementalPatchKind::Recompute);
    assert_eq!(incremental_failure.key, None);
    assert_eq!(incremental_failure.parent, None);
    assert_eq!(incremental_failure.stage, PatchStage::VerifyPostcondition);
    assert!(matches!(
        incremental_failure.source.as_ref(),
        PatchTransactionCause::Invariant(IncrementalInvariantError::CurrentFrameContextMismatch)
    ));
    assert!(engine.has_tree());
    assert!(engine.get_layout(text_id).is_some());
    let mut output = Output::new(0, 0);
    rnk::try_render_element_tree_checked(&target, &engine, &mut output, 0.0, 0.0)
        .expect("recovered target publishes the required TextFlow");
}

#[test]
fn target_aware_patch_failure_rebuilds_once() {
    let (_, _, _, failure) = committed_then_rebuild_failure();
    assert!(matches!(
        &failure,
        TransactionalLayoutError::RecoveryFailed { .. }
    ));
    assert!(failure.incremental_failure().is_some());
    assert!(failure.rebuild_failure().is_some());
}

#[test]
fn rebuild_failure_returns_both_causes_and_preserves_committed_state() {
    let (engine, before_id, after_id, failure) = committed_then_rebuild_failure();
    let (incremental, rebuild) = match failure {
        TransactionalLayoutError::RecoveryFailed {
            incremental,
            rebuild,
        } => (incremental, rebuild),
        other => panic!("expected dual recovery error, got {other}"),
    };
    assert!(matches!(
        incremental.source.as_ref(),
        PatchTransactionCause::Invariant(_)
    ));
    assert!(matches!(rebuild.source, RebuildFailure::Invariant(_)));
    assert!(engine.get_layout(before_id).is_some());
    assert!(engine.get_layout(after_id).is_none());
}

#[test]
fn cloned_nan_props_produce_no_planner_update() {
    let mut vnode = VNode::root();
    vnode.props.style.flex_grow = f32::NAN;
    assert!(
        try_diff(&vnode, &vnode.clone())
            .expect("a cloned valid tree plans")
            .is_empty()
    );
}

#[test]
fn cloned_nan_props_are_unchanged_for_previous_frame_validation() {
    let mut element = Element::root();
    element.style.flex_grow = f32::NAN;
    let mut engine = LayoutEngine::new();
    let (previous, first) = engine
        .try_compute_element_incremental_transactional(&element, None, 20, 4)
        .expect("initial NaN style frame commits");
    assert_eq!(first, CheckedIncrementalLayoutReport::InitialFullBuild);
    let (_, second) = engine
        .try_compute_element_incremental_transactional(&element, Some(&previous), 20, 4)
        .expect("an identical NaN style frame remains valid");
    assert_eq!(second, CheckedIncrementalLayoutReport::NoChange);
}

#[test]
fn cloned_nan_props_are_valid_raw_update_create_and_replace_payloads() {
    let mut updated = VNode::root();
    updated.props.style.flex_grow = f32::NAN;
    let mut update_engine = LayoutEngine::new();
    update_engine.compute_vnode(&updated, 20, 4);
    let old_props = updated.props.clone();
    let mut new_props = old_props.clone();
    new_props.scroll_offset_x = Some(1);
    let update_applied = matches!(
        update_engine.try_apply_patches_transactional(&[Patch::update(
            updated.key,
            old_props,
            new_props,
        )]),
        Ok(DirectPatchApplyReport::Applied { patch_count: 1 })
    );

    let root = VNode::root();
    let mut created = VNode::box_node().with_key("created");
    created.props.style.flex_grow = f32::NAN;
    let mut create_engine = LayoutEngine::new();
    create_engine.compute_vnode(&root, 20, 4);
    let create_applied = matches!(
        create_engine.try_apply_patches_transactional(&[Patch::create(created, root.key)]),
        Ok(DirectPatchApplyReport::Applied { patch_count: 1 })
    );

    let existing = VNode::box_node().with_key("existing");
    let existing_key = existing.key;
    let replace_root = VNode::root().child(existing);
    let mut replacement = VNode::text("replacement").with_key("replacement");
    replacement.props.style.flex_grow = f32::NAN;
    let mut replace_engine = LayoutEngine::new();
    replace_engine.compute_vnode(&replace_root, 20, 4);
    let replace_applied = matches!(
        replace_engine
            .try_apply_patches_transactional(&[Patch::replace(existing_key, replacement)]),
        Ok(DirectPatchApplyReport::Applied { patch_count: 1 })
    );

    assert_eq!(
        (update_applied, create_applied, replace_applied),
        (true, true, true)
    );
}
