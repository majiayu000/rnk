use std::panic::{AssertUnwindSafe, catch_unwind};

use taffy::NodeId;

use crate::core::{Element, ElementId, NodeKey, Style, VNode};
use crate::layout::{
    DirectPatchError, DirectPatchPreflightCause, DirectPatchPreflightError, FullRebuildError,
    IncrementalInvariantError, IncrementalPatchKind, PatchFailure, PatchKind, PatchStage,
    PatchTransactionCause, RebuildFailure, RebuildStage, TextFlowError, TextFlowInput,
    TextFlowSourceKind, TransactionalLayoutError,
};
use crate::reconciler::{
    Patch, ReconcilePlanError, ScopedIdentityArena, plan_diff_in, plan_initial_tree_in,
};

use super::super::super::{
    LayoutEngine,
    context_sync::{
        ContextSyncError, LayoutRunError, set_context_pin_fault, set_layout_compute_fault,
        set_layout_read_back_fault,
    },
    incremental::{ApplyPlanError, ElementVNodeSnapshot, IncrementalFault, set_incremental_fault},
    patch_error::PatchError,
    postcondition::TargetValidationCause,
};
use super::super::{
    LayoutPatchOrigins, context_sync_cause, layout_run_error_parts,
    legacy_direct_transaction_error, legacy_preflight_error, rebuild_failure,
    rebuild_stage_for_layout_error, target_validation_cause, transaction_error_for_plan,
    transaction_stage_error_for_key,
};

fn update_plan() -> (
    LayoutEngine,
    VNode,
    crate::reconciler::ReconcilePlan,
    NodeKey,
) {
    let old = VNode::root().child(VNode::box_node().with_key("node"));
    let mut new = old.clone();
    new.children[0].props.style.width = crate::core::Dimension::Points(3.0);
    let mut engine = LayoutEngine::new();
    engine.try_compute_vnode(&old, 20, 4).expect("old frame");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let plan = plan_diff_in(&old, &new, &mut arena).expect("update plan");
    let key = match plan.patches() {
        [Patch::Update { key, .. }] => *key,
        patches => panic!("expected one update, got {patches:?}"),
    };
    (engine, new, plan, key)
}

fn element_snapshot(element: &Element) -> ElementVNodeSnapshot {
    ElementVNodeSnapshot::from_element(element, &mut ScopedIdentityArena::default())
        .expect("valid element snapshot")
}

fn require_rebuild_failure(
    result: Result<LayoutEngine, FullRebuildError>,
    context: &str,
) -> FullRebuildError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("{context}"),
    }
}

#[test]
fn transaction_locator_helpers_cover_batch_and_single_patch_paths() {
    let (engine, _target, plan, key) = update_plan();
    let error = ApplyPlanError {
        patch: PatchError::new(PatchKind::Update, key, PatchFailure::UnknownNode),
        stage: PatchStage::SetStyle,
        source: PatchTransactionCause::Patch(PatchFailure::UnknownNode),
        patch_index: None,
    };
    let batch = transaction_error_for_plan(&plan, &LayoutPatchOrigins::default(), error);
    assert_eq!(batch.patch_index, None);
    assert_eq!(batch.key, Some(key));

    let single = transaction_stage_error_for_key(
        &plan,
        Some(&LayoutPatchOrigins::for_plan(&engine, &plan)),
        Some(key),
        PatchStage::SetContext,
        PatchTransactionCause::Invariant(IncrementalInvariantError::ScopedMapMismatch),
    );
    assert_eq!(single.patch_index, Some(0));

    let no_key = transaction_stage_error_for_key(
        &plan,
        None,
        None,
        PatchStage::ReadBack,
        PatchTransactionCause::Invariant(IncrementalInvariantError::MissingComputedLayout),
    );
    assert_eq!(no_key.patch_index, None);
    assert_eq!(no_key.key, None);
}

#[test]
fn legacy_error_mappers_cover_identity_lookup_passthrough_and_impossible_input() {
    let key = VNode::box_node().with_key("node").key;
    let identity = legacy_preflight_error(DirectPatchPreflightError {
        patch_index: 0,
        kind: IncrementalPatchKind::Update,
        key: Some(key),
        parent: None,
        source: Box::new(DirectPatchPreflightCause::Identity(
            ReconcilePlanError::PreviousTreeMismatch,
        )),
    });
    assert!(matches!(identity, DirectPatchError::Identity(_)));

    let lookup = legacy_preflight_error(DirectPatchPreflightError {
        patch_index: 0,
        kind: IncrementalPatchKind::Update,
        key: Some(key),
        parent: None,
        source: Box::new(DirectPatchPreflightCause::AmbiguousTarget { match_count: 2 }),
    });
    assert!(matches!(lookup, DirectPatchError::Lookup(_)));

    let parent_lookup = legacy_preflight_error(DirectPatchPreflightError {
        patch_index: 0,
        kind: IncrementalPatchKind::Create,
        key: None,
        parent: Some(key),
        source: Box::new(DirectPatchPreflightCause::AmbiguousParent { match_count: 3 }),
    });
    assert!(matches!(parent_lookup, DirectPatchError::Lookup(_)));

    let passthrough = legacy_direct_transaction_error(TransactionalLayoutError::DirectPatch(
        DirectPatchError::Identity(ReconcilePlanError::PreviousTreeMismatch),
    ));
    assert!(matches!(passthrough, DirectPatchError::Identity(_)));

    let impossible = catch_unwind(AssertUnwindSafe(|| {
        legacy_direct_transaction_error(TransactionalLayoutError::InitialBuild(FullRebuildError {
            stage: RebuildStage::BuildTarget,
            key: None,
            source: RebuildFailure::InvalidTargetRoot,
        }))
    }));
    assert!(impossible.is_err());
}

#[test]
fn rebuild_cause_and_layout_error_mappers_are_exhaustive() {
    let invalid = NodeId::new(u64::MAX);
    let causes = [
        PatchTransactionCause::Taffy(taffy::TaffyError::InvalidInputNode(invalid)),
        PatchTransactionCause::TextFlow(TextFlowError::InvalidTabStop),
        PatchTransactionCause::Invariant(IncrementalInvariantError::MissingRoot),
        PatchTransactionCause::Patch(PatchFailure::UnknownNode),
        PatchTransactionCause::Patch(PatchFailure::MissingParent),
        PatchTransactionCause::Patch(PatchFailure::BuildFailed),
        PatchTransactionCause::Patch(PatchFailure::TreeRejected),
        PatchTransactionCause::Patch(PatchFailure::LayoutFailed),
        PatchTransactionCause::Patch(PatchFailure::PostconditionViolated),
    ];
    let failures: Vec<_> = causes.into_iter().map(rebuild_failure).collect();
    assert_eq!(
        failures,
        vec![
            RebuildFailure::Taffy(taffy::TaffyError::InvalidInputNode(invalid)),
            RebuildFailure::TextFlow(TextFlowError::InvalidTabStop),
            RebuildFailure::Invariant(IncrementalInvariantError::MissingRoot),
            RebuildFailure::Invariant(IncrementalInvariantError::ScopedMapMismatch),
            RebuildFailure::Invariant(IncrementalInvariantError::MissingRoot),
            RebuildFailure::Invariant(IncrementalInvariantError::InvalidMappedNode),
            RebuildFailure::Invariant(IncrementalInvariantError::InvalidMappedNode),
            RebuildFailure::Invariant(IncrementalInvariantError::MissingComputedLayout),
            RebuildFailure::Invariant(IncrementalInvariantError::CurrentFrameContextMismatch),
        ]
    );

    let stages = [
        rebuild_stage_for_layout_error(&LayoutRunError::Taffy {
            node_id: None,
            source: taffy::TaffyError::InvalidInputNode(invalid),
        }),
        rebuild_stage_for_layout_error(&LayoutRunError::TextFlow {
            node_id: None,
            source: TextFlowError::Interrupted,
        }),
        rebuild_stage_for_layout_error(&LayoutRunError::ReadBackTaffy {
            node_id: None,
            source: taffy::TaffyError::InvalidInputNode(invalid),
        }),
        rebuild_stage_for_layout_error(&LayoutRunError::ReadBackTextFlow {
            node_id: None,
            source: TextFlowError::Interrupted,
        }),
        rebuild_stage_for_layout_error(&LayoutRunError::Invariant {
            node_id: None,
            source: IncrementalInvariantError::MissingComputedLayout,
        }),
    ];
    assert_eq!(
        stages,
        [
            RebuildStage::ComputeLayout,
            RebuildStage::ComputeLayout,
            RebuildStage::VerifyPostcondition,
            RebuildStage::VerifyPostcondition,
            RebuildStage::VerifyPostcondition,
        ]
    );

    let (stage, cause) = layout_run_error_parts(LayoutRunError::Invariant {
        node_id: None,
        source: IncrementalInvariantError::CurrentFrameContextMismatch,
    });
    assert_eq!(stage, PatchStage::ReadBack);
    assert!(matches!(
        cause,
        PatchTransactionCause::Invariant(IncrementalInvariantError::CurrentFrameContextMismatch)
    ));
    let (stage, cause) = layout_run_error_parts(LayoutRunError::ReadBackTextFlow {
        node_id: None,
        source: TextFlowError::Interrupted,
    });
    assert_eq!(stage, PatchStage::ReadBack);
    assert!(matches!(cause, PatchTransactionCause::TextFlow(_)));

    let context_taffy = context_sync_cause(ContextSyncError::Taffy {
        node_id: Some(invalid),
        key: None,
        source: taffy::TaffyError::InvalidInputNode(invalid),
    });
    assert!(matches!(context_taffy, PatchTransactionCause::Taffy(_)));
    let context_invariant = context_sync_cause(ContextSyncError::Invariant {
        node_id: None,
        key: None,
        source: IncrementalInvariantError::ScopedMapMismatch,
    });
    assert!(matches!(
        context_invariant,
        PatchTransactionCause::Invariant(IncrementalInvariantError::ScopedMapMismatch)
    ));

    let validation_taffy = target_validation_cause(TargetValidationCause::Taffy(
        taffy::TaffyError::InvalidInputNode(invalid),
    ));
    assert!(matches!(validation_taffy, PatchTransactionCause::Taffy(_)));
    let validation_invariant = target_validation_cause(TargetValidationCause::Invariant(
        IncrementalInvariantError::CurrentFrameContextMismatch,
    ));
    assert!(matches!(
        validation_invariant,
        PatchTransactionCause::Invariant(IncrementalInvariantError::CurrentFrameContextMismatch)
    ));
}

#[test]
fn fresh_rebuild_reports_every_naturally_reachable_stage() {
    let root = Element::root();
    let root_snapshot = element_snapshot(&root);
    let duplicate = VNode::root().children([
        VNode::box_node().with_key("duplicate"),
        VNode::box_node().with_key("duplicate"),
    ]);
    let invalid = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(&root_snapshot, &duplicate, 20, 4),
        "a target with duplicate identities cannot be rebuilt",
    );
    assert_eq!(invalid.stage, RebuildStage::BuildTarget);

    let mut boxed = Element::root();
    boxed.add_child(Element::box_element().with_key("box"));
    let boxed_snapshot = element_snapshot(&boxed);
    set_incremental_fault(IncrementalFault::CreateBoxContext);
    let apply = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(
            &boxed_snapshot,
            &boxed_snapshot.vnode,
            20,
            4,
        ),
        "fresh materialization fault",
    );
    assert_eq!(apply.stage, RebuildStage::SetContext);

    let mut ghost_context = element_snapshot(&root);
    let ghost = VNode::root().child(VNode::text("ghost").with_key("ghost"));
    let ghost_plan =
        plan_initial_tree_in(&ghost, &mut ScopedIdentityArena::default()).expect("ghost plan");
    ghost_context.text_inputs.insert(
        ghost_plan.root.children[0].identity.clone(),
        TextFlowInput::plain("ghost", TextFlowSourceKind::Exact, Style::default()),
    );
    let sync = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(&ghost_context, &ghost_context.vnode, 20, 4),
        "unmapped text context",
    );
    assert_eq!(sync.stage, RebuildStage::SetContext);

    let mut ghost_alias = element_snapshot(&root);
    ghost_alias.element_scopes.insert(
        ElementId::new(),
        ghost_plan.root.children[0].identity.clone(),
    );
    let alias = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(&ghost_alias, &ghost_alias.vnode, 20, 4),
        "unmapped element alias",
    );
    assert_eq!(alias.stage, RebuildStage::VerifyPostcondition);

    set_layout_compute_fault();
    let compute = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(&root_snapshot, &root_snapshot.vnode, 20, 4),
        "layout compute fault",
    );
    assert_eq!(compute.stage, RebuildStage::ComputeLayout);

    let mut text = Element::root();
    text.add_child(Element::text("text").with_key("text"));
    let text_snapshot = element_snapshot(&text);
    set_layout_read_back_fault();
    let read_back = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(&text_snapshot, &text_snapshot.vnode, 20, 4),
        "layout read-back fault",
    );
    assert_eq!(read_back.stage, RebuildStage::VerifyPostcondition);

    set_context_pin_fault();
    let pin = require_rebuild_failure(
        LayoutEngine::new().try_rebuild_snapshot_fresh(&text_snapshot, &text_snapshot.vnode, 20, 4),
        "context pin fault",
    );
    assert_eq!(pin.stage, RebuildStage::VerifyPostcondition);

    let mut text_flow = LayoutEngine::new();
    text_flow.set_text_flow_policy(0, "…", 1);
    let flow = require_rebuild_failure(
        text_flow.try_rebuild_snapshot_fresh(&text_snapshot, &text_snapshot.vnode, 20, 4),
        "invalid text policy",
    );
    assert_eq!(flow.stage, RebuildStage::ComputeLayout);
}
