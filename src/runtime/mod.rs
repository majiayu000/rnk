//! Runtime utilities for terminal state management
//!
//! This module provides:
//! - Unified runtime context for app state
//! - Panic hook for terminal restoration
//! - Signal handling (SIGINT, SIGTERM, SIGHUP)
//! - Environment detection (CI, TTY)

mod context;
mod environment;
mod panic_handler;
mod signal_handler;

pub use context::{
    RuntimeContext, current_runtime, set_current_runtime, with_current_runtime, with_runtime,
};
pub use environment::{Environment, is_ci, is_tty};
pub use panic_handler::{install_panic_hook, restore_terminal};
pub use signal_handler::{SignalHandler, install_signal_handler};
