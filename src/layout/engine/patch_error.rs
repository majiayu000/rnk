//! Why a batch of incremental patches was rejected.
//!
//! A batch used to be accepted whenever *any* patch in it succeeded, so a patch
//! naming a node that no longer existed was skipped in silence while its
//! siblings applied. The tree then disagreed with the VNode that produced the
//! batch, and nothing said so.
//!
//! Applying is all-or-nothing now, and a rejection names the patch, the node and
//! the reason so the caller can act on it rather than guess.

use std::fmt;

use crate::core::NodeKey;
use crate::layout::TextFlowError;
use crate::reconciler::{Patch, ReconcilePlanError, SiblingIdentity};

/// Which kind of patch failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchKind {
    Create,
    Update,
    Remove,
    Replace,
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

/// A rejected patch batch, with enough detail to locate the cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchError {
    pub kind: PatchKind,
    /// The node the patch names — the target, or the parent for create/reorder.
    pub key: NodeKey,
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
    Identity(ReconcilePlanError),
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
    AmbiguousLegacyNodeKey {
        key: NodeKey,
        scoped_match_count: usize,
    },
    CompositeIdentityCollision {
        identity: SiblingIdentity,
    },
    AmbiguousMeasurementKey {
        key_token: u64,
        scoped_match_count: usize,
    },
    AmbiguousMeasurementNodeIdentity {
        identity: SiblingIdentity,
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

/// Checked failure from applying a public raw [`Patch`] batch.
///
/// This boundary preserves the legacy [`PatchError`] surface while retaining
/// canonical identity and scoped-lookup causes for new callers.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectPatchError {
    Identity(ReconcilePlanError),
    Lookup(LayoutLookupError),
    Patch(PatchError),
}

impl fmt::Display for DirectPatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(source) => write!(formatter, "direct patch identity failed: {source}"),
            Self::Lookup(source) => write!(formatter, "direct patch lookup failed: {source}"),
            Self::Patch(source) => write!(formatter, "direct patch application failed: {source}"),
        }
    }
}

impl std::error::Error for DirectPatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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

pub(super) fn batch_key(patches: &[Patch]) -> NodeKey {
    patches
        .first()
        .map(|patch| match patch {
            Patch::Create { parent, .. } | Patch::Reorder { parent, .. } => *parent,
            Patch::Update { key, .. } | Patch::Remove { key } | Patch::Replace { key, .. } => *key,
        })
        .unwrap_or_else(NodeKey::root)
}
