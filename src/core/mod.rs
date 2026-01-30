//! Core types and abstractions

mod color;
mod element;
mod style;

pub use color::{
    AdaptiveColor, Color, adaptive_colors, detect_background, init_background_detection,
    is_dark_background, set_dark_background,
};
pub use element::{Children, Element, ElementId, ElementType};
pub use style::{
    AlignItems, AlignSelf, BorderStyle, Dimension, Display, Edges, FlexDirection, JustifyContent,
    Overflow, Position, Style, TextWrap,
};
