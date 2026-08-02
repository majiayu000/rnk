use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) use super::super::test_fingerprint::EngineFingerprint;
use super::LayoutEngine;
use crate::core::{Dimension, Element, NodeKey, Props, Style, VNode};
use crate::layout::{
    CheckedIncrementalLayoutReport, DirectPatchError, DirectPatchPreflightCause,
    IncrementalPatchKind, PatchError, PatchFailure, PatchKind, PatchStage, PatchTransactionCause,
    RebuildFailure, TransactionalLayoutError,
};
use crate::reconciler::{Patch, try_diff};

mod compute_provenance;
pub(crate) mod freeze_regressions;
mod raw_alias_regressions;

fn width_props(width: f32) -> Props {
    let mut style = Style::new();
    style.width = Dimension::Points(width);
    Props::with_style(style)
}

fn keyed_children() -> VNode {
    VNode::box_node().children([
        VNode::box_node().with_key("a"),
        VNode::box_node().with_key("b"),
        VNode::box_node().with_key("c"),
    ])
}

#[test]
fn root_remove_is_a_typed_atomic_rejection() {
    let root = VNode::box_node().child(VNode::text("kept").with_key("child"));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.try_apply_patches_checked(&[Patch::remove(root.key)])
    }));

    let error = result
        .expect("root remove must not panic")
        .expect_err("root remove must be rejected");
    assert!(matches!(
        error,
        DirectPatchError::Patch(PatchError {
            kind: PatchKind::Remove,
            key,
            ..
        }) if key == root.key
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn batch_layout_failure_uses_the_recompute_locator() {
    let removed = VNode::box_node().with_key("removed");
    let removed_key = removed.key;
    let text = VNode::text("text").with_key("text");
    let text_key = text.key;
    let root = VNode::box_node().children([removed, text]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    engine.set_text_flow_policy(0, "…", 1);
    let before = EngineFingerprint::capture(&engine);

    let error = engine
        .try_apply_patches_transactional(&[Patch::remove(removed_key)])
        .expect_err("invalid text policy must fail candidate layout");
    let diagnostic = format!("{error:?}");

    assert!(
        matches!(
            error,
            TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
                if error.patch_index.is_none()
                    && error.kind == IncrementalPatchKind::Recompute
                    && error.key == Some(text_key)
                    && error.parent.is_none()
                    && error.stage == PatchStage::ReadBack
                    && matches!(*error.source, PatchTransactionCause::TextFlow(_))
        ),
        "{diagnostic}"
    );
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn remove_recomputes_surviving_parent_like_a_cold_build() {
    let mut parent_props = Props::new();
    parent_props.style.flex_direction = crate::core::FlexDirection::Column;
    let wide = VNode::box_node()
        .with_key("wide")
        .with_props(width_props(12.0));
    let wide_key = wide.key;
    let narrow = VNode::box_node()
        .with_key("narrow")
        .with_props(width_props(3.0));
    let parent = VNode::box_node()
        .with_key("parent")
        .with_props(parent_props.clone())
        .children([wide, narrow.clone()]);
    let parent_key = parent.key;
    let root = VNode::box_node().child(parent);
    let target = VNode::box_node().child(
        VNode::box_node()
            .with_key("parent")
            .with_props(parent_props)
            .child(narrow),
    );
    let mut incremental = LayoutEngine::new();
    incremental.compute_vnode(&root, 40, 10);

    incremental
        .try_apply_patches_checked(&[Patch::remove(wide_key)])
        .expect("remove succeeds");
    let mut cold = LayoutEngine::new();
    cold.compute_vnode(&target, 40, 10);

    assert_eq!(
        incremental
            .get_vnode_layout(parent_key)
            .map(|layout| (layout.width, layout.height)),
        cold.get_vnode_layout(target.children[0].key)
            .map(|layout| (layout.width, layout.height))
    );
}

#[test]
fn public_diff_root_replace_succeeds_and_publishes_new_root() {
    let old = VNode::box_node().child(VNode::text("old"));
    let new = VNode::text("new root");
    let patches = try_diff(&old, &new).expect("root replacement diff is valid");
    assert!(matches!(patches.as_slice(), [Patch::Replace { .. }]));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);

    assert_eq!(engine.try_apply_patches_checked(&patches), Ok(true));
    assert!(engine.get_vnode_layout(new.key).is_some());
    assert_eq!(engine.committed_vnode.as_ref(), Some(&new));
}

#[test]
fn reorder_requires_an_exact_unique_permutation() {
    let root = keyed_children();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);
    let omitted = vec![root.children[0].key, root.children[1].key];

    let omitted_result = catch_unwind(AssertUnwindSafe(|| {
        engine.try_apply_patches_checked(&[Patch::reorder(root.key, omitted)])
    }));
    assert!(
        omitted_result
            .expect("omitted reorder must not panic")
            .is_err()
    );
    assert_eq!(EngineFingerprint::capture(&engine), before);

    let duplicate = vec![
        root.children[0].key,
        root.children[0].key,
        root.children[2].key,
    ];
    let duplicate_result = catch_unwind(AssertUnwindSafe(|| {
        engine.try_apply_patches_checked(&[Patch::reorder(root.key, duplicate)])
    }));
    assert!(
        duplicate_result
            .expect("duplicate reorder must not panic")
            .is_err()
    );
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn positional_reorder_is_rejected_atomically() {
    let root = VNode::root().children([VNode::box_node(), VNode::box_node()]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let result = engine.try_apply_patches_checked(&[Patch::reorder(
        root.key,
        vec![root.children[1].key, root.children[0].key],
    )]);

    assert!(result.is_err());
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn keyed_reorder_is_allowed_when_positional_children_keep_their_ordinals() {
    let root = VNode::root().children([
        VNode::box_node().with_key("a"),
        VNode::box_node(),
        VNode::box_node().with_key("b"),
    ]);
    let positional = root.children[1].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    assert_eq!(
        engine.try_apply_patches_checked(&[Patch::reorder(
            root.key,
            vec![root.children[2].key, positional, root.children[0].key],
        )]),
        Ok(true)
    );
    let committed = engine.committed_vnode.as_ref().expect("batch committed");
    assert_eq!(committed.children[1].key, positional);
    assert!(engine.get_vnode_layout(positional).is_some());
}

#[test]
fn update_checks_old_props_and_preserves_canonical_frame_across_batches() {
    let original_props = width_props(2.0);
    let first_props = width_props(5.0);
    let second_props = width_props(8.0);
    let child = VNode::box_node()
        .with_key("child")
        .with_props(original_props.clone());
    let key = child.key;
    let root = VNode::box_node().child(child);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let stale =
        engine.try_apply_patches_checked(&[Patch::update(key, Props::new(), first_props.clone())]);
    assert!(stale.is_err());
    assert_eq!(EngineFingerprint::capture(&engine), before);

    assert_eq!(
        engine.try_apply_patches_checked(&[Patch::update(
            key,
            original_props,
            first_props.clone(),
        )]),
        Ok(true)
    );
    assert_eq!(
        engine.try_apply_patches_checked(&[Patch::update(key, first_props, second_props.clone(),)]),
        Ok(true)
    );
    assert_eq!(
        engine
            .committed_vnode
            .as_ref()
            .and_then(|vnode| vnode.children.first())
            .map(|vnode| &vnode.props),
        Some(&second_props)
    );
}

#[test]
fn virtual_preflight_allows_remove_then_create_of_the_same_identity() {
    let old_child = VNode::text("old").with_key("same");
    let key = old_child.key;
    let root = VNode::box_node().child(old_child);
    let replacement = VNode::text("new").with_key("same");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    assert_eq!(
        engine.try_apply_patches_checked(&[
            Patch::remove(key),
            Patch::create(replacement.clone(), root.key),
        ]),
        Ok(true)
    );
    assert_eq!(
        engine
            .committed_vnode
            .as_ref()
            .and_then(|vnode| vnode.children.first())
            .and_then(VNode::get_text),
        Some("new")
    );
}

#[test]
fn virtual_preflight_allows_create_parent_then_create_child() {
    let root = VNode::box_node();
    let parent = VNode::box_node().with_key("new-parent");
    let parent_key = parent.key;
    let child = VNode::text("nested").with_key("new-child");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    assert_eq!(
        engine.try_apply_patches_checked(&[
            Patch::create(parent, root.key),
            Patch::create(child, parent_key),
        ]),
        Ok(true)
    );
    assert_eq!(
        engine
            .committed_vnode
            .as_ref()
            .and_then(|vnode| vnode.children.first())
            .map(VNode::node_count),
        Some(2)
    );
}

#[test]
fn raw_payload_duplicates_are_validated_before_mutation() {
    let root = VNode::box_node();
    let node = VNode::box_node().with_key("node");
    let mut mismatched_props = node.props.clone();
    mismatched_props.style.width = Dimension::Points(9.0);
    let patch = Patch::Create {
        key: VNode::box_node().with_key("different").key,
        parent: root.key,
        props: mismatched_props,
        node,
    };
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    assert!(engine.try_apply_patches_checked(&[patch]).is_err());
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

fn exact_key_props(key: &str) -> Props {
    let mut props = Props::new();
    props.key = Some(key.to_owned());
    props
}

#[test]
fn public_diff_props_only_keys_apply_create_and_reorder() {
    let old = VNode::root().child(VNode::box_node().with_props(exact_key_props("a")));
    let created = VNode::box_node().with_props(exact_key_props("b"));
    let created_target = old.clone().child(created);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    engine
        .try_apply_patches_checked(&try_diff(&old, &created_target).unwrap())
        .expect("canonical props-only create applies");

    let reordered = VNode::root().children([
        VNode::box_node().with_props(exact_key_props("b")),
        VNode::box_node().with_props(exact_key_props("a")),
    ]);
    engine
        .try_apply_patches_checked(&try_diff(&created_target, &reordered).unwrap())
        .expect("canonical props-only reorder applies");
    assert_eq!(engine.committed_vnode.as_ref(), Some(&reordered));
}

#[test]
fn public_diff_root_key_metadata_update_is_directly_applicable() {
    let old = VNode::root().with_props(exact_key_props("old-root-metadata"));
    let new = VNode::root().with_props(exact_key_props("new-root-metadata"));
    let patches = try_diff(&old, &new).expect("root metadata update is valid");
    assert!(matches!(patches.as_slice(), [Patch::Update { .. }]));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);

    assert_eq!(engine.try_apply_patches_checked(&patches), Ok(true));
    assert_eq!(engine.committed_vnode.as_ref(), Some(&new));
}

#[test]
fn replace_collision_reports_exact_preflight_cause_atomically() {
    let old_child = VNode::box_node().with_key("a");
    let old_key = old_child.key;
    let sibling = VNode::box_node().with_key("b");
    let sibling_key = sibling.key;
    let root = VNode::root().children([old_child, sibling]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = engine
        .try_apply_patches_transactional(&[Patch::replace(
            old_key,
            VNode::box_node().with_key("b"),
        )])
        .expect_err("replacement must not collide with its sibling");
    let TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error)) = error else {
        panic!("expected a replace preflight error");
    };
    assert_eq!(error.patch_index, 0);
    assert_eq!(error.kind, IncrementalPatchKind::Replace);
    assert_eq!(error.key, Some(old_key));
    assert!(
        error.parent.is_some(),
        "resolved parent locator is retained"
    );
    assert!(matches!(
        *error.source,
        DirectPatchPreflightCause::SubtreeCollision { conflicting_key }
            if conflicting_key.identity() == sibling_key.identity()
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn reorder_reports_removed_and_replaced_dependencies() {
    let a = VNode::box_node().with_key("a");
    let a_key = a.key;
    let b = VNode::box_node().with_key("b");
    let b_key = b.key;
    let root = VNode::root().children([a, b]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    let removed = engine
        .try_apply_patches_transactional(&[
            Patch::remove(a_key),
            Patch::reorder(root.key, vec![a_key]),
        ])
        .expect_err("reorder cannot name a removed child");
    assert!(matches!(
        removed,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 1
                && matches!(*error.source, DirectPatchPreflightCause::DependencyRemoved { prior_patch_index: 0 })
    ));

    let replaced = engine
        .try_apply_patches_transactional(&[
            Patch::replace(a_key, VNode::box_node().with_key("c")),
            Patch::reorder(root.key, vec![a_key, b_key]),
        ])
        .expect_err("reorder cannot name a replaced child");
    assert!(matches!(
        replaced,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 1
                && matches!(*error.source, DirectPatchPreflightCause::DependencyReplaced { prior_patch_index: 0 })
    ));
}

#[test]
fn public_diff_multiple_positional_tail_removals_is_directly_applicable() {
    let old = VNode::root().children([VNode::box_node(), VNode::box_node(), VNode::box_node()]);
    let target = VNode::root().child(VNode::box_node());
    let patches = try_diff(&old, &target).expect("positional tail diff is valid");
    assert_eq!(patches.len(), 2);
    let mut incremental = LayoutEngine::new();
    incremental.compute_vnode(&old, 20, 4);

    assert_eq!(incremental.try_apply_patches_checked(&patches), Ok(true));
    assert_eq!(incremental.committed_vnode.as_ref(), Some(&target));
    assert_eq!(incremental.taffy.total_node_count(), target.node_count());
}

#[test]
fn raw_batch_executes_each_original_ordinal_with_exact_stage() {
    let root = VNode::root();
    let original_props = width_props(2.0);
    let updated_props = width_props(5.0);
    let created = VNode::box_node()
        .with_key("created")
        .with_props(original_props.clone());
    let key = created.key;
    let patches = [
        Patch::create(created, root.key),
        Patch::update(key, original_props, updated_props),
    ];
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let before = EngineFingerprint::capture(&engine);
    super::super::incremental::set_incremental_fault(
        super::super::incremental::IncrementalFault::UpdateStyle,
    );

    let error = engine
        .try_apply_patches_transactional(&patches)
        .expect_err("the second original operation receives its own fault");
    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
            if error.patch_index == Some(1)
                && error.kind == IncrementalPatchKind::Update
                && error.key == Some(key)
                && error.stage == PatchStage::SetStyle
                && matches!(*error.source, PatchTransactionCause::Taffy(_))
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn hand_written_raw_keys_address_props_only_canonical_nodes() {
    let props_a = exact_key_props("a");
    let raw_a = VNode::box_node().with_props(props_a.clone());
    let raw_a_key = raw_a.key;
    let root = VNode::root();
    let mut create = LayoutEngine::new();
    create.compute_vnode(&root, 20, 4);
    assert_eq!(
        create.try_apply_patches_checked(&[Patch::create(raw_a.clone(), root.key)]),
        Ok(true)
    );

    let mut updated_props = props_a.clone();
    updated_props.style.width = Dimension::Points(7.0);
    assert_eq!(
        create.try_apply_patches_checked(&[Patch::update(
            raw_a_key,
            props_a.clone(),
            updated_props,
        )]),
        Ok(true)
    );
    assert_eq!(
        create.try_apply_patches_checked(&[Patch::remove(raw_a_key)]),
        Ok(true)
    );

    let raw_b = VNode::box_node().with_props(exact_key_props("b"));
    let keyed = VNode::root().children([raw_a, raw_b]);
    let raw_order = vec![keyed.children[1].key, keyed.children[0].key];
    let mut reorder = LayoutEngine::new();
    reorder.compute_vnode(&keyed, 20, 4);
    assert_eq!(
        reorder.try_apply_patches_checked(&[Patch::reorder(keyed.key, raw_order)]),
        Ok(true)
    );
    assert_eq!(
        reorder.committed_vnode.as_ref().map(|root| {
            root.children
                .iter()
                .map(|child| child.props.key.as_deref())
                .collect::<Vec<_>>()
        }),
        Some(vec![Some("b"), Some("a")])
    );
}

#[test]
fn positional_create_and_replace_use_the_target_slot() {
    let keep = VNode::box_node().with_key("keep");
    let old = VNode::root().child(keep.clone());
    let target = VNode::root().children([VNode::box_node(), keep]);
    let patches = try_diff(&old, &target).expect("slot-zero insertion diff is valid");
    let mut inserted = LayoutEngine::new();
    inserted.compute_vnode(&old, 20, 4);
    assert_eq!(inserted.try_apply_patches_checked(&patches), Ok(true));
    assert_eq!(inserted.committed_vnode.as_ref(), Some(&target));

    let old = VNode::root().children([
        VNode::box_node().with_key("keep"),
        VNode::text("old positional"),
    ]);
    let target = VNode::root().children([VNode::box_node().with_key("keep"), VNode::box_node()]);
    let stale_payload = VNode::box_node();
    assert_eq!(stale_payload.key.index, 0);
    let mut replaced = LayoutEngine::new();
    replaced.compute_vnode(&old, 20, 4);
    assert_eq!(
        replaced.try_apply_patches_checked(&[Patch::replace(old.children[1].key, stale_payload,)]),
        Ok(true)
    );
    assert_eq!(replaced.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn mixed_positional_removals_keep_stable_survivors() {
    let keep = VNode::box_node().with_key("keep");
    let old = VNode::root().children([
        VNode::box_node(),
        VNode::text("remove-one"),
        keep.clone(),
        VNode::box_node(),
    ]);
    let target = VNode::root().children([VNode::box_node(), keep]);
    let patches = try_diff(&old, &target).expect("mixed removal diff is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);

    assert_eq!(engine.try_apply_patches_checked(&patches), Ok(true));
    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn second_tail_remove_failure_keeps_its_backend_ordinal() {
    let old = VNode::root().children([VNode::box_node(), VNode::box_node(), VNode::box_node()]);
    let target = VNode::root().child(VNode::box_node());
    let patches = try_diff(&old, &target).expect("tail removal diff is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let before = EngineFingerprint::capture(&engine);
    super::super::incremental::set_incremental_fault_at(
        super::super::incremental::IncrementalFault::Remove,
        1,
    );

    let error = engine
        .try_apply_patches_transactional(&patches)
        .expect_err("second backend removal is faulted");
    let diagnostic = format!("{error:?}");
    assert!(
        matches!(
            error,
            TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
                if error.patch_index == Some(1)
                    && error.kind == IncrementalPatchKind::Remove
                    && error.stage == PatchStage::RemoveNode
                    && matches!(*error.source, PatchTransactionCause::Taffy(_))
        ),
        "{diagnostic}"
    );
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

fn recovered_failure(
    before: &Element,
    after: &Element,
    fault: super::super::incremental::IncrementalFault,
) -> crate::layout::PatchTransactionError {
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(before, None, 20, 4);
    super::super::incremental::set_incremental_fault_at(fault, 1);
    let (_, report) = engine
        .try_compute_element_incremental_transactional(after, Some(&previous), 20, 4)
        .expect("one-shot second operation fault recovers");
    match report {
        CheckedIncrementalLayoutReport::RecoveredFullRebuild {
            incremental_failure,
            ..
        } => incremental_failure,
        other => panic!("expected recovery report, got {other:?}"),
    }
}

#[test]
pub(crate) fn target_aware_second_create_and_update_have_exact_ordinals() {
    let empty = Element::root();
    let mut created = Element::root();
    created.add_child(Element::box_element().with_key("a"));
    created.add_child(Element::box_element().with_key("b"));
    let create = recovered_failure(
        &empty,
        &created,
        super::super::incremental::IncrementalFault::CreateBox,
    );
    assert_eq!(
        (create.patch_index, create.kind),
        (Some(1), IncrementalPatchKind::Create)
    );
    assert!(matches!(*create.source, PatchTransactionCause::Taffy(_)));

    let mut before = Element::root();
    let mut after = Element::root();
    for key in ["a", "b"] {
        let mut old = Element::box_element().with_key(key);
        old.style.width = Dimension::Points(1.0);
        before.add_child(old);
        let mut new = Element::box_element().with_key(key);
        new.style.width = Dimension::Points(2.0);
        after.add_child(new);
    }
    let update = recovered_failure(
        &before,
        &after,
        super::super::incremental::IncrementalFault::UpdateStyle,
    );
    assert_eq!(
        (update.patch_index, update.kind),
        (Some(1), IncrementalPatchKind::Update)
    );
    assert_eq!(update.stage, PatchStage::SetStyle);
    assert!(matches!(*update.source, PatchTransactionCause::Taffy(_)));
}

#[test]
fn post_compute_readback_has_its_own_stage_and_rebuild_keeps_taffy_cause() {
    let mut frame = Element::root();
    frame.add_child(Element::text("readback").with_key("text"));
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&frame, None, 20, 4);
    super::super::context_sync::set_layout_read_back_fault();
    super::super::incremental::set_incremental_fault(
        super::super::incremental::IncrementalFault::CreateBox,
    );

    let error = engine
        .try_compute_element_incremental_transactional(&frame, Some(&previous), 21, 4)
        .expect_err("readback and rebuild faults are both retained");
    assert!(matches!(
        error,
        TransactionalLayoutError::RecoveryFailed { incremental, rebuild }
            if incremental.patch_index.is_none()
                && incremental.kind == IncrementalPatchKind::Recompute
                && incremental.stage == PatchStage::ReadBack
                && matches!(*incremental.source, PatchTransactionCause::Taffy(_))
                && matches!(rebuild.source, RebuildFailure::Taffy(_))
    ));
}

#[test]
fn create_batch_compute_failure_is_recompute_but_legacy_fails_loudly() {
    let root = VNode::root();
    let created = VNode::box_node().with_key("created");
    let patch = Patch::create(created, root.key);
    let mut transactional = LayoutEngine::new();
    transactional.compute_vnode(&root, 20, 4);
    super::super::context_sync::set_layout_compute_fault();

    let error = transactional
        .try_apply_patches_transactional(std::slice::from_ref(&patch))
        .expect_err("candidate layout fault is typed");
    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
                if error.patch_index.is_none()
                && error.kind == IncrementalPatchKind::Recompute
                && error.key == Some(root.key)
                && error.parent.is_none()
                && error.stage == PatchStage::ComputeLayout
    ));

    let mut legacy = LayoutEngine::new();
    legacy.compute_vnode(&root, 20, 4);
    super::super::context_sync::set_layout_compute_fault();
    assert!(matches!(
        legacy.try_apply_patches_checked(&[patch]),
        Err(DirectPatchError::Patch(PatchError {
            kind: PatchKind::Update,
            key,
            failure: PatchFailure::LayoutFailed,
        })) if key == NodeKey::root()
    ));
}
