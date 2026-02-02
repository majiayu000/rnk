//! Viewport state management
//!
//! Manages scroll position, content, and navigation state for the viewport.

use std::cmp;

/// Viewport state containing scroll position and content
#[derive(Debug, Clone)]
pub struct ViewportState {
    /// Content lines (pre-split for performance)
    lines: Vec<String>,

    /// Current vertical scroll offset (line index)
    y_offset: usize,

    /// Current horizontal scroll offset (column index)
    x_offset: usize,

    /// Viewport height in lines
    height: usize,

    /// Viewport width in columns
    width: usize,

    /// Longest line width (for horizontal scrolling bounds)
    max_line_width: usize,

    /// Mouse wheel scroll delta (lines per wheel tick)
    mouse_wheel_delta: usize,

    /// Whether mouse wheel scrolling is enabled
    mouse_wheel_enabled: bool,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            y_offset: 0,
            x_offset: 0,
            height: 10,
            width: 80,
            max_line_width: 0,
            mouse_wheel_delta: 3,
            mouse_wheel_enabled: true,
        }
    }
}

impl ViewportState {
    /// Create a new viewport state with given dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Set the content string (will be split into lines)
    pub fn set_content(&mut self, content: &str) {
        // Normalize line endings and split
        let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
        self.lines = normalized.lines().map(String::from).collect();

        // Calculate max line width for horizontal scrolling
        self.max_line_width = self
            .lines
            .iter()
            .map(|l| unicode_width(l))
            .max()
            .unwrap_or(0);

        // Clamp scroll position to valid range
        self.clamp_scroll();
    }

    /// Set content from pre-split lines
    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.max_line_width = lines.iter().map(|l| unicode_width(l)).max().unwrap_or(0);
        self.lines = lines;
        self.clamp_scroll();
    }

    /// Append a line to the content
    pub fn push_line(&mut self, line: String) {
        let line_width = unicode_width(&line);
        if line_width > self.max_line_width {
            self.max_line_width = line_width;
        }
        self.lines.push(line);
    }

    /// Append multiple lines
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = String>) {
        for line in lines {
            self.push_line(line);
        }
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.lines.clear();
        self.y_offset = 0;
        self.x_offset = 0;
        self.max_line_width = 0;
    }

    /// Set viewport dimensions
    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.clamp_scroll();
    }

    /// Set viewport height
    pub fn set_height(&mut self, height: usize) {
        self.height = height;
        self.clamp_scroll();
    }

    /// Set viewport width
    pub fn set_width(&mut self, width: usize) {
        self.width = width;
        self.clamp_scroll();
    }

    /// Get viewport height
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get viewport width
    pub fn width(&self) -> usize {
        self.width
    }

    // ========== Scroll Position ==========

    /// Get current vertical scroll offset
    pub fn y_offset(&self) -> usize {
        self.y_offset
    }

    /// Get current horizontal scroll offset
    pub fn x_offset(&self) -> usize {
        self.x_offset
    }

    /// Set vertical scroll offset directly
    pub fn set_y_offset(&mut self, offset: usize) {
        self.y_offset = offset;
        self.clamp_scroll();
    }

    /// Set horizontal scroll offset directly
    pub fn set_x_offset(&mut self, offset: usize) {
        self.x_offset = offset;
        self.clamp_scroll();
    }

    // ========== Vertical Scrolling ==========

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.y_offset = self.y_offset.saturating_add(n);
        self.clamp_scroll();
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.y_offset = self.y_offset.saturating_sub(n);
    }

    /// Scroll down by one page (viewport height)
    pub fn page_down(&mut self) {
        self.scroll_down(self.height);
    }

    /// Scroll up by one page (viewport height)
    pub fn page_up(&mut self) {
        self.scroll_up(self.height);
    }

    /// Scroll down by half a page
    pub fn half_page_down(&mut self) {
        self.scroll_down(self.height / 2);
    }

    /// Scroll up by half a page
    pub fn half_page_up(&mut self) {
        self.scroll_up(self.height / 2);
    }

    /// Jump to the top of the content
    pub fn goto_top(&mut self) {
        self.y_offset = 0;
    }

    /// Jump to the bottom of the content
    pub fn goto_bottom(&mut self) {
        self.y_offset = self.max_y_offset();
    }

    // ========== Horizontal Scrolling ==========

    /// Scroll right by n columns
    pub fn scroll_right(&mut self, n: usize) {
        self.x_offset = self.x_offset.saturating_add(n);
        self.clamp_scroll();
    }

    /// Scroll left by n columns
    pub fn scroll_left(&mut self, n: usize) {
        self.x_offset = self.x_offset.saturating_sub(n);
    }

    /// Jump to the left edge
    pub fn goto_left(&mut self) {
        self.x_offset = 0;
    }

    /// Jump to the right edge
    pub fn goto_right(&mut self) {
        self.x_offset = self.max_x_offset();
    }

    // ========== Mouse Wheel ==========

    /// Handle mouse wheel scroll (returns true if handled)
    pub fn handle_mouse_wheel(&mut self, delta_y: i32, delta_x: i32) -> bool {
        if !self.mouse_wheel_enabled {
            return false;
        }

        if delta_y < 0 {
            self.scroll_up(self.mouse_wheel_delta);
        } else if delta_y > 0 {
            self.scroll_down(self.mouse_wheel_delta);
        }

        if delta_x < 0 {
            self.scroll_left(self.mouse_wheel_delta);
        } else if delta_x > 0 {
            self.scroll_right(self.mouse_wheel_delta);
        }

        delta_y != 0 || delta_x != 0
    }

    /// Set mouse wheel delta (lines per tick)
    pub fn set_mouse_wheel_delta(&mut self, delta: usize) {
        self.mouse_wheel_delta = delta;
    }

    /// Enable or disable mouse wheel scrolling
    pub fn set_mouse_wheel_enabled(&mut self, enabled: bool) {
        self.mouse_wheel_enabled = enabled;
    }

    // ========== Status Queries ==========

    /// Check if scrolled to the top
    pub fn at_top(&self) -> bool {
        self.y_offset == 0
    }

    /// Check if scrolled to the bottom
    pub fn at_bottom(&self) -> bool {
        self.y_offset >= self.max_y_offset()
    }

    /// Check if scrolled to the left edge
    pub fn at_left(&self) -> bool {
        self.x_offset == 0
    }

    /// Check if scrolled to the right edge
    pub fn at_right(&self) -> bool {
        self.x_offset >= self.max_x_offset()
    }

    /// Get scroll percentage (0.0 to 1.0)
    pub fn scroll_percent(&self) -> f64 {
        let max = self.max_y_offset();
        if max == 0 {
            return 1.0;
        }
        self.y_offset as f64 / max as f64
    }

    /// Get horizontal scroll percentage (0.0 to 1.0)
    pub fn scroll_percent_x(&self) -> f64 {
        let max = self.max_x_offset();
        if max == 0 {
            return 1.0;
        }
        self.x_offset as f64 / max as f64
    }

    /// Total number of lines in content
    pub fn total_line_count(&self) -> usize {
        self.lines.len()
    }

    /// Number of visible lines (min of content and viewport height)
    pub fn visible_line_count(&self) -> usize {
        cmp::min(self.lines.len().saturating_sub(self.y_offset), self.height)
    }

    /// Check if content fits in viewport (no scrolling needed)
    pub fn fits_in_viewport(&self) -> bool {
        self.lines.len() <= self.height
    }

    /// Check if horizontal scrolling is needed
    pub fn needs_horizontal_scroll(&self) -> bool {
        self.max_line_width > self.width
    }

    // ========== Content Access ==========

    /// Get all content lines
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get visible lines (respecting scroll offset)
    pub fn visible_lines(&self) -> impl Iterator<Item = &str> {
        self.lines
            .iter()
            .skip(self.y_offset)
            .take(self.height)
            .map(|s| s.as_str())
    }

    /// Get visible lines with horizontal offset applied
    pub fn visible_lines_clipped(&self) -> Vec<String> {
        self.lines
            .iter()
            .skip(self.y_offset)
            .take(self.height)
            .map(|line| clip_line(line, self.x_offset, self.width))
            .collect()
    }

    /// Get a specific line by index
    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|s| s.as_str())
    }

    // ========== Internal Helpers ==========

    /// Maximum valid Y offset
    fn max_y_offset(&self) -> usize {
        self.lines.len().saturating_sub(self.height)
    }

    /// Maximum valid X offset
    fn max_x_offset(&self) -> usize {
        self.max_line_width.saturating_sub(self.width)
    }

    /// Clamp scroll position to valid range
    fn clamp_scroll(&mut self) {
        self.y_offset = cmp::min(self.y_offset, self.max_y_offset());
        self.x_offset = cmp::min(self.x_offset, self.max_x_offset());
    }
}

/// Calculate unicode display width of a string
fn unicode_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    s.width()
}

/// Clip a line to the visible portion based on x_offset and width
fn clip_line(line: &str, x_offset: usize, width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    let mut result = String::new();
    let mut current_width = 0;
    let mut chars_skipped = 0;

    for ch in line.chars() {
        let char_width = ch.width().unwrap_or(0);

        // Skip characters until we reach x_offset
        if chars_skipped < x_offset {
            chars_skipped += char_width;
            continue;
        }

        // Stop if we've filled the width
        if current_width + char_width > width {
            break;
        }

        result.push(ch);
        current_width += char_width;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_content() {
        let mut state = ViewportState::new(80, 10);
        state.set_content("line1\nline2\nline3");

        assert_eq!(state.total_line_count(), 3);
        assert_eq!(state.line(0), Some("line1"));
        assert_eq!(state.line(1), Some("line2"));
        assert_eq!(state.line(2), Some("line3"));
    }

    #[test]
    fn test_scroll_down_up() {
        let mut state = ViewportState::new(80, 5);
        state.set_content("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");

        assert_eq!(state.y_offset(), 0);
        assert!(state.at_top());

        state.scroll_down(3);
        assert_eq!(state.y_offset(), 3);
        assert!(!state.at_top());

        state.scroll_up(2);
        assert_eq!(state.y_offset(), 1);

        state.scroll_up(10); // Should clamp to 0
        assert_eq!(state.y_offset(), 0);
        assert!(state.at_top());
    }

    #[test]
    fn test_page_navigation() {
        let mut state = ViewportState::new(80, 5);
        state.set_content("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15");

        state.page_down();
        assert_eq!(state.y_offset(), 5);

        state.page_down();
        assert_eq!(state.y_offset(), 10);

        state.page_up();
        assert_eq!(state.y_offset(), 5);
    }

    #[test]
    fn test_goto_top_bottom() {
        let mut state = ViewportState::new(80, 5);
        state.set_content("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");

        state.goto_bottom();
        assert!(state.at_bottom());
        assert_eq!(state.y_offset(), 5); // 10 lines - 5 height = 5

        state.goto_top();
        assert!(state.at_top());
        assert_eq!(state.y_offset(), 0);
    }

    #[test]
    fn test_scroll_percent() {
        let mut state = ViewportState::new(80, 5);
        state.set_content("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");

        assert_eq!(state.scroll_percent(), 0.0);

        state.goto_bottom();
        assert_eq!(state.scroll_percent(), 1.0);

        state.set_y_offset(2);
        assert!((state.scroll_percent() - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_visible_lines() {
        let mut state = ViewportState::new(80, 3);
        state.set_content("a\nb\nc\nd\ne");

        let visible: Vec<_> = state.visible_lines().collect();
        assert_eq!(visible, vec!["a", "b", "c"]);

        state.scroll_down(2);
        let visible: Vec<_> = state.visible_lines().collect();
        assert_eq!(visible, vec!["c", "d", "e"]);
    }

    #[test]
    fn test_fits_in_viewport() {
        let mut state = ViewportState::new(80, 10);
        state.set_content("a\nb\nc");
        assert!(state.fits_in_viewport());

        state.set_content("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11");
        assert!(!state.fits_in_viewport());
    }

    #[test]
    fn test_horizontal_scroll() {
        let mut state = ViewportState::new(10, 5);
        state.set_content("short\nthis is a very long line that needs scrolling\nend");

        assert!(state.needs_horizontal_scroll());
        assert!(state.at_left());

        state.scroll_right(5);
        assert_eq!(state.x_offset(), 5);
        assert!(!state.at_left());

        state.goto_left();
        assert!(state.at_left());
    }
}
