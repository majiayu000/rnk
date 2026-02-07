//! Effect hook for side effects

use crate::hooks::context::{Effect, current_context};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send + 'static,
    D: Deps + 'static,
{
    let ctx = current_context().expect("use_effect must be called within a component");
    let mut ctx_ref = ctx.write().unwrap();

    let new_deps_hash = deps.to_hash();

    // Get or create effect storage
    let (storage, effect_slot) = ctx_ref.use_hook_with_index(|| EffectStorage {
        prev_deps_hash: None,
    });

    let prev_deps_hash = storage
        .get::<EffectStorage>()
        .and_then(|s| s.prev_deps_hash);

    // Check if deps changed
    let should_run = match prev_deps_hash {
        None => true,                        // First render
        Some(prev) => prev != new_deps_hash, // Deps changed
    };

    if should_run {
        // Update stored deps
        storage.set(EffectStorage {
            prev_deps_hash: Some(new_deps_hash),
        });

        // Add effect to run after render
        ctx_ref.add_effect(Effect {
            callback: Box::new(effect),
            slot: effect_slot,
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
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send + 'static,
{
    let ctx = current_context().expect("use_effect_once must be called within a component");
    let mut ctx_ref = ctx.write().unwrap();

    // Use a flag to track if effect has run
    let (storage, effect_slot) = ctx_ref.use_hook_with_index(|| false);

    let has_run = storage.get::<bool>().unwrap_or(false);

    if !has_run {
        storage.set(true);

        ctx_ref.add_effect(Effect {
            callback: Box::new(effect),
            slot: effect_slot,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::sync::{Arc, Mutex, RwLock};

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
        let ctx = Arc::new(RwLock::new(HookContext::new()));
        let effect_ran = Arc::new(Mutex::new(false));

        let effect_ran_clone = effect_ran.clone();
        with_hooks(ctx.clone(), || {
            use_effect(
                move || {
                    *effect_ran_clone.lock().unwrap() = true;
                    None
                },
                (),
            );
        });

        assert!(*effect_ran.lock().unwrap());
    }

    #[test]
    fn test_use_effect_with_deps() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));
        let run_count = Arc::new(Mutex::new(0));

        // First render with deps = 1
        let run_count_clone = run_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(
                move || {
                    *run_count_clone.lock().unwrap() += 1;
                    None
                },
                (1i32,),
            );
        });
        assert_eq!(*run_count.lock().unwrap(), 1);

        // Second render with same deps = 1 (should not run)
        let run_count_clone = run_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(
                move || {
                    *run_count_clone.lock().unwrap() += 1;
                    None
                },
                (1i32,),
            );
        });
        assert_eq!(*run_count.lock().unwrap(), 1); // Still 1

        // Third render with different deps = 2 (should run)
        let run_count_clone = run_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(
                move || {
                    *run_count_clone.lock().unwrap() += 1;
                    None
                },
                (2i32,),
            );
        });
        assert_eq!(*run_count.lock().unwrap(), 2);
    }

    #[test]
    fn test_use_effect_cleanup() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));
        let cleanup_ran = Arc::new(Mutex::new(false));

        // First render - effect with cleanup
        let cleanup_ran_clone = cleanup_ran.clone();
        with_hooks(ctx.clone(), || {
            use_effect(
                move || {
                    Some(Box::new(move || {
                        *cleanup_ran_clone.lock().unwrap() = true;
                    }) as Box<dyn FnOnce() + Send>)
                },
                (1i32,),
            );
        });

        // Cleanup hasn't run yet
        assert!(!*cleanup_ran.lock().unwrap());

        // Second render with different deps - should trigger cleanup
        with_hooks(ctx.clone(), || {
            use_effect(|| None, (2i32,));
        });

        // Now cleanup should have run
        assert!(*cleanup_ran.lock().unwrap());
    }

    #[test]
    fn test_use_effect_does_not_cleanup_when_deps_unchanged() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));
        let cleanup_count = Arc::new(Mutex::new(0usize));

        let cleanup_count_clone = cleanup_count.clone();
        with_hooks(ctx.clone(), || {
            use_effect(
                move || {
                    let cleanup_count_inner = cleanup_count_clone.clone();
                    Some(Box::new(move || {
                        *cleanup_count_inner.lock().unwrap() += 1;
                    }) as Box<dyn FnOnce() + Send>)
                },
                (1i32,),
            );
        });
        assert_eq!(*cleanup_count.lock().unwrap(), 0);

        // Same deps: effect should not rerun, cleanup should not run.
        with_hooks(ctx.clone(), || {
            use_effect(|| None, (1i32,));
        });
        assert_eq!(*cleanup_count.lock().unwrap(), 0);
    }

    #[test]
    fn test_use_effect_once_cleanup_runs_on_drop() {
        let cleanup_ran = Arc::new(Mutex::new(false));
        let ctx = Arc::new(RwLock::new(HookContext::new()));

        let cleanup_ran_clone = cleanup_ran.clone();
        with_hooks(ctx.clone(), || {
            use_effect_once(move || {
                let cleanup_ran_inner = cleanup_ran_clone.clone();
                Some(Box::new(move || {
                    *cleanup_ran_inner.lock().unwrap() = true;
                }) as Box<dyn FnOnce() + Send>)
            });
        });

        // Should not cleanup on normal rerender when use_effect_once doesn't rerun.
        with_hooks(ctx.clone(), || {
            use_effect_once(|| None);
        });
        assert!(!*cleanup_ran.lock().unwrap());

        drop(ctx);
        assert!(*cleanup_ran.lock().unwrap());
    }
}
