//! Shared style primitives for rnk crates.

mod border;
mod color;

pub use border::BorderStyle;
pub use color::{
    AdaptiveColor, Color, adaptive_colors, detect_background, init_background_detection,
    is_dark_background, set_dark_background,
};
