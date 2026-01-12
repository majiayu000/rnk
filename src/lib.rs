//! # rnk - React-like Terminal UI for Rust
//!
//! A terminal UI framework inspired by [ink](https://github.com/vadimdemedes/ink).
//!
//! ## Features
//!
//! - Declarative UI with flexbox layout
//! - Reactive state management with hooks
//! - Keyboard input handling
//! - ANSI color and style support
//! - **Cross-thread render requests** for async/multi-threaded apps
//!
//! ## Example
//!
//! ```rust,no_run
//! use rnk::prelude::*;
//!
//! fn main() -> std::io::Result<()> {
//!     render(app)
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

pub mod core;
pub mod components;
pub mod hooks;
pub mod layout;
pub mod renderer;
pub mod runtime;

/// Testing utilities for verifying UI components
pub mod testing;

// Re-export prelude
pub mod prelude;

// Re-export main types
pub use crate::core::{Element, ElementId, Style, Color};
pub use crate::components::{Box, Text};
pub use crate::renderer::{render, request_render, render_handle, RenderHandle};
