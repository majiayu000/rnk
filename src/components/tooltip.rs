//! Tooltip component for contextual information
//!
//! Displays tooltip text alongside content.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Tooltip;
//!
//! fn app() -> Element {
//!     Tooltip::new("Hover for info")
//!         .content("This is helpful information")
//!         .position(TooltipPosition::Right)
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Tooltip position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipPosition {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

/// A tooltip component
#[derive(Debug, Clone)]
pub struct Tooltip {
    label: String,
    content: String,
    position: TooltipPosition,
    visible: bool,
}

impl Tooltip {
    /// Create a new tooltip
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            content: String::new(),
            position: TooltipPosition::Right,
            visible: true,
        }
    }

    /// Set the tooltip content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Set the tooltip position
    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    /// Set visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let label_elem = Text::new(&self.label).color(Color::White).into_element();

        if !self.visible || self.content.is_empty() {
            return label_elem;
        }

        let tooltip_elem = RnkBox::new()
            .padding_x(1.0)
            .background(Color::Ansi256(240))
            .child(Text::new(&self.content).color(Color::White).into_element())
            .into_element();

        let (direction, children) = match self.position {
            TooltipPosition::Top => (FlexDirection::Column, vec![tooltip_elem, label_elem]),
            TooltipPosition::Bottom => (FlexDirection::Column, vec![label_elem, tooltip_elem]),
            TooltipPosition::Left => (
                FlexDirection::Row,
                vec![tooltip_elem, Text::new(" ").into_element(), label_elem],
            ),
            TooltipPosition::Right => (
                FlexDirection::Row,
                vec![label_elem, Text::new(" ").into_element(), tooltip_elem],
            ),
        };

        RnkBox::new()
            .flex_direction(direction)
            .children(children)
            .into_element()
    }
}

impl Default for Tooltip {
    fn default() -> Self {
        Self::new("")
    }
}

/// Create a tooltip
pub fn tooltip(label: impl Into<String>, content: impl Into<String>) -> Element {
    Tooltip::new(label).content(content).into_element()
}

/// Create a tooltip on the right
pub fn tooltip_right(label: impl Into<String>, content: impl Into<String>) -> Element {
    Tooltip::new(label)
        .content(content)
        .position(TooltipPosition::Right)
        .into_element()
}

/// Create a tooltip on the left
pub fn tooltip_left(label: impl Into<String>, content: impl Into<String>) -> Element {
    Tooltip::new(label)
        .content(content)
        .position(TooltipPosition::Left)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tooltip_creation() {
        let t = Tooltip::new("Label");
        assert_eq!(t.label, "Label");
    }

    #[test]
    fn test_tooltip_content() {
        let t = Tooltip::new("Label").content("Info");
        assert_eq!(t.content, "Info");
    }

    #[test]
    fn test_tooltip_positions() {
        let _ = Tooltip::new("L")
            .content("C")
            .position(TooltipPosition::Top)
            .into_element();
        let _ = Tooltip::new("L")
            .content("C")
            .position(TooltipPosition::Bottom)
            .into_element();
        let _ = Tooltip::new("L")
            .content("C")
            .position(TooltipPosition::Left)
            .into_element();
        let _ = Tooltip::new("L")
            .content("C")
            .position(TooltipPosition::Right)
            .into_element();
    }

    #[test]
    fn test_tooltip_helpers() {
        let _ = tooltip("Label", "Content");
        let _ = tooltip_right("Label", "Content");
        let _ = tooltip_left("Label", "Content");
    }
}
