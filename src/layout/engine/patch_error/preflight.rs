#![forbid(missing_docs)]

//! Typed, pre-mutation rejection details for public raw patch batches.
//!
//! ```
//! use rnk::layout::DirectPatchPreflightCause;
//! let cause = DirectPatchPreflightCause::InvalidReorderMove {
//!     from: 2,
//!     to: 0,
//!     child_count: 2,
//! };
//! assert!(matches!(cause, DirectPatchPreflightCause::InvalidReorderMove { .. }));
//! ```

use std::fmt;

use crate::core::NodeKey;
use crate::reconciler::ReconcilePlanError;

use super::IncrementalPatchKind;

/// Why a raw patch failed before a candidate engine was cloned.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectPatchPreflightCause {
    /// The requested target is absent.
    MissingTarget,
    /// The requested target is ambiguous across parent scopes.
    AmbiguousTarget {
        /// Number of matching scopes.
        match_count: usize,
    },
    /// The requested parent is absent.
    MissingParent,
    /// The requested parent is ambiguous across parent scopes.
    AmbiguousParent {
        /// Number of matching scopes.
        match_count: usize,
    },
    /// Removing the canonical root is not a legal transition.
    RootMutation,
    /// The identity already exists in the selected parent.
    AlreadyExists,
    /// The new subtree collides with the virtual tree.
    SubtreeCollision {
        /// First colliding compatibility key.
        conflicting_key: NodeKey,
    },
    /// A reorder move is out of bounds, duplicated, omitted, or changes a positional identity.
    InvalidReorderMove {
        /// Current child slot named by the move.
        from: usize,
        /// Requested destination slot.
        to: usize,
        /// Current number of children under the resolved parent.
        child_count: usize,
    },
    /// The operation would silently change a positional identity.
    PositionalIdentityShift,
    /// A previous patch removed the requested identity.
    DependencyRemoved {
        /// Ordinal of the removal.
        prior_patch_index: usize,
    },
    /// A previous patch replaced the requested identity.
    DependencyReplaced {
        /// Ordinal of the replacement.
        prior_patch_index: usize,
    },
    /// `Update.old_props` does not equal the canonical committed properties.
    StaleProps,
    /// Duplicated public patch payload fields disagree.
    PayloadMismatch,
    /// Canonical identity validation rejected the virtual tree.
    Identity(ReconcilePlanError),
}

impl fmt::Display for DirectPatchPreflightCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTarget => formatter.write_str("target is missing"),
            Self::AmbiguousTarget { match_count } => {
                write!(formatter, "target matches {match_count} scopes")
            }
            Self::MissingParent => formatter.write_str("parent is missing"),
            Self::AmbiguousParent { match_count } => {
                write!(formatter, "parent matches {match_count} scopes")
            }
            Self::RootMutation => formatter.write_str("the canonical root cannot be removed"),
            Self::AlreadyExists => formatter.write_str("identity already exists"),
            Self::SubtreeCollision { conflicting_key } => {
                write!(formatter, "subtree collides at {conflicting_key:?}")
            }
            Self::InvalidReorderMove {
                from,
                to,
                child_count,
            } => write!(
                formatter,
                "invalid reorder move {from}->{to} for {child_count} children"
            ),
            Self::PositionalIdentityShift => {
                formatter.write_str("operation would shift a positional identity")
            }
            Self::DependencyRemoved { prior_patch_index } => {
                write!(formatter, "target was removed by patch {prior_patch_index}")
            }
            Self::DependencyReplaced { prior_patch_index } => {
                write!(
                    formatter,
                    "target was replaced by patch {prior_patch_index}"
                )
            }
            Self::StaleProps => formatter.write_str("old properties are stale"),
            Self::PayloadMismatch => formatter.write_str("duplicated patch payloads disagree"),
            Self::Identity(source) => write!(formatter, "identity validation failed: {source}"),
        }
    }
}

impl std::error::Error for DirectPatchPreflightCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Identity(source) => Some(source),
            _ => None,
        }
    }
}

/// Locator and cause for a pre-mutation raw-patch rejection.
///
/// ```
/// use rnk::{core::NodeKey, layout::{DirectPatchPreflightCause, DirectPatchPreflightError, IncrementalPatchKind}};
/// let error = DirectPatchPreflightError { patch_index: 2, kind: IncrementalPatchKind::Remove, key: Some(NodeKey::root()), parent: None, source: Box::new(DirectPatchPreflightCause::MissingTarget) };
/// assert_eq!(error.patch_index, 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPatchPreflightError {
    /// Original batch ordinal.
    pub patch_index: usize,
    /// Patch kind at that ordinal.
    pub kind: IncrementalPatchKind,
    /// Target key when the patch has one.
    pub key: Option<NodeKey>,
    /// Parent key when the patch has one.
    pub parent: Option<NodeKey>,
    /// Closed preflight cause.
    pub source: Box<DirectPatchPreflightCause>,
}

impl fmt::Display for DirectPatchPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} patch {} for key {:?}, parent {:?}: {}",
            self.kind, self.patch_index, self.key, self.parent, self.source
        )
    }
}

impl std::error::Error for DirectPatchPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}
