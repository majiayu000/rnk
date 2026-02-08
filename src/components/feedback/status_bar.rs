//! StatusBar component for application status display
//!
//! A flexible status bar typically shown at the bottom of the screen.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::StatusBar;
//!
//! fn app() -> Element {
//!     StatusBar::new()
//!         .left("INSERT")
//!         .center("main.rs")
//!         .right("Ln 42, Col 10")
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{AlignItems, Color, Element, FlexDirection, JustifyContent};

/// A status bar component
#[derive(Debug, Clone)]
pub struct StatusBar {
    left: Option<String>,
    center: Option<String>,
    right: Option<String>,
    background: Color,
    foreground: Color,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            left: None,
            center: None,
            right: None,
            background: Color::Ansi256(236),
            foreground: Color::White,
        }
    }

    /// Set the left section content
    pub fn left(mut self, content: impl Into<String>) -> Self {
        self.left = Some(content.into());
        self
    }

    /// Set the center section content
    pub fn center(mut self, content: impl Into<String>) -> Self {
        self.center = Some(content.into());
        self
    }

    /// Set the right section content
    pub fn right(mut self, content: impl Into<String>) -> Self {
        self.right = Some(content.into());
        self
    }

    /// Set the background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Set the foreground color
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = color;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut children = Vec::new();

        // Left section
        children.push(
            RnkBox::new()
                .flex_grow(1.0)
                .child(
                    Text::new(self.left.unwrap_or_default())
                        .color(self.foreground)
                        .into_element(),
                )
                .into_element(),
        );

        // Center section
        if let Some(center) = self.center {
            children.push(
                RnkBox::new()
                    .flex_grow(1.0)
                    .justify_content(JustifyContent::Center)
                    .child(Text::new(center).color(self.foreground).into_element())
                    .into_element(),
            );
        }

        // Right section
        children.push(
            RnkBox::new()
                .flex_grow(1.0)
                .justify_content(JustifyContent::FlexEnd)
                .child(
                    Text::new(self.right.unwrap_or_default())
                        .color(self.foreground)
                        .into_element(),
                )
                .into_element(),
        );

        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .align_items(AlignItems::Center)
            .padding_x(1.0)
            .background(self.background)
            .children(children)
            .into_element()
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a simple status bar
pub fn status_bar(left: &str, right: &str) -> Element {
    StatusBar::new().left(left).right(right).into_element()
}

/// Create a status bar with all three sections
pub fn status_bar_full(left: &str, center: &str, right: &str) -> Element {
    StatusBar::new()
        .left(left)
        .center(center)
        .right(right)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_creation() {
        let sb = StatusBar::new();
        assert!(sb.left.is_none());
    }

    #[test]
    fn test_status_bar_sections() {
        let sb = StatusBar::new()
            .left("LEFT")
            .center("CENTER")
            .right("RIGHT");
        assert_eq!(sb.left, Some("LEFT".to_string()));
        assert_eq!(sb.center, Some("CENTER".to_string()));
        assert_eq!(sb.right, Some("RIGHT".to_string()));
    }

    #[test]
    fn test_status_bar_into_element() {
        let _ = StatusBar::new().left("Mode").right("100%").into_element();
    }

    #[test]
    fn test_status_bar_helpers() {
        let _ = status_bar("Left", "Right");
        let _ = status_bar_full("L", "C", "R");
    }
}
