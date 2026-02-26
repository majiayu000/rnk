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
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

#[derive(Clone)]
struct IntervalTask {
    delay: Duration,
    next_fire: Instant,
    callback: Arc<dyn Fn() + Send + Sync>,
}

enum IntervalCommand {
    Register {
        id: u64,
        delay: Duration,
        callback: Arc<dyn Fn() + Send + Sync>,
    },
    Unregister {
        id: u64,
    },
}

static NEXT_INTERVAL_ID: AtomicU64 = AtomicU64::new(1);

fn interval_scheduler() -> &'static mpsc::Sender<IntervalCommand> {
    static SCHEDULER: OnceLock<mpsc::Sender<IntervalCommand>> = OnceLock::new();
    SCHEDULER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<IntervalCommand>();
        std::thread::Builder::new()
            .name("rnk-interval-scheduler".to_string())
            .spawn(move || run_interval_scheduler(rx))
            .expect("failed to spawn interval scheduler thread");
        tx
    })
}

fn apply_interval_command(tasks: &mut HashMap<u64, IntervalTask>, cmd: IntervalCommand) {
    match cmd {
        IntervalCommand::Register {
            id,
            delay,
            callback,
        } => {
            tasks.insert(
                id,
                IntervalTask {
                    delay,
                    next_fire: Instant::now() + delay,
                    callback,
                },
            );
        }
        IntervalCommand::Unregister { id } => {
            tasks.remove(&id);
        }
    }
}

fn fire_due_tasks(tasks: &mut HashMap<u64, IntervalTask>) {
    let now = Instant::now();

    for task in tasks.values_mut() {
        if task.next_fire > now {
            continue;
        }

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (task.callback)();
        }));

        while task.next_fire <= now {
            task.next_fire += task.delay;
        }
    }
}

fn run_interval_scheduler(rx: mpsc::Receiver<IntervalCommand>) {
    let mut tasks: HashMap<u64, IntervalTask> = HashMap::new();

    loop {
        if tasks.is_empty() {
            match rx.recv() {
                Ok(cmd) => apply_interval_command(&mut tasks, cmd),
                Err(_) => break,
            }
            continue;
        }

        let next_fire = tasks
            .values()
            .map(|task| task.next_fire)
            .min()
            .unwrap_or_else(Instant::now);

        let wait = next_fire.saturating_duration_since(Instant::now());

        match rx.recv_timeout(wait) {
            Ok(cmd) => apply_interval_command(&mut tasks, cmd),
            Err(mpsc::RecvTimeoutError::Timeout) => fire_due_tasks(&mut tasks),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

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

            let id = NEXT_INTERVAL_ID.fetch_add(1, Ordering::Relaxed);
            let cb = callback.clone();
            let callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || cb());
            let scheduler = interval_scheduler().clone();

            let _ = scheduler.send(IntervalCommand::Register {
                id,
                delay,
                callback,
            });

            Some(Box::new(move || {
                let _ = scheduler.send(IntervalCommand::Unregister { id });
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
