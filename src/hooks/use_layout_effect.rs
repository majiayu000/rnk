//! use_layout_effect hook.
//!
//! Layout effects run before passive `use_effect` callbacks within the same
//! render lifecycle.

use crate::hooks::context::{Effect, current_context};
use crate::hooks::deps::DepsHash;
use std::any::TypeId;

#[derive(Clone)]
struct LayoutEffectStorage {
    prev_deps_hash: Option<u64>,
}

/// Run an effect when dependencies change.
pub fn use_layout_effect<F, D>(effect: F, deps: D)
where
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send + 'static,
    D: DepsHash + 'static,
{
    let Some(ctx) = current_context() else {
        if let Some(cleanup) = effect() {
            cleanup();
        }
        return;
    };
    let Ok(mut ctx_ref) = ctx.try_borrow_mut() else {
        if let Some(cleanup) = effect() {
            cleanup();
        }
        return;
    };

    let new_deps_hash = deps.deps_hash();
    let (storage, effect_slot) = ctx_ref.use_hook_with_index(|| LayoutEffectStorage {
        prev_deps_hash: None,
    });

    let prev_deps_hash = storage
        .get::<LayoutEffectStorage>()
        .and_then(|s| s.prev_deps_hash);

    let always_run = TypeId::of::<D>() == TypeId::of::<()>();
    let should_run = if always_run {
        true
    } else {
        match prev_deps_hash {
            None => true,
            Some(prev) => prev != new_deps_hash,
        }
    };

    if should_run {
        storage.set(LayoutEffectStorage {
            prev_deps_hash: Some(new_deps_hash),
        });

        ctx_ref.add_layout_effect(Effect {
            callback: Box::new(effect),
            slot: effect_slot,
        });
    }
}

/// Run an effect once on the first render.
pub fn use_layout_effect_once<F>(effect: F)
where
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send + 'static,
{
    let Some(ctx) = current_context() else {
        if let Some(cleanup) = effect() {
            cleanup();
        }
        return;
    };
    let Ok(mut ctx_ref) = ctx.try_borrow_mut() else {
        if let Some(cleanup) = effect() {
            cleanup();
        }
        return;
    };

    let (storage, effect_slot) = ctx_ref.use_hook_with_index(|| false);
    let has_run = storage.get::<bool>().unwrap_or(false);

    if !has_run {
        storage.set(true);

        ctx_ref.add_layout_effect(Effect {
            callback: Box::new(effect),
            slot: effect_slot,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use crate::hooks::use_effect::use_effect;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_use_layout_effect_runs_after_render() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        with_hooks(ctx.clone(), || {
            use_layout_effect(
                move || {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    None
                },
                1usize,
            );
        });

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_use_layout_effect_deps_gate_rerun() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let calls = Arc::new(AtomicUsize::new(0));

        {
            let calls_clone = calls.clone();
            with_hooks(ctx.clone(), || {
                use_layout_effect(
                    move || {
                        calls_clone.fetch_add(1, Ordering::SeqCst);
                        None
                    },
                    "same",
                );
            });
        }

        {
            let calls_clone = calls.clone();
            with_hooks(ctx.clone(), || {
                use_layout_effect(
                    move || {
                        calls_clone.fetch_add(1, Ordering::SeqCst);
                        None
                    },
                    "same",
                );
            });
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_use_layout_effect_once_runs_once() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let calls = Arc::new(AtomicUsize::new(0));

        {
            let calls_clone = calls.clone();
            with_hooks(ctx.clone(), || {
                use_layout_effect_once(move || {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    None
                });
            });
        }

        {
            let calls_clone = calls.clone();
            with_hooks(ctx.clone(), || {
                use_layout_effect_once(move || {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    None
                });
            });
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_layout_effect_runs_before_passive_effect() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let calls: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));

        let layout_calls = calls.clone();
        let effect_calls = calls.clone();

        with_hooks(ctx, || {
            use_layout_effect(
                move || {
                    layout_calls.lock().unwrap().push("layout");
                    None
                },
                (),
            );

            use_effect(
                move || {
                    effect_calls.lock().unwrap().push("effect");
                    None
                },
                (),
            );
        });

        let observed = calls.lock().unwrap().clone();
        assert_eq!(observed, vec!["layout", "effect"]);
    }
}
