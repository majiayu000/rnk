//! MultiSelect component for selecting multiple items
//!
//! A multi-selection component similar to Ink's ink-multi-select that handles
//! keyboard navigation and selection internally.

use crate::components::Box as RnkBox;
use crate::components::navigation::{NavigationConfig, handle_list_navigation};
use crate::components::selection_list::{ListStyle, indicator_padding, render_list};
use crate::components::{InteractionMode, InteractionOutcome};
use crate::core::{Color, Element};
use crate::hooks::{use_input, use_signal};

/// A selectable item in the MultiSelect
#[derive(Debug, Clone)]
pub struct MultiSelectItem<T: Clone> {
    /// Display label for the item
    pub label: String,
    /// Value associated with the item
    pub value: T,
    /// Whether this item is initially selected
    pub selected: bool,
}

impl<T: Clone> MultiSelectItem<T> {
    /// Create a new multi-select item
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
            selected: false,
        }
    }

    /// Create a new item that is initially selected
    pub fn selected(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
            selected: true,
        }
    }

    /// Set whether this item is selected
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// State for controlled MultiSelect usage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MultiSelectState {
    /// Currently highlighted item index.
    highlighted: usize,
    /// Per-item selected flags.
    selected: Vec<bool>,
    /// Whether the interaction was cancelled.
    cancelled: bool,
}

impl MultiSelectState {
    /// Create multi-select state from a highlighted index and selected flags.
    pub fn new(highlighted: usize, selected: Vec<bool>) -> Self {
        Self {
            highlighted,
            selected,
            cancelled: false,
        }
    }

    /// Create multi-select state from item defaults.
    pub fn from_items<T: Clone>(items: &[MultiSelectItem<T>], highlighted: usize) -> Self {
        Self::new(
            highlighted,
            items.iter().map(|item| item.selected).collect(),
        )
    }

    /// Get the highlighted item index.
    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    /// Set the highlighted item index, clamped to item count.
    pub fn set_highlighted(&mut self, index: usize, item_count: usize) {
        self.highlighted = index.min(item_count.saturating_sub(1));
    }

    /// Get selected flags.
    pub fn selected_flags(&self) -> &[bool] {
        &self.selected
    }

    /// Get selected item indices.
    pub fn selected_indices(&self) -> Vec<usize> {
        self.selected
            .iter()
            .enumerate()
            .filter_map(|(idx, selected)| selected.then_some(idx))
            .collect()
    }

    /// Return true when input cancelled this multi-select.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    fn ensure_len(&mut self, item_count: usize) {
        self.selected.resize(item_count, false);
        self.set_highlighted(self.highlighted, item_count);
    }
}

/// Configuration for MultiSelect appearance
#[derive(Debug, Clone)]
pub struct MultiSelectStyle {
    /// Color for the highlighted item
    pub highlight_color: Option<Color>,
    /// Background color for the highlighted item
    pub highlight_bg: Option<Color>,
    /// Whether to show the highlighted item in bold
    pub highlight_bold: bool,
    /// Indicator shown before the highlighted item
    pub indicator: String,
    /// Indicator shown before non-highlighted items
    pub indicator_padding: String,
    /// Checkbox for selected items
    pub checkbox_selected: String,
    /// Checkbox for unselected items
    pub checkbox_unselected: String,
    /// Color for selected items
    pub selected_color: Option<Color>,
    /// Color for unselected items
    pub item_color: Option<Color>,
}

impl Default for MultiSelectStyle {
    fn default() -> Self {
        Self {
            highlight_color: Some(Color::Cyan),
            highlight_bg: None,
            highlight_bold: true,
            indicator: "❯ ".to_string(),
            indicator_padding: "  ".to_string(),
            checkbox_selected: "◉ ".to_string(),
            checkbox_unselected: "◯ ".to_string(),
            selected_color: Some(Color::Green),
            item_color: None,
        }
    }
}

impl MultiSelectStyle {
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

    /// Set the checkbox characters
    pub fn checkboxes(
        mut self,
        selected: impl Into<String>,
        unselected: impl Into<String>,
    ) -> Self {
        self.checkbox_selected = selected.into();
        self.checkbox_unselected = unselected.into();
        self
    }

    /// Set the selected item color
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = Some(color);
        self
    }

    /// Set the item color
    pub fn item_color(mut self, color: Color) -> Self {
        self.item_color = Some(color);
        self
    }
}

/// MultiSelect component with built-in keyboard navigation
///
/// # Example
///
/// ```ignore
/// use rnk::components::{MultiSelect, MultiSelectItem};
///
/// let items = vec![
///     MultiSelectItem::new("Option 1", 1),
///     MultiSelectItem::selected("Option 2", 2), // Initially selected
///     MultiSelectItem::new("Option 3", 3),
/// ];
///
/// MultiSelect::new(items).into_element()
/// ```
pub struct MultiSelect<T: Clone + 'static> {
    /// Items to display
    items: Vec<MultiSelectItem<T>>,
    /// Currently highlighted index
    highlighted: usize,
    /// Maximum number of visible items (None = show all)
    limit: Option<usize>,
    /// Style configuration
    style: MultiSelectStyle,
    /// Whether the component is focused (receives input)
    is_focused: bool,
    /// Whether to enable vim-style navigation (j/k)
    vim_navigation: bool,
    /// Whether to enable number key shortcuts (1-9)
    number_shortcuts: bool,
    /// Input mode for disabled/read-only behavior.
    mode: InteractionMode,
}

impl<T: Clone + 'static> MultiSelect<T> {
    /// Create a new MultiSelect with items
    pub fn new(items: Vec<MultiSelectItem<T>>) -> Self {
        Self {
            items,
            highlighted: 0,
            limit: None,
            style: MultiSelectStyle::default(),
            is_focused: true,
            vim_navigation: true,
            number_shortcuts: false,
            mode: InteractionMode::Enabled,
        }
    }

    /// Create from an iterator of items
    pub fn from_items<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = MultiSelectItem<T>>,
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
    pub fn style(mut self, style: MultiSelectStyle) -> Self {
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

    /// Enable normal navigation and selection behavior.
    pub fn enabled(mut self) -> Self {
        self.mode = InteractionMode::Enabled;
        self
    }

    /// Ignore all input.
    pub fn disabled(mut self) -> Self {
        self.mode = InteractionMode::Disabled;
        self
    }

    /// Allow highlight navigation but block selection changes and submit.
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

    /// Get the currently selected items
    pub fn selected_items(&self) -> Vec<&MultiSelectItem<T>> {
        self.items.iter().filter(|item| item.selected).collect()
    }

    /// Get the currently selected values
    pub fn selected_values(&self) -> Vec<&T> {
        self.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| &item.value)
            .collect()
    }

    /// Convert to element with internal state management
    pub fn into_element(self) -> Element {
        if self.items.is_empty() {
            return RnkBox::new().into_element();
        }

        let initial_highlighted = self.highlighted;
        let initial_selections: Vec<bool> = self.items.iter().map(|i| i.selected).collect();
        let items = self.items.clone();
        let limit = self.limit;
        let style = self.style.clone();
        let is_focused = self.is_focused;
        let vim_navigation = self.vim_navigation;
        let number_shortcuts = self.number_shortcuts;
        let mode = self.mode;

        // Create signal for interaction state
        let state_signal =
            use_signal(|| MultiSelectState::new(initial_highlighted, initial_selections));

        // Set up input handling if focused
        if is_focused {
            let items_len = items.len();
            let state_for_input = state_signal.clone();

            use_input(move |input, key| {
                let config = NavigationConfig::new()
                    .vim_navigation(vim_navigation)
                    .number_shortcuts(number_shortcuts);

                let mut next = state_for_input.get();
                let outcome =
                    handle_multi_select_input(&mut next, items_len, input, key, &config, mode);
                if outcome.is_handled() {
                    state_for_input.set(next);
                }
            });
        }

        // Render the list
        render_multi_select_list(&items, state_signal, limit, &style)
    }
}

/// Handle MultiSelect navigation, selection changes, submit, and cancel.
pub fn handle_multi_select_input(
    state: &mut MultiSelectState,
    item_count: usize,
    input: &str,
    key: &crate::hooks::Key,
    config: &NavigationConfig,
    mode: InteractionMode,
) -> InteractionOutcome<Vec<usize>> {
    if mode.is_disabled() || item_count == 0 {
        return InteractionOutcome::Ignored;
    }

    state.ensure_len(item_count);

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

    if key.return_key {
        return InteractionOutcome::Submitted(state.selected_indices());
    }

    if key.space {
        if let Some(selected) = state.selected.get_mut(current) {
            *selected = !*selected;
            return InteractionOutcome::Changed(state.selected_indices());
        }
    }

    if input == "a" && key.ctrl {
        for selected in &mut state.selected {
            *selected = true;
        }
        return InteractionOutcome::Changed(state.selected_indices());
    }

    if input == "d" && key.ctrl {
        for selected in &mut state.selected {
            *selected = false;
        }
        return InteractionOutcome::Changed(state.selected_indices());
    }

    InteractionOutcome::Ignored
}

impl ListStyle for MultiSelectStyle {
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

/// Render the multi-select list as an Element
fn render_multi_select_list<T: Clone + 'static>(
    items: &[MultiSelectItem<T>],
    state_signal: crate::hooks::Signal<MultiSelectState>,
    limit: Option<usize>,
    style: &MultiSelectStyle,
) -> Element {
    let state = state_signal.get();
    let highlighted = state.highlighted();
    let selections = state.selected_flags().to_vec();

    render_list(
        items,
        highlighted,
        limit,
        style,
        |item, idx, _is_highlighted, prefix| {
            let is_selected = selections.get(idx).copied().unwrap_or(item.selected);
            let checkbox = if is_selected {
                &style.checkbox_selected
            } else {
                &style.checkbox_unselected
            };
            format!("{}{}{}", prefix, checkbox, item.label)
        },
        |item, idx, style, _is_highlighted, mut text| {
            let is_selected = selections.get(idx).copied().unwrap_or(item.selected);
            if is_selected {
                if let Some(color) = style.selected_color {
                    text = text.color(color);
                }
            } else if let Some(color) = style.item_color() {
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
    fn test_multi_select_item_creation() {
        let item = MultiSelectItem::new("Test", 42);
        assert_eq!(item.label, "Test");
        assert_eq!(item.value, 42);
        assert!(!item.selected);
    }

    #[test]
    fn test_multi_select_item_selected() {
        let item = MultiSelectItem::selected("Test", 42);
        assert!(item.selected);
    }

    #[test]
    fn test_multi_select_creation() {
        let items = vec![
            MultiSelectItem::new("One", 1),
            MultiSelectItem::selected("Two", 2),
            MultiSelectItem::new("Three", 3),
        ];
        let select = MultiSelect::new(items);
        assert_eq!(select.len(), 3);
        assert!(!select.is_empty());
    }

    #[test]
    fn test_multi_select_empty() {
        let select: MultiSelect<i32> = MultiSelect::new(vec![]);
        assert!(select.is_empty());
        assert_eq!(select.len(), 0);
    }

    #[test]
    fn test_multi_select_selected_values() {
        let items = vec![
            MultiSelectItem::new("One", 1),
            MultiSelectItem::selected("Two", 2),
            MultiSelectItem::selected("Three", 3),
        ];
        let select = MultiSelect::new(items);
        let selected = select.selected_values();
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&&2));
        assert!(selected.contains(&&3));
    }

    #[test]
    fn test_multi_select_style() {
        let style = MultiSelectStyle::new()
            .highlight_color(Color::Green)
            .indicator("> ")
            .checkboxes("[x]", "[ ]");

        assert_eq!(style.highlight_color, Some(Color::Green));
        assert_eq!(style.indicator, "> ");
        assert_eq!(style.checkbox_selected, "[x]");
        assert_eq!(style.checkbox_unselected, "[ ]");
    }

    #[test]
    fn test_multi_select_builder_chain() {
        let items = vec![MultiSelectItem::new("Test", 1)];
        let select = MultiSelect::new(items)
            .highlighted(0)
            .limit(5)
            .focused(true)
            .vim_navigation(true)
            .highlight_color(Color::Yellow)
            .indicator("→ ");

        assert_eq!(select.highlighted, 0);
        assert_eq!(select.limit, Some(5));
        assert!(select.is_focused);
        assert!(select.vim_navigation);
    }

    #[test]
    fn test_multi_select_from_items() {
        let items = vec![
            MultiSelectItem::new("A", 'a'),
            MultiSelectItem::new("B", 'b'),
        ];
        let select = MultiSelect::from_items(items);
        assert_eq!(select.len(), 2);
    }

    #[test]
    fn test_handle_multi_select_input_change_submit_and_cancel() {
        let mut state = MultiSelectState::new(0, vec![false, true, false]);
        let config = NavigationConfig::new().vim_navigation(true);

        let outcome = handle_multi_select_input(
            &mut state,
            3,
            "",
            &crate::hooks::Key {
                space: true,
                ..Default::default()
            },
            &config,
            InteractionMode::Enabled,
        );
        assert_eq!(outcome, InteractionOutcome::Changed(vec![0, 1]));
        assert_eq!(state.selected_indices(), vec![0, 1]);

        let outcome = handle_multi_select_input(
            &mut state,
            3,
            "j",
            &crate::hooks::Key::default(),
            &config,
            InteractionMode::Enabled,
        );
        assert_eq!(outcome, InteractionOutcome::Handled);
        assert_eq!(state.highlighted(), 1);

        let outcome = handle_multi_select_input(
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
        assert_eq!(outcome, InteractionOutcome::Submitted(vec![0, 1]));

        let outcome = handle_multi_select_input(
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
    fn test_handle_multi_select_input_modes() {
        let config = NavigationConfig::new().vim_navigation(true);
        let mut state = MultiSelectState::new(0, vec![false, false]);

        let outcome = handle_multi_select_input(
            &mut state,
            2,
            "",
            &crate::hooks::Key {
                space: true,
                ..Default::default()
            },
            &config,
            InteractionMode::ReadOnly,
        );
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert_eq!(state.selected_indices(), Vec::<usize>::new());

        let outcome = handle_multi_select_input(
            &mut state,
            2,
            "j",
            &crate::hooks::Key::default(),
            &config,
            InteractionMode::ReadOnly,
        );
        assert_eq!(outcome, InteractionOutcome::Handled);
        assert_eq!(state.highlighted(), 1);

        let outcome = handle_multi_select_input(
            &mut state,
            2,
            "k",
            &crate::hooks::Key::default(),
            &config,
            InteractionMode::Disabled,
        );
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert_eq!(state.highlighted(), 1);
    }
}
