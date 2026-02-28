//! Cursor component for blinking text cursor
//!
//! Provides a customizable blinking cursor for text input fields.

use crate::components::Text;
use crate::core::{Color, Element};

/// Cursor shape variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorShape {
    /// Block cursor (█)
    #[default]
    Block,
    /// Underline cursor (_)
    Underline,
    /// Bar/Line cursor (|)
    Bar,
    /// Custom character
    Custom(char),
}

impl CursorShape {
    /// Get the character representation of the cursor
    pub fn char(&self) -> char {
        match self {
            CursorShape::Block => '█',
            CursorShape::Underline => '_',
            CursorShape::Bar => '│',
            CursorShape::Custom(c) => *c,
        }
    }
}

/// Cursor style configuration
#[derive(Debug, Clone)]
pub struct CursorStyle {
    /// Cursor shape
    pub shape: CursorShape,
    /// Cursor color
    pub color: Option<Color>,
    /// Whether cursor blinks
    pub blink: bool,
    /// Blink interval in milliseconds
    pub blink_interval_ms: u64,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Block,
            color: None,
            blink: true,
            blink_interval_ms: 530,
        }
    }
}

impl CursorStyle {
    /// Create a new cursor style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set cursor shape
    pub fn shape(mut self, shape: CursorShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set cursor color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set whether cursor blinks
    pub fn blink(mut self, blink: bool) -> Self {
        self.blink = blink;
        self
    }

    /// Set blink interval in milliseconds
    pub fn blink_interval(mut self, ms: u64) -> Self {
        self.blink_interval_ms = ms;
        self
    }

    /// Block cursor style
    pub fn block() -> Self {
        Self::default().shape(CursorShape::Block)
    }

    /// Underline cursor style
    pub fn underline() -> Self {
        Self::default().shape(CursorShape::Underline)
    }

    /// Bar cursor style
    pub fn bar() -> Self {
        Self::default().shape(CursorShape::Bar)
    }

    /// Smooth blinking cursor style (faster blink for fade effect)
    ///
    /// Creates a cursor optimized for smooth fade animations.
    /// Use with `CursorState::opacity()` for smooth transitions.
    pub fn smooth() -> Self {
        Self::new()
            .shape(CursorShape::Block)
            .blink(true)
            .blink_interval(400)
    }
}

/// Cursor state for managing blink animation
#[derive(Debug, Clone)]
pub struct CursorState {
    /// Whether cursor is currently visible (for blink animation)
    visible: bool,
    /// Whether cursor is active/focused
    active: bool,
    /// Style configuration
    style: CursorStyle,
    /// Last toggle timestamp (for blink timing)
    last_toggle_ms: u64,
}

impl Default for CursorState {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorState {
    /// Create a new cursor state
    pub fn new() -> Self {
        Self {
            visible: true,
            active: true,
            style: CursorStyle::default(),
            last_toggle_ms: 0,
        }
    }

    /// Create cursor state with custom style
    pub fn with_style(style: CursorStyle) -> Self {
        Self {
            visible: true,
            active: true,
            style,
            last_toggle_ms: 0,
        }
    }

    /// Check if cursor is visible
    pub fn is_visible(&self) -> bool {
        self.visible && self.active
    }

    /// Check if cursor is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set cursor active state
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if active {
            self.visible = true;
        }
    }

    /// Toggle visibility (for blink animation)
    pub fn toggle_visibility(&mut self) {
        if self.style.blink {
            self.visible = !self.visible;
        }
    }

    /// Update blink state based on elapsed time
    pub fn update(&mut self, current_time_ms: u64) {
        if !self.style.blink || !self.active {
            return;
        }

        let elapsed = current_time_ms.saturating_sub(self.last_toggle_ms);
        if elapsed >= self.style.blink_interval_ms {
            self.visible = !self.visible;
            self.last_toggle_ms = current_time_ms;
        }
    }

    /// Reset cursor to visible state
    pub fn reset(&mut self) {
        self.visible = true;
        self.last_toggle_ms = 0;
    }

    /// Get the cursor style
    pub fn style(&self) -> &CursorStyle {
        &self.style
    }

    /// Set the cursor style
    pub fn set_style(&mut self, style: CursorStyle) {
        self.style = style;
    }

    /// Get cursor character
    pub fn char(&self) -> char {
        self.style.shape.char()
    }

    /// Get opacity for smooth animation (0.0 - 1.0)
    ///
    /// Returns a value between 0.0 and 1.0 based on the current blink cycle.
    /// Use this for smooth fade animations instead of binary on/off blinking.
    ///
    /// # Example
    ///
    /// ```
    /// use rnk::components::{CursorState, CursorStyle};
    ///
    /// let mut state = CursorState::with_style(CursorStyle::smooth());
    /// state.update(200); // Update with current time
    /// let opacity = state.opacity(200);
    /// // opacity will be between 0.0 and 1.0
    /// ```
    pub fn opacity(&self, current_time_ms: u64) -> f32 {
        if !self.style.blink || !self.active {
            return 1.0;
        }

        let interval = self.style.blink_interval_ms as f32;
        let full_cycle = interval * 2.0;
        let cycle_position = (current_time_ms % (full_cycle as u64)) as f32;

        // Smooth sine wave for fade effect
        // First half: fade out (1.0 -> 0.0)
        // Second half: fade in (0.0 -> 1.0)
        if cycle_position < interval {
            // Fade out
            1.0 - (cycle_position / interval)
        } else {
            // Fade in
            (cycle_position - interval) / interval
        }
    }

    /// Get smooth opacity using cosine interpolation
    ///
    /// Similar to `opacity()` but uses cosine for smoother transitions.
    pub fn smooth_opacity(&self, current_time_ms: u64) -> f32 {
        if !self.style.blink || !self.active {
            return 1.0;
        }

        let interval = self.style.blink_interval_ms as f32;
        let full_cycle = interval * 2.0;
        let cycle_position = (current_time_ms % (full_cycle as u64)) as f32;

        // Use cosine for smooth easing
        let t = cycle_position / full_cycle;
        (1.0 + (t * std::f32::consts::PI * 2.0).cos()) / 2.0
    }
}

/// Cursor component for rendering
#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    state: &'a CursorState,
    /// Override character to show when cursor is hidden
    placeholder: Option<char>,
}

impl<'a> Cursor<'a> {
    /// Create a new cursor component
    pub fn new(state: &'a CursorState) -> Self {
        Self {
            state,
            placeholder: None,
        }
    }

    /// Set placeholder character when cursor is hidden
    pub fn placeholder(mut self, ch: char) -> Self {
        self.placeholder = Some(ch);
        self
    }

    /// Render the cursor to a string
    pub fn render(&self) -> String {
        if self.state.is_visible() {
            let ch = self.state.char();
            if let Some(color) = &self.state.style.color {
                format!("{}{}\x1b[0m", color.to_ansi_fg(), ch)
            } else {
                ch.to_string()
            }
        } else if let Some(placeholder) = self.placeholder {
            placeholder.to_string()
        } else {
            " ".to_string()
        }
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let ch = if self.state.is_visible() {
            self.state.char().to_string()
        } else if let Some(placeholder) = self.placeholder {
            placeholder.to_string()
        } else {
            " ".to_string()
        };

        let mut text = Text::new(ch);
        if let Some(color) = &self.state.style.color {
            text = text.color(*color);
        }
        text.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_shape() {
        assert_eq!(CursorShape::Block.char(), '█');
        assert_eq!(CursorShape::Underline.char(), '_');
        assert_eq!(CursorShape::Bar.char(), '│');
        assert_eq!(CursorShape::Custom('▌').char(), '▌');
    }

    #[test]
    fn test_cursor_style_builder() {
        let style = CursorStyle::new()
            .shape(CursorShape::Bar)
            .color(Color::Cyan)
            .blink(false)
            .blink_interval(1000);

        assert_eq!(style.shape, CursorShape::Bar);
        assert_eq!(style.color, Some(Color::Cyan));
        assert!(!style.blink);
        assert_eq!(style.blink_interval_ms, 1000);
    }

    #[test]
    fn test_cursor_style_presets() {
        let block = CursorStyle::block();
        assert_eq!(block.shape, CursorShape::Block);

        let underline = CursorStyle::underline();
        assert_eq!(underline.shape, CursorShape::Underline);

        let bar = CursorStyle::bar();
        assert_eq!(bar.shape, CursorShape::Bar);
    }

    #[test]
    fn test_cursor_state_visibility() {
        let mut state = CursorState::new();
        assert!(state.is_visible());

        state.toggle_visibility();
        assert!(!state.is_visible());

        state.toggle_visibility();
        assert!(state.is_visible());
    }

    #[test]
    fn test_cursor_state_active() {
        let mut state = CursorState::new();
        assert!(state.is_active());

        state.set_active(false);
        assert!(!state.is_active());
        assert!(!state.is_visible());

        state.set_active(true);
        assert!(state.is_active());
        assert!(state.is_visible());
    }

    #[test]
    fn test_cursor_state_update() {
        let style = CursorStyle::new().blink_interval(100);
        let mut state = CursorState::with_style(style);

        assert!(state.is_visible());

        // Simulate time passing
        state.update(150);
        assert!(!state.is_visible());

        state.update(300);
        assert!(state.is_visible());
    }

    #[test]
    fn test_cursor_state_no_blink() {
        let style = CursorStyle::new().blink(false);
        let mut state = CursorState::with_style(style);

        assert!(state.is_visible());
        state.update(1000);
        assert!(state.is_visible()); // Should stay visible
    }

    #[test]
    fn test_cursor_render() {
        let state = CursorState::new();
        let cursor = Cursor::new(&state);
        assert_eq!(cursor.render(), "█");
    }

    #[test]
    fn test_cursor_render_hidden() {
        let mut state = CursorState::new();
        state.toggle_visibility();

        let cursor = Cursor::new(&state);
        assert_eq!(cursor.render(), " ");

        let cursor_with_placeholder = Cursor::new(&state).placeholder('_');
        assert_eq!(cursor_with_placeholder.render(), "_");
    }

    #[test]
    fn test_cursor_render_with_color() {
        let style = CursorStyle::new().color(Color::Cyan);
        let state = CursorState::with_style(style);
        let cursor = Cursor::new(&state);
        let rendered = cursor.render();
        assert!(rendered.contains("█"));
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn test_cursor_reset() {
        let mut state = CursorState::new();
        state.toggle_visibility();
        assert!(!state.is_visible());

        state.reset();
        assert!(state.is_visible());
    }

    #[test]
    fn test_cursor_style_smooth() {
        let style = CursorStyle::smooth();
        assert_eq!(style.shape, CursorShape::Block);
        assert!(style.blink);
        assert_eq!(style.blink_interval_ms, 400);
    }

    #[test]
    fn test_cursor_opacity() {
        let style = CursorStyle::new().blink(true).blink_interval(100);
        let state = CursorState::with_style(style);

        // At t=0, opacity should be 1.0 (start of fade out)
        let opacity = state.opacity(0);
        assert!((opacity - 1.0).abs() < 0.01);

        // At t=50, opacity should be around 0.5 (middle of fade out)
        let opacity = state.opacity(50);
        assert!((opacity - 0.5).abs() < 0.01);

        // At t=100, opacity should be 0.0 (end of fade out)
        let opacity = state.opacity(100);
        assert!(opacity < 0.01);

        // At t=150, opacity should be around 0.5 (middle of fade in)
        let opacity = state.opacity(150);
        assert!((opacity - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cursor_opacity_no_blink() {
        let style = CursorStyle::new().blink(false);
        let state = CursorState::with_style(style);

        // Should always return 1.0 when blink is disabled
        assert_eq!(state.opacity(0), 1.0);
        assert_eq!(state.opacity(500), 1.0);
        assert_eq!(state.opacity(1000), 1.0);
    }

    #[test]
    fn test_cursor_smooth_opacity() {
        let style = CursorStyle::smooth();
        let state = CursorState::with_style(style);

        // Smooth opacity should be between 0.0 and 1.0
        let opacity = state.smooth_opacity(200);
        assert!((0.0..=1.0).contains(&opacity));
    }
}
