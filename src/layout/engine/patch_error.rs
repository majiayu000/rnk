#![forbid(missing_docs)]

//! Why a batch of incremental patches was rejected.
//!
//! A batch used to be accepted whenever *any* patch in it succeeded, so a patch
//! naming a node that no longer existed was skipped in silence while its
//! siblings applied. The tree then disagreed with the VNode that produced the
//! batch, and nothing said so.
//!
//! Applying is all-or-nothing now, and a rejection names the patch, the node and
//! the reason so the caller can act on it rather than guess.

mod preflight;

pub use preflight::{DirectPatchPreflightCause, DirectPatchPreflightError};

use std::fmt;

use crate::core::NodeKey;
use crate::layout::TextFlowError;
use crate::reconciler::{ReconcilePlanError, SiblingIdentity};

use super::IncrementalInvariantError;

/// Which kind of patch failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchKind {
    /// Create a child node.
    Create,
    /// Update an existing node.
    Update,
    /// Remove an existing node.
    Remove,
    /// Replace an existing node.
    Replace,
    /// Reorder a parent's children.
    Reorder,
}

impl fmt::Display for PatchKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Remove => "remove",
            Self::Replace => "replace",
            Self::Reorder => "reorder",
        };
        f.write_str(name)
    }
}

/// What went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchFailure {
    /// The patch names a node with no entry in the identity map.
    UnknownNode,
    /// The node exists but has no parent to attach to or reorder within.
    MissingParent,
    /// Taffy rejected the structural change.
    TreeRejected,
    /// A subtree could not be built.
    BuildFailed,
    /// Layout did not converge after the batch applied.
    LayoutFailed,
    /// The tree no longer matches the batch after applying it.
    PostconditionViolated,
}

impl fmt::Display for PatchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::UnknownNode => "no node is registered under this key",
            Self::MissingParent => "the node has no parent in the tree",
            Self::TreeRejected => "the layout tree rejected the change",
            Self::BuildFailed => "the replacement subtree could not be built",
            Self::LayoutFailed => "layout failed after the batch applied",
            Self::PostconditionViolated => "the tree does not match the applied batch",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for PatchFailure {}

/// A rejected patch batch, with enough detail to locate the cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchError {
    /// Kind of rejected patch.
    pub kind: PatchKind,
    /// The node the patch names — the target, or the parent for create/reorder.
    pub key: NodeKey,
    /// Concrete legacy rejection category.
    pub failure: PatchFailure,
}

impl PatchError {
    pub(super) fn new(kind: PatchKind, key: NodeKey, failure: PatchFailure) -> Self {
        Self { kind, key, failure }
    }
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} patch for {:?} was rejected: {}",
            self.kind, self.key, self.failure
        )
    }
}

impl std::error::Error for PatchError {}

/// Checked incremental-layout failure.
///
/// Identity planning is intentionally separate from TextFlow so callers can
/// reject invalid trees without treating them as rebuildable patch failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalLayoutError {
    /// Canonical identity planning failed.
    Identity(ReconcilePlanError),
    /// Text measurement or flow failed.
    TextFlow(TextFlowError),
}

impl fmt::Display for IncrementalLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(source) => write!(formatter, "incremental identity failed: {source}"),
            Self::TextFlow(source) => write!(formatter, "incremental text flow failed: {source}"),
        }
    }
}

impl std::error::Error for IncrementalLayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Identity(source) => Some(source),
            Self::TextFlow(source) => Some(source),
        }
    }
}

impl From<ReconcilePlanError> for IncrementalLayoutError {
    fn from(source: ReconcilePlanError) -> Self {
        Self::Identity(source)
    }
}

impl From<TextFlowError> for IncrementalLayoutError {
    fn from(source: TextFlowError) -> Self {
        Self::TextFlow(source)
    }
}

/// Recoverable failure of a compatibility lookup that lacks parent scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutLookupError {
    /// A raw compatibility key matched nodes in multiple scopes.
    AmbiguousLegacyNodeKey {
        /// Ambiguous raw key.
        key: NodeKey,
        /// Number of scoped matches.
        scoped_match_count: usize,
    },
    /// Projecting scoped identities produced a duplicate compatibility identity.
    CompositeIdentityCollision {
        /// Colliding compatibility identity.
        identity: SiblingIdentity,
    },
    /// A measurement key token matched nodes in multiple scopes.
    AmbiguousMeasurementKey {
        /// Ambiguous hashed key token.
        key_token: u64,
        /// Number of scoped matches.
        scoped_match_count: usize,
    },
    /// A measurement compatibility identity matched nodes in multiple scopes.
    AmbiguousMeasurementNodeIdentity {
        /// Ambiguous compatibility identity.
        identity: SiblingIdentity,
        /// Number of scoped matches.
        scoped_match_count: usize,
    },
}

impl fmt::Display for LayoutLookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmbiguousLegacyNodeKey {
                key,
                scoped_match_count,
            } => write!(
                formatter,
                "legacy node key {key:?} matches {scoped_match_count} parent scopes"
            ),
            Self::CompositeIdentityCollision { identity } => write!(
                formatter,
                "scoped layout projection collided at compatibility identity {identity:?}"
            ),
            Self::AmbiguousMeasurementKey {
                key_token,
                scoped_match_count,
            } => write!(
                formatter,
                "measurement key token {key_token:#018x} matches {scoped_match_count} parent scopes"
            ),
            Self::AmbiguousMeasurementNodeIdentity {
                identity,
                scoped_match_count,
            } => write!(
                formatter,
                "measurement node identity {identity:?} matches {scoped_match_count} parent scopes"
            ),
        }
    }
}

impl std::error::Error for LayoutLookupError {}

/// Checked failure from applying a public raw [`crate::reconciler::Patch`] batch.
///
/// This boundary preserves the legacy [`PatchError`] surface while retaining
/// canonical identity and scoped-lookup causes for new callers.
///
/// ```
/// use rnk::layout::DirectPatchError;
/// fn classify(error: &DirectPatchError) -> &'static str {
///     match error { DirectPatchError::Preflight(_) => "preflight", _ => "other" }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectPatchError {
    /// A raw patch batch was invalid before candidate construction.
    Preflight(DirectPatchPreflightError),
    /// Candidate mutation, layout, or verification failed.
    Transaction(PatchTransactionError),
    /// Canonical identity validation failed.
    Identity(ReconcilePlanError),
    /// A raw compatibility lookup was ambiguous.
    Lookup(LayoutLookupError),
    /// A legacy patch application failed.
    Patch(PatchError),
}

impl fmt::Display for DirectPatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight(source) => write!(formatter, "direct patch preflight failed: {source}"),
            Self::Transaction(source) => {
                write!(formatter, "direct patch transaction failed: {source}")
            }
            Self::Identity(source) => write!(formatter, "direct patch identity failed: {source}"),
            Self::Lookup(source) => write!(formatter, "direct patch lookup failed: {source}"),
            Self::Patch(source) => write!(formatter, "direct patch application failed: {source}"),
        }
    }
}

impl std::error::Error for DirectPatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(source) => Some(source),
            Self::Transaction(source) => Some(source),
            Self::Identity(source) => Some(source),
            Self::Lookup(source) => Some(source),
            Self::Patch(source) => Some(source),
        }
    }
}

impl From<ReconcilePlanError> for DirectPatchError {
    fn from(source: ReconcilePlanError) -> Self {
        Self::Identity(source)
    }
}

impl From<LayoutLookupError> for DirectPatchError {
    fn from(source: LayoutLookupError) -> Self {
        Self::Lookup(source)
    }
}

impl From<PatchError> for DirectPatchError {
    fn from(source: PatchError) -> Self {
        Self::Patch(source)
    }
}

impl From<DirectPatchPreflightError> for DirectPatchError {
    fn from(source: DirectPatchPreflightError) -> Self {
        Self::Preflight(source)
    }
}

impl From<PatchTransactionError> for DirectPatchError {
    fn from(source: PatchTransactionError) -> Self {
        Self::Transaction(source)
    }
}

/// Patch kind used by the transactional boundary.
///
/// Unlike the legacy [`PatchKind`], this also represents a viewport-only
/// recomputation that has no public patch ordinal.
///
/// ```
/// use rnk::layout::IncrementalPatchKind;
/// assert_eq!(IncrementalPatchKind::Recompute.to_string(), "recompute");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrementalPatchKind {
    /// Create a node.
    Create,
    /// Update node properties.
    Update,
    /// Remove a node.
    Remove,
    /// Replace a node.
    Replace,
    /// Reorder a parent's children.
    Reorder,
    /// Recompute without a structural patch.
    Recompute,
}

impl IncrementalPatchKind {
    pub(super) fn legacy(self) -> PatchKind {
        match self {
            Self::Create => PatchKind::Create,
            Self::Update | Self::Recompute => PatchKind::Update,
            Self::Remove => PatchKind::Remove,
            Self::Replace => PatchKind::Replace,
            Self::Reorder => PatchKind::Reorder,
        }
    }
}

impl From<PatchKind> for IncrementalPatchKind {
    fn from(kind: PatchKind) -> Self {
        match kind {
            PatchKind::Create => Self::Create,
            PatchKind::Update => Self::Update,
            PatchKind::Remove => Self::Remove,
            PatchKind::Replace => Self::Replace,
            PatchKind::Reorder => Self::Reorder,
        }
    }
}

impl fmt::Display for IncrementalPatchKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create => formatter.write_str("create"),
            Self::Update => formatter.write_str("update"),
            Self::Remove => formatter.write_str("remove"),
            Self::Replace => formatter.write_str("replace"),
            Self::Reorder => formatter.write_str("reorder"),
            Self::Recompute => formatter.write_str("recompute"),
        }
    }
}

/// Stage at which a candidate transaction failed.
///
/// ```
/// use rnk::layout::PatchStage;
/// assert_eq!(PatchStage::SetChildren, PatchStage::SetChildren);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchStage {
    /// Resolve and validate the public target.
    ResolveTarget,
    /// Allocate a new layout node.
    CreateNode,
    /// Set a node context.
    SetContext,
    /// Set a node style.
    SetStyle,
    /// Remove a layout node.
    RemoveNode,
    /// Publish an exact child order.
    SetChildren,
    /// Read candidate state back from the layout backend.
    ReadBack,
    /// Compute candidate layout.
    ComputeLayout,
    /// Verify target-exact postconditions.
    VerifyPostcondition,
}

/// Concrete cause retained by a candidate transaction failure.
///
/// ```
/// use rnk::layout::{PatchFailure, PatchTransactionCause};
/// let cause = PatchTransactionCause::Patch(PatchFailure::UnknownNode);
/// assert!(std::error::Error::source(&cause).is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchTransactionCause {
    /// Legacy patch/backend classification.
    Patch(PatchFailure),
    /// Taffy returned an operation error.
    Taffy(taffy::TaffyError),
    /// TextFlow rejected or interrupted layout.
    TextFlow(TextFlowError),
    /// A target-exact invariant was violated.
    Invariant(IncrementalInvariantError),
}

impl fmt::Display for PatchTransactionCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Patch(source) => source.fmt(formatter),
            Self::Taffy(source) => source.fmt(formatter),
            Self::TextFlow(source) => source.fmt(formatter),
            Self::Invariant(reason) => reason.fmt(formatter),
        }
    }
}

impl std::error::Error for PatchTransactionCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Patch(source) => Some(source),
            Self::Taffy(source) => Some(source),
            Self::TextFlow(source) => Some(source),
            Self::Invariant(source) => Some(source),
        }
    }
}

/// Typed failure from candidate apply, layout, or verification.
///
/// ```
/// use rnk::layout::{IncrementalPatchKind, PatchFailure, PatchStage, PatchTransactionCause, PatchTransactionError};
/// let error = PatchTransactionError { patch_index: None, kind: IncrementalPatchKind::Recompute, key: None, parent: None, stage: PatchStage::ComputeLayout, source: Box::new(PatchTransactionCause::Patch(PatchFailure::LayoutFailed)) };
/// assert!(error.patch_index.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchTransactionError {
    /// Original public patch ordinal, or `None` for viewport-only work.
    pub patch_index: Option<usize>,
    /// Operation associated with the failure.
    pub kind: IncrementalPatchKind,
    /// Target key, when one exists.
    pub key: Option<NodeKey>,
    /// Parent key, when one exists.
    pub parent: Option<NodeKey>,
    /// Failing transaction stage.
    pub stage: PatchStage,
    /// Concrete backend or invariant cause.
    pub source: Box<PatchTransactionCause>,
}

impl PatchTransactionError {
    pub(super) fn legacy(&self) -> PatchError {
        let failure = match self.source.as_ref() {
            PatchTransactionCause::Patch(failure) => *failure,
            PatchTransactionCause::Taffy(_) => match self.stage {
                PatchStage::ComputeLayout | PatchStage::ReadBack => PatchFailure::LayoutFailed,
                PatchStage::CreateNode => PatchFailure::BuildFailed,
                PatchStage::SetContext
                    if matches!(
                        self.kind,
                        IncrementalPatchKind::Create | IncrementalPatchKind::Replace
                    ) =>
                {
                    PatchFailure::BuildFailed
                }
                PatchStage::VerifyPostcondition => PatchFailure::PostconditionViolated,
                _ => PatchFailure::TreeRejected,
            },
            PatchTransactionCause::TextFlow(_) => PatchFailure::LayoutFailed,
            PatchTransactionCause::Invariant(_) => PatchFailure::PostconditionViolated,
        };
        let key = if self.kind == IncrementalPatchKind::Create {
            self.parent.or(self.key)
        } else {
            self.key.or(self.parent)
        }
        .unwrap_or_else(NodeKey::root);
        PatchError::new(self.kind.legacy(), key, failure)
    }
}

impl fmt::Display for PatchTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} transaction at {:?} for key {:?}, parent {:?}: {}",
            self.kind, self.stage, self.key, self.parent, self.source
        )
    }
}

impl std::error::Error for PatchTransactionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Result of an atomic targetless raw-patch batch.
///
/// ```
/// use rnk::layout::DirectPatchApplyReport;
/// let name = match DirectPatchApplyReport::NoChange { DirectPatchApplyReport::NoChange => "none", _ => "applied" };
/// assert_eq!(name, "none");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectPatchApplyReport {
    /// The batch was empty.
    NoChange,
    /// Every patch and postcondition committed.
    Applied {
        /// Number of committed public patches.
        patch_count: usize,
    },
}

/// Stage of a fresh target rebuild.
///
/// ```
/// use rnk::layout::RebuildStage;
/// assert_eq!(RebuildStage::BuildTarget, RebuildStage::BuildTarget);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildStage {
    /// Materialize the target tree.
    BuildTarget,
    /// Synchronize target contexts and element aliases.
    SetContext,
    /// Compute target layout.
    ComputeLayout,
    /// Verify the rebuilt target.
    VerifyPostcondition,
}

/// Concrete cause from a fresh target rebuild.
///
/// ```
/// use rnk::layout::RebuildFailure;
/// assert_eq!(RebuildFailure::InvalidTargetRoot.to_string(), "target has no valid root");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebuildFailure {
    /// The target did not produce a valid root.
    InvalidTargetRoot,
    /// Taffy returned an operation error.
    Taffy(taffy::TaffyError),
    /// TextFlow rejected or interrupted layout.
    TextFlow(TextFlowError),
    /// A target-exact invariant was violated.
    Invariant(IncrementalInvariantError),
}

impl fmt::Display for RebuildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargetRoot => formatter.write_str("target has no valid root"),
            Self::Taffy(source) => source.fmt(formatter),
            Self::TextFlow(source) => source.fmt(formatter),
            Self::Invariant(reason) => reason.fmt(formatter),
        }
    }
}

impl std::error::Error for RebuildFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Taffy(source) => Some(source),
            Self::TextFlow(source) => Some(source),
            Self::Invariant(source) => Some(source),
            Self::InvalidTargetRoot => None,
        }
    }
}

/// Failure from one fresh target rebuild attempt.
///
/// ```
/// use rnk::layout::{FullRebuildError, RebuildFailure, RebuildStage};
/// let error = FullRebuildError { stage: RebuildStage::BuildTarget, key: None, source: RebuildFailure::InvalidTargetRoot };
/// assert_eq!(error.stage, RebuildStage::BuildTarget);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullRebuildError {
    /// Failing rebuild stage.
    pub stage: RebuildStage,
    /// Target key associated with the failure, when available.
    pub key: Option<NodeKey>,
    /// Concrete rebuild cause.
    pub source: RebuildFailure,
}

impl fmt::Display for FullRebuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "fresh rebuild failed at {:?} for {:?}: {}",
            self.stage, self.key, self.source
        )
    }
}

impl std::error::Error for FullRebuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// A target rejected before any incremental candidate or recovery build ran.
///
/// This is distinct from [`FullRebuildError`]: it records target validation,
/// not an attempted rebuild.
///
/// ```
/// use rnk::layout::{InvalidLayoutTargetError, RebuildFailure};
///
/// let error = InvalidLayoutTargetError {
///     key: None,
///     source: RebuildFailure::InvalidTargetRoot,
/// };
/// assert!(matches!(error.source, RebuildFailure::InvalidTargetRoot));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidLayoutTargetError {
    /// Target key associated with the rejection, when available.
    pub key: Option<NodeKey>,
    /// Concrete reason the target cannot form a layout tree.
    pub source: RebuildFailure,
}

impl fmt::Display for InvalidLayoutTargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "layout target validation failed for {:?}: {}",
            self.key, self.source
        )
    }
}

impl std::error::Error for InvalidLayoutTargetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
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
            Self::RecoveryFailed { rebuild, .. } => Some(rebuild.as_ref()),
        }
    }
}

impl From<IncrementalLayoutError> for TransactionalLayoutError {
    fn from(source: IncrementalLayoutError) -> Self {
        Self::Upstream(source)
    }
}

#[cfg(test)]
mod tests;

/// Successful checked incremental-layout classification.
///
/// ```
/// use rnk::{core::Element, layout::{CheckedIncrementalLayoutReport, LayoutEngine}};
/// let mut engine = LayoutEngine::new();
/// let (_, report) = engine.try_compute_element_incremental_transactional(&Element::root(), None, 20, 4).expect("initial layout");
/// let category = match report { CheckedIncrementalLayoutReport::InitialFullBuild => "initial", _ => "other" };
/// assert_eq!(category, "initial");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedIncrementalLayoutReport {
    /// First committed frame was built from scratch.
    InitialFullBuild,
    /// Tree and viewport were unchanged.
    NoChange,
    /// A patch plan committed normally.
    Incremental {
        /// Number of planned public patches.
        patch_count: usize,
    },
    /// Only the viewport required recomputation.
    RecomputedViewport,
    /// One incremental failure recovered through one fresh rebuild.
    RecoveredFullRebuild {
        /// Number of planned public patches.
        patch_count: usize,
        /// Original candidate failure.
        incremental_failure: PatchTransactionError,
    },
}
