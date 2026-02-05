//! Link component for styled hyperlinks
//!
//! Displays styled links with optional icons.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Link;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Column)
//!         .children(vec![
//!             Link::new("Documentation", "https://docs.rs/rnk").into_element(),
//!             Link::new("GitHub", "https://github.com").icon("").into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::Text;
use crate::core::{Color, Element};

/// A link component
#[derive(Debug, Clone)]
pub struct Link {
    text: String,
    url: String,
    icon: Option<String>,
    color: Color,
    underline: bool,
}

impl Link {
    /// Create a new link
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
            icon: None,
            color: Color::Cyan,
            underline: true,
        }
    }

    /// Set an icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Disable underline
    pub fn no_underline(mut self) -> Self {
        self.underline = false;
        self
    }

    /// Get the URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut content = String::new();

        if let Some(icon) = &self.icon {
            content.push_str(icon);
            content.push(' ');
        }

        content.push_str(&self.text);

        let mut text = Text::new(content).color(self.color);
        if self.underline {
            text = text.underline();
        }
        text.into_element()
    }
}

impl Default for Link {
    fn default() -> Self {
        Self::new("", "")
    }
}

/// Create a simple link
pub fn link_styled(text: impl Into<String>, url: impl Into<String>) -> Element {
    Link::new(text, url).into_element()
}

/// Create a link with icon
pub fn link_with_icon(text: impl Into<String>, url: impl Into<String>, icon: impl Into<String>) -> Element {
    Link::new(text, url).icon(icon).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_creation() {
        let l = Link::new("Text", "https://example.com");
        assert_eq!(l.text, "Text");
        assert_eq!(l.url, "https://example.com");
    }

    #[test]
    fn test_link_with_icon() {
        let l = Link::new("GitHub", "https://github.com").icon("");
        assert_eq!(l.icon, Some("".to_string()));
    }

    #[test]
    fn test_link_into_element() {
        let _ = Link::new("Test", "https://test.com").into_element();
        let _ = Link::new("Test", "https://test.com").icon("*").into_element();
        let _ = Link::new("Test", "https://test.com").no_underline().into_element();
    }

    #[test]
    fn test_link_helpers() {
        let _ = link_styled("Link", "https://example.com");
        let _ = link_with_icon("Link", "https://example.com", ">");
    }
}
