//! Signal hook for reactive state management

use crate::hooks::context::{RenderCallback, current_context};
use std::sync::{Arc, RwLock};

/// A reactive signal that triggers re-renders when updated
#[derive(Clone)]
pub struct Signal<T> {
    value: Arc<RwLock<T>>,
    render_callback: Option<RenderCallback>,
}

impl<T> Signal<T> {
    /// Create a new signal with an initial value
    fn new(value: T, render_callback: Option<RenderCallback>) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
            render_callback,
        }
    }

    /// Get a clone of the current value
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.read().unwrap().clone()
    }

    /// Get a read lock guard, avoiding clone (useful for large data structures)
    ///
    /// # Panics
    /// Panics if the lock is poisoned. Use `try_get_ref()` for non-panicking version.
    pub fn get_ref(&self) -> std::sync::RwLockReadGuard<'_, T> {
        self.value.read().unwrap()
    }

    /// Try to get a clone of the value, returning None if lock is poisoned
    pub fn try_get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.value.read().ok().map(|g| g.clone())
    }

    /// Try to get a read lock guard, returning None if lock is poisoned
    pub fn try_get_ref(&self) -> Option<std::sync::RwLockReadGuard<'_, T>> {
        self.value.read().ok()
    }

    /// Get a reference to the current value
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.value.read().unwrap())
    }

    /// Try to get a reference to the current value, returning None if lock is poisoned
    pub fn try_with<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.value.read().ok().map(|guard| f(&guard))
    }

    /// Set a new value and trigger re-render
    pub fn set(&self, value: T) {
        *self.value.write().unwrap() = value;
        self.trigger_render();
    }

    /// Try to set a new value, returning false if lock is poisoned
    pub fn try_set(&self, value: T) -> bool {
        if let Ok(mut guard) = self.value.write() {
            *guard = value;
            self.trigger_render();
            true
        } else {
            false
        }
    }

    /// Update the value using a function and trigger re-render
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.write().unwrap());
        self.trigger_render();
    }

    /// Try to update the value using a function, returning false if lock is poisoned
    pub fn try_update(&self, f: impl FnOnce(&mut T)) -> bool {
        if let Ok(mut guard) = self.value.write() {
            f(&mut guard);
            self.trigger_render();
            true
        } else {
            false
        }
    }

    /// Modify the value without triggering re-render
    pub fn set_silent(&self, value: T) {
        *self.value.write().unwrap() = value;
    }

    /// Try to modify the value without triggering re-render, returning false if lock is poisoned
    pub fn try_set_silent(&self, value: T) -> bool {
        if let Ok(mut guard) = self.value.write() {
            *guard = value;
            true
        } else {
            false
        }
    }

    fn trigger_render(&self) {
        if let Some(callback) = &self.render_callback {
            callback();
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.read().unwrap())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signal({:?})", self.value.read().unwrap())
    }
}

/// Internal storage for signal
#[derive(Clone)]
struct SignalStorage<T> {
    signal: Signal<T>,
}

/// Create a reactive signal
///
/// # Example
///
/// ```ignore
/// let count = use_signal(|| 0);
/// count.set(count.get() + 1);
/// ```
pub fn use_signal<T: Clone + Send + Sync + 'static>(init: impl FnOnce() -> T) -> Signal<T> {
    let Some(ctx) = current_context() else {
        return Signal::new(init(), None);
    };
    let Ok(mut ctx_ref) = ctx.try_borrow_mut() else {
        return Signal::new(init(), None);
    };
    let render_callback = ctx_ref.get_render_callback();
    let mut init_fn = Some(init);

    let storage = ctx_ref.use_hook(|| SignalStorage {
        signal: Signal::new(
            init_fn
                .take()
                .expect("use_signal initializer should be available on first render")(),
            render_callback.clone(),
        ),
    });

    storage
        .get::<SignalStorage<T>>()
        .map(|s| s.signal)
        .unwrap_or_else(|| {
            Signal::new(
                init_fn
                    .take()
                    .expect("use_signal initializer should be available for fallback")(
                ),
                render_callback,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_signal_get_set() {
        let signal: Signal<i32> = Signal::new(42, None);
        assert_eq!(signal.get(), 42);

        signal.set(100);
        assert_eq!(signal.get(), 100);
    }

    #[test]
    fn test_signal_update() {
        let signal: Signal<i32> = Signal::new(10, None);

        signal.update(|v| *v += 5);
        assert_eq!(signal.get(), 15);

        signal.update(|v| *v *= 2);
        assert_eq!(signal.get(), 30);
    }

    #[test]
    fn test_signal_with() {
        let signal: Signal<Vec<i32>> = Signal::new(vec![1, 2, 3], None);

        let len = signal.with(|v| v.len());
        assert_eq!(len, 3);

        let sum: i32 = signal.with(|v| v.iter().sum());
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_use_signal_in_context() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // First render
        let signal1 = with_hooks(ctx.clone(), || use_signal(|| 0i32));

        assert_eq!(signal1.get(), 0);
        signal1.set(42);

        // Second render - should preserve value
        let signal2 = with_hooks(ctx.clone(), || {
            use_signal(|| 999i32) // init ignored
        });

        assert_eq!(signal2.get(), 42);
    }

    #[test]
    fn test_use_signal_initializer_runs_once_in_hook_context() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let init_calls = Arc::new(AtomicUsize::new(0));

        let calls = init_calls.clone();
        let _ = with_hooks(ctx.clone(), || {
            use_signal(|| {
                calls.fetch_add(1, Ordering::SeqCst);
                1i32
            })
        });

        let calls = init_calls.clone();
        let _ = with_hooks(ctx.clone(), || {
            use_signal(|| {
                calls.fetch_add(1, Ordering::SeqCst);
                2i32
            })
        });

        assert_eq!(init_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_multiple_signals() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        with_hooks(ctx.clone(), || {
            let count = use_signal(|| 0i32);
            let name = use_signal(|| "Alice".to_string());

            assert_eq!(count.get(), 0);
            assert_eq!(name.get(), "Alice");

            count.set(10);
            name.set("Bob".to_string());
        });

        // Verify persistence
        with_hooks(ctx.clone(), || {
            let count = use_signal(|| 999i32);
            let name = use_signal(|| "ignored".to_string());

            assert_eq!(count.get(), 10);
            assert_eq!(name.get(), "Bob");
        });
    }

    #[test]
    fn test_use_signal_without_context_does_not_panic() {
        let signal = use_signal(|| 7i32);
        assert_eq!(signal.get(), 7);
        signal.set(9);
        assert_eq!(signal.get(), 9);
    }

    #[test]
    fn test_signal_thread_safety() {
        use std::thread;

        let signal: Signal<i32> = Signal::new(0, None);
        let signal_clone = signal.clone();

        let handle = thread::spawn(move || {
            for _ in 0..100 {
                signal_clone.update(|v| *v += 1);
            }
        });

        for _ in 0..100 {
            signal.update(|v| *v += 1);
        }

        handle.join().unwrap();
        assert_eq!(signal.get(), 200);
    }
}
