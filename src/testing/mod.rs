//! Testing infrastructure for Tink
//!
//! Provides utilities for testing terminal UI components without
//! actual terminal interaction.
//!
//! # Example
//!
//! ```rust
//! use tink::testing::TestRenderer;
//! use tink::prelude::*;
//!
//! let renderer = TestRenderer::new(80, 24);
//! let element = Text::new("Hello").into_element();
//! let output = renderer.render_to_plain(&element);
//! assert_eq!(output.trim(), "Hello");
//! ```

mod renderer;
mod assertions;
mod golden;
mod generators;

pub use renderer::{TestRenderer, LayoutError, strip_ansi_codes, display_width};
pub use assertions::*;
pub use golden::*;
pub use generators::*;
