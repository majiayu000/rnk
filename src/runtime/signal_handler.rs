//! Signal handler for graceful shutdown
//!
//! Handles SIGINT (Ctrl+C), SIGTERM, and SIGHUP signals to ensure
//! terminal state is restored before exit.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::runtime::restore_terminal;

/// Signal handler that manages graceful shutdown
pub struct SignalHandler {
    /// Flag indicating whether a shutdown signal was received
    should_exit: Arc<AtomicBool>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new() -> Self {
        Self {
            should_exit: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a clone of the should_exit flag for checking in event loops
    pub fn should_exit_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.should_exit)
    }

    /// Check if a shutdown signal was received
    pub fn should_exit(&self) -> bool {
        self.should_exit.load(Ordering::SeqCst)
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Install a global signal handler that restores terminal state on signals
///
/// This handles:
/// - SIGINT (Ctrl+C)
/// - SIGTERM (with "termination" feature)
/// - SIGHUP (with "termination" feature)
///
/// # Returns
///
/// Returns a `SignalHandler` that can be used to check if exit was requested.
///
/// # Example
///
/// ```no_run
/// use tink::runtime::install_signal_handler;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let signal_handler = install_signal_handler()?;
///
///     loop {
///         if signal_handler.should_exit() {
///             break;
///         }
///         // Your app logic
///     }
///     Ok(())
/// }
/// ```
pub fn install_signal_handler() -> Result<SignalHandler, ctrlc::Error> {
    let handler = SignalHandler::new();
    let should_exit = handler.should_exit_flag();

    ctrlc::set_handler(move || {
        // Restore terminal state
        restore_terminal();

        // Mark that we should exit
        should_exit.store(true, Ordering::SeqCst);

        // Give the main loop a chance to exit gracefully
        // If it doesn't exit in time, the process will be terminated by the OS
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Force exit if still running
        std::process::exit(130); // 128 + SIGINT(2)
    })?;

    Ok(handler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        assert!(!handler.should_exit());
    }

    #[test]
    fn test_signal_handler_flag_sharing() {
        let handler = SignalHandler::new();
        let flag = handler.should_exit_flag();

        assert!(!flag.load(Ordering::SeqCst));
        flag.store(true, Ordering::SeqCst);
        assert!(handler.should_exit());
    }
}
