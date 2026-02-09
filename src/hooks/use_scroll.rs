//! Scroll state management hook
//!
//! Provides scroll state management for scrollable content areas.

use std::sync::{Arc, RwLock};

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
    state: Arc<RwLock<ScrollState>>,
}

impl ScrollHandle {
    /// Get the current scroll state
    pub fn get(&self) -> ScrollState {
        self.state.read().unwrap().clone()
    }

    /// Get the current vertical offset
    pub fn offset_y(&self) -> usize {
        self.state.read().unwrap().offset_y
    }

    /// Get the current horizontal offset
    pub fn offset_x(&self) -> usize {
        self.state.read().unwrap().offset_x
    }

    /// Set content size
    pub fn set_content_size(&self, width: usize, height: usize) {
        self.state.write().unwrap().set_content_size(width, height);
    }

    /// Set viewport size
    pub fn set_viewport_size(&self, width: usize, height: usize) {
        self.state.write().unwrap().set_viewport_size(width, height);
    }

    /// Scroll up
    pub fn scroll_up(&self, lines: usize) {
        self.state.write().unwrap().scroll_up(lines);
    }

    /// Scroll down
    pub fn scroll_down(&self, lines: usize) {
        self.state.write().unwrap().scroll_down(lines);
    }

    /// Scroll left
    pub fn scroll_left(&self, cols: usize) {
        self.state.write().unwrap().scroll_left(cols);
    }

    /// Scroll right
    pub fn scroll_right(&self, cols: usize) {
        self.state.write().unwrap().scroll_right(cols);
    }

    /// Scroll to specific Y position
    pub fn scroll_to_y(&self, offset: usize) {
        self.state.write().unwrap().scroll_to_y(offset);
    }

    /// Scroll to specific X position
    pub fn scroll_to_x(&self, offset: usize) {
        self.state.write().unwrap().scroll_to_x(offset);
    }

    /// Scroll to top
    pub fn scroll_to_top(&self) {
        self.state.write().unwrap().scroll_to_top();
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&self) {
        self.state.write().unwrap().scroll_to_bottom();
    }

    /// Page up
    pub fn page_up(&self) {
        self.state.write().unwrap().page_up();
    }

    /// Page down
    pub fn page_down(&self) {
        self.state.write().unwrap().page_down();
    }

    /// Scroll to make an item visible
    pub fn scroll_to_item(&self, index: usize) {
        self.state.write().unwrap().scroll_to_item(index);
    }

    /// Check if can scroll up
    pub fn can_scroll_up(&self) -> bool {
        self.state.read().unwrap().can_scroll_up()
    }

    /// Check if can scroll down
    pub fn can_scroll_down(&self) -> bool {
        self.state.read().unwrap().can_scroll_down()
    }

    /// Get vertical scroll percentage
    pub fn scroll_percent_y(&self) -> f32 {
        self.state.read().unwrap().scroll_percent_y()
    }

    /// Get visible range
    pub fn visible_range(&self) -> (usize, usize) {
        self.state.read().unwrap().visible_range()
    }

    // =========================================================================
    // Try methods (non-panicking versions)
    // =========================================================================

    /// Try to get the current scroll state, returning None if lock is poisoned
    pub fn try_get(&self) -> Option<ScrollState> {
        self.state.read().ok().map(|g| g.clone())
    }

    /// Try to get the current vertical offset, returning None if lock is poisoned
    pub fn try_offset_y(&self) -> Option<usize> {
        self.state.read().ok().map(|g| g.offset_y)
    }

    /// Try to get the current horizontal offset, returning None if lock is poisoned
    pub fn try_offset_x(&self) -> Option<usize> {
        self.state.read().ok().map(|g| g.offset_x)
    }

    /// Try to set content size, returning false if lock is poisoned
    pub fn try_set_content_size(&self, width: usize, height: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.set_content_size(width, height);
            true
        } else {
            false
        }
    }

    /// Try to set viewport size, returning false if lock is poisoned
    pub fn try_set_viewport_size(&self, width: usize, height: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.set_viewport_size(width, height);
            true
        } else {
            false
        }
    }

    /// Try to scroll up, returning false if lock is poisoned
    pub fn try_scroll_up(&self, lines: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_up(lines);
            true
        } else {
            false
        }
    }

    /// Try to scroll down, returning false if lock is poisoned
    pub fn try_scroll_down(&self, lines: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_down(lines);
            true
        } else {
            false
        }
    }

    /// Try to scroll left, returning false if lock is poisoned
    pub fn try_scroll_left(&self, cols: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_left(cols);
            true
        } else {
            false
        }
    }

    /// Try to scroll right, returning false if lock is poisoned
    pub fn try_scroll_right(&self, cols: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_right(cols);
            true
        } else {
            false
        }
    }

    /// Try to scroll to specific Y position, returning false if lock is poisoned
    pub fn try_scroll_to_y(&self, offset: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_to_y(offset);
            true
        } else {
            false
        }
    }

    /// Try to scroll to specific X position, returning false if lock is poisoned
    pub fn try_scroll_to_x(&self, offset: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_to_x(offset);
            true
        } else {
            false
        }
    }

    /// Try to scroll to top, returning false if lock is poisoned
    pub fn try_scroll_to_top(&self) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_to_top();
            true
        } else {
            false
        }
    }

    /// Try to scroll to bottom, returning false if lock is poisoned
    pub fn try_scroll_to_bottom(&self) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_to_bottom();
            true
        } else {
            false
        }
    }

    /// Try to page up, returning false if lock is poisoned
    pub fn try_page_up(&self) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.page_up();
            true
        } else {
            false
        }
    }

    /// Try to page down, returning false if lock is poisoned
    pub fn try_page_down(&self) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.page_down();
            true
        } else {
            false
        }
    }

    /// Try to scroll to make an item visible, returning false if lock is poisoned
    pub fn try_scroll_to_item(&self, index: usize) -> bool {
        if let Ok(mut guard) = self.state.write() {
            guard.scroll_to_item(index);
            true
        } else {
            false
        }
    }

    /// Try to check if can scroll up, returning None if lock is poisoned
    pub fn try_can_scroll_up(&self) -> Option<bool> {
        self.state.read().ok().map(|g| g.can_scroll_up())
    }

    /// Try to check if can scroll down, returning None if lock is poisoned
    pub fn try_can_scroll_down(&self) -> Option<bool> {
        self.state.read().ok().map(|g| g.can_scroll_down())
    }

    /// Try to get vertical scroll percentage, returning None if lock is poisoned
    pub fn try_scroll_percent_y(&self) -> Option<f32> {
        self.state.read().ok().map(|g| g.scroll_percent_y())
    }

    /// Try to get visible range, returning None if lock is poisoned
    pub fn try_visible_range(&self) -> Option<(usize, usize)> {
        self.state.read().ok().map(|g| g.visible_range())
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

    let Some(ctx) = current_context() else {
        return ScrollHandle {
            state: Arc::new(RwLock::new(ScrollState::new())),
        };
    };
    let Ok(mut ctx_ref) = ctx.try_borrow_mut() else {
        return ScrollHandle {
            state: Arc::new(RwLock::new(ScrollState::new())),
        };
    };

    // Use the hook API to get or create scroll state
    let storage = ctx_ref.use_hook(|| Arc::new(RwLock::new(ScrollState::new())));
    let state = storage
        .get::<Arc<RwLock<ScrollState>>>()
        .unwrap_or_else(|| Arc::new(RwLock::new(ScrollState::new())));

    ScrollHandle { state }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;

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

    #[test]
    fn test_use_scroll_without_context_does_not_panic() {
        let scroll = use_scroll();
        scroll.set_content_size(100, 50);
        scroll.set_viewport_size(80, 10);
        scroll.scroll_down(7);
        assert_eq!(scroll.offset_y(), 7);
    }

    #[test]
    fn test_use_scroll_preserves_state_in_context() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let scroll1 = with_hooks(ctx.clone(), use_scroll);
        scroll1.set_content_size(100, 50);
        scroll1.set_viewport_size(80, 10);
        scroll1.scroll_down(9);

        let scroll2 = with_hooks(ctx, use_scroll);
        assert_eq!(scroll2.offset_y(), 9);
    }
}
