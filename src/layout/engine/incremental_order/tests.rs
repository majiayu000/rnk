use super::*;
use crate::core::{Color, Element, Style, VNode};
use crate::layout::PatchTransactionCause;
use crate::reconciler::plan_diff;

#[derive(Clone)]
struct EngineBaseline {
    root: Option<NodeId>,
    vnode_map: HashMap<ScopedNodeIdentity, NodeId>,
    legacy: HashMap<ScopedNodeIdentity, crate::core::NodeKey>,
    committed: Option<VNode>,
    root_children: Vec<NodeId>,
    node_count: usize,
}

impl EngineBaseline {
    fn capture(engine: &LayoutEngine) -> Self {
        let root = engine.root_node;
        Self {
            root,
            vnode_map: (*engine.vnode_map).clone(),
            legacy: (*engine.vnode_legacy_keys).clone(),
            committed: (*engine.committed_vnode).clone(),
            root_children: engine
                .taffy
                .children(root.expect("fixture has a root"))
                .expect("fixture root is valid"),
            node_count: engine.taffy.total_node_count(),
        }
    }

    fn assert_unchanged(&self, engine: &LayoutEngine) {
        assert_eq!(engine.root_node, self.root);
        assert_eq!(&*engine.vnode_map, &self.vnode_map);
        assert_eq!(&*engine.vnode_legacy_keys, &self.legacy);
        assert_eq!(&*engine.committed_vnode, &self.committed);
        assert_eq!(engine.taffy.total_node_count(), self.node_count);
        assert_eq!(
            engine
                .taffy
                .children(self.root.expect("fixture root"))
                .expect("fixture tree remains committed"),
            self.root_children
        );
    }
}

fn two_child_tree() -> VNode {
    VNode::box_node().children([
        VNode::box_node().with_key("left"),
        VNode::box_node().with_key("right"),
    ])
}

fn root_parent_index(plan: &ReconcilePlan) -> usize {
    plan.parents
        .iter()
        .position(|parent| parent.parent == ScopedNodeIdentity::Root)
        .expect("plan contains root parent")
}

fn assert_committed_reason(engine: &LayoutEngine, plan: &ReconcilePlan, expected: &'static str) {
    match engine.validate_committed_plan(plan) {
        Err(ReconcilePlanError::CommittedTreeMismatch { reason }) => {
            assert_eq!(reason, expected)
        }
        other => panic!("expected committed mismatch {expected:?}, got {other:?}"),
    }
}

#[test]
pub(crate) fn invalid_final_order_variants_fail_before_mutation() {
    let tree = VNode::box_node().child(VNode::text("child").with_key("child"));
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let baseline = EngineBaseline::capture(&engine);
    let valid = plan_diff(&tree, &tree).expect("fixture plan is valid");
    let root_index = root_parent_index(&valid);

    let mut duplicate = valid.clone();
    let child_identity = duplicate.parents[root_index].final_children[0].clone();
    duplicate.parents[root_index]
        .final_children
        .push(child_identity.clone());
    assert!(matches!(
        engine.preflight_reconcile_plan(&duplicate),
        Err(ReconcilePlanError::DuplicateFinalIdentity { .. })
    ));
    baseline.assert_unchanged(&engine);

    let mut missing = valid.clone();
    missing.parents[root_index].survivors.clear();
    assert!(matches!(
        engine.preflight_reconcile_plan(&missing),
        Err(ReconcilePlanError::MissingFinalIdentity { .. })
    ));
    baseline.assert_unchanged(&engine);

    let mut duplicate_source = valid.clone();
    duplicate_source.parents[root_index]
        .creates
        .push(child_identity);
    assert!(matches!(
        engine.preflight_reconcile_plan(&duplicate_source),
        Err(ReconcilePlanError::DuplicateFinalIdentitySource { .. })
    ));
    baseline.assert_unchanged(&engine);

    let mut extra = valid;
    extra.parents[root_index].final_children.clear();
    assert!(matches!(
        engine.preflight_reconcile_plan(&extra),
        Err(ReconcilePlanError::ExtraPlannedIdentity { .. })
    ));
    baseline.assert_unchanged(&engine);
}

#[test]
fn preflight_existing_identity_and_node_use_matrix_is_exact() {
    let tree = two_child_tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let baseline = EngineBaseline::capture(&engine);
    let valid = plan_diff(&tree, &tree).expect("fixture plan is valid");

    let mut missing_old_identity = valid.clone();
    missing_old_identity.root.children[0].old_identity = None;
    assert!(matches!(
        engine.preflight_reconcile_plan(&missing_old_identity),
        Err(ReconcilePlanError::MissingExistingNodeId { .. })
    ));
    baseline.assert_unchanged(&engine);

    let first_old = valid.root.children[0]
        .old_identity
        .clone()
        .expect("survivor has old identity");
    let mut duplicate_identity = valid.clone();
    duplicate_identity.root.children[1].old_identity = Some(first_old.clone());
    assert!(matches!(
        engine.preflight_reconcile_plan(&duplicate_identity),
        Err(ReconcilePlanError::DuplicateExistingIdentityUse { .. })
    ));
    baseline.assert_unchanged(&engine);

    let mut missing_node_engine = engine.staged_clone();
    missing_node_engine.vnode_map.remove(&first_old);
    assert!(matches!(
        missing_node_engine.preflight_reconcile_plan(&valid),
        Err(ReconcilePlanError::MissingExistingNodeId { .. })
    ));

    let second_old = valid.root.children[1]
        .old_identity
        .clone()
        .expect("survivor has old identity");
    let mut aliased_node_engine = engine.staged_clone();
    let first_node = aliased_node_engine.vnode_map[&first_old];
    aliased_node_engine.vnode_map.insert(second_old, first_node);
    assert!(matches!(
        aliased_node_engine.preflight_reconcile_plan(&valid),
        Err(ReconcilePlanError::DuplicateExistingNodeIdUse { .. })
    ));

    let mut removal_reuses_root = valid.clone();
    let root_index = root_parent_index(&removal_reuses_root);
    removal_reuses_root.parents[root_index]
        .removals
        .push(ScopedNodeIdentity::Root);
    assert!(matches!(
        engine.preflight_reconcile_plan(&removal_reuses_root),
        Err(ReconcilePlanError::DuplicateExistingIdentityUse { .. })
    ));

    let mut missing_removal = valid;
    let root_index = root_parent_index(&missing_removal);
    let absent = plan_diff(
        &VNode::box_node().child(VNode::box_node().with_key("absent")),
        &VNode::box_node(),
    )
    .expect("fixture removal plan is valid")
    .parents
    .into_iter()
    .flat_map(|parent| parent.removals)
    .next()
    .expect("fixture has a removal");
    missing_removal.parents[root_index].removals.push(absent);
    assert!(matches!(
        engine.preflight_reconcile_plan(&missing_removal),
        Err(ReconcilePlanError::MissingExistingNodeId { .. })
    ));
    baseline.assert_unchanged(&engine);
}

#[test]
fn preflight_rejects_two_removals_that_alias_one_layout_node() {
    let old = two_child_tree();
    let new = VNode::box_node();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let plan = plan_diff(&old, &new).expect("fixture removal plan is valid");
    let removals: Vec<_> = plan
        .parents
        .iter()
        .flat_map(|parent| &parent.removals)
        .cloned()
        .collect();
    assert_eq!(removals.len(), 2);

    let first_node = engine.vnode_map[&removals[0]];
    engine.vnode_map.insert(removals[1].clone(), first_node);
    let baseline = EngineBaseline::capture(&engine);

    assert!(matches!(
        engine.preflight_reconcile_plan(&plan),
        Err(ReconcilePlanError::DuplicateExistingNodeIdUse { .. })
    ));
    baseline.assert_unchanged(&engine);
}

#[test]
fn preflight_checks_create_update_replace_and_taffy_style_contracts() {
    let old = VNode::box_node().children([
        VNode::box_node().with_key("update"),
        VNode::text("old").with_key("replace"),
    ]);
    let mut updated = VNode::box_node().with_key("update");
    updated.props.scroll_offset_x = Some(1);
    let new = VNode::box_node().children([
        updated,
        VNode::box_node().with_key("replace"),
        VNode::box_node().with_key("create"),
    ]);
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    let plan = plan_diff(&old, &new).expect("fixture plan is valid");
    engine
        .preflight_reconcile_plan(&plan)
        .expect("all action variants have valid existing-node requirements");

    set_incremental_order_fault(IncrementalOrderFault::PreflightStyle);
    assert!(matches!(
        engine.preflight_reconcile_plan(&plan),
        Err(ReconcilePlanError::MissingExistingNodeId { .. })
    ));
}

#[test]
fn committed_validation_reports_map_count_alias_root_and_projection_corruption() {
    let tree = two_child_tree();
    let plan = plan_diff(&tree, &tree).expect("fixture plan is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    engine
        .validate_committed_plan(&plan)
        .expect("fresh committed tree validates");

    let mut map_mismatch = engine.staged_clone();
    let extra_plan = plan_diff(
        &VNode::box_node(),
        &VNode::box_node().child(VNode::box_node().with_key("extra")),
    )
    .expect("extra fixture plan is valid");
    let extra_identity = extra_plan.root.children[0].identity.clone();
    map_mismatch
        .vnode_legacy_keys
        .insert(extra_identity, extra_plan.root.children[0].legacy_key);
    assert_committed_reason(
        &map_mismatch,
        &plan,
        "identity map does not exactly match the committed VNode",
    );

    let mut vnode_map_mismatch = engine.staged_clone();
    let extra_node = vnode_map_mismatch.vnode_map[&ScopedNodeIdentity::Root];
    vnode_map_mismatch
        .vnode_map
        .insert(extra_plan.root.children[0].identity.clone(), extra_node);
    assert_committed_reason(
        &vnode_map_mismatch,
        &plan,
        "identity map does not exactly match the committed VNode",
    );

    let mut count_mismatch = engine.staged_clone();
    count_mismatch
        .taffy
        .new_leaf(taffy::Style::default())
        .expect("fixture extra node is allocated");
    assert_committed_reason(
        &count_mismatch,
        &plan,
        "layout tree contains unmapped or missing nodes",
    );

    let mut alias = engine.staged_clone();
    let left = plan.root.children[0].identity.clone();
    let right = plan.root.children[1].identity.clone();
    let left_node = alias.vnode_map[&left];
    alias.vnode_map.insert(right, left_node);
    assert_committed_reason(
        &alias,
        &plan,
        "committed child order differs from the committed VNode",
    );

    let mut root_mismatch = engine.staged_clone();
    root_mismatch.root_node = Some(root_mismatch.vnode_map[&left]);
    assert_committed_reason(
        &root_mismatch,
        &plan,
        "root identity and root layout node disagree",
    );

    let mut stale_projection = engine.staged_clone();
    stale_projection
        .vnode_legacy_keys
        .insert(left, crate::core::NodeKey::root());
    assert_committed_reason(
        &stale_projection,
        &plan,
        "legacy compatibility projection is stale",
    );
}

#[test]
fn committed_validation_reports_duplicate_missing_and_order_corruption() {
    let tree = two_child_tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let valid = plan_diff(&tree, &tree).expect("fixture plan is valid");

    let mut duplicate = valid.clone();
    let duplicate_identity = duplicate.root.children[0].identity.clone();
    duplicate.root.children[1].identity = duplicate_identity.clone();
    let mut duplicate_engine = engine.staged_clone();
    let duplicate_node = duplicate_engine.vnode_map[&duplicate_identity];
    duplicate_engine
        .taffy
        .set_children(
            duplicate_engine.root_node.expect("fixture root"),
            &[duplicate_node, duplicate_node],
        )
        .expect("duplicate fixture order is committed");
    assert_committed_reason(
        &duplicate_engine,
        &duplicate,
        "committed VNode contains a duplicate scoped identity",
    );

    let mut missing = engine.staged_clone();
    missing.vnode_map.remove(&valid.root.children[0].identity);
    assert!(matches!(
        missing.validate_committed_plan(&valid),
        Err(ReconcilePlanError::CommittedTreeMismatch {
            reason: "committed child order differs from the committed VNode"
        })
    ));

    let mut missing_root = engine.staged_clone();
    missing_root.vnode_map.remove(&ScopedNodeIdentity::Root);
    assert!(matches!(
        missing_root.validate_committed_plan(&valid),
        Err(ReconcilePlanError::MissingExistingNodeId { .. })
    ));

    let root = engine.root_node.expect("fixture root");
    let mut reversed = engine.taffy.children(root).expect("fixture order");
    reversed.reverse();
    engine
        .taffy
        .set_children(root, &reversed)
        .expect("fixture order corruption succeeds");
    assert_committed_reason(
        &engine,
        &valid,
        "committed child order differs from the committed VNode",
    );

    let mut healthy = LayoutEngine::new();
    healthy.compute_vnode(&tree, 20, 4);
    set_incremental_order_fault(IncrementalOrderFault::ValidateChildren);
    assert_committed_reason(
        &healthy,
        &valid,
        "mapped layout node is no longer in the Taffy tree",
    );
}

#[test]
pub(crate) fn incremental_reorder_fault_reports_the_real_parent_locator() {
    let old = VNode::root().children([
        VNode::box_node().with_key("left"),
        VNode::box_node().with_key("right"),
    ]);
    let new = VNode::root().children([
        VNode::box_node().with_key("right"),
        VNode::box_node().with_key("left"),
    ]);
    let plan = plan_diff(&old, &new).expect("reorder fixture plan");
    let expected_parent = plan
        .patches()
        .iter()
        .find_map(|patch| match patch {
            crate::reconciler::Patch::Reorder { parent, .. } => Some(*parent),
            _ => None,
        })
        .expect("fixture has one reorder");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    set_incremental_order_fault(IncrementalOrderFault::CommitChildren);

    assert_eq!(
        engine
            .apply_reconcile_plan(&plan)
            .map_err(|error| error.patch),
        Err(PatchError {
            kind: PatchKind::Reorder,
            key: expected_parent,
            failure: PatchFailure::TreeRejected,
        })
    );
}

fn assert_structural_set_children_origin(old: &VNode, new: &VNode, kind: PatchKind) {
    let plan = plan_diff(old, new).expect("structural SetChildren fixture plan");
    let patch_index = plan
        .patches()
        .iter()
        .position(|patch| {
            matches!(
                (kind, patch),
                (PatchKind::Create, Patch::Create { .. })
                    | (PatchKind::Remove, Patch::Remove { .. })
                    | (PatchKind::Replace, Patch::Replace { .. })
            )
        })
        .expect("fixture contains the requested structural patch");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(old, 20, 4);
    set_incremental_order_fault(IncrementalOrderFault::CommitChildren);

    let error = engine
        .apply_reconcile_plan(&plan)
        .expect_err("changed child order must exercise the SetChildren fault");

    assert_eq!(error.patch_index, Some(patch_index));
    assert_eq!(error.patch.kind, kind);
    assert_eq!(error.stage, PatchStage::SetChildren);
    assert!(matches!(error.source, PatchTransactionCause::Taffy(_)));
}

#[test]
pub(crate) fn structural_set_children_faults_keep_create_remove_replace_origins() {
    let empty = VNode::root();
    let boxed = VNode::root().child(VNode::box_node().with_key("child"));
    let text = VNode::root().child(VNode::text("child").with_key("child"));

    assert_structural_set_children_origin(&empty, &boxed, PatchKind::Create);
    assert_structural_set_children_origin(&boxed, &empty, PatchKind::Remove);
    assert_structural_set_children_origin(&boxed, &text, PatchKind::Replace);
}

#[test]
fn unchanged_child_lists_skip_the_fallible_set_children_write() {
    let old = VNode::root().child(VNode::box_node().with_key("child"));
    let mut style = Style::new();
    style.color = Some(Color::Red);
    let new = VNode::root().child(VNode::box_node().with_key("child").with_style(style));
    let plan = plan_diff(&old, &new).expect("update-only fixture plan");
    assert!(
        plan.patches()
            .iter()
            .all(|patch| matches!(patch, Patch::Update { .. }))
    );
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&old, 20, 4);
    set_incremental_order_fault(IncrementalOrderFault::CommitChildren);

    engine
        .apply_reconcile_plan(&plan)
        .expect("update-only plans do not rewrite unchanged child lists");

    assert!(take_incremental_order_fault(
        IncrementalOrderFault::CommitChildren
    ));
}

#[test]
fn committed_validation_detects_multiple_identities_for_one_node_after_order_is_coherent() {
    let tree = two_child_tree();
    let plan = plan_diff(&tree, &tree).expect("fixture plan is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let left = plan.root.children[0].identity.clone();
    let right = plan.root.children[1].identity.clone();
    let shared = engine.vnode_map[&left];
    engine.vnode_map.insert(right, shared);
    let root = engine.root_node.expect("fixture root");
    engine
        .taffy
        .set_children(root, &[shared, shared])
        .expect("fixture alias order is committed");

    assert_committed_reason(
        &engine,
        &plan,
        "multiple scoped identities reference one layout node",
    );
}

#[test]
pub(crate) fn commit_and_postcondition_faults_recover_atomically_through_checked_caller() {
    fn element_tree(order: &[&str]) -> Element {
        let mut root = Element::root();
        for key in order {
            root.add_child(Element::box_element().with_key(*key));
        }
        root
    }

    for fault in [
        IncrementalOrderFault::CommitChildren,
        IncrementalOrderFault::PostconditionChildren,
    ] {
        let old = element_tree(&["left", "right"]);
        let new = element_tree(&["right", "left"]);
        let mut engine = LayoutEngine::new();
        let (previous, _) = engine.compute_element_incremental(&old, None, 20, 4);
        set_incremental_order_fault(fault);

        let (current, outcome) = engine
            .try_compute_element_incremental_checked(&new, Some(&previous), 20, 4)
            .expect("one-shot order fault recovers by rebuilding the staged candidate");

        assert!(outcome.fallback_full_rebuild);
        assert!(matches!(
            outcome.patch_error,
            Some(PatchError {
                kind: PatchKind::Reorder,
                failure: PatchFailure::TreeRejected | PatchFailure::PostconditionViolated,
                ..
            })
        ));
        assert_eq!(engine.committed_vnode.as_ref(), Some(&current));
        assert_eq!(engine.taffy.total_node_count(), engine.vnode_map.len());
    }
}

#[test]
fn postcondition_matrix_returns_exact_failure_without_silent_success() {
    let tree = two_child_tree();
    let plan = plan_diff(&tree, &tree).expect("fixture plan is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    engine
        .check_reconcile_postconditions(&plan.root)
        .expect("fresh committed tree satisfies postconditions");

    let mut missing_root = engine.staged_clone();
    missing_root.vnode_map.remove(&ScopedNodeIdentity::Root);
    assert!(matches!(
        missing_root.check_reconcile_postconditions(&plan.root),
        Err(PatchError {
            kind: PatchKind::Reorder,
            failure: PatchFailure::PostconditionViolated,
            ..
        })
    ));

    let mut missing_child = engine.staged_clone();
    missing_child
        .vnode_map
        .remove(&plan.root.children[0].identity);
    assert!(matches!(
        missing_child.check_reconcile_postconditions(&plan.root),
        Err(PatchError {
            failure: PatchFailure::PostconditionViolated,
            ..
        })
    ));

    let root = engine.root_node.expect("fixture root");
    engine
        .taffy
        .set_children(root, &[])
        .expect("fixture order corruption succeeds");
    assert!(matches!(
        engine.check_reconcile_postconditions(&plan.root),
        Err(PatchError {
            failure: PatchFailure::PostconditionViolated,
            ..
        })
    ));

    let mut healthy = LayoutEngine::new();
    healthy.compute_vnode(&tree, 20, 4);
    set_incremental_order_fault(IncrementalOrderFault::PostconditionChildren);
    assert!(matches!(
        healthy.check_reconcile_postconditions(&plan.root),
        Err(PatchError {
            failure: PatchFailure::PostconditionViolated,
            ..
        })
    ));
}

#[test]
fn direct_commit_sets_exact_recursive_child_order() {
    let tree = VNode::box_node().child(VNode::box_node().with_key("branch").children([
        VNode::box_node().with_key("left"),
        VNode::box_node().with_key("right"),
    ]));
    let plan = plan_diff(&tree, &tree).expect("fixture plan is valid");
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 20, 4);
    let target_map = engine.vnode_map.clone();

    engine
        .commit_planned_children(&plan.root, &target_map)
        .expect("recursive exact order commit succeeds");
    engine
        .check_reconcile_postconditions(&plan.root)
        .expect("recursive exact order is readable after commit");
}
