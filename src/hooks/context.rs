//! Hook context management

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

// Import Cmd type for command queue
use crate::cmd::Cmd;

/// Callback type for triggering re-renders
pub type RenderCallback = Rc<dyn Fn()>;

/// Effect callback type that returns an optional cleanup function
pub type EffectCallback = Box<dyn FnOnce() -> Option<Box<dyn FnOnce()>>>;

/// Hook storage for a single hook
#[derive(Clone)]
pub struct HookStorage {
    pub value: Rc<RefCell<Box<dyn Any>>>,
}

impl HookStorage {
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(Box::new(value))),
        }
    }

    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.value.borrow().downcast_ref::<T>().cloned()
    }

    pub fn set<T: 'static>(&self, value: T) {
        *self.value.borrow_mut() = Box::new(value);
    }
}

/// Effect to be run after render
pub struct Effect {
    pub callback: EffectCallback,
    pub cleanup: Option<Box<dyn FnOnce()>>,
    pub deps: Option<Vec<u64>>, // Hash of dependencies
}

/// Hook context for a component
pub struct HookContext {
    /// Hook values storage
    hooks: Vec<HookStorage>,
    /// Current hook index during render
    hook_index: usize,
    /// Effects to run after render
    effects: Vec<Effect>,
    /// Cleanup functions from previous effects
    cleanups: Vec<Option<Box<dyn FnOnce()>>>,
    /// Callback to trigger re-render
    render_callback: Option<RenderCallback>,
    /// Flag indicating if context is being rendered
    is_rendering: bool,
    /// Commands to execute after render
    cmd_queue: Vec<Cmd>,
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
    }

    /// Get or create a hook at the current index
    pub fn use_hook<T: Clone + 'static, F: FnOnce() -> T>(&mut self, init: F) -> HookStorage {
        let index = self.hook_index;
        self.hook_index += 1;

        if index >= self.hooks.len() {
            // First render - create the hook
            let storage = HookStorage::new(init());
            self.hooks.push(storage.clone());
            storage
        } else {
            // Subsequent render - return existing hook
            self.hooks[index].clone()
        }
    }

    /// Add an effect to run after render
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// Run all pending effects
    pub fn run_effects(&mut self) {
        // Run cleanup functions from previous render
        for cleanup_fn in self.cleanups.drain(..).flatten() {
            cleanup_fn();
        }

        // Run new effects and collect cleanup functions
        let effects = std::mem::take(&mut self.effects);
        for effect in effects {
            let cleanup = (effect.callback)();
            self.cleanups.push(cleanup);
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
    // Set the current context
    CURRENT_CONTEXT.with(|current| {
        *current.borrow_mut() = Some(ctx.clone());
    });

    // Begin render
    ctx.borrow_mut().begin_render();

    // Run the function
    let result = f();

    // End render
    ctx.borrow_mut().end_render();

    // Run effects
    ctx.borrow_mut().run_effects();

    // Clear the current context
    CURRENT_CONTEXT.with(|current| {
        *current.borrow_mut() = None;
    });

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
}
