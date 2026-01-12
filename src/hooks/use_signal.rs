//! Signal hook for reactive state management

use crate::hooks::context::{RenderCallback, current_context};
use std::cell::RefCell;
use std::rc::Rc;

/// A reactive signal that triggers re-renders when updated
#[derive(Clone)]
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    render_callback: Option<RenderCallback>,
}

impl<T> Signal<T> {
    /// Create a new signal with an initial value
    fn new(value: T, render_callback: Option<RenderCallback>) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            render_callback,
        }
    }

    /// Get a clone of the current value
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }

    /// Get a reference to the current value
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.value.borrow())
    }

    /// Set a new value and trigger re-render
    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.trigger_render();
    }

    /// Update the value using a function and trigger re-render
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.borrow_mut());
        self.trigger_render();
    }

    /// Modify the value without triggering re-render
    pub fn set_silent(&self, value: T) {
        *self.value.borrow_mut() = value;
    }

    fn trigger_render(&self) {
        if let Some(callback) = &self.render_callback {
            callback();
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.borrow())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signal({:?})", self.value.borrow())
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
pub fn use_signal<T: Clone + 'static>(init: impl FnOnce() -> T) -> Signal<T> {
    let ctx = current_context().expect("use_signal must be called within a component");
    let mut ctx_ref = ctx.borrow_mut();

    let render_callback = ctx_ref.get_render_callback();

    // Use a wrapper that can be cloned
    let init_value = init();

    let storage = ctx_ref.use_hook(|| SignalStorage {
        signal: Signal::new(init_value.clone(), render_callback.clone()),
    });

    // Get the signal from storage
    storage
        .get::<SignalStorage<T>>()
        .map(|s| s.signal)
        .unwrap_or_else(|| {
            // This shouldn't happen, but provide a sensible default
            Signal::new(init_value, render_callback)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};

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
}
