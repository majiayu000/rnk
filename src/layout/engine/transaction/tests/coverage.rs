use taffy::NodeId;

use crate::core::{Dimension, Element, ElementId, VNode, VNodeType};
use crate::layout::{
    IncrementalInvariantError, IncrementalLayoutError, PatchStage, PatchTransactionCause,
    TransactionalLayoutError,
};
use crate::reconciler::{
    ReconcilePlanError, ScopedIdentityArena, plan_diff_in, plan_initial_tree_in,
};

use super::super::super::{
    LayoutEngine, Shared,
    context_sync::ContextSyncError,
    incremental::ElementVNodeSnapshot,
    patching::LayoutPatchOrigins,
    postcondition::{TargetValidationCause, TargetValidationError},
};
use super::super::{context_sync_error, postcondition_error};

#[test]
fn prepared_target_and_missing_committed_snapshot_are_observed() {
    let root = Element::root();
    let mut engine = LayoutEngine::new();
    let prepared = engine
        .prepare_element_incremental(&root, None, 20, 4)
        .expect("initial frame");
    assert!(matches!(
        prepared.current_vnode().node_type,
        VNodeType::Root
    ));
    let (previous, _) = prepared.commit(&mut engine);

    engine.committed_vnode = Shared::new(None);
    let error = match engine.prepare_element_incremental(&root, Some(&previous), 20, 4) {
        Err(error) => error,
        Ok(_) => panic!("a live tree without its committed snapshot is inconsistent"),
    };
    assert!(matches!(
        error,
        TransactionalLayoutError::Upstream(IncrementalLayoutError::Identity(
            ReconcilePlanError::PreviousTreeMismatch
        ))
    ));
}

#[test]
fn changed_candidate_reports_element_alias_sync_failure() {
    let before = Element::root();
    let mut after = Element::root();
    after.add_child(Element::box_element().with_key("created"));
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 4)
        .expect("initial frame");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let mut snapshot =
        ElementVNodeSnapshot::from_element(&after, &mut arena).expect("target snapshot");
    let plan = plan_diff_in(&previous, &snapshot.vnode, &mut arena).expect("target plan");

    let ghost = VNode::root().child(VNode::box_node().with_key("ghost"));
    let mut ghost_arena = ScopedIdentityArena::default();
    let ghost_plan = plan_initial_tree_in(&ghost, &mut ghost_arena).expect("ghost plan");
    snapshot.element_scopes.insert(
        ElementId::new(),
        ghost_plan.root.children[0].identity.clone(),
    );

    let error =
        match engine.prepare_changed_element_candidate(&snapshot, &snapshot.vnode, &plan, 20, 4) {
            Err(error) => error,
            Ok(_) => panic!("an unmapped element alias must fail candidate preparation"),
        };
    assert_eq!(error.stage, PatchStage::VerifyPostcondition);
    assert!(matches!(
        *error.source,
        PatchTransactionCause::Invariant(IncrementalInvariantError::ElementMapMismatch)
    ));
}

#[test]
fn context_and_postcondition_error_mappers_cover_both_origin_shapes() {
    let old = VNode::root().child(VNode::box_node().with_key("node"));
    let mut new = old.clone();
    new.children[0].props.style.width = Dimension::Points(4.0);
    let mut engine = LayoutEngine::new();
    engine.try_compute_vnode(&old, 20, 4).expect("old frame");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let plan = plan_diff_in(&old, &new, &mut arena).expect("update plan");
    let origins = LayoutPatchOrigins::for_plan(&engine, &plan);
    let node_id = engine.vnode_map[&plan.root.children[0].identity];
    let taffy = context_sync_error(
        &plan,
        &engine,
        &origins,
        ContextSyncError::Taffy {
            node_id: Some(node_id),
            key: None,
            source: taffy::TaffyError::InvalidInputNode(node_id),
        },
    );
    assert_eq!(taffy.patch_index, Some(0));
    assert!(matches!(*taffy.source, PatchTransactionCause::Taffy(_)));

    let invariant = context_sync_error(
        &plan,
        &engine,
        &LayoutPatchOrigins::default(),
        ContextSyncError::Invariant {
            node_id: None,
            key: Some(new.key),
            source: IncrementalInvariantError::ScopedMapMismatch,
        },
    );
    assert_eq!(invariant.patch_index, None);
    assert!(matches!(
        *invariant.source,
        PatchTransactionCause::Invariant(IncrementalInvariantError::ScopedMapMismatch)
    ));

    let mapped = postcondition_error(
        &plan,
        None,
        TargetValidationError {
            key: Some(new.key),
            source: TargetValidationCause::Taffy(taffy::TaffyError::InvalidInputNode(NodeId::new(
                u64::MAX,
            ))),
        },
    );
    assert!(matches!(*mapped.source, PatchTransactionCause::Taffy(_)));
}
