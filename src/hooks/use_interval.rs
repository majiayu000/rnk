//! use_interval hook for periodic callbacks
//!
//! Similar to JavaScript's setInterval, but integrated with rnk's rendering.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use std::time::Duration;
//!
//! fn app() -> Element {
//!     let count = use_signal(|| 0);
//!
//!     // Increment every second
//!     use_interval(Duration::from_secs(1), move || {
//!         count.update(|c| *c += 1);
//!     });
//!
//!     Text::new(format!("Count: {}", count.get())).into_element()
//! }
//! ```

use crate::cmd::Cmd;
use crate::hooks::use_cmd::use_cmd_once;
use crate::hooks::use_effect::use_effect;
use std::time::Duration;

/// Run a callback at regular intervals
///
/// The callback will be called repeatedly with the specified delay between calls.
/// The interval starts immediately when the component mounts.
pub fn use_interval<F>(delay: Duration, callback: F)
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    use_interval_when(delay, true, callback);
}

/// Run a callback at regular intervals with control
///
/// Returns a handle that can be used to check if the interval is active.
/// The interval can be conditionally enabled/disabled.
pub fn use_interval_when<F>(delay: Duration, enabled: bool, callback: F)
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    use_effect(
        move || {
            if !enabled || delay.is_zero() {
                return None;
            }

            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
            let cb = callback.clone();

            let handle = std::thread::spawn(move || {
                loop {
                    match stop_rx.recv_timeout(delay) {
                        Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => cb(),
                    }
                }
            });

            Some(Box::new(move || {
                let _ = stop_tx.send(());
                let _ = handle.join();
            }) as Box<dyn FnOnce() + Send>)
        },
        (delay, enabled),
    );
}

/// Run a callback once after a delay (setTimeout equivalent)
///
/// The callback will be called once after the specified delay.
pub fn use_timeout<F>(delay: Duration, callback: F)
where
    F: FnOnce() + Send + 'static,
{
    use_cmd_once(move |_| {
        Cmd::sleep(delay).and_then(Cmd::perform(move || async move {
            callback();
        }))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_use_interval_compiles() {
        // Just verify the types compile correctly
        fn _test_interval() {
            use_interval(Duration::from_secs(1), || {
                println!("tick");
            });
        }
    }

    #[test]
    fn test_use_interval_when_compiles() {
        fn _test_interval_when() {
            use_interval_when(Duration::from_secs(1), true, || {
                println!("tick");
            });
        }
    }

    #[test]
    fn test_use_timeout_compiles() {
        fn _test_timeout() {
            use_timeout(Duration::from_secs(1), || {
                println!("done");
            });
        }
    }

    #[test]
    fn test_use_interval_runs_repeatedly() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        with_hooks(ctx.clone(), || {
            use_interval(Duration::from_millis(20), move || {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
        });

        std::thread::sleep(Duration::from_millis(90));
        let observed = count.load(Ordering::SeqCst);
        assert!(
            observed >= 2,
            "expected at least 2 interval ticks, observed {}",
            observed
        );

        drop(ctx);
    }

    #[test]
    fn test_use_interval_when_toggle_stops_ticks() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let count = Arc::new(AtomicUsize::new(0));

        let count_clone = count.clone();
        with_hooks(ctx.clone(), || {
            use_interval_when(Duration::from_millis(20), true, move || {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
        });

        std::thread::sleep(Duration::from_millis(70));
        let before_disable = count.load(Ordering::SeqCst);
        assert!(before_disable > 0, "interval did not tick before disable");

        let count_clone = count.clone();
        with_hooks(ctx.clone(), || {
            use_interval_when(Duration::from_millis(20), false, move || {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
        });

        // Allow any in-flight tick that raced with disable to drain.
        std::thread::sleep(Duration::from_millis(70));
        let settled = count.load(Ordering::SeqCst);
        assert!(
            settled <= before_disable + 1,
            "too many ticks after disable; before={}, settled={}",
            before_disable,
            settled
        );

        // After settling, count should no longer increase.
        std::thread::sleep(Duration::from_millis(80));
        let after_disable = count.load(Ordering::SeqCst);
        assert_eq!(
            settled, after_disable,
            "interval continued ticking after disable"
        );

        drop(ctx);
    }
}
