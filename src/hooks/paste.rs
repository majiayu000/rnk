//! Bracketed paste support
//!
//! Provides support for bracketed paste mode in terminals.
//! When enabled, pasted text is wrapped in escape sequences,
//! allowing the application to distinguish between typed and pasted input.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for bracketed paste mode
static BRACKETED_PASTE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if bracketed paste mode is currently enabled
pub fn is_bracketed_paste_enabled() -> bool {
    BRACKETED_PASTE_ENABLED.load(Ordering::SeqCst)
}

/// Enable bracketed paste mode
///
/// When enabled, pasted text will be wrapped in escape sequences:
/// - Start: ESC [200~
/// - End: ESC [201~
///
/// This allows the application to handle pasted text differently
/// from typed input (e.g., not triggering shortcuts).
pub fn enable_bracketed_paste() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b[?2004h")?;
    stdout.flush()?;
    BRACKETED_PASTE_ENABLED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Disable bracketed paste mode
pub fn disable_bracketed_paste() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b[?2004l")?;
    stdout.flush()?;
    BRACKETED_PASTE_ENABLED.store(false, Ordering::SeqCst);
    Ok(())
}

/// RAII guard for bracketed paste mode
///
/// Enables bracketed paste on creation and disables it on drop.
///
/// # Example
///
/// ```ignore
/// {
///     let _guard = BracketedPasteGuard::new()?;
///     // Bracketed paste is enabled here
/// }
/// // Bracketed paste is disabled when guard is dropped
/// ```
pub struct BracketedPasteGuard {
    was_enabled: bool,
}

impl BracketedPasteGuard {
    /// Create a new guard, enabling bracketed paste
    pub fn new() -> io::Result<Self> {
        let was_enabled = is_bracketed_paste_enabled();
        if !was_enabled {
            enable_bracketed_paste()?;
        }
        Ok(Self { was_enabled })
    }
}

impl Drop for BracketedPasteGuard {
    fn drop(&mut self) {
        if !self.was_enabled {
            let _ = disable_bracketed_paste();
        }
    }
}

/// Paste event containing the pasted text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasteEvent {
    /// The pasted text content
    pub content: String,
}

impl PasteEvent {
    /// Create a new paste event
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Get the pasted content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the number of characters in the paste
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the paste is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Get the number of lines in the paste
    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    /// Check if the paste contains multiple lines
    pub fn is_multiline(&self) -> bool {
        self.content.contains('\n')
    }

    /// Get the lines of the paste
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.content.lines()
    }
}

/// Register a paste handler for the current render pass (requires RuntimeContext).
pub(crate) fn register_paste_handler<F>(handler: F)
where
    F: Fn(&PasteEvent) + 'static,
{
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().register_paste_handler(handler);
    }
}

/// Dispatch a paste event to all handlers
pub fn dispatch_paste(content: &str) {
    let event = PasteEvent::new(content);
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow().dispatch_paste(&event);
    }
}

/// Hook to handle paste events
///
/// # Example
///
/// ```ignore
/// use_paste(|event| {
///     println!("Pasted {} characters", event.len());
///     if event.is_multiline() {
///         println!("Multi-line paste with {} lines", event.line_count());
///     }
/// });
/// ```
pub fn use_paste<F>(handler: F)
where
    F: Fn(&PasteEvent) + 'static,
{
    // Reserve a hook slot so use_paste follows the same ordering rules
    // as other hooks (catches conditional hook calls).
    if let Some(ctx) = crate::hooks::context::current_context() {
        ctx.borrow_mut().use_hook(|| ());
    }
    register_paste_handler(handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paste_event_creation() {
        let event = PasteEvent::new("Hello, World!");
        assert_eq!(event.content(), "Hello, World!");
        assert_eq!(event.len(), 13);
        assert!(!event.is_empty());
    }

    #[test]
    fn test_paste_event_empty() {
        let event = PasteEvent::new("");
        assert!(event.is_empty());
        assert_eq!(event.len(), 0);
    }

    #[test]
    fn test_paste_event_multiline() {
        let event = PasteEvent::new("Line 1\nLine 2\nLine 3");
        assert!(event.is_multiline());
        assert_eq!(event.line_count(), 3);
    }

    #[test]
    fn test_paste_event_single_line() {
        let event = PasteEvent::new("Single line");
        assert!(!event.is_multiline());
        assert_eq!(event.line_count(), 1);
    }

    #[test]
    fn test_paste_event_lines() {
        let event = PasteEvent::new("A\nB\nC");
        let lines: Vec<&str> = event.lines().collect();
        assert_eq!(lines, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_bracketed_paste_flag() {
        // Initially should be false
        BRACKETED_PASTE_ENABLED.store(false, Ordering::SeqCst);
        assert!(!is_bracketed_paste_enabled());

        // Set to true
        BRACKETED_PASTE_ENABLED.store(true, Ordering::SeqCst);
        assert!(is_bracketed_paste_enabled());

        // Reset
        BRACKETED_PASTE_ENABLED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_paste_handler_dispatch() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(ctx.clone()));

        let received = Rc::new(RefCell::new(String::new()));
        let received_clone = received.clone();

        register_paste_handler(move |event| {
            *received_clone.borrow_mut() = event.content().to_string();
        });

        dispatch_paste("test paste");

        assert_eq!(*received.borrow(), "test paste");

        set_current_runtime(None);
    }

    #[test]
    fn test_multiple_paste_handlers() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(ctx.clone()));

        let count = Rc::new(RefCell::new(0));
        let count1 = count.clone();
        let count2 = count.clone();

        register_paste_handler(move |_| {
            *count1.borrow_mut() += 1;
        });

        register_paste_handler(move |_| {
            *count2.borrow_mut() += 1;
        });

        dispatch_paste("test");

        assert_eq!(*count.borrow(), 2);

        set_current_runtime(None);
    }

    #[test]
    #[should_panic(expected = "Hook order violation")]
    fn test_use_paste_participates_in_hook_order() {
        use crate::hooks::context::{HookContext, with_hooks};
        use crate::hooks::use_signal;
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(HookContext::new()));

        with_hooks(ctx.clone(), || {
            use_paste(|_| {});
            let _state = use_signal(|| 1usize);
        });

        with_hooks(ctx, || {
            let _state = use_signal(|| 1usize);
        });
    }
}
