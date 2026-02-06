//! KeyHint component for keyboard shortcut hints
//!
//! Displays keyboard shortcuts in a consistent format.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::KeyHint;
//!
//! fn footer() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Row)
//!         .gap(2.0)
//!         .children(vec![
//!             KeyHint::new("q", "Quit").into_element(),
//!             KeyHint::new("↑↓", "Navigate").into_element(),
//!             KeyHint::new("Enter", "Select").into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// A key hint component for displaying keyboard shortcuts
#[derive(Debug, Clone)]
pub struct KeyHint {
    key: String,
    description: String,
    key_color: Color,
    desc_color: Color,
}

impl KeyHint {
    /// Create a new key hint
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            key_color: Color::Yellow,
            desc_color: Color::BrightBlack,
        }
    }

    /// Set the key color
    pub fn key_color(mut self, color: Color) -> Self {
        self.key_color = color;
        self
    }

    /// Set the description color
    pub fn desc_color(mut self, color: Color) -> Self {
        self.desc_color = color;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .gap(0.5)
            .children(vec![
                Text::new(&self.key)
                    .color(self.key_color)
                    .bold()
                    .into_element(),
                Text::new(&self.description)
                    .color(self.desc_color)
                    .into_element(),
            ])
            .into_element()
    }
}

impl Default for KeyHint {
    fn default() -> Self {
        Self::new("", "")
    }
}

/// Create a key hint
pub fn key_hint(key: impl Into<String>, description: impl Into<String>) -> Element {
    KeyHint::new(key, description).into_element()
}

/// Create a row of key hints
pub fn key_hints(hints: Vec<(&str, &str)>) -> Element {
    let children: Vec<Element> = hints
        .into_iter()
        .map(|(key, desc)| KeyHint::new(key, desc).into_element())
        .collect();

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .gap(2.0)
        .children(children)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_hint_creation() {
        let kh = KeyHint::new("q", "Quit");
        assert_eq!(kh.key, "q");
        assert_eq!(kh.description, "Quit");
    }

    #[test]
    fn test_key_hint_into_element() {
        let _ = KeyHint::new("Enter", "Select").into_element();
    }

    #[test]
    fn test_key_hint_helper() {
        let _ = key_hint("q", "Quit");
    }

    #[test]
    fn test_key_hints_helper() {
        let _ = key_hints(vec![("q", "Quit"), ("↑↓", "Navigate"), ("Enter", "Select")]);
    }
}
