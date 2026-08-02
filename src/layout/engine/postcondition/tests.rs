use crate::core::{Dimension, Element, ElementType, Props, Style, VNode};
use crate::reconciler::{ReconcilePlan, ScopedIdentityArena, ScopedNodeIdentity, plan_diff_in};

use super::{
    super::{IncrementalInvariantError, LayoutEngine, normalized_taffy_style},
    TargetAliasExpectation, TargetValidationCause, TargetValidationError,
};

mod coverage;

fn raw_fixture() -> (LayoutEngine, VNode, ReconcilePlan) {
    let target = VNode::root().children([
        VNode::box_node().with_key("left"),
        VNode::box_node().with_key("right"),
    ]);
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_vnode(&target, 20, 4)
        .expect("fixture target is valid");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let plan = plan_diff_in(&target, &target, &mut arena).expect("fixture plan is valid");
    (engine, target, plan)
}

fn assert_invariant(error: TargetValidationError, expected: IncrementalInvariantError) {
    assert!(
        matches!(error.source, TargetValidationCause::Invariant(actual) if actual == expected),
        "expected {expected:?}, got {error:?}"
    );
}

#[test]
fn target_snapshot_comparison_is_reflexive_for_non_finite_style() {
    let mut props = Props::default();
    props.style.top = Some(f32::NAN);
    props.style.flex_grow = f32::NAN;
    props.style.padding.left = f32::NAN;
    props.style.margin.right = f32::NAN;
    props.style.row_gap = Some(f32::NAN);
    props.style.width = Dimension::Points(f32::NAN);
    props.style.max_height = Dimension::Percent(f32::NAN);

    assert!(super::props_snapshots_match(&props, &props.clone()));

    let mut changed = props.clone();
    changed.scroll_offset_x = Some(1);
    assert!(!super::props_snapshots_match(&props, &changed));

    let mut changed_dimension = props.clone();
    changed_dimension.style.width = Dimension::Percent(f32::NAN);
    assert!(!super::props_snapshots_match(&props, &changed_dimension));
}

#[test]
fn clearing_backend_drops_stale_node_context_storage() {
    let target = VNode::root().child(VNode::text("context").with_key("text"));
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_vnode(&target, 20, 4)
        .expect("fixture target is valid");
    let old_root = engine.root_node.expect("root");
    let committed_count = engine.taffy.total_node_count();

    assert!(
        engine
            .build_tree(&Element::new(ElementType::VirtualText))
            .is_none()
    );
    assert_eq!(engine.taffy.total_node_count(), committed_count);
    assert!(engine.taffy.get_node_context(old_root).is_some());

    let mut candidate = engine.staged_clone();
    assert!(
        candidate
            .build_tree_in_place(&Element::new(ElementType::VirtualText))
            .is_none()
    );
    assert_eq!(candidate.taffy.total_node_count(), 0);
    assert!(candidate.taffy.get_node_context(old_root).is_none());
}

#[test]
fn target_exact_validator_rejects_stale_style() {
    let root = VNode::box_node().child(VNode::text("target").with_key("text"));
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_vnode(&root, 20, 4)
        .expect("target is valid");
    let target = root;
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let plan = plan_diff_in(&target, &target, &mut arena).expect("target plan is valid");
    let text = &plan.root.children[0];
    let text_node = engine.vnode_map[&text.identity];
    let stale = Style {
        width: Dimension::Points(999.0),
        ..Style::default()
    };
    engine
        .taffy
        .set_style(text_node, normalized_taffy_style(&stale, true))
        .expect("test fault can stale the backend style");

    assert!(
        engine
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .is_err()
    );
}

#[test]
fn target_exact_validator_compares_complete_target_props() {
    let mut stale = VNode::box_node().with_key("box");
    stale.props.style.width = Dimension::Points(2.0);
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_vnode(&stale, 20, 4)
        .expect("stale frame is valid");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let plan = plan_diff_in(&stale, &stale, &mut arena).expect("stale no-op plan is valid");

    let mut changed_style = stale.clone();
    changed_style.props.style.width = Dimension::Points(9.0);
    assert!(
        engine
            .validate_target_exact(
                &plan,
                TargetAliasExpectation::RawVNode,
                &changed_style,
                20,
                4,
            )
            .is_err(),
        "layout props must be target-exact even when the supplied plan is stale"
    );

    let mut changed_scroll = stale.clone();
    changed_scroll.props.scroll_offset_x = Some(3);
    assert!(
        engine
            .validate_target_exact(
                &plan,
                TargetAliasExpectation::RawVNode,
                &changed_scroll,
                20,
                4,
            )
            .is_err(),
        "non-layout props must also be target-exact"
    );
}

#[test]
fn target_exact_validator_rejects_structural_corruption_matrix() {
    let (mut missing_root, target, plan) = raw_fixture();
    missing_root.root_node = None;
    assert_invariant(
        missing_root
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("missing root"),
        IncrementalInvariantError::MissingRoot,
    );

    let (mut invalid_root, target, plan) = raw_fixture();
    invalid_root.root_node = Some(invalid_root.vnode_map[&plan.root.children[0].identity]);
    assert_invariant(
        invalid_root
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("invalid root"),
        IncrementalInvariantError::InvalidRoot,
    );

    let (mut missing_child, target, plan) = raw_fixture();
    let root = missing_child.root_node.expect("root");
    let left = missing_child.vnode_map[&plan.root.children[0].identity];
    missing_child
        .taffy
        .set_children(root, &[left])
        .expect("test can detach one target node");
    assert_invariant(
        missing_child
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("missing reachable child"),
        IncrementalInvariantError::ReachableNodeSetMismatch,
    );

    let (mut orphan, target, plan) = raw_fixture();
    orphan
        .taffy
        .new_leaf(taffy::Style::default())
        .expect("test can add an orphan");
    assert_invariant(
        orphan
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("orphan backend node"),
        IncrementalInvariantError::NodeCountMismatch,
    );

    let (mut reordered, target, plan) = raw_fixture();
    let root = reordered.root_node.expect("root");
    let left = reordered.vnode_map[&plan.root.children[0].identity];
    let right = reordered.vnode_map[&plan.root.children[1].identity];
    reordered
        .taffy
        .set_children(root, &[right, left])
        .expect("test can reverse children");
    reordered
        .run_layout_and_publish_checked(&mut || false)
        .expect("reordered backend can recompute");
    assert_invariant(
        reordered
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("wrong child order"),
        IncrementalInvariantError::ChildOrderMismatch,
    );

    let (mut duplicate_edge, target, plan) = raw_fixture();
    let root = duplicate_edge.root_node.expect("root");
    let left = duplicate_edge.vnode_map[&plan.root.children[0].identity];
    duplicate_edge
        .taffy
        .set_children(root, &[left, left])
        .expect("test can expose a duplicate reachable edge");
    assert_invariant(
        duplicate_edge
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("duplicate reachable node"),
        IncrementalInvariantError::ReachableNodeCycle,
    );
}

#[test]
fn target_exact_validator_rejects_mapping_layout_context_and_viewport_corruption() {
    let (mut missing_map, target, plan) = raw_fixture();
    missing_map
        .vnode_map
        .remove(&plan.root.children[0].identity);
    assert_invariant(
        missing_map
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("missing scoped mapping"),
        IncrementalInvariantError::ScopedMapMismatch,
    );

    let (mut duplicate_map, target, plan) = raw_fixture();
    let root = duplicate_map.vnode_map[&ScopedNodeIdentity::Root];
    duplicate_map
        .vnode_map
        .insert(plan.root.children[0].identity.clone(), root);
    assert_invariant(
        duplicate_map
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("duplicate backend mapping"),
        IncrementalInvariantError::InvalidMappedNode,
    );

    let (mut deleted_node, target, plan) = raw_fixture();
    let left = deleted_node.vnode_map[&plan.root.children[0].identity];
    deleted_node
        .taffy
        .remove(left)
        .expect("test can delete a mapped backend node");
    assert_invariant(
        deleted_node
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("deleted mapped node"),
        IncrementalInvariantError::InvalidMappedNode,
    );

    let (mut deleted_root, target, plan) = raw_fixture();
    let root = deleted_root.root_node.expect("root");
    deleted_root
        .taffy
        .new_leaf(taffy::Style::default())
        .expect("test can add a count-preserving orphan");
    deleted_root
        .taffy
        .remove(root)
        .expect("test can delete the mapped root");
    assert_eq!(
        deleted_root.taffy.total_node_count(),
        deleted_root.vnode_map.len()
    );
    assert_invariant(
        deleted_root
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("deleted root mapping"),
        IncrementalInvariantError::InvalidMappedNode,
    );

    let (mut replaced_backend_node, target, plan) = raw_fixture();
    let left = replaced_backend_node.vnode_map[&plan.root.children[0].identity];
    replaced_backend_node
        .taffy
        .remove(left)
        .expect("test can delete a mapped child");
    replaced_backend_node
        .taffy
        .new_leaf(taffy::Style::default())
        .expect("test can restore the backend node count with an orphan");
    assert_eq!(
        replaced_backend_node.taffy.total_node_count(),
        replaced_backend_node.vnode_map.len(),
        "the invalid mapping must not be detected only by a node-count deficit"
    );
    assert_invariant(
        replaced_backend_node
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("stale mapping with count-preserving orphan"),
        IncrementalInvariantError::InvalidMappedNode,
    );

    let (mut compatibility, target, plan) = raw_fixture();
    compatibility.vnode_legacy_keys.insert(
        plan.root.children[0].identity.clone(),
        VNode::box_node().with_key("wrong").key,
    );
    assert_invariant(
        compatibility
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("wrong compatibility key"),
        IncrementalInvariantError::CompatibilityMapMismatch,
    );

    let (mut dirty, target, plan) = raw_fixture();
    let left = dirty.vnode_map[&plan.root.children[0].identity];
    dirty.taffy.mark_dirty(left).expect("test can dirty node");
    assert_invariant(
        dirty
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("missing current layout"),
        IncrementalInvariantError::MissingComputedLayout,
    );

    let (mut viewport, target, plan) = raw_fixture();
    viewport.last_width = 19;
    assert_invariant(
        viewport
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("stale viewport"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let text = VNode::root().child(VNode::text("text").with_key("text"));
    let mut context = LayoutEngine::new();
    context
        .try_compute_vnode(&text, 20, 4)
        .expect("text fixture is valid");
    let mut arena = ScopedIdentityArena::seeded(context.vnode_map.keys());
    let plan = plan_diff_in(&text, &text, &mut arena).expect("text no-op plan");
    let text_node = context.vnode_map[&plan.root.children[0].identity];
    context
        .taffy
        .set_node_context(
            text_node,
            Some(super::super::text_flow_bridge::NodeContext::new(
                None,
                &context.text_flow_policy,
            )),
        )
        .expect("test can stale text context");
    context
        .run_layout_and_publish_checked(&mut || false)
        .expect("stale non-text context still computes");
    assert_invariant(
        context
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &text, 20, 4)
            .expect_err("stale text context"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );
}

#[test]
fn target_exact_validator_rejects_element_alias_corruption() {
    let mut element = Element::root();
    let child = Element::box_element().with_key("child");
    let child_id = child.id;
    element.add_child(child);
    let mut engine = LayoutEngine::new();
    let (target, _) = engine
        .try_compute_element_incremental_transactional(&element, None, 20, 4)
        .expect("element fixture is valid");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let snapshot =
        super::super::incremental::ElementVNodeSnapshot::from_element(&element, &mut arena)
            .expect("element snapshot");
    let plan = plan_diff_in(&target, &snapshot.vnode, &mut arena).expect("element no-op plan");
    engine.node_map.remove(&child_id);

    assert_invariant(
        engine
            .validate_target_exact(
                &plan,
                TargetAliasExpectation::Element(&snapshot),
                &snapshot.vnode,
                20,
                4,
            )
            .expect_err("missing element alias"),
        IncrementalInvariantError::ElementMapMismatch,
    );
}
