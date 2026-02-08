//! Tag component for labels and categories
//!
//! Displays tags with optional close button.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Tag;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Row)
//!         .gap(1.0)
//!         .children(vec![
//!             Tag::new("rust").into_element(),
//!             Tag::new("terminal").color(Color::Cyan).into_element(),
//!             Tag::new("ui").closable().into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::capsule::CapsuleElementBuilder;
use crate::core::{Color, Element};

/// A tag component for labels and categories
#[derive(Debug, Clone)]
pub struct Tag {
    text: String,
    color: Color,
    background: Color,
    closable: bool,
    icon: Option<String>,
}

impl Tag {
    /// Create a new tag
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::White,
            background: Color::Ansi256(240),
            closable: false,
            icon: None,
        }
    }

    /// Set the text color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Make the tag closable (shows x)
    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    /// Add an icon before the text
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut builder = CapsuleElementBuilder::new(self.text, self.color, self.background);

        if let Some(icon) = self.icon {
            builder = builder.icon(icon);
        }

        if self.closable {
            builder = builder.suffix("×");
        }

        builder.into_element()
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_creation() {
        let t = Tag::new("test");
        assert_eq!(t.text, "test");
    }

    #[test]
    fn test_tag_closable() {
        let t = Tag::new("test").closable();
        assert!(t.closable);
    }

    #[test]
    fn test_tag_with_icon() {
        let t = Tag::new("rust").icon("🦀");
        assert_eq!(t.icon, Some("🦀".to_string()));
    }

    #[test]
    fn test_tag_into_element() {
        let _ = Tag::new("test").into_element();
        let _ = Tag::new("test").closable().into_element();
        let _ = Tag::new("test").icon("*").into_element();
    }

    #[test]
    fn test_tag_presets() {
        let _ = Tag::new("blue")
            .color(Color::White)
            .background(Color::Blue)
            .into_element();
        let _ = Tag::new("green")
            .color(Color::White)
            .background(Color::Green)
            .into_element();
        let _ = Tag::new("red")
            .color(Color::White)
            .background(Color::Red)
            .into_element();
    }
}
