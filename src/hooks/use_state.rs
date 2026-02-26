//! use_state hook with a simpler `(value, setter)` API.

use crate::hooks::use_signal::use_signal;

/// Setter handle returned by `use_state`.
#[derive(Clone)]
pub struct StateSetter<T> {
    signal: crate::hooks::Signal<T>,
}

impl<T: Clone + Send + Sync + 'static> StateSetter<T> {
    /// Replace state with a new value.
    pub fn set(&self, value: T) {
        self.signal.set(value);
    }

    /// Mutate state in place.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.signal.update(f);
    }
}

/// Create state with a tuple-style API similar to React's `useState`.
///
/// Returns `(current_value, setter)`.
pub fn use_state<T: Clone + Send + Sync + 'static>(
    init: impl FnOnce() -> T,
) -> (T, StateSetter<T>) {
    let signal = use_signal(init);
    let value = signal.get();
    (value, StateSetter { signal })
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
    fn test_use_state_outside_context() {
        let (value, set_value) = use_state(|| 1i32);
        assert_eq!(value, 1);
        set_value.set(5);
    }

    #[test]
    fn test_use_state_persists_between_renders() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let (_value, set_value) = with_hooks(ctx.clone(), || use_state(|| 0i32));
        set_value.set(7);

        let (value, _) = with_hooks(ctx.clone(), || use_state(|| 999i32));
        assert_eq!(value, 7);
    }

    #[test]
    fn test_use_state_setter_requests_render() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let render_count = Arc::new(AtomicUsize::new(0));
        let render_count_clone = render_count.clone();

        ctx.borrow_mut().set_render_callback(Arc::new(move || {
            render_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let (_value, set_value) = with_hooks(ctx.clone(), || use_state(|| 10i32));
        set_value.set(11);
        set_value.update(|v| *v += 1);

        assert_eq!(render_count.load(Ordering::SeqCst), 2);
    }
}
