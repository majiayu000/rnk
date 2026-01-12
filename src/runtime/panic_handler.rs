//! Panic handler for terminal state restoration
//!
//! Ensures terminal is restored to a usable state even when the program panics.

use std::io::Write;
use std::panic;
use std::sync::Once;

use crossterm::{
    cursor, execute,
    terminal::{self, LeaveAlternateScreen},
};

static PANIC_HOOK_INSTALLED: Once = Once::new();

/// Restore terminal to a normal state
///
/// This function:
/// 1. Disables raw mode
/// 2. Leaves alternate screen (if active)
/// 3. Shows cursor
/// 4. Disables mouse capture
pub fn restore_terminal() {
    let mut stdout = std::io::stdout();

    // Disable raw mode first
    let _ = terminal::disable_raw_mode();

    // Leave alternate screen and show cursor
    let _ = execute!(
        stdout,
        LeaveAlternateScreen,
        cursor::Show,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::event::DisableFocusChange,
    );

    // Flush to ensure all escape sequences are sent
    let _ = stdout.flush();
}

/// Install a panic hook that restores terminal state before printing panic info
///
/// This hook:
/// 1. Restores terminal to normal mode
/// 2. Prints panic information in a readable format
/// 3. Preserves the original panic hook behavior
///
/// # Example
///
/// ```no_run
/// use rnk::runtime::install_panic_hook;
///
/// install_panic_hook();
/// // Your app code here
/// ```
pub fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let original_hook = panic::take_hook();

        panic::set_hook(Box::new(move |panic_info| {
            // First, restore terminal state
            restore_terminal();

            // Print a separator for clarity
            eprintln!(
                "\n\x1b[1;31m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
            );
            eprintln!("\x1b[1;31mPanic occurred! Terminal state has been restored.\x1b[0m");
            eprintln!(
                "\x1b[1;31m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m\n"
            );

            // Call the original hook to print the panic info
            original_hook(panic_info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_terminal_doesnt_panic() {
        // Just ensure it doesn't panic when called
        restore_terminal();
    }

    #[test]
    fn test_install_panic_hook_idempotent() {
        // Can be called multiple times safely
        install_panic_hook();
        install_panic_hook();
    }
}
