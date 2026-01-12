//! Runtime utilities for terminal state management
//!
//! This module provides:
//! - Panic hook for terminal restoration
//! - Signal handling (SIGINT, SIGTERM, SIGHUP)
//! - Environment detection (CI, TTY)

mod environment;
mod panic_handler;
mod signal_handler;

pub use environment::{Environment, is_ci, is_tty};
pub use panic_handler::{install_panic_hook, restore_terminal};
pub use signal_handler::{SignalHandler, install_signal_handler};
