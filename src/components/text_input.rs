//! TextInput component - Single-line text input with cursor

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};
use crate::hooks::{FocusState, UseFocusOptions, use_focus, use_input, use_signal};

/// A single-line text input component
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn app() -> Element {
///     let input = use_text_input(TextInputOptions::default());
///
///     Box::new()
///         .child(Text::new("Name: ").into_element())
///         .child(input.view())
///         .into_element()
/// }
/// ```
#[derive(Clone)]
pub struct TextInputState {
    /// Current text value
    value: String,
    /// Cursor position (character index)
    cursor: usize,
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
        }
    }
}

impl TextInputState {
    /// Get the current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the value
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.chars().count();
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// Insert character at cursor
    pub fn insert(&mut self, ch: char) {
        let byte_pos = self.cursor_byte_pos();
        self.value.insert(byte_pos, ch);
        self.cursor += 1;
    }

    /// Insert string at cursor
    pub fn insert_str(&mut self, s: &str) {
        let byte_pos = self.cursor_byte_pos();
        self.value.insert_str(byte_pos, s);
        self.cursor += s.chars().count();
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let byte_pos = self.cursor_byte_pos();
            let prev_char_start = self.prev_char_byte_pos();
            self.value.drain(prev_char_start..byte_pos);
            self.cursor -= 1;
        }
    }

    /// Delete character at cursor (delete)
    pub fn delete(&mut self) {
        let byte_pos = self.cursor_byte_pos();
        if byte_pos < self.value.len() {
            let next_char_end = self.next_char_byte_pos();
            self.value.drain(byte_pos..next_char_end);
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        if self.cursor < self.char_count() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_to_end(&mut self) {
        self.cursor = self.char_count();
    }

    /// Get character count
    fn char_count(&self) -> usize {
        self.value.chars().count()
    }

    /// Get byte position of cursor
    fn cursor_byte_pos(&self) -> usize {
        self.value
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())
    }

    /// Get byte position of previous character
    fn prev_char_byte_pos(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }
        self.value
            .char_indices()
            .nth(self.cursor - 1)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Get byte position after current character
    fn next_char_byte_pos(&self) -> usize {
        let byte_pos = self.cursor_byte_pos();
        if byte_pos >= self.value.len() {
            return self.value.len();
        }
        self.value[byte_pos..]
            .chars()
            .next()
            .map(|c| byte_pos + c.len_utf8())
            .unwrap_or(self.value.len())
    }
}

/// Options for TextInput
#[derive(Clone, Default)]
pub struct TextInputOptions {
    /// Placeholder text when empty
    pub placeholder: Option<String>,
    /// Whether to mask input (for passwords)
    pub mask: bool,
    /// Mask character (default: '*')
    pub mask_char: char,
    /// Maximum length (0 = unlimited)
    pub max_length: usize,
    /// Focus options
    pub focus: UseFocusOptions,
    /// Text color
    pub color: Option<Color>,
    /// Placeholder color
    pub placeholder_color: Option<Color>,
    /// Cursor color
    pub cursor_color: Option<Color>,
}

impl TextInputOptions {
    pub fn new() -> Self {
        Self {
            mask_char: '*',
            ..Default::default()
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn mask(mut self) -> Self {
        self.mask = true;
        self
    }

    pub fn mask_char(mut self, ch: char) -> Self {
        self.mask_char = ch;
        self
    }

    pub fn max_length(mut self, len: usize) -> Self {
        self.max_length = len;
        self
    }

    pub fn auto_focus(mut self) -> Self {
        self.focus = self.focus.auto_focus();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = Some(color);
        self
    }

    pub fn cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = Some(color);
        self
    }
}

/// Handle for controlling the text input
#[derive(Clone)]
pub struct TextInputHandle {
    state: crate::hooks::Signal<TextInputState>,
    focus: FocusState,
    options: TextInputOptions,
}

impl TextInputHandle {
    /// Get the current value
    pub fn value(&self) -> String {
        self.state.get().value
    }

    /// Set the value
    pub fn set_value(&self, value: impl Into<String>) {
        self.state.update(|s| s.set_value(value));
    }

    /// Clear the input
    pub fn clear(&self) {
        self.state.update(|s| s.clear());
    }

    /// Check if focused
    pub fn is_focused(&self) -> bool {
        self.focus.is_focused
    }

    /// Render the text input element
    pub fn view(&self) -> Element {
        let state = self.state.get();
        let options = &self.options;

        let display_value = if state.value.is_empty() {
            // Show placeholder
            if let Some(ref placeholder) = options.placeholder {
                let mut text = Text::new(placeholder).dim();
                if let Some(color) = options.placeholder_color {
                    text = text.color(color);
                }
                return text.into_element();
            }
            String::new()
        } else if options.mask {
            // Mask the input
            options
                .mask_char
                .to_string()
                .repeat(state.value.chars().count())
        } else {
            state.value.clone()
        };

        if self.focus.is_focused {
            // Split at cursor position for rendering
            let chars: Vec<char> = display_value.chars().collect();
            let (before, after) = chars.split_at(state.cursor.min(chars.len()));
            let before: String = before.iter().collect();
            let after: String = after.iter().collect();

            let cursor_char = if after.is_empty() {
                ' '
            } else {
                after.chars().next().unwrap_or(' ')
            };
            let after_cursor: String = after.chars().skip(1).collect();

            let cursor_color = options.cursor_color.unwrap_or(Color::Yellow);

            Box::new()
                .flex_direction(FlexDirection::Row)
                .child({
                    let mut text = Text::new(&before);
                    if let Some(color) = options.color {
                        text = text.color(color);
                    }
                    text.into_element()
                })
                .child(
                    Text::new(cursor_char.to_string())
                        .background(cursor_color)
                        .color(Color::Black)
                        .into_element(),
                )
                .child({
                    let mut text = Text::new(&after_cursor);
                    if let Some(color) = options.color {
                        text = text.color(color);
                    }
                    text.into_element()
                })
                .into_element()
        } else {
            let mut text = Text::new(&display_value);
            if let Some(color) = options.color {
                text = text.color(color);
            }
            text.into_element()
        }
    }
}

/// Hook to create a text input
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn app() -> Element {
///     let input = use_text_input(TextInputOptions::new().placeholder("Type here..."));
///
///     use_input({
///         let input = input.clone();
///         move |ch, key| {
///             if key.return_key {
///                 println!("Submitted: {}", input.value());
///                 input.clear();
///             }
///         }
///     });
///
///     Box::new()
///         .flex_direction(FlexDirection::Row)
///         .child(Text::new("> ").color(Color::Yellow).into_element())
///         .child(input.view())
///         .into_element()
/// }
/// ```
pub fn use_text_input(options: TextInputOptions) -> TextInputHandle {
    let state = use_signal(TextInputState::default);
    let focus = use_focus(options.focus.clone());
    let max_length = options.max_length;

    // Handle input when focused
    use_input({
        let state = state.clone();
        let is_focused = focus.is_focused;

        move |input, key| {
            if !is_focused {
                return;
            }

            // Handle special keys
            if key.backspace {
                state.update(|s| s.backspace());
                return;
            }

            if key.delete {
                state.update(|s| s.delete());
                return;
            }

            if key.left_arrow {
                state.update(|s| s.move_left());
                return;
            }

            if key.right_arrow {
                state.update(|s| s.move_right());
                return;
            }

            if key.home || (key.ctrl && input == "a") {
                state.update(|s| s.move_to_start());
                return;
            }

            if key.end || (key.ctrl && input == "e") {
                state.update(|s| s.move_to_end());
                return;
            }

            // Ignore control sequences
            if key.ctrl || key.alt || key.escape || key.tab || key.return_key {
                return;
            }

            // Insert regular characters
            if !input.is_empty() {
                state.update(|s| {
                    if max_length == 0 || s.value.chars().count() < max_length {
                        s.insert_str(input);
                    }
                });
            }
        }
    });

    TextInputHandle {
        state,
        focus,
        options,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_state_basic() {
        let mut state = TextInputState::default();
        assert_eq!(state.value(), "");
        assert_eq!(state.cursor, 0);

        state.insert('H');
        state.insert('e');
        state.insert('l');
        state.insert('l');
        state.insert('o');
        assert_eq!(state.value(), "Hello");
        assert_eq!(state.cursor, 5);
    }

    #[test]
    fn test_text_input_state_backspace() {
        let mut state = TextInputState::default();
        state.set_value("Hello");
        state.backspace();
        assert_eq!(state.value(), "Hell");
        assert_eq!(state.cursor, 4);
    }

    #[test]
    fn test_text_input_state_cursor_movement() {
        let mut state = TextInputState::default();
        state.set_value("Hello");
        assert_eq!(state.cursor, 5);

        state.move_left();
        assert_eq!(state.cursor, 4);

        state.move_to_start();
        assert_eq!(state.cursor, 0);

        state.move_right();
        assert_eq!(state.cursor, 1);

        state.move_to_end();
        assert_eq!(state.cursor, 5);
    }

    #[test]
    fn test_text_input_state_insert_middle() {
        let mut state = TextInputState::default();
        state.set_value("Hllo");
        state.cursor = 1;
        state.insert('e');
        assert_eq!(state.value(), "Hello");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_text_input_state_delete() {
        let mut state = TextInputState::default();
        state.set_value("Hello");
        state.cursor = 0;
        state.delete();
        assert_eq!(state.value(), "ello");
    }

    #[test]
    fn test_text_input_state_unicode() {
        let mut state = TextInputState::default();
        state.insert('你');
        state.insert('好');
        assert_eq!(state.value(), "你好");
        assert_eq!(state.cursor, 2);

        state.backspace();
        assert_eq!(state.value(), "你");
        assert_eq!(state.cursor, 1);
    }
}
