//! # rnk - React-like Terminal UI for Rust
//!
//! A terminal UI framework inspired by [Ink](https://github.com/vadimdemedes/ink)
//! and [Bubbletea](https://github.com/charmbracelet/bubbletea).
//!
//! ## Features
//!
//! - Declarative UI with flexbox layout
//! - Reactive state management with hooks
//! - Keyboard and mouse input handling
//! - ANSI color and style support
//! - **Inline mode** (default): Output persists in terminal history
//! - **Fullscreen mode**: Uses alternate screen buffer
//! - **Cross-thread render requests** for async/multi-threaded apps
//! - **Runtime mode switching** between inline and fullscreen
//! - **Println** for persistent messages above the UI
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rnk::prelude::*;
//!
//! fn main() -> std::io::Result<()> {
//!     render(app).run()
//! }
//!
//! fn app() -> Element {
//!     Box::new()
//!         .padding(1)
//!         .child(Text::new("Hello, rnk!").bold().into_element())
//!         .into_element()
//! }
//! ```
//!
//! ## Render Modes
//!
//! ### Inline Mode (Default)
//!
//! Output appears at the current cursor position and persists in terminal history.
//! This is the default mode, matching Ink and Bubbletea's behavior.
//!
//! ```rust,ignore
//! render(app).run()?;           // Inline mode (default)
//! render(app).inline().run()?;  // Explicit inline mode
//! ```
//!
//! ### Fullscreen Mode
//!
//! Uses the alternate screen buffer. Content is cleared when the app exits.
//!
//! ```rust,ignore
//! render(app).fullscreen().run()?;
//! ```
//!
//! ## Runtime Mode Switching
//!
//! Switch between modes at runtime (like Bubbletea):
//!
//! ```rust,ignore
//! let app = use_app();
//!
//! use_input(move |key| {
//!     if key == Key::Char(' ') {
//!         if app.is_alt_screen() {
//!             app.exit_alt_screen();  // Switch to inline
//!         } else {
//!             app.enter_alt_screen(); // Switch to fullscreen
//!         }
//!     }
//! });
//! ```
//!
//! ## Println for Persistent Messages
//!
//! In inline mode, use `println()` to output messages above the UI:
//!
//! ```rust,ignore
//! use rnk::println;
//!
//! // In an input handler
//! rnk::println("Task completed!");
//! rnk::println(format!("Downloaded {} files", count));
//!
//! // Or via AppContext
//! let app = use_app();
//! app.println("Another message");
//! ```
//!
//! ## Cross-Thread Rendering
//!
//! When updating state from a background thread, use `request_render()` to
//! notify the UI to refresh:
//!
//! ```rust,ignore
//! use std::thread;
//! use std::sync::{Arc, RwLock};
//! use rnk::request_render;
//!
//! let state = Arc::new(RwLock::new(0));
//! let state_clone = Arc::clone(&state);
//!
//! thread::spawn(move || {
//!     *state_clone.write().unwrap() += 1;
//!     request_render(); // Notify rnk to re-render
//! });
//! ```

pub mod cmd;
pub mod components;
pub mod core;
pub mod hooks;
pub mod layout;
pub mod renderer;
pub mod runtime;

/// Testing utilities for verifying UI components
pub mod testing;

// Re-export prelude
pub mod prelude;

// Re-export main types
pub use crate::components::{Box, Text};
pub use crate::core::{Color, Element, ElementId, Style};

// Re-export rendering APIs
pub use crate::renderer::{
    AppBuilder,
    AppOptions,
    IntoPrintable,
    ModeSwitch,
    Printable,
    // Types
    RenderHandle,
    enter_alt_screen,
    exit_alt_screen,
    is_alt_screen,
    println,
    println_trimmed,
    // Main entry points
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
