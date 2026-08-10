#![forbid(missing_docs)]

//! Immutable, producer-independent terminal-cell layout snapshots.

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{core::ElementId, layout::TextFlow, reconciler::ScopedNodeIdentity};

mod error;
mod quantize;

pub use error::{
    ArithmeticOperation, Axis, CellOutputError, Edge, GeometryField, LayoutAliasError,
    LayoutSnapshotError, SnapshotInvariantError, SnapshotTargetMismatchReason,
};
pub(crate) use quantize::{
    add as checked_add, extent as checked_extent, finite as checked_finite, rect as quantize_rect,
    subtract as checked_subtract,
};

static NEXT_FRAME_REVISION: AtomicU64 = AtomicU64::new(1);

/// Opaque, exact semantic identity of one node in a layout snapshot.
///
/// Identities can be compared and used as map keys, but callers cannot forge
/// one or couple application logic to reconciler storage details.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnapshotIdentity(ScopedNodeIdentity);

impl SnapshotIdentity {
    pub(crate) fn from_scoped(identity: ScopedNodeIdentity) -> Self {
        Self(identity)
    }

    /// Stable diagnostic label suitable for error messages and test output.
    pub fn diagnostic(&self) -> String {
        self.0.diagnostic()
    }

    pub(crate) fn scoped(&self) -> &ScopedNodeIdentity {
        &self.0
    }
}

/// Signed point in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPoint {
    x: i32,
    y: i32,
}

impl CellPoint {
    pub(crate) const fn checked(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    /// Horizontal cell coordinate.
    pub const fn x(self) -> i32 {
        self.x
    }
    /// Vertical cell coordinate.
    pub const fn y(self) -> i32 {
        self.y
    }
}

/// Signed translation in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellVector {
    dx: i32,
    dy: i32,
}

impl CellVector {
    pub(crate) const fn checked(dx: i32, dy: i32) -> Self {
        Self { dx, dy }
    }
    /// Horizontal translation.
    pub const fn dx(self) -> i32 {
        self.dx
    }
    /// Vertical translation.
    pub const fn dy(self) -> i32 {
        self.dy
    }
}

/// Signed half-open rectangle in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl CellRect {
    pub(crate) const fn checked(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    /// Left edge, inclusive.
    pub const fn left(self) -> i32 {
        self.left
    }
    /// Top edge, inclusive.
    pub const fn top(self) -> i32 {
        self.top
    }
    /// Right edge, exclusive.
    pub const fn right(self) -> i32 {
        self.right
    }
    /// Bottom edge, exclusive.
    pub const fn bottom(self) -> i32 {
        self.bottom
    }
    /// Width derived from the same quantized edge pair.
    pub const fn width(self) -> i32 {
        self.right - self.left
    }
    /// Height derived from the same quantized edge pair.
    pub const fn height(self) -> i32 {
        self.bottom - self.top
    }
    /// Top-left point.
    pub const fn origin(self) -> CellPoint {
        CellPoint::checked(self.left, self.top)
    }

    pub(crate) fn intersect(self, other: Self) -> Self {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        Self::checked(
            left,
            top,
            self.right.min(other.right).max(left),
            self.bottom.min(other.bottom).max(top),
        )
    }
}

/// Signed half-open span on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpan {
    start: i32,
    end: i32,
}

impl CellSpan {
    pub(crate) const fn checked(start: i32, end: i32) -> Self {
        Self { start, end }
    }
    /// Inclusive start edge.
    pub const fn start(self) -> i32 {
        self.start
    }
    /// Exclusive end edge.
    pub const fn end(self) -> i32 {
        self.end
    }
    /// Whether the span contains no cells.
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
    pub(crate) fn intersect(self, other: Self) -> Self {
        let start = self.start.max(other.start);
        Self::checked(start, self.end.min(other.end).max(start))
    }
}

/// Independent horizontal and vertical effective clip spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisClip {
    x: CellSpan,
    y: CellSpan,
}

impl AxisClip {
    pub(crate) const fn checked(x: CellSpan, y: CellSpan) -> Self {
        Self { x, y }
    }
    pub(crate) const fn from_rect(rect: CellRect) -> Self {
        Self::checked(
            CellSpan::checked(rect.left, rect.right),
            CellSpan::checked(rect.top, rect.bottom),
        )
    }
    /// Horizontal clip span.
    pub const fn x(self) -> CellSpan {
        self.x
    }
    /// Vertical clip span.
    pub const fn y(self) -> CellSpan {
        self.y
    }
}

/// Opaque index into one snapshot's preorder node array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapshotNodeIndex(usize);

impl SnapshotNodeIndex {
    pub(crate) const fn checked(index: usize) -> Self {
        Self(index)
    }
    /// Zero-based preorder position.
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Semantic TextFlow identity attached to one snapshot node.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFlowSemanticStamp {
    flow: Arc<TextFlow>,
}

impl TextFlowSemanticStamp {
    pub(crate) fn checked(flow: Arc<TextFlow>) -> Self {
        Self { flow }
    }
    /// Maximum content width used to build the flow.
    pub fn max_width(&self) -> usize {
        self.flow.cache_identity().options.max_width
    }
    /// Unicode width policy revision used to build the flow.
    pub fn width_policy_revision(&self) -> u16 {
        self.flow.cache_identity().options.width_policy.revision
    }
    /// Number of logical rows.
    pub fn logical_row_count(&self) -> usize {
        self.flow.logical_rows().len()
    }
    pub(crate) fn flow(&self) -> &TextFlow {
        &self.flow
    }
}

/// One immutable semantic node in a layout snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotNode {
    identity: SnapshotIdentity,
    parent: Option<SnapshotNodeIndex>,
    children: Arc<[SnapshotNodeIndex]>,
    border_bounds: CellRect,
    content_bounds: CellRect,
    text_origin: CellPoint,
    effective_clip: AxisClip,
    scroll_transform: CellVector,
    text_flow: Option<TextFlowSemanticStamp>,
}

impl SnapshotNode {
    /// Semantic identity.
    pub fn identity(&self) -> &SnapshotIdentity {
        &self.identity
    }
    /// Parent node index, or `None` for the root.
    pub const fn parent(&self) -> Option<SnapshotNodeIndex> {
        self.parent
    }
    /// Child indexes in target order.
    pub fn children(&self) -> &[SnapshotNodeIndex] {
        &self.children
    }
    /// Signed half-open border bounds.
    pub const fn border_bounds(&self) -> CellRect {
        self.border_bounds
    }
    /// Signed half-open content bounds.
    pub const fn content_bounds(&self) -> CellRect {
        self.content_bounds
    }
    /// Signed origin used by this node's TextFlow projection.
    pub const fn text_origin(&self) -> CellPoint {
        self.text_origin
    }
    /// Effective per-axis clip.
    pub const fn effective_clip(&self) -> AxisClip {
        self.effective_clip
    }
    /// Scroll translation applied only to descendants.
    pub const fn scroll_transform(&self) -> CellVector {
        self.scroll_transform
    }
    /// Semantic TextFlow stamp for text nodes.
    pub fn text_flow(&self) -> Option<&TextFlowSemanticStamp> {
        self.text_flow.as_ref()
    }
}

/// Complete immutable terminal-cell geometry for one target and viewport.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSnapshot {
    viewport: CellRect,
    nodes: Arc<[SnapshotNode]>,
    root: SnapshotNodeIndex,
    semantic_index: Arc<HashMap<SnapshotIdentity, SnapshotNodeIndex>>,
}

impl LayoutSnapshot {
    /// Terminal viewport represented by the snapshot.
    pub const fn viewport(&self) -> CellRect {
        self.viewport
    }
    /// Root node.
    pub fn root(&self) -> &SnapshotNode {
        &self.nodes[self.root.0]
    }
    /// Nodes in target preorder.
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &SnapshotNode> {
        self.nodes.iter()
    }
    /// Look up a node by exact semantic identity.
    pub fn get(&self, identity: &SnapshotIdentity) -> Option<&SnapshotNode> {
        self.semantic_index
            .get(identity)
            .map(|index| &self.nodes[index.0])
    }
    pub(crate) fn node(&self, index: SnapshotNodeIndex) -> &SnapshotNode {
        &self.nodes[index.0]
    }
}

/// Opaque revision for a frame-local alias overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameRevision(u64);

impl FrameRevision {
    fn next() -> Self {
        Self(NEXT_FRAME_REVISION.fetch_add(1, Ordering::Relaxed))
    }
    /// Numeric diagnostic revision.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
struct FrameAliasOverlay {
    revision: FrameRevision,
    elements: HashMap<ElementId, SnapshotNodeIndex>,
}

/// A semantic snapshot paired with the current frame's exact element aliases.
#[derive(Debug, Clone)]
pub struct PreparedSnapshotFrame {
    snapshot: Arc<LayoutSnapshot>,
    frame_aliases: FrameAliasOverlay,
}

impl PreparedSnapshotFrame {
    /// Read the immutable semantic snapshot.
    pub fn snapshot(&self) -> &LayoutSnapshot {
        &self.snapshot
    }
    /// Frame-local alias revision.
    pub const fn frame_revision(&self) -> FrameRevision {
        self.frame_aliases.revision
    }
    pub(crate) fn node_for_element(
        &self,
        element_id: ElementId,
    ) -> Result<&SnapshotNode, LayoutAliasError> {
        let index = self
            .frame_aliases
            .elements
            .get(&element_id)
            .copied()
            .ok_or(LayoutAliasError::MissingFrameAlias {
                element_id,
                frame_revision: self.frame_aliases.revision,
            })?;
        Ok(self.snapshot.node(index))
    }

    pub(crate) fn element_nodes(&self) -> impl Iterator<Item = (ElementId, &SnapshotNode)> + '_ {
        self.frame_aliases
            .elements
            .iter()
            .map(|(element_id, index)| (*element_id, self.snapshot.node(*index)))
    }
}

/// Producer strategy that created a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotBuildStrategy {
    /// Fresh initial build.
    InitialFull,
    /// Incremental or unchanged candidate.
    Incremental,
    /// Fresh rebuild after one incremental transaction failure.
    RecoveredFull,
}

/// Deterministic work performed while building one snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotWorkCounters {
    nodes_visited: usize,
    text_flows_bound: usize,
}

impl SnapshotWorkCounters {
    /// Render-required nodes visited.
    pub const fn nodes_visited(self) -> usize {
        self.nodes_visited
    }
    /// TextFlow stamps bound.
    pub const fn text_flows_bound(self) -> usize {
        self.text_flows_bound
    }
}

/// Non-semantic evidence for one snapshot build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotBuildReport {
    strategy: SnapshotBuildStrategy,
    patch_count: usize,
    rebuild_count: usize,
    work: SnapshotWorkCounters,
}

impl SnapshotBuildReport {
    /// Producer strategy.
    pub const fn strategy(&self) -> SnapshotBuildStrategy {
        self.strategy
    }
    /// Planned patch count.
    pub const fn patch_count(&self) -> usize {
        self.patch_count
    }
    /// Fresh recovery rebuild count.
    pub const fn rebuild_count(&self) -> usize {
        self.rebuild_count
    }
    /// Deterministic work counters.
    pub const fn work(&self) -> SnapshotWorkCounters {
        self.work
    }
}

pub(crate) struct LayoutSnapshotBuilder {
    viewport: CellRect,
    nodes: Vec<SnapshotNode>,
    semantic_index: HashMap<SnapshotIdentity, SnapshotNodeIndex>,
    aliases: HashMap<ElementId, SnapshotNodeIndex>,
    text_flows_bound: usize,
}

impl LayoutSnapshotBuilder {
    pub(crate) fn new(width: u16, height: u16) -> Self {
        Self {
            viewport: CellRect::checked(0, 0, i32::from(width), i32::from(height)),
            nodes: Vec::new(),
            semantic_index: HashMap::new(),
            aliases: HashMap::new(),
            text_flows_bound: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push(
        &mut self,
        element_id: ElementId,
        identity: SnapshotIdentity,
        parent: Option<SnapshotNodeIndex>,
        border_bounds: CellRect,
        content_bounds: CellRect,
        text_origin: CellPoint,
        effective_clip: AxisClip,
        scroll_transform: CellVector,
        text_flow: Option<TextFlowSemanticStamp>,
    ) -> Result<SnapshotNodeIndex, LayoutSnapshotError> {
        if self.semantic_index.contains_key(&identity) {
            return Err(LayoutSnapshotError::DuplicateIdentity { identity });
        }
        let index = SnapshotNodeIndex::checked(self.nodes.len());
        if self.aliases.insert(element_id, index).is_some() {
            return Err(LayoutSnapshotError::InvalidTree {
                identity: Some(identity.clone()),
                source: SnapshotInvariantError::SnapshotTargetMismatch {
                    identity,
                    reason: SnapshotTargetMismatchReason::MissingAlias,
                },
            });
        }
        if text_flow.is_some() {
            self.text_flows_bound += 1;
        }
        self.semantic_index.insert(identity.clone(), index);
        self.nodes.push(SnapshotNode {
            identity,
            parent,
            children: Arc::from([]),
            border_bounds,
            content_bounds,
            text_origin,
            effective_clip,
            scroll_transform,
            text_flow,
        });
        Ok(index)
    }

    pub(crate) fn set_children(
        &mut self,
        parent: SnapshotNodeIndex,
        children: Vec<SnapshotNodeIndex>,
    ) {
        self.nodes[parent.0].children = children.into();
    }

    pub(crate) fn finish(
        self,
        root: SnapshotNodeIndex,
        strategy: SnapshotBuildStrategy,
        patch_count: usize,
        rebuild_count: usize,
    ) -> (PreparedSnapshotFrame, SnapshotBuildReport) {
        let work = SnapshotWorkCounters {
            nodes_visited: self.nodes.len(),
            text_flows_bound: self.text_flows_bound,
        };
        let snapshot = Arc::new(LayoutSnapshot {
            viewport: self.viewport,
            nodes: self.nodes.into(),
            root,
            semantic_index: Arc::new(self.semantic_index),
        });
        let prepared = PreparedSnapshotFrame {
            snapshot,
            frame_aliases: FrameAliasOverlay {
                revision: FrameRevision::next(),
                elements: self.aliases,
            },
        };
        let report = SnapshotBuildReport {
            strategy,
            patch_count,
            rebuild_count,
            work,
        };
        (prepared, report)
    }

    pub(crate) const fn viewport(&self) -> CellRect {
        self.viewport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box as RnkBox, Text};
    use crate::core::{Dimension, FlexDirection, Overflow};
    use crate::layout::LayoutEngine;

    fn target() -> crate::core::Element {
        RnkBox::new()
            .width(10)
            .flex_direction(FlexDirection::Column)
            .child(Text::new("first").into_element().with_key("first"))
            .child(Text::new("second").into_element().with_key("second"))
            .into_element()
            .with_key("root")
    }

    #[test]
    fn semantic_identity_and_final_order() {
        let target = target();
        let engine = LayoutEngine::new();
        let frame = engine
            .prepare_element_incremental(&target, None, 10, 4)
            .unwrap();
        let snapshot = frame.snapshot();
        let children = snapshot.root().children();
        assert_eq!(children.len(), 2);
        let first = snapshot.nodes().nth(children[0].as_usize()).unwrap();
        let second = snapshot.nodes().nth(children[1].as_usize()).unwrap();
        assert_ne!(first.identity(), second.identity());
        assert_eq!(first.parent(), Some(SnapshotNodeIndex::checked(0)));
        assert_eq!(second.parent(), Some(SnapshotNodeIndex::checked(0)));
    }

    #[test]
    fn mixed_axis_overflow_clips_only_selected_axis() {
        let mut target = target();
        target.style.width = Dimension::Points(6.0);
        target.style.height = Dimension::Points(2.0);
        target.style.overflow_x = Overflow::Hidden;
        target.style.overflow_y = Overflow::Visible;
        let engine = LayoutEngine::new();
        let frame = engine
            .prepare_element_incremental(&target, None, 20, 8)
            .unwrap();
        assert_eq!(frame.snapshot().root().effective_clip().x().end(), 6);
        assert_eq!(frame.snapshot().root().effective_clip().y().end(), 8);
    }

    #[test]
    fn producer_report_does_not_change_semantic_equality() {
        let first_target = target();
        let mut engine = LayoutEngine::new();
        let first = engine
            .prepare_element_incremental(&first_target, None, 10, 4)
            .unwrap();
        let first_snapshot = first.snapshot().clone();
        let (previous, _) = first.commit(&mut engine);
        let second_target = target();
        let second = engine
            .prepare_element_incremental(&second_target, Some(&previous), 10, 4)
            .unwrap();
        assert_eq!(&first_snapshot, second.snapshot());
        assert_ne!(
            SnapshotBuildStrategy::InitialFull,
            second.snapshot_report().strategy()
        );
    }

    #[test]
    fn cancelled_builder_is_hidden_and_published_snapshot_is_immutable() {
        let target = target();
        let engine = LayoutEngine::new();
        let frame = engine
            .prepare_element_incremental(&target, None, 10, 4)
            .unwrap();
        let published = frame.snapshot().clone();
        drop(frame);
        assert_eq!(published.nodes().len(), 3);
        assert_eq!(published.root().children().len(), 2);
    }
}
