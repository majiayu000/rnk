//! Empty state component for when there's no data
//!
//! Displays a placeholder when lists or content are empty.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::EmptyState;
//!
//! fn app() -> Element {
//!     let items = use_signal(|| Vec::<String>::new());
//!
//!     if items.get().is_empty() {
//!         EmptyState::new()
//!             .icon("📭")
//!             .title("No items")
//!             .description("Add some items to get started")
//!             .into_element()
//!     } else {
//!         // Render items...
//!     }
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{AlignItems, Color, Element, FlexDirection, JustifyContent};

/// An empty state component
#[derive(Debug, Clone)]
pub struct EmptyState {
    icon: Option<String>,
    title: String,
    description: Option<String>,
}

impl EmptyState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            icon: None,
            title: "No data".to_string(),
            description: None,
        }
    }

    /// Set the icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut children = Vec::new();

        // Icon
        if let Some(icon) = self.icon {
            children.push(Text::new(icon).color(Color::BrightBlack).into_element());
        }

        // Title
        children.push(
            Text::new(&self.title)
                .color(Color::White)
                .bold()
                .into_element(),
        );

        // Description
        if let Some(desc) = self.description {
            children.push(Text::new(desc).color(Color::BrightBlack).into_element());
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center)
            .padding(2)
            .gap(0.5)
            .children(children)
            .into_element()
    }
}

impl Default for EmptyState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state_creation() {
        let es = EmptyState::new();
        assert_eq!(es.title, "No data");
    }

    #[test]
    fn test_empty_state_with_content() {
        let es = EmptyState::new()
            .icon("📭")
            .title("Empty")
            .description("Nothing here");
        assert_eq!(es.icon, Some("📭".to_string()));
        assert_eq!(es.title, "Empty");
        assert_eq!(es.description, Some("Nothing here".to_string()));
    }

    #[test]
    fn test_empty_state_into_element() {
        let _ = EmptyState::new().into_element();
        let _ = EmptyState::new()
            .icon("🔍")
            .title("No results")
            .description("Try a different search")
            .into_element();
    }
}
