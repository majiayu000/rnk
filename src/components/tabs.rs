//! Tabs component for tabbed interfaces
//!
//! Provides a tab bar widget with selectable tabs.

use crate::core::{Color, Element, Style};
use crate::components::{Box as TinkBox, Text, Span, Line};

/// Tab item
#[derive(Debug, Clone)]
pub struct Tab {
    /// Tab title
    pub title: String,
    /// Optional style
    pub style: Option<Style>,
}

impl Tab {
    /// Create a new tab
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            style: None,
        }
    }

    /// Set tab style
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl<T: Into<String>> From<T> for Tab {
    fn from(s: T) -> Self {
        Tab::new(s)
    }
}

/// Tabs component builder
#[derive(Debug, Clone)]
pub struct Tabs {
    /// Tab items
    tabs: Vec<Tab>,
    /// Currently selected tab index
    selected: usize,
    /// Style for selected tab
    highlight_style: Style,
    /// Normal tab style
    normal_style: Style,
    /// Divider between tabs
    divider: String,
    /// Key for reconciliation
    key: Option<String>,
}

impl Tabs {
    /// Create new tabs
    pub fn new() -> Self {
        let mut highlight_style = Style::new();
        highlight_style.bold = true;

        Self {
            tabs: Vec::new(),
            selected: 0,
            highlight_style,
            normal_style: Style::new(),
            divider: " | ".to_string(),
            key: None,
        }
    }

    /// Create tabs from items
    pub fn from_items<I, T>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Tab>,
    {
        let mut tabs = Self::new();
        tabs.tabs = items.into_iter().map(|t| t.into()).collect();
        tabs
    }

    /// Set tabs
    pub fn tabs<I, T>(mut self, tabs: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Tab>,
    {
        self.tabs = tabs.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Add a tab
    pub fn tab(mut self, tab: impl Into<Tab>) -> Self {
        self.tabs.push(tab.into());
        self
    }

    /// Set selected tab index
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Set highlight style for selected tab
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Set highlight color
    pub fn highlight_color(mut self, color: Color) -> Self {
        self.highlight_style.color = Some(color);
        self
    }

    /// Set normal tab style
    pub fn normal_style(mut self, style: Style) -> Self {
        self.normal_style = style;
        self
    }

    /// Set divider between tabs
    pub fn divider(mut self, divider: impl Into<String>) -> Self {
        self.divider = divider.into();
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Get number of tabs
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        let mut spans = Vec::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            // Add divider between tabs (not before first)
            if i > 0 {
                spans.push(Span::new(&self.divider).dim());
            }

            // Create tab span
            let is_selected = i == self.selected;
            let mut span = Span::new(&tab.title);

            if is_selected {
                if let Some(color) = self.highlight_style.color {
                    span = span.color(color);
                }
                if self.highlight_style.bold {
                    span = span.bold();
                }
                if self.highlight_style.underline {
                    span = span.underline();
                }
                if self.highlight_style.inverse {
                    span = span.inverse();
                }
            } else {
                if let Some(color) = self.normal_style.color {
                    span = span.color(color);
                }
                if self.normal_style.dim {
                    span = span.dim();
                }
            }

            spans.push(span);
        }

        let text = Text::line(Line::from_spans(spans));
        let mut container = TinkBox::new()
            .child(text.into_element());

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_creation() {
        let tab = Tab::new("Home");
        assert_eq!(tab.title, "Home");
    }

    #[test]
    fn test_tabs_creation() {
        let tabs = Tabs::from_items(vec!["Home", "Settings", "About"]);
        assert_eq!(tabs.len(), 3);
    }

    #[test]
    fn test_tabs_selected() {
        let tabs = Tabs::from_items(vec!["A", "B", "C"])
            .selected(1);
        assert_eq!(tabs.selected, 1);
    }

    #[test]
    fn test_tabs_divider() {
        let tabs = Tabs::new()
            .tab("Home")
            .tab("Settings")
            .divider(" │ ");
        assert_eq!(tabs.divider, " │ ");
    }
}
