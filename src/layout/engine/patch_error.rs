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
