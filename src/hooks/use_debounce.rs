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
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Debounce a value, only updating after the specified delay
///
/// Returns the debounced value that only updates after `delay` has passed
/// since the last change to `value`.
///
/// Uses channel-based cancellation: when a new value arrives, the old timer
/// thread is immediately unblocked via a dropped sender, so it exits without
/// waiting for the full delay.
pub fn use_debounce<T>(value: T, delay: Duration) -> T
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let debounced = use_signal(|| value.clone());
    let last_value = use_signal(|| value.clone());
    let last_delay = use_signal(|| delay);
    let cancel_tx: Signal<Option<mpsc::Sender<()>>> = use_signal(|| None);

    // Zero-delay debounce should update immediately.
    if delay.is_zero() {
        if last_value.get() != value {
            last_value.set(value.clone());
        }
        if last_delay.get() != delay {
            last_delay.set(delay);
        }
        if debounced.get() != value {
            debounced.set(value);
        }
        return debounced.get();
    }

    let value_changed = last_value.get() != value;
    let delay_changed = last_delay.get() != delay;

    if value_changed {
        last_value.set(value);
    }
    if delay_changed {
        last_delay.set(delay);
    }

    if value_changed || delay_changed {
        // Create a new channel. Storing the new tx drops the old one,
        // which immediately unblocks the old timer thread's recv_timeout
        // with a Disconnected error, causing it to exit.
        let (tx, rx) = mpsc::channel::<()>();
        cancel_tx.set(Some(tx));

        let last_value_clone = last_value.clone();
        let debounced_clone = debounced.clone();
        let wait = delay;

        std::thread::spawn(move || {
            match rx.recv_timeout(wait) {
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Delay expired naturally — commit the value.
                    let latest = last_value_clone.get();
                    if debounced_clone.get() != latest {
                        debounced_clone.set(latest);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) | Ok(()) => {
                    // Sender was dropped (newer change arrived) or explicit
                    // cancel signal — discard this timer.
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
    use std::cell::RefCell;
    use std::rc::Rc;

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
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let first = with_hooks(ctx.clone(), || {
            use_debounce("a".to_string(), Duration::from_millis(30))
        });
        assert_eq!(first, "a");

        let second = with_hooks(ctx.clone(), || {
            use_debounce("b".to_string(), Duration::from_millis(30))
        });
        assert_eq!(second, "a");

        std::thread::sleep(Duration::from_millis(60));

        let third = with_hooks(ctx.clone(), || {
            use_debounce("b".to_string(), Duration::from_millis(30))
        });
        assert_eq!(third, "b");
    }

    #[test]
    fn test_use_debounce_respects_updated_delay_for_pending_value() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let first = with_hooks(ctx.clone(), || {
            use_debounce("a".to_string(), Duration::from_millis(100))
        });
        assert_eq!(first, "a");

        let second = with_hooks(ctx.clone(), || {
            use_debounce("b".to_string(), Duration::from_millis(100))
        });
        assert_eq!(second, "a");

        // Same value, shorter delay. The pending update should now settle quickly.
        let third = with_hooks(ctx.clone(), || {
            use_debounce("b".to_string(), Duration::from_millis(10))
        });
        assert_eq!(third, "a");

        std::thread::sleep(Duration::from_millis(30));

        let fourth = with_hooks(ctx.clone(), || {
            use_debounce("b".to_string(), Duration::from_millis(10))
        });
        assert_eq!(fourth, "b");
    }
}
