//! Core types and abstractions.
//!
//! This module is an advanced public surface. Application code should prefer
//! `rnk::prelude::*`; direct `core` imports are useful for custom components,
//! tests, and low-level integration.

mod color;
mod component;
mod element;
mod style;
mod vnode;

pub use color::{
    AdaptiveColor, Color, adaptive_colors, detect_background, init_background_detection,
    is_dark_background, set_dark_background,
};
#[doc(hidden)]
pub use component::{Component, ComponentInstance, StatelessComponent};
pub use element::{
    AccessibilityProps, AccessibilityRole, Children, Element, ElementId, ElementType,
};
pub use style::{
    AlignItems, AlignSelf, BorderStyle, Dimension, Display, Edges, FlexDirection, JustifyContent,
    Overflow, Position, Style, TextWrap,
};
#[doc(hidden)]
pub use vnode::{NodeKey, Props, VNode, VNodeType};
