//! Snapshot-aware transaction failures.

use std::fmt;

use crate::layout::snapshot::SnapshotBuildFailure;
use crate::layout::{LayoutSnapshotError, SnapshotAttemptReport};

use super::{
    DirectPatchError, FullRebuildError, IncrementalLayoutError, InvalidLayoutTargetError,
    PatchTransactionError,
};

/// A successful recovery layout whose immutable snapshot could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredSnapshotError {
    incremental: Box<PatchTransactionError>,
    snapshot: Box<SnapshotBuildFailure>,
}

impl RecoveredSnapshotError {
    pub(crate) fn new(incremental: PatchTransactionError, snapshot: SnapshotBuildFailure) -> Self {
        Self {
            incremental: Box::new(incremental),
            snapshot: Box::new(snapshot),
        }
    }

    /// Original incremental transaction failure.
    pub fn incremental_failure(&self) -> &PatchTransactionError {
        &self.incremental
    }

    /// Final snapshot construction failure.
    pub fn snapshot_failure(&self) -> &LayoutSnapshotError {
        self.snapshot.source_error()
    }

    /// Complete work captured by the failed recovery snapshot attempt.
    pub fn snapshot_attempt_report(&self) -> &SnapshotAttemptReport {
        self.snapshot.attempt_report()
    }
}

impl fmt::Display for RecoveredSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "incremental layout failed ({}); recovered snapshot failed ({})",
            self.incremental, self.snapshot
        )
    }
}

impl std::error::Error for RecoveredSnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.snapshot.as_ref())
    }
}

/// Recoverable checked transaction boundary for incremental layout.
///
/// ```
/// use rnk::{core::{Element, ElementType}, layout::{LayoutEngine, TransactionalLayoutError}};
/// let mut engine = LayoutEngine::new();
/// let error = engine.try_compute_element_incremental_transactional(&Element::new(ElementType::VirtualText), None, 20, 4).expect_err("invalid root");
/// let category = match &error { TransactionalLayoutError::InitialBuild(_) => "initial", _ => "other" };
/// assert_eq!(category, "initial");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionalLayoutError {
    /// GH-59 planning or TextFlow validation failed before transaction work.
    Upstream(IncrementalLayoutError),
    /// A targetless raw patch batch failed.
    DirectPatch(DirectPatchError),
    /// The initial frame could not be built.
    InitialBuild(FullRebuildError),
    /// The target was invalid before transaction or recovery work began.
    InvalidTarget(InvalidLayoutTargetError),
    /// Layout succeeded but immutable snapshot construction failed.
    Snapshot(LayoutSnapshotError),
    /// Recovery layout succeeded but its immutable snapshot failed.
    RecoveredSnapshot(RecoveredSnapshotError),
    /// Incremental commit and its single fresh recovery attempt both failed.
    RecoveryFailed {
        /// Primary incremental failure.
        incremental: Box<PatchTransactionError>,
        /// Failure from the one fresh rebuild.
        rebuild: Box<FullRebuildError>,
    },
}

impl TransactionalLayoutError {
    /// Returns the primary candidate failure when incremental recovery failed.
    pub fn incremental_failure(&self) -> Option<&PatchTransactionError> {
        match self {
            Self::RecoveryFailed { incremental, .. } => Some(incremental),
            Self::RecoveredSnapshot(source) => Some(source.incremental_failure()),
            _ => None,
        }
    }

    /// Returns the fresh-build failure from an initial or recovery attempt.
    pub fn rebuild_failure(&self) -> Option<&FullRebuildError> {
        match self {
            Self::InitialBuild(rebuild) => Some(rebuild),
            Self::RecoveryFailed { rebuild, .. } => Some(rebuild),
            _ => None,
        }
    }
}

impl fmt::Display for TransactionalLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upstream(source) => write!(formatter, "upstream layout failed: {source}"),
            Self::DirectPatch(source) => write!(formatter, "direct patch failed: {source}"),
            Self::InitialBuild(source) => write!(formatter, "initial build failed: {source}"),
            Self::InvalidTarget(source) => source.fmt(formatter),
            Self::Snapshot(source) => write!(formatter, "snapshot failed: {source}"),
            Self::RecoveredSnapshot(source) => source.fmt(formatter),
            Self::RecoveryFailed {
                incremental,
                rebuild,
            } => write!(
                formatter,
                "incremental commit failed ({incremental}); fresh rebuild failed ({rebuild})"
            ),
        }
    }
}

impl std::error::Error for TransactionalLayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Upstream(source) => Some(source),
            Self::DirectPatch(source) => Some(source),
            Self::InitialBuild(source) => Some(source),
            Self::InvalidTarget(source) => Some(source),
            Self::Snapshot(source) => Some(source),
            Self::RecoveredSnapshot(source) => Some(source),
            Self::RecoveryFailed { rebuild, .. } => Some(rebuild.as_ref()),
        }
    }
}

impl From<IncrementalLayoutError> for TransactionalLayoutError {
    fn from(source: IncrementalLayoutError) -> Self {
        Self::Upstream(source)
    }
}
