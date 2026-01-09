//! Core types and abstractions

mod element;
mod style;
mod color;

pub use element::{Element, ElementId, ElementType, Children};
pub use style::{
    Style,
    FlexDirection,
    AlignItems,
    AlignSelf,
    JustifyContent,
    Display,
    Position,
    Overflow,
    TextWrap,
    BorderStyle,
    Dimension,
    Edges,
};
pub use color::Color;
