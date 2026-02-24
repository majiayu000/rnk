//! Testing infrastructure for rnk
//!
//! Provides utilities for testing terminal UI components without
//! actual terminal interaction.
//!
//! # Example
//!
//! ```rust
//! use rnk::testing::TestRenderer;
//! use rnk::prelude::*;
//!
//! let renderer = TestRenderer::new(80, 24);
//! let element = Text::new("Hello").into_element();
//! let output = renderer.render_to_plain(&element);
//! assert_eq!(output.trim(), "Hello");
//! ```

mod assertions;
mod generators;
mod golden;
mod harness;
mod renderer;

pub use assertions::*;
pub use generators::*;
pub use golden::*;
pub use harness::{Snapshot, StringSnapshot, TestHarness};
pub use renderer::{LayoutError, TestRenderer, display_width, strip_ansi_codes};
