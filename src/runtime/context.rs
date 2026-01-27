//! Unified runtime context for rnk applications
//!
//! This module provides a centralized context that holds all runtime state,
//! replacing scattered global/thread-local state with explicit context passing.
//!
//! # Architecture
//!
//! The `RuntimeContext` is the single source of truth for:
//! - Hook state (HookContext)
//! - Input handlers
//! - Mouse handlers
//! - Focus management
//! - App control (exit, render requests)
//! - Accessibility state
//!
//! This design enables:
//! - Multiple concurrent apps (each with its own context)
//! - Better testability (no global state pollution)
//! - Clearer ownership and lifecycle

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cmd::Cmd;
use crate::hooks::context::{HookContext, HookStorage};
use crate::hooks::use_focus::FocusManager;
use crate::hooks::use_input::Key;
use crate::hooks::use_mouse::Mouse;
use crate::renderer::{IntoPrintable, RenderHandle};

/// Input handler function type
pub type InputHandlerFn = Rc<dyn Fn(&str, &Key)>;

/// Mouse handler function type
pub type MouseHandlerFn = Rc<dyn Fn(&Mouse)>;

/// Unified runtime context for an rnk application
///
/// This context holds all state needed during rendering and event handling.
/// It replaces the previous scattered thread-local and global state.
pub struct RuntimeContext {
    /// Hook state for the component tree
    hook_context: HookContext,

    /// Input handlers registered via use_input
    input_handlers: Vec<InputHandlerFn>,

    /// Mouse handlers registered via use_mouse
    mouse_handlers: Vec<MouseHandlerFn>,

    /// Whether mouse mode is enabled
    mouse_enabled: bool,

    /// Focus manager for Tab navigation
    focus_manager: FocusManager,

    /// Exit flag for the application
    exit_flag: Arc<AtomicBool>,

    /// Render handle for cross-thread communication
    render_handle: Option<RenderHandle>,

    /// Whether screen reader mode is enabled
    screen_reader_enabled: bool,

    /// Measured element dimensions (element_id -> (width, height))
    measurements: std::collections::HashMap<u64, (u16, u16)>,
}

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new() -> Self {
        Self {
            hook_context: HookContext::new(),
            input_handlers: Vec::new(),
            mouse_handlers: Vec::new(),
            mouse_enabled: false,
            focus_manager: FocusManager::new(),
            exit_flag: Arc::new(AtomicBool::new(false)),
            render_handle: None,
            screen_reader_enabled: false,
            measurements: std::collections::HashMap::new(),
        }
    }

    /// Create a runtime context with app control
    pub fn with_app_control(exit_flag: Arc<AtomicBool>, render_handle: RenderHandle) -> Self {
        Self {
            hook_context: HookContext::new(),
            input_handlers: Vec::new(),
            mouse_handlers: Vec::new(),
            mouse_enabled: false,
            focus_manager: FocusManager::new(),
            exit_flag,
            render_handle: Some(render_handle),
            screen_reader_enabled: false,
            measurements: std::collections::HashMap::new(),
        }
    }

    // === Hook Context Methods ===

    /// Begin a render cycle
    pub fn begin_render(&mut self) {
        self.hook_context.begin_render();
        // Clear handlers for fresh registration
        self.input_handlers.clear();
        self.mouse_handlers.clear();
        self.mouse_enabled = false;
    }

    /// End a render cycle
    pub fn end_render(&mut self) {
        self.hook_context.end_render();
    }

    /// Run effects after render
    pub fn run_effects(&mut self) {
        self.hook_context.run_effects();
    }

    /// Get or create a hook at the current index
    pub fn use_hook<T: Clone + 'static, F: FnOnce() -> T>(&mut self, init: F) -> HookStorage {
        self.hook_context.use_hook(init)
    }

    /// Queue a command to execute after render
    pub fn queue_cmd(&mut self, cmd: Cmd) {
        self.hook_context.queue_cmd(cmd);
    }

    /// Take all queued commands
    pub fn take_cmds(&mut self) -> Vec<Cmd> {
        self.hook_context.take_cmds()
    }

    /// Set the render callback for hooks
    pub fn set_render_callback(&mut self, callback: crate::hooks::context::RenderCallback) {
        self.hook_context.set_render_callback(callback);
    }

    /// Request a re-render
    pub fn request_render(&self) {
        self.hook_context.request_render();
        if let Some(handle) = &self.render_handle {
            handle.request_render();
        }
    }

    // === Input Handler Methods ===

    /// Register an input handler
    pub fn register_input_handler<F>(&mut self, handler: F)
    where
        F: Fn(&str, &Key) + 'static,
    {
        self.input_handlers.push(Rc::new(handler));
    }

    /// Dispatch input to all handlers
    pub fn dispatch_input(&self, input: &str, key: &Key) {
        for handler in &self.input_handlers {
            handler(input, key);
        }
    }

    /// Get the number of registered input handlers
    pub fn input_handler_count(&self) -> usize {
        self.input_handlers.len()
    }

    // === Mouse Handler Methods ===

    /// Register a mouse handler
    pub fn register_mouse_handler<F>(&mut self, handler: F)
    where
        F: Fn(&Mouse) + 'static,
    {
        self.mouse_handlers.push(Rc::new(handler));
        self.mouse_enabled = true;
    }

    /// Dispatch mouse event to all handlers
    pub fn dispatch_mouse(&self, mouse: &Mouse) {
        for handler in &self.mouse_handlers {
            handler(mouse);
        }
    }

    /// Check if mouse mode is enabled
    pub fn is_mouse_enabled(&self) -> bool {
        self.mouse_enabled
    }

    /// Set mouse enabled state
    pub fn set_mouse_enabled(&mut self, enabled: bool) {
        self.mouse_enabled = enabled;
    }

    // === Focus Manager Methods ===

    /// Get mutable access to the focus manager
    pub fn focus_manager_mut(&mut self) -> &mut FocusManager {
        &mut self.focus_manager
    }

    /// Get read access to the focus manager
    pub fn focus_manager(&self) -> &FocusManager {
        &self.focus_manager
    }

    // === App Control Methods ===

    /// Get the exit flag
    pub fn exit_flag(&self) -> Arc<AtomicBool> {
        self.exit_flag.clone()
    }

    /// Request app exit
    pub fn exit(&self) {
        self.exit_flag.store(true, Ordering::SeqCst);
    }

    /// Check if exit was requested
    pub fn should_exit(&self) -> bool {
        self.exit_flag.load(Ordering::SeqCst)
    }

    /// Get the render handle
    pub fn render_handle(&self) -> Option<&RenderHandle> {
        self.render_handle.as_ref()
    }

    /// Print a message (delegates to render handle)
    pub fn println(&self, message: impl IntoPrintable) {
        if let Some(handle) = &self.render_handle {
            handle.println(message);
        }
    }

    /// Enter alternate screen mode
    pub fn enter_alt_screen(&self) {
        if let Some(handle) = &self.render_handle {
            handle.enter_alt_screen();
        }
    }

    /// Exit alternate screen mode
    pub fn exit_alt_screen(&self) {
        if let Some(handle) = &self.render_handle {
            handle.exit_alt_screen();
        }
    }

    /// Check if in alternate screen mode
    pub fn is_alt_screen(&self) -> bool {
        self.render_handle
            .as_ref()
            .map(|h| h.is_alt_screen())
            .unwrap_or(false)
    }

    // === Accessibility Methods ===

    /// Check if screen reader mode is enabled
    pub fn is_screen_reader_enabled(&self) -> bool {
        self.screen_reader_enabled
    }

    /// Set screen reader mode
    pub fn set_screen_reader_enabled(&mut self, enabled: bool) {
        self.screen_reader_enabled = enabled;
    }

    // === Measurement Methods ===

    /// Store a measurement for an element
    pub fn set_measurement(&mut self, element_id: u64, width: u16, height: u16) {
        self.measurements.insert(element_id, (width, height));
    }

    /// Get a measurement for an element
    pub fn get_measurement(&self, element_id: u64) -> Option<(u16, u16)> {
        self.measurements.get(&element_id).copied()
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

// === Thread-local Context Access ===

thread_local! {
    static CURRENT_RUNTIME: RefCell<Option<Rc<RefCell<RuntimeContext>>>> = const { RefCell::new(None) };
}

/// Get the current runtime context
pub fn current_runtime() -> Option<Rc<RefCell<RuntimeContext>>> {
    CURRENT_RUNTIME.with(|ctx| ctx.borrow().clone())
}

/// Set the current runtime context
pub fn set_current_runtime(ctx: Option<Rc<RefCell<RuntimeContext>>>) {
    CURRENT_RUNTIME.with(|current| {
        *current.borrow_mut() = ctx;
    });
}

/// Run a function with a runtime context
pub fn with_runtime<F, R>(ctx: Rc<RefCell<RuntimeContext>>, f: F) -> R
where
    F: FnOnce() -> R,
{
    // Set the current context
    set_current_runtime(Some(ctx.clone()));

    // Begin render
    ctx.borrow_mut().begin_render();

    // Run the function
    let result = f();

    // End render
    ctx.borrow_mut().end_render();

    // Run effects
    ctx.borrow_mut().run_effects();

    // Clear the current context
    set_current_runtime(None);

    result
}

/// Execute a function with access to the current runtime context
///
/// This is a convenience function for hooks that need to access the context.
pub fn with_current_runtime<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut RuntimeContext) -> R,
{
    current_runtime().map(|ctx| f(&mut ctx.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_context_creation() {
        let ctx = RuntimeContext::new();
        assert!(!ctx.should_exit());
        assert!(!ctx.is_mouse_enabled());
        assert!(!ctx.is_screen_reader_enabled());
    }

    #[test]
    fn test_runtime_context_exit() {
        let ctx = RuntimeContext::new();
        assert!(!ctx.should_exit());
        ctx.exit();
        assert!(ctx.should_exit());
    }

    #[test]
    fn test_runtime_context_input_handlers() {
        let mut ctx = RuntimeContext::new();
        assert_eq!(ctx.input_handler_count(), 0);

        ctx.register_input_handler(|_, _| {});
        assert_eq!(ctx.input_handler_count(), 1);

        ctx.register_input_handler(|_, _| {});
        assert_eq!(ctx.input_handler_count(), 2);
    }

    #[test]
    fn test_runtime_context_mouse_enabled() {
        let mut ctx = RuntimeContext::new();
        assert!(!ctx.is_mouse_enabled());

        ctx.register_mouse_handler(|_| {});
        assert!(ctx.is_mouse_enabled());
    }

    #[test]
    fn test_runtime_context_measurements() {
        let mut ctx = RuntimeContext::new();
        assert!(ctx.get_measurement(1).is_none());

        ctx.set_measurement(1, 80, 24);
        assert_eq!(ctx.get_measurement(1), Some((80, 24)));
    }

    #[test]
    fn test_with_runtime() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

        let result = with_runtime(ctx.clone(), || {
            let runtime = current_runtime().unwrap();
            runtime.borrow_mut().register_input_handler(|_, _| {});
            runtime.borrow().input_handler_count()
        });

        assert_eq!(result, 1);

        // Context should be cleared after with_runtime
        assert!(current_runtime().is_none());
    }

    #[test]
    fn test_hook_state_persistence() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

        // First render
        with_runtime(ctx.clone(), || {
            let runtime = current_runtime().unwrap();
            let hook = runtime.borrow_mut().use_hook(|| 42i32);
            assert_eq!(hook.get::<i32>(), Some(42));
            hook.set(100i32);
        });

        // Second render - hook state should persist
        with_runtime(ctx.clone(), || {
            let runtime = current_runtime().unwrap();
            let hook = runtime.borrow_mut().use_hook(|| 0i32); // init ignored
            assert_eq!(hook.get::<i32>(), Some(100));
        });
    }

    #[test]
    fn test_handlers_cleared_on_render() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

        // First render - register handlers
        with_runtime(ctx.clone(), || {
            let runtime = current_runtime().unwrap();
            runtime.borrow_mut().register_input_handler(|_, _| {});
            runtime.borrow_mut().register_input_handler(|_, _| {});
            assert_eq!(runtime.borrow().input_handler_count(), 2);
        });

        // Second render - handlers should be cleared and re-registered
        with_runtime(ctx.clone(), || {
            let runtime = current_runtime().unwrap();
            // Handlers were cleared at begin_render
            assert_eq!(runtime.borrow().input_handler_count(), 0);
            runtime.borrow_mut().register_input_handler(|_, _| {});
            assert_eq!(runtime.borrow().input_handler_count(), 1);
        });
    }
}
