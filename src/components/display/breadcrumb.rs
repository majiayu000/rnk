//! Breadcrumb component for navigation paths
//!
//! Displays a navigation path with clickable items.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Breadcrumb;
//!
//! fn app() -> Element {
//!     Breadcrumb::new()
//!         .items(vec!["Home", "Products", "Electronics", "Phones"])
//!         .separator(" > ")
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// A breadcrumb component for navigation paths
#[derive(Debug, Clone)]
pub struct Breadcrumb {
    items: Vec<String>,
    separator: String,
    active_color: Color,
    inactive_color: Color,
}

impl Breadcrumb {
    /// Create a new breadcrumb
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            separator: " / ".to_string(),
            active_color: Color::White,
            inactive_color: Color::BrightBlack,
        }
    }

    /// Set the breadcrumb items
    pub fn items(mut self, items: Vec<impl Into<String>>) -> Self {
        self.items = items.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set the separator between items
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Set the color for the active (last) item
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    /// Set the color for inactive items
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_color = color;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut children = Vec::new();
        let last_idx = self.items.len().saturating_sub(1);

        for (i, item) in self.items.iter().enumerate() {
            let is_last = i == last_idx;

            // Add item
            children.push(
                Text::new(item)
                    .color(if is_last {
                        self.active_color
                    } else {
                        self.inactive_color
                    })
                    .bold()
                    .into_element(),
            );

            // Add separator (except after last item)
            if !is_last {
                children.push(
                    Text::new(&self.separator)
                        .color(Color::BrightBlack)
                        .into_element(),
                );
            }
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .children(children)
            .into_element()
    }
}

impl Default for Breadcrumb {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a breadcrumb from a path string
pub fn breadcrumb_from_path(path: &str) -> Element {
    let items: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    Breadcrumb::new()
        .items(items)
        .separator(" / ")
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breadcrumb_creation() {
        let bc = Breadcrumb::new();
        assert!(bc.items.is_empty());
    }

    #[test]
    fn test_breadcrumb_items() {
        let bc = Breadcrumb::new().items(vec!["Home", "About"]);
        assert_eq!(bc.items.len(), 2);
    }

    #[test]
    fn test_breadcrumb_separator() {
        let bc = Breadcrumb::new().separator(" > ");
        assert_eq!(bc.separator, " > ");
    }

    #[test]
    fn test_breadcrumb_into_element() {
        let bc = Breadcrumb::new().items(vec!["A", "B", "C"]);
        let _ = bc.into_element();
    }

    #[test]
    fn test_breadcrumb_from_path() {
        let _ = breadcrumb_from_path("/home/user/documents");
    }
}
