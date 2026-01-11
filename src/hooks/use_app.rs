//! App hook - provides access to app control functions

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// App context that provides control over the application
#[derive(Clone)]
pub struct AppContext {
    exit_flag: Arc<AtomicBool>,
}

impl AppContext {
    /// Create a new app context
    pub fn new(exit_flag: Arc<AtomicBool>) -> Self {
        Self { exit_flag }
    }

    /// Exit the application
    pub fn exit(&self) {
        self.exit_flag.store(true, Ordering::SeqCst);
    }
}

// Thread-local storage for the current app context
thread_local! {
    static APP_CONTEXT: std::cell::RefCell<Option<AppContext>> = const { std::cell::RefCell::new(None) };
}

/// Set the current app context (called by App during render)
pub fn set_app_context(ctx: Option<AppContext>) {
    APP_CONTEXT.with(|c| {
        *c.borrow_mut() = ctx;
    });
}

/// Get the current app context
pub fn get_app_context() -> Option<AppContext> {
    APP_CONTEXT.with(|c| c.borrow().clone())
}

/// Hook to access app control functions
///
/// # Example
///
/// ```ignore
/// let app = use_app();
///
/// use_input(move |input, key| {
///     if input == "q" {
///         app.exit();
///     }
/// });
/// ```
pub fn use_app() -> AppContext {
    get_app_context().expect("use_app must be called within an App context")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_context_exit() {
        let exit_flag = Arc::new(AtomicBool::new(false));
        let ctx = AppContext::new(exit_flag.clone());

        assert!(!exit_flag.load(Ordering::SeqCst));
        ctx.exit();
        assert!(exit_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_set_get_app_context() {
        let exit_flag = Arc::new(AtomicBool::new(false));
        let ctx = AppContext::new(exit_flag.clone());

        set_app_context(Some(ctx));

        let retrieved = get_app_context();
        assert!(retrieved.is_some());

        retrieved.unwrap().exit();
        assert!(exit_flag.load(Ordering::SeqCst));

        // Clean up
        set_app_context(None);
    }
}
