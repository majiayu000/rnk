//! Library-level error types for rnk
//!
//! This module provides unified error handling across the library,
//! replacing scattered `unwrap()` calls with proper error propagation.

use thiserror::Error;

/// Main error type for rnk operations
#[derive(Error, Debug)]
pub enum RnkError {
    /// A lock (Mutex/RwLock) was poisoned by a panicking thread
    #[error("Lock poisoned: {context}")]
    LockPoisoned {
        /// Description of which lock was poisoned
        context: &'static str,
    },

    /// Hook was called outside of a component render context
    #[error("Hook context not available: {0}")]
    NoHookContext(&'static str),

    /// IO operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Layout calculation failed
    #[error("Layout error: {0}")]
    Layout(String),

    /// Rendering operation failed
    #[error("Render error: {0}")]
    Render(String),
}

/// Result type alias using RnkError
pub type Result<T> = std::result::Result<T, RnkError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RnkError::LockPoisoned {
            context: "signal value",
        };
        assert_eq!(err.to_string(), "Lock poisoned: signal value");

        let err = RnkError::NoHookContext("use_signal");
        assert_eq!(err.to_string(), "Hook context not available: use_signal");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rnk_err: RnkError = io_err.into();
        assert!(matches!(rnk_err, RnkError::Io(_)));
    }
}
