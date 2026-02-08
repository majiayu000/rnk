//! Card component for content containers
//!
//! A flexible card component for grouping related content.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Card;
//!
//! fn app() -> Element {
//!     Card::new()
//!         .title("User Profile")
//!         .body("John Doe\njohn@example.com")
//!         .footer("Last login: 2 hours ago")
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{BorderStyle, Color, Element, FlexDirection};

/// A card component
#[derive(Debug, Clone)]
pub struct Card {
    title: Option<String>,
    body: Option<String>,
    footer: Option<String>,
    border_color: Color,
    width: Option<u16>,
}

impl Card {
    /// Create a new card
    pub fn new() -> Self {
        Self {
            title: None,
            body: None,
            footer: None,
            border_color: Color::BrightBlack,
            width: None,
        }
    }

    /// Set the card title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the card body
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the card footer
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Set the border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Set the card width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut children = Vec::new();

        // Title
        if let Some(title) = &self.title {
            children.push(
                RnkBox::new()
                    .padding_x(1.0)
                    .background(Color::Ansi256(238))
                    .child(Text::new(title).color(Color::White).bold().into_element())
                    .into_element(),
            );
        }

        // Body
        if let Some(body) = &self.body {
            children.push(
                RnkBox::new()
                    .padding(1)
                    .child(Text::new(body).color(Color::White).into_element())
                    .into_element(),
            );
        }

        // Footer
        if let Some(footer) = &self.footer {
            children.push(
                RnkBox::new()
                    .padding_x(1.0)
                    .background(Color::Ansi256(236))
                    .child(Text::new(footer).color(Color::BrightBlack).into_element())
                    .into_element(),
            );
        }

        let mut card = RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .border_style(BorderStyle::Round)
            .border_color(self.border_color)
            .children(children);

        if let Some(w) = self.width {
            card = card.width(w);
        }

        card.into_element()
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a simple card with title and body
pub fn card(title: impl Into<String>, body: impl Into<String>) -> Element {
    Card::new().title(title).body(body).into_element()
}

/// Create a card with all sections
pub fn card_full(
    title: impl Into<String>,
    body: impl Into<String>,
    footer: impl Into<String>,
) -> Element {
    Card::new()
        .title(title)
        .body(body)
        .footer(footer)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_creation() {
        let c = Card::new();
        assert!(c.title.is_none());
    }

    #[test]
    fn test_card_with_content() {
        let c = Card::new().title("Title").body("Body").footer("Footer");
        assert_eq!(c.title, Some("Title".to_string()));
        assert_eq!(c.body, Some("Body".to_string()));
        assert_eq!(c.footer, Some("Footer".to_string()));
    }

    #[test]
    fn test_card_into_element() {
        let _ = Card::new().title("Test").body("Content").into_element();
    }

    #[test]
    fn test_card_helpers() {
        let _ = card("Title", "Body");
        let _ = card_full("Title", "Body", "Footer");
    }
}
