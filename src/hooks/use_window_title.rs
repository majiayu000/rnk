//! Window title control hook
//!
//! Provides functions to set the terminal window title.

use std::io::{Write, stdout};

/// ANSI escape sequence for setting window title (OSC - Operating System Command)
/// Format: ESC ] 0 ; <title> BEL  or  ESC ] 0 ; <title> ST
fn set_title_escape(title: &str) -> String {
    // OSC 0 sets both window title and icon name
    // BEL (0x07) terminates the sequence in most terminals
    format!("\x1b]0;{}\x07", title)
}

/// Restore the original window title (best effort)
/// Some terminals support saving/restoring, but it's not universal
fn restore_title_escape() -> &'static str {
    // Try to restore using ST (String Terminator)
    // This may not work on all terminals
    "\x1b]0;\x07"
}

/// Set the terminal window title
///
/// # Example
///
/// ```ignore
/// use tink::hooks::use_window_title::set_window_title;
///
/// set_window_title("My Application - v1.0");
/// ```
pub fn set_window_title(title: &str) {
    let mut stdout = stdout();
    let _ = write!(stdout, "{}", set_title_escape(title));
    let _ = stdout.flush();
}

/// Clear the window title (set to empty)
pub fn clear_window_title() {
    set_window_title("");
}

/// RAII guard for window title that restores on drop
pub struct WindowTitleGuard {
    original_title: Option<String>,
}

impl WindowTitleGuard {
    /// Create a new guard that will attempt to restore the title on drop
    pub fn new(original_title: Option<String>) -> Self {
        Self { original_title }
    }
}

impl Drop for WindowTitleGuard {
    fn drop(&mut self) {
        if let Some(ref title) = self.original_title {
            set_window_title(title);
        } else {
            // Try to restore to empty
            let mut stdout = stdout();
            let _ = write!(stdout, "{}", restore_title_escape());
            let _ = stdout.flush();
        }
    }
}

/// Hook to set the window title
///
/// The title will be set each time the component renders.
///
/// # Example
///
/// ```ignore
/// use tink::prelude::*;
///
/// fn app() -> Element {
///     // Set window title
///     use_window_title("My Tink App");
///
///     // Or with dynamic title
///     let count = use_signal(|| 0);
///     use_window_title(&format!("Count: {}", count.get()));
///
///     Box::new()
///         .child(Text::new("Hello").into_element())
///         .into_element()
/// }
/// ```
pub fn use_window_title(title: &str) {
    set_window_title(title);
}

/// Hook to set the window title with a function
///
/// Useful when the title depends on state that changes.
///
/// # Example
///
/// ```ignore
/// use tink::prelude::*;
///
/// fn app() -> Element {
///     let items = use_signal(|| vec!["a", "b", "c"]);
///
///     use_window_title_fn(|| format!("Items: {}", items.get().len()));
///
///     // ...
/// }
/// ```
pub fn use_window_title_fn<F>(f: F)
where
    F: FnOnce() -> String,
{
    set_window_title(&f());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_escape_sequence() {
        let escape = set_title_escape("Test Title");
        assert_eq!(escape, "\x1b]0;Test Title\x07");
    }

    #[test]
    fn test_empty_title() {
        let escape = set_title_escape("");
        assert_eq!(escape, "\x1b]0;\x07");
    }

    #[test]
    fn test_title_with_special_chars() {
        let escape = set_title_escape("My App - [1/10]");
        assert_eq!(escape, "\x1b]0;My App - [1/10]\x07");
    }
}
