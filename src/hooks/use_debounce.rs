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

enum DebounceMessage<T> {
    Update { value: T, delay: Duration },
}

fn spawn_debounce_worker<T>(debounced: Signal<T>) -> mpsc::Sender<DebounceMessage<T>>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel::<DebounceMessage<T>>();

    std::thread::spawn(move || {
        let mut pending: Option<(T, Duration, Instant)> = None;

        loop {
            if let Some((pending_value, pending_delay, started_at)) = pending.as_ref() {
                let remaining = pending_delay.saturating_sub(started_at.elapsed());

                match rx.recv_timeout(remaining) {
                    Ok(DebounceMessage::Update { value, delay }) => {
                        if delay.is_zero() {
                            if debounced.get() != value {
                                debounced.set(value);
                            }
                            pending = None;
                        } else {
                            pending = Some((value, delay, Instant::now()));
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let value = pending_value.clone();
                        if debounced.get() != value {
                            debounced.set(value);
                        }
                        pending = None;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match rx.recv() {
                    Ok(DebounceMessage::Update { value, delay }) => {
                        if delay.is_zero() {
                            if debounced.get() != value {
                                debounced.set(value);
                            }
                        } else {
                            pending = Some((value, delay, Instant::now()));
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    });

    tx
}

/// Debounce a value, only updating after the specified delay
///
/// Returns the debounced value that only updates after `delay` has passed
/// since the last change to `value`.
///
/// Uses one worker thread per hook instance. New values are pushed to the
/// worker, which keeps only the latest pending value and delay.
pub fn use_debounce<T>(value: T, delay: Duration) -> T
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let debounced = use_signal(|| value.clone());
    let last_value = use_signal(|| value.clone());
    let last_delay = use_signal(|| delay);
    let worker_tx: Signal<Option<mpsc::Sender<DebounceMessage<T>>>> = use_signal(|| None);

    if worker_tx.get().is_none() {
        worker_tx.set(Some(spawn_debounce_worker(debounced.clone())));
    }

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
        let latest_value = last_value.get();
        let latest_delay = last_delay.get();

        let mut sent = false;

        if let Some(tx) = worker_tx.get() {
            sent = tx
                .send(DebounceMessage::Update {
                    value: latest_value.clone(),
                    delay: latest_delay,
                })
                .is_ok();
        }

        if !sent {
            // Worker might have exited unexpectedly. Recreate and retry once.
            let tx = spawn_debounce_worker(debounced.clone());
            let _ = tx.send(DebounceMessage::Update {
                value: latest_value,
                delay: latest_delay,
            });
            worker_tx.set(Some(tx));
        }
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

        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            let settled = with_hooks(ctx.clone(), || {
                use_debounce("b".to_string(), Duration::from_millis(30))
            });

            if settled == "b" {
                break;
            }

            if Instant::now() >= deadline {
                panic!("debounced value did not settle to 'b' before timeout");
            }

            std::thread::sleep(Duration::from_millis(5));
        }
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

        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            let settled = with_hooks(ctx.clone(), || {
                use_debounce("b".to_string(), Duration::from_millis(10))
            });

            if settled == "b" {
                break;
            }

            if Instant::now() >= deadline {
                panic!("debounced value did not settle to 'b' before timeout");
            }

            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
