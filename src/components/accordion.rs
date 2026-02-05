//! Accordion component for collapsible sections
//!
//! Displays collapsible content sections.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Accordion;
//!
//! fn app() -> Element {
//!     Accordion::new()
//!         .item("Section 1", "Content for section 1")
//!         .item("Section 2", "Content for section 2")
//!         .expanded(0)
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// An accordion item
#[derive(Debug, Clone)]
pub struct AccordionItem {
    title: String,
    content: String,
}

/// An accordion component with collapsible sections
#[derive(Debug, Clone)]
pub struct Accordion {
    items: Vec<AccordionItem>,
    expanded: Option<usize>,
    allow_multiple: bool,
    expanded_set: Vec<usize>,
}

impl Accordion {
    /// Create a new accordion
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            expanded: None,
            allow_multiple: false,
            expanded_set: Vec::new(),
        }
    }

    /// Add an item to the accordion
    pub fn item(mut self, title: impl Into<String>, content: impl Into<String>) -> Self {
        self.items.push(AccordionItem {
            title: title.into(),
            content: content.into(),
        });
        self
    }

    /// Set the expanded item index (single mode)
    pub fn expanded(mut self, index: usize) -> Self {
        self.expanded = Some(index);
        self
    }

    /// Allow multiple items to be expanded
    pub fn allow_multiple(mut self) -> Self {
        self.allow_multiple = true;
        self
    }

    /// Set multiple expanded items
    pub fn expanded_items(mut self, indices: Vec<usize>) -> Self {
        self.expanded_set = indices;
        self.allow_multiple = true;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut children = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            let is_expanded = if self.allow_multiple {
                self.expanded_set.contains(&i)
            } else {
                self.expanded == Some(i)
            };

            // Header
            let indicator = if is_expanded { "▼" } else { "▶" };
            children.push(
                RnkBox::new()
                    .flex_direction(FlexDirection::Row)
                    .padding_x(1.0)
                    .background(Color::Ansi256(238))
                    .children(vec![
                        Text::new(indicator)
                            .color(Color::Cyan)
                            .into_element(),
                        Text::new(format!(" {}", item.title))
                            .color(Color::White)
                            .bold()
                            .into_element(),
                    ])
                    .into_element(),
            );

            // Content (if expanded)
            if is_expanded {
                children.push(
                    RnkBox::new()
                        .padding_x(2.0)
                        .padding_y(0.5)
                        .background(Color::Ansi256(236))
                        .child(
                            Text::new(&item.content)
                                .color(Color::White)
                                .into_element(),
                        )
                        .into_element(),
                );
            }
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .children(children)
            .into_element()
    }
}

impl Default for Accordion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accordion_creation() {
        let acc = Accordion::new();
        assert!(acc.items.is_empty());
    }

    #[test]
    fn test_accordion_items() {
        let acc = Accordion::new()
            .item("Title 1", "Content 1")
            .item("Title 2", "Content 2");
        assert_eq!(acc.items.len(), 2);
    }

    #[test]
    fn test_accordion_expanded() {
        let acc = Accordion::new()
            .item("A", "a")
            .item("B", "b")
            .expanded(0);
        assert_eq!(acc.expanded, Some(0));
    }

    #[test]
    fn test_accordion_into_element() {
        let _ = Accordion::new()
            .item("Section", "Content")
            .expanded(0)
            .into_element();
    }

    #[test]
    fn test_accordion_multiple() {
        let _ = Accordion::new()
            .item("A", "a")
            .item("B", "b")
            .allow_multiple()
            .expanded_items(vec![0, 1])
            .into_element();
    }
}
