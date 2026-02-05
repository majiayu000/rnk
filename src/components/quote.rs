//! Quote component for displaying quotations
//!
//! Displays styled quotations with optional attribution.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Quote;
//!
//! fn app() -> Element {
//!     Quote::new("The only way to do great work is to love what you do.")
//!         .author("Steve Jobs")
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// A quote component
#[derive(Debug, Clone)]
pub struct Quote {
    text: String,
    author: Option<String>,
    source: Option<String>,
    style: QuoteStyle,
}

/// Quote display style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuoteStyle {
    #[default]
    Block,
    Inline,
    Fancy,
}

impl Quote {
    /// Create a new quote
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            author: None,
            source: None,
            style: QuoteStyle::Block,
        }
    }

    /// Set the author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the style
    pub fn style(mut self, style: QuoteStyle) -> Self {
        self.style = style;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        match self.style {
            QuoteStyle::Block => self.render_block(),
            QuoteStyle::Inline => self.render_inline(),
            QuoteStyle::Fancy => self.render_fancy(),
        }
    }

    fn render_block(self) -> Element {
        let mut children = Vec::new();

        // Quote text with bar
        children.push(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .children(vec![
                    Text::new("│ ")
                        .color(Color::Cyan)
                        .into_element(),
                    Text::new(&self.text)
                        .color(Color::White)
                        .italic()
                        .into_element(),
                ])
                .into_element(),
        );

        // Attribution
        if self.author.is_some() || self.source.is_some() {
            let mut attr = String::from("  — ");
            if let Some(author) = &self.author {
                attr.push_str(author);
            }
            if let Some(source) = &self.source {
                if self.author.is_some() {
                    attr.push_str(", ");
                }
                attr.push_str(source);
            }
            children.push(
                Text::new(attr)
                    .color(Color::BrightBlack)
                    .into_element(),
            );
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .padding_y(0.5)
            .children(children)
            .into_element()
    }

    fn render_inline(self) -> Element {
        let mut content = format!("\"{}\"", self.text);
        if let Some(author) = &self.author {
            content.push_str(&format!(" — {}", author));
        }
        Text::new(content).italic().into_element()
    }

    fn render_fancy(self) -> Element {
        let mut children = Vec::new();

        // Opening quote
        children.push(
            Text::new("\u{201C}")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        );

        // Quote text
        children.push(
            Text::new(&self.text)
                .color(Color::White)
                .italic()
                .into_element(),
        );

        // Closing quote
        children.push(
            Text::new("\u{201D}")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        );

        // Attribution
        if let Some(author) = &self.author {
            children.push(
                Text::new(format!(" — {}", author))
                    .color(Color::BrightBlack)
                    .into_element(),
            );
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .children(children)
            .into_element()
    }
}

impl Default for Quote {
    fn default() -> Self {
        Self::new("")
    }
}

/// Create a simple quote
pub fn quote(text: impl Into<String>) -> Element {
    Quote::new(text).into_element()
}

/// Create a quote with author
pub fn quote_with_author(text: impl Into<String>, author: impl Into<String>) -> Element {
    Quote::new(text).author(author).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_creation() {
        let q = Quote::new("Test quote");
        assert_eq!(q.text, "Test quote");
    }

    #[test]
    fn test_quote_with_author() {
        let q = Quote::new("Quote").author("Author");
        assert_eq!(q.author, Some("Author".to_string()));
    }

    #[test]
    fn test_quote_styles() {
        let _ = Quote::new("Test").style(QuoteStyle::Block).into_element();
        let _ = Quote::new("Test").style(QuoteStyle::Inline).into_element();
        let _ = Quote::new("Test").style(QuoteStyle::Fancy).into_element();
    }

    #[test]
    fn test_quote_helpers() {
        let _ = quote("Simple quote");
        let _ = quote_with_author("Quote", "Author");
    }
}
