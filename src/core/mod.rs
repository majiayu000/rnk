//! Core types and abstractions

mod color;
mod element;
mod style;

pub use color::Color;
pub use element::{Children, Element, ElementId, ElementType};
pub use style::{
    AlignItems, AlignSelf, BorderStyle, Dimension, Display, Edges, FlexDirection, JustifyContent,
    Overflow, Position, Style, TextWrap,
};
