use super::{EngineFingerprint, LayoutEngine};
use crate::core::{Dimension, Element, ElementId, VNode};
use crate::layout::{
    CheckedIncrementalLayoutReport, DirectPatchError, IncrementalPatchKind, PatchStage,
    PatchTransactionCause, RebuildFailure, RebuildStage, TransactionalLayoutError,
};
use crate::reconciler::{Patch, ScopedIdentityArena, ScopedNodeIdentity, plan_diff_in};

use super::super::super::incremental::ElementVNodeSnapshot;

fn assert_target_text_flow_failure(
    before: &Element,
    after: &Element,
    failing_element: ElementId,
    expected_kind: IncrementalPatchKind,
) {
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(before, None, 20, 6)
        .expect("initial frame");
    super::super::rebuild_counter::take_attempts();
    let (kind, expected_key, expected_parent, rebuild_key) = {
        let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
        let snapshot =
            ElementVNodeSnapshot::from_element(after, &mut arena).expect("valid target snapshot");
        let plan = plan_diff_in(&previous, &snapshot.vnode, &mut arena).expect("valid plan");
        let (kind, key, parent) = match plan.patches() {
            [Patch::Create { key, parent, .. }] => {
                (IncrementalPatchKind::Create, *key, Some(*parent))
            }
            [Patch::Update { key, .. }] => (
                IncrementalPatchKind::Update,
                *key,
                Some(ScopedNodeIdentity::Root.scoped_patch_address(previous.key)),
            ),
            [Patch::Replace { key, .. }] => (
                IncrementalPatchKind::Replace,
                *key,
                Some(ScopedNodeIdentity::Root.scoped_patch_address(previous.key)),
            ),
            patches => panic!("expected one target patch, got {patches:?}"),
        };
        (kind, key, parent, snapshot.element_keys[&failing_element])
    };
    assert_eq!(kind, expected_kind);
    engine.set_text_flow_policy(0, "…", 1);
    let before_failure = EngineFingerprint::capture(&engine);

    let error = engine
        .try_compute_element_incremental_transactional(after, Some(&previous), 20, 6)
        .expect_err("invalid tab stop must fail candidate and rebuild");
    let diagnostic = format!("{error:?}");

    assert!(
        matches!(
            error,
            TransactionalLayoutError::RecoveryFailed { incremental, rebuild }
                if incremental.patch_index == Some(0)
                    && incremental.kind == expected_kind
                    && incremental.key == Some(expected_key)
                    && incremental.parent == expected_parent
                    && incremental.stage == PatchStage::ComputeLayout
                    && matches!(
                        *incremental.source,
                        PatchTransactionCause::TextFlow(
                            crate::layout::TextFlowError::InvalidTabStop
                        )
                    )
                    && rebuild.stage == RebuildStage::ComputeLayout
                    && rebuild.key == Some(rebuild_key)
                    && matches!(
                        rebuild.source,
                        RebuildFailure::TextFlow(crate::layout::TextFlowError::InvalidTabStop)
                    )
        ),
        "{diagnostic}"
    );
    assert_eq!(super::super::rebuild_counter::take_attempts(), 1);
    assert_eq!(EngineFingerprint::capture(&engine), before_failure);
}

fn assert_raw_text_flow_failure(
    initial: &VNode,
    patch: Patch,
    expected_kind: IncrementalPatchKind,
    expected_key: crate::core::NodeKey,
    expected_parent: crate::core::NodeKey,
) {
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(initial, 20, 6);
    engine.set_text_flow_policy(0, "…", 1);
    let before_failure = EngineFingerprint::capture(&engine);
    super::super::rebuild_counter::take_attempts();

    let error = engine
        .try_apply_patches_transactional(&[patch])
        .expect_err("invalid tab stop must reject the raw transaction");
    let diagnostic = format!("{error:?}");
    assert!(
        matches!(
            error,
            TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
                if error.patch_index == Some(0)
                    && error.kind == expected_kind
                    && error.key == Some(expected_key)
                    && error.parent == Some(expected_parent)
                    && error.stage == PatchStage::ComputeLayout
                    && matches!(
                        *error.source,
                        PatchTransactionCause::TextFlow(
                            crate::layout::TextFlowError::InvalidTabStop
                        )
                    )
        ),
        "{diagnostic}"
    );
    assert_eq!(super::super::rebuild_counter::take_attempts(), 0);
    assert_eq!(EngineFingerprint::capture(&engine), before_failure);
}

#[test]
fn target_aware_compute_text_flow_failures_keep_exact_origins() {
    let mut before_update = Element::root();
    before_update.add_child(Element::text("same").with_key("node"));
    let mut updated_text = Element::text("same").with_key("node");
    updated_text.style.width = Dimension::Points(7.0);
    let update_id = updated_text.id;
    let mut after_update = Element::root();
    after_update.add_child(updated_text);
    assert_target_text_flow_failure(
        &before_update,
        &after_update,
        update_id,
        IncrementalPatchKind::Update,
    );

    let mut before_replace = Element::root();
    before_replace.add_child(Element::box_element().with_key("branch"));
    let replacement = Element::text("replacement").with_key("branch");
    let replacement_id = replacement.id;
    let mut after_replace = Element::root();
    after_replace.add_child(replacement);
    assert_target_text_flow_failure(
        &before_replace,
        &after_replace,
        replacement_id,
        IncrementalPatchKind::Replace,
    );

    let before_create = Element::root();
    let leaf = Element::text("created").with_key("leaf");
    let leaf_id = leaf.id;
    let mut outer = Element::box_element().with_key("outer");
    outer.add_child(leaf);
    let mut after_create = Element::root();
    after_create.add_child(outer);
    assert_target_text_flow_failure(
        &before_create,
        &after_create,
        leaf_id,
        IncrementalPatchKind::Create,
    );
}

#[test]
fn raw_compute_text_flow_failures_keep_exact_origins() {
    let text = VNode::text("same").with_key("text");
    let update_root = VNode::root().child(text.clone());
    let mut updated_props = text.props.clone();
    updated_props.style.width = Dimension::Points(7.0);
    assert_raw_text_flow_failure(
        &update_root,
        Patch::update(text.key, text.props.clone(), updated_props),
        IncrementalPatchKind::Update,
        text.key,
        ScopedNodeIdentity::Root.scoped_patch_address(update_root.key),
    );

    let old_box = VNode::box_node().with_key("branch");
    let replace_root = VNode::root().child(old_box.clone());
    assert_raw_text_flow_failure(
        &replace_root,
        Patch::replace(old_box.key, VNode::text("replacement").with_key("branch")),
        IncrementalPatchKind::Replace,
        old_box.key,
        ScopedNodeIdentity::Root.scoped_patch_address(replace_root.key),
    );

    let create_root = VNode::root();
    let outer = VNode::box_node()
        .with_key("outer")
        .child(VNode::text("created").with_key("leaf"));
    assert_raw_text_flow_failure(
        &create_root,
        Patch::create(outer.clone(), create_root.key),
        IncrementalPatchKind::Create,
        outer.key,
        ScopedNodeIdentity::Root.scoped_patch_address(create_root.key),
    );
}

#[test]
fn compute_taffy_failures_keep_known_root_update_origins() {
    let root = VNode::root();
    let mut raw = LayoutEngine::new();
    raw.compute_vnode(&root, 20, 6);
    let old_props = root.props.clone();
    let mut new_props = old_props.clone();
    new_props.style.width = Dimension::Points(4.0);
    super::super::super::context_sync::set_layout_compute_fault();
    let raw_error = raw
        .try_apply_patches_transactional(&[Patch::update(root.key, old_props, new_props)])
        .expect_err("raw root compute fault");
    assert!(matches!(
        raw_error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
            if error.patch_index == Some(0)
                && error.kind == IncrementalPatchKind::Update
                && error.key == Some(root.key)
                && error.parent.is_none()
                && error.stage == PatchStage::ComputeLayout
                && matches!(*error.source, PatchTransactionCause::Taffy(_))
    ));

    let before = Element::root();
    let mut after = Element::root();
    after.style.width = Dimension::Points(4.0);
    let mut target = LayoutEngine::new();
    let (previous, _) = target
        .try_compute_element_incremental_transactional(&before, None, 20, 6)
        .expect("initial root");
    super::super::rebuild_counter::take_attempts();
    super::super::super::context_sync::set_layout_compute_fault();
    let (_, report) = target
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 6)
        .expect("one-shot root compute fault rebuilds");
    let diagnostic = format!("{report:?}");
    assert!(
        matches!(
            &report,
            CheckedIncrementalLayoutReport::RecoveredFullRebuild {
                patch_count: 1,
                incremental_failure,
            } if incremental_failure.patch_index == Some(0)
                && incremental_failure.kind == IncrementalPatchKind::Update
                && incremental_failure.key
                    == Some(ScopedNodeIdentity::Root.scoped_patch_address(previous.key))
                && incremental_failure.parent.is_none()
                && incremental_failure.stage == PatchStage::ComputeLayout
                && matches!(*incremental_failure.source, PatchTransactionCause::Taffy(_))
        ),
        "{diagnostic}"
    );
    assert_eq!(super::super::rebuild_counter::take_attempts(), 1);
    assert_eq!(
        target
            .committed_vnode
            .as_ref()
            .map(|root| &root.props.style),
        Some(&after.style)
    );
}
