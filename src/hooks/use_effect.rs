//! Effect hook for side effects

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::hooks::context::{current_context, Effect};

/// Trait for types that can be used as effect dependencies
pub trait Deps {
    fn to_hash(&self) -> u64;
}

impl Deps for () {
    fn to_hash(&self) -> u64 {
        0
    }
}

impl<T: Hash> Deps for (T,) {
    fn to_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T1: Hash, T2: Hash> Deps for (T1, T2) {
    fn to_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        self.1.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T1: Hash, T2: Hash, T3: Hash> Deps for (T1, T2, T3) {
    fn to_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        self.1.hash(&mut hasher);
        self.2.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T1: Hash, T2: Hash, T3: Hash, T4: Hash> Deps for (T1, T2, T3, T4) {
    fn to_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        self.1.hash(&mut hasher);
        self.2.hash(&mut hasher);
        self.3.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Hash> Deps for Vec<T> {
    fn to_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for item in self {
            item.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Storage for effect state
#[derive(Clone)]
struct EffectStorage {
    prev_deps_hash: Option<u64>,
}

/// Run a side effect after render
///
/// The effect runs after every render by default.
/// Pass dependencies to only run when they change.
///
/// # Example
///
/// ```ignore
/// // Run on every render
/// use_effect(|| {
///     println!("Rendered!");
///     None  // No cleanup
/// }, ());
///
/// // Run when count changes
/// use_effect(|| {
///     println!("Count changed to {}", count.get());
///     Some(Box::new(|| println!("Cleanup!")))
/// }, (count.get(),));
/// ```
pub fn use_effect<F, D>(effect: F, deps: D)
where
    F: FnOnce() -> Option<Box<dyn FnOnce()>> + 'static,
    D: Deps + 'static,
{
    let ctx = current_context().expect("use_effect must be called within a component");
    let mut ctx_ref = ctx.borrow_mut();

    let new_deps_hash = deps.to_hash();

    // Get or create effect storage
    let storage = ctx_ref.use_hook(|| EffectStorage {
        prev_deps_hash: None,
    });

    let prev_deps_hash = storage.get::<EffectStorage>()
        .and_then(|s| s.prev_deps_hash);

    // Check if deps changed
    let should_run = match prev_deps_hash {
        None => true,  // First render
        Some(prev) => prev != new_deps_hash,  // Deps changed
    };

    if should_run {
        // Update stored deps
        storage.set(EffectStorage {
            prev_deps_hash: Some(new_deps_hash),
        });

        // Add effect to run after render
        ctx_ref.add_effect(Effect {
            callback: Box::new(effect),
            cleanup: None,
            deps: Some(vec![new_deps_hash]),
        });
    }
}

/// Run a side effect only once on mount
///
/// # Example
///
/// ```ignore
/// use_effect_once(|| {
///     println!("Component mounted!");
///     Some(Box::new(|| println!("Component unmounted!")))
/// });
/// ```
pub fn use_effect_once<F>(effect: F)
where
    F: FnOnce() -> Option<Box<dyn FnOnce()>> + 'static,
{
    let ctx = current_context().expect("use_effect_once must be called within a component");
    let mut ctx_ref = ctx.borrow_mut();

    // Use a flag to track if effect has run
    let storage = ctx_ref.use_hook(|| false);

    let has_run = storage.get::<bool>().unwrap_or(false);

    if !has_run {
        storage.set(true);

        ctx_ref.add_effect(Effect {
            callback: Box::new(effect),
            cleanup: None,
            deps: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_deps_hash() {
        let deps1 = (1i32, 2i32);
        let deps2 = (1i32, 2i32);
        let deps3 = (1i32, 3i32);

        assert_eq!(deps1.to_hash(), deps2.to_hash());
        assert_ne!(deps1.to_hash(), deps3.to_hash());
    }

    #[test]
    fn test_use_effect_runs() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let effect_ran = Rc::new(RefCell::new(false));

        let effect_ran_clone = effect_ran.clone();
        with_hooks(ctx.clone(), || {
            use_effect(move || {
                *effect_ran_clone.borrow_mut() = true;
                None
            }, ());
        });

        assert!(*effect_ran.borrow());
    }

    #[test]
    fn test_use_effect_with_deps() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let run_count = Rc::new(RefCell::new(0));

        // First render with deps = 1
        let run_count_clone = run_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(move || {
                *run_count_clone.borrow_mut() += 1;
                None
            }, (1i32,));
        });
        assert_eq!(*run_count.borrow(), 1);

        // Second render with same deps = 1 (should not run)
        let run_count_clone = run_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(move || {
                *run_count_clone.borrow_mut() += 1;
                None
            }, (1i32,));
        });
        assert_eq!(*run_count.borrow(), 1);  // Still 1

        // Third render with different deps = 2 (should run)
        let run_count_clone = run_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(move || {
                *run_count_clone.borrow_mut() += 1;
                None
            }, (2i32,));
        });
        assert_eq!(*run_count.borrow(), 2);
    }

    #[test]
    fn test_use_effect_cleanup() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let cleanup_ran = Rc::new(RefCell::new(false));

        // First render - effect with cleanup
        let cleanup_ran_clone = cleanup_ran.clone();
        with_hooks(ctx.clone(), || {
            use_effect(move || {
                Some(Box::new(move || {
                    *cleanup_ran_clone.borrow_mut() = true;
                }) as Box<dyn FnOnce()>)
            }, (1i32,));
        });

        // Cleanup hasn't run yet
        assert!(!*cleanup_ran.borrow());

        // Second render with different deps - should trigger cleanup
        with_hooks(ctx.clone(), || {
            use_effect(|| None, (2i32,));
        });

        // Now cleanup should have run
        assert!(*cleanup_ran.borrow());
    }
}
