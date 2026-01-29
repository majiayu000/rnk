//! Suspend/Resume support for Unix systems
//!
//! This module provides support for suspending the TUI with Ctrl+Z
//! and resuming with `fg`. This is a common Unix feature that allows
//! users to temporarily return to the shell.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Handler for suspend/resume signals
#[derive(Clone)]
pub struct SuspendHandler {
    /// Flag indicating a suspend was requested
    suspend_requested: Arc<AtomicBool>,
    /// Flag indicating the process was resumed
    resumed: Arc<AtomicBool>,
}

impl SuspendHandler {
    /// Create a new suspend handler
    pub fn new() -> Self {
        Self {
            suspend_requested: Arc::new(AtomicBool::new(false)),
            resumed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Request a suspend (called from signal handler or Ctrl+Z detection)
    pub fn request_suspend(&self) {
        self.suspend_requested.store(true, Ordering::SeqCst);
    }

    /// Check if suspend was requested and clear the flag
    pub fn take_suspend_request(&self) -> bool {
        self.suspend_requested.swap(false, Ordering::SeqCst)
    }

    /// Check if suspend was requested without clearing
    pub fn suspend_requested(&self) -> bool {
        self.suspend_requested.load(Ordering::SeqCst)
    }

    /// Mark that the process was resumed
    pub fn mark_resumed(&self) {
        self.resumed.store(true, Ordering::SeqCst);
    }

    /// Check if resumed and clear the flag
    pub fn take_resumed(&self) -> bool {
        self.resumed.swap(false, Ordering::SeqCst)
    }
}

impl Default for SuspendHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Install signal handlers for SIGTSTP and SIGCONT (Unix only)
///
/// Note: This is a no-op in the current implementation. Signal handling
/// is done via Ctrl+Z key detection in the event loop instead.
#[cfg(unix)]
pub fn install_suspend_handlers(_handler: &SuspendHandler) -> std::io::Result<()> {
    // Signal handling is done via Ctrl+Z key detection in the event loop
    // This function is kept for API compatibility
    Ok(())
}

/// Install signal handlers (no-op on non-Unix platforms)
#[cfg(not(unix))]
pub fn install_suspend_handlers(_handler: &SuspendHandler) -> std::io::Result<()> {
    // Suspend/resume is not supported on non-Unix platforms
    Ok(())
}

/// Suspend the current process (Unix only)
///
/// This sends SIGTSTP to the current process, which will suspend it.
/// The process can be resumed with `fg` in the shell.
#[cfg(unix)]
pub fn suspend_self() -> std::io::Result<()> {
    // Use libc directly to send SIGTSTP to ourselves
    unsafe {
        let result = libc::raise(libc::SIGTSTP);
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Suspend the current process (no-op on non-Unix platforms)
#[cfg(not(unix))]
pub fn suspend_self() -> std::io::Result<()> {
    // Suspend is not supported on non-Unix platforms
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suspend_handler_creation() {
        let handler = SuspendHandler::new();
        assert!(!handler.suspend_requested());
    }

    #[test]
    fn test_suspend_handler_request() {
        let handler = SuspendHandler::new();
        assert!(!handler.suspend_requested());

        handler.request_suspend();
        assert!(handler.suspend_requested());

        // take_suspend_request should clear the flag
        assert!(handler.take_suspend_request());
        assert!(!handler.suspend_requested());
    }

    #[test]
    fn test_suspend_handler_resumed() {
        let handler = SuspendHandler::new();

        handler.mark_resumed();
        assert!(handler.take_resumed());
        assert!(!handler.take_resumed()); // Should be cleared
    }

    #[test]
    fn test_suspend_handler_clone() {
        let handler = SuspendHandler::new();
        let handler2 = handler.clone();

        handler.request_suspend();
        assert!(handler2.suspend_requested());
    }
}
