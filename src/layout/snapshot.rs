#![forbid(missing_docs)]

//! Immutable, producer-independent terminal-cell layout snapshots.

use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{core::ElementId, layout::TextFlow, reconciler::ScopedNodeIdentity};

mod builder;
mod error;
mod quantize;

pub(crate) use builder::{CheckedSnapshotNodeInput, LayoutSnapshotBuilder};
pub use error::{
    ArithmeticOperation, AttemptedContentBounds, Axis, CellOutputError, Edge, GeometryField,
    LayoutAliasError, LayoutSnapshotError, SnapshotAttemptReport, SnapshotBuildFailure,
    SnapshotCounterError, SnapshotInvariantError, SnapshotTargetMismatchReason,
    SnapshotWorkCounterField,
};
pub(crate) use quantize::{
    add as checked_add, edge as quantize_edge, extent as checked_extent, finite as checked_finite,
    rect as quantize_rect, subtract as checked_subtract,
};

static NEXT_FRAME_REVISION: AtomicU64 = AtomicU64::new(1);

/// Opaque, exact semantic identity of one node in a layout snapshot.
///
/// Identities can be compared and used as map keys, but callers cannot forge
/// one or couple application logic to reconciler storage details.
#[derive(Clone, PartialEq, Eq)]
pub struct SnapshotIdentity(ScopedNodeIdentity);

impl SnapshotIdentity {
    pub(crate) fn from_scoped(identity: ScopedNodeIdentity) -> Self {
        Self(identity)
    }

    /// Stable diagnostic label suitable for error messages and test output.
    pub fn diagnostic(&self) -> String {
        format!("snapshot:{:016x}", self.opaque_digest())
    }

    pub(crate) fn scoped(&self) -> &ScopedNodeIdentity {
        &self.0
    }

    fn opaque_digest(&self) -> u64 {
        let mut private = std::collections::hash_map::DefaultHasher::new();
        private.write(b"rnk.snapshot.identity.v1");
        self.0.hash(&mut private);
        private.finish() ^ 0x5a17_4f9c_d821_b603
    }
}

impl fmt::Debug for SnapshotIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SnapshotIdentity(<opaque>)")
    }
}

impl fmt::Display for SnapshotIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("snapshot:<opaque>")
    }
}

impl Hash for SnapshotIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(b"rnk.snapshot.identity.digest.v1");
        state.write_u64(self.opaque_digest());
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
    pub(crate) const fn checked(left: i32, top: i32, right: i32, bottom: i32) -> Option<Self> {
        let width = right as i64 - left as i64;
        let height = bottom as i64 - top as i64;
        if left <= right && top <= bottom && width <= i32::MAX as i64 && height <= i32::MAX as i64 {
            Some(Self {
                left,
                top,
                right,
                bottom,
            })
        } else {
            None
        }
    }
    pub(crate) const fn viewport(width: u16, height: u16) -> Self {
        Self {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
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
        let x = self.x_span().intersect(other.x_span());
        let y = self.y_span().intersect(other.y_span());
        Self {
            left: x.start,
            top: y.start,
            right: x.end,
            bottom: y.end,
        }
    }

    pub(crate) const fn contains(self, other: Self) -> bool {
        other.is_empty()
            || (self.left <= other.left
                && self.top <= other.top
                && other.right <= self.right
                && other.bottom <= self.bottom)
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.left == self.right || self.top == self.bottom
    }

    pub(crate) const fn x_span(self) -> CellSpan {
        CellSpan {
            start: self.left,
            end: self.right,
        }
    }

    pub(crate) const fn y_span(self) -> CellSpan {
        CellSpan {
            start: self.top,
            end: self.bottom,
        }
    }
}

/// Signed half-open span on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpan {
    start: i32,
    end: i32,
}

impl CellSpan {
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
        self.start == self.end
    }
    pub(crate) fn intersect(self, other: Self) -> Self {
        let start = self.start.max(other.start);
        let min_end = self.end.min(other.end);
        if start <= min_end {
            Self {
                start,
                end: min_end,
            }
        } else {
            Self {
                start: min_end,
                end: min_end,
            }
        }
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
            CellSpan {
                start: rect.left,
                end: rect.right,
            },
            CellSpan {
                start: rect.top,
                end: rect.bottom,
            },
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
#[derive(Clone, PartialEq)]
pub struct TextFlowSemanticStamp {
    flow: Arc<TextFlow>,
}

impl fmt::Debug for TextFlowSemanticStamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TextFlowSemanticStamp(<semantic>)")
    }
}

impl TextFlowSemanticStamp {
    pub(crate) fn checked(flow: Arc<TextFlow>) -> Self {
        Self { flow }
    }
    /// Return the first exact semantic difference without exposing raw text.
    ///
    /// String differences are represented by total byte lengths, the first
    /// differing byte index, and exact hexadecimal byte values (or `missing`
    /// for a prefix). Scalar and enum differences name their fixed structural
    /// path and exact values. The diagnostic never embeds source strings, and
    /// its size is independent of their length. Equal stamps return `None`.
    pub fn first_difference_diagnostic(&self, other: &Self) -> Option<String> {
        self.flow.first_semantic_difference(&other.flow)
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
#[derive(Clone, PartialEq)]
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

impl fmt::Debug for SnapshotNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnapshotNode")
            .field("identity", &self.identity)
            .field("child_count", &self.children.len())
            .field("has_text_flow", &self.text_flow.is_some())
            .finish()
    }
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
#[derive(Clone, PartialEq)]
pub struct LayoutSnapshot {
    viewport: CellRect,
    nodes: Arc<[SnapshotNode]>,
    root: SnapshotNodeIndex,
    semantic_index: Arc<HashMap<SnapshotIdentity, SnapshotNodeIndex>>,
}

impl fmt::Debug for LayoutSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LayoutSnapshot")
            .field("viewport", &self.viewport)
            .field("node_count", &self.nodes.len())
            .field("root", &"<opaque>")
            .finish()
    }
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
#[derive(Clone)]
pub struct PreparedSnapshotFrame {
    snapshot: Arc<LayoutSnapshot>,
    frame_aliases: FrameAliasOverlay,
}

impl fmt::Debug for PreparedSnapshotFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSnapshotFrame")
            .field("snapshot", &self.snapshot)
            .field("alias_count", &self.frame_aliases.elements.len())
            .finish()
    }
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

    pub(crate) fn resolve_exact_alias(
        &self,
        element_id: ElementId,
        expected_identity: &SnapshotIdentity,
    ) -> Result<&SnapshotNode, LayoutAliasError> {
        if self.snapshot.get(expected_identity).is_none() {
            return Err(LayoutAliasError::AliasTargetMissing {
                element_id,
                identity: expected_identity.clone(),
            });
        }
        let node = self.node_for_element(element_id)?;
        if node.identity() != expected_identity {
            return Err(LayoutAliasError::AliasIdentityMismatch {
                element_id,
                expected_identity: expected_identity.clone(),
                actual_identity: node.identity().clone(),
            });
        }
        Ok(node)
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
    visited_nodes: u64,
    mutated_nodes: u64,
    text_flow_recomputes: u64,
    snapshot_nodes: u64,
    rebuild_count: u64,
}

impl SnapshotWorkCounters {
    /// Render-required nodes visited.
    pub const fn nodes_visited(self) -> usize {
        self.visited_nodes as usize
    }
    /// TextFlow stamps bound.
    pub const fn text_flows_bound(self) -> usize {
        self.text_flow_recomputes as usize
    }
    /// Accepted target identities that began checked lookup.
    pub const fn visited_nodes(self) -> u64 {
        self.visited_nodes
    }
    /// Distinct validated planned mutations and removals.
    pub const fn mutated_nodes(self) -> u64 {
        self.mutated_nodes
    }
    /// Successful TextFlow cache misses.
    pub const fn text_flow_recomputes(self) -> u64 {
        self.text_flow_recomputes
    }
    /// Nodes in the finalized snapshot.
    pub const fn snapshot_nodes(self) -> u64 {
        self.snapshot_nodes
    }
    /// GH60 recovery transitions.
    pub const fn rebuild_count(self) -> u64 {
        self.rebuild_count
    }
    pub(crate) const fn zero() -> Self {
        Self::from_fields(0, 0, 0, 0, 0)
    }
    pub(crate) const fn from_fields(
        visited_nodes: u64,
        mutated_nodes: u64,
        text_flow_recomputes: u64,
        snapshot_nodes: u64,
        rebuild_count: u64,
    ) -> Self {
        Self {
            visited_nodes,
            mutated_nodes,
            text_flow_recomputes,
            snapshot_nodes,
            rebuild_count,
        }
    }
    pub(crate) fn checked_add(self, rhs: Self) -> Result<Self, SnapshotCounterError> {
        fn add(
            field: SnapshotWorkCounterField,
            lhs: u64,
            rhs: u64,
        ) -> Result<u64, SnapshotCounterError> {
            lhs.checked_add(rhs)
                .ok_or(SnapshotCounterError::Overflow { field, lhs, rhs })
        }
        Ok(Self {
            visited_nodes: add(
                SnapshotWorkCounterField::VisitedNodes,
                self.visited_nodes,
                rhs.visited_nodes,
            )?,
            mutated_nodes: add(
                SnapshotWorkCounterField::MutatedNodes,
                self.mutated_nodes,
                rhs.mutated_nodes,
            )?,
            text_flow_recomputes: add(
                SnapshotWorkCounterField::TextFlowRecomputes,
                self.text_flow_recomputes,
                rhs.text_flow_recomputes,
            )?,
            snapshot_nodes: add(
                SnapshotWorkCounterField::SnapshotNodes,
                self.snapshot_nodes,
                rhs.snapshot_nodes,
            )?,
            rebuild_count: add(
                SnapshotWorkCounterField::RebuildCount,
                self.rebuild_count,
                rhs.rebuild_count,
            )?,
        })
    }
}

/// Non-semantic evidence for one snapshot build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotBuildReport {
    strategy: SnapshotBuildStrategy,
    patch_count: usize,
    recovery_cause: Option<crate::layout::PatchTransactionError>,
    cache_hits: u64,
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
        self.work.rebuild_count as usize
    }
    /// Deterministic work counters.
    pub const fn work(&self) -> SnapshotWorkCounters {
        self.work
    }
    /// Complete five-field work counters.
    pub const fn work_counters(&self) -> SnapshotWorkCounters {
        self.work
    }
    /// Original incremental cause for a recovered producer.
    pub fn recovery_cause(&self) -> Option<&crate::layout::PatchTransactionError> {
        self.recovery_cause.as_ref()
    }
    /// Exact cache hits recorded outside semantic equality.
    pub const fn cache_hits(&self) -> u64 {
        self.cache_hits
    }
}

#[cfg(test)]
mod tests;
