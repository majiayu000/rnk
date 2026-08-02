use std::{collections::HashSet, sync::Arc};

use taffy::{AvailableSpace, NodeId, Size};

use crate::core::{
    AlignItems, AlignSelf, BorderStyle, Color, Dimension, Display, Edges, ElementId, FlexDirection,
    JustifyContent, Overflow, Position, Props, Style, TextWrap, VNode, VNodeType,
};
use crate::layout::{IncrementalInvariantError, TextFlow, TextFlowInput, TextFlowSourceKind};
use crate::reconciler::{ScopedIdentityArena, ScopedNodeIdentity, plan_initial_tree_in};

use super::super::super::{Shared, text_flow_bridge::NodeContext};
use super::super::{
    TargetAliasExpectation, TargetValidationCause, TargetValidationError,
    planned_tree_matches_target, props_snapshots_match, same_dimension, same_edges, same_float,
    same_optional_float, style_snapshots_match,
};

mod remaining;

fn assert_style_difference(mutate: fn(&mut Style)) {
    let left = Style::default();
    let mut right = left.clone();
    mutate(&mut right);
    assert!(!style_snapshots_match(&left, &right));
}

fn plain_flow(source: &str, style: &Style) -> Arc<TextFlow> {
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, style.clone());
    Arc::new(
        TextFlow::try_build(
            &input,
            &crate::layout::TextFlowOptions::new(20, style.text_wrap),
        )
        .expect("valid flow"),
    )
}

#[test]
fn snapshot_comparators_reject_every_independent_field_change() {
    let mutations: &[fn(&mut Style)] = &[
        |style| style.display = Display::None,
        |style| style.position = Position::Absolute,
        |style| style.top = Some(1.0),
        |style| style.right = Some(1.0),
        |style| style.bottom = Some(1.0),
        |style| style.left = Some(1.0),
        |style| style.flex_direction = FlexDirection::Column,
        |style| style.flex_wrap = true,
        |style| style.flex_grow = 1.0,
        |style| style.flex_shrink = 2.0,
        |style| style.flex_basis = Dimension::Points(1.0),
        |style| style.align_items = AlignItems::Center,
        |style| style.align_self = AlignSelf::Center,
        |style| style.justify_content = JustifyContent::Center,
        |style| style.padding.top = 1.0,
        |style| style.padding.right = 1.0,
        |style| style.padding.bottom = 1.0,
        |style| style.padding.left = 1.0,
        |style| style.margin.top = 1.0,
        |style| style.margin.right = 1.0,
        |style| style.margin.bottom = 1.0,
        |style| style.margin.left = 1.0,
        |style| style.gap = 1.0,
        |style| style.row_gap = Some(1.0),
        |style| style.column_gap = Some(1.0),
        |style| style.width = Dimension::Points(1.0),
        |style| style.height = Dimension::Points(1.0),
        |style| style.min_width = Dimension::Points(1.0),
        |style| style.min_height = Dimension::Points(1.0),
        |style| style.max_width = Dimension::Points(1.0),
        |style| style.max_height = Dimension::Points(1.0),
        |style| style.border_style = BorderStyle::Single,
        |style| style.border_color = Some(Color::Red),
        |style| style.border_top_color = Some(Color::Red),
        |style| style.border_right_color = Some(Color::Red),
        |style| style.border_bottom_color = Some(Color::Red),
        |style| style.border_left_color = Some(Color::Red),
        |style| style.border_dim = true,
        |style| style.border_top = false,
        |style| style.border_bottom = false,
        |style| style.border_left = false,
        |style| style.border_right = false,
        |style| style.color = Some(Color::Red),
        |style| style.background_color = Some(Color::Blue),
        |style| style.bold = true,
        |style| style.italic = true,
        |style| style.underline = true,
        |style| style.strikethrough = true,
        |style| style.dim = true,
        |style| style.inverse = true,
        |style| style.text_wrap = TextWrap::Truncate,
        |style| style.overflow_x = Overflow::Hidden,
        |style| style.overflow_y = Overflow::Scroll,
        |style| style.is_static = true,
    ];
    for mutate in mutations {
        assert_style_difference(*mutate);
    }

    assert!(!same_edges(
        Edges::default(),
        Edges::new(1.0, 0.0, 0.0, 0.0)
    ));
    assert!(!same_edges(
        Edges::default(),
        Edges::new(0.0, 1.0, 0.0, 0.0)
    ));
    assert!(!same_edges(
        Edges::default(),
        Edges::new(0.0, 0.0, 1.0, 0.0)
    ));
    assert!(!same_edges(
        Edges::default(),
        Edges::new(0.0, 0.0, 0.0, 1.0)
    ));
    assert!(same_dimension(
        Dimension::Points(f32::NAN),
        Dimension::Points(f32::NAN)
    ));
    assert!(same_dimension(
        Dimension::Percent(f32::NAN),
        Dimension::Percent(f32::NAN)
    ));
    assert!(!same_dimension(Dimension::Auto, Dimension::Points(0.0)));
    assert!(same_optional_float(Some(f32::NAN), Some(f32::NAN)));
    assert!(!same_optional_float(Some(0.0), None));
    assert!(!same_float(0.0, 1.0));
}

#[test]
fn planned_tree_and_props_comparators_cover_each_short_circuit() {
    let target = VNode::root().child(VNode::box_node().with_key("child"));
    let plan =
        plan_initial_tree_in(&target, &mut ScopedIdentityArena::default()).expect("target plan");
    assert!(planned_tree_matches_target(&plan.root, &target));

    let mut changed = target.clone();
    changed.key = VNode::box_node().key;
    assert!(!planned_tree_matches_target(&plan.root, &changed));
    let mut changed = target.clone();
    changed.node_type = VNodeType::Box;
    assert!(!planned_tree_matches_target(&plan.root, &changed));
    let mut changed = target.clone();
    changed.props.scroll_offset_y = Some(1);
    assert!(!planned_tree_matches_target(&plan.root, &changed));
    let mut changed = target.clone();
    changed.children.clear();
    assert!(!planned_tree_matches_target(&plan.root, &changed));
    let mut changed = target.clone();
    changed.children[0].key = VNode::text("other").key;
    assert!(!planned_tree_matches_target(&plan.root, &changed));

    let left = Props::default();
    let mut right = left.clone();
    right.key = Some("other".into());
    assert!(!props_snapshots_match(&left, &right));
    let mut right = left.clone();
    right.scroll_offset_x = Some(1);
    assert!(!props_snapshots_match(&left, &right));
    let mut right = left.clone();
    right.scroll_offset_y = Some(1);
    assert!(!props_snapshots_match(&left, &right));
}

#[test]
fn target_validator_covers_mapping_viewport_and_flow_error_details() {
    let (mut missing_committed, target, plan) = super::raw_fixture();
    missing_committed.committed_vnode = Shared::new(None);
    super::assert_invariant(
        missing_committed
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("missing committed target"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut height, target, plan) = super::raw_fixture();
    height.last_height = 3;
    super::assert_invariant(
        height
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("stale height"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut legacy_len, target, plan) = super::raw_fixture();
    legacy_len
        .vnode_legacy_keys
        .remove(&plan.root.children[0].identity);
    super::assert_invariant(
        legacy_len
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("legacy map length mismatch"),
        IncrementalInvariantError::ScopedMapMismatch,
    );

    let (duplicate, target, mut plan) = super::raw_fixture();
    plan.root.children[1].identity = plan.root.children[0].identity.clone();
    super::assert_invariant(
        duplicate
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("duplicate planned identity"),
        IncrementalInvariantError::ScopedMapMismatch,
    );

    let (mut missing_equal_len, target, plan) = super::raw_fixture();
    let missing = plan.root.children[0].identity.clone();
    let removed = missing_equal_len
        .vnode_map
        .remove(&missing)
        .expect("mapped child");
    let ghost = VNode::root().child(VNode::box_node().with_key("ghost"));
    let ghost_plan =
        plan_initial_tree_in(&ghost, &mut ScopedIdentityArena::default()).expect("ghost plan");
    missing_equal_len
        .vnode_map
        .insert(ghost_plan.root.children[0].identity.clone(), removed);
    super::assert_invariant(
        missing_equal_len
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("equal-length map misses a planned identity"),
        IncrementalInvariantError::ScopedMapMismatch,
    );

    let (mut stale_flows, target, plan) = super::raw_fixture();
    stale_flows.current_vnode_flows.insert(
        ScopedNodeIdentity::Root,
        plain_flow("ghost", &Style::default()),
    );
    super::assert_invariant(
        stale_flows
            .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
            .expect_err("unexpected current flow"),
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (engine, target, _) = super::raw_fixture();
    super::assert_invariant(
        engine
            .reachable_target_nodes(NodeId::new(u64::MAX), target.key)
            .expect_err("invalid traversal root"),
        IncrementalInvariantError::InvalidMappedNode,
    );
}

#[test]
fn planned_node_validator_covers_style_projection_context_and_non_text_failures() {
    let (mut stale_style, _target, plan) = super::raw_fixture();
    let planned = &plan.root.children[0];
    let node_id = stale_style.vnode_map[&planned.identity];
    let mut style = planned.vnode.props.style.clone();
    style.width = Dimension::Points(9.0);
    stale_style
        .taffy
        .set_style(
            node_id,
            super::super::super::normalized_taffy_style(&style, false),
        )
        .expect("stale style");
    stale_style
        .run_layout_and_publish_checked(&mut || false)
        .expect("stale style computes");
    let error = stale_style
        .validate_planned_node_exact(
            planned,
            node_id,
            TargetAliasExpectation::RawVNode,
            &mut HashSet::new(),
            &mut HashSet::new(),
        )
        .expect_err("backend style differs from target");
    super::assert_invariant(
        error,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (engine, _target, plan) = super::raw_fixture();
    let planned = &plan.root.children[0];
    let node_id = engine.vnode_map[&planned.identity];
    let mut projections = HashSet::new();
    engine
        .validate_planned_node_exact(
            planned,
            node_id,
            TargetAliasExpectation::RawVNode,
            &mut projections,
            &mut HashSet::new(),
        )
        .expect("first projection");
    let error = engine
        .validate_planned_node_exact(
            planned,
            node_id,
            TargetAliasExpectation::RawVNode,
            &mut projections,
            &mut HashSet::new(),
        )
        .expect_err("duplicate compatibility projection");
    super::assert_invariant(error, IncrementalInvariantError::CompatibilityMapMismatch);

    let (mut no_context, _target, plan) = super::raw_fixture();
    let planned = &plan.root.children[0];
    let node_id = no_context.vnode_map[&planned.identity];
    no_context
        .taffy
        .set_node_context(node_id, None)
        .expect("remove context");
    let root = no_context.root_node.expect("root");
    no_context
        .taffy
        .compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(no_context.last_width as f32),
                height: AvailableSpace::Definite(no_context.last_height as f32),
            },
        )
        .expect("Taffy computes without the engine context read-back");
    let error = no_context
        .validate_planned_node_exact(
            planned,
            node_id,
            TargetAliasExpectation::RawVNode,
            &mut HashSet::new(),
            &mut HashSet::new(),
        )
        .expect_err("mapped node context is required");
    super::assert_invariant(
        error,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );

    let (mut polluted, _target, plan) = super::raw_fixture();
    let planned = &plan.root.children[0];
    let node_id = polluted.vnode_map[&planned.identity];
    let input = TextFlowInput::plain("polluted", TextFlowSourceKind::Exact, Style::default());
    polluted
        .taffy
        .set_node_context(
            node_id,
            Some(NodeContext::new(Some(input), &polluted.text_flow_policy)),
        )
        .expect("pollute non-text context");
    let root = polluted.root_node.expect("root");
    polluted
        .taffy
        .compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(polluted.last_width as f32),
                height: AvailableSpace::Definite(polluted.last_height as f32),
            },
        )
        .expect("Taffy computes with a polluted non-text context");
    let error = polluted
        .validate_planned_node_exact(
            planned,
            node_id,
            TargetAliasExpectation::RawVNode,
            &mut HashSet::new(),
            &mut HashSet::new(),
        )
        .expect_err("non-text context must stay empty");
    super::assert_invariant(
        error,
        IncrementalInvariantError::CurrentFrameContextMismatch,
    );
}

#[test]
fn validation_error_and_raw_alias_mappers_cover_taffy_and_each_map() {
    let source = taffy::TaffyError::InvalidInputNode(NodeId::new(u64::MAX));
    let error = TargetValidationError::taffy(None, source.clone());
    assert!(matches!(error.source, TargetValidationCause::Taffy(actual) if actual == source));

    let mut cases = Vec::new();
    for discriminator in 0..4 {
        let (mut engine, target, plan) = super::raw_fixture();
        let root = engine.root_node.expect("root");
        let element_id = ElementId::new();
        match discriminator {
            0 => {
                engine.node_map.insert(element_id, root);
            }
            1 => {
                engine.element_keys.insert(element_id, target.key);
            }
            2 => {
                engine
                    .element_scopes
                    .insert(element_id, ScopedNodeIdentity::Root);
            }
            3 => {
                engine
                    .current_text_flows
                    .insert(element_id, plain_flow("extra", &Style::default()));
            }
            _ => unreachable!(),
        }
        cases.push((engine, target, plan));
    }
    for (engine, target, plan) in cases {
        super::assert_invariant(
            engine
                .validate_target_exact(&plan, TargetAliasExpectation::RawVNode, &target, 20, 4)
                .expect_err("raw target cannot retain element aliases"),
            IncrementalInvariantError::ElementMapMismatch,
        );
    }
}
