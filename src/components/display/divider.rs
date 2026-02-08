//! Divider component for visual separation
//!
//! Creates horizontal or vertical dividers with optional labels.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Divider;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Column)
//!         .children(vec![
//!             Text::new("Section 1").into_element(),
//!             Divider::horizontal().into_element(),
//!             Text::new("Section 2").into_element(),
//!             Divider::horizontal().label("OR").into_element(),
//!             Text::new("Section 3").into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::Text;
use crate::core::{Color, Element};

/// Divider orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Divider style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    Double,
}

/// A divider component for visual separation
#[derive(Debug, Clone)]
pub struct Divider {
    orientation: DividerOrientation,
    style: DividerStyle,
    label: Option<String>,
    color: Color,
    width: usize,
}

impl Divider {
    /// Create a new horizontal divider
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            style: DividerStyle::Solid,
            label: None,
            color: Color::BrightBlack,
            width: 40,
        }
    }

    /// Create a new vertical divider
    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
            style: DividerStyle::Solid,
            label: None,
            color: Color::BrightBlack,
            width: 1,
        }
    }

    /// Set the divider style
    pub fn style(mut self, style: DividerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set a label in the middle of the divider
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the divider color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the width (for horizontal) or height (for vertical)
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let char = match (self.orientation, self.style) {
            (DividerOrientation::Horizontal, DividerStyle::Solid) => '─',
            (DividerOrientation::Horizontal, DividerStyle::Dashed) => '╌',
            (DividerOrientation::Horizontal, DividerStyle::Dotted) => '┄',
            (DividerOrientation::Horizontal, DividerStyle::Double) => '═',
            (DividerOrientation::Vertical, DividerStyle::Solid) => '│',
            (DividerOrientation::Vertical, DividerStyle::Dashed) => '╎',
            (DividerOrientation::Vertical, DividerStyle::Dotted) => '┆',
            (DividerOrientation::Vertical, DividerStyle::Double) => '║',
        };

        let content = match &self.label {
            Some(label) if self.orientation == DividerOrientation::Horizontal => {
                let label_len = label.len() + 2; // " label "
                let side_len = self.width.saturating_sub(label_len) / 2;
                let left = char.to_string().repeat(side_len);
                let right = char
                    .to_string()
                    .repeat(self.width.saturating_sub(side_len + label_len));
                format!("{} {} {}", left, label, right)
            }
            _ => char.to_string().repeat(self.width),
        };

        Text::new(content).color(self.color).into_element()
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::horizontal()
    }
}

/// Create a simple horizontal divider
pub fn hr() -> Element {
    Divider::horizontal().into_element()
}

/// Create a horizontal divider with a label
pub fn hr_label(label: impl Into<String>) -> Element {
    Divider::horizontal().label(label).into_element()
}

/// Create a dashed horizontal divider
pub fn hr_dashed() -> Element {
    Divider::horizontal()
        .style(DividerStyle::Dashed)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_horizontal() {
        let divider = Divider::horizontal();
        assert_eq!(divider.orientation, DividerOrientation::Horizontal);
    }

    #[test]
    fn test_divider_vertical() {
        let divider = Divider::vertical();
        assert_eq!(divider.orientation, DividerOrientation::Vertical);
    }

    #[test]
    fn test_divider_with_label() {
        let divider = Divider::horizontal().label("OR");
        assert_eq!(divider.label, Some("OR".to_string()));
    }

    #[test]
    fn test_divider_into_element() {
        let _ = Divider::horizontal().into_element();
        let _ = Divider::vertical().into_element();
        let _ = Divider::horizontal().label("Test").into_element();
    }

    #[test]
    fn test_divider_helpers() {
        let _ = hr();
        let _ = hr_label("Section");
        let _ = hr_dashed();
    }
}
