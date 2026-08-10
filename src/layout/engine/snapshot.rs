//! Target-exact snapshot adapter over a validated GH59 plan and GH60 candidate.

use std::collections::{HashMap, HashSet};

use crate::{
    core::{Display, Element, ElementId, ElementType, Overflow, VNode},
    layout::{
        PatchTransactionError,
        snapshot::{
            AttemptedContentBounds, Axis, AxisClip, CellPoint, CellRect, CellVector,
            CheckedSnapshotNodeInput, Edge, GeometryField, LayoutSnapshotBuilder,
            LayoutSnapshotError, PreparedSnapshotFrame, SnapshotBuildFailure, SnapshotBuildReport,
            SnapshotBuildStrategy, SnapshotCounterError, SnapshotIdentity, SnapshotInvariantError,
            SnapshotNodeIndex, SnapshotTargetMismatchReason, SnapshotWorkCounterField,
            SnapshotWorkCounters, TextFlowSemanticStamp, checked_add, checked_extent,
            checked_finite, checked_subtract, quantize_edge, quantize_rect,
        },
    },
    reconciler::{
        PlannedNode, PlannedNodeAction, ReconcilePlan, ScopedIdentityArena, ScopedNodeIdentity,
        semantically_equal_vnode_in,
    },
};

use super::{LayoutEngine, incremental::ElementVNodeSnapshot};

#[derive(Debug, Clone)]
pub(super) struct SnapshotProducerEvidence {
    pub(super) strategy: SnapshotBuildStrategy,
    pub(super) patch_count: usize,
    pub(super) recovery_cause: Option<PatchTransactionError>,
    pub(super) pre_snapshot_mutations: usize,
    pub(super) mutate_each_visited_node: bool,
    pub(super) text_flow_recomputes: [Option<u64>; 2],
    pub(super) rebuild_count: u64,
}

pub(super) struct SnapshotTargetPlan<'a> {
    target: &'a Element,
    element_scopes: &'a HashMap<ElementId, ScopedNodeIdentity>,
    final_children: HashMap<ScopedNodeIdentity, Vec<ScopedNodeIdentity>>,
}

impl SnapshotProducerEvidence {
    pub(super) fn initial(text_flow_recomputes: Option<u64>) -> Self {
        Self {
            strategy: SnapshotBuildStrategy::InitialFull,
            patch_count: 0,
            recovery_cause: None,
            pre_snapshot_mutations: 0,
            mutate_each_visited_node: true,
            text_flow_recomputes: [text_flow_recomputes, Some(0)],
            rebuild_count: 0,
        }
    }

    pub(super) fn incremental(
        patch_count: usize,
        mutations: usize,
        text_flow_recomputes: Option<u64>,
    ) -> Self {
        Self {
            strategy: SnapshotBuildStrategy::Incremental,
            patch_count,
            recovery_cause: None,
            pre_snapshot_mutations: mutations,
            mutate_each_visited_node: false,
            text_flow_recomputes: [text_flow_recomputes, Some(0)],
            rebuild_count: 0,
        }
    }

    pub(super) fn recovered(
        patch_count: usize,
        recovery_cause: PatchTransactionError,
        failed_mutations: usize,
        failed_text_flow_recomputes: Option<u64>,
        recovered_text_flow_recomputes: Option<u64>,
    ) -> Self {
        Self {
            strategy: SnapshotBuildStrategy::RecoveredFull,
            patch_count,
            recovery_cause: Some(recovery_cause),
            pre_snapshot_mutations: failed_mutations,
            mutate_each_visited_node: true,
            text_flow_recomputes: [failed_text_flow_recomputes, recovered_text_flow_recomputes],
            rebuild_count: 1,
        }
    }
}

impl<'a> SnapshotTargetPlan<'a> {
    pub(super) fn new(
        target: &'a Element,
        snapshot: &'a ElementVNodeSnapshot,
        plan: &ReconcilePlan,
    ) -> Result<Self, LayoutSnapshotError> {
        let final_children = plan
            .parents
            .iter()
            .map(|parent| (parent.parent.clone(), parent.final_children.clone()))
            .collect();
        let target_plan = Self {
            target,
            element_scopes: &snapshot.element_scopes,
            final_children,
        };
        target_plan.validate_node(target)?;
        Ok(target_plan)
    }

    fn validate_node(&self, element: &Element) -> Result<(), LayoutSnapshotError> {
        if element.element_type == ElementType::VirtualText {
            return Ok(());
        }
        let identity = self.identity(element)?;
        let actual: Vec<_> = element
            .children
            .iter()
            .filter(|child| child.element_type != ElementType::VirtualText)
            .map(|child| self.identity(child))
            .collect::<Result<_, _>>()?;
        let expected = self.final_children.get(identity.scoped()).ok_or_else(|| {
            LayoutSnapshotError::InvalidTree {
                identity: Some(identity.clone()),
                source: SnapshotInvariantError::SnapshotTargetMismatch {
                    identity: identity.clone(),
                    reason: SnapshotTargetMismatchReason::ChildOrder,
                },
            }
        })?;
        if actual
            .iter()
            .map(SnapshotIdentity::scoped)
            .ne(expected.iter())
        {
            let child = actual
                .iter()
                .zip(expected)
                .find_map(|(actual, expected)| {
                    (actual.scoped() != expected).then(|| actual.clone())
                })
                .or_else(|| actual.last().cloned())
                .unwrap_or_else(|| identity.clone());
            return Err(LayoutSnapshotError::InvalidTree {
                identity: Some(child.clone()),
                source: SnapshotInvariantError::ChildOrderMismatch {
                    parent: identity,
                    child,
                    expected_index: expected.len(),
                    actual_index: actual.len(),
                },
            });
        }
        for child in &element.children {
            self.validate_node(child)?;
        }
        Ok(())
    }

    fn identity(&self, element: &Element) -> Result<SnapshotIdentity, LayoutSnapshotError> {
        self.element_scopes
            .get(&element.id)
            .cloned()
            .map(SnapshotIdentity::from_scoped)
            .ok_or(LayoutSnapshotError::MissingIdentity {
                element_id: element.id,
            })
    }
}

impl LayoutEngine {
    /// Return the already-published snapshot for this exact committed frame.
    ///
    /// This compatibility accessor never rebuilds geometry from live engine
    /// maps. A stale target alias or an engine without a published snapshot is
    /// rejected.
    pub fn try_snapshot(
        &self,
        target: &Element,
    ) -> Result<(PreparedSnapshotFrame, SnapshotBuildReport), LayoutSnapshotError> {
        let snapshot =
            self.published_snapshot
                .as_ref()
                .ok_or(LayoutSnapshotError::MissingIdentity {
                    element_id: target.id,
                })?;
        self.validate_committed_target(target, snapshot)?;
        let report = self.published_snapshot_report.as_ref().ok_or(
            LayoutSnapshotError::MissingIdentity {
                element_id: target.id,
            },
        )?;
        Ok((snapshot.clone(), report.clone()))
    }

    fn validate_committed_target(
        &self,
        target: &Element,
        snapshot: &PreparedSnapshotFrame,
    ) -> Result<(), LayoutSnapshotError> {
        let mut arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
        let requested = ElementVNodeSnapshot::from_element(target, &mut arena).map_err(|_| {
            LayoutSnapshotError::MissingIdentity {
                element_id: target.id,
            }
        })?;
        let requested_visible = visible_vnode(&requested.vnode);
        let committed_visible = self.committed_vnode.as_ref().and_then(visible_vnode);
        let exact = requested.has_layout_root
            && match (committed_visible, requested_visible) {
                (Some(committed), Some(requested)) => {
                    semantically_equal_vnode_in(&committed, &requested, &mut arena).unwrap_or(false)
                }
                (None, None) => true,
                _ => false,
            };
        if !exact {
            let identity = SnapshotIdentity::from_scoped(ScopedNodeIdentity::Root);
            return Err(LayoutSnapshotError::InvalidTree {
                identity: Some(identity.clone()),
                source: SnapshotInvariantError::SnapshotTargetMismatch {
                    identity,
                    reason: SnapshotTargetMismatchReason::ChildOrder,
                },
            });
        }
        validate_committed_aliases(target, &requested.element_scopes, self, snapshot)
    }

    pub(super) fn staged_clone(&self) -> Self {
        Self {
            taffy: self.taffy.clone(),
            node_map: self.node_map.clone(),
            element_keys: self.element_keys.clone(),
            element_scopes: self.element_scopes.clone(),
            vnode_map: self.vnode_map.clone(),
            vnode_legacy_keys: self.vnode_legacy_keys.clone(),
            root_node: self.root_node,
            last_width: self.last_width,
            last_height: self.last_height,
            flow_cache: self.flow_cache.clone(),
            text_flow_policy: self.text_flow_policy.clone(),
            current_text_flows: self.current_text_flows.clone(),
            current_vnode_flows: self.current_vnode_flows.clone(),
            committed_vnode: self.committed_vnode.clone(),
            commit_epoch: self.commit_epoch.clone(),
            published_snapshot: self.published_snapshot.clone(),
            published_snapshot_report: self.published_snapshot_report.clone(),
        }
    }

    pub(super) fn stage_prepared_snapshot(
        &mut self,
        snapshot: PreparedSnapshotFrame,
        report: SnapshotBuildReport,
    ) {
        self.published_snapshot = Some(snapshot);
        self.published_snapshot_report = Some(report);
    }

    pub(super) fn prepared_snapshot(&self) -> &PreparedSnapshotFrame {
        self.published_snapshot
            .as_ref()
            .expect("validated candidate carries its prepared snapshot")
    }

    pub(super) fn prepared_snapshot_report(&self) -> &SnapshotBuildReport {
        self.published_snapshot_report
            .as_ref()
            .expect("validated candidate carries its snapshot report")
    }

    pub(super) fn try_build_snapshot_for(
        &self,
        target_plan: &SnapshotTargetPlan<'_>,
        evidence: &SnapshotProducerEvidence,
    ) -> Result<(PreparedSnapshotFrame, SnapshotBuildReport), SnapshotBuildFailure> {
        let mut builder = LayoutSnapshotBuilder::new(self.last_width, self.last_height, 1);
        add_pre_snapshot_work(&mut builder, evidence)?;
        let viewport_clip = AxisClip::from_rect(builder.viewport());
        self.snapshot_subtree(
            target_plan.target,
            target_plan,
            evidence,
            None,
            0.0,
            0.0,
            viewport_clip,
            &mut builder,
        )?;
        builder.finish(
            evidence.strategy,
            evidence.patch_count,
            evidence.recovery_cause.clone(),
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn snapshot_subtree(
        &self,
        element: &Element,
        target_plan: &SnapshotTargetPlan<'_>,
        evidence: &SnapshotProducerEvidence,
        parent: Option<SnapshotNodeIndex>,
        parent_child_x: f64,
        parent_child_y: f64,
        inherited_clip: AxisClip,
        builder: &mut LayoutSnapshotBuilder,
    ) -> Result<Option<SnapshotNodeIndex>, SnapshotBuildFailure> {
        if element.style.display == Display::None
            || element.element_type == ElementType::VirtualText
        {
            return Ok(None);
        }
        let identity = target_plan
            .identity(element)
            .map_err(|source| builder.fail(source))?;
        if self.element_scopes.get(&element.id) != Some(identity.scoped()) {
            return Err(builder.fail(LayoutSnapshotError::MissingIdentity {
                element_id: element.id,
            }));
        }
        builder.add_work(SnapshotWorkCounters::from_fields(
            1,
            u64::from(evidence.mutate_each_visited_node),
            0,
            0,
            0,
        ))?;
        match self.try_get_required_layout(element.id) {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                return Err(builder.fail(LayoutSnapshotError::MissingLayout { identity }));
            }
        }
        let node_id = self.node_map[&element.id];
        let layout = self.taffy.unrounded_layout(node_id);
        let local_x = checked_finite(&identity, GeometryField::X, layout.location.x)
            .map_err(|source| builder.fail(source))?;
        let local_y = checked_finite(&identity, GeometryField::Y, layout.location.y)
            .map_err(|source| builder.fail(source))?;
        let width = checked_extent(&identity, Axis::X, GeometryField::Width, layout.size.width)
            .map_err(|source| builder.fail(source))?;
        let height = checked_extent(
            &identity,
            Axis::Y,
            GeometryField::Height,
            layout.size.height,
        )
        .map_err(|source| builder.fail(source))?;
        let absolute_left = checked_add(&identity, parent_child_x, local_x)
            .map_err(|source| builder.fail(source))?;
        let absolute_top = checked_add(&identity, parent_child_y, local_y)
            .map_err(|source| builder.fail(source))?;
        let absolute_right =
            checked_add(&identity, absolute_left, width).map_err(|source| builder.fail(source))?;
        let absolute_bottom =
            checked_add(&identity, absolute_top, height).map_err(|source| builder.fail(source))?;
        let border_bounds = quantize_rect(
            &identity,
            absolute_left,
            absolute_top,
            absolute_right,
            absolute_bottom,
        )
        .map_err(|source| builder.fail(source))?;

        let insets =
            checked_insets(element, &identity, layout).map_err(|source| builder.fail(source))?;
        let content_left = checked_add(&identity, absolute_left, insets.0)
            .map_err(|source| builder.fail(source))?;
        let content_top = checked_add(&identity, absolute_top, insets.1)
            .map_err(|source| builder.fail(source))?;
        let content_right = checked_subtract(&identity, absolute_right, insets.2)
            .map_err(|source| builder.fail(source))?;
        let content_bottom = checked_subtract(&identity, absolute_bottom, insets.3)
            .map_err(|source| builder.fail(source))?;
        let (attempted_content, content_bounds) = checked_content_bounds(
            &identity,
            border_bounds,
            content_left,
            content_top,
            content_right,
            content_bottom,
        )
        .map_err(|source| builder.fail(source))?;
        let text_origin = CellPoint::checked(attempted_content.left(), attempted_content.top());
        let effective_clip = effective_clip(element, inherited_clip, content_bounds);
        let scroll_x = i32::from(element.scroll_offset_x.unwrap_or(0));
        let scroll_y = i32::from(element.scroll_offset_y.unwrap_or(0));
        let scroll_transform = CellVector::checked(-scroll_x, -scroll_y);
        let text_flow = if element.element_type == ElementType::Text {
            Some(TextFlowSemanticStamp::checked(
                self.current_text_flow(element.id).ok_or_else(|| {
                    builder.fail(LayoutSnapshotError::MissingTextFlowRevision {
                        identity: identity.clone(),
                    })
                })?,
            ))
        } else {
            None
        };
        let index = builder.push_ordered(CheckedSnapshotNodeInput {
            element_id: element.id,
            identity: identity.clone(),
            parent,
            border_bounds,
            content_bounds,
            text_origin,
            effective_clip,
            scroll_transform,
            text_flow,
        })?;
        let child_origin_x = checked_subtract(&identity, absolute_left, f64::from(scroll_x))
            .map_err(|source| builder.fail(source))?;
        let child_origin_y = checked_subtract(&identity, absolute_top, f64::from(scroll_y))
            .map_err(|source| builder.fail(source))?;
        for child in &element.children {
            self.snapshot_subtree(
                child,
                target_plan,
                evidence,
                Some(index),
                child_origin_x,
                child_origin_y,
                effective_clip,
                builder,
            )?;
        }
        Ok(Some(index))
    }
}

fn visible_vnode(vnode: &VNode) -> Option<VNode> {
    if vnode.props.style.display == Display::None {
        return None;
    }
    let mut visible = vnode.clone();
    visible.children = vnode.children.iter().filter_map(visible_vnode).collect();
    Some(visible)
}

fn validate_committed_aliases(
    element: &Element,
    requested_scopes: &HashMap<ElementId, ScopedNodeIdentity>,
    engine: &LayoutEngine,
    snapshot: &PreparedSnapshotFrame,
) -> Result<(), LayoutSnapshotError> {
    if element.style.display == Display::None || element.element_type == ElementType::VirtualText {
        return Ok(());
    }
    let node = snapshot.node_for_element(element.id).map_err(|_| {
        LayoutSnapshotError::MissingIdentity {
            element_id: element.id,
        }
    })?;
    let requested = requested_scopes.get(&element.id);
    if requested != engine.element_scopes.get(&element.id)
        || requested != Some(node.identity().scoped())
    {
        return Err(LayoutSnapshotError::MissingIdentity {
            element_id: element.id,
        });
    }
    for child in &element.children {
        validate_committed_aliases(child, requested_scopes, engine, snapshot)?;
    }
    Ok(())
}

fn add_pre_snapshot_work(
    builder: &mut LayoutSnapshotBuilder,
    evidence: &SnapshotProducerEvidence,
) -> Result<(), SnapshotBuildFailure> {
    let mutations = u64::try_from(evidence.pre_snapshot_mutations).map_err(|_| {
        builder.fail(LayoutSnapshotError::WorkCounters {
            source: SnapshotCounterError::Overflow {
                field: SnapshotWorkCounterField::MutatedNodes,
                lhs: 0,
                rhs: u64::MAX,
            },
        })
    })?;
    builder.add_work(SnapshotWorkCounters::from_fields(
        0,
        mutations,
        0,
        0,
        evidence.rebuild_count,
    ))?;
    for recomputes in evidence.text_flow_recomputes {
        let recomputes = recomputes.ok_or_else(|| {
            builder.fail(LayoutSnapshotError::WorkCounters {
                source: SnapshotCounterError::Overflow {
                    field: SnapshotWorkCounterField::TextFlowRecomputes,
                    lhs: u64::MAX,
                    rhs: 1,
                },
            })
        })?;
        builder.add_work(SnapshotWorkCounters::from_fields(0, 0, recomputes, 0, 0))?;
    }
    Ok(())
}

fn checked_insets(
    element: &Element,
    identity: &SnapshotIdentity,
    layout: &taffy::Layout,
) -> Result<(f64, f64, f64, f64), LayoutSnapshotError> {
    let sum = |field, border: f32, padding: f32| {
        checked_add(
            identity,
            checked_finite(identity, field, border)?,
            checked_finite(identity, field, padding)?,
        )
    };
    Ok((
        sum(
            GeometryField::LeftInset,
            layout.border.left,
            element.style.padding.left,
        )?,
        sum(
            GeometryField::TopInset,
            layout.border.top,
            element.style.padding.top,
        )?,
        sum(
            GeometryField::RightInset,
            layout.border.right,
            element.style.padding.right,
        )?,
        sum(
            GeometryField::BottomInset,
            layout.border.bottom,
            element.style.padding.bottom,
        )?,
    ))
}

fn checked_content_bounds(
    identity: &SnapshotIdentity,
    border: CellRect,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
) -> Result<(AttemptedContentBounds, CellRect), LayoutSnapshotError> {
    let left = quantize_edge(identity, Edge::Left, left)?;
    let top = quantize_edge(identity, Edge::Top, top)?;
    let right = quantize_edge(identity, Edge::Right, right)?;
    let bottom = quantize_edge(identity, Edge::Bottom, bottom)?;
    let attempted = AttemptedContentBounds::from_raw(left, top, right, bottom);
    if left > right || top > bottom {
        return Err(LayoutSnapshotError::ReversedContentBounds {
            identity: identity.clone(),
            border_bounds: border,
            attempted_content_bounds: attempted,
        });
    }
    let ordered = CellRect::checked(left, top, right, bottom).ok_or(
        LayoutSnapshotError::CellSpanOverflow {
            identity: identity.clone(),
            axis: if i64::from(right) - i64::from(left) > i64::from(i32::MAX) {
                Axis::X
            } else {
                Axis::Y
            },
            start: if i64::from(right) - i64::from(left) > i64::from(i32::MAX) {
                left
            } else {
                top
            },
            end: if i64::from(right) - i64::from(left) > i64::from(i32::MAX) {
                right
            } else {
                bottom
            },
        },
    )?;
    Ok((attempted, ordered.intersect(border)))
}

fn effective_clip(element: &Element, inherited: AxisClip, content: CellRect) -> AxisClip {
    let x = if matches!(
        element.style.overflow_x,
        Overflow::Hidden | Overflow::Scroll
    ) {
        inherited.x().intersect(content.x_span())
    } else {
        inherited.x()
    };
    let y = if matches!(
        element.style.overflow_y,
        Overflow::Hidden | Overflow::Scroll
    ) {
        inherited.y().intersect(content.y_span())
    } else {
        inherited.y()
    };
    AxisClip::checked(x, y)
}

pub(super) fn recomputes_since(candidate: &LayoutEngine, base: &LayoutEngine) -> Option<u64> {
    candidate
        .flow_cache
        .successful_recomputes()
        .checked_sub(base.flow_cache.successful_recomputes())
}

pub(super) fn count_plan_mutations(plan: &ReconcilePlan) -> usize {
    fn collect(node: &PlannedNode, identities: &mut HashSet<ScopedNodeIdentity>) {
        if node.action != PlannedNodeAction::Reuse {
            identities.insert(node.identity.clone());
        }
        for child in &node.children {
            collect(child, identities);
        }
    }
    let mut identities = HashSet::new();
    collect(&plan.root, &mut identities);
    for parent in &plan.parents {
        identities.extend(parent.removals.iter().cloned());
    }
    identities.len()
}
