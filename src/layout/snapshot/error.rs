#![forbid(missing_docs)]

//! Closed errors for immutable terminal-cell layout snapshots.

use std::{error::Error, fmt};

use crate::core::ElementId;
use crate::layout::TextFlowError;

use super::{CellRect, FrameRevision, SnapshotIdentity};

/// Raw rejected content edges retained only for diagnostics.
///
/// This type cannot be converted into checked [`CellRect`] geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptedContentBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl AttemptedContentBounds {
    pub(crate) const fn from_raw(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    /// Attempted left edge.
    pub const fn left(self) -> i32 {
        self.left
    }
    /// Attempted top edge.
    pub const fn top(self) -> i32 {
        self.top
    }
    /// Attempted right edge.
    pub const fn right(self) -> i32 {
        self.right
    }
    /// Attempted bottom edge.
    pub const fn bottom(self) -> i32 {
        self.bottom
    }
}

/// Coordinate axis used by snapshot validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Horizontal axis.
    X,
    /// Vertical axis.
    Y,
}

/// Raw geometry field that failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryField {
    /// Relative x coordinate.
    X,
    /// Relative y coordinate.
    Y,
    /// Width.
    Width,
    /// Height.
    Height,
    /// Left border or padding inset.
    LeftInset,
    /// Top border or padding inset.
    TopInset,
    /// Right border or padding inset.
    RightInset,
    /// Bottom border or padding inset.
    BottomInset,
}

/// A half-open rectangle edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Left edge.
    Left,
    /// Top edge.
    Top,
    /// Right edge.
    Right,
    /// Bottom edge.
    Bottom,
}

/// One complete snapshot work-counter field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotWorkCounterField {
    /// Target nodes accepted for checked lookup.
    VisitedNodes,
    /// Distinct planned mutations and removals.
    MutatedNodes,
    /// Successful TextFlow cache misses.
    TextFlowRecomputes,
    /// Nodes in a successfully finalized snapshot.
    SnapshotNodes,
    /// GH60 recovery transitions.
    RebuildCount,
}

/// Checked aggregation failure for snapshot work evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotCounterError {
    /// One field overflowed during checked addition.
    Overflow {
        /// Counter field.
        field: SnapshotWorkCounterField,
        /// Existing value.
        lhs: u64,
        /// Added value.
        rhs: u64,
    },
}

impl fmt::Display for SnapshotCounterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("snapshot work counter overflowed")
    }
}

impl Error for SnapshotCounterError {}

/// Arithmetic operation that overflowed while accumulating absolute geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOperation {
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
}

/// Why a render-required target did not match the prepared layout candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotTargetMismatchReason {
    /// The target had no renderable root.
    MissingRoot,
    /// A target node had no prepared element alias.
    MissingAlias,
    /// The prepared child order differed from the target traversal.
    ChildOrder,
}

/// Structural failure in an otherwise checked snapshot candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotInvariantError {
    /// A child did not reference the expected parent.
    MissingParent {
        /// Child identity.
        child: SnapshotIdentity,
        /// Expected parent identity.
        expected_parent: SnapshotIdentity,
    },
    /// Child order differed from the target order.
    ChildOrderMismatch {
        /// Parent identity.
        parent: SnapshotIdentity,
        /// Child identity.
        child: SnapshotIdentity,
        /// Expected child position.
        expected_index: usize,
        /// Actual child position.
        actual_index: usize,
    },
    /// A node was not reachable from the snapshot root.
    OrphanNode {
        /// Orphan identity.
        identity: SnapshotIdentity,
    },
    /// The render-required target and prepared candidate differed.
    SnapshotTargetMismatch {
        /// Identity associated with the mismatch.
        identity: SnapshotIdentity,
        /// Closed mismatch reason.
        reason: SnapshotTargetMismatchReason,
    },
}

impl fmt::Display for SnapshotInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid layout snapshot tree: {self:?}")
    }
}

impl Error for SnapshotInvariantError {}

/// Failure while constructing immutable terminal-cell geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutSnapshotError {
    /// Raw geometry was NaN or infinite.
    NonFiniteGeometry {
        /// Node identity.
        identity: SnapshotIdentity,
        /// Invalid field.
        field: GeometryField,
        /// Exact `f32` payload bits.
        value_bits: u32,
    },
    /// Taffy produced a negative extent.
    NegativeExtent {
        /// Node identity.
        identity: SnapshotIdentity,
        /// Invalid axis.
        axis: Axis,
        /// Exact `f32` payload bits.
        value_bits: u32,
    },
    /// Absolute edge accumulation overflowed.
    EdgeArithmeticOverflow {
        /// Node identity.
        identity: SnapshotIdentity,
        /// Failed operation.
        operation: ArithmeticOperation,
        /// Left operand bits as `f64`.
        lhs_bits: u64,
        /// Right operand bits as `f64`.
        rhs_bits: u64,
    },
    /// A floored edge cannot be represented by the signed cell model.
    CellCoordinateOverflow {
        /// Node identity.
        identity: SnapshotIdentity,
        /// Failed edge.
        edge: Edge,
        /// Floored `f64` payload bits.
        rounded_bits: u64,
    },
    /// An ordered cell span exceeded the representable public width.
    CellSpanOverflow {
        /// Node identity.
        identity: SnapshotIdentity,
        /// Failed axis.
        axis: Axis,
        /// Inclusive start edge.
        start: i32,
        /// Exclusive end edge.
        end: i32,
    },
    /// Insets produced reversed content geometry.
    ReversedContentBounds {
        /// Node identity.
        identity: SnapshotIdentity,
        /// Quantized border bounds.
        border_bounds: CellRect,
        /// Attempted quantized content bounds.
        attempted_content_bounds: AttemptedContentBounds,
    },
    /// A render-required element had no semantic identity.
    MissingIdentity {
        /// Missing frame-local element alias.
        element_id: ElementId,
    },
    /// Two target nodes resolved to the same semantic identity.
    DuplicateIdentity {
        /// Duplicated identity.
        identity: SnapshotIdentity,
    },
    /// A render-required identity had no computed layout.
    MissingLayout {
        /// Missing identity.
        identity: SnapshotIdentity,
    },
    /// A text node had no current semantic TextFlow.
    MissingTextFlowRevision {
        /// Text node identity.
        identity: SnapshotIdentity,
    },
    /// The current TextFlow could not be rebound to the quantized content width.
    TextFlowRevision {
        /// Text node identity.
        identity: SnapshotIdentity,
        /// Concrete TextFlow construction failure.
        source: TextFlowError,
    },
    /// Work evidence could not be aggregated exactly.
    WorkCounters {
        /// Concrete checked-add failure.
        source: SnapshotCounterError,
    },
    /// The target traversal was structurally invalid.
    InvalidTree {
        /// Identity nearest to the failure.
        identity: Option<SnapshotIdentity>,
        /// Concrete invariant failure.
        source: SnapshotInvariantError,
    },
}

impl fmt::Display for LayoutSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteGeometry { .. } => "snapshot geometry is non-finite",
            Self::NegativeExtent { .. } => "snapshot extent is negative",
            Self::EdgeArithmeticOverflow { .. } => "snapshot edge arithmetic overflowed",
            Self::CellCoordinateOverflow { .. } => "snapshot cell coordinate overflowed",
            Self::CellSpanOverflow { .. } => "snapshot cell span overflowed",
            Self::ReversedContentBounds { .. } => "snapshot content bounds are reversed",
            Self::MissingIdentity { .. } => "snapshot identity is missing",
            Self::DuplicateIdentity { .. } => "snapshot identity is duplicated",
            Self::MissingLayout { .. } => "snapshot layout is missing",
            Self::MissingTextFlowRevision { .. } => "snapshot TextFlow revision is missing",
            Self::TextFlowRevision { .. } => "snapshot TextFlow revision failed",
            Self::WorkCounters { .. } => "snapshot work counters failed",
            Self::InvalidTree { .. } => "snapshot tree is invalid",
        })
    }
}

impl Error for LayoutSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTree { source, .. } => Some(source),
            Self::TextFlowRevision { source, .. } => Some(source),
            Self::WorkCounters { source } => Some(source),
            _ => None,
        }
    }
}

/// Read-only work captured at the end of or during one snapshot attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotAttemptReport {
    operation_count: u64,
    work_counters: super::SnapshotWorkCounters,
}

impl SnapshotAttemptReport {
    pub(crate) const fn new(
        operation_count: u64,
        work_counters: super::SnapshotWorkCounters,
    ) -> Self {
        Self {
            operation_count,
            work_counters,
        }
    }
    /// Operations represented by this report.
    pub const fn operation_count(&self) -> u64 {
        self.operation_count
    }
    /// Complete five-field work evidence.
    pub const fn work_counters(&self) -> super::SnapshotWorkCounters {
        self.work_counters
    }
    pub(crate) fn set_work_counters(&mut self, work_counters: super::SnapshotWorkCounters) {
        self.work_counters = work_counters;
    }
}

/// Authoritative crate-private builder failure preserving partial work.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotBuildFailure {
    source: LayoutSnapshotError,
    attempt_report: SnapshotAttemptReport,
}

impl SnapshotBuildFailure {
    pub(crate) fn new(source: LayoutSnapshotError, attempt_report: SnapshotAttemptReport) -> Self {
        Self {
            source,
            attempt_report,
        }
    }
    pub(crate) fn into_parts(self) -> (LayoutSnapshotError, SnapshotAttemptReport) {
        (self.source, self.attempt_report)
    }
}

impl fmt::Display for SnapshotBuildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl Error for SnapshotBuildFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Checked snapshot-construction failure with lossless attempt evidence.
///
/// Partial builder state remains private. Callers can inspect only the closed
/// source algebra and the immutable work report captured at first failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotBuildError {
    source: LayoutSnapshotError,
    attempt_report: SnapshotAttemptReport,
}

impl SnapshotBuildError {
    pub(crate) fn from_failure(failure: SnapshotBuildFailure) -> Self {
        let (source, attempt_report) = failure.into_parts();
        Self {
            source,
            attempt_report,
        }
    }

    pub(crate) fn from_source(source: LayoutSnapshotError) -> Self {
        Self {
            source,
            attempt_report: SnapshotAttemptReport::new(1, super::SnapshotWorkCounters::zero()),
        }
    }

    /// Closed snapshot failure that poisoned the attempt.
    pub const fn source_error(&self) -> &LayoutSnapshotError {
        &self.source
    }

    /// Complete work captured before the first failure.
    pub const fn attempt_report(&self) -> &SnapshotAttemptReport {
        &self.attempt_report
    }
}

impl fmt::Display for SnapshotBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl Error for SnapshotBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Failure while resolving frame-local aliases against a semantic snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutAliasError {
    /// A frame-local element had no alias.
    MissingFrameAlias {
        /// Missing element.
        element_id: ElementId,
        /// Frame revision.
        frame_revision: FrameRevision,
    },
    /// One frame-local element was assigned two identities.
    DuplicateFrameAlias {
        /// Duplicated element.
        element_id: ElementId,
        /// First identity.
        first_identity: SnapshotIdentity,
        /// Second identity.
        second_identity: SnapshotIdentity,
    },
    /// An alias referred to an identity absent from the snapshot.
    AliasTargetMissing {
        /// Frame-local element.
        element_id: ElementId,
        /// Missing semantic identity.
        identity: SnapshotIdentity,
    },
    /// An alias overlay came from another frame.
    StaleFrameAlias {
        /// Frame-local element.
        element_id: ElementId,
        /// Expected revision.
        expected_frame_revision: FrameRevision,
        /// Actual revision.
        actual_frame_revision: FrameRevision,
    },
    /// An alias resolved to a different semantic node.
    AliasIdentityMismatch {
        /// Frame-local element.
        element_id: ElementId,
        /// Expected identity.
        expected_identity: SnapshotIdentity,
        /// Actual identity.
        actual_identity: SnapshotIdentity,
    },
}

impl fmt::Display for LayoutAliasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "layout snapshot alias failed: {self:?}")
    }
}

impl Error for LayoutAliasError {}

/// Terminal conversion failure after snapshot clipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellOutputError {
    /// A negative cell survived clipping.
    NegativeAfterClip {
        /// Failed axis.
        axis: Axis,
        /// Negative value.
        value: i32,
    },
    /// A coordinate exceeded the terminal cell type.
    CoordinateOutOfRange {
        /// Failed axis.
        axis: Axis,
        /// Invalid value.
        value: i32,
    },
    /// A half-open extent exceeded the terminal cell type.
    ExtentOutOfRange {
        /// Failed axis.
        axis: Axis,
        /// Start edge.
        start: i32,
        /// End edge.
        end: i32,
    },
}

impl fmt::Display for CellOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "snapshot cell output failed: {self:?}")
    }
}

impl Error for CellOutputError {}
