//! Spacer component - Flexible space in layouts

use crate::core::{Element, ElementType, Style};

/// Spacer component that expands to fill available space
///
/// Useful for pushing elements apart in a flex container.
///
/// # Example
///
/// ```ignore
/// Box::new()
///     .flex_direction(FlexDirection::Row)
///     .child(Text::new("Left").into_element())
///     .child(Spacer::new().into_element())
///     .child(Text::new("Right").into_element())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Spacer {
    flex_grow: f32,
}

impl Spacer {
    /// Create a new spacer with flex-grow: 1
    pub fn new() -> Self {
        Self { flex_grow: 1.0 }
    }

    /// Set the flex grow value
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut element = Element::new(ElementType::Box);
        element.style = Style::new();
        element.style.flex_grow = self.flex_grow;
        element.style.flex_shrink = 0.0;
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spacer_default() {
        let element = Spacer::new().into_element();
        assert_eq!(element.style.flex_grow, 1.0);
        assert_eq!(element.style.flex_shrink, 0.0);
    }

    #[test]
    fn test_spacer_custom_flex() {
        let element = Spacer::new().flex(2.0).into_element();
        assert_eq!(element.style.flex_grow, 2.0);
    }
}
