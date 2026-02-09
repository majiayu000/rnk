//! Hook context management

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Import Cmd type for command queue
use crate::cmd::Cmd;

/// Callback type for triggering re-renders (thread-safe)
pub type RenderCallback = Arc<dyn Fn() + Send + Sync>;

/// Effect callback type that returns an optional cleanup function
pub type EffectCallback = Box<dyn FnOnce() -> Option<Box<dyn FnOnce() + Send>> + Send>;

/// Hook storage for a single hook (thread-safe)
#[derive(Clone)]
pub struct HookStorage {
    pub value: Arc<RwLock<Box<dyn Any + Send + Sync>>>,
}

impl HookStorage {
    pub fn new<T: Send + Sync + 'static>(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(Box::new(value))),
        }
    }

    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.value.read().ok()?.downcast_ref::<T>().cloned()
    }

    pub fn set<T: Send + Sync + 'static>(&self, value: T) {
        if let Ok(mut guard) = self.value.write() {
            *guard = Box::new(value);
        }
    }
}

/// Effect to be run after render (thread-safe)
pub struct Effect {
    pub callback: EffectCallback,
    /// Hook slot index this effect belongs to
    pub slot: usize,
}

/// Hook context for a component (thread-safe)
pub struct HookContext {
    /// Hook values storage
    hooks: Vec<HookStorage>,
    /// Current hook index during render
    hook_index: usize,
    /// Effects to run after render
    effects: Vec<Effect>,
    /// Cleanup functions from previous effects
    cleanups: Vec<Option<Box<dyn FnOnce() + Send>>>,
    /// Callback to trigger re-render
    render_callback: Option<RenderCallback>,
    /// Flag indicating if context is being rendered
    is_rendering: bool,
    /// Commands to execute after render
    cmd_queue: Vec<Cmd>,
    /// Hook type IDs for order verification
    hook_types: Vec<std::any::TypeId>,
    /// Whether this is the first render (for hook order verification)
    first_render_complete: bool,
}

impl HookContext {
    /// Create a new hook context
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
            hook_index: 0,
            effects: Vec::new(),
            cleanups: Vec::new(),
            render_callback: None,
            is_rendering: false,
            cmd_queue: Vec::new(),
            hook_types: Vec::new(),
            first_render_complete: false,
        }
    }

    /// Set the render callback
    pub fn set_render_callback(&mut self, callback: RenderCallback) {
        self.render_callback = Some(callback);
    }

    /// Get the render callback
    pub fn get_render_callback(&self) -> Option<RenderCallback> {
        self.render_callback.clone()
    }

    /// Start a render cycle
    pub fn begin_render(&mut self) {
        self.hook_index = 0;
        self.effects.clear();
        self.is_rendering = true;
    }

    /// End a render cycle
    pub fn end_render(&mut self) {
        self.is_rendering = false;
        self.first_render_complete = true;
    }

    /// Get or create a hook at the current index
    pub fn use_hook<T: Clone + Send + Sync + 'static, F: FnOnce() -> T>(
        &mut self,
        init: F,
    ) -> HookStorage {
        self.use_hook_with_index(init).0
    }

    /// Get or create a hook at the current index, returning both storage and slot index
    pub fn use_hook_with_index<T: Clone + Send + Sync + 'static, F: FnOnce() -> T>(
        &mut self,
        init: F,
    ) -> (HookStorage, usize) {
        let index = self.hook_index;
        self.hook_index += 1;

        let storage = if index >= self.hooks.len() {
            // First render - create the hook
            self.hook_types.push(std::any::TypeId::of::<T>());
            let storage = HookStorage::new(init());
            self.hooks.push(storage.clone());
            storage
        } else {
            // Subsequent render - verify hook type matches.
            if self.first_render_complete {
                let expected = self.hook_types[index];
                let actual = std::any::TypeId::of::<T>();
                if expected != actual {
                    panic!(
                        "Hook order violation at index {}! \
                        Hooks must be called in the same order on every render. \
                        This usually happens when hooks are called conditionally. \
                        Move conditional logic inside the hook or use separate components.",
                        index
                    );
                }
            }
            // Return existing hook
            self.hooks[index].clone()
        };

        (storage, index)
    }

    /// Add an effect to run after render
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// Run all pending effects
    pub fn run_effects(&mut self) {
        // Run effects and update cleanup functions only for the slots that re-ran.
        let effects = std::mem::take(&mut self.effects);
        for effect in effects {
            if effect.slot >= self.cleanups.len() {
                self.cleanups.resize_with(effect.slot + 1, || None);
            }

            if let Some(cleanup_fn) = self.cleanups[effect.slot].take() {
                cleanup_fn();
            }

            self.cleanups[effect.slot] = (effect.callback)();
        }
    }

    /// Request a re-render
    pub fn request_render(&self) {
        if let Some(callback) = &self.render_callback {
            callback();
        }
    }

    /// Queue a command to execute after render
    pub fn queue_cmd(&mut self, cmd: Cmd) {
        self.cmd_queue.push(cmd);
    }

    /// Take all queued commands
    pub fn take_cmds(&mut self) -> Vec<Cmd> {
        std::mem::take(&mut self.cmd_queue)
    }
}

impl Default for HookContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HookContext {
    fn drop(&mut self) {
        for cleanup_fn in self.cleanups.drain(..).flatten() {
            cleanup_fn();
        }
    }
}

// Thread-local storage for the current hook context
thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Rc<RefCell<HookContext>>>> = const { RefCell::new(None) };
}

/// Get the current hook context
pub fn current_context() -> Option<Rc<RefCell<HookContext>>> {
    CURRENT_CONTEXT.with(|ctx| ctx.borrow().clone())
}

/// Run a function with a hook context
pub fn with_hooks<F, R>(ctx: Rc<RefCell<HookContext>>, f: F) -> R
where
    F: FnOnce() -> R,
{
    // Save the previous context so nested calls work correctly
    let prev = current_context();

    // Set the current context
    CURRENT_CONTEXT.with(|current| {
        *current.borrow_mut() = Some(ctx.clone());
    });

    // Begin render
    ctx.borrow_mut().begin_render();

    // Use a guard to ensure context is restored on panic
    struct HooksGuard {
        prev: Option<Rc<RefCell<HookContext>>>,
    }
    impl Drop for HooksGuard {
        fn drop(&mut self) {
            CURRENT_CONTEXT.with(|current| {
                *current.borrow_mut() = self.prev.take();
            });
        }
    }
    let guard = HooksGuard { prev };

    // Run the function
    let result = f();

    // End render and run effects
    {
        let mut ctx_ref = ctx.borrow_mut();
        ctx_ref.end_render();
        ctx_ref.run_effects();
    }

    // Restore the previous context
    drop(guard);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_context_creation() {
        let ctx = HookContext::new();
        assert_eq!(ctx.hook_index, 0);
        assert!(ctx.hooks.is_empty());
    }

    #[test]
    fn test_use_hook() {
        let mut ctx = HookContext::new();
        ctx.begin_render();

        let hook1 = ctx.use_hook(|| 42i32);
        let hook2 = ctx.use_hook(|| "hello".to_string());

        assert_eq!(hook1.get::<i32>(), Some(42));
        assert_eq!(hook2.get::<String>(), Some("hello".to_string()));
        assert_eq!(ctx.hook_index, 2);
    }

    #[test]
    fn test_hook_persistence() {
        let mut ctx = HookContext::new();

        // First render
        ctx.begin_render();
        let hook = ctx.use_hook(|| 1i32);
        assert_eq!(hook.get::<i32>(), Some(1));
        hook.set(2i32);
        ctx.end_render();

        // Second render - should get same hook
        ctx.begin_render();
        let hook = ctx.use_hook(|| 999i32); // init should be ignored
        assert_eq!(hook.get::<i32>(), Some(2)); // should be 2, not 999
        ctx.end_render();
    }

    #[test]
    fn test_with_hooks() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let result = with_hooks(ctx.clone(), || {
            let ctx = current_context().unwrap();
            let hook = ctx.borrow_mut().use_hook(|| 42i32);
            hook.get::<i32>().unwrap()
        });

        assert_eq!(result, 42);
    }

    #[test]
    #[should_panic(expected = "Hook order violation")]
    fn test_hook_order_violation() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // First render - establish hook order
        with_hooks(ctx.clone(), || {
            let ctx = current_context().unwrap();
            let mut guard = ctx.borrow_mut();
            let _ = guard.use_hook(|| 42i32);
            let _ = guard.use_hook(|| "hello".to_string());
        });

        // Second render - violate hook order by using different types
        with_hooks(ctx.clone(), || {
            let ctx = current_context().unwrap();
            let mut guard = ctx.borrow_mut();
            // This should panic because we're using String where i32 was expected
            let _ = guard.use_hook(|| "wrong type".to_string());
        });
    }
}
