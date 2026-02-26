//! use_ref hook for mutable values that should not trigger re-renders.

use crate::hooks::context::current_context;
use crate::hooks::lock_utils::{read_or_recover, write_or_recover};
use std::sync::{Arc, RwLock};

/// A mutable reference-like handle that does not request renders on updates.
#[derive(Clone)]
pub struct RefHandle<T> {
    value: Arc<RwLock<T>>,
}

impl<T> RefHandle<T> {
    fn new(value: Arc<RwLock<T>>) -> Self {
        Self { value }
    }

    /// Get a clone of the current value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        read_or_recover(&self.value).clone()
    }

    /// Try to get a clone of the current value, returning None if lock is poisoned.
    pub fn try_get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.value.read().ok().map(|g| g.clone())
    }

    /// Access the current value by shared reference.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&read_or_recover(&self.value))
    }

    /// Try to access the current value by shared reference.
    pub fn try_with<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.value.read().ok().map(|guard| f(&guard))
    }

    /// Replace the current value.
    pub fn set(&self, value: T) {
        *write_or_recover(&self.value) = value;
    }

    /// Try to replace the current value, returning false if lock is poisoned.
    pub fn try_set(&self, value: T) -> bool {
        if let Ok(mut guard) = self.value.write() {
            *guard = value;
            true
        } else {
            false
        }
    }

    /// Mutate the current value in place.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut write_or_recover(&self.value));
    }

    /// Try to mutate the current value in place, returning false if lock is poisoned.
    pub fn try_update(&self, f: impl FnOnce(&mut T)) -> bool {
        if let Ok(mut guard) = self.value.write() {
            f(&mut guard);
            true
        } else {
            false
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for RefHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RefHandle({:?})", read_or_recover(&self.value))
    }
}

#[derive(Clone)]
struct RefStorage<T> {
    value: Arc<RwLock<T>>,
}

/// Create a mutable ref-like handle that persists across renders.
///
/// Unlike `use_signal`, updates made through this handle do not request a re-render.
pub fn use_ref<T: Clone + Send + Sync + 'static>(init: impl FnOnce() -> T) -> RefHandle<T> {
    let Some(ctx) = current_context() else {
        return RefHandle::new(Arc::new(RwLock::new(init())));
    };
    let Ok(mut ctx_ref) = ctx.try_borrow_mut() else {
        return RefHandle::new(Arc::new(RwLock::new(init())));
    };
    let mut init_fn = Some(init);

    let storage = ctx_ref.use_hook(|| RefStorage {
        value: Arc::new(RwLock::new(init_fn
            .take()
            .expect("use_ref initializer should be available on first render")(
        ))),
    });

    storage
        .get::<RefStorage<T>>()
        .map(|s| RefHandle::new(s.value))
        .unwrap_or_else(|| {
            RefHandle::new(Arc::new(RwLock::new(init_fn
                .take()
                .expect("use_ref initializer should be available for fallback")(
            ))))
        })
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
    fn test_use_ref_outside_context() {
        let value = use_ref(|| 1i32);
        assert_eq!(value.get(), 1);
        value.set(7);
        assert_eq!(value.get(), 7);
    }

    #[test]
    fn test_use_ref_persists_across_renders() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let first = with_hooks(ctx.clone(), || {
            let value = use_ref(|| 1usize);
            value.set(42);
            value
        });
        assert_eq!(first.get(), 42);

        let second = with_hooks(ctx.clone(), || use_ref(|| 999usize));
        assert_eq!(second.get(), 42);
    }

    #[test]
    fn test_use_ref_initializer_runs_once_in_hook_context() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let init_calls = Arc::new(AtomicUsize::new(0));

        let calls = init_calls.clone();
        let _ = with_hooks(ctx.clone(), || {
            use_ref(|| {
                calls.fetch_add(1, Ordering::SeqCst);
                1i32
            })
        });

        let calls = init_calls.clone();
        let _ = with_hooks(ctx.clone(), || {
            use_ref(|| {
                calls.fetch_add(1, Ordering::SeqCst);
                2i32
            })
        });

        assert_eq!(init_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_use_ref_does_not_trigger_render_callback() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let render_count = Arc::new(AtomicUsize::new(0));
        let render_count_clone = render_count.clone();

        ctx.borrow_mut().set_render_callback(Arc::new(move || {
            render_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let value = with_hooks(ctx.clone(), || use_ref(|| 10i32));
        value.set(11);
        value.update(|v| *v += 5);

        assert_eq!(value.get(), 16);
        assert_eq!(render_count.load(Ordering::SeqCst), 0);
    }
}
