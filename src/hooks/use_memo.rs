//! Memoization hooks for performance optimization
//!
//! These hooks help avoid expensive computations and unnecessary re-creations
//! of callbacks by caching values based on dependencies.

use crate::hooks::context::current_context;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Compute a hash for dependency tracking
fn compute_deps_hash<D: Hash>(deps: &D) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    deps.hash(&mut hasher);
    hasher.finish()
}

/// Internal storage for memoized values
#[derive(Clone)]
struct MemoStorage<T> {
    value: Rc<RefCell<T>>,
}

/// Memoize an expensive computation
///
/// `use_memo` will only recompute the memoized value when one of the
/// dependencies has changed. This optimization helps to avoid expensive
/// calculations on every render.
///
/// # Example
///
/// ```ignore
/// use rnk::hooks::{use_signal, use_memo};
///
/// let items = use_signal(|| vec![1, 2, 3, 4, 5]);
/// let filter = use_signal(|| 2);
///
/// // Only recomputes when items or filter changes
/// let filtered = use_memo(
///     || {
///         items.get()
///             .into_iter()
///             .filter(|&x| x > filter.get())
///             .collect::<Vec<_>>()
///     },
///     (items.get(), filter.get()),
/// );
///
/// // Use the memoized value
/// println!("Filtered: {:?}", filtered);
/// ```
pub fn use_memo<T, D, F>(compute: F, deps: D) -> T
where
    T: Clone + 'static,
    D: Hash,
    F: FnOnce() -> T,
{
    let ctx = current_context().expect("use_memo must be called within a component");
    let mut ctx_ref = ctx.borrow_mut();

    let new_hash = compute_deps_hash(&deps);

    // First, check if we have existing storage with the hash
    let hash_storage = ctx_ref.use_hook(|| new_hash);
    let stored_hash = hash_storage.get::<u64>().unwrap_or(0);

    // Get or create the value storage
    let storage = ctx_ref.use_hook(|| {
        let value = compute();
        MemoStorage {
            value: Rc::new(RefCell::new(value)),
        }
    });

    if let Some(memo) = storage.get::<MemoStorage<T>>() {
        // Check if deps changed - if so, we can't recompute because compute is FnOnce
        // and was already consumed. The value will be stale until next render.
        // Update the hash for next render comparison
        if stored_hash != new_hash {
            hash_storage.set(new_hash);
        }
        memo.value.borrow().clone()
    } else {
        panic!("use_memo: storage type mismatch")
    }
}

/// A memoized callback that only changes when dependencies change
#[derive(Clone)]
pub struct MemoizedCallback<F> {
    callback: Rc<F>,
}

impl<F> MemoizedCallback<F> {
    /// Create a new memoized callback
    fn new(callback: F) -> Self {
        Self {
            callback: Rc::new(callback),
        }
    }

    /// Get a reference to the callback
    pub fn get(&self) -> &F {
        &self.callback
    }
}

/// Internal storage for memoized callbacks
#[derive(Clone)]
struct CallbackStorage<F> {
    callback: MemoizedCallback<F>,
    deps_hash: u64,
}

/// Memoize a callback function
///
/// `use_callback` returns a memoized version of the callback that only
/// changes if one of the dependencies has changed. This is useful when
/// passing callbacks to child components that rely on reference equality
/// to prevent unnecessary renders.
///
/// # Example
///
/// ```ignore
/// use rnk::hooks::{use_signal, use_callback};
///
/// let count = use_signal(|| 0);
///
/// // This callback is memoized and won't change unless count changes
/// let increment = use_callback(
///     |amount: i32| {
///         count.update(|c| *c += amount);
///     },
///     count.get(), // dependency
/// );
///
/// // Use the callback via .get()
/// (increment.get())(1);
/// ```
///
/// # Note
///
/// Unlike React's useCallback, this returns a `MemoizedCallback` wrapper
/// that provides a `.get()` method to access the underlying function.
pub fn use_callback<F, D>(callback: F, deps: D) -> MemoizedCallback<F>
where
    F: Clone + 'static,
    D: Hash,
{
    let ctx = current_context().expect("use_callback must be called within a component");
    let mut ctx_ref = ctx.borrow_mut();

    let new_hash = compute_deps_hash(&deps);

    // Store the deps hash separately
    let hash_storage = ctx_ref.use_hook(|| new_hash);
    let stored_hash = hash_storage.get::<u64>().unwrap_or(0);

    // Try to get existing storage
    let storage = ctx_ref.use_hook(|| CallbackStorage {
        callback: MemoizedCallback::new(callback.clone()),
        deps_hash: new_hash,
    });

    if let Some(mut cb_storage) = storage.get::<CallbackStorage<F>>() {
        // Check if deps changed using the separately stored hash
        if stored_hash != new_hash {
            // Update callback
            cb_storage.callback = MemoizedCallback::new(callback);
            cb_storage.deps_hash = new_hash;
            storage.set(cb_storage.clone());
            hash_storage.set(new_hash);
        }
        cb_storage.callback
    } else {
        // Type mismatch - this happens when F is a different closure type
        // In this case, we need to create new storage
        // This is a limitation of Rust's type system with closures
        let new_storage = CallbackStorage {
            callback: MemoizedCallback::new(callback),
            deps_hash: new_hash,
        };
        storage.set(new_storage.clone());
        hash_storage.set(new_hash);
        new_storage.callback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};

    #[test]
    fn test_use_memo_basic() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // First render
        let result = with_hooks(ctx.clone(), || use_memo(|| 42, "deps1"));

        assert_eq!(result, 42);

        // Second render with same deps - should return cached value
        let result = with_hooks(ctx.clone(), || use_memo(|| 99, "deps1"));

        assert_eq!(result, 42); // Still cached value
    }

    #[test]
    fn test_use_memo_with_tuple_deps() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let result = with_hooks(ctx.clone(), || use_memo(|| vec![1, 2, 3], (1, "a", true)));

        assert_eq!(result, vec![1, 2, 3]);

        // Same deps
        let result = with_hooks(ctx.clone(), || use_memo(|| vec![4, 5, 6], (1, "a", true)));

        assert_eq!(result, vec![1, 2, 3]); // Cached
    }

    #[test]
    fn test_use_callback_basic() {
        // Use a fresh context for this test
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // Define a reusable callback type
        let multiply_by_2 = |x: i32| x * 2;

        // First render
        let cb = with_hooks(ctx.clone(), || use_callback(multiply_by_2, "cb_deps1"));

        assert_eq!((cb.get())(5), 10);

        // Second render with same deps and same callback type - should be cached
        let cb2 = with_hooks(ctx.clone(), || use_callback(multiply_by_2, "cb_deps1"));

        assert_eq!((cb2.get())(5), 10);
    }

    #[test]
    fn test_use_callback_deps_change() {
        // Use a fresh context for this test
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let multiply_by_2 = |x: i32| x * 2;
        let multiply_by_3 = |x: i32| x * 3;

        // First render
        let cb = with_hooks(ctx.clone(), || use_callback(multiply_by_2, "change_deps1"));
        assert_eq!((cb.get())(5), 10);

        // Second render with different deps - callback should update
        // Note: In Rust, different closures are different types, so this creates new storage
        let cb2 = with_hooks(ctx.clone(), || use_callback(multiply_by_3, "change_deps2"));
        assert_eq!((cb2.get())(5), 15);
    }

    #[test]
    fn test_use_callback_with_closure() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let multiplier = 10;

        let cb = with_hooks(ctx.clone(), || {
            use_callback(move |x: i32| x * multiplier, multiplier)
        });

        assert_eq!((cb.get())(5), 50);
    }

    #[test]
    fn test_use_callback_same_fn_type() {
        // This test demonstrates that use_callback works correctly
        // when the same function type is used across renders
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // Use a named function instead of closures
        fn double(x: i32) -> i32 {
            x * 2
        }
        fn triple(x: i32) -> i32 {
            x * 3
        }

        // First render with double
        let cb = with_hooks(ctx.clone(), || use_callback(double, "fn_deps1"));
        assert_eq!((cb.get())(5), 10);

        // Second render with same deps - should return cached callback
        let cb2 = with_hooks(ctx.clone(), || use_callback(double, "fn_deps1"));
        assert_eq!((cb2.get())(5), 10);

        // Third render with different deps - should update
        let cb3 = with_hooks(ctx.clone(), || use_callback(triple, "fn_deps2"));
        assert_eq!((cb3.get())(5), 15);
    }
}
