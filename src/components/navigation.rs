//! Shared navigation utilities for list-based components
//!
//! This module provides common navigation logic used by SelectInput, MultiSelect,
//! and other list-based components.

use crate::hooks::Key;

/// Trait for components that maintain selection state
///
/// This trait provides a common interface for list-like components
/// that need to track a selected index and scroll offset.
pub trait SelectionState {
    /// Get the currently selected index
    fn selected(&self) -> Option<usize>;

    /// Set the selected index
    fn select(&mut self, index: Option<usize>);

    /// Get the scroll offset
    fn offset(&self) -> usize;

    /// Set the scroll offset
    fn set_offset(&mut self, offset: usize);

    /// Select the next item
    fn select_next(&mut self, len: usize) {
        if len == 0 {
            self.select(None);
            return;
        }
        let new_index = match self.selected() {
            Some(i) => (i + 1).min(len - 1),
            None => 0,
        };
        self.select(Some(new_index));
    }

    /// Select the previous item
    fn select_previous(&mut self, len: usize) {
        if len == 0 {
            self.select(None);
            return;
        }
        let new_index = match self.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.select(Some(new_index));
    }

    /// Select the first item
    fn select_first(&mut self, len: usize) {
        if len > 0 {
            self.select(Some(0));
        }
    }

    /// Select the last item
    fn select_last(&mut self, len: usize) {
        if len > 0 {
            self.select(Some(len - 1));
        }
    }

    /// Adjust scroll offset to keep selection visible
    fn scroll_to_selected(&mut self, viewport_height: usize) {
        if let Some(selected) = self.selected() {
            let offset = self.offset();
            // If selection is above viewport, scroll up
            if selected < offset {
                self.set_offset(selected);
            }
            // If selection is below viewport, scroll down
            else if selected >= offset + viewport_height {
                self.set_offset(selected.saturating_sub(viewport_height - 1));
            }
        }
    }
}

/// Navigation result indicating the new cursor position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationResult {
    /// Cursor moved to a new position
    Moved(usize),
    /// No navigation occurred
    None,
}

impl NavigationResult {
    /// Get the new position if moved, otherwise return the current position
    pub fn unwrap_or(self, current: usize) -> usize {
        match self {
            NavigationResult::Moved(pos) => pos,
            NavigationResult::None => current,
        }
    }

    /// Check if navigation occurred
    pub fn is_moved(&self) -> bool {
        matches!(self, NavigationResult::Moved(_))
    }
}

/// Configuration for list navigation behavior
#[derive(Debug, Clone)]
pub struct NavigationConfig {
    /// Enable vim-style navigation (j/k)
    pub vim_navigation: bool,
    /// Enable number shortcuts (1-9)
    pub number_shortcuts: bool,
    /// Page size for page up/down
    pub page_size: usize,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            vim_navigation: false,
            number_shortcuts: false,
            page_size: 5,
        }
    }
}

impl NavigationConfig {
    /// Create a new navigation config
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable vim-style navigation
    pub fn vim_navigation(mut self, enabled: bool) -> Self {
        self.vim_navigation = enabled;
        self
    }

    /// Enable number shortcuts
    pub fn number_shortcuts(mut self, enabled: bool) -> Self {
        self.number_shortcuts = enabled;
        self
    }

    /// Set page size for page up/down
    pub fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }
}

/// Handle list navigation based on input and key events
///
/// Returns the new cursor position if navigation occurred.
///
/// # Arguments
///
/// * `current` - Current cursor position
/// * `total` - Total number of items in the list
/// * `input` - Input string from key event
/// * `key` - Key event information
/// * `config` - Navigation configuration
///
/// # Example
///
/// ```
/// use rnk::components::navigation::{handle_list_navigation, NavigationConfig};
/// use rnk::hooks::Key;
///
/// let config = NavigationConfig::default().vim_navigation(true);
/// let key = Key::default();
/// let new_pos = handle_list_navigation(0, 10, "j", key, &config);
/// ```
pub fn handle_list_navigation(
    current: usize,
    total: usize,
    input: &str,
    key: Key,
    config: &NavigationConfig,
) -> NavigationResult {
    if total == 0 {
        return NavigationResult::None;
    }

    let max_index = total.saturating_sub(1);

    // Arrow key navigation
    if key.up_arrow {
        return NavigationResult::Moved(current.saturating_sub(1));
    }
    if key.down_arrow {
        return NavigationResult::Moved((current + 1).min(max_index));
    }

    // Vim-style navigation
    if config.vim_navigation {
        if input == "k" {
            return NavigationResult::Moved(current.saturating_sub(1));
        }
        if input == "j" {
            return NavigationResult::Moved((current + 1).min(max_index));
        }
    }

    // Number shortcuts (1-9)
    if config.number_shortcuts {
        if let Some(num) = input.chars().next().and_then(|c| c.to_digit(10)) {
            if (1..=9).contains(&num) {
                let index = (num as usize) - 1;
                if index < total {
                    return NavigationResult::Moved(index);
                }
            }
        }
    }

    // Home/End navigation
    if key.home {
        return NavigationResult::Moved(0);
    }
    if key.end {
        return NavigationResult::Moved(max_index);
    }

    // Page up/down
    if key.page_up {
        return NavigationResult::Moved(current.saturating_sub(config.page_size));
    }
    if key.page_down {
        return NavigationResult::Moved((current + config.page_size).min(max_index));
    }

    NavigationResult::None
}

/// Calculate visible range for a scrollable list
///
/// Returns (start, end) indices for the visible portion of the list.
///
/// # Arguments
///
/// * `highlighted` - Currently highlighted/focused index
/// * `total` - Total number of items
/// * `limit` - Optional maximum number of visible items
pub fn calculate_visible_range(
    highlighted: usize,
    total: usize,
    limit: Option<usize>,
) -> (usize, usize) {
    if let Some(limit) = limit {
        let half = limit / 2;
        let start = if highlighted <= half {
            0
        } else if highlighted >= total.saturating_sub(half) {
            total.saturating_sub(limit)
        } else {
            highlighted.saturating_sub(half)
        };
        let end = (start + limit).min(total);
        (start, end)
    } else {
        (0, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_result() {
        let moved = NavigationResult::Moved(5);
        assert!(moved.is_moved());
        assert_eq!(moved.unwrap_or(0), 5);

        let none = NavigationResult::None;
        assert!(!none.is_moved());
        assert_eq!(none.unwrap_or(3), 3);
    }

    #[test]
    fn test_navigation_config_builder() {
        let config = NavigationConfig::new()
            .vim_navigation(true)
            .number_shortcuts(true)
            .page_size(10);

        assert!(config.vim_navigation);
        assert!(config.number_shortcuts);
        assert_eq!(config.page_size, 10);
    }

    #[test]
    fn test_arrow_navigation() {
        let config = NavigationConfig::default();
        let down_key = Key {
            down_arrow: true,
            ..Key::default()
        };
        let up_key = Key {
            up_arrow: true,
            ..Key::default()
        };

        // Down arrow
        let result = handle_list_navigation(0, 10, "", down_key, &config);
        assert_eq!(result, NavigationResult::Moved(1));

        // Up arrow
        let result = handle_list_navigation(5, 10, "", up_key, &config);
        assert_eq!(result, NavigationResult::Moved(4));

        // Up arrow at start (should stay at 0)
        let result = handle_list_navigation(0, 10, "", up_key, &config);
        assert_eq!(result, NavigationResult::Moved(0));
    }

    #[test]
    fn test_vim_navigation() {
        let config = NavigationConfig::default().vim_navigation(true);
        let key = Key::default();

        // j for down
        let result = handle_list_navigation(0, 10, "j", key, &config);
        assert_eq!(result, NavigationResult::Moved(1));

        // k for up
        let result = handle_list_navigation(5, 10, "k", key, &config);
        assert_eq!(result, NavigationResult::Moved(4));
    }

    #[test]
    fn test_vim_navigation_disabled() {
        let config = NavigationConfig::default().vim_navigation(false);
        let key = Key::default();

        // j should not navigate when vim_navigation is disabled
        let result = handle_list_navigation(0, 10, "j", key, &config);
        assert_eq!(result, NavigationResult::None);
    }

    #[test]
    fn test_number_shortcuts() {
        let config = NavigationConfig::default().number_shortcuts(true);
        let key = Key::default();

        // Press "1" to go to index 0
        let result = handle_list_navigation(5, 10, "1", key, &config);
        assert_eq!(result, NavigationResult::Moved(0));

        // Press "5" to go to index 4
        let result = handle_list_navigation(0, 10, "5", key, &config);
        assert_eq!(result, NavigationResult::Moved(4));

        // Press "9" when only 5 items (should not navigate)
        let result = handle_list_navigation(0, 5, "9", key, &config);
        assert_eq!(result, NavigationResult::None);
    }

    #[test]
    fn test_home_end_navigation() {
        let config = NavigationConfig::default();
        let home_key = Key {
            home: true,
            ..Key::default()
        };
        let end_key = Key {
            end: true,
            ..Key::default()
        };

        // Home
        let result = handle_list_navigation(5, 10, "", home_key, &config);
        assert_eq!(result, NavigationResult::Moved(0));

        // End
        let result = handle_list_navigation(0, 10, "", end_key, &config);
        assert_eq!(result, NavigationResult::Moved(9));
    }

    #[test]
    fn test_page_navigation() {
        let config = NavigationConfig::default().page_size(5);
        let page_down_key = Key {
            page_down: true,
            ..Key::default()
        };
        let page_up_key = Key {
            page_up: true,
            ..Key::default()
        };

        // Page down
        let result = handle_list_navigation(0, 20, "", page_down_key, &config);
        assert_eq!(result, NavigationResult::Moved(5));

        // Page up
        let result = handle_list_navigation(10, 20, "", page_up_key, &config);
        assert_eq!(result, NavigationResult::Moved(5));
    }

    #[test]
    fn test_boundary_conditions() {
        let config = NavigationConfig::default();
        let down_key = Key {
            down_arrow: true,
            ..Key::default()
        };

        // Down at end
        let result = handle_list_navigation(9, 10, "", down_key, &config);
        assert_eq!(result, NavigationResult::Moved(9));

        // Empty list
        let result = handle_list_navigation(0, 0, "", down_key, &config);
        assert_eq!(result, NavigationResult::None);
    }

    #[test]
    fn test_calculate_visible_range_no_limit() {
        let (start, end) = calculate_visible_range(5, 20, None);
        assert_eq!(start, 0);
        assert_eq!(end, 20);
    }

    #[test]
    fn test_calculate_visible_range_with_limit() {
        // At start
        let (start, end) = calculate_visible_range(0, 20, Some(5));
        assert_eq!(start, 0);
        assert_eq!(end, 5);

        // In middle
        let (start, end) = calculate_visible_range(10, 20, Some(5));
        assert_eq!(start, 8);
        assert_eq!(end, 13);

        // At end
        let (start, end) = calculate_visible_range(19, 20, Some(5));
        assert_eq!(start, 15);
        assert_eq!(end, 20);
    }
}
