//! Reconciliation system for efficient UI updates
//!
//! The reconciler compares old and new VNode trees to produce
//! minimal patches that can be applied incrementally.
//!
//! This module backs the renderer internally and is not part of the primary
//! hooks-first application API.

mod diff;
mod error;
mod identity;
mod plan;
mod registry;

pub use diff::{Patch, diff, diff_children, try_diff, try_diff_children};
pub use error::{IdentityKeyKind, ReconcilePlanError};
pub use identity::SiblingIdentity;
pub(crate) use identity::{
    ScopedIdentityArena, ScopedNodeIdentity, compatibility_token_for_exact,
    insert_composite_projection, resolve_child_identity, semantically_equal_vnode_in,
};
#[cfg(test)]
pub(crate) use plan::plan_diff;
pub(crate) use plan::{
    PlannedNode, PlannedNodeAction, ReconcilePlan, plan_diff_in, plan_initial_tree,
    plan_initial_tree_in,
};
#[doc(hidden)]
pub use registry::{ComponentInstance, ComponentRegistry};
