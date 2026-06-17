//! SelectInput component with built-in keyboard navigation
//!
//! A selection component similar to Ink's ink-select-input that handles
//! keyboard navigation internally.

use crate::components::Box as RnkBox;
use crate::components::navigation::{NavigationConfig, handle_list_navigation};
use crate::components::selection_list::{ListStyle, indicator_padding, render_list};
use crate::components::{InteractionMode, InteractionOutcome};
use crate::core::{Color, Element};
use crate::hooks::{Signal, use_input, use_signal};

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

/// State for controlled SelectInput usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectInputState {
    /// Currently highlighted item index.
    highlighted: usize,
    /// Last submitted item index.
    submitted: Option<usize>,
    /// Whether the interaction was cancelled.
    cancelled: bool,
}

impl SelectInputState {
    /// Create select state with an initial highlighted index.
    pub fn new(highlighted: usize) -> Self {
        Self {
            highlighted,
            submitted: None,
            cancelled: false,
        }
    }

    /// Get the highlighted item index.
    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    /// Set the highlighted item index, clamped to item count.
    pub fn set_highlighted(&mut self, index: usize, item_count: usize) {
        self.highlighted = index.min(item_count.saturating_sub(1));
    }

    /// Get the last submitted item index.
    pub fn submitted(&self) -> Option<usize> {
        self.submitted
    }

    /// Return true when input cancelled this select.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
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
        self.indicator_padding = indicator_padding(&ind);
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
    /// Input mode for disabled/read-only behavior.
    mode: InteractionMode,
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
            mode: InteractionMode::Enabled,
        }
    }

    /// Create from an iterator of items
    pub fn from_items<I>(iter: I) -> Self
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

    /// Enable normal navigation and submit behavior.
    pub fn enabled(mut self) -> Self {
        self.mode = InteractionMode::Enabled;
        self
    }

    /// Ignore all input.
    pub fn disabled(mut self) -> Self {
        self.mode = InteractionMode::Disabled;
        self
    }

    /// Allow highlight navigation but block submit.
    pub fn read_only(mut self) -> Self {
        self.mode = InteractionMode::ReadOnly;
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
            return RnkBox::new().into_element();
        }

        let items = self.items.clone();
        let initial_highlighted = self.highlighted;
        let limit = self.limit;
        let style = self.style.clone();
        let is_focused = self.is_focused;
        let vim_navigation = self.vim_navigation;
        let number_shortcuts = self.number_shortcuts;
        let mode = self.mode;

        // Create signal for interaction state
        let state_signal = use_signal(|| SelectInputState::new(initial_highlighted));

        // Set up input handling if focused
        if is_focused {
            let items_len = items.len();
            let state_for_input = state_signal.clone();

            use_input(move |input, key| {
                let config = NavigationConfig::new()
                    .vim_navigation(vim_navigation)
                    .number_shortcuts(number_shortcuts);

                let mut next = state_for_input.get();
                let outcome = handle_select_input(&mut next, items_len, input, key, &config, mode);
                if outcome.is_handled() {
                    state_for_input.set(next);
                }
            });
        }

        // Render the list
        render_select_list(&items, state_signal, limit, &style)
    }
}

/// Handle SelectInput navigation, submit, and cancel against explicit state.
pub fn handle_select_input(
    state: &mut SelectInputState,
    item_count: usize,
    input: &str,
    key: &crate::hooks::Key,
    config: &NavigationConfig,
    mode: InteractionMode,
) -> InteractionOutcome<usize> {
    if mode.is_disabled() || item_count == 0 {
        return InteractionOutcome::Ignored;
    }

    state.set_highlighted(state.highlighted, item_count);

    if key.escape {
        state.cancelled = true;
        return InteractionOutcome::Cancelled;
    }

    let current = state.highlighted;
    let result = handle_list_navigation(current, item_count, input, *key, config);
    if result.is_moved() {
        let new_pos = result.unwrap_or(current);
        if new_pos != current {
            state.highlighted = new_pos;
        }
        return InteractionOutcome::Handled;
    }

    if mode.is_read_only() {
        return InteractionOutcome::Ignored;
    }

    if key.return_key || key.space {
        state.submitted = Some(state.highlighted);
        return InteractionOutcome::Submitted(state.highlighted);
    }

    InteractionOutcome::Ignored
}

impl ListStyle for SelectInputStyle {
    fn highlight_color(&self) -> Option<Color> {
        self.highlight_color
    }

    fn highlight_bg(&self) -> Option<Color> {
        self.highlight_bg
    }

    fn highlight_bold(&self) -> bool {
        self.highlight_bold
    }

    fn indicator(&self) -> &str {
        &self.indicator
    }

    fn indicator_padding(&self) -> &str {
        &self.indicator_padding
    }

    fn item_color(&self) -> Option<Color> {
        self.item_color
    }
}

/// Render the select list as an Element
fn render_select_list<T: Clone + 'static>(
    items: &[SelectItem<T>],
    state_signal: Signal<SelectInputState>,
    limit: Option<usize>,
    style: &SelectInputStyle,
) -> Element {
    let highlighted = state_signal.get().highlighted();

    render_list(
        items,
        highlighted,
        limit,
        style,
        |item, _idx, _is_highlighted, prefix| format!("{}{}", prefix, item.label),
        |_item, _idx, style, _is_highlighted, mut text| {
            if let Some(color) = style.item_color() {
                text = text.color(color);
            }
            text
        },
    )
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
    fn test_select_input_from_items() {
        let items = vec![SelectItem::new("A", 'a'), SelectItem::new("B", 'b')];
        let select = SelectInput::from_items(items);
        assert_eq!(select.len(), 2);
    }

    #[test]
    fn test_handle_select_input_navigation_submit_and_cancel() {
        let mut state = SelectInputState::new(0);
        let config = NavigationConfig::new().vim_navigation(true);

        let outcome = handle_select_input(
            &mut state,
            3,
            "j",
            &crate::hooks::Key::default(),
            &config,
            InteractionMode::Enabled,
        );
        assert_eq!(outcome, InteractionOutcome::Handled);
        assert_eq!(state.highlighted(), 1);

        let outcome = handle_select_input(
            &mut state,
            3,
            "",
            &crate::hooks::Key {
                return_key: true,
                ..Default::default()
            },
            &config,
            InteractionMode::Enabled,
        );
        assert_eq!(outcome, InteractionOutcome::Submitted(1));
        assert_eq!(state.submitted(), Some(1));

        let outcome = handle_select_input(
            &mut state,
            3,
            "",
            &crate::hooks::Key {
                escape: true,
                ..Default::default()
            },
            &config,
            InteractionMode::Enabled,
        );
        assert_eq!(outcome, InteractionOutcome::Cancelled);
        assert!(state.is_cancelled());
    }

    #[test]
    fn test_handle_select_input_modes() {
        let config = NavigationConfig::new().vim_navigation(true);
        let mut state = SelectInputState::new(0);

        let outcome = handle_select_input(
            &mut state,
            3,
            "j",
            &crate::hooks::Key::default(),
            &config,
            InteractionMode::Disabled,
        );
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert_eq!(state.highlighted(), 0);

        let outcome = handle_select_input(
            &mut state,
            3,
            "j",
            &crate::hooks::Key::default(),
            &config,
            InteractionMode::ReadOnly,
        );
        assert_eq!(outcome, InteractionOutcome::Handled);
        assert_eq!(state.highlighted(), 1);

        let outcome = handle_select_input(
            &mut state,
            3,
            "",
            &crate::hooks::Key {
                return_key: true,
                ..Default::default()
            },
            &config,
            InteractionMode::ReadOnly,
        );
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert_eq!(state.submitted(), None);
    }
}
