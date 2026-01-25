//! use_cmd Hook - Execute commands based on dependency changes
//!
//! This hook allows components to execute side effects (commands) when
//! dependencies change, integrating the Hook system with the Command system.

use super::context::current_context;
use crate::cmd::Cmd;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Trait for types that can be used as hook dependencies
pub trait Deps {
    /// Output type returned when getting the dependency values
    type Output;

    /// Calculate hash of the dependencies
    fn deps_hash(&self) -> u64;

    /// Get the dependency values
    fn output(&self) -> Self::Output;
}

// Implement Deps for ()
impl Deps for () {
    type Output = ();

    fn deps_hash(&self) -> u64 {
        0
    }

    fn output(&self) -> Self::Output {}
}

// Implement Deps for tuples
impl<T1: Hash + Clone, T2: Hash + Clone> Deps for (T1, T2) {
    type Output = (T1, T2);

    fn deps_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        self.1.hash(&mut hasher);
        hasher.finish()
    }

    fn output(&self) -> Self::Output {
        (self.0.clone(), self.1.clone())
    }
}

impl<T1: Hash + Clone, T2: Hash + Clone, T3: Hash + Clone> Deps for (T1, T2, T3) {
    type Output = (T1, T2, T3);

    fn deps_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        self.1.hash(&mut hasher);
        self.2.hash(&mut hasher);
        hasher.finish()
    }

    fn output(&self) -> Self::Output {
        (self.0.clone(), self.1.clone(), self.2.clone())
    }
}

impl<T1: Hash + Clone, T2: Hash + Clone, T3: Hash + Clone, T4: Hash + Clone> Deps
    for (T1, T2, T3, T4)
{
    type Output = (T1, T2, T3, T4);

    fn deps_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        self.1.hash(&mut hasher);
        self.2.hash(&mut hasher);
        self.3.hash(&mut hasher);
        hasher.finish()
    }

    fn output(&self) -> Self::Output {
        (
            self.0.clone(),
            self.1.clone(),
            self.2.clone(),
            self.3.clone(),
        )
    }
}

// Implement Deps for Vec
impl<T: Hash + Clone> Deps for Vec<T> {
    type Output = Vec<T>;

    fn deps_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for item in self {
            item.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn output(&self) -> Self::Output {
        self.clone()
    }
}

// Macro to implement Deps for common types
macro_rules! impl_deps_for_type {
    ($($t:ty),*) => {
        $(
            impl Deps for $t {
                type Output = $t;

                fn deps_hash(&self) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    self.hash(&mut hasher);
                    hasher.finish()
                }

                fn output(&self) -> Self::Output {
                    self.clone()
                }
            }
        )*
    };
}

// Implement for common types
impl_deps_for_type!(i8, i16, i32, i64, i128, isize);
impl_deps_for_type!(u8, u16, u32, u64, u128, usize);
impl_deps_for_type!(bool, char);
impl_deps_for_type!(String);

/// Internal state for use_cmd hook
#[derive(Clone)]
struct CmdHookState {
    deps_hash: u64,
    is_first_render: bool,
}

/// Execute a command when dependencies change
///
/// This hook monitors dependencies and executes a command-returning function
/// whenever the dependencies change. The command is queued and will be
/// executed after the render completes.
///
/// # Example
///
/// ```rust,ignore
/// use rnk::hooks::use_cmd;
/// use rnk::hooks::use_signal;
/// use rnk::cmd::Cmd;
/// use std::time::Duration;
///
/// fn my_component() -> Element {
///     let count = use_signal(|| 0);
///
///     // Execute command when count changes
///     use_cmd(count.get(), |val| {
///         if val > 10 {
///             Cmd::perform(|| async {
///                 println!("Count exceeded 10!");
///             })
///         } else {
///             Cmd::none()
///         }
///     });
///
///     // ... render UI
/// }
/// ```
///
/// # Panics
///
/// Panics if called outside of a hook context (i.e., not during render).
pub fn use_cmd<D, F>(deps: D, f: F)
where
    D: Deps + 'static,
    F: FnOnce(D::Output) -> Cmd + 'static,
{
    let ctx = current_context().expect("use_cmd must be called within a component render");

    let new_hash = deps.deps_hash();

    // Get or create hook state
    let hook = ctx.borrow_mut().use_hook(|| CmdHookState {
        deps_hash: 0,
        is_first_render: true,
    });

    let mut state = hook.get::<CmdHookState>().unwrap();
    let old_hash = state.deps_hash;
    let is_first = state.is_first_render;

    // Check if dependencies changed OR if it's the first render
    if is_first || old_hash != new_hash {
        // Update stored hash and mark as no longer first render
        state.deps_hash = new_hash;
        state.is_first_render = false;
        hook.set(state);

        // Execute function to get command
        let cmd = f(deps.output());

        // Queue command for execution
        ctx.borrow_mut().queue_cmd(cmd);
    }
}

/// Execute a command only once on first render
///
/// This is a convenience wrapper around `use_cmd` with empty dependencies.
///
/// # Example
///
/// ```rust,ignore
/// use rnk::hooks::use_cmd_once;
/// use rnk::cmd::Cmd;
///
/// fn my_component() -> Element {
///     use_cmd_once(|| {
///         Cmd::perform(|| async {
///             println!("Component mounted!");
///         })
///     });
///
///     // ... render UI
/// }
/// ```
pub fn use_cmd_once<F>(f: F)
where
    F: FnOnce(()) -> Cmd + 'static,
{
    use_cmd((), f);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_deps_unit() {
        let deps = ();
        assert_eq!(deps.deps_hash(), 0);
    }

    #[test]
    fn test_deps_single_value() {
        let deps = 42i32;
        let hash1 = deps.deps_hash();
        let hash2 = 42i32.deps_hash();
        assert_eq!(hash1, hash2);

        let hash3 = 43i32.deps_hash();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_deps_tuple() {
        let deps = (1, 2);
        let hash1 = deps.deps_hash();
        let hash2 = (1, 2).deps_hash();
        assert_eq!(hash1, hash2);

        let hash3 = (1, 3).deps_hash();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_deps_vec() {
        let deps = vec![1, 2, 3];
        let hash1 = deps.deps_hash();
        let hash2 = vec![1, 2, 3].deps_hash();
        assert_eq!(hash1, hash2);

        let hash3 = vec![1, 2, 4].deps_hash();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_use_cmd_executes_on_first_render() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let cmd_executed = Rc::new(RefCell::new(false));

        {
            let flag = Rc::clone(&cmd_executed);
            with_hooks(ctx.clone(), move || {
                use_cmd((), move |_| {
                    *flag.borrow_mut() = true;
                    Cmd::none()
                });
            });
        }

        // The command function was executed
        assert!(*cmd_executed.borrow());
        // And a command was queued
        assert_eq!(ctx.borrow_mut().take_cmds().len(), 1);
    }

    #[test]
    fn test_use_cmd_executes_on_deps_change() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let execution_count = Rc::new(RefCell::new(0));

        // First render
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd(1, move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 1);
        ctx.borrow_mut().take_cmds(); // Clear commands

        // Second render - same deps
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd(1, move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 1); // Should not execute again

        // Third render - different deps
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd(2, move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 2); // Should execute again
    }

    #[test]
    fn test_use_cmd_receives_correct_value() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let received_value = Rc::new(RefCell::new(0));

        {
            let value = Rc::clone(&received_value);
            with_hooks(ctx.clone(), move || {
                use_cmd(42, move |val| {
                    *value.borrow_mut() = val;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*received_value.borrow(), 42);
    }

    #[test]
    fn test_use_cmd_queues_command() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        with_hooks(ctx.clone(), || {
            use_cmd((), |_| Cmd::perform(|| async {}));
        });

        let cmds = ctx.borrow_mut().take_cmds();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], Cmd::Perform { .. }));
    }

    #[test]
    fn test_use_cmd_once() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let execution_count = Rc::new(RefCell::new(0));

        // First render
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd_once(move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 1);
        ctx.borrow_mut().take_cmds();

        // Second render - should not execute
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd_once(move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 1); // Still 1
    }

    #[test]
    fn test_use_cmd_with_tuple_deps() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));
        let execution_count = Rc::new(RefCell::new(0));

        // First render
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd((1, 2), move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 1);
        ctx.borrow_mut().take_cmds();

        // Same deps
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd((1, 2), move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 1);

        // Different deps
        {
            let count = Rc::clone(&execution_count);
            with_hooks(ctx.clone(), move || {
                use_cmd((1, 3), move |_| {
                    *count.borrow_mut() += 1;
                    Cmd::none()
                });
            });
        }

        assert_eq!(*execution_count.borrow(), 2);
    }

    #[test]
    fn test_use_cmd_multiple_in_same_render() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        with_hooks(ctx.clone(), || {
            use_cmd(1, |_| Cmd::perform(|| async {}));
            use_cmd(2, |_| Cmd::sleep(std::time::Duration::from_secs(1)));
            use_cmd(3, |_| Cmd::none());
        });

        let cmds = ctx.borrow_mut().take_cmds();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    #[should_panic(expected = "use_cmd must be called within a component render")]
    fn test_use_cmd_panics_outside_context() {
        use_cmd((), |_| Cmd::none());
    }
}
