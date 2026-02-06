//! Paginator component for page navigation
//!
//! A pagination component for navigating through pages of content,
//! similar to Bubbles' paginator component.
//!
//! # Features
//!
//! - Two display modes: Dots (●○○) and Arabic (1/10)
//! - Page navigation (next/prev)
//! - Customizable symbols and format
//! - Keyboard navigation support
//!
//! # Example
//!
//! ```ignore
//! use rnk::components::{Paginator, PaginatorType};
//! use rnk::hooks::{use_signal, use_input};
//!
//! fn app() -> Element {
//!     let page = use_signal(|| 0usize);
//!     let total_pages = 10;
//!
//!     use_input(move |_input, key| {
//!         if key.left_arrow && page.get() > 0 {
//!             page.set(page.get() - 1);
//!         }
//!         if key.right_arrow && page.get() < total_pages - 1 {
//!             page.set(page.get() + 1);
//!         }
//!     });
//!
//!     Paginator::new(page.get(), total_pages)
//!         .paginator_type(PaginatorType::Dots)
//!         .into_element()
//! }
//! ```

use crate::components::Text;
use crate::core::{Color, Element};

/// Paginator display type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaginatorType {
    /// Dot indicators (●○○○)
    #[default]
    Dots,
    /// Arabic numerals (1/10)
    Arabic,
}

/// Style configuration for the paginator
#[derive(Debug, Clone)]
pub struct PaginatorStyle {
    /// Active page indicator (for Dots mode)
    pub active_dot: String,
    /// Inactive page indicator (for Dots mode)
    pub inactive_dot: String,
    /// Separator between dots
    pub dot_separator: String,
    /// Format string for Arabic mode (use {} for current and total)
    pub arabic_format: String,
    /// Color for active indicator
    pub active_color: Option<Color>,
    /// Color for inactive indicator
    pub inactive_color: Option<Color>,
}

impl Default for PaginatorStyle {
    fn default() -> Self {
        Self {
            active_dot: "●".to_string(),
            inactive_dot: "○".to_string(),
            dot_separator: " ".to_string(),
            arabic_format: "{}/{}".to_string(),
            active_color: Some(Color::Cyan),
            inactive_color: Some(Color::BrightBlack),
        }
    }
}

impl PaginatorStyle {
    /// Create a new style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set active dot character
    pub fn active_dot(mut self, dot: impl Into<String>) -> Self {
        self.active_dot = dot.into();
        self
    }

    /// Set inactive dot character
    pub fn inactive_dot(mut self, dot: impl Into<String>) -> Self {
        self.inactive_dot = dot.into();
        self
    }

    /// Set dot separator
    pub fn dot_separator(mut self, sep: impl Into<String>) -> Self {
        self.dot_separator = sep.into();
        self
    }

    /// Set Arabic format string
    pub fn arabic_format(mut self, format: impl Into<String>) -> Self {
        self.arabic_format = format.into();
        self
    }

    /// Set active color
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = Some(color);
        self
    }

    /// Set inactive color
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_color = Some(color);
        self
    }

    /// Use filled/empty circles
    pub fn circles() -> Self {
        Self {
            active_dot: "●".to_string(),
            inactive_dot: "○".to_string(),
            ..Default::default()
        }
    }

    /// Use filled/empty squares
    pub fn squares() -> Self {
        Self {
            active_dot: "■".to_string(),
            inactive_dot: "□".to_string(),
            ..Default::default()
        }
    }

    /// Use dash style
    pub fn dashes() -> Self {
        Self {
            active_dot: "━".to_string(),
            inactive_dot: "─".to_string(),
            dot_separator: "".to_string(),
            ..Default::default()
        }
    }

    /// Use block style
    pub fn blocks() -> Self {
        Self {
            active_dot: "█".to_string(),
            inactive_dot: "░".to_string(),
            dot_separator: "".to_string(),
            ..Default::default()
        }
    }
}

/// Paginator state
#[derive(Debug, Clone)]
pub struct PaginatorState {
    /// Current page (0-indexed)
    page: usize,
    /// Total number of pages
    total_pages: usize,
    /// Items per page (for slice calculations)
    per_page: usize,
    /// Total number of items
    total_items: usize,
}

impl PaginatorState {
    /// Create a new paginator state
    pub fn new(total_pages: usize) -> Self {
        Self {
            page: 0,
            total_pages: total_pages.max(1),
            per_page: 1,
            total_items: total_pages,
        }
    }

    /// Create from total items and items per page
    pub fn from_items(total_items: usize, per_page: usize) -> Self {
        let per_page = per_page.max(1);
        let total_pages = total_items.div_ceil(per_page);
        Self {
            page: 0,
            total_pages: total_pages.max(1),
            per_page,
            total_items,
        }
    }

    /// Get current page (0-indexed)
    pub fn page(&self) -> usize {
        self.page
    }

    /// Get current page (1-indexed, for display)
    pub fn page_display(&self) -> usize {
        self.page + 1
    }

    /// Get total pages
    pub fn total_pages(&self) -> usize {
        self.total_pages
    }

    /// Get items per page
    pub fn per_page(&self) -> usize {
        self.per_page
    }

    /// Set current page
    pub fn set_page(&mut self, page: usize) {
        self.page = page.min(self.total_pages.saturating_sub(1));
    }

    /// Set total pages
    pub fn set_total_pages(&mut self, total: usize) {
        self.total_pages = total.max(1);
        self.page = self.page.min(self.total_pages.saturating_sub(1));
    }

    /// Set total items (recalculates total pages)
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        self.total_pages = total.div_ceil(self.per_page).max(1);
        self.page = self.page.min(self.total_pages.saturating_sub(1));
    }

    /// Set items per page (recalculates total pages)
    pub fn set_per_page(&mut self, per_page: usize) {
        self.per_page = per_page.max(1);
        self.total_pages = self.total_items.div_ceil(self.per_page).max(1);
        self.page = self.page.min(self.total_pages.saturating_sub(1));
    }

    /// Go to next page
    pub fn next_page(&mut self) {
        if self.page < self.total_pages - 1 {
            self.page += 1;
        }
    }

    /// Go to previous page
    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    /// Go to first page
    pub fn first_page(&mut self) {
        self.page = 0;
    }

    /// Go to last page
    pub fn last_page(&mut self) {
        self.page = self.total_pages.saturating_sub(1);
    }

    /// Check if on first page
    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }

    /// Check if on last page
    pub fn on_last_page(&self) -> bool {
        self.page >= self.total_pages.saturating_sub(1)
    }

    /// Get slice bounds for current page (start, end)
    pub fn slice_bounds(&self) -> (usize, usize) {
        let start = self.page * self.per_page;
        let end = (start + self.per_page).min(self.total_items);
        (start, end)
    }

    /// Get items for current page from a slice
    pub fn page_items<'a, T>(&self, items: &'a [T]) -> &'a [T] {
        let (start, end) = self.slice_bounds();
        let start = start.min(items.len());
        let end = end.min(items.len());
        &items[start..end]
    }

    /// Get progress as percentage (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.total_pages <= 1 {
            return 1.0;
        }
        self.page as f64 / (self.total_pages - 1) as f64
    }
}

/// Paginator component
///
/// # Example
///
/// ```ignore
/// Paginator::new(current_page, total_pages)
///     .paginator_type(PaginatorType::Dots)
///     .into_element()
/// ```
#[derive(Debug, Clone)]
pub struct Paginator {
    /// Current page (0-indexed)
    page: usize,
    /// Total number of pages
    total_pages: usize,
    /// Display type
    paginator_type: PaginatorType,
    /// Style configuration
    style: PaginatorStyle,
    /// Maximum dots to show (for Dots mode with many pages)
    max_dots: Option<usize>,
}

impl Paginator {
    /// Create a new paginator
    pub fn new(page: usize, total_pages: usize) -> Self {
        Self {
            page: page.min(total_pages.saturating_sub(1)),
            total_pages: total_pages.max(1),
            paginator_type: PaginatorType::default(),
            style: PaginatorStyle::default(),
            max_dots: None,
        }
    }

    /// Create from a PaginatorState
    pub fn from_state(state: &PaginatorState) -> Self {
        Self::new(state.page(), state.total_pages())
    }

    /// Set the display type
    pub fn paginator_type(mut self, t: PaginatorType) -> Self {
        self.paginator_type = t;
        self
    }

    /// Use dots display
    pub fn dots(mut self) -> Self {
        self.paginator_type = PaginatorType::Dots;
        self
    }

    /// Use arabic (numeric) display
    pub fn arabic(mut self) -> Self {
        self.paginator_type = PaginatorType::Arabic;
        self
    }

    /// Set the style
    pub fn style(mut self, style: PaginatorStyle) -> Self {
        self.style = style;
        self
    }

    /// Set maximum dots to display
    pub fn max_dots(mut self, max: usize) -> Self {
        self.max_dots = Some(max);
        self
    }

    /// Set active dot character
    pub fn active_dot(mut self, dot: impl Into<String>) -> Self {
        self.style.active_dot = dot.into();
        self
    }

    /// Set inactive dot character
    pub fn inactive_dot(mut self, dot: impl Into<String>) -> Self {
        self.style.inactive_dot = dot.into();
        self
    }

    /// Set active color
    pub fn active_color(mut self, color: Color) -> Self {
        self.style.active_color = Some(color);
        self
    }

    /// Set inactive color
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.style.inactive_color = Some(color);
        self
    }

    /// Render as string (for non-TUI usage)
    pub fn render(&self) -> String {
        match self.paginator_type {
            PaginatorType::Dots => self.render_dots(),
            PaginatorType::Arabic => self.render_arabic(),
        }
    }

    /// Render dots mode
    fn render_dots(&self) -> String {
        let _total = match self.max_dots {
            Some(max) if self.total_pages > max => max,
            _ => self.total_pages,
        };

        // Calculate which dots to show if we have max_dots limit
        let (start, end) = if let Some(max) = self.max_dots {
            if self.total_pages <= max {
                (0, self.total_pages)
            } else {
                let half = max / 2;
                let start = if self.page <= half {
                    0
                } else if self.page >= self.total_pages - half {
                    self.total_pages - max
                } else {
                    self.page - half
                };
                (start, start + max)
            }
        } else {
            (0, self.total_pages)
        };

        let mut parts = Vec::new();
        for i in start..end {
            if i == self.page {
                parts.push(self.style.active_dot.clone());
            } else {
                parts.push(self.style.inactive_dot.clone());
            }
        }

        parts.join(&self.style.dot_separator)
    }

    /// Render arabic mode
    fn render_arabic(&self) -> String {
        // Replace first {} with current page, second {} with total
        let current = (self.page + 1).to_string();
        let total = self.total_pages.to_string();

        let result = self.style.arabic_format.replacen("{}", &current, 1);
        result.replacen("{}", &total, 1)
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        match self.paginator_type {
            PaginatorType::Dots => self.render_dots_element(),
            PaginatorType::Arabic => self.render_arabic_element(),
        }
    }

    /// Render dots as Element
    fn render_dots_element(&self) -> Element {
        use crate::components::Box as RnkBox;
        use crate::core::FlexDirection;

        let _total = match self.max_dots {
            Some(max) if self.total_pages > max => max,
            _ => self.total_pages,
        };

        let (start, end) = if let Some(max) = self.max_dots {
            if self.total_pages <= max {
                (0, self.total_pages)
            } else {
                let half = max / 2;
                let start = if self.page <= half {
                    0
                } else if self.page >= self.total_pages - half {
                    self.total_pages - max
                } else {
                    self.page - half
                };
                (start, start + max)
            }
        } else {
            (0, self.total_pages)
        };

        let mut container = RnkBox::new().flex_direction(FlexDirection::Row);

        for i in start..end {
            let is_active = i == self.page;
            let dot = if is_active {
                &self.style.active_dot
            } else {
                &self.style.inactive_dot
            };

            // Add separator before (except first)
            if i > start && !self.style.dot_separator.is_empty() {
                container = container.child(Text::new(&self.style.dot_separator).into_element());
            }

            let mut text = Text::new(dot);
            if is_active {
                if let Some(color) = self.style.active_color {
                    text = text.color(color);
                }
            } else if let Some(color) = self.style.inactive_color {
                text = text.color(color);
            }

            container = container.child(text.into_element());
        }

        container.into_element()
    }

    /// Render arabic as Element
    fn render_arabic_element(&self) -> Element {
        let text = self.render_arabic();
        let mut elem = Text::new(&text);
        if let Some(color) = self.style.active_color {
            elem = elem.color(color);
        }
        elem.into_element()
    }
}

/// Handle paginator input
pub fn handle_paginator_input(
    state: &mut PaginatorState,
    _input: &str,
    key: &crate::hooks::Key,
) -> bool {
    let mut handled = false;

    if key.left_arrow || key.page_up {
        state.prev_page();
        handled = true;
    } else if key.right_arrow || key.page_down {
        state.next_page();
        handled = true;
    } else if key.home {
        state.first_page();
        handled = true;
    } else if key.end {
        state.last_page();
        handled = true;
    }

    handled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginator_state_new() {
        let state = PaginatorState::new(10);
        assert_eq!(state.page(), 0);
        assert_eq!(state.total_pages(), 10);
        assert!(state.on_first_page());
        assert!(!state.on_last_page());
    }

    #[test]
    fn test_paginator_state_from_items() {
        let state = PaginatorState::from_items(25, 10);
        assert_eq!(state.total_pages(), 3);
        assert_eq!(state.per_page(), 10);
    }

    #[test]
    fn test_paginator_navigation() {
        let mut state = PaginatorState::new(5);

        state.next_page();
        assert_eq!(state.page(), 1);

        state.next_page();
        state.next_page();
        state.next_page();
        assert_eq!(state.page(), 4);
        assert!(state.on_last_page());

        state.next_page(); // Should not go beyond last
        assert_eq!(state.page(), 4);

        state.prev_page();
        assert_eq!(state.page(), 3);

        state.first_page();
        assert_eq!(state.page(), 0);

        state.last_page();
        assert_eq!(state.page(), 4);
    }

    #[test]
    fn test_paginator_slice_bounds() {
        let mut state = PaginatorState::from_items(25, 10);

        assert_eq!(state.slice_bounds(), (0, 10));

        state.next_page();
        assert_eq!(state.slice_bounds(), (10, 20));

        state.next_page();
        assert_eq!(state.slice_bounds(), (20, 25));
    }

    #[test]
    fn test_paginator_page_items() {
        let items: Vec<i32> = (0..25).collect();
        let mut state = PaginatorState::from_items(25, 10);

        let page_items = state.page_items(&items);
        assert_eq!(page_items, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        state.last_page();
        let page_items = state.page_items(&items);
        assert_eq!(page_items, &[20, 21, 22, 23, 24]);
    }

    #[test]
    fn test_paginator_render_dots() {
        let paginator = Paginator::new(2, 5).dots();
        let rendered = paginator.render();
        assert!(rendered.contains("●"));
        assert!(rendered.contains("○"));
    }

    #[test]
    fn test_paginator_render_arabic() {
        let paginator = Paginator::new(2, 5).arabic();
        let rendered = paginator.render();
        assert_eq!(rendered, "3/5");
    }

    #[test]
    fn test_paginator_max_dots() {
        let paginator = Paginator::new(5, 20).dots().max_dots(5);
        let rendered = paginator.render();
        // Should only show 5 dots
        let dot_count = rendered.matches('●').count() + rendered.matches('○').count();
        assert_eq!(dot_count, 5);
    }

    #[test]
    fn test_paginator_progress() {
        let mut state = PaginatorState::new(5);
        assert_eq!(state.progress(), 0.0);

        state.set_page(2);
        assert_eq!(state.progress(), 0.5);

        state.last_page();
        assert_eq!(state.progress(), 1.0);
    }

    #[test]
    fn test_paginator_style_presets() {
        let _circles = PaginatorStyle::circles();
        let _squares = PaginatorStyle::squares();
        let _dashes = PaginatorStyle::dashes();
        let _blocks = PaginatorStyle::blocks();
    }
}
