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
pub(crate) mod element_renderer;
mod output;
pub(crate) mod registry;
pub(crate) mod render_to_string;
pub(crate) mod runtime;
pub(crate) mod static_content;
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
    // Cross-thread APIs
    request_render,
};

// Element rendering APIs (from render_to_string module)
pub use render_to_string::{render_to_string, render_to_string_auto, render_to_string_no_trim};

// Re-export AppSink from registry (internal use only)
pub use output::Output;
pub use registry::AppSink;
pub use terminal::Terminal;
