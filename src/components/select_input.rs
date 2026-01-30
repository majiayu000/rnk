//! SelectInput component with built-in keyboard navigation
//!
//! A selection component similar to Ink's ink-select-input that handles
//! keyboard navigation internally.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};
use crate::hooks::{use_input, use_signal, Signal};

/// A selectable item in the SelectInput
#[derive(Debug, Clone)]
pub struct SelectItem<T: Clone> {
    /// Display label for the item
    pub label: String,
    /// Value associated with the item
    pub value: T,
}

impl<T: Clone> SelectItem<T> {
    /// Create a new select item
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

impl<T: Clone + ToString> From<T> for SelectItem<T> {
    fn from(value: T) -> Self {
        Self {
            label: value.to_string(),
            value,
        }
    }
}

/// Configuration for SelectInput appearance
#[derive(Debug, Clone)]
pub struct SelectInputStyle {
    /// Color for the selected/highlighted item
    pub highlight_color: Option<Color>,
    /// Background color for the selected item
    pub highlight_bg: Option<Color>,
    /// Whether to show the selected item in bold
    pub highlight_bold: bool,
    /// Indicator shown before the selected item
    pub indicator: String,
    /// Indicator shown before unselected items (for alignment)
    pub indicator_padding: String,
    /// Color for unselected items
    pub item_color: Option<Color>,
}

impl Default for SelectInputStyle {
    fn default() -> Self {
        Self {
            highlight_color: Some(Color::Cyan),
            highlight_bg: None,
            highlight_bold: true,
            indicator: "❯ ".to_string(),
            indicator_padding: "  ".to_string(),
            item_color: None,
        }
    }
}

impl SelectInputStyle {
    /// Create a new style with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the highlight color
    pub fn highlight_color(mut self, color: Color) -> Self {
        self.highlight_color = Some(color);
        self
    }

    /// Set the highlight background color
    pub fn highlight_bg(mut self, color: Color) -> Self {
        self.highlight_bg = Some(color);
        self
    }

    /// Set whether to bold the highlighted item
    pub fn highlight_bold(mut self, bold: bool) -> Self {
        self.highlight_bold = bold;
        self
    }

    /// Set the indicator string
    pub fn indicator(mut self, indicator: impl Into<String>) -> Self {
        let ind = indicator.into();
        self.indicator_padding = " ".repeat(ind.chars().count());
        self.indicator = ind;
        self
    }

    /// Set the item color
    pub fn item_color(mut self, color: Color) -> Self {
        self.item_color = Some(color);
        self
    }
}

/// SelectInput component with built-in keyboard navigation
///
/// # Example
///
/// ```ignore
/// use rnk::components::{SelectInput, SelectItem};
///
/// let items = vec![
///     SelectItem::new("Option 1", 1),
///     SelectItem::new("Option 2", 2),
///     SelectItem::new("Option 3", 3),
/// ];
///
/// SelectInput::new(items)
///     .into_element()
/// ```
pub struct SelectInput<T: Clone + 'static> {
    /// Items to display
    items: Vec<SelectItem<T>>,
    /// Currently highlighted index
    highlighted: usize,
    /// Maximum number of visible items (None = show all)
    limit: Option<usize>,
    /// Style configuration
    style: SelectInputStyle,
    /// Whether the component is focused (receives input)
    is_focused: bool,
    /// Whether to enable vim-style navigation (j/k)
    vim_navigation: bool,
    /// Whether to enable number key shortcuts
    number_shortcuts: bool,
}

impl<T: Clone + 'static> SelectInput<T> {
    /// Create a new SelectInput with items
    pub fn new(items: Vec<SelectItem<T>>) -> Self {
        Self {
            items,
            highlighted: 0,
            limit: None,
            style: SelectInputStyle::default(),
            is_focused: true,
            vim_navigation: true,
            number_shortcuts: true,
        }
    }

    /// Create from an iterator of items
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = SelectItem<T>>,
    {
        Self::new(iter.into_iter().collect())
    }

    /// Set the initially highlighted index
    pub fn highlighted(mut self, index: usize) -> Self {
        self.highlighted = index.min(self.items.len().saturating_sub(1));
        self
    }

    /// Set the maximum number of visible items
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the style configuration
    pub fn style(mut self, style: SelectInputStyle) -> Self {
        self.style = style;
        self
    }

    /// Set whether the component is focused
    pub fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    /// Enable or disable vim-style navigation (j/k keys)
    pub fn vim_navigation(mut self, enabled: bool) -> Self {
        self.vim_navigation = enabled;
        self
    }

    /// Enable or disable number key shortcuts (1-9)
    pub fn number_shortcuts(mut self, enabled: bool) -> Self {
        self.number_shortcuts = enabled;
        self
    }

    /// Set highlight color
    pub fn highlight_color(mut self, color: Color) -> Self {
        self.style.highlight_color = Some(color);
        self
    }

    /// Set indicator string
    pub fn indicator(mut self, indicator: impl Into<String>) -> Self {
        self.style = self.style.indicator(indicator);
        self
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Convert to element with internal state management
    pub fn into_element(self) -> Element {
        if self.items.is_empty() {
            return TinkBox::new().into_element();
        }

        let items = self.items.clone();
        let initial_highlighted = self.highlighted;
        let limit = self.limit;
        let style = self.style.clone();
        let is_focused = self.is_focused;
        let vim_navigation = self.vim_navigation;
        let number_shortcuts = self.number_shortcuts;

        // Create signal for highlighted index
        let highlighted_signal = use_signal(|| initial_highlighted);

        // Set up input handling if focused
        if is_focused {
            let items_len = items.len();
            let highlighted_for_input = highlighted_signal.clone();

            use_input(move |input, key| {
                let current = highlighted_for_input.get();
                let mut new_highlighted = current;

                // Arrow key navigation
                if key.up_arrow {
                    new_highlighted = current.saturating_sub(1);
                } else if key.down_arrow {
                    new_highlighted = (current + 1).min(items_len.saturating_sub(1));
                }
                // Vim-style navigation
                else if vim_navigation {
                    if input == "k" {
                        new_highlighted = current.saturating_sub(1);
                    } else if input == "j" {
                        new_highlighted = (current + 1).min(items_len.saturating_sub(1));
                    }
                }
                // Number shortcuts (1-9)
                if number_shortcuts {
                    if let Some(num) = input.chars().next().and_then(|c| c.to_digit(10)) {
                        if num >= 1 && num <= 9 {
                            let index = (num as usize) - 1;
                            if index < items_len {
                                new_highlighted = index;
                            }
                        }
                    }
                }

                // Home/End navigation
                if key.home {
                    new_highlighted = 0;
                } else if key.end {
                    new_highlighted = items_len.saturating_sub(1);
                }

                // Page up/down
                if key.page_up {
                    new_highlighted = current.saturating_sub(5);
                } else if key.page_down {
                    new_highlighted = (current + 5).min(items_len.saturating_sub(1));
                }

                // Update highlighted if changed
                if new_highlighted != current {
                    highlighted_for_input.set(new_highlighted);
                }
            });
        }

        // Render the list
        render_select_list(&items, highlighted_signal, limit, &style)
    }
}

/// Render the select list as an Element
fn render_select_list<T: Clone + 'static>(
    items: &[SelectItem<T>],
    highlighted_signal: Signal<usize>,
    limit: Option<usize>,
    style: &SelectInputStyle,
) -> Element {
    let highlighted = highlighted_signal.get();
    let total_items = items.len();

    // Calculate visible range
    let (start, end) = if let Some(limit) = limit {
        let half = limit / 2;
        let start = if highlighted <= half {
            0
        } else if highlighted >= total_items.saturating_sub(half) {
            total_items.saturating_sub(limit)
        } else {
            highlighted.saturating_sub(half)
        };
        let end = (start + limit).min(total_items);
        (start, end)
    } else {
        (0, total_items)
    };

    let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

    for (idx, item) in items.iter().enumerate().skip(start).take(end - start) {
        let is_highlighted = idx == highlighted;

        let prefix = if is_highlighted {
            &style.indicator
        } else {
            &style.indicator_padding
        };

        let label = format!("{}{}", prefix, item.label);
        let mut text = Text::new(&label);

        if is_highlighted {
            if let Some(color) = style.highlight_color {
                text = text.color(color);
            }
            if let Some(bg) = style.highlight_bg {
                text = text.background(bg);
            }
            if style.highlight_bold {
                text = text.bold();
            }
        } else if let Some(color) = style.item_color {
            text = text.color(color);
        }

        container = container.child(text.into_element());
    }

    container.into_element()
}

/// Create a simple SelectInput from string items
pub fn select_input<T: Clone + ToString + 'static>(items: Vec<T>) -> SelectInput<T> {
    let select_items: Vec<SelectItem<T>> = items
        .into_iter()
        .map(|item| SelectItem::new(item.to_string(), item))
        .collect();
    SelectInput::new(select_items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_item_creation() {
        let item = SelectItem::new("Test", 42);
        assert_eq!(item.label, "Test");
        assert_eq!(item.value, 42);
    }

    #[test]
    fn test_select_input_creation() {
        let items = vec![
            SelectItem::new("One", 1),
            SelectItem::new("Two", 2),
            SelectItem::new("Three", 3),
        ];
        let select = SelectInput::new(items);
        assert_eq!(select.len(), 3);
        assert!(!select.is_empty());
    }

    #[test]
    fn test_select_input_empty() {
        let select: SelectInput<i32> = SelectInput::new(vec![]);
        assert!(select.is_empty());
        assert_eq!(select.len(), 0);
    }

    #[test]
    fn test_select_input_highlighted() {
        let items = vec![
            SelectItem::new("One", 1),
            SelectItem::new("Two", 2),
            SelectItem::new("Three", 3),
        ];
        let select = SelectInput::new(items).highlighted(1);
        assert_eq!(select.highlighted, 1);
    }

    #[test]
    fn test_select_input_highlighted_bounds() {
        let items = vec![SelectItem::new("One", 1), SelectItem::new("Two", 2)];
        let select = SelectInput::new(items).highlighted(10);
        assert_eq!(select.highlighted, 1); // Clamped to max index
    }

    #[test]
    fn test_select_input_style() {
        let style = SelectInputStyle::new()
            .highlight_color(Color::Green)
            .indicator("> ");

        assert_eq!(style.highlight_color, Some(Color::Green));
        assert_eq!(style.indicator, "> ");
        assert_eq!(style.indicator_padding, "  ");
    }

    #[test]
    fn test_select_input_limit() {
        let items = vec![
            SelectItem::new("One", 1),
            SelectItem::new("Two", 2),
            SelectItem::new("Three", 3),
            SelectItem::new("Four", 4),
            SelectItem::new("Five", 5),
        ];
        let select = SelectInput::new(items).limit(3);
        assert_eq!(select.limit, Some(3));
    }

    #[test]
    fn test_select_input_builder_chain() {
        let items = vec![SelectItem::new("Test", 1)];
        let select = SelectInput::new(items)
            .highlighted(0)
            .limit(5)
            .focused(true)
            .vim_navigation(true)
            .number_shortcuts(false)
            .highlight_color(Color::Yellow)
            .indicator("→ ");

        assert_eq!(select.highlighted, 0);
        assert_eq!(select.limit, Some(5));
        assert!(select.is_focused);
        assert!(select.vim_navigation);
        assert!(!select.number_shortcuts);
    }

    #[test]
    fn test_select_input_from_iter() {
        let items = vec![SelectItem::new("A", 'a'), SelectItem::new("B", 'b')];
        let select = SelectInput::from_iter(items);
        assert_eq!(select.len(), 2);
    }
}
