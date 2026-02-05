//! Chip component for selectable tags
//!
//! Interactive chips that can be selected/deselected.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Chip;
//!
//! fn app() -> Element {
//!     let selected = use_signal(|| false);
//!
//!     Chip::new("Option")
//!         .selected(selected.get())
//!         .into_element()
//! }
//! ```

use crate::components::Text;
use crate::core::{Color, Element};

/// A chip component for selectable options
#[derive(Debug, Clone)]
pub struct Chip {
    label: String,
    selected: bool,
    disabled: bool,
    icon: Option<String>,
}

impl Chip {
    /// Create a new chip
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            disabled: false,
            icon: None,
        }
    }

    /// Set the selected state
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set the disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add an icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let (fg, bg) = if self.disabled {
            (Color::BrightBlack, Color::Ansi256(238))
        } else if self.selected {
            (Color::White, Color::Blue)
        } else {
            (Color::White, Color::Ansi256(240))
        };

        let mut content = String::new();

        // Selected indicator
        if self.selected {
            content.push_str("● ");
        } else {
            content.push_str("○ ");
        }

        // Icon
        if let Some(icon) = &self.icon {
            content.push_str(icon);
            content.push(' ');
        }

        // Label
        content.push_str(&self.label);

        Text::new(format!(" {} ", content))
            .color(fg)
            .background(bg)
            .into_element()
    }
}

impl Default for Chip {
    fn default() -> Self {
        Self::new("")
    }
}

/// Create a chip
pub fn chip(label: impl Into<String>) -> Element {
    Chip::new(label).into_element()
}

/// Create a selected chip
pub fn chip_selected(label: impl Into<String>) -> Element {
    Chip::new(label).selected(true).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_creation() {
        let c = Chip::new("Test");
        assert_eq!(c.label, "Test");
        assert!(!c.selected);
    }

    #[test]
    fn test_chip_selected() {
        let c = Chip::new("Test").selected(true);
        assert!(c.selected);
    }

    #[test]
    fn test_chip_disabled() {
        let c = Chip::new("Test").disabled(true);
        assert!(c.disabled);
    }

    #[test]
    fn test_chip_into_element() {
        let _ = Chip::new("Test").into_element();
        let _ = Chip::new("Test").selected(true).into_element();
        let _ = Chip::new("Test").disabled(true).into_element();
    }

    #[test]
    fn test_chip_helpers() {
        let _ = chip("Option");
        let _ = chip_selected("Selected");
    }
}
