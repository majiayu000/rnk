//! # Tink - Terminal Ink for Rust
//!
//! A terminal UI framework inspired by [ink](https://github.com/vadimdemedes/ink).
//!
//! ## Features
//!
//! - Declarative UI with flexbox layout
//! - Reactive state management with hooks
//! - Keyboard input handling
//! - ANSI color and style support
//!
//! ## Example
//!
//! ```rust,no_run
//! use tink::prelude::*;
//!
//! fn main() -> std::io::Result<()> {
//!     render(app)
//! }
//!
//! fn app() -> Element {
//!     Box::new()
//!         .padding(1)
//!         .child(Text::new("Hello, Tink!").bold().into_element())
//!         .into_element()
//! }
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
pub use crate::renderer::render;
