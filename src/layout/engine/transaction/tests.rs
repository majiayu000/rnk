use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::core::{Dimension, Element, ElementType, VNode};
use crate::layout::{
    CheckedIncrementalLayoutReport, IncrementalInvariantError, IncrementalPatchKind, PatchStage,
    PatchTransactionCause, RebuildFailure, RebuildStage, TransactionalLayoutError,
};
use crate::reconciler::{Patch, ScopedIdentityArena, plan_diff_in};

use super::super::{
    LayoutEngine,
    context_sync::set_layout_compute_fault,
    incremental::{IncrementalFault, set_incremental_fault},
    postcondition::{PostconditionFault, set_postcondition_fault},
    test_fingerprint::EngineFingerprint,
};

mod coverage;

fn frame(branch_is_text: bool, extra: bool) -> Element {
    let mut root = Element::root();
    if branch_is_text {
        root.add_child(Element::text("replacement").with_key("branch"));
    } else {
        let mut branch = Element::box_element().with_key("branch");
        branch.add_child(Element::text("leaf").with_key("leaf"));
        root.add_child(branch);
    }
    if extra {
        root.add_child(Element::text("extra").with_key("extra"));
    }
    root
}

#[test]
fn invalid_incremental_root_is_not_reported_as_an_initial_build() {
    let before = frame(false, false);
    let target = Element::new(ElementType::VirtualText);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    super::super::patching::take_fresh_rebuild_attempts();
    let fingerprint = EngineFingerprint::capture(&engine);

    let error = engine
        .try_compute_element_incremental_transactional(&target, Some(&previous), 20, 6)
        .expect_err("an incremental target still needs a layout root");

    assert!(matches!(
        error,
        TransactionalLayoutError::InvalidTarget(error)
            if error.key.is_none()
                && matches!(error.source, RebuildFailure::InvalidTargetRoot)
    ));
    assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 0);
    assert_eq!(EngineFingerprint::capture(&engine), fingerprint);
}

#[test]
fn created_text_readback_recovery_preserves_incremental_cause() {
    let before = Element::root();
    let created = Element::text("created").with_key("created");
    let created_id = created.id;
    let mut after = Element::root();
    after.add_child(created);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let (expected_target, expected_key, expected_parent) = {
        let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
        let snapshot =
            super::super::incremental::ElementVNodeSnapshot::from_element(&after, &mut arena)
                .expect("target snapshot");
        let plan = plan_diff_in(&previous, &snapshot.vnode, &mut arena).expect("create plan");
        match plan.patches() {
            [Patch::Create { key, parent, .. }] => (snapshot.vnode, *key, *parent),
            patches => panic!("expected one create patch, got {patches:?}"),
        }
    };
    super::super::patching::take_fresh_rebuild_attempts();
    super::super::context_sync::set_layout_read_back_fault();

    let (current, report) = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect("one-shot readback fault recovers through a fresh build");

    assert!(matches!(
        report,
        CheckedIncrementalLayoutReport::RecoveredFullRebuild {
            patch_count: 1,
            incremental_failure,
        } if incremental_failure.patch_index == Some(0)
            && incremental_failure.kind == IncrementalPatchKind::Create
            && incremental_failure.key == Some(expected_key)
            && incremental_failure.parent == Some(expected_parent)
            && incremental_failure.stage == PatchStage::ReadBack
            && matches!(*incremental_failure.source, PatchTransactionCause::Taffy(_))
    ));
    assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 1);
    assert_eq!(current, expected_target);
    assert_eq!(engine.committed_vnode.as_ref(), Some(&current));
    assert!(engine.get_layout(created_id).is_some());
    assert!(engine.current_text_flow(created_id).is_some());
}

pub(crate) fn incremental_success_has_target_exact_tree_root_and_order() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let (_, report) = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect("incremental target");

    assert!(matches!(
        report,
        CheckedIncrementalLayoutReport::Incremental { patch_count } if patch_count > 0
    ));
    assert_eq!(engine.taffy.total_node_count(), 4);
    assert_eq!(engine.vnode_map.len(), 4);
    assert_eq!(
        engine.root_node,
        engine
            .vnode_map
            .get(&crate::reconciler::ScopedNodeIdentity::Root)
            .copied()
    );
    let root = engine.root_node.expect("target root");
    let children = engine.taffy.children(root).expect("target children");
    assert_eq!(children.len(), 2);
    assert_eq!(engine.node_map[&after.id], root);
}

pub(crate) fn remove_replace_success_has_no_descendant_or_orphan_state() {
    let before = frame(false, true);
    let replacement = frame(true, false);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let (_, report) = engine
        .try_compute_element_incremental_transactional(&replacement, Some(&previous), 20, 6)
        .expect("replace and remove target");

    assert!(matches!(
        report,
        CheckedIncrementalLayoutReport::Incremental { .. }
    ));
    assert_eq!(engine.taffy.total_node_count(), 2);
    assert_eq!(engine.vnode_map.len(), 2);
    assert_eq!(engine.node_map.len(), 2);
    assert!(engine.committed_vnode.as_ref().is_some_and(
        |root| root.children[0].node_type == crate::core::VNodeType::Text("replacement".into())
    ));
}

pub(crate) fn commit_failure_attempts_exactly_one_fresh_rebuild() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    super::super::patching::take_fresh_rebuild_attempts();
    set_postcondition_fault(PostconditionFault::MissingRoot);

    let (_, report) = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect("one fresh rebuild recovers the candidate postcondition fault");
    assert!(matches!(
        report,
        CheckedIncrementalLayoutReport::RecoveredFullRebuild {
            incremental_failure,
            ..
        } if incremental_failure.stage == PatchStage::VerifyPostcondition
            && matches!(*incremental_failure.source, crate::layout::PatchTransactionCause::Invariant(IncrementalInvariantError::MissingRoot))
    ));
    assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 1);
}

pub(crate) fn rebuild_success_must_pass_target_exact_postcondition() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    super::super::patching::take_fresh_rebuild_attempts();
    let fingerprint = EngineFingerprint::capture(&engine);
    set_incremental_fault(IncrementalFault::CreateText);
    set_postcondition_fault(PostconditionFault::MissingComputedLayout);

    let error = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect_err("a rebuild that misses the postcondition cannot commit");
    assert!(matches!(
        error,
        TransactionalLayoutError::RecoveryFailed { rebuild, .. }
            if rebuild.stage == RebuildStage::VerifyPostcondition
                && matches!(rebuild.source, RebuildFailure::Invariant(IncrementalInvariantError::MissingComputedLayout))
    ));
    assert_eq!(EngineFingerprint::capture(&engine), fingerprint);
    assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 1);
}

pub(crate) fn repeated_fault_has_stable_result_and_rebuild_count() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let committed = EngineFingerprint::capture(&engine);
    let mut failures = Vec::new();

    for _ in 0..2 {
        super::super::patching::take_fresh_rebuild_attempts();
        set_incremental_fault(IncrementalFault::CreateText);
        set_postcondition_fault(PostconditionFault::InvalidRoot);
        let (incremental, rebuild) = match engine
            .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
            .expect_err("both attempts fail")
        {
            TransactionalLayoutError::RecoveryFailed {
                incremental,
                rebuild,
            } => (*incremental, *rebuild),
            other => panic!("expected dual failure, got {other:?}"),
        };
        assert_eq!(incremental.stage, PatchStage::CreateNode);
        assert_eq!(rebuild.stage, RebuildStage::VerifyPostcondition);
        assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 1);
        assert_eq!(EngineFingerprint::capture(&engine), committed);
        failures.push((incremental, rebuild));
    }

    assert_eq!(failures[0], failures[1]);
}

pub(crate) fn candidate_and_recovery_resources_drop_on_every_exit() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let fingerprint = EngineFingerprint::capture(&engine);
    for _ in 0..8 {
        super::super::patching::take_fresh_rebuild_attempts();
        set_incremental_fault(IncrementalFault::CreateText);
        set_layout_compute_fault();
        assert!(matches!(
            engine.try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6,),
            Err(TransactionalLayoutError::RecoveryFailed { .. })
        ));
        assert_eq!(EngineFingerprint::capture(&engine), fingerprint);
        assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 1);
    }
    let unwind = catch_unwind(AssertUnwindSafe(|| {
        let prepared = engine
            .prepare_element_incremental(&after, Some(&previous), 20, 6)
            .expect("a successful candidate can remain uncommitted");
        assert!(matches!(
            prepared.report(),
            CheckedIncrementalLayoutReport::Incremental { .. }
        ));
        assert_eq!(EngineFingerprint::capture(&engine), fingerprint);
        panic!("exercise prepared-candidate unwind");
    }));
    assert!(unwind.is_err());
    assert_eq!(EngineFingerprint::capture(&engine), fingerprint);
    engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect("a later clean attempt is not contaminated by dropped candidates");
}

#[test]
fn preflight_and_raw_patch_paths_do_not_attempt_recovery_builds() {
    let before = frame(false, false);
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    super::super::patching::take_fresh_rebuild_attempts();
    let mismatched_previous = VNode::root();
    assert!(matches!(
        engine.try_compute_element_incremental_transactional(
            &before,
            Some(&mismatched_previous),
            20,
            6,
        ),
        Err(TransactionalLayoutError::Upstream(_))
    ));
    assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 0);

    let root = VNode::root();
    let mut raw = LayoutEngine::new();
    raw.compute_vnode(&root, 20, 6);
    let old_props = root.props.clone();
    let mut new_props = old_props.clone();
    new_props.style.width = Dimension::Points(4.0);
    super::super::context_sync::set_layout_compute_fault();
    assert!(matches!(
        raw.try_apply_patches_transactional(&[Patch::update(root.key, old_props, new_props,)]),
        Err(TransactionalLayoutError::DirectPatch(_))
    ));
    assert_eq!(super::super::patching::take_fresh_rebuild_attempts(), 0);
}

pub(crate) fn initial_frame_success_commits_target_exact_state() {
    let target = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (_, report) = engine
        .try_compute_element_incremental_transactional(&target, None, 20, 6)
        .expect("initial frame");
    assert_eq!(report, CheckedIncrementalLayoutReport::InitialFullBuild);
    assert_eq!(engine.taffy.total_node_count(), 4);
    assert_eq!(engine.vnode_map.len(), 4);
    assert_eq!(engine.node_map.len(), 4);
    assert!(
        engine
            .node_map
            .keys()
            .all(|element_id| engine.get_layout(*element_id).is_some())
    );
}

pub(crate) fn initial_build_failure_has_no_incremental_cause_or_commit() {
    let target = frame(false, true);
    let mut engine = LayoutEngine::new();
    let empty = EngineFingerprint::capture(&engine);
    set_incremental_fault(IncrementalFault::CreateBox);
    let error = engine
        .try_compute_element_incremental_transactional(&target, None, 20, 6)
        .expect_err("initial build fault");
    assert!(matches!(error, TransactionalLayoutError::InitialBuild(_)));
    assert!(error.incremental_failure().is_none());
    assert_eq!(EngineFingerprint::capture(&engine), empty);
}

pub(crate) fn initial_compute_failure_has_no_incremental_cause_or_commit() {
    let target = frame(false, true);
    let mut engine = LayoutEngine::new();
    let empty = EngineFingerprint::capture(&engine);
    set_layout_compute_fault();
    let error = engine
        .try_compute_element_incremental_transactional(&target, None, 20, 6)
        .expect_err("initial compute fault");
    assert!(matches!(error, TransactionalLayoutError::InitialBuild(_)));
    assert!(error.incremental_failure().is_none());
    assert_eq!(EngineFingerprint::capture(&engine), empty);
}

pub(crate) fn initial_postcondition_failure_has_no_incremental_cause_or_commit() {
    let target = frame(false, true);
    let mut engine = LayoutEngine::new();
    let empty = EngineFingerprint::capture(&engine);
    set_postcondition_fault(PostconditionFault::CurrentFrameContextMismatch);
    let error = engine
        .try_compute_element_incremental_transactional(&target, None, 20, 6)
        .expect_err("initial postcondition fault");
    assert!(matches!(
        error,
        TransactionalLayoutError::InitialBuild(ref rebuild)
            if rebuild.stage == RebuildStage::VerifyPostcondition
    ));
    assert!(error.incremental_failure().is_none());
    assert_eq!(EngineFingerprint::capture(&engine), empty);
}

pub(crate) fn viewport_only_recompute_is_transactional() {
    let before = frame(false, false);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let committed = EngineFingerprint::capture(&engine);

    let current = frame(false, false);
    let prepared = engine
        .prepare_element_incremental(&current, Some(&previous), 21, 6)
        .expect("viewport-only candidate");
    assert_eq!(
        prepared.report(),
        &CheckedIncrementalLayoutReport::RecomputedViewport
    );
    assert_eq!(EngineFingerprint::capture(&engine), committed);
    assert!(prepared.engine().get_layout(current.id).is_some());
    drop(prepared);
    assert_eq!(EngineFingerprint::capture(&engine), committed);

    let prepared = engine
        .prepare_element_incremental(&current, Some(&previous), 21, 6)
        .expect("second viewport-only candidate");
    let (_, report) = prepared.commit(&mut engine);
    assert_eq!(report, CheckedIncrementalLayoutReport::RecomputedViewport);
    assert_ne!(EngineFingerprint::capture(&engine), committed);
    assert!(engine.get_layout(current.id).is_some());
}

pub(crate) fn each_patch_failure_has_exact_locator_and_cause() {
    super::super::incremental::tests::incremental_create_fault_reports_the_real_patch_locator();
    super::super::incremental::tests::incremental_replace_fault_reports_the_real_patch_locator();
    super::super::incremental::tests::incremental_remove_fault_reports_the_real_patch_locator();
    super::super::incremental::tests::replacement_descendant_cleanup_fault_keeps_replace_root_locator();
    super::super::patching::tests::target_aware_second_create_and_update_have_exact_ordinals();
    super::super::incremental_order::tests::structural_set_children_faults_keep_create_remove_replace_origins();
    super::super::incremental_order::tests::incremental_reorder_fault_reports_the_real_parent_locator();
}

pub(crate) fn failed_or_dropped_candidate_preserves_committed_fingerprint() {
    prepared_layout_drop_and_commit_are_atomic();
    candidate_and_recovery_resources_drop_on_every_exit();
}

pub(crate) fn all_backend_failures_are_observed() {
    super::super::incremental::tests::taffy_fault_seams_preserve_error_mapping_and_checked_caller_atomicity();
    super::super::incremental::tests::context_faults_report_set_context_stage();
    super::super::incremental_order::tests::commit_and_postcondition_faults_recover_atomically_through_checked_caller();
    super::super::patching::tests::freeze_regressions::batch_compute_readback_postcondition_use_recompute_locator();
}

pub(crate) fn fault_backend_is_test_only_and_diagnostics_are_terminal_safe() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    set_incremental_fault(IncrementalFault::CreateText);
    set_layout_compute_fault();

    let error = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect_err("private one-shot faults remain visible as a dual typed error");
    for diagnostic in [format!("{error}"), format!("{error:?}")] {
        assert!(
            diagnostic.chars().all(|character| !character.is_control()),
            "diagnostic contains a terminal control character: {diagnostic:?}"
        );
    }
}

#[test]
pub(crate) fn prepared_layout_drop_and_commit_are_atomic() {
    let before = frame(false, false);
    let after = frame(false, true);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");
    let fingerprint = EngineFingerprint::capture(&engine);
    let prepared = engine
        .prepare_element_incremental(&after, Some(&previous), 20, 6)
        .expect("prepared frame");
    assert_eq!(EngineFingerprint::capture(&engine), fingerprint);
    assert!(prepared.engine().get_layout(after.id).is_some());
    drop(prepared);
    assert_eq!(EngineFingerprint::capture(&engine), fingerprint);

    let prepared = engine
        .prepare_element_incremental(&after, Some(&previous), 20, 6)
        .expect("second prepared frame");
    prepared.commit(&mut engine);
    assert_ne!(EngineFingerprint::capture(&engine), fingerprint);
}

#[test]
fn element_fixture_keeps_virtual_text_outside_layout_target() {
    let mut root = Element::root();
    root.add_child(Element::new(ElementType::VirtualText));
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_element_incremental_transactional(&root, None, 20, 4)
        .expect("virtual text child is filtered");
    assert_eq!(engine.taffy.total_node_count(), 1);
}

#[test]
fn stale_or_cross_engine_prepared_frames_cannot_publish() {
    let before = frame(false, false);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial frame");

    let fresh_aliases = frame(false, false);
    let stale = engine
        .prepare_element_incremental(&fresh_aliases, Some(&previous), 20, 6)
        .expect("unchanged alias frame");
    let changed = frame(true, false);
    let (changed_vnode, _) = engine
        .try_compute_element_incremental_transactional(&changed, Some(&previous), 20, 6)
        .expect("newer frame commits first");
    let changed_fingerprint = EngineFingerprint::capture(&engine);

    let stale_commit = catch_unwind(AssertUnwindSafe(|| stale.commit(&mut engine)));
    assert!(stale_commit.is_err(), "stale frame must fail loudly");
    assert_eq!(EngineFingerprint::capture(&engine), changed_fingerprint);

    let source = LayoutEngine::new();
    let cross_engine = source
        .prepare_element_incremental(&Element::root(), None, 20, 6)
        .expect("source frame");
    let mut other = LayoutEngine::new();
    let other_fingerprint = EngineFingerprint::capture(&other);
    let cross_commit = catch_unwind(AssertUnwindSafe(|| cross_engine.commit(&mut other)));
    assert!(cross_commit.is_err(), "cross-engine frame must fail loudly");
    assert_eq!(EngineFingerprint::capture(&other), other_fingerprint);

    let current = frame(true, false);
    let current_id = current.id;
    let after_failed_mutation = engine
        .prepare_element_incremental(&current, Some(&changed_vnode), 20, 6)
        .expect("current alias frame");
    assert!(
        engine
            .try_apply_patches_transactional(&[Patch::remove(VNode::root().key)])
            .is_err()
    );
    after_failed_mutation.commit(&mut engine);
    assert!(engine.get_layout(current_id).is_some());
}
