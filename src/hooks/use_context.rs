//! Context hooks for cross-component value sharing.

use crate::hooks::context::current_context;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

static CONTEXT_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static CONTEXT_VALUES: RefCell<HashMap<usize, Vec<Box<dyn Any>>>> = RefCell::new(HashMap::new());
}

/// A typed context container created by `create_context`.
#[derive(Clone)]
pub struct Context<T> {
    id: usize,
    default: T,
}

impl<T: Clone + Send + Sync + 'static> Context<T> {
    /// Get the unique context identifier.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the default value of this context.
    pub fn default_value(&self) -> T {
        self.default.clone()
    }

    /// Provide a value for this context while running `f`.
    pub fn with_provider<R>(&self, value: T, f: impl FnOnce() -> R) -> R {
        with_context(self, value, f)
    }
}

/// Create a context with a default value.
pub fn create_context<T: Clone + Send + Sync + 'static>(default: T) -> Context<T> {
    let id = CONTEXT_ID_COUNTER
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            current.checked_add(1)
        })
        .expect("Context ID counter overflow")
        + 1;
    Context { id, default }
}

/// Read the current value from a context, falling back to its default value.
pub fn use_context<T: Clone + Send + Sync + 'static>(context: &Context<T>) -> T {
    // Participate in hook ordering checks when a hook context is active.
    if let Some(hook_ctx) = current_context()
        && let Ok(mut hook_ctx_ref) = hook_ctx.try_borrow_mut()
    {
        hook_ctx_ref.use_hook(|| ());
    }

    CONTEXT_VALUES.with(|values| {
        values
            .borrow()
            .get(&context.id)
            .and_then(|stack| stack.last())
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .cloned()
            .unwrap_or_else(|| context.default.clone())
    })
}

/// Provide a context value for the duration of `f`.
pub fn with_context<T, R>(context: &Context<T>, value: T, f: impl FnOnce() -> R) -> R
where
    T: Clone + Send + Sync + 'static,
{
    CONTEXT_VALUES.with(|values| {
        values
            .borrow_mut()
            .entry(context.id)
            .or_default()
            .push(Box::new(value));
    });

    struct ProviderGuard {
        id: usize,
    }

    impl Drop for ProviderGuard {
        fn drop(&mut self) {
            CONTEXT_VALUES.with(|values| {
                let mut values = values.borrow_mut();
                if let Some(stack) = values.get_mut(&self.id) {
                    let _ = stack.pop();
                    if stack.is_empty() {
                        values.remove(&self.id);
                    }
                }
            });
        }
    }

    let _guard = ProviderGuard { id: context.id };
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use crate::hooks::use_signal::use_signal;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_use_context_returns_default_without_provider() {
        let name_ctx = create_context("guest".to_string());
        assert_eq!(use_context(&name_ctx), "guest");
    }

    #[test]
    fn test_use_context_reads_provided_value() {
        let theme_ctx = create_context("light".to_string());
        let value = with_context(&theme_ctx, "dark".to_string(), || use_context(&theme_ctx));
        assert_eq!(value, "dark");
    }

    #[test]
    fn test_with_context_nested_provider_restores_parent() {
        let language_ctx = create_context("en".to_string());
        let value = with_context(&language_ctx, "zh".to_string(), || {
            let outer = use_context(&language_ctx);
            let inner = with_context(&language_ctx, "ja".to_string(), || {
                use_context(&language_ctx)
            });
            let after_inner = use_context(&language_ctx);
            (outer, inner, after_inner)
        });
        assert_eq!(value.0, "zh");
        assert_eq!(value.1, "ja");
        assert_eq!(value.2, "zh");
    }

    #[test]
    fn test_contexts_are_isolated() {
        let a = create_context(1usize);
        let b = create_context(2usize);
        let value = with_context(&a, 10usize, || (use_context(&a), use_context(&b)));
        assert_eq!(value, (10, 2));
    }

    #[test]
    #[should_panic(expected = "Hook order violation")]
    fn test_use_context_participates_in_hook_order() {
        let hook_ctx = Rc::new(RefCell::new(HookContext::new()));
        let value_ctx = create_context(0i32);
        let mut use_ctx = true;

        let _ = with_hooks(hook_ctx.clone(), || {
            if use_ctx {
                let _ = use_context(&value_ctx);
            }
            use_signal(|| 1i32)
        });

        use_ctx = false;

        let _ = with_hooks(hook_ctx.clone(), || {
            if use_ctx {
                let _ = use_context(&value_ctx);
            }
            use_signal(|| 2i32)
        });
    }
}
