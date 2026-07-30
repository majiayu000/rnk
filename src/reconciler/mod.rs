//! Reconciliation system for efficient UI updates
//!
//! The reconciler compares old and new VNode trees to produce
//! minimal patches that can be applied incrementally.
//!
//! This module backs the renderer internally and is not part of the primary
//! hooks-first application API.

mod diff;
mod identity;
mod registry;

pub use diff::{Patch, diff, diff_children};
pub use identity::SiblingIdentity;
#[doc(hidden)]
pub use registry::{ComponentInstance, ComponentRegistry};
