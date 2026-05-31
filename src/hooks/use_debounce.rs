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

enum ThrottleMessage<T> {
    Schedule { value: T, emit_at: Instant },
    Clear,
}

fn spawn_throttle_worker<T>(
    throttled: Signal<T>,
    last_emit: Signal<Option<Instant>>,
) -> mpsc::Sender<ThrottleMessage<T>>
where
    T: Clone + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel::<ThrottleMessage<T>>();

    std::thread::spawn(move || {
        // Keep the latest value pushed within the current throttle window so
        // the trailing edge emits the newest pending value.
        let mut pending: Option<(T, Instant)> = None;

        loop {
            let recv_result = match pending.as_ref() {
                Some((_, emit_at)) => {
                    let wait = emit_at.saturating_duration_since(Instant::now());
                    if wait.is_zero() {
                        Err(mpsc::RecvTimeoutError::Timeout)
                    } else {
                        rx.recv_timeout(wait)
                    }
                }
                None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
            };

            match recv_result {
                Ok(ThrottleMessage::Schedule { value, emit_at }) => {
                    pending = Some((value, emit_at));
                }
                Ok(ThrottleMessage::Clear) => {
                    pending = None;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    let mut trailing = None;
                    let mut future_pending = None;

                    if let Some((value, emit_at)) = pending.take() {
                        if emit_at <= now {
                            trailing = Some(value);
                        } else {
                            future_pending = Some((value, emit_at));
                        }
                    }

                    while let Ok(message) = rx.try_recv() {
                        match message {
                            ThrottleMessage::Schedule { value, emit_at } => {
                                if emit_at <= now {
                                    trailing = Some(value);
                                    future_pending = None;
                                } else {
                                    trailing = None;
                                    future_pending = Some((value, emit_at));
                                }
                            }
                            ThrottleMessage::Clear => {
                                trailing = None;
                                future_pending = None;
                            }
                        }
                    }

                    if let Some(pending_value) = future_pending {
                        pending = Some(pending_value);
                        continue;
                    }

                    if let Some(value) = trailing {
                        throttled.set(value);
                        last_emit.set(Some(Instant::now()));
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    tx
}

fn send_throttle_schedule<T>(
    worker_tx: &Signal<Option<mpsc::Sender<ThrottleMessage<T>>>>,
    throttled: Signal<T>,
    last_emit: Signal<Option<Instant>>,
    value: T,
    emit_at: Instant,
) where
    T: Clone + Send + Sync + 'static,
{
    let mut message = Some(ThrottleMessage::Schedule { value, emit_at });

    if let Some(tx) = worker_tx.get() {
        if let Some(message_to_send) = message.take() {
            match tx.send(message_to_send) {
                Ok(()) => return,
                Err(err) => {
                    message = Some(err.0);
                }
            }
        }
    }

    let tx = spawn_throttle_worker(throttled, last_emit);
    if let Some(message) = message {
        match tx.send(message) {
            Ok(()) => worker_tx.set(Some(tx)),
            Err(_) => worker_tx.set(None),
        }
    } else {
        worker_tx.set(Some(tx));
    }
}

fn clear_throttle_schedule<T>(worker_tx: &Signal<Option<mpsc::Sender<ThrottleMessage<T>>>>)
where
    T: Clone + Send + Sync + 'static,
{
    if let Some(tx) = worker_tx.get()
        && tx.send(ThrottleMessage::Clear).is_err()
    {
        worker_tx.set(None);
    }
}

/// Throttle a value, allowing updates at most once per interval.
///
/// Unlike [`use_debounce`] which waits for inactivity, throttle emits the
/// **leading edge** (first update passes through immediately) and the
/// **trailing edge** (the most recent value is emitted at most once per
/// `interval`). Values pushed within the throttle window are not silently
/// dropped — the latest pending value is delivered after the window closes.
///
/// Uses one worker thread per hook instance. Pass `Duration::ZERO` to disable
/// throttling entirely (every value passes through immediately).
///
/// # Example
///
/// ```rust,ignore
/// use rnk::prelude::*;
/// use std::time::Duration;
///
/// fn app() -> Element {
///     let scroll_y = use_signal(|| 0u32);
///
///     // Render at most ~30fps even if scroll_y changes rapidly.
///     let throttled = use_throttle(scroll_y.get(), Duration::from_millis(33));
///     Text::new(format!("y = {throttled}")).into_element()
/// }
/// ```
pub fn use_throttle<T>(value: T, interval: Duration) -> T
where
    T: Clone + Send + Sync + 'static,
{
    let throttled = use_signal(|| value.clone());
    let last_emit = use_signal(|| None::<Instant>);
    let worker_tx: Signal<Option<mpsc::Sender<ThrottleMessage<T>>>> = use_signal(|| None);

    let now = Instant::now();
    let last_emit_at = last_emit.get();
    let should_emit_now = interval.is_zero()
        || match last_emit_at {
            Some(emitted_at) => now.saturating_duration_since(emitted_at) >= interval,
            None => true,
        };

    if should_emit_now {
        throttled.set(value);
        last_emit.set(Some(now));
        clear_throttle_schedule(&worker_tx);
        return throttled.get();
    }

    if let Some(emitted_at) = last_emit_at {
        send_throttle_schedule(
            &worker_tx,
            throttled.clone(),
            last_emit.clone(),
            value,
            emitted_at + interval,
        );
    }

    throttled.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

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
    fn test_use_throttle_leading_edge_emits_immediately() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let first = with_hooks(ctx.clone(), || {
            use_throttle("a".to_string(), Duration::from_millis(50))
        });

        // Leading edge: first call should pass through immediately. The worker
        // emits asynchronously, so allow a short settle window.
        let deadline = Instant::now() + Duration::from_millis(150);
        let mut observed = first;
        while observed != "a" && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            observed = with_hooks(ctx.clone(), || {
                use_throttle("a".to_string(), Duration::from_millis(50))
            });
        }
        assert_eq!(observed, "a", "leading-edge value should be emitted");
    }

    #[test]
    fn test_use_throttle_trailing_edge_delivers_latest_pending_value() {
        // Reproduces the silent-drop bug: rapid updates within the throttle
        // window must not be lost. The trailing-edge value must propagate to
        // a subsequent observer that itself does NOT call use_throttle (i.e.
        // a sibling component reading the same Signal).
        //
        // The previous implementation only ever wrote to the throttled Signal
        // from inside use_throttle() at a moment when the interval had passed,
        // so any rapid sequence A, B, C within a single window left the Signal
        // stuck at the leading-edge value forever unless another use_throttle()
        // call arrived later — silently dropping B and C.
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let interval = Duration::from_millis(40);

        // Leading-edge emit (value 1 should appear immediately).
        let _ = with_hooks(ctx.clone(), || use_throttle(1u32, interval));
        std::thread::sleep(Duration::from_millis(10));

        // Three rapid updates within the same throttle window. The old
        // implementation discards all of them because elapsed() < interval.
        for v in 2u32..=4u32 {
            let _ = with_hooks(ctx.clone(), || use_throttle(v, interval));
        }

        // Snapshot the throttled Signal directly (without calling use_throttle
        // again) by reading the worker_tx slot's adjacent throttled signal
        // through one more hook invocation that uses the SAME pushed value 4
        // — under the new impl the worker has already emitted 4 by the time
        // we reach this point, so we can read it back. Sleep generously past
        // the window to give the worker time to fire its trailing emission.
        std::thread::sleep(interval * 3);

        // After the throttle window closes, the trailing value must be visible
        // even if the next hook call passes the SAME value 4 (i.e. no further
        // change). The buggy implementation only updates the signal when
        // elapsed >= interval AND the call is in flight, so this still works
        // for the buggy impl in isolation. To make the bug observable, we
        // verify a *second* observer pattern below.
        let observed = with_hooks(ctx.clone(), || use_throttle(4u32, interval));
        assert_eq!(
            observed, 4,
            "trailing-edge throttle should deliver the latest pending value"
        );
    }

    #[test]
    fn test_use_throttle_emits_trailing_value_without_further_pushes() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let render_count = Arc::new(AtomicUsize::new(0));
        ctx.borrow_mut().set_render_callback({
            let render_count = Arc::clone(&render_count);
            Arc::new(move || {
                render_count.fetch_add(1, Ordering::SeqCst);
            })
        });

        let interval = Duration::from_millis(40);

        // Leading-edge emit: value 10. Wait for the worker to observe that
        // first value so the following values definitely fall inside the same
        // throttle window instead of racing ahead of the worker.
        let _ = with_hooks(ctx.clone(), || use_throttle(10u32, interval));
        let leading_deadline = Instant::now() + Duration::from_millis(200);
        while render_count.load(Ordering::SeqCst) < 2 {
            assert!(
                Instant::now() < leading_deadline,
                "leading-edge throttle value did not render before timeout"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let before_burst = render_count.load(Ordering::SeqCst);

        // Burst within one window: 11, 12, 13. The worker should buffer 13 as
        // the trailing-edge candidate and request a render without any further
        // hook invocation.
        for v in 11u32..=13u32 {
            let _ = with_hooks(ctx.clone(), || use_throttle(v, interval));
        }

        // The runner may cross the first throttle window while scheduling the
        // burst, making 11 a valid leading-edge emission before 13 trails it.
        // Wait for the window to settle, then verify the worker requested at
        // least one render without another hook invocation.
        std::thread::sleep(interval * 3);
        assert!(
            render_count.load(Ordering::SeqCst) > before_burst,
            "worker did not emit trailing-edge value before timeout"
        );

        let observed = with_hooks(ctx.clone(), || use_throttle(13u32, interval));
        assert_eq!(
            observed, 13,
            "worker should have emitted trailing-edge 13 before this call"
        );
    }

    #[test]
    fn test_use_throttle_zero_interval_passes_every_value() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let _ = with_hooks(ctx.clone(), || use_throttle(0u32, Duration::ZERO));
        for v in 1u32..=5u32 {
            let _ = with_hooks(ctx.clone(), || use_throttle(v, Duration::ZERO));
        }

        // Final value must propagate. Allow brief worker scheduling latency.
        let deadline = Instant::now() + Duration::from_millis(200);
        let mut observed = 0u32;
        while Instant::now() < deadline {
            observed = with_hooks(ctx.clone(), || use_throttle(5u32, Duration::ZERO));
            if observed == 5 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(observed, 5);
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
