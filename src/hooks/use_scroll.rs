//! Scroll state management hook
//!
//! Provides scroll state management for scrollable content areas.

use std::cell::RefCell;

/// Scroll state for a scrollable area
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    /// Current vertical scroll offset (in rows)
    pub offset_y: usize,
    /// Current horizontal scroll offset (in columns)
    pub offset_x: usize,
    /// Total content height (in rows)
    pub content_height: usize,
    /// Total content width (in columns)
    pub content_width: usize,
    /// Viewport height (visible rows)
    pub viewport_height: usize,
    /// Viewport width (visible columns)
    pub viewport_width: usize,
}

impl ScrollState {
    /// Create a new scroll state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a scroll state with initial viewport size
    pub fn with_viewport(viewport_width: usize, viewport_height: usize) -> Self {
        Self {
            viewport_width,
            viewport_height,
            ..Default::default()
        }
    }

    /// Set the content size
    pub fn set_content_size(&mut self, width: usize, height: usize) {
        self.content_width = width;
        self.content_height = height;
        // Ensure offset is still valid
        self.clamp_offset();
    }

    /// Set the viewport size
    pub fn set_viewport_size(&mut self, width: usize, height: usize) {
        self.viewport_width = width;
        self.viewport_height = height;
        // Ensure offset is still valid
        self.clamp_offset();
    }

    /// Scroll up by a number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.offset_y = self.offset_y.saturating_sub(lines);
    }

    /// Scroll down by a number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.offset_y = self.offset_y.saturating_add(lines);
        self.clamp_offset();
    }

    /// Scroll left by a number of columns
    pub fn scroll_left(&mut self, cols: usize) {
        self.offset_x = self.offset_x.saturating_sub(cols);
    }

    /// Scroll right by a number of columns
    pub fn scroll_right(&mut self, cols: usize) {
        self.offset_x = self.offset_x.saturating_add(cols);
        self.clamp_offset();
    }

    /// Scroll to a specific vertical position
    pub fn scroll_to_y(&mut self, offset: usize) {
        self.offset_y = offset;
        self.clamp_offset();
    }

    /// Scroll to a specific horizontal position
    pub fn scroll_to_x(&mut self, offset: usize) {
        self.offset_x = offset;
        self.clamp_offset();
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.offset_y = 0;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        if self.content_height > self.viewport_height {
            self.offset_y = self.content_height - self.viewport_height;
        } else {
            self.offset_y = 0;
        }
    }

    /// Page up (scroll by viewport height)
    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.max(1));
    }

    /// Page down (scroll by viewport height)
    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.max(1));
    }

    /// Ensure an item at a given index is visible
    pub fn scroll_to_item(&mut self, index: usize) {
        if index < self.offset_y {
            self.offset_y = index;
        } else if index >= self.offset_y + self.viewport_height {
            self.offset_y = index.saturating_sub(self.viewport_height - 1);
        }
    }

    /// Get the maximum vertical scroll offset
    pub fn max_offset_y(&self) -> usize {
        self.content_height.saturating_sub(self.viewport_height)
    }

    /// Get the maximum horizontal scroll offset
    pub fn max_offset_x(&self) -> usize {
        self.content_width.saturating_sub(self.viewport_width)
    }

    /// Check if there's more content above
    pub fn can_scroll_up(&self) -> bool {
        self.offset_y > 0
    }

    /// Check if there's more content below
    pub fn can_scroll_down(&self) -> bool {
        self.offset_y < self.max_offset_y()
    }

    /// Check if there's more content to the left
    pub fn can_scroll_left(&self) -> bool {
        self.offset_x > 0
    }

    /// Check if there's more content to the right
    pub fn can_scroll_right(&self) -> bool {
        self.offset_x < self.max_offset_x()
    }

    /// Get the vertical scroll percentage (0.0 to 1.0)
    pub fn scroll_percent_y(&self) -> f32 {
        let max = self.max_offset_y();
        if max == 0 {
            0.0
        } else {
            self.offset_y as f32 / max as f32
        }
    }

    /// Get the horizontal scroll percentage (0.0 to 1.0)
    pub fn scroll_percent_x(&self) -> f32 {
        let max = self.max_offset_x();
        if max == 0 {
            0.0
        } else {
            self.offset_x as f32 / max as f32
        }
    }

    /// Get visible range of items (start_index, end_index exclusive)
    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.offset_y;
        let end = (self.offset_y + self.viewport_height).min(self.content_height);
        (start, end)
    }

    /// Clamp offset to valid range
    fn clamp_offset(&mut self) {
        self.offset_y = self.offset_y.min(self.max_offset_y());
        self.offset_x = self.offset_x.min(self.max_offset_x());
    }
}

/// Scroll handle for managing scroll state
#[derive(Clone)]
pub struct ScrollHandle {
    state: std::rc::Rc<RefCell<ScrollState>>,
}

impl ScrollHandle {
    /// Get the current scroll state
    pub fn get(&self) -> ScrollState {
        self.state.borrow().clone()
    }

    /// Get the current vertical offset
    pub fn offset_y(&self) -> usize {
        self.state.borrow().offset_y
    }

    /// Get the current horizontal offset
    pub fn offset_x(&self) -> usize {
        self.state.borrow().offset_x
    }

    /// Set content size
    pub fn set_content_size(&self, width: usize, height: usize) {
        self.state.borrow_mut().set_content_size(width, height);
    }

    /// Set viewport size
    pub fn set_viewport_size(&self, width: usize, height: usize) {
        self.state.borrow_mut().set_viewport_size(width, height);
    }

    /// Scroll up
    pub fn scroll_up(&self, lines: usize) {
        self.state.borrow_mut().scroll_up(lines);
    }

    /// Scroll down
    pub fn scroll_down(&self, lines: usize) {
        self.state.borrow_mut().scroll_down(lines);
    }

    /// Scroll left
    pub fn scroll_left(&self, cols: usize) {
        self.state.borrow_mut().scroll_left(cols);
    }

    /// Scroll right
    pub fn scroll_right(&self, cols: usize) {
        self.state.borrow_mut().scroll_right(cols);
    }

    /// Scroll to specific Y position
    pub fn scroll_to_y(&self, offset: usize) {
        self.state.borrow_mut().scroll_to_y(offset);
    }

    /// Scroll to specific X position
    pub fn scroll_to_x(&self, offset: usize) {
        self.state.borrow_mut().scroll_to_x(offset);
    }

    /// Scroll to top
    pub fn scroll_to_top(&self) {
        self.state.borrow_mut().scroll_to_top();
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&self) {
        self.state.borrow_mut().scroll_to_bottom();
    }

    /// Page up
    pub fn page_up(&self) {
        self.state.borrow_mut().page_up();
    }

    /// Page down
    pub fn page_down(&self) {
        self.state.borrow_mut().page_down();
    }

    /// Scroll to make an item visible
    pub fn scroll_to_item(&self, index: usize) {
        self.state.borrow_mut().scroll_to_item(index);
    }

    /// Check if can scroll up
    pub fn can_scroll_up(&self) -> bool {
        self.state.borrow().can_scroll_up()
    }

    /// Check if can scroll down
    pub fn can_scroll_down(&self) -> bool {
        self.state.borrow().can_scroll_down()
    }

    /// Get vertical scroll percentage
    pub fn scroll_percent_y(&self) -> f32 {
        self.state.borrow().scroll_percent_y()
    }

    /// Get visible range
    pub fn visible_range(&self) -> (usize, usize) {
        self.state.borrow().visible_range()
    }
}

/// Hook to manage scroll state
///
/// # Example
///
/// ```ignore
/// let scroll = use_scroll();
///
/// // Set content and viewport sizes
/// scroll.set_content_size(100, 500);  // 100 cols, 500 rows
/// scroll.set_viewport_size(80, 20);   // 80 cols, 20 rows visible
///
/// // Handle scroll input
/// use_input(move |_ch, key| {
///     if key.up_arrow {
///         scroll.scroll_up(1);
///     } else if key.down_arrow {
///         scroll.scroll_down(1);
///     } else if key.page_up {
///         scroll.page_up();
///     } else if key.page_down {
///         scroll.page_down();
///     }
/// });
///
/// // Get visible range for rendering
/// let (start, end) = scroll.visible_range();
/// for i in start..end {
///     // Render item i
/// }
/// ```
pub fn use_scroll() -> ScrollHandle {
    use crate::hooks::context::current_context;

    let ctx = current_context().expect("use_scroll must be called within a render context");
    let mut ctx_ref = ctx.borrow_mut();

    // Use the hook API to get or create scroll state
    let storage = ctx_ref.use_hook(|| std::rc::Rc::new(RefCell::new(ScrollState::new())));
    let state = storage
        .get::<std::rc::Rc<RefCell<ScrollState>>>()
        .expect("scroll state should be the correct type")
        .clone();

    ScrollHandle { state }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_state_basic() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        assert_eq!(state.offset_y, 0);
        assert_eq!(state.max_offset_y(), 40);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        state.scroll_down(5);
        assert_eq!(state.offset_y, 5);

        state.scroll_down(100);
        assert_eq!(state.offset_y, 40); // Clamped to max
    }

    #[test]
    fn test_scroll_up() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        state.scroll_down(20);
        state.scroll_up(5);
        assert_eq!(state.offset_y, 15);

        state.scroll_up(100);
        assert_eq!(state.offset_y, 0);
    }

    #[test]
    fn test_page_navigation() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        state.page_down();
        assert_eq!(state.offset_y, 10);

        state.page_up();
        assert_eq!(state.offset_y, 0);
    }

    #[test]
    fn test_scroll_to_item() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        state.scroll_to_item(15);
        assert_eq!(state.offset_y, 6); // 15 - (10 - 1) = 6

        state.scroll_to_item(3);
        assert_eq!(state.offset_y, 3);
    }

    #[test]
    fn test_visible_range() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);
        state.scroll_down(5);

        let (start, end) = state.visible_range();
        assert_eq!(start, 5);
        assert_eq!(end, 15);
    }

    #[test]
    fn test_scroll_percent() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        assert_eq!(state.scroll_percent_y(), 0.0);

        state.scroll_to_bottom();
        assert_eq!(state.scroll_percent_y(), 1.0);

        state.scroll_to_y(20);
        assert!((state.scroll_percent_y() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_can_scroll() {
        let mut state = ScrollState::new();
        state.set_content_size(100, 50);
        state.set_viewport_size(80, 10);

        assert!(!state.can_scroll_up());
        assert!(state.can_scroll_down());

        state.scroll_to_bottom();
        assert!(state.can_scroll_up());
        assert!(!state.can_scroll_down());
    }
}
