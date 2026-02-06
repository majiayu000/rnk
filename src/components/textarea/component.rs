//! TextArea component for multi-line text editing
//!
//! A multi-line text input component similar to Bubbles' textarea.

use crate::components::{Box as RnkBox, Text};
use crate::core::{BorderStyle, Color, Element, FlexDirection, Overflow};

use super::keymap::{TextAreaAction, TextAreaKeyMap};
use super::state::TextAreaState;

/// Style configuration for the textarea
#[derive(Debug, Clone)]
pub struct TextAreaStyle {
    /// Border style when focused
    pub focused_border: Option<BorderStyle>,
    /// Border style when blurred
    pub blurred_border: Option<BorderStyle>,
    /// Border color when focused
    pub focused_border_color: Option<Color>,
    /// Border color when blurred
    pub blurred_border_color: Option<Color>,
    /// Background color
    pub background: Option<Color>,
    /// Text color
    pub text_color: Option<Color>,
    /// Placeholder text color
    pub placeholder_color: Option<Color>,
    /// Cursor color
    pub cursor_color: Option<Color>,
    /// Selection background color
    pub selection_color: Option<Color>,
    /// Line number color
    pub line_number_color: Option<Color>,
    /// Show line numbers
    pub line_numbers: bool,
    /// Cursor character
    pub cursor_char: char,
    /// Prompt string (shown before each line)
    pub prompt: String,
}

impl Default for TextAreaStyle {
    fn default() -> Self {
        Self {
            focused_border: Some(BorderStyle::Round),
            blurred_border: Some(BorderStyle::Round),
            focused_border_color: Some(Color::Cyan),
            blurred_border_color: Some(Color::BrightBlack),
            background: None,
            text_color: None,
            placeholder_color: Some(Color::BrightBlack),
            cursor_color: Some(Color::Cyan),
            selection_color: Some(Color::Blue),
            line_number_color: Some(Color::BrightBlack),
            line_numbers: false,
            cursor_char: '█',
            prompt: String::new(),
        }
    }
}

impl TextAreaStyle {
    /// Create a new style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set focused border style
    pub fn focused_border(mut self, style: BorderStyle) -> Self {
        self.focused_border = Some(style);
        self
    }

    /// Set blurred border style
    pub fn blurred_border(mut self, style: BorderStyle) -> Self {
        self.blurred_border = Some(style);
        self
    }

    /// Set focused border color
    pub fn focused_border_color(mut self, color: Color) -> Self {
        self.focused_border_color = Some(color);
        self
    }

    /// Set blurred border color
    pub fn blurred_border_color(mut self, color: Color) -> Self {
        self.blurred_border_color = Some(color);
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

    /// Set placeholder color
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = Some(color);
        self
    }

    /// Set cursor color
    pub fn cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = Some(color);
        self
    }

    /// Enable line numbers
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Set cursor character
    pub fn cursor_char(mut self, ch: char) -> Self {
        self.cursor_char = ch;
        self
    }

    /// Set prompt string
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }
}

/// TextArea component for multi-line text editing
///
/// # Example
///
/// ```ignore
/// use rnk::components::textarea::{TextArea, TextAreaState};
/// use rnk::hooks::use_signal;
///
/// fn app() -> Element {
///     let state = use_signal(TextAreaState::new);
///
///     TextArea::new(&state.get())
///         .focused(true)
///         .into_element()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TextArea<'a> {
    /// Reference to textarea state
    state: &'a TextAreaState,
    /// Style configuration
    style: TextAreaStyle,
    /// Key mapping
    keymap: TextAreaKeyMap,
    /// Whether the textarea is focused
    focused: bool,
    /// Width in characters
    width: Option<usize>,
    /// Height in lines
    height: Option<usize>,
}

impl<'a> TextArea<'a> {
    /// Create a new textarea with the given state
    pub fn new(state: &'a TextAreaState) -> Self {
        Self {
            state,
            style: TextAreaStyle::default(),
            keymap: TextAreaKeyMap::default(),
            focused: true,
            width: None,
            height: None,
        }
    }

    /// Set the style
    pub fn style(mut self, style: TextAreaStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the keymap
    pub fn keymap(mut self, keymap: TextAreaKeyMap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Set whether the textarea is focused
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set width
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set height
    pub fn height(mut self, height: usize) -> Self {
        self.height = Some(height);
        self
    }

    /// Enable line numbers
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.style.line_numbers = show;
        self
    }

    /// Set prompt string
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.style.prompt = prompt.into();
        self
    }

    /// Get the keymap for external input handling
    pub fn get_keymap(&self) -> &TextAreaKeyMap {
        &self.keymap
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let height = self.height.unwrap_or(self.state.viewport_height());

        // Build the container
        let mut container = RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .overflow_y(Overflow::Hidden);

        if let Some(w) = self.width {
            container = container.width(w as i32);
        }
        container = container.height(height as i32);

        // Apply border based on focus state
        if self.focused {
            if let Some(border) = self.style.focused_border {
                container = container.border_style(border);
            }
            if let Some(color) = self.style.focused_border_color {
                container = container.border_color(color);
            }
        } else {
            if let Some(border) = self.style.blurred_border {
                container = container.border_style(border);
            }
            if let Some(color) = self.style.blurred_border_color {
                container = container.border_color(color);
            }
        }

        if let Some(color) = self.style.background {
            container = container.background(color);
        }

        // Calculate line number width
        let line_num_width = if self.style.line_numbers {
            let total = self.state.line_count();
            if total == 0 {
                1
            } else {
                (total as f64).log10().floor() as usize + 1
            }
        } else {
            0
        };

        // Check if we should show placeholder
        if self.state.is_empty() && !self.state.placeholder().is_empty() {
            let mut placeholder_text = Text::new(self.state.placeholder());
            if let Some(color) = self.style.placeholder_color {
                placeholder_text = placeholder_text.color(color);
            }
            container = container.child(placeholder_text.into_element());
            return container.into_element();
        }

        // Render visible lines
        let cursor = self.state.cursor();
        let scroll_offset = self.state.scroll_offset();

        for (row, line) in self.state.visible_lines() {
            let is_cursor_line = row == cursor.row;

            let line_element = self.render_line(row, line, is_cursor_line, line_num_width);

            container = container.child(line_element);
        }

        // Fill remaining lines if needed
        let visible_count = self
            .state
            .line_count()
            .saturating_sub(scroll_offset)
            .min(height);
        for _ in visible_count..height {
            let empty_line = if self.style.line_numbers {
                let padding = " ".repeat(line_num_width + 1);
                Text::new(&padding).into_element()
            } else {
                Text::new("").into_element()
            };
            container = container.child(empty_line);
        }

        container.into_element()
    }

    /// Render a single line
    fn render_line(
        &self,
        row: usize,
        line: &str,
        is_cursor_line: bool,
        line_num_width: usize,
    ) -> Element {
        let cursor = self.state.cursor();
        let mut parts = Vec::new();

        // Line number
        if self.style.line_numbers {
            let num_str = format!("{:>width$} ", row + 1, width = line_num_width);
            let mut num_text = Text::new(&num_str);
            if let Some(color) = self.style.line_number_color {
                num_text = num_text.color(color);
            }
            num_text = num_text.dim();
            parts.push(num_text.into_element());
        }

        // Prompt
        if !self.style.prompt.is_empty() {
            parts.push(Text::new(&self.style.prompt).into_element());
        }

        // Line content with cursor
        if is_cursor_line && self.focused {
            let content = self.render_line_with_cursor(line, cursor.col);
            parts.push(content);
        } else {
            let mut text = Text::new(line);
            if let Some(color) = self.style.text_color {
                text = text.color(color);
            }
            parts.push(text.into_element());
        }

        if parts.len() == 1 {
            parts.pop().unwrap_or_else(|| Text::new("").into_element())
        } else {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .children(parts)
                .into_element()
        }
    }

    /// Render a line with cursor
    fn render_line_with_cursor(&self, line: &str, cursor_col: usize) -> Element {
        let chars: Vec<char> = line.chars().collect();

        // Split line into before cursor, cursor char, and after cursor
        let before: String = chars.iter().take(cursor_col).collect();
        let cursor_char = chars.get(cursor_col).copied().unwrap_or(' ');
        let after: String = chars.iter().skip(cursor_col + 1).collect();

        let mut container = RnkBox::new().flex_direction(FlexDirection::Row);

        // Before cursor
        if !before.is_empty() {
            let mut text = Text::new(&before);
            if let Some(color) = self.style.text_color {
                text = text.color(color);
            }
            container = container.child(text.into_element());
        }

        // Cursor
        let cursor_str = if cursor_char == ' ' {
            self.style.cursor_char.to_string()
        } else {
            cursor_char.to_string()
        };
        let mut cursor_text = Text::new(&cursor_str);
        if let Some(color) = self.style.cursor_color {
            cursor_text = cursor_text.background(color);
        }
        cursor_text = cursor_text.color(Color::Black);
        container = container.child(cursor_text.into_element());

        // After cursor
        if !after.is_empty() {
            let mut text = Text::new(&after);
            if let Some(color) = self.style.text_color {
                text = text.color(color);
            }
            container = container.child(text.into_element());
        }

        container.into_element()
    }
}

/// Handle textarea input and return the action performed
pub fn handle_textarea_input(
    state: &mut TextAreaState,
    input: &str,
    key: &crate::hooks::Key,
    keymap: &TextAreaKeyMap,
) -> Option<TextAreaAction> {
    // First check for mapped actions
    if let Some(action) = keymap.match_action(input, key) {
        apply_textarea_action(state, action);
        return Some(action);
    }

    // If no mapped action, insert the character (if it's a printable char)
    if input.len() == 1 && !key.ctrl && !key.alt {
        if let Some(ch) = input.chars().next() {
            if !ch.is_control() {
                state.insert_char(ch);
                return None; // Character inserted, but no specific action
            }
        }
    }

    None
}

/// Apply a textarea action to the state
pub fn apply_textarea_action(state: &mut TextAreaState, action: TextAreaAction) {
    match action {
        // Movement
        TextAreaAction::MoveLeft => state.move_left(),
        TextAreaAction::MoveRight => state.move_right(),
        TextAreaAction::MoveUp => state.move_up(),
        TextAreaAction::MoveDown => state.move_down(),
        TextAreaAction::MoveWordLeft => state.move_word_left(),
        TextAreaAction::MoveWordRight => state.move_word_right(),
        TextAreaAction::MoveToLineStart => state.move_to_line_start(),
        TextAreaAction::MoveToLineEnd => state.move_to_line_end(),
        TextAreaAction::MoveToStart => state.move_to_start(),
        TextAreaAction::MoveToEnd => state.move_to_end(),

        // Deletion
        TextAreaAction::DeleteBefore => state.delete_before_cursor(),
        TextAreaAction::DeleteAfter => state.delete_after_cursor(),
        TextAreaAction::DeleteWordBefore => state.delete_word_before(),
        TextAreaAction::DeleteWordAfter => state.delete_word_after(),
        TextAreaAction::DeleteLine => state.delete_line(),

        // Selection
        TextAreaAction::SelectAll => state.select_all(),

        // Clipboard (these need external handling)
        TextAreaAction::Copy => {
            // Handled externally - get selected_text() and copy to clipboard
        }
        TextAreaAction::Cut => {
            // Handled externally - get selected_text(), copy, then delete selection
        }
        TextAreaAction::Paste => {
            // Handled externally - get clipboard content and call insert_string()
        }

        // Undo/Redo (not implemented in basic state)
        TextAreaAction::Undo => {
            // Would need undo history in state
        }
        TextAreaAction::Redo => {
            // Would need redo history in state
        }

        // Special
        TextAreaAction::InsertNewline => state.insert_char('\n'),
        TextAreaAction::InsertTab => state.insert_char('\t'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textarea_creation() {
        let state = TextAreaState::new();
        let textarea = TextArea::new(&state);
        let element = textarea.into_element();

        // Should have at least one child (empty line or placeholder)
        assert!(!element.children.is_empty() || element.text_content.is_some());
    }

    #[test]
    fn test_textarea_with_content() {
        let mut state = TextAreaState::new();
        state.set_content("Hello\nWorld");

        let textarea = TextArea::new(&state).height(5);
        let element = textarea.into_element();

        // Should have children for each visible line
        assert!(element.children.len() >= 2);
    }

    #[test]
    fn test_textarea_with_line_numbers() {
        let mut state = TextAreaState::new();
        state.set_content("Line 1\nLine 2\nLine 3");

        let textarea = TextArea::new(&state).line_numbers(true);
        let element = textarea.into_element();

        assert!(!element.children.is_empty());
    }
}
