//! Lock poisoning recovery utilities
//!
//! When a thread panics while holding a lock, the lock becomes "poisoned".
//! These helpers recover from poisoned locks instead of propagating the panic,
//! which is appropriate for UI state where stale data is preferable to crashing.

use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Acquire a read lock, recovering from poisoning if necessary.
pub(crate) fn read_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Acquire a write lock, recovering from poisoning if necessary.
pub(crate) fn write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
