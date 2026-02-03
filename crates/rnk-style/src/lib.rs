//! # rnk-style
//!
//! Standalone terminal styling library for Rust.
//!
//! Style your terminal output without the full TUI framework.
//!
//! ## Example
//!
//! ```
//! use rnk_style::{Style, Color, BorderStyle};
//!
//! let style = Style::new()
//!     .fg(Color::Cyan)
//!     .bold()
//!     .padding(1, 2)
//!     .border(BorderStyle::Rounded);
//!
//! println!("{}", style.render("Hello, World!"));
//! ```

mod color;
mod style;
mod border;
mod render;

pub use color::Color;
pub use style::Style;
pub use border::BorderStyle;
