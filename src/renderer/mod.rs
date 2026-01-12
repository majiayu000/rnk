//! Rendering system
//!
//! This module provides the core rendering infrastructure for rnk:
//!
//! - **App**: The main application runner
//! - **AppBuilder**: Fluent API for configuring apps
//! - **Terminal**: Low-level terminal abstraction
//! - **Output**: Virtual output buffer for rendering
//!
//! ## Render Modes
//!
//! rnk supports two rendering modes:
//!
//! - **Inline mode** (default): Output appears at current cursor position,
//!   persists in terminal history. Like Ink and Bubbletea's default.
//!
//! - **Fullscreen mode**: Uses alternate screen buffer, content is cleared
//!   on exit. Like vim or Bubbletea's `WithAltScreen()`.
//!
//! ## Example
//!
//! ```ignore
//! use rnk::prelude::*;
//!
//! // Inline mode (default)
//! render(my_app).run()?;
//!
//! // Fullscreen mode
//! render(my_app).fullscreen().run()?;
//! ```

mod app;
mod output;
mod terminal;

pub use app::{
    // Core types
    App,
    AppBuilder,
    AppOptions,
    IntoPrintable,
    ModeSwitch,
    Printable,
    RenderHandle,
    enter_alt_screen,
    exit_alt_screen,
    is_alt_screen,
    println,
    println_trimmed,
    // Main entry point
    render,
    render_fullscreen,
    render_handle,
    render_inline,
    // Element rendering APIs
    render_to_string,
    render_to_string_auto,
    render_to_string_no_trim,
    // Cross-thread APIs
    request_render,
};
pub use output::Output;
pub use terminal::Terminal;
