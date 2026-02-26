//! use_layout_effect hook.
//!
//! In the current renderer lifecycle, layout effects share the same scheduling
//! phase as `use_effect` (post-render). This API is provided for ergonomics
//! and future lifecycle differentiation.

use crate::hooks::deps::DepsHash;
use crate::hooks::use_effect::{use_effect, use_effect_once};

/// Run an effect when dependencies change.
///
/// Today this is equivalent to `use_effect`.
pub fn use_layout_effect<F, D>(effect: F, deps: D)
where
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send + 'static,
    D: DepsHash + 'static,
{
    use_effect(effect, deps);
}

/// Run an effect once on the first render.
///
/// Today this is equivalent to `use_effect_once`.
pub fn use_layout_effect_once<F>(effect: F)
where
    F: FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send + 'static,
{
    use_effect_once(effect);
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
}
