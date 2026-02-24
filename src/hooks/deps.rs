//! Shared dependency hashing trait for hooks.
//!
//! Used by `use_effect`, `use_cmd`, and `use_memo` to detect dependency changes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Trait for computing a hash of hook dependencies.
///
/// Automatically implemented for all `Hash` types.
pub trait DepsHash {
    /// Compute a u64 hash of the dependency value.
    fn deps_hash(&self) -> u64;
}

impl<T: Hash> DepsHash for T {
    fn deps_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
