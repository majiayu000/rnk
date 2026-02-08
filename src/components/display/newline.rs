//! Newline component - Line breaks in the UI

use crate::core::{Element, ElementType, Style};

/// Newline component for adding line breaks
#[derive(Debug, Clone, Default)]
pub struct Newline {
    count: u16,
}

impl Newline {
    /// Create a single newline
    pub fn new() -> Self {
        Self { count: 1 }
    }

    /// Create multiple newlines
    pub fn count(mut self, count: u16) -> Self {
        self.count = count;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut element = Element::new(ElementType::Text);
        element.text_content = Some("\n".repeat(self.count as usize));
        element.style = Style::new();
        element.style.flex_basis = crate::core::Dimension::Points(0.0);
        element.style.width = crate::core::Dimension::Percent(100.0);
        element.style.height = crate::core::Dimension::Points(self.count as f32);
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newline_single() {
        let element = Newline::new().into_element();
        assert_eq!(element.text_content, Some("\n".to_string()));
    }

    #[test]
    fn test_newline_multiple() {
        let element = Newline::new().count(3).into_element();
        assert_eq!(element.text_content, Some("\n\n\n".to_string()));
    }
}
