//! Badge component for status indicators
//!
//! Displays a small badge with text, useful for counts, status, or labels.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Badge;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Row)
//!         .gap(1.0)
//!         .children(vec![
//!             Badge::new("3").variant(BadgeVariant::Primary).into_element(),
//!             Badge::new("New").variant(BadgeVariant::Success).into_element(),
//!             Badge::new("Error").variant(BadgeVariant::Error).into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::capsule::{capsule_padded, capsule_wrapped};
use crate::core::{Color, Element};

/// Badge variant for different styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
    Info,
}

/// A badge component for displaying status or counts
#[derive(Debug, Clone)]
pub struct Badge {
    text: String,
    variant: BadgeVariant,
    pill: bool,
}

impl Badge {
    /// Create a new badge with the given text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BadgeVariant::Default,
            pill: false,
        }
    }

    /// Set the badge variant
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Make the badge pill-shaped (rounded)
    pub fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let (fg, bg) = match self.variant {
            BadgeVariant::Default => (Color::White, Color::Ansi256(240)),
            BadgeVariant::Primary => (Color::White, Color::Blue),
            BadgeVariant::Secondary => (Color::White, Color::Ansi256(245)),
            BadgeVariant::Success => (Color::White, Color::Green),
            BadgeVariant::Warning => (Color::Black, Color::Yellow),
            BadgeVariant::Error => (Color::White, Color::Red),
            BadgeVariant::Info => (Color::White, Color::Cyan),
        };

        let text = if self.pill {
            capsule_padded(self.text, fg, bg)
        } else {
            capsule_wrapped(self.text, fg, bg, "[", "]")
        };

        text.into_element()
    }
}

impl Default for Badge {
    fn default() -> Self {
        Self::new("")
    }
}

/// Create a primary badge
pub fn badge_primary(text: impl Into<String>) -> Element {
    Badge::new(text)
        .variant(BadgeVariant::Primary)
        .into_element()
}

/// Create a success badge
pub fn badge_success(text: impl Into<String>) -> Element {
    Badge::new(text)
        .variant(BadgeVariant::Success)
        .into_element()
}

/// Create an error badge
pub fn badge_error(text: impl Into<String>) -> Element {
    Badge::new(text).variant(BadgeVariant::Error).into_element()
}

/// Create a warning badge
pub fn badge_warning(text: impl Into<String>) -> Element {
    Badge::new(text)
        .variant(BadgeVariant::Warning)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_creation() {
        let badge = Badge::new("5");
        assert_eq!(badge.text, "5");
    }

    #[test]
    fn test_badge_variant() {
        let badge = Badge::new("New").variant(BadgeVariant::Success);
        assert_eq!(badge.variant, BadgeVariant::Success);
    }

    #[test]
    fn test_badge_into_element() {
        let badge = Badge::new("Test").variant(BadgeVariant::Primary);
        let _ = badge.into_element();
    }

    #[test]
    fn test_badge_helpers() {
        let _ = badge_primary("1");
        let _ = badge_success("OK");
        let _ = badge_error("!");
        let _ = badge_warning("?");
    }
}
