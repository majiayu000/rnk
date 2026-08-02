use super::*;
use crate::core::Props;
use crate::reconciler::identity::{CanonicalKey, ScopedIdentitySegment};
use std::any::TypeId;

fn one_child_tree() -> VNode {
    VNode::box_node().child(VNode::text("child").with_key("child"))
}

fn root_parent_index(plan: &ReconcilePlan) -> usize {
    plan.parents
        .iter()
        .position(|parent| parent.parent == ScopedNodeIdentity::Root)
        .expect("every plan has a root parent order")
}

fn keyed_segment(value: &str) -> ScopedIdentitySegment {
    ScopedIdentitySegment::Keyed {
        key: CanonicalKey::Exact(value.into()),
        type_id: TypeId::of::<u8>(),
    }
}

fn constant_projection(_: &PlannedNode) -> SiblingIdentity {
    SiblingIdentity::Keyed {
        user_key: 7,
        type_id: TypeId::of::<u8>(),
    }
}

fn assert_planned_vnodes_are_shallow(planned: &PlannedNode, target: &VNode) {
    assert!(
        planned.vnode.children.is_empty(),
        "planned node {:?} recursively cloned {} descendants",
        planned.vnode.key,
        planned.vnode.children.len()
    );
    assert_eq!(planned.vnode.node_type, target.node_type);
    assert_eq!(planned.vnode.props, target.props);
    assert_eq!(planned.children.len(), target.children.len());
    for (planned_child, target_child) in planned.children.iter().zip(&target.children) {
        assert_planned_vnodes_are_shallow(planned_child, target_child);
    }
}

#[test]
fn planned_nodes_are_shallow_while_patch_payloads_retain_full_subtrees() {
    let target = VNode::box_node().children([
        VNode::box_node().with_key("branch").child(
            VNode::box_node()
                .with_key("nested")
                .child(VNode::text("leaf").with_key("leaf")),
        ),
        VNode::text("sibling").with_key("sibling"),
    ]);
    let no_op = plan_diff(&target, &target).expect("mixed no-op tree is valid");
    assert_planned_vnodes_are_shallow(&no_op.root, &target);
    assert!(
        crate::reconciler::try_diff(&target, &target)
            .expect("public no-op diff is valid")
            .is_empty()
    );

    let created = plan_diff(&VNode::box_node(), &target).expect("create plan is valid");
    let created_branch = created
        .patches()
        .iter()
        .find_map(|patch| match patch {
            Patch::Create { node, .. } if !node.children.is_empty() => Some(node),
            _ => None,
        })
        .expect("create patch retains its full branch payload");
    assert_eq!(created_branch.children[0].children.len(), 1);

    let old = VNode::box_node().child(VNode::text("old").with_key("replace"));
    let new = VNode::box_node().child(
        VNode::box_node()
            .with_key("replace")
            .child(VNode::text("replacement leaf").with_key("leaf")),
    );
    let replaced = plan_diff(&old, &new).expect("replace plan is valid");
    let replacement = replaced
        .patches()
        .iter()
        .find_map(|patch| match patch {
            Patch::Replace { node, .. } => Some(node),
            _ => None,
        })
        .expect("replace patch retains its full subtree payload");
    assert_eq!(replacement.children.len(), 1);
}

#[test]
fn parent_plan_validation_reports_each_exact_structural_error() {
    let valid = plan_initial_tree(&one_child_tree()).expect("fixture plan is valid");
    let parent = valid.parents[root_parent_index(&valid)].clone();
    let identity = parent.final_children[0].clone();

    let mut duplicate_final = parent.clone();
    duplicate_final.final_children.push(identity.clone());
    assert!(matches!(
        duplicate_final.validate(),
        Err(ReconcilePlanError::DuplicateFinalIdentity {
            first_index: 0,
            second_index: 1,
            ..
        })
    ));

    let mut missing_source = parent.clone();
    missing_source.creates.clear();
    assert!(matches!(
        missing_source.validate(),
        Err(ReconcilePlanError::MissingFinalIdentity { .. })
    ));

    let mut duplicate_source = parent.clone();
    duplicate_source.survivors.push(identity.clone());
    assert!(matches!(
        duplicate_source.validate(),
        Err(ReconcilePlanError::DuplicateFinalIdentitySource { .. })
    ));

    let mut extra_source = parent;
    extra_source.final_children.clear();
    assert!(matches!(
        extra_source.validate(),
        Err(ReconcilePlanError::ExtraPlannedIdentity { .. })
    ));
}

#[test]
fn reconcile_plan_parent_index_validation_matrix_is_closed() {
    let valid = plan_initial_tree(&one_child_tree()).expect("fixture plan is valid");

    let mut duplicate_parent = valid.clone();
    duplicate_parent
        .parents
        .push(duplicate_parent.parents[0].clone());
    assert!(matches!(
        duplicate_parent.validate_final_orders(),
        Err(ReconcilePlanError::DuplicateParentPlan { .. })
    ));

    let child_identity = valid.root.children[0].identity.clone();
    let mut missing_parent = valid.clone();
    missing_parent
        .parents
        .retain(|parent| parent.parent != child_identity);
    assert!(matches!(
        missing_parent.validate_final_orders(),
        Err(ReconcilePlanError::MissingParentPlan { .. })
    ));

    let mut arena = ScopedIdentityArena::default();
    let extra_identity = arena.child(&ScopedNodeIdentity::Root, keyed_segment("extra"));
    let mut extra_parent = valid;
    extra_parent.parents.push(ParentPlan {
        parent: extra_identity,
        final_children: Vec::new(),
        survivors: Vec::new(),
        creates: Vec::new(),
        removals: Vec::new(),
    });
    assert!(matches!(
        extra_parent.validate_final_orders(),
        Err(ReconcilePlanError::ExtraParentPlan { .. })
    ));
}

#[test]
fn reconcile_plan_tree_validation_rejects_order_and_global_identity_corruption() {
    let tree = VNode::box_node().children([
        VNode::box_node()
            .with_key("branch")
            .child(VNode::text("leaf").with_props(Props::new().key("leaf"))),
        VNode::text("sibling").with_key("sibling"),
    ]);
    let valid = plan_initial_tree(&tree).expect("fixture plan is valid");

    let mut order_mismatch = valid.clone();
    let root_index = root_parent_index(&order_mismatch);
    order_mismatch.parents[root_index].final_children.swap(0, 1);
    assert!(matches!(
        order_mismatch.validate_final_orders(),
        Err(ReconcilePlanError::PlannedChildrenMismatch { .. })
    ));

    let mut duplicate_identity = valid;
    let branch_identity = duplicate_identity.root.children[0].identity.clone();
    let old_leaf_identity = duplicate_identity.root.children[0].children[0]
        .identity
        .clone();
    duplicate_identity.root.children[0].children[0].identity = branch_identity.clone();
    let branch_parent = duplicate_identity
        .parents
        .iter_mut()
        .find(|parent| parent.parent == branch_identity)
        .expect("fixture contains branch parent plan");
    branch_parent.final_children[0] = branch_identity.clone();
    branch_parent
        .creates
        .retain(|item| item != &old_leaf_identity);
    branch_parent.creates.push(branch_identity);
    assert!(matches!(
        duplicate_identity.validate_final_orders(),
        Err(ReconcilePlanError::DuplicatePlannedIdentity { .. })
    ));
}

#[test]
fn composite_projection_validation_is_idempotent_and_collision_checked() {
    let plan = plan_initial_tree(&one_child_tree()).expect("fixture plan is valid");
    plan.validate_composite_projections_with(&composite_projection)
        .expect("real scoped projection is injective");
    plan.validate_composite_projection_union_with(&plan, &composite_projection)
        .expect("reinserting identical scope projections is idempotent");
    assert!(matches!(
        plan.validate_composite_projections_with(&constant_projection),
        Err(ReconcilePlanError::CompositeIdentityCollision { .. })
    ));

    let same = plan.clone();
    assert!(matches!(
        plan.validate_composite_projection_union_with(&same, &constant_projection),
        Err(ReconcilePlanError::CompositeIdentityCollision { .. })
    ));
}

#[test]
fn nested_cross_frame_projection_collision_propagates_from_exact_parent_scope() {
    let old = VNode::box_node().child(
        VNode::box_node()
            .with_key("branch")
            .child(VNode::text("same").with_props(Props::new().key("old"))),
    );
    let new = VNode::box_node().child(
        VNode::box_node()
            .with_key("branch")
            .child(VNode::text("same").with_props(Props::new().key("new"))),
    );
    let failure = plan_diff_with_token_source(&old, &new, &constant_token)
        .expect_err("distinct nested exact sources cannot alias through one token");

    match failure {
        ReconcilePlanError::KeyTokenCollision {
            parent_scope,
            token,
            first_index,
            second_index,
        } => {
            assert_ne!(parent_scope, ScopedNodeIdentity::Root.diagnostic());
            assert_eq!(token, 7);
            assert_eq!((first_index, second_index), (0, 0));
        }
        other => panic!("unexpected failure: {other:?}"),
    }
}

#[test]
fn child_patch_planner_covers_create_update_replace_remove_and_reorder_contracts() {
    let parent = NodeKey::root();
    let old = [
        VNode::box_node().with_key("move"),
        VNode::text("old").with_key("replace"),
        VNode::box_node().with_key("remove"),
        VNode::box_node().with_key("update"),
    ];
    let mut updated = VNode::box_node().with_key("update");
    updated.props.scroll_offset_y = Some(1);
    let new = [
        updated,
        VNode::box_node().with_key("move"),
        VNode::box_node().with_key("replace"),
        VNode::box_node().with_key("create"),
    ];

    let patches = plan_child_patches(&old, &new, parent).expect("fixture plan is valid");
    assert!(
        patches
            .iter()
            .any(|patch| matches!(patch, Patch::Create { .. }))
    );
    assert!(
        patches
            .iter()
            .any(|patch| matches!(patch, Patch::Update { .. }))
    );
    assert!(
        patches
            .iter()
            .any(|patch| matches!(patch, Patch::Replace { .. }))
    );
    assert!(
        patches
            .iter()
            .any(|patch| matches!(patch, Patch::Remove { .. }))
    );
    assert!(
        patches
            .iter()
            .any(|patch| matches!(patch, Patch::Reorder { .. }))
    );
}

#[test]
fn nan_style_scroll_only_update_does_not_mark_style_or_text_context() {
    let mut old = VNode::text("stable");
    old.props.style.flex_grow = f32::NAN;
    let mut new = old.clone();
    new.props.scroll_offset_y = Some(1);

    let plan = plan_diff(&old, &new).expect("scroll-only update plans");

    assert_eq!(plan.root.action, PlannedNodeAction::Update);
    assert!(!plan.root.mutations.style);
    assert!(!plan.root.mutations.text_context);
}

#[test]
fn plan_patch_views_preserve_owned_complete_result() {
    let old = VNode::box_node();
    let new = VNode::box_node().child(VNode::text("new").with_key("new"));
    let plan = plan_diff(&old, &new).expect("fixture plan is valid");

    assert_eq!(plan.patches().len(), 1);
    assert_eq!(plan.into_patches().len(), 1);
}
