//! List component for displaying selectable lists
//!
//! Provides a scrollable, selectable list widget similar to Ratatui's List.

use crate::core::{Color, Element, Style};
use crate::components::{Box as TinkBox, Text, Span, Line};

/// List item with content and optional styling
#[derive(Debug, Clone)]
pub struct ListItem {
    /// The content of the item (can be rich text)
    pub content: Line,
    /// Custom style for this item
    pub style: Option<Style>,
}

impl ListItem {
    /// Create a new list item from a string
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: Line::raw(content),
            style: None,
        }
    }

    /// Create a list item from a Line (rich text)
    pub fn from_line(line: Line) -> Self {
        Self {
            content: line,
            style: None,
        }
    }

    /// Create a list item from spans
    pub fn from_spans(spans: Vec<Span>) -> Self {
        Self {
            content: Line::from_spans(spans),
            style: None,
        }
    }

    /// Set custom style for this item
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl<T: Into<String>> From<T> for ListItem {
    fn from(s: T) -> Self {
        ListItem::new(s)
    }
}

/// List state for tracking selection and scroll
#[derive(Debug, Clone, Default)]
pub struct ListState {
    /// Currently selected index (None if nothing selected)
    pub selected: Option<usize>,
    /// Scroll offset
    pub offset: usize,
}

impl ListState {
    /// Create a new list state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with initial selection
    pub fn with_selected(selected: Option<usize>) -> Self {
        Self {
            selected,
            offset: 0,
        }
    }

    /// Select the next item
    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1).min(len - 1),
            None => 0,
        });
    }

    /// Select the previous item
    pub fn select_previous(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => i.saturating_sub(1),
            None => 0,
        });
    }

    /// Select first item
    pub fn select_first(&mut self, len: usize) {
        if len > 0 {
            self.selected = Some(0);
        }
    }

    /// Select last item
    pub fn select_last(&mut self, len: usize) {
        if len > 0 {
            self.selected = Some(len - 1);
        }
    }

    /// Select a specific index
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// Adjust scroll offset to keep selection visible
    pub fn scroll_to_selected(&mut self, viewport_height: usize) {
        if let Some(selected) = self.selected {
            // If selection is above viewport, scroll up
            if selected < self.offset {
                self.offset = selected;
            }
            // If selection is below viewport, scroll down
            else if selected >= self.offset + viewport_height {
                self.offset = selected.saturating_sub(viewport_height - 1);
            }
        }
    }
}

/// List component builder
#[derive(Debug, Clone)]
pub struct List {
    /// List items
    items: Vec<ListItem>,
    /// Style for the selected item highlight
    highlight_style: Style,
    /// Symbol shown before selected item
    highlight_symbol: Option<String>,
    /// Whether to show selection
    show_selection: bool,
    /// Key for reconciliation
    key: Option<String>,
}

impl List {
    /// Create a new empty list
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            highlight_style: Style::new(),
            highlight_symbol: None,
            show_selection: true,
            key: None,
        }
    }

    /// Create a list from items
    pub fn from_items<I, T>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<ListItem>,
    {
        Self {
            items: items.into_iter().map(|i| i.into()).collect(),
            highlight_style: Style::new(),
            highlight_symbol: None,
            show_selection: true,
            key: None,
        }
    }

    /// Add an item to the list
    pub fn item(mut self, item: impl Into<ListItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Set all items
    pub fn items<I, T>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<ListItem>,
    {
        self.items = items.into_iter().map(|i| i.into()).collect();
        self
    }

    /// Set the highlight style for selected item
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Set the highlight color
    pub fn highlight_color(mut self, color: Color) -> Self {
        self.highlight_style.color = Some(color);
        self
    }

    /// Set highlight background color
    pub fn highlight_bg(mut self, color: Color) -> Self {
        self.highlight_style.background_color = Some(color);
        self
    }

    /// Set the highlight symbol (e.g., "> " or "* ")
    pub fn highlight_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.highlight_symbol = Some(symbol.into());
        self
    }

    /// Set whether to show selection highlight
    pub fn show_selection(mut self, show: bool) -> Self {
        self.show_selection = show;
        self
    }

    /// Set key for reconciliation
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Render the list with state to an Element
    pub fn render(self, state: &ListState) -> Element {
        self.render_with_height(state, None)
    }

    /// Render the list with a specific viewport height
    pub fn render_with_height(self, state: &ListState, viewport_height: Option<usize>) -> Element {
        let selected = state.selected;
        let offset = state.offset;
        let height = viewport_height.unwrap_or(self.items.len());
        let symbol_width = self.highlight_symbol.as_ref().map(|s| s.len()).unwrap_or(0);

        // Get visible items
        let visible_items: Vec<_> = self.items
            .iter()
            .enumerate()
            .skip(offset)
            .take(height)
            .collect();

        // Build list element
        let mut container = TinkBox::new()
            .flex_direction(crate::core::FlexDirection::Column);

        if let Some(key) = self.key {
            container = container.key(key);
        }

        for (idx, item) in visible_items {
            let is_selected = self.show_selection && selected == Some(idx);

            // Build the item content
            let mut spans = Vec::new();

            // Add highlight symbol if configured
            if let Some(ref symbol) = self.highlight_symbol {
                if is_selected {
                    spans.push(Span::new(symbol.clone()));
                } else {
                    // Add padding to align non-selected items
                    spans.push(Span::new(" ".repeat(symbol_width)));
                }
            }

            // Add item content spans
            spans.extend(item.content.spans.iter().cloned());

            // Apply styling
            let line = Line::from_spans(spans);
            let mut text = Text::line(line);

            if is_selected {
                if let Some(color) = self.highlight_style.color {
                    text = text.color(color);
                }
                if let Some(bg) = self.highlight_style.background_color {
                    text = text.background(bg);
                }
                if self.highlight_style.bold {
                    text = text.bold();
                }
                if self.highlight_style.inverse {
                    text = text.inverse();
                }
            } else if let Some(ref item_style) = item.style
                && let Some(color) = item_style.color {
                    text = text.color(color);
                }

            container = container.child(text.into_element());
        }

        container.into_element()
    }

    /// Convert to element (no selection)
    pub fn into_element(self) -> Element {
        self.render(&ListState::new())
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_item_creation() {
        let item = ListItem::new("Test item");
        assert_eq!(item.content.spans[0].content, "Test item");
    }

    #[test]
    fn test_list_creation() {
        let list = List::from_items(vec!["Item 1", "Item 2", "Item 3"]);
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_list_state_navigation() {
        let mut state = ListState::new();

        state.select_next(5);
        assert_eq!(state.selected, Some(0));

        state.select_next(5);
        assert_eq!(state.selected, Some(1));

        state.select_previous(5);
        assert_eq!(state.selected, Some(0));

        state.select_previous(5);
        assert_eq!(state.selected, Some(0)); // Can't go below 0
    }

    #[test]
    fn test_list_state_first_last() {
        let mut state = ListState::new();

        state.select_last(10);
        assert_eq!(state.selected, Some(9));

        state.select_first(10);
        assert_eq!(state.selected, Some(0));
    }

    #[test]
    fn test_scroll_to_selected() {
        let mut state = ListState::with_selected(Some(15));
        state.scroll_to_selected(10);
        assert_eq!(state.offset, 6); // 15 - (10 - 1) = 6
    }
}
