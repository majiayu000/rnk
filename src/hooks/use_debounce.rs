//! use_debounce hook for debouncing rapid value changes
//!
//! Useful for search inputs and other scenarios where you want to
//! delay processing until the user stops typing.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use std::time::Duration;
//!
//! fn search_component() -> Element {
//!     let query = use_signal(|| String::new());
//!     let results = use_signal(|| Vec::new());
//!
//!     // Only search after user stops typing for 300ms
//!     let debounced_query = use_debounce(query.get(), Duration::from_millis(300));
//!
//!     // Perform search when debounced value changes
//!     use_effect(move || {
//!         if !debounced_query.is_empty() {
//!             // Perform search...
//!         }
//!     }, vec![debounced_query.clone()]);
//!
//!     // ... render UI
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};
use std::time::{Duration, Instant};

/// Debounce a value, only updating after the specified delay
///
/// Returns the debounced value that only updates after `delay` has passed
/// since the last change to `value`.
pub fn use_debounce<T>(value: T, delay: Duration) -> T
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let debounced = use_signal(|| value.clone());
    let last_value = use_signal(|| value.clone());
    let generation = use_signal(|| 0u64);

    // Zero-delay debounce should update immediately.
    if delay.is_zero() {
        if debounced.get() != value {
            debounced.set(value);
        }
        return debounced.get();
    }

    if last_value.get() != value {
        last_value.set(value);
        generation.update(|g| *g = g.wrapping_add(1));

        let expected_generation = generation.get();
        let generation_clone = generation.clone();
        let last_value_clone = last_value.clone();
        let debounced_clone = debounced.clone();

        std::thread::spawn(move || {
            std::thread::sleep(delay);
            if generation_clone.get() == expected_generation {
                let latest = last_value_clone.get();
                if debounced_clone.get() != latest {
                    debounced_clone.set(latest);
                }
            }
        });
    }

    debounced.get()
}

/// Handle for tracking debounce state
#[derive(Clone)]
pub struct DebounceHandle {
    pending: Signal<bool>,
    last_trigger: Signal<Instant>,
    delay: Duration,
}

impl DebounceHandle {
    /// Trigger the debounce timer
    pub fn trigger(&self) {
        self.pending.set(true);
        self.last_trigger.set(Instant::now());
    }

    /// Check if the debounce period has elapsed
    pub fn is_ready(&self) -> bool {
        self.pending.get() && self.last_trigger.get().elapsed() >= self.delay
    }

    /// Reset the debounce state
    pub fn reset(&self) {
        self.pending.set(false);
    }

    /// Check if there's a pending trigger
    pub fn is_pending(&self) -> bool {
        self.pending.get()
    }
}

/// Create a debounce handle for manual control
pub fn use_debounce_handle(delay: Duration) -> DebounceHandle {
    let pending = use_signal(|| false);
    let last_trigger = use_signal(Instant::now);

    DebounceHandle {
        pending,
        last_trigger,
        delay,
    }
}

/// Throttle a value, only allowing updates at most once per interval
///
/// Unlike debounce which waits for inactivity, throttle ensures
/// updates happen at a regular maximum rate.
pub fn use_throttle<T>(value: T, interval: Duration) -> T
where
    T: Clone + Send + Sync + 'static,
{
    let throttled = use_signal(|| value.clone());
    let last_update = use_signal(Instant::now);

    // Check if enough time has passed since last update
    if last_update.get().elapsed() >= interval {
        throttled.set(value);
        last_update.set(Instant::now());
    }

    throttled.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_use_debounce_compiles() {
        fn _test() {
            let _debounced = use_debounce("test".to_string(), Duration::from_millis(300));
        }
    }

    #[test]
    fn test_use_throttle_compiles() {
        fn _test() {
            let _throttled = use_throttle(42, Duration::from_millis(100));
        }
    }

    #[test]
    fn test_debounce_handle_compiles() {
        fn _test() {
            let handle = use_debounce_handle(Duration::from_millis(300));
            handle.trigger();
            let _ = handle.is_ready();
            handle.reset();
        }
    }

    #[test]
    fn test_use_debounce_updates_after_delay() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));

        let first = with_hooks(ctx.clone(), || use_debounce("a".to_string(), Duration::from_millis(30)));
        assert_eq!(first, "a");

        let second =
            with_hooks(ctx.clone(), || use_debounce("b".to_string(), Duration::from_millis(30)));
        assert_eq!(second, "a");

        std::thread::sleep(Duration::from_millis(60));

        let third =
            with_hooks(ctx.clone(), || use_debounce("b".to_string(), Duration::from_millis(30)));
        assert_eq!(third, "b");
    }
}
