//! Highlight component for text highlighting
//!
//! Highlights text with customizable colors.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Highlight;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Column)
//!         .children(vec![
//!             Highlight::new("Important text").into_element(),
//!             Highlight::new("Warning").variant(HighlightVariant::Warning).into_element(),
//!             Highlight::new("Error").variant(HighlightVariant::Error).into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::capsule::CapsuleElementBuilder;
use crate::components::capsule_variant::CapsuleVariant;
use crate::core::Element;

/// Highlight variant.
pub type HighlightVariant = CapsuleVariant;

/// A highlight component for emphasized text
#[derive(Debug, Clone)]
pub struct Highlight {
    text: String,
    variant: HighlightVariant,
}

impl Highlight {
    /// Create a new highlight
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: HighlightVariant::Default,
        }
    }

    /// Set the variant
    pub fn variant(mut self, variant: HighlightVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let (fg, bg) = self.variant.highlight_colors();

        CapsuleElementBuilder::new(self.text, fg, bg).into_element()
    }
}

impl Default for Highlight {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_creation() {
        let h = Highlight::new("Test");
        assert_eq!(h.text, "Test");
    }

    #[test]
    fn test_highlight_variants() {
        let _ = Highlight::new("P")
            .variant(HighlightVariant::Primary)
            .into_element();
        let _ = Highlight::new("S")
            .variant(HighlightVariant::Success)
            .into_element();
        let _ = Highlight::new("W")
            .variant(HighlightVariant::Warning)
            .into_element();
        let _ = Highlight::new("E")
            .variant(HighlightVariant::Error)
            .into_element();
        let _ = Highlight::new("I")
            .variant(HighlightVariant::Info)
            .into_element();
    }
}
