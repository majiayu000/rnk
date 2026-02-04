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
use std::time::Duration;

/// Run a callback at regular intervals
///
/// The callback will be called repeatedly with the specified delay between calls.
/// The interval starts immediately when the component mounts.
pub fn use_interval<F>(delay: Duration, callback: F)
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    use_cmd_once(move |_| {
        let cb = callback.clone();
        Cmd::every(delay, move |_| {
            cb();
        })
    });
}

/// Run a callback at regular intervals with control
///
/// Returns a handle that can be used to check if the interval is active.
/// The interval can be conditionally enabled/disabled.
pub fn use_interval_when<F>(delay: Duration, enabled: bool, callback: F)
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    use_cmd_once(move |_| {
        if enabled {
            let cb = callback.clone();
            Cmd::every(delay, move |_| {
                cb();
            })
        } else {
            Cmd::none()
        }
    });
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
}
