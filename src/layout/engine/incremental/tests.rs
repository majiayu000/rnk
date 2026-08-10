use super::super::RebuildFailure;
use super::*;
use crate::core::Dimension;
use crate::layout::{
    CheckedIncrementalLayoutReport, IncrementalPatchKind, TransactionalLayoutError,
};
use crate::reconciler::plan_diff;

fn keyed_tree(order: &[&str]) -> Element {
    let mut root = Element::root();
    for key in order {
        let mut parent = Element::box_element().with_key(*key);
        let mut inner = Element::box_element().with_key("inner");
        inner.add_child(Element::text(format!("{key}-leaf")));
        parent.add_child(inner);
        root.add_child(parent);
    }
    root
}

fn node_id(engine: &LayoutEngine, element: &Element) -> NodeId {
    let identity = engine
        .element_scopes
        .get(&element.id)
        .expect("current element must have a scoped identity");
    engine.vnode_map[identity]
}

fn keyed_vnode_identity(engine: &LayoutEngine, key: NodeKey) -> ScopedNodeIdentity {
    engine
        .vnode_legacy_keys
        .iter()
        .find_map(|(identity, candidate)| candidate.matches(&key).then(|| identity.clone()))
        .expect("fixture key has one scoped identity")
}

fn assert_fault_falls_back_atomically(
    before: &Element,
    after: &Element,
    fault: IncrementalFault,
    kind: PatchKind,
    failure: PatchFailure,
) {
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(before, None, 40, 10);
    let previous_root = engine.root_node;
    set_incremental_fault(fault);

    let (current, outcome) = engine
        .try_compute_element_incremental_checked(after, Some(&previous), 40, 10)
        .expect("one-shot commit fault must recover through the checked caller");

    assert!(outcome.used_reconciler);
    assert!(outcome.fallback_full_rebuild);
    assert_eq!(
        outcome.patch_error,
        Some(PatchError {
            kind,
            key: outcome.patch_error.expect("fault is reported").key,
            failure,
        })
    );
    assert_eq!(engine.committed_vnode.as_ref(), Some(&current));
    assert_eq!(engine.taffy.total_node_count(), engine.vnode_map.len());
    assert_ne!(engine.root_node, None);
    assert_ne!(previous_root, None);
    for element in after.children.iter() {
        assert!(engine.get_layout(element.id).is_some());
    }
}

#[test]
pub(crate) fn keyed_ancestor_reorder_preserves_descendant_identity() {
    let mut engine = LayoutEngine::new();
    let first = keyed_tree(&["left", "right"]);
    let (previous, _) = engine.compute_element_incremental(&first, None, 40, 10);
    let first_nodes: HashMap<_, _> = first
        .children
        .iter()
        .map(|parent| {
            let inner = parent.children.iter().next().expect("one inner child");
            let leaf = inner.children.iter().next().expect("one leaf child");
            (
                parent.key.clone().expect("fixture parent is keyed"),
                (
                    node_id(&engine, parent),
                    node_id(&engine, inner),
                    node_id(&engine, leaf),
                ),
            )
        })
        .collect();
    assert_ne!(first_nodes["left"].1, first_nodes["right"].1);

    let second = keyed_tree(&["right", "left"]);
    let (_, outcome) = engine.compute_element_incremental(&second, Some(&previous), 40, 10);
    assert!(!outcome.fallback_full_rebuild);
    for parent in &second.children {
        let key = parent.key.as_deref().expect("fixture parent is keyed");
        let inner = parent.children.iter().next().expect("one inner child");
        let leaf = inner.children.iter().next().expect("one leaf child");
        assert_eq!(
            (
                node_id(&engine, parent),
                node_id(&engine, inner),
                node_id(&engine, leaf),
            ),
            first_nodes[key],
            "moving keyed ancestor {key:?} must preserve every descendant NodeId"
        );
    }

    let root_node = engine.root_node.expect("incremental tree has a root");
    let actual_order = engine
        .taffy
        .children(root_node)
        .expect("root remains in the Taffy tree");
    let expected_order: Vec<_> = second
        .children
        .iter()
        .map(|child| node_id(&engine, child))
        .collect();
    assert_eq!(actual_order, expected_order);
}

#[test]
pub(crate) fn cross_parent_move_cleans_old_scope_without_deleting_new_scope() {
    fn tree(moved_on_left: bool) -> Element {
        let mut moved = Element::box_element().with_key("moved");
        moved.add_child(Element::text("leaf"));
        let mut left = Element::box_element().with_key("left");
        let mut right = Element::box_element().with_key("right");
        if moved_on_left {
            left.add_child(moved);
        } else {
            right.add_child(moved);
        }
        let mut root = Element::root();
        root.add_child(left);
        root.add_child(right);
        root
    }

    let mut engine = LayoutEngine::new();
    let first = tree(true);
    let first_left = first.children.iter().next().expect("left parent");
    let first_right = first.children.iter().nth(1).expect("right parent");
    let old_moved = first_left.children.iter().next().expect("moved subtree");
    let old_leaf = old_moved.children.iter().next().expect("moved leaf");
    let (previous, _) = engine.compute_element_incremental(&first, None, 40, 10);
    let old_parent_nodes = (node_id(&engine, first_left), node_id(&engine, first_right));
    let old_scopes = [
        engine.element_scopes[&old_moved.id].clone(),
        engine.element_scopes[&old_leaf.id].clone(),
    ];
    let old_moved_node = node_id(&engine, old_moved);

    let second = tree(false);
    let second_left = second.children.iter().next().expect("left parent");
    let second_right = second.children.iter().nth(1).expect("right parent");
    let new_moved = second_right.children.iter().next().expect("moved subtree");
    let (_, outcome) = engine.compute_element_incremental(&second, Some(&previous), 40, 10);
    assert!(!outcome.fallback_full_rebuild);
    assert_eq!(
        (
            node_id(&engine, second_left),
            node_id(&engine, second_right),
        ),
        old_parent_nodes
    );

    let new_scope = engine.element_scopes[&new_moved.id].clone();
    assert_ne!(new_scope, old_scopes[0]);
    assert_ne!(node_id(&engine, new_moved), old_moved_node);
    for scope in &old_scopes {
        assert!(!engine.vnode_map.contains_key(scope));
        assert!(!engine.vnode_legacy_keys.contains_key(scope));
    }
    for old_element_id in [old_moved.id, old_leaf.id] {
        assert!(!engine.node_map.contains_key(&old_element_id));
        assert!(!engine.element_keys.contains_key(&old_element_id));
        assert!(!engine.element_scopes.contains_key(&old_element_id));
    }
    assert!(engine.vnode_map.contains_key(&new_scope));
    assert_eq!(engine.taffy.total_node_count(), engine.vnode_map.len());
}

#[test]
fn virtual_text_root_and_child_are_absent_from_layout_snapshot() {
    let virtual_root = Element::new(ElementType::VirtualText);
    let snapshot =
        ElementVNodeSnapshot::from_element(&virtual_root, &mut ScopedIdentityArena::default())
            .expect("virtual root conversion is valid");
    assert_eq!(snapshot.vnode, VNode::root());
    assert!(snapshot.element_scopes.is_empty());
    assert!(snapshot.element_keys.is_empty());
    assert!(snapshot.text_inputs.is_empty());

    let mut root = Element::root();
    root.add_child(Element::new(ElementType::VirtualText));
    root.add_child(Element::text("visible").with_key("visible"));
    let snapshot = ElementVNodeSnapshot::from_element(&root, &mut ScopedIdentityArena::default())
        .expect("virtual child is skipped without consuming an index");
    assert_eq!(snapshot.vnode.children.len(), 1);
    assert_eq!(snapshot.vnode.children[0].key.index, 0);
}

#[test]
fn element_conversion_covers_root_box_text_keys_scroll_and_text_input() {
    let mut root = Element::box_element().with_key("ignored-root-key");
    root.scroll_offset_x = Some(2);
    root.scroll_offset_y = Some(3);
    root.add_child(Element::box_element());
    root.add_child(Element::text("text").with_key("text"));
    let snapshot = ElementVNodeSnapshot::from_element(&root, &mut ScopedIdentityArena::default())
        .expect("all concrete element variants convert");

    assert!(matches!(snapshot.vnode.node_type, VNodeType::Box));
    assert_eq!(snapshot.vnode.props.scroll_offset_x, Some(2));
    assert_eq!(snapshot.vnode.props.scroll_offset_y, Some(3));
    assert_eq!(snapshot.vnode.children.len(), 2);
    assert!(matches!(
        snapshot.vnode.children[0].node_type,
        VNodeType::Box
    ));
    assert!(matches!(
        snapshot.vnode.children[1].node_type,
        VNodeType::Text(_)
    ));
    assert!(
        snapshot
            .text_inputs
            .contains_key(&snapshot.element_scopes[&root.children.get(1).expect("text").id])
    );
}

#[test]
fn non_style_update_reuses_node_without_touching_style_or_context() {
    let old = VNode::box_node().child(VNode::box_node().with_key("child"));
    let mut new_child = VNode::box_node().with_key("child");
    new_child.props.scroll_offset_x = Some(1);
    let new = VNode::box_node().child(new_child);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let identity = keyed_vnode_identity(&engine, old.children[0].key);
    let node = engine.vnode_map[&identity];
    let style = engine.taffy.style(node).expect("node exists").clone();
    let had_context = engine.taffy.get_node_context(node).is_some();

    let plan = plan_diff(&old, &new).expect("fixture plan is valid");
    engine
        .apply_reconcile_plan(&plan)
        .expect("non-style update applies");

    assert_eq!(engine.vnode_map[&identity], node);
    assert_eq!(engine.taffy.style(node).expect("node survives"), &style);
    assert_eq!(engine.taffy.get_node_context(node).is_some(), had_context);
}

#[test]
fn reuse_does_not_rewrite_style_or_context() {
    let tree = VNode::box_node().child(VNode::box_node().with_key("child"));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let child_identity = keyed_vnode_identity(&engine, tree.children[0].key);
    let child_node = engine.vnode_map[&child_identity];
    let mut sentinel_style = taffy::Style::default();
    sentinel_style.gap.width = taffy::LengthPercentage::Length(13.0);
    engine
        .taffy
        .set_style(child_node, sentinel_style.clone())
        .expect("fixture node remains writable");
    engine
        .taffy
        .set_node_context(child_node, None)
        .expect("fixture context remains writable");

    let plan = plan_diff(&tree, &tree).expect("fixture has a no-op plan");
    engine
        .apply_reconcile_plan(&plan)
        .expect("no-op plan remains applicable");

    assert_eq!(
        engine.taffy.style(child_node).expect("node survives"),
        &sentinel_style
    );
    assert!(engine.taffy.get_node_context(child_node).is_none());
}

#[test]
fn matching_style_and_text_context_are_not_rewritten() {
    let old = VNode::box_node().child(VNode::text("same").with_key("text"));
    let mut next = VNode::text("same").with_key("text");
    next.props.style.width = Dimension::Points(5.0);
    let new = VNode::box_node().child(next);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let identity = keyed_vnode_identity(&engine, old.children[0].key);
    let node = engine.vnode_map[&identity];
    let desired_style = normalized_taffy_style(&new.children[0].props.style, true);
    engine
        .taffy
        .set_style(node, desired_style.clone())
        .expect("fixture style is writable");
    let input = input_from_vnode(&new.children[0]);
    engine
        .taffy
        .set_node_context(
            node,
            Some(NodeContext::new(input, &engine.text_flow_policy)),
        )
        .expect("fixture context is writable");

    let plan = plan_diff(&old, &new).expect("fixture plan is valid");
    engine
        .apply_reconcile_plan(&plan)
        .expect("matching materialized state applies without rewrite");
    assert_eq!(
        engine.taffy.style(node).expect("node survives"),
        &desired_style
    );
}

#[test]
fn changed_style_and_text_context_are_written_by_the_production_operations() {
    let old = VNode::box_node().child(VNode::text("same").with_key("text"));
    let mut changed = VNode::text("same").with_key("text");
    changed.props.style.width = Dimension::Points(7.0);
    let new = VNode::box_node().child(changed);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let identity = keyed_vnode_identity(&engine, old.children[0].key);
    let node = engine.vnode_map[&identity];

    let plan = plan_diff(&old, &new).expect("fixture plan is valid");
    engine
        .apply_reconcile_plan(&plan)
        .expect("style and text context updates apply");

    let desired_style = normalized_taffy_style(&new.children[0].props.style, true);
    assert_eq!(
        engine.taffy.style(node).expect("node survives"),
        &desired_style
    );
    let input = input_from_vnode(&new.children[0]);
    assert!(input.as_ref().is_some_and(|input| {
        engine
            .taffy
            .get_node_context(node)
            .is_some_and(|context| context.matches(input, &engine.text_flow_policy))
    }));
}

#[test]
pub(crate) fn taffy_fault_seams_preserve_error_mapping_and_checked_caller_atomicity() {
    let empty = Element::root();
    let mut with_text = Element::root();
    with_text.add_child(Element::text("new").with_key("new"));
    assert_fault_falls_back_atomically(
        &empty,
        &with_text,
        IncrementalFault::CreateText,
        PatchKind::Create,
        PatchFailure::BuildFailed,
    );

    let mut with_box = Element::root();
    with_box.add_child(Element::box_element().with_key("new"));
    assert_fault_falls_back_atomically(
        &empty,
        &with_box,
        IncrementalFault::CreateBox,
        PatchKind::Create,
        PatchFailure::BuildFailed,
    );
    assert_fault_falls_back_atomically(
        &empty,
        &with_box,
        IncrementalFault::CreateBoxContext,
        PatchKind::Create,
        PatchFailure::BuildFailed,
    );

    let mut old_box = Element::root();
    old_box.add_child(Element::box_element().with_key("stable"));
    let mut changed_box = Element::root();
    let mut box_child = Element::box_element().with_key("stable");
    box_child.style.width = Dimension::Points(5.0);
    changed_box.add_child(box_child);
    assert_fault_falls_back_atomically(
        &old_box,
        &changed_box,
        IncrementalFault::UpdateStyle,
        PatchKind::Update,
        PatchFailure::TreeRejected,
    );

    let mut old_text = Element::root();
    old_text.add_child(Element::text("same").with_key("stable"));
    let mut changed_text = Element::root();
    let mut text_child = Element::text("same").with_key("stable");
    text_child.style.width = Dimension::Points(5.0);
    changed_text.add_child(text_child);
    assert_fault_falls_back_atomically(
        &old_text,
        &changed_text,
        IncrementalFault::UpdateTextContext,
        PatchKind::Update,
        PatchFailure::TreeRejected,
    );

    let mut with_removed = Element::root();
    with_removed.add_child(Element::box_element().with_key("remove"));
    assert_fault_falls_back_atomically(
        &with_removed,
        &empty,
        IncrementalFault::Remove,
        PatchKind::Remove,
        PatchFailure::TreeRejected,
    );
}

fn recovered_stage(before: &Element, after: &Element, fault: IncrementalFault) -> PatchStage {
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(before, None, 20, 4);
    set_incremental_fault(fault);
    let (_, report) = engine
        .try_compute_element_incremental_transactional(after, Some(&previous), 20, 4)
        .expect("one-shot fault recovers");
    match report {
        CheckedIncrementalLayoutReport::RecoveredFullRebuild {
            incremental_failure,
            ..
        } => incremental_failure.stage,
        other => panic!("expected recovery report, got {other:?}"),
    }
}

#[test]
pub(crate) fn context_faults_report_set_context_stage() {
    let empty = Element::root();
    let mut created = Element::root();
    created.add_child(Element::box_element().with_key("created"));
    assert_eq!(
        recovered_stage(&empty, &created, IncrementalFault::CreateBoxContext),
        PatchStage::SetContext
    );

    let mut old = Element::root();
    old.add_child(Element::text("same").with_key("stable"));
    let mut updated = Element::root();
    let mut text = Element::text("same").with_key("stable");
    text.style.width = Dimension::Points(5.0);
    updated.add_child(text);
    assert_eq!(
        recovered_stage(&old, &updated, IncrementalFault::UpdateTextContext),
        PatchStage::SetContext
    );
}

#[test]
fn corrupt_materialization_state_returns_exact_defensive_errors() {
    let tree = VNode::box_node().child(VNode::box_node().with_key("child"));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);

    let mut missing_old_identity = plan_diff(&tree, &tree).expect("fixture plan is valid");
    missing_old_identity.root.old_identity = None;
    assert!(matches!(
        engine
            .apply_reconcile_plan(&missing_old_identity)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Update,
            failure: PatchFailure::UnknownNode,
            ..
        })
    ));

    let mut missing_old_node_engine = engine.staged_clone();
    missing_old_node_engine.vnode_map.clear();
    assert!(matches!(
        missing_old_node_engine
            .apply_reconcile_plan(&plan_diff(&tree, &tree).unwrap())
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Update,
            failure: PatchFailure::UnknownNode,
            ..
        })
    ));

    let mut duplicate_target = plan_diff(&tree, &tree).expect("fixture plan is valid");
    duplicate_target.root.children[0].identity = ScopedNodeIdentity::Root;
    assert!(matches!(
        engine
            .apply_reconcile_plan(&duplicate_target)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Create,
            failure: PatchFailure::PostconditionViolated,
            ..
        })
    ));
}

#[test]
fn corrupt_taffy_node_count_fails_closed() {
    let new = VNode::box_node();
    let mut count_engine = LayoutEngine::new();
    count_engine.compute_vnode(&new, 20, 4);
    count_engine
        .taffy
        .new_leaf(taffy::Style::default())
        .expect("unmapped fixture node is allocated");
    assert!(matches!(
        count_engine
            .apply_reconcile_plan(&plan_diff(&new, &new).unwrap())
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Remove,
            failure: PatchFailure::PostconditionViolated,
            ..
        })
    ));
}

#[test]
fn reset_and_element_map_sync_cover_success_and_fail_loud_contracts() {
    let element = Element::box_element();
    let mut engine = LayoutEngine::new();
    let snapshot =
        ElementVNodeSnapshot::from_element(&element, &mut ScopedIdentityArena::default())
            .expect("fixture snapshot is valid");
    engine.compute_vnode(&snapshot.vnode, 20, 4);
    engine.sync_element_node_map_scoped(&snapshot);
    assert!(engine.node_map.contains_key(&element.id));

    let mut invalid = ElementVNodeSnapshot {
        vnode: VNode::root(),
        has_layout_root: true,
        element_scopes: HashMap::new(),
        element_keys: HashMap::new(),
        text_inputs: HashMap::new(),
    };
    invalid
        .element_scopes
        .insert(ElementId::new(), ScopedNodeIdentity::Root);
    engine.vnode_map.clear();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.sync_element_node_map_scoped(&invalid);
        }))
        .is_err()
    );

    engine.reset_scoped_vnode_tree();
    assert_eq!(engine.taffy.total_node_count(), 0);
    assert!(engine.node_map.is_empty());
    assert!(engine.element_keys.is_empty());
    assert!(engine.element_scopes.is_empty());
    assert!(engine.vnode_map.is_empty());
    assert!(engine.vnode_legacy_keys.is_empty());
    assert!(engine.root_node.is_none());
    assert!(engine.current_text_flows.is_empty());
    assert!(engine.current_vnode_flows.is_empty());
    assert!(engine.committed_vnode.is_none());
}

fn only_patch_key(plan: &ReconcilePlan, expected_kind: PatchKind) -> NodeKey {
    let matches: Vec<_> = plan
        .patches()
        .iter()
        .filter_map(|patch| match (expected_kind, patch) {
            (PatchKind::Create, crate::reconciler::Patch::Create { parent: key, .. })
            | (PatchKind::Update, crate::reconciler::Patch::Update { key, .. })
            | (PatchKind::Remove, crate::reconciler::Patch::Remove { key })
            | (PatchKind::Replace, crate::reconciler::Patch::Replace { key, .. }) => Some(*key),
            (PatchKind::Reorder, crate::reconciler::Patch::Reorder { parent, .. }) => Some(*parent),
            _ => None,
        })
        .collect();
    assert_eq!(matches.len(), 1, "fixture must contain one matching patch");
    matches[0]
}

#[test]
pub(crate) fn incremental_create_fault_reports_the_real_patch_locator() {
    let empty = VNode::root();
    let created = VNode::root().child(VNode::box_node().with_key("created"));
    let create_plan = plan_diff(&empty, &created).expect("create fixture plan");
    let mut create_engine = LayoutEngine::new();
    create_engine.compute_vnode(&empty, 20, 4);
    set_incremental_fault(IncrementalFault::CreateBox);
    assert_eq!(
        create_engine
            .apply_reconcile_plan(&create_plan)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Create,
            key: only_patch_key(&create_plan, PatchKind::Create),
            failure: PatchFailure::BuildFailed,
        })
    );
}

#[test]
pub(crate) fn incremental_replace_fault_reports_the_real_patch_locator() {
    let old = VNode::root().child(VNode::text("old").with_key("same"));
    let replaced = VNode::root().child(VNode::box_node().with_key("same"));
    let replace_plan = plan_diff(&old, &replaced).expect("replace fixture plan");
    let mut replace_engine = LayoutEngine::new();
    replace_engine.compute_vnode(&old, 20, 4);
    set_incremental_fault(IncrementalFault::CreateBox);
    assert_eq!(
        replace_engine
            .apply_reconcile_plan(&replace_plan)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Replace,
            key: only_patch_key(&replace_plan, PatchKind::Replace),
            failure: PatchFailure::BuildFailed,
        })
    );
}

#[test]
pub(crate) fn incremental_remove_fault_reports_the_real_patch_locator() {
    let removed = VNode::root().child(VNode::box_node().with_key("removed"));
    let remove_plan = plan_diff(&removed, &VNode::root()).expect("remove fixture plan");
    let mut remove_engine = LayoutEngine::new();
    remove_engine.compute_vnode(&removed, 20, 4);
    set_incremental_fault(IncrementalFault::Remove);
    assert_eq!(
        remove_engine
            .apply_reconcile_plan(&remove_plan)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Remove,
            key: only_patch_key(&remove_plan, PatchKind::Remove),
            failure: PatchFailure::TreeRejected,
        })
    );
}

#[test]
pub(crate) fn replacement_descendant_cleanup_fault_keeps_replace_root_locator() {
    let old = VNode::root().child(
        VNode::box_node()
            .with_key("branch")
            .child(VNode::box_node().child(VNode::text("leaf"))),
    );
    let target = VNode::root().child(VNode::text("replacement").with_key("branch"));
    let plan = plan_diff(&old, &target).expect("replacement fixture plan");
    let expected_key = only_patch_key(&plan, PatchKind::Replace);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    set_incremental_fault(IncrementalFault::Remove);

    assert_eq!(
        engine
            .apply_reconcile_plan(&plan)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Replace,
            key: expected_key,
            failure: PatchFailure::TreeRejected,
        })
    );
}

#[test]
fn persistent_recovery_failure_returns_both_typed_causes_and_keeps_committed_state() {
    let before = Element::root();
    let mut after = Element::root();
    after.add_child(Element::box_element().with_key("created"));
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&before, None, 20, 4);
    let root_before = engine.root_node;
    let committed_before = engine.committed_vnode.clone();
    set_incremental_fault(IncrementalFault::CreateBox);
    super::super::context_sync::set_layout_compute_fault();

    let failure = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 4)
        .expect_err("candidate and fresh rebuild faults must both be retained");

    match failure {
        TransactionalLayoutError::RecoveryFailed {
            incremental,
            rebuild,
        } => {
            assert_eq!(incremental.kind, IncrementalPatchKind::Create);
            assert_eq!(incremental.stage, PatchStage::CreateNode);
            assert!(matches!(rebuild.source, RebuildFailure::Taffy(_)));
        }
        other => panic!("expected both primary and recovery causes, got {other:?}"),
    }
    assert_eq!(engine.root_node, root_before);
    assert_eq!(engine.committed_vnode, committed_before);
}

fn unchanged_element_fixture() -> Element {
    let mut root = Element::root();
    root.add_child(Element::text("same").with_key("stable"));
    root
}

#[test]
pub(crate) fn unchanged_target_and_viewport_is_noop_with_fresh_aliases() {
    let first = unchanged_element_fixture();
    let first_id = first.id;
    let first_text_id = first.children.iter().next().expect("text child").id;
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&first, None, 20, 4);
    let root_node = engine.root_node;
    let vnode_map = engine.vnode_map.clone();
    let vnode_legacy_keys = engine.vnode_legacy_keys.clone();
    let current_vnode_flows = engine.current_vnode_flows.clone();
    let committed_vnode = engine.committed_vnode.clone();
    let node_count = engine.taffy.total_node_count();
    let cache_len = engine.flow_cache.len();
    assert!(
        cache_len > 0,
        "fixture must populate the committed flow cache"
    );
    super::super::context_sync::set_layout_compute_fault();

    let current_frame = unchanged_element_fixture();
    let current_id = current_frame.id;
    let current_text_id = current_frame.children.iter().next().expect("text child").id;
    let prepared = engine
        .prepare_element_incremental(&current_frame, Some(&previous), 20, 4)
        .expect("unchanged frame prepares aliases without layout work");
    assert!(
        engine.taffy.shares_storage(&prepared.engine().taffy),
        "no-op preparation must share, not clone, the Taffy backend"
    );
    assert!(
        engine
            .vnode_map
            .shares_storage(&prepared.engine().vnode_map)
    );
    assert!(
        engine
            .vnode_legacy_keys
            .shares_storage(&prepared.engine().vnode_legacy_keys)
    );
    assert!(
        engine
            .current_vnode_flows
            .shares_storage(&prepared.engine().current_vnode_flows)
    );
    assert!(
        engine
            .committed_vnode
            .shares_storage(&prepared.engine().committed_vnode)
    );
    assert_eq!(engine.flow_cache.len(), cache_len);
    assert!(prepared.engine().flow_cache.len() >= cache_len);
    let text_node = prepared
        .snapshot()
        .nodes()
        .find(|node| node.text_flow().is_some())
        .expect("unchanged text remains in the authoritative snapshot");
    assert_eq!(
        text_node.text_flow().unwrap().max_width(),
        text_node.content_bounds().width() as usize
    );
    assert!(engine.get_layout(first_text_id).is_some());
    assert!(engine.get_layout(current_text_id).is_none());
    assert!(prepared.engine().get_layout(current_text_id).is_some());
    let (current, report) = prepared.commit(&mut engine);

    assert_eq!(report, CheckedIncrementalLayoutReport::NoChange);
    assert_eq!(engine.root_node, root_node);
    assert_eq!(engine.vnode_map, vnode_map);
    assert!(engine.vnode_map.shares_storage(&vnode_map));
    assert!(engine.vnode_legacy_keys.shares_storage(&vnode_legacy_keys));
    assert!(
        engine
            .current_vnode_flows
            .shares_storage(&current_vnode_flows)
    );
    assert!(engine.committed_vnode.shares_storage(&committed_vnode));
    assert_eq!(engine.taffy.total_node_count(), node_count);
    assert_eq!(engine.flow_cache.len(), cache_len);
    assert!(engine.get_layout(current_id).is_some());
    assert!(engine.get_layout(current_text_id).is_some());
    assert!(engine.current_text_flow(current_text_id).is_some());
    assert_eq!(first_id, current_id, "root ElementId is canonical");
    assert!(engine.get_layout(first_text_id).is_none());

    let viewport_frame = unchanged_element_fixture();
    let (_, viewport_report) = engine
        .try_compute_element_incremental_transactional(&viewport_frame, Some(&current), 21, 4)
        .expect("the untouched one-shot fault is consumed by viewport recompute recovery");
    assert!(matches!(
        viewport_report,
        CheckedIncrementalLayoutReport::RecoveredFullRebuild { patch_count: 0, .. }
    ));
}
