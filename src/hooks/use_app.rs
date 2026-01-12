//! App hook - provides access to app control functions
//!
//! This module provides the `use_app` hook for accessing application-level
//! functionality like exiting the app, switching display modes, and printing
//! persistent messages.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// App context that provides control over the application.
///
/// This is obtained via the `use_app()` hook and provides methods for:
/// - Exiting the application
/// - Switching between inline and fullscreen modes
/// - Printing persistent messages (in inline mode)
/// - Checking the current display mode
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn my_component() -> Element {
///     let app = use_app();
///
///     use_input(move |key| {
///         match key {
///             Key::Char('q') => app.exit(),
///             Key::Char(' ') => {
///                 if app.is_alt_screen() {
///                     app.exit_alt_screen();
///                 } else {
///                     app.enter_alt_screen();
///                 }
///             }
///             Key::Enter => app.println("Action completed!"),
///             _ => {}
///         }
///     });
///
///     // ... render UI ...
/// }
/// ```
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

    /// Print a message that persists above the UI.
    ///
    /// In inline mode, this clears the current UI, writes the message,
    /// and the UI is re-rendered below it. The message stays in terminal history.
    ///
    /// In fullscreen mode, this is a no-op (messages are ignored, like Bubbletea).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let app = use_app();
    /// app.println("Task completed!");
    /// app.println(format!("Processed {} items", count));
    ///
    /// // Print elements
    /// let banner = Box::new()
    ///     .child(Text::new("Welcome!").bold().into_element())
    ///     .into_element();
    /// app.println(banner);
    /// ```
    pub fn println(&self, message: impl crate::renderer::IntoPrintable) {
        crate::renderer::println(message);
    }

    /// Request to enter fullscreen mode (alternate screen).
    ///
    /// This switches from inline mode to fullscreen mode at runtime.
    /// Like Bubbletea's `EnterAltScreen` command.
    ///
    /// In fullscreen mode:
    /// - Uses alternate screen buffer
    /// - Content is cleared on exit
    /// - `println()` is a no-op
    pub fn enter_alt_screen(&self) {
        crate::renderer::enter_alt_screen();
    }

    /// Request to exit fullscreen mode (return to inline).
    ///
    /// This switches from fullscreen mode to inline mode at runtime.
    /// Like Bubbletea's `ExitAltScreen` command.
    ///
    /// In inline mode:
    /// - Output appears at current cursor position
    /// - Content persists in terminal history
    /// - `println()` works for persistent messages
    pub fn exit_alt_screen(&self) {
        crate::renderer::exit_alt_screen();
    }

    /// Check if currently in fullscreen mode (alternate screen).
    ///
    /// Returns `true` if in fullscreen mode, `false` if in inline mode.
    pub fn is_alt_screen(&self) -> bool {
        crate::renderer::is_alt_screen().unwrap_or(false)
    }

    /// Request a re-render.
    ///
    /// This is useful after updating shared state to ensure the UI reflects
    /// the new state.
    pub fn request_render(&self) {
        crate::renderer::request_render();
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
