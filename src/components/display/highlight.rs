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

use crate::components::capsule::CapsuleLabel;
use crate::core::{Color, Element};

/// Highlight variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HighlightVariant {
    #[default]
    Default,
    Primary,
    Success,
    Warning,
    Error,
    Info,
}

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

    /// Create a primary highlight
    pub fn primary(text: impl Into<String>) -> Self {
        Self::new(text).variant(HighlightVariant::Primary)
    }

    /// Create a success highlight
    pub fn success(text: impl Into<String>) -> Self {
        Self::new(text).variant(HighlightVariant::Success)
    }

    /// Create a warning highlight
    pub fn warning(text: impl Into<String>) -> Self {
        Self::new(text).variant(HighlightVariant::Warning)
    }

    /// Create an error highlight
    pub fn error(text: impl Into<String>) -> Self {
        Self::new(text).variant(HighlightVariant::Error)
    }

    /// Create an info highlight
    pub fn info(text: impl Into<String>) -> Self {
        Self::new(text).variant(HighlightVariant::Info)
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let (fg, bg) = match self.variant {
            HighlightVariant::Default => (Color::Black, Color::Yellow),
            HighlightVariant::Primary => (Color::White, Color::Blue),
            HighlightVariant::Success => (Color::White, Color::Green),
            HighlightVariant::Warning => (Color::Black, Color::Yellow),
            HighlightVariant::Error => (Color::White, Color::Red),
            HighlightVariant::Info => (Color::White, Color::Cyan),
        };

        CapsuleLabel::padded(self.text, fg, bg).into_element()
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
        let _ = Highlight::primary("P").into_element();
        let _ = Highlight::success("S").into_element();
        let _ = Highlight::warning("W").into_element();
        let _ = Highlight::error("E").into_element();
        let _ = Highlight::info("I").into_element();
    }
}
