//! Reconciliation system for efficient UI updates
//!
//! The reconciler compares old and new VNode trees to produce
//! minimal patches that can be applied incrementally.

mod diff;
mod registry;

pub use diff::{Patch, PatchType, diff, diff_children};
pub use registry::{ComponentInstance, ComponentRegistry};
