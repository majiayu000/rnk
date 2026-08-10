use std::{collections::HashSet, sync::Arc};

use taffy::{AvailableSpace, Size};

use crate::core::{Element, ElementId, Style, VNode};
use crate::layout::{IncrementalInvariantError, TextFlow, TextFlowInput, TextFlowSourceKind};
use crate::reconciler::{
    ReconcilePlan, ScopedIdentityArena, ScopedNodeIdentity, plan_diff_in, plan_initial_tree_in,
};

use super::super::super::super::{
    LayoutEngine,
    incremental::ElementVNodeSnapshot,
    text_flow_bridge::{NodeContext, input_from_vnode},
};
use super::super::super::{PostconditionFault, TargetAliasExpectation, set_postcondition_fault_at};

fn raw_text_fixture() -> (LayoutEngine, VNode, ReconcilePlan) {
    let target = VNode::root().child(VNode::text("text").with_key("text"));
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_vnode(&target, 20, 4)
        .expect("text fixture");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let plan = plan_diff_in(&target, &target, &mut arena).expect("text no-op plan");
    (engine, target, plan)
}

fn recompute_taffy_only(engine: &mut LayoutEngine) {
    let root = engine.root_node.expect("root");
    engine
        .taffy
        .compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(engine.last_width as f32),
                height: AvailableSpace::Definite(engine.last_height as f32),
            },
        )
        .expect("Taffy-only recomputation");
}

fn ghost_identity() -> ScopedNodeIdentity {
    let target = VNode::root().child(VNode::box_node().with_key("ghost"));
    plan_initial_tree_in(&target, &mut ScopedIdentityArena::default())
        .expect("ghost plan")
        .root
        .children[0]
        .identity
        .clone()
}

fn element_fixture(text: bool) -> (LayoutEngine, ElementVNodeSnapshot) {
    let mut element = Element::root();
    if text {
        element.add_child(Element::text("text").with_key("text"));
    } else {
        element.add_child(Element::box_element().with_key("box"));
    }
    let mut engine = LayoutEngine::new();
    engine
        .try_compute_element_incremental_transactional(&element, None, 20, 4)
        .expect("element fixture");
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let snapshot =
        ElementVNodeSnapshot::from_element(&element, &mut arena).expect("element snapshot");
    (engine, snapshot)
}

fn expected_text_identities(snapshot: &ElementVNodeSnapshot) -> HashSet<ScopedNodeIdentity> {
    snapshot.text_inputs.keys().cloned().collect()
}

fn assert_element_alias_error(
    engine: &LayoutEngine,
    snapshot: &ElementVNodeSnapshot,
    expected: &HashSet<ScopedNodeIdentity>,
    invariant: IncrementalInvariantError,
) {
    super::super::assert_invariant(
        engine
            .validate_aliases_exact(
                TargetAliasExpectation::Element(snapshot),
                expected,
                snapshot.vnode.key,
            )
            .expect_err("corrupt aliases must fail"),
        invariant,
    );
}

fn install_expected_text_context(
    engine: &mut LayoutEngine,
    node_id: taffy::NodeId,
    input: &TextFlowInput,
) {
    engine
        .taffy
        .set_node_context(
            node_id,
            Some(NodeContext::new(
                Some(input.clone()),
                &engine.text_flow_policy,
            )),
        )
        .expect("replace text context");
    recompute_taffy_only(engine);
}

fn flow_for_current_width(
    engine: &LayoutEngine,
    node_id: taffy::NodeId,
    input: &TextFlowInput,
    width_delta: usize,
) -> Arc<TextFlow> {
    let layout = engine.taffy.layout(node_id).expect("computed layout");
    let inset =
        layout.padding.left + layout.padding.right + layout.border.left + layout.border.right;
    let width = (layout.size.width - inset).max(0.0).floor() as usize + width_delta;
    let options = engine.text_flow_policy.options(input, width);
    Arc::new(TextFlow::try_build(input, &options).expect("valid test flow"))
}

fn validate_text_child(
    engine: &LayoutEngine,
    plan: &ReconcilePlan,
) -> super::super::super::TargetValidationError {
    let planned = &plan.root.children[0];
    engine
        .validate_planned_node_exact(
            planned,
            engine.vnode_map[&planned.identity],
            TargetAliasExpectation::RawVNode,
            false,
            &mut HashSet::new(),
            &mut HashSet::new(),
        )
        .expect_err("corrupt text state must fail")
}

#[test]
fn delayed_fault_and_reachable_target_tail_checks_are_exercised() {
    let (engine, target, plan) = super::super::raw_fixture();
    set_postcondition_fault_at(PostconditionFault::MissingRoot, 1);
    engine
        .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
        .expect("first validation consumes only the delay");
    super::super::assert_invariant(
        engine
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("second validation injects the fault"),
        IncrementalInvariantError::MissingRoot,
    );

    let (mut parented_root, target, plan) = super::super::raw_fixture();
    let root = parented_root.root_node.expect("root");
    let orphan = parented_root
        .taffy
        .new_leaf(taffy::Style::default())
        .expect("orphan parent");
    parented_root
        .taffy
        .add_child(orphan, root)
        .expect("attach committed root below an orphan");
    super::super::assert_invariant(
        parented_root
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("a committed root cannot have a parent"),
        IncrementalInvariantError::InvalidRoot,
    );

    let (mut extra_flow, target, plan) = super::super::raw_fixture();
    extra_flow.current_vnode_flows.insert(
        ghost_identity(),
        super::plain_flow("ghost", &Style::default()),
    );
    super::super::assert_invariant(
        extra_flow
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("an unplanned published flow must fail"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );
}

#[test]
fn planned_text_validation_rejects_each_independent_active_flow_fault() {
    let (mut missing_active, _target, plan) = raw_text_fixture();
    let planned = &plan.root.children[0];
    let node_id = missing_active.vnode_map[&planned.identity];
    let input = input_from_vnode(&planned.vnode).expect("text input");
    install_expected_text_context(&mut missing_active, node_id, &input);
    super::super::assert_invariant(
        validate_text_child(&missing_active, &plan),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut wrong_input, _target, plan) = raw_text_fixture();
    let planned = &plan.root.children[0];
    let node_id = wrong_input.vnode_map[&planned.identity];
    let input = input_from_vnode(&planned.vnode).expect("text input");
    install_expected_text_context(&mut wrong_input, node_id, &input);
    let other_input = TextFlowInput::plain(
        "other",
        TextFlowSourceKind::Exact,
        planned.vnode.props.style.clone(),
    );
    let active = flow_for_current_width(&wrong_input, node_id, &other_input, 0);
    wrong_input
        .taffy
        .get_node_context_mut(node_id)
        .expect("text context")
        .pin_active_flow(&active);
    wrong_input
        .current_vnode_flows
        .insert(planned.identity.clone(), active);
    super::super::assert_invariant(
        validate_text_child(&wrong_input, &plan),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut wrong_options, _target, plan) = raw_text_fixture();
    let planned = &plan.root.children[0];
    let node_id = wrong_options.vnode_map[&planned.identity];
    let input = input_from_vnode(&planned.vnode).expect("text input");
    install_expected_text_context(&mut wrong_options, node_id, &input);
    let active = flow_for_current_width(&wrong_options, node_id, &input, 1);
    wrong_options
        .taffy
        .get_node_context_mut(node_id)
        .expect("text context")
        .pin_active_flow(&active);
    wrong_options
        .current_vnode_flows
        .insert(planned.identity.clone(), active);
    super::super::assert_invariant(
        validate_text_child(&wrong_options, &plan),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut wrong_publication, _target, plan) = raw_text_fixture();
    let planned = &plan.root.children[0];
    let node_id = wrong_publication.vnode_map[&planned.identity];
    let input = input_from_vnode(&planned.vnode).expect("text input");
    install_expected_text_context(&mut wrong_publication, node_id, &input);
    let active = flow_for_current_width(&wrong_publication, node_id, &input, 0);
    let separate = flow_for_current_width(&wrong_publication, node_id, &input, 0);
    wrong_publication
        .taffy
        .get_node_context_mut(node_id)
        .expect("text context")
        .pin_active_flow(&active);
    wrong_publication
        .current_vnode_flows
        .insert(planned.identity.clone(), separate);
    super::super::assert_invariant(
        validate_text_child(&wrong_publication, &plan),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );
}

#[test]
fn non_text_active_flow_and_element_alias_matrix_are_exhaustive() {
    let (mut active_non_text, _target, plan) = super::super::raw_fixture();
    let planned = &plan.root.children[0];
    let node_id = active_non_text.vnode_map[&planned.identity];
    active_non_text
        .taffy
        .set_node_context(
            node_id,
            Some(NodeContext::new(None, &active_non_text.text_flow_policy)),
        )
        .expect("empty non-text context");
    recompute_taffy_only(&mut active_non_text);
    let active = super::plain_flow("impossible", &Style::default());
    active_non_text
        .taffy
        .get_node_context_mut(node_id)
        .expect("non-text context")
        .pin_active_flow(&active);
    super::super::assert_invariant(
        active_non_text
            .validate_planned_node_exact(
                planned,
                node_id,
                TargetAliasExpectation::RawVNode,
                false,
                &mut HashSet::new(),
                &mut HashSet::new(),
            )
            .expect_err("a non-text context cannot publish a flow"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut scopes, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    scopes.element_scopes.remove(
        snapshot
            .element_scopes
            .keys()
            .next()
            .expect("element scope"),
    );
    assert_element_alias_error(
        &scopes,
        &snapshot,
        &expected,
        IncrementalInvariantError::ElementMapMismatch,
    );

    let (mut keys, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    keys.element_keys
        .remove(snapshot.element_keys.keys().next().expect("element key"));
    assert_element_alias_error(
        &keys,
        &snapshot,
        &expected,
        IncrementalInvariantError::ElementMapMismatch,
    );

    let (mut nodes, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    nodes
        .node_map
        .remove(snapshot.element_scopes.keys().next().expect("element node"));
    assert_element_alias_error(
        &nodes,
        &snapshot,
        &expected,
        IncrementalInvariantError::ElementMapMismatch,
    );

    let (mut vnode_len, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    let root = vnode_len.vnode_map[&ScopedNodeIdentity::Root];
    vnode_len.vnode_map.insert(ghost_identity(), root);
    assert_element_alias_error(
        &vnode_len,
        &snapshot,
        &expected,
        IncrementalInvariantError::ElementMapMismatch,
    );

    let (mut missing_vnode, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    let missing = snapshot
        .element_scopes
        .values()
        .find(|identity| **identity != ScopedNodeIdentity::Root)
        .expect("child identity")
        .clone();
    let node = missing_vnode
        .vnode_map
        .remove(&missing)
        .expect("mapped child");
    missing_vnode.vnode_map.insert(ghost_identity(), node);
    assert_element_alias_error(
        &missing_vnode,
        &snapshot,
        &expected,
        IncrementalInvariantError::ElementMapMismatch,
    );

    let (mut wrong_node, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    let child_element = snapshot
        .element_scopes
        .iter()
        .find(|(_, identity)| **identity != ScopedNodeIdentity::Root)
        .map(|(element, _)| *element)
        .expect("child element");
    let root = wrong_node.vnode_map[&ScopedNodeIdentity::Root];
    wrong_node.node_map.insert(child_element, root);
    assert_element_alias_error(
        &wrong_node,
        &snapshot,
        &expected,
        IncrementalInvariantError::ElementMapMismatch,
    );

    let (mut flow_len, snapshot) = element_fixture(false);
    let expected = expected_text_identities(&snapshot);
    flow_len.current_text_flows.insert(
        ElementId::new(),
        super::plain_flow("extra", &Style::default()),
    );
    assert_element_alias_error(
        &flow_len,
        &snapshot,
        &expected,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (text_count, snapshot) = element_fixture(false);
    let expected = HashSet::from([ghost_identity()]);
    assert_element_alias_error(
        &text_count,
        &snapshot,
        &expected,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );
}

#[test]
fn element_text_aliases_reject_pointer_and_membership_mismatches() {
    let (mut pointer, snapshot) = element_fixture(true);
    let expected = expected_text_identities(&snapshot);
    let identity = snapshot
        .text_inputs
        .keys()
        .next()
        .expect("text identity")
        .clone();
    pointer
        .current_vnode_flows
        .insert(identity, super::plain_flow("separate", &Style::default()));
    assert_element_alias_error(
        &pointer,
        &snapshot,
        &expected,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut membership, snapshot) = element_fixture(true);
    let expected = expected_text_identities(&snapshot);
    let identity = snapshot.text_inputs.keys().next().expect("text identity");
    let element = snapshot
        .element_scopes
        .iter()
        .find(|(_, candidate)| *candidate == identity)
        .map(|(element, _)| *element)
        .expect("text element");
    let flow = membership
        .current_text_flows
        .remove(&element)
        .expect("published text flow");
    membership.current_text_flows.insert(ElementId::new(), flow);
    assert_element_alias_error(
        &membership,
        &snapshot,
        &expected,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );
}
