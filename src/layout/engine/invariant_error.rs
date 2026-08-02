#![forbid(missing_docs)]

//! Closed target-exact invariant failures for transactional layout.

use std::fmt;

/// A concrete mismatch found while validating a prepared layout candidate.
///
/// ```
/// use rnk::layout::IncrementalInvariantError;
/// assert_eq!(IncrementalInvariantError::MissingRoot.to_string(), "layout root is missing");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrementalInvariantError {
    /// No committed or candidate root was present.
    MissingRoot,
    /// The root identifier was not readable or did not represent the target root.
    InvalidRoot,
    /// Traversing backend children reached the same node more than once.
    ReachableNodeCycle,
    /// The reachable backend node set differed from the target node set.
    ReachableNodeSetMismatch,
    /// The backend retained an orphan or omitted a target node.
    NodeCountMismatch,
    /// The scoped identity map differed from the target identities.
    ScopedMapMismatch,
    /// Current-frame element aliases differed from the target elements.
    ElementMapMismatch,
    /// A legacy or composite compatibility projection differed from the target.
    CompatibilityMapMismatch,
    /// A mapped backend node was missing, invalid, duplicated, or unreachable.
    InvalidMappedNode,
    /// A parent's backend child order differed from the target order.
    ChildOrderMismatch,
    /// A target node had no readable computed layout.
    MissingComputedLayout,
    /// Style, text source, TextFlow, viewport, or frame context was stale.
    CurrentFrameContextMismatch,
}

impl fmt::Display for IncrementalInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingRoot => "layout root is missing",
            Self::InvalidRoot => "layout root is invalid",
            Self::ReachableNodeCycle => "reachable layout nodes contain a cycle or duplicate",
            Self::ReachableNodeSetMismatch => "reachable layout nodes differ from the target",
            Self::NodeCountMismatch => "layout node count differs from the target",
            Self::ScopedMapMismatch => "scoped identity map differs from the target",
            Self::ElementMapMismatch => "element aliases differ from the target",
            Self::CompatibilityMapMismatch => {
                "compatibility identity projection differs from the target"
            }
            Self::InvalidMappedNode => "an identity map references an invalid layout node",
            Self::ChildOrderMismatch => "layout child order differs from the target",
            Self::MissingComputedLayout => "a target node has no computed layout",
            Self::CurrentFrameContextMismatch => {
                "layout style, text flow, viewport, or frame context is stale"
            }
        })
    }
}

impl std::error::Error for IncrementalInvariantError {}
