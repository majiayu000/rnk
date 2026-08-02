use super::{EngineFingerprint, LayoutEngine, exact_key_props, width_props};
use crate::core::{Element, ElementType, Props, VNode};
use crate::layout::{
    CheckedIncrementalLayoutReport, DirectPatchError, DirectPatchPreflightCause,
    IncrementalPatchKind, PatchStage, PatchTransactionCause, RebuildFailure,
    TransactionalLayoutError,
};
use crate::reconciler::{Patch, ScopedNodeIdentity, try_diff};

#[test]
fn duplicate_grouped_remove_is_a_dependency_error_not_a_second_delete() {
    let tree = VNode::root().children([
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node().with_key("keep"),
    ]);
    let key = tree.children[1].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = engine
        .try_apply_patches_transactional(&[Patch::remove(key), Patch::remove(key)])
        .expect_err("the second remove depends on the first");
    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 1
                && matches!(*error.source, DirectPatchPreflightCause::DependencyRemoved { prior_patch_index: 0 })
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn props_only_create_ignores_an_existing_raw_slot_alias() {
    let tree = VNode::root().child(VNode::box_node());
    let created = VNode::box_node().with_props(exact_key_props("canonical-new"));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    assert_eq!(
        engine.try_apply_patches_checked(&[Patch::create(created, tree.key)]),
        Ok(true)
    );
    assert_eq!(engine.committed_vnode.as_ref().unwrap().children.len(), 2);
}

#[test]
fn descendant_create_failure_inherits_the_public_create_ordinal() {
    let before = Element::root();
    let mut outer = Element::box_element().with_key("outer");
    outer.add_child(Element::box_element());
    let mut after = Element::root();
    after.add_child(outer);
    let failure = super::recovered_failure(
        &before,
        &after,
        super::super::super::incremental::IncrementalFault::CreateBox,
    );

    assert_eq!(failure.patch_index, Some(0));
    assert_eq!(failure.kind, IncrementalPatchKind::Create);
    assert!(matches!(*failure.source, PatchTransactionCause::Taffy(_)));
}

#[test]
fn postcondition_backend_reads_keep_taffy_causes_in_candidate_and_rebuild() {
    let mut old = Element::root();
    old.add_child(Element::box_element().with_key("a"));
    old.add_child(Element::box_element().with_key("b"));
    let mut target = Element::root();
    target.add_child(Element::box_element().with_key("b"));
    target.add_child(Element::box_element().with_key("a"));
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&old, None, 20, 4);
    super::super::super::incremental_order::set_incremental_order_fault(
        super::super::super::incremental_order::IncrementalOrderFault::PostconditionChildren,
    );
    let (_, report) = engine
        .try_compute_element_incremental_transactional(&target, Some(&previous), 20, 4)
        .expect("one-shot postcondition fault recovers");
    assert!(matches!(
        report,
        CheckedIncrementalLayoutReport::RecoveredFullRebuild { incremental_failure, .. }
            if incremental_failure.patch_index == Some(0)
                && incremental_failure.stage == PatchStage::VerifyPostcondition
                && matches!(*incremental_failure.source, PatchTransactionCause::Taffy(_))
    ));

    let mut frame = Element::root();
    frame.add_child(Element::text("readback").with_key("text"));
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&frame, None, 20, 4);
    super::super::super::incremental_order::set_incremental_order_fault_at(
        super::super::super::incremental_order::IncrementalOrderFault::PostconditionChildren,
        2,
    );
    super::super::super::context_sync::set_layout_read_back_fault();
    let error = engine
        .try_compute_element_incremental_transactional(&frame, Some(&previous), 21, 4)
        .expect_err("rebuild postcondition failure is concrete");
    assert!(matches!(
        error,
        TransactionalLayoutError::RecoveryFailed { incremental, rebuild }
            if incremental.stage == PatchStage::ReadBack
                && matches!(rebuild.source, RebuildFailure::Taffy(_))
    ));
}

#[test]
fn canonical_mixed_create_remove_batch_preserves_raw_scoped_addresses() {
    let old = VNode::root().child(VNode::box_node().with_key("a"));
    let target = VNode::root().children([VNode::box_node().with_key("b"), VNode::box_node()]);
    let patches = try_diff(&old, &target).expect("canonical diff");
    assert!(matches!(
        patches.as_slice(),
        [
            Patch::Create { .. },
            Patch::Create { .. },
            Patch::Remove { .. }
        ]
    ));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);

    engine
        .try_apply_patches_transactional(&patches)
        .expect("every canonical diff batch must be directly applicable");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn positional_create_requires_zero_prospective_matches() {
    let tree = VNode::root().child(VNode::box_node());
    let colliding = VNode::box_node().with_index(0);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let before = EngineFingerprint::capture(&engine);

    let error = engine
        .try_apply_patches_transactional(&[Patch::create(colliding, tree.key)])
        .expect_err("a positional create cannot replace an existing scoped identity");

    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 0
                && error.kind == IncrementalPatchKind::Create
                && matches!(*error.source, DirectPatchPreflightCause::AlreadyExists)
    ));
    assert_eq!(EngineFingerprint::capture(&engine), before);
}

#[test]
fn keyed_create_uses_the_declared_target_slot() {
    let old = VNode::root().children([
        VNode::box_node().with_key("a"),
        VNode::box_node().with_key("c"),
    ]);
    let created = VNode::box_node().with_key("b").with_index(1);
    let target = VNode::root().children([
        VNode::box_node().with_key("a"),
        VNode::box_node().with_key("b"),
        VNode::box_node().with_key("c"),
    ]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);

    engine
        .try_apply_patches_transactional(&[Patch::create(created, old.key)])
        .expect("keyed create at a valid target slot");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn duplicate_keys_inside_create_and_replace_subtrees_are_subtree_collisions() {
    fn duplicate_subtree(key: &str) -> VNode {
        VNode::box_node().with_key(key).children([
            VNode::box_node().with_props(Props::new().key("duplicate")),
            VNode::box_node().with_props(Props::new().key("duplicate")),
        ])
    }

    let empty = VNode::root();
    let mut create_engine = LayoutEngine::new();
    create_engine.compute_vnode(&empty, 20, 4);
    let create_error = create_engine
        .try_apply_patches_transactional(&[Patch::create(duplicate_subtree("new"), empty.key)])
        .expect_err("duplicate create subtree");
    assert!(matches!(
        create_error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 0
                && error.kind == IncrementalPatchKind::Create
                && matches!(*error.source, DirectPatchPreflightCause::SubtreeCollision { .. })
    ));

    let old_child = VNode::box_node().with_key("old");
    let old_key = old_child.key;
    let old = VNode::root().child(old_child);
    let mut replace_engine = LayoutEngine::new();
    replace_engine.compute_vnode(&old, 20, 4);
    let replace_error = replace_engine
        .try_apply_patches_transactional(&[Patch::replace(old_key, duplicate_subtree("new"))])
        .expect_err("duplicate replace subtree");
    assert!(matches!(
        replace_error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 0
                && error.kind == IncrementalPatchKind::Replace
                && matches!(*error.source, DirectPatchPreflightCause::SubtreeCollision { .. })
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoundedSibling {
    PositionalBox,
    PositionalText,
    KeyA,
    KeyB,
}

fn bounded_lists() -> Vec<Vec<BoundedSibling>> {
    let alphabet = [
        BoundedSibling::PositionalBox,
        BoundedSibling::PositionalText,
        BoundedSibling::KeyA,
        BoundedSibling::KeyB,
    ];
    let mut lists = Vec::new();
    for length in 0usize..=3 {
        for mut encoded in 0..alphabet.len().pow(length as u32) {
            let mut list = Vec::with_capacity(length);
            for _ in 0..length {
                list.push(alphabet[encoded % alphabet.len()]);
                encoded /= alphabet.len();
            }
            if [BoundedSibling::KeyA, BoundedSibling::KeyB]
                .into_iter()
                .all(|key| list.iter().filter(|item| **item == key).count() <= 1)
            {
                lists.push(list);
            }
        }
    }
    lists
}

fn bounded_tree(items: &[BoundedSibling]) -> VNode {
    VNode::root().children(items.iter().map(|item| match item {
        BoundedSibling::PositionalBox => VNode::box_node(),
        BoundedSibling::PositionalText => VNode::text("positional"),
        BoundedSibling::KeyA => VNode::box_node().with_key("a"),
        BoundedSibling::KeyB => VNode::text("keyed").with_key("b"),
    }))
}

#[test]
fn canonical_try_diff_batches_apply_to_exact_target_bounded() {
    let lists = bounded_lists();
    assert_eq!(lists.len(), 63);
    let mut checked = 0usize;
    for before in &lists {
        for after in &lists {
            let old = bounded_tree(before);
            let target = bounded_tree(after);
            let patches = try_diff(&old, &target).unwrap_or_else(|error| {
                panic!("valid bounded diff failed: {before:?} -> {after:?}: {error}")
            });
            let mut engine = LayoutEngine::new();
            engine.compute_vnode(&old, 20, 8);
            engine
                .try_apply_patches_transactional(&patches)
                .unwrap_or_else(|error| {
                    panic!(
                        "canonical batch rejected: {before:?} -> {after:?}, patches={patches:?}: {error:?}"
                    )
                });
            assert_eq!(
                engine.committed_vnode.as_ref(),
                Some(&target),
                "canonical result mismatch: {before:?} -> {after:?}, patches={patches:?}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 3_969);
}

#[test]
fn canonical_high_slot_create_survives_later_lower_slot_removals() {
    let old = VNode::root().children([
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
    ]);
    let target = VNode::root().children([
        VNode::box_node().with_key("a"),
        VNode::box_node().with_key("b"),
        VNode::box_node(),
        VNode::box_node(),
        VNode::text("positional"),
    ]);
    let patches = try_diff(&old, &target).expect("valid canonical diff");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 8);

    engine
        .try_apply_patches_transactional(&patches)
        .expect("the created generation keeps its target-slot alias");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn canonical_positional_creates_resolve_create_to_create_dependencies() {
    let old = VNode::root().children([
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node().with_key("a"),
        VNode::box_node(),
    ]);
    let target = VNode::root().children([
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node().with_key("a"),
    ]);
    let patches = try_diff(&old, &target).expect("valid canonical diff");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 8);

    engine
        .try_apply_patches_transactional(&patches)
        .expect("a higher positional create can precede a lower create");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn canonical_reorder_keeps_positional_generations_after_an_earlier_removal() {
    let old = VNode::root().children([
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node(),
        VNode::box_node().with_key("a"),
    ]);
    let target = VNode::root().children([
        VNode::box_node(),
        VNode::box_node().with_key("a"),
        VNode::box_node(),
        VNode::box_node(),
    ]);
    let patches = try_diff(&old, &target).expect("valid canonical diff");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 8);

    engine
        .try_apply_patches_transactional(&patches)
        .expect("scoped reorder entries preserve the surviving positional generations");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn canonical_reorder_uses_target_alias_for_a_positional_replacement() {
    let old = VNode::root().children([VNode::box_node(), VNode::box_node()]);
    let target = VNode::root().children([
        VNode::box_node().with_key("a"),
        VNode::text("replacement"),
        VNode::box_node().with_key("b"),
    ]);
    let patches = try_diff(&old, &target).expect("valid canonical diff");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 8);

    engine
        .try_apply_patches_transactional(&patches)
        .expect("a replaced generation remains addressable at its target ordinal");

    assert_eq!(engine.committed_vnode.as_ref(), Some(&target));
}

#[test]
fn batch_created_raw_alias_survives_a_later_sibling_insertion() {
    let root = VNode::root();
    let created = VNode::box_node().with_props(width_props(1.0));
    let created_key = created.key;
    let inserted = VNode::box_node().with_key("inserted");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    engine
        .try_apply_patches_transactional(&[
            Patch::create(created.clone(), root.key),
            Patch::create(inserted.clone(), root.key),
            Patch::update(created_key, created.props, width_props(9.0)),
        ])
        .expect("the create ordinal keeps a stable raw alias");

    let committed = engine.committed_vnode.as_ref().expect("batch committed");
    assert_eq!(
        committed.children[0].key.identity(),
        inserted.key.identity()
    );
    assert_eq!(committed.children[1].props, width_props(9.0));
}

#[test]
fn batch_created_parent_raw_alias_survives_a_later_sibling_insertion() {
    let root = VNode::root();
    let parent = VNode::box_node();
    let parent_key = parent.key;
    let inserted = VNode::box_node().with_key("inserted");
    let child = VNode::text("child").with_key("child");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    engine
        .try_apply_patches_transactional(&[
            Patch::create(parent, root.key),
            Patch::create(inserted, root.key),
            Patch::create(child, parent_key),
        ])
        .expect("a batch-created parent remains addressable by its declared key");

    assert_eq!(
        engine.committed_vnode.as_ref().unwrap().children[1]
            .children
            .len(),
        1
    );
}

#[test]
fn removed_batch_created_alias_reports_its_dependency_ordinal() {
    let root = VNode::root();
    let created = VNode::box_node();
    let created_key = created.key;
    let inserted = VNode::box_node().with_key("inserted");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    let error = engine
        .try_apply_patches_transactional(&[
            Patch::create(created, root.key),
            Patch::create(inserted, root.key),
            Patch::remove(created_key),
            Patch::update(created_key, Props::new(), width_props(9.0)),
        ])
        .expect_err("the removed create alias becomes a tombstone");

    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 3
                && matches!(*error.source, DirectPatchPreflightCause::DependencyRemoved { prior_patch_index: 2 })
    ));
}

#[test]
fn final_plan_preflight_failure_uses_the_first_invalid_prefix_ordinal() {
    let child = VNode::box_node().with_key("child");
    let child_key = child.key;
    let root = VNode::root().child(child);
    let mut root_props = root.props.clone();
    root_props.style.width = crate::core::Dimension::Points(9.0);
    let mut child_props = root.children[0].props.clone();
    child_props.style.width = crate::core::Dimension::Points(7.0);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    super::super::super::incremental_order::set_incremental_order_fault_at(
        super::super::super::incremental_order::IncrementalOrderFault::PreflightStyle,
        0,
    );

    let error = engine
        .try_apply_patches_transactional(&[
            Patch::update(root.key, root.props.clone(), root_props),
            Patch::update(child_key, root.children[0].props.clone(), child_props),
        ])
        .expect_err("final-plan root preflight fault");
    let diagnostic = format!("{error:?}");

    assert!(
        matches!(
            error,
            TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
                if error.patch_index == 0
                    && error.kind == IncrementalPatchKind::Update
                    && error.key == Some(root.key)
                    && matches!(*error.source, DirectPatchPreflightCause::Identity(_))
        ),
        "{diagnostic}"
    );
}

#[test]
fn structural_change_does_not_authorize_positional_survivor_swap() {
    let root = VNode::root().children([
        VNode::box_node().with_props(width_props(1.0)),
        VNode::box_node().with_props(width_props(2.0)),
    ]);
    let tail = VNode::box_node().with_key("tail").with_index(2);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    let error = engine
        .try_apply_patches_transactional(&[
            Patch::create(tail.clone(), root.key),
            Patch::reorder(
                root.key,
                vec![root.children[1].key, root.children[0].key, tail.key],
            ),
        ])
        .expect_err("committed positional identities cannot be swapped");

    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(error))
            if error.patch_index == 1
                && error.kind == IncrementalPatchKind::Reorder
                && error.key == Some(root.children[1].key)
                && error.parent.is_some()
                && matches!(
                    *error.source,
                    DirectPatchPreflightCause::InvalidReorderMove {
                        from: 1,
                        to: 0,
                        child_count: 3,
                    }
                )
    ));
}

#[test]
pub(crate) fn batch_compute_readback_postcondition_use_recompute_locator() {
    fn assert_batch(
        error: TransactionalLayoutError,
        stage: PatchStage,
        expected_key: Option<crate::core::NodeKey>,
    ) {
        let diagnostic = format!("expected key {expected_key:?}, got {error:?}");
        assert!(
            matches!(
                error,
                TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
                    if error.patch_index.is_none()
                        && error.kind == IncrementalPatchKind::Recompute
                        && error.key == expected_key
                        && error.parent.is_none()
                        && error.stage == stage
            ),
            "{diagnostic}"
        );
    }

    let removed = VNode::box_node().with_key("removed");
    let removed_key = removed.key;
    let kept = VNode::text("kept").with_key("kept");
    let kept_key = kept.key;
    let tree = VNode::root().children([removed, kept]);

    let mut compute = LayoutEngine::new();
    compute.compute_vnode(&tree, 20, 4);
    super::super::super::context_sync::set_layout_compute_fault();
    assert_batch(
        compute
            .try_apply_patches_transactional(&[Patch::remove(removed_key)])
            .expect_err("batch compute fault"),
        PatchStage::ComputeLayout,
        Some(tree.key),
    );

    let mut readback = LayoutEngine::new();
    readback.compute_vnode(&tree, 20, 4);
    super::super::super::context_sync::set_layout_read_back_fault();
    assert_batch(
        readback
            .try_apply_patches_transactional(&[Patch::remove(removed_key)])
            .expect_err("batch readback fault"),
        PatchStage::ReadBack,
        Some(kept_key),
    );

    let mut postcondition = LayoutEngine::new();
    postcondition.compute_vnode(&tree, 20, 4);
    super::super::super::postcondition::set_postcondition_fault(
        super::super::super::postcondition::PostconditionFault::ScopedMapMismatch,
    );
    assert_batch(
        postcondition
            .try_apply_patches_transactional(&[Patch::remove(removed_key)])
            .expect_err("batch postcondition fault"),
        PatchStage::VerifyPostcondition,
        Some(tree.key),
    );
}

#[test]
fn created_text_readback_failure_keeps_the_create_locator() {
    let root = VNode::root();
    let target = VNode::root().child(VNode::text("created").with_key("created"));
    let patches = try_diff(&root, &target).expect("canonical create diff");
    assert!(matches!(patches.as_slice(), [Patch::Create { .. }]));
    let child_key = target.children[0].key;
    let parent = ScopedNodeIdentity::Root.scoped_patch_address(root.key);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    super::super::super::context_sync::set_layout_read_back_fault();
    let error = engine
        .try_apply_patches_transactional(&patches)
        .expect_err("created text readback fault");
    let diagnostic = format!("{error:?}");
    assert!(
        matches!(
            error,
            TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
                if error.patch_index == Some(0)
                    && error.kind == IncrementalPatchKind::Create
                    && error.key == Some(child_key)
                    && error.parent == Some(parent)
                    && error.stage == PatchStage::ReadBack
                    && matches!(*error.source, PatchTransactionCause::Taffy(_))
        ),
        "{diagnostic}"
    );
}

#[test]
fn no_op_reorder_keeps_later_layout_failure_locators_aligned() {
    let root = VNode::root();
    let created = VNode::text("created").with_key("created");
    let child_key = created.key;
    let parent = ScopedNodeIdentity::Root.scoped_patch_address(root.key);
    let patches = [
        Patch::reorder(root.key, Vec::new()),
        Patch::create(created, root.key),
    ];
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);

    super::super::super::context_sync::set_layout_read_back_fault();
    let error = engine
        .try_apply_patches_transactional(&patches)
        .expect_err("created text readback fault");

    assert!(matches!(
        error,
        TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
            if error.patch_index == Some(1)
                && error.kind == IncrementalPatchKind::Create
                && error.key == Some(child_key)
                && error.parent == Some(parent)
                && error.stage == PatchStage::ReadBack
    ));
}

#[test]
fn remove_descendant_cleanup_uses_ancestor_patch_locator() {
    let mut branch = Element::box_element().with_key("branch");
    let mut middle = Element::box_element().with_key("middle");
    middle.add_child(Element::box_element().with_key("leaf"));
    branch.add_child(middle);
    let mut before = Element::root();
    before.add_child(branch);
    let after = Element::root();
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&before, None, 20, 4);
    let expected_parent = ScopedNodeIdentity::Root.scoped_patch_address(previous.key);
    super::super::super::incremental::set_incremental_fault_at(
        super::super::super::incremental::IncrementalFault::Remove,
        1,
    );

    let (_, report) = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 4)
        .expect("one descendant cleanup fault recovers through the single rebuild");
    let CheckedIncrementalLayoutReport::RecoveredFullRebuild {
        incremental_failure,
        ..
    } = report
    else {
        panic!("expected recovered report, got {report:?}");
    };
    let diagnostic = format!("{incremental_failure:?}");
    assert!(
        incremental_failure.patch_index == Some(0)
            && incremental_failure.kind == IncrementalPatchKind::Remove
            && incremental_failure.key.is_some()
            && incremental_failure.parent == Some(expected_parent)
            && incremental_failure.stage == PatchStage::RemoveNode,
        "{diagnostic}"
    );
}

#[test]
fn virtual_text_root_is_invalid_initial_target() {
    let target = Element::new(ElementType::VirtualText);
    let mut engine = LayoutEngine::new();

    let error = engine
        .try_compute_element_incremental_transactional(&target, None, 20, 4)
        .expect_err("VirtualText cannot be a layout root");

    assert!(matches!(
        error,
        TransactionalLayoutError::InitialBuild(error)
            if error.key.is_none()
                && matches!(error.source, RebuildFailure::InvalidTargetRoot)
    ));
    assert!(!engine.has_tree());
    assert_eq!(engine.node_count(), 0);
}
