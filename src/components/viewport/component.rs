//! Viewport component for scrollable text content
//!
//! A high-performance scrollable view for displaying large text content,
//! similar to Bubbles' viewport component.

use crate::components::{Box as TinkBox, Text};
use crate::core::{BorderStyle, Color, Element, FlexDirection, Overflow};

use super::keymap::{ViewportAction, ViewportKeyMap};
use super::state::ViewportState;

/// Style configuration for the viewport
#[derive(Debug, Clone)]
pub struct ViewportStyle {
    /// Border style
    pub border: Option<BorderStyle>,
    /// Border color
    pub border_color: Option<Color>,
    /// Background color
    pub background: Option<Color>,
    /// Text color
    pub text_color: Option<Color>,
    /// Show line numbers
    pub line_numbers: bool,
    /// Line number color
    pub line_number_color: Option<Color>,
    /// Line number width (auto-calculated if None)
    pub line_number_width: Option<usize>,
    /// Show scrollbar
    pub scrollbar: bool,
    /// Scrollbar color
    pub scrollbar_color: Option<Color>,
    /// Scrollbar track color
    pub scrollbar_track_color: Option<Color>,
}

impl Default for ViewportStyle {
    fn default() -> Self {
        Self {
            border: None,
            border_color: None,
            background: None,
            text_color: None,
            line_numbers: false,
            line_number_color: Some(Color::BrightBlack),
            line_number_width: None,
            scrollbar: false,
            scrollbar_color: Some(Color::BrightBlack),
            scrollbar_track_color: None,
        }
    }
}

impl ViewportStyle {
    /// Create a new style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set border style
    pub fn border(mut self, style: BorderStyle) -> Self {
        self.border = Some(style);
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set text color
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Enable line numbers
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Set line number color
    pub fn line_number_color(mut self, color: Color) -> Self {
        self.line_number_color = Some(color);
        self
    }

    /// Enable scrollbar
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.scrollbar = show;
        self
    }

    /// Set scrollbar color
    pub fn scrollbar_color(mut self, color: Color) -> Self {
        self.scrollbar_color = Some(color);
        self
    }
}

/// Viewport component for scrollable text content
///
/// # Example
///
/// ```ignore
/// use rnk::components::viewport::{Viewport, ViewportState};
/// use rnk::hooks::use_signal;
///
/// fn app() -> Element {
///     let state = use_signal(|| {
///         let mut s = ViewportState::new(80, 20);
///         s.set_content(include_str!("long_file.txt"));
///         s
///     });
///
///     Viewport::new(&state.get())
///         .into_element()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Viewport<'a> {
    /// Reference to viewport state
    state: &'a ViewportState,
    /// Style configuration
    style: ViewportStyle,
    /// Key mapping
    keymap: ViewportKeyMap,
    /// Whether the viewport is focused (receives input)
    focused: bool,
}

impl<'a> Viewport<'a> {
    /// Create a new viewport with the given state
    pub fn new(state: &'a ViewportState) -> Self {
        Self {
            state,
            style: ViewportStyle::default(),
            keymap: ViewportKeyMap::default(),
            focused: true,
        }
    }

    /// Set the style
    pub fn style(mut self, style: ViewportStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the keymap
    pub fn keymap(mut self, keymap: ViewportKeyMap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Set whether the viewport is focused
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Enable or disable line numbers
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.style.line_numbers = show;
        self
    }

    /// Enable or disable scrollbar
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.style.scrollbar = show;
        self
    }

    /// Set border style
    pub fn border(mut self, style: BorderStyle) -> Self {
        self.style.border = Some(style);
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    /// Get the keymap for external input handling
    pub fn get_keymap(&self) -> &ViewportKeyMap {
        &self.keymap
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let height = self.state.height();
        let width = self.state.width();

        // Build the content
        let mut container = TinkBox::new()
            .flex_direction(FlexDirection::Column)
            .height(height as i32)
            .width(width as i32)
            .overflow_y(Overflow::Hidden);

        // Apply border if set
        if let Some(border) = self.style.border {
            container = container.border_style(border);
        }
        if let Some(color) = self.style.border_color {
            container = container.border_color(color);
        }
        if let Some(color) = self.style.background {
            container = container.background(color);
        }

        // Calculate line number width
        let line_num_width = if self.style.line_numbers {
            self.style.line_number_width.unwrap_or_else(|| {
                let total = self.state.total_line_count();
                if total == 0 {
                    1
                } else {
                    (total as f64).log10().floor() as usize + 1
                }
            })
        } else {
            0
        };

        // Render visible lines
        let y_offset = self.state.y_offset();
        for (i, line) in self.state.visible_lines().enumerate() {
            let global_line_num = y_offset + i + 1;

            let line_element = if self.style.line_numbers {
                // Line with line number
                let num_str = format!("{:>width$} ", global_line_num, width = line_num_width);
                let mut num_text = Text::new(&num_str);
                if let Some(color) = self.style.line_number_color {
                    num_text = num_text.color(color);
                }
                num_text = num_text.dim();

                let mut content_text = Text::new(line);
                if let Some(color) = self.style.text_color {
                    content_text = content_text.color(color);
                }

                TinkBox::new()
                    .flex_direction(FlexDirection::Row)
                    .child(num_text.into_element())
                    .child(content_text.into_element())
                    .into_element()
            } else {
                // Line without line number
                let mut text = Text::new(line);
                if let Some(color) = self.style.text_color {
                    text = text.color(color);
                }
                text.into_element()
            };

            container = container.child(line_element);
        }

        // Add scrollbar if enabled
        if self.style.scrollbar && !self.state.fits_in_viewport() {
            // Scrollbar is rendered separately
            let scrollbar = self.render_scrollbar();
            return TinkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(container.into_element())
                .child(scrollbar)
                .into_element();
        }

        container.into_element()
    }

    /// Render the scrollbar
    fn render_scrollbar(&self) -> Element {
        let height = self.state.height();
        let total_lines = self.state.total_line_count();

        if total_lines == 0 || height == 0 {
            return TinkBox::new().into_element();
        }

        // Calculate scrollbar thumb position and size
        let thumb_size = ((height as f64 / total_lines as f64) * height as f64)
            .max(1.0)
            .min(height as f64) as usize;
        let thumb_pos = (self.state.scroll_percent() * (height - thumb_size) as f64) as usize;

        let mut scrollbar_box = TinkBox::new()
            .flex_direction(FlexDirection::Column)
            .width(1);

        for i in 0..height {
            let is_thumb = i >= thumb_pos && i < thumb_pos + thumb_size;
            let char = if is_thumb { "█" } else { "░" };

            let mut text = Text::new(char);
            if is_thumb {
                if let Some(color) = self.style.scrollbar_color {
                    text = text.color(color);
                }
            } else if let Some(color) = self.style.scrollbar_track_color {
                text = text.color(color);
            }

            scrollbar_box = scrollbar_box.child(text.into_element());
        }

        scrollbar_box.into_element()
    }
}

/// Handle viewport input and return the action to perform
pub fn handle_viewport_input(
    state: &mut ViewportState,
    input: &str,
    key: &crate::hooks::Key,
    keymap: &ViewportKeyMap,
) -> bool {
    if let Some(action) = keymap.match_action(input, key) {
        apply_viewport_action(state, action);
        true
    } else {
        false
    }
}

/// Apply a viewport action to the state
pub fn apply_viewport_action(state: &mut ViewportState, action: ViewportAction) {
    match action {
        ViewportAction::ScrollUp => state.scroll_up(1),
        ViewportAction::ScrollDown => state.scroll_down(1),
        ViewportAction::PageUp => state.page_up(),
        ViewportAction::PageDown => state.page_down(),
        ViewportAction::HalfPageUp => state.half_page_up(),
        ViewportAction::HalfPageDown => state.half_page_down(),
        ViewportAction::GotoTop => state.goto_top(),
        ViewportAction::GotoBottom => state.goto_bottom(),
        ViewportAction::ScrollLeft => state.scroll_left(1),
        ViewportAction::ScrollRight => state.scroll_right(1),
        ViewportAction::GotoLeft => state.goto_left(),
        ViewportAction::GotoRight => state.goto_right(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_creation() {
        let mut state = ViewportState::new(80, 10);
        state.set_content("line1\nline2\nline3");

        let viewport = Viewport::new(&state);
        let element = viewport.into_element();

        assert_eq!(element.children.len(), 3);
    }

    #[test]
    fn test_viewport_with_line_numbers() {
        let mut state = ViewportState::new(80, 10);
        state.set_content("a\nb\nc");

        let viewport = Viewport::new(&state).line_numbers(true);
        let element = viewport.into_element();

        assert_eq!(element.children.len(), 3);
    }

    #[test]
    fn test_viewport_with_scrollbar() {
        let mut state = ViewportState::new(80, 5);
        state.set_content("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");

        let viewport = Viewport::new(&state).scrollbar(true);
        let element = viewport.into_element();

        // Should have 2 children: content box and scrollbar
        assert_eq!(element.children.len(), 2);
    }
}
