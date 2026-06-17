//! TextInput component - Single-line text input with cursor

use crate::components::{Box, InteractionMode, InteractionOutcome, Text};
use crate::core::{AccessibilityProps, AccessibilityRole, Color, Element, FlexDirection};
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
#[derive(Clone, Default)]
pub struct TextInputState {
    /// Current text value
    value: String,
    /// Cursor position (character index)
    cursor: usize,
}

impl TextInputState {
    /// Get the current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the current cursor position as a character index.
    pub fn cursor(&self) -> usize {
        self.cursor
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
#[derive(Clone)]
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
    /// Input mode for disabled/read-only behavior.
    pub mode: InteractionMode,
}

impl Default for TextInputOptions {
    fn default() -> Self {
        Self {
            placeholder: None,
            mask: false,
            mask_char: '*',
            max_length: 0,
            focus: UseFocusOptions::default(),
            color: None,
            placeholder_color: None,
            cursor_color: None,
            mode: InteractionMode::Enabled,
        }
    }
}

impl TextInputOptions {
    pub fn new() -> Self {
        Self::default()
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

    /// Enable normal editing behavior.
    pub fn enabled(mut self) -> Self {
        self.mode = InteractionMode::Enabled;
        self
    }

    /// Ignore all input.
    pub fn disabled(mut self) -> Self {
        self.mode = InteractionMode::Disabled;
        self
    }

    /// Allow focus/cursor movement, but block value mutation and submit.
    pub fn read_only(mut self) -> Self {
        self.mode = InteractionMode::ReadOnly;
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
        let accessible_label = options
            .placeholder
            .clone()
            .unwrap_or_else(|| "Text input".to_string());
        let mut accessibility = AccessibilityProps::new(AccessibilityRole::TextInput)
            .label(accessible_label)
            .disabled(options.mode.is_disabled())
            .read_only(options.mode.is_read_only())
            .focusable(!options.mode.is_disabled());
        if !state.value.is_empty() {
            let accessible_value = if options.mask {
                options
                    .mask_char
                    .to_string()
                    .repeat(state.value.chars().count())
            } else {
                state.value.clone()
            };
            accessibility = accessibility.value(accessible_value);
        }

        let display_value = if state.value.is_empty() {
            // Show placeholder
            if let Some(ref placeholder) = options.placeholder {
                let mut text = Text::new(placeholder).dim();
                if let Some(color) = options.placeholder_color {
                    text = text.color(color);
                }
                return text.into_element().with_accessibility(accessibility);
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
                .with_accessibility(accessibility)
        } else {
            let mut text = Text::new(&display_value);
            if let Some(color) = options.color {
                text = text.color(color);
            }
            text.into_element().with_accessibility(accessibility)
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
    let input_options = options.clone();

    // Handle input when focused
    use_input({
        let state = state.clone();
        let is_focused = focus.is_focused;

        move |input, key| {
            if !is_focused {
                return;
            }

            let mut next = state.get();
            let outcome = handle_text_input(&mut next, input, key, &input_options);
            if outcome.is_changed() || matches!(outcome, InteractionOutcome::Handled) {
                state.set(next);
            }
        }
    });

    TextInputHandle {
        state,
        focus,
        options,
    }
}

/// Handle a text input key event against explicit state.
///
/// Enabled mode edits state and returns changed/submitted/cancelled outcomes.
/// Read-only mode allows cursor movement but blocks value mutation and submit.
/// Disabled mode ignores every input and leaves state unchanged.
pub fn handle_text_input(
    state: &mut TextInputState,
    input: &str,
    key: &crate::hooks::Key,
    options: &TextInputOptions,
) -> InteractionOutcome<String> {
    if options.mode.is_disabled() {
        return InteractionOutcome::Ignored;
    }

    if key.escape {
        return InteractionOutcome::Cancelled;
    }

    if key.left_arrow {
        state.move_left();
        return InteractionOutcome::Handled;
    }

    if key.right_arrow {
        state.move_right();
        return InteractionOutcome::Handled;
    }

    if key.home || (key.ctrl && input == "a") {
        state.move_to_start();
        return InteractionOutcome::Handled;
    }

    if key.end || (key.ctrl && input == "e") {
        state.move_to_end();
        return InteractionOutcome::Handled;
    }

    if !options.mode.is_enabled() {
        return InteractionOutcome::Ignored;
    }

    if key.return_key {
        return InteractionOutcome::Submitted(state.value.clone());
    }

    if key.backspace {
        state.backspace();
        return InteractionOutcome::Changed(state.value.clone());
    }

    if key.delete {
        state.delete();
        return InteractionOutcome::Changed(state.value.clone());
    }

    if key.ctrl || key.alt || key.tab {
        return InteractionOutcome::Ignored;
    }

    if !input.is_empty() {
        let remaining = if options.max_length == 0 {
            input.chars().count()
        } else {
            options.max_length.saturating_sub(state.char_count())
        };
        if remaining == 0 {
            return InteractionOutcome::Ignored;
        }

        let inserted: String = input.chars().take(remaining).collect();
        if !inserted.is_empty() {
            state.insert_str(&inserted);
            return InteractionOutcome::Changed(state.value.clone());
        }
    }

    InteractionOutcome::Ignored
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

    #[test]
    fn test_handle_text_input_change_submit_and_cancel() {
        let mut state = TextInputState::default();
        let options = TextInputOptions::default();

        let outcome = handle_text_input(&mut state, "a", &crate::hooks::Key::default(), &options);
        assert_eq!(outcome, InteractionOutcome::Changed("a".to_string()));
        assert_eq!(state.value(), "a");

        let outcome = handle_text_input(
            &mut state,
            "",
            &crate::hooks::Key {
                return_key: true,
                ..Default::default()
            },
            &options,
        );
        assert_eq!(outcome, InteractionOutcome::Submitted("a".to_string()));

        let outcome = handle_text_input(
            &mut state,
            "",
            &crate::hooks::Key {
                escape: true,
                ..Default::default()
            },
            &options,
        );
        assert_eq!(outcome, InteractionOutcome::Cancelled);
    }

    #[test]
    fn test_handle_text_input_modes() {
        let mut state = TextInputState::default();
        state.set_value("abc");

        let read_only = TextInputOptions::default().read_only();
        let outcome = handle_text_input(&mut state, "x", &crate::hooks::Key::default(), &read_only);
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert_eq!(state.value(), "abc");

        let outcome = handle_text_input(
            &mut state,
            "",
            &crate::hooks::Key {
                left_arrow: true,
                ..Default::default()
            },
            &read_only,
        );
        assert_eq!(outcome, InteractionOutcome::Handled);
        assert_eq!(state.cursor(), 2);

        let disabled = TextInputOptions::default().disabled();
        let outcome = handle_text_input(
            &mut state,
            "",
            &crate::hooks::Key {
                right_arrow: true,
                ..Default::default()
            },
            &disabled,
        );
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert_eq!(state.cursor(), 2);
    }
}
