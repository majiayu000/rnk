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
//!     .border(BorderStyle::Round);
//!
//! println!("{}", style.render("Hello, World!"));
//! ```

mod render;
mod style;

pub use rnk_style_core::{
    AdaptiveColor, BorderStyle, Color, adaptive_colors, detect_background,
    init_background_detection, is_dark_background, set_dark_background,
};
pub use style::Style;
