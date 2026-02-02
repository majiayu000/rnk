//! TextArea state management
//!
//! Manages text content, cursor position, selection, and editing operations.

use std::cmp;

/// Position in the text (row, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Selection range in the text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        // Normalize so start <= end
        if start.row < end.row || (start.row == end.row && start.col <= end.col) {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// Check if selection is empty (cursor position)
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// TextArea state containing text content and cursor
#[derive(Debug, Clone)]
pub struct TextAreaState {
    /// Text content as lines
    lines: Vec<String>,

    /// Cursor position
    cursor: Position,

    /// Selection (None = no selection, just cursor)
    selection: Option<Selection>,

    /// Maximum number of lines (None = unlimited)
    max_lines: Option<usize>,

    /// Maximum line length (None = unlimited)
    max_length: Option<usize>,

    /// Whether the textarea is read-only
    read_only: bool,

    /// Viewport scroll offset (for display)
    scroll_offset: usize,

    /// Viewport height
    viewport_height: usize,

    /// Character limit (total characters, None = unlimited)
    char_limit: Option<usize>,

    /// Placeholder text when empty
    placeholder: String,

    /// Whether to show line numbers
    show_line_numbers: bool,

    /// Tab width in spaces
    tab_width: usize,

    /// Whether to use soft tabs (spaces instead of \t)
    soft_tabs: bool,
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: Position::default(),
            selection: None,
            max_lines: None,
            max_length: None,
            read_only: false,
            scroll_offset: 0,
            viewport_height: 10,
            char_limit: None,
            placeholder: String::new(),
            show_line_numbers: false,
            tab_width: 4,
            soft_tabs: true,
        }
    }
}

impl TextAreaState {
    /// Create a new empty textarea state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with initial content
    pub fn with_content(content: &str) -> Self {
        let mut state = Self::new();
        state.set_content(content);
        state
    }

    /// Create with dimensions
    pub fn with_size(viewport_height: usize) -> Self {
        Self {
            viewport_height,
            ..Default::default()
        }
    }

    // ========== Content Management ==========

    /// Set the entire content
    pub fn set_content(&mut self, content: &str) {
        let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
        self.lines = normalized.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.clamp_cursor();
        self.selection = None;
    }

    /// Get the entire content as a string
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get content lines
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get a specific line
    pub fn line(&self, row: usize) -> Option<&str> {
        self.lines.get(row).map(|s| s.as_str())
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get total character count
    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.chars().count()).sum::<usize>()
            + self.lines.len().saturating_sub(1) // newlines
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor = Position::default();
        self.selection = None;
        self.scroll_offset = 0;
    }

    // ========== Cursor Management ==========

    /// Get cursor position
    pub fn cursor(&self) -> Position {
        self.cursor
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, pos: Position) {
        self.cursor = pos;
        self.clamp_cursor();
        self.ensure_cursor_visible();
    }

    /// Get cursor row
    pub fn cursor_row(&self) -> usize {
        self.cursor.row
    }

    /// Get cursor column
    pub fn cursor_col(&self) -> usize {
        self.cursor.col
    }

    // ========== Cursor Movement ==========

    /// Move cursor left by one character
    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.row > 0 {
            // Move to end of previous line
            self.cursor.row -= 1;
            self.cursor.col = self.current_line_len();
        }
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor right by one character
    pub fn move_right(&mut self) {
        let line_len = self.current_line_len();
        if self.cursor.col < line_len {
            self.cursor.col += 1;
        } else if self.cursor.row < self.lines.len() - 1 {
            // Move to start of next line
            self.cursor.row += 1;
            self.cursor.col = 0;
        }
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor up by one line
    pub fn move_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.clamp_cursor_col();
        }
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor down by one line
    pub fn move_down(&mut self) {
        if self.cursor.row < self.lines.len() - 1 {
            self.cursor.row += 1;
            self.clamp_cursor_col();
        }
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor to start of line
    pub fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
        self.clear_selection();
    }

    /// Move cursor to end of line
    pub fn move_to_line_end(&mut self) {
        self.cursor.col = self.current_line_len();
        self.clear_selection();
    }

    /// Move cursor to start of text
    pub fn move_to_start(&mut self) {
        self.cursor = Position::default();
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor to end of text
    pub fn move_to_end(&mut self) {
        self.cursor.row = self.lines.len() - 1;
        self.cursor.col = self.current_line_len();
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor to previous word
    pub fn move_word_left(&mut self) {
        if self.cursor.col == 0 {
            if self.cursor.row > 0 {
                self.cursor.row -= 1;
                self.cursor.col = self.current_line_len();
            }
        } else {
            let line = &self.lines[self.cursor.row];
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.cursor.col;

            // Skip whitespace
            while col > 0 && chars.get(col - 1).is_some_and(|c| c.is_whitespace()) {
                col -= 1;
            }
            // Skip word characters
            while col > 0 && chars.get(col - 1).is_some_and(|c| !c.is_whitespace()) {
                col -= 1;
            }

            self.cursor.col = col;
        }
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    /// Move cursor to next word
    pub fn move_word_right(&mut self) {
        let line_len = self.current_line_len();
        if self.cursor.col >= line_len {
            if self.cursor.row < self.lines.len() - 1 {
                self.cursor.row += 1;
                self.cursor.col = 0;
            }
        } else {
            let line = &self.lines[self.cursor.row];
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.cursor.col;

            // Skip word characters
            while col < chars.len() && !chars[col].is_whitespace() {
                col += 1;
            }
            // Skip whitespace
            while col < chars.len() && chars[col].is_whitespace() {
                col += 1;
            }

            self.cursor.col = col;
        }
        self.clear_selection();
        self.ensure_cursor_visible();
    }

    // ========== Text Editing ==========

    /// Insert a character at cursor position
    pub fn insert_char(&mut self, ch: char) {
        if self.read_only {
            return;
        }

        // Check char limit
        if let Some(limit) = self.char_limit {
            if self.char_count() >= limit {
                return;
            }
        }

        // Delete selection first if any
        self.delete_selection();

        if ch == '\n' {
            self.insert_newline();
        } else if ch == '\t' {
            self.insert_tab();
        } else {
            // Check line length limit
            if let Some(max_len) = self.max_length {
                if self.current_line_len() >= max_len {
                    return;
                }
            }

            let line = &mut self.lines[self.cursor.row];
            let byte_pos = char_to_byte_pos(line, self.cursor.col);
            line.insert(byte_pos, ch);
            self.cursor.col += 1;
        }

        self.ensure_cursor_visible();
    }

    /// Insert a string at cursor position
    pub fn insert_string(&mut self, s: &str) {
        if self.read_only {
            return;
        }

        self.delete_selection();

        for ch in s.chars() {
            // Check limits
            if let Some(limit) = self.char_limit {
                if self.char_count() >= limit {
                    break;
                }
            }

            if ch == '\n' {
                if let Some(max_lines) = self.max_lines {
                    if self.lines.len() >= max_lines {
                        continue;
                    }
                }
                self.insert_newline();
            } else if ch == '\t' {
                self.insert_tab();
            } else {
                if let Some(max_len) = self.max_length {
                    if self.current_line_len() >= max_len {
                        continue;
                    }
                }

                let line = &mut self.lines[self.cursor.row];
                let byte_pos = char_to_byte_pos(line, self.cursor.col);
                line.insert(byte_pos, ch);
                self.cursor.col += 1;
            }
        }

        self.ensure_cursor_visible();
    }

    /// Insert a newline at cursor position
    fn insert_newline(&mut self) {
        if let Some(max_lines) = self.max_lines {
            if self.lines.len() >= max_lines {
                return;
            }
        }

        let line = &self.lines[self.cursor.row];
        let byte_pos = char_to_byte_pos(line, self.cursor.col);
        let rest = line[byte_pos..].to_string();
        self.lines[self.cursor.row].truncate(byte_pos);
        self.cursor.row += 1;
        self.cursor.col = 0;
        self.lines.insert(self.cursor.row, rest);
    }

    /// Insert a tab (or spaces if soft tabs)
    fn insert_tab(&mut self) {
        if self.soft_tabs {
            let spaces_needed = self.tab_width - (self.cursor.col % self.tab_width);
            for _ in 0..spaces_needed {
                self.insert_char(' ');
            }
        } else {
            let line = &mut self.lines[self.cursor.row];
            let byte_pos = char_to_byte_pos(line, self.cursor.col);
            line.insert(byte_pos, '\t');
            self.cursor.col += 1;
        }
    }

    /// Delete character before cursor (backspace)
    pub fn delete_before_cursor(&mut self) {
        if self.read_only {
            return;
        }

        if self.delete_selection() {
            return;
        }

        if self.cursor.col > 0 {
            let line = &mut self.lines[self.cursor.row];
            let byte_pos = char_to_byte_pos(line, self.cursor.col - 1);
            let end_pos = char_to_byte_pos(line, self.cursor.col);
            line.replace_range(byte_pos..end_pos, "");
            self.cursor.col -= 1;
        } else if self.cursor.row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].chars().count();
            self.lines[self.cursor.row].push_str(&current_line);
        }

        self.ensure_cursor_visible();
    }

    /// Delete character after cursor (delete key)
    pub fn delete_after_cursor(&mut self) {
        if self.read_only {
            return;
        }

        if self.delete_selection() {
            return;
        }

        let line_len = self.current_line_len();
        if self.cursor.col < line_len {
            let line = &mut self.lines[self.cursor.row];
            let byte_pos = char_to_byte_pos(line, self.cursor.col);
            let end_pos = char_to_byte_pos(line, self.cursor.col + 1);
            line.replace_range(byte_pos..end_pos, "");
        } else if self.cursor.row < self.lines.len() - 1 {
            // Merge with next line
            let next_line = self.lines.remove(self.cursor.row + 1);
            self.lines[self.cursor.row].push_str(&next_line);
        }
    }

    /// Delete word before cursor
    pub fn delete_word_before(&mut self) {
        if self.read_only {
            return;
        }

        if self.delete_selection() {
            return;
        }

        let start_col = self.cursor.col;
        self.move_word_left();
        let end_col = self.cursor.col;

        if start_col > end_col {
            let line = &mut self.lines[self.cursor.row];
            let start_byte = char_to_byte_pos(line, end_col);
            let end_byte = char_to_byte_pos(line, start_col);
            line.replace_range(start_byte..end_byte, "");
        }
    }

    /// Delete word after cursor
    pub fn delete_word_after(&mut self) {
        if self.read_only {
            return;
        }

        if self.delete_selection() {
            return;
        }

        let start_col = self.cursor.col;
        let line = &self.lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let mut end_col = start_col;

        // Skip word characters
        while end_col < chars.len() && !chars[end_col].is_whitespace() {
            end_col += 1;
        }
        // Skip whitespace
        while end_col < chars.len() && chars[end_col].is_whitespace() {
            end_col += 1;
        }

        if end_col > start_col {
            let line = &mut self.lines[self.cursor.row];
            let start_byte = char_to_byte_pos(line, start_col);
            let end_byte = char_to_byte_pos(line, end_col);
            line.replace_range(start_byte..end_byte, "");
        }
    }

    /// Delete entire line
    pub fn delete_line(&mut self) {
        if self.read_only {
            return;
        }

        if self.lines.len() > 1 {
            self.lines.remove(self.cursor.row);
            if self.cursor.row >= self.lines.len() {
                self.cursor.row = self.lines.len() - 1;
            }
            self.clamp_cursor_col();
        } else {
            self.lines[0].clear();
            self.cursor.col = 0;
        }

        self.selection = None;
        self.ensure_cursor_visible();
    }

    // ========== Selection ==========

    /// Get current selection
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// Check if there's an active selection
    pub fn has_selection(&self) -> bool {
        self.selection.is_some_and(|s| !s.is_empty())
    }

    /// Start or extend selection
    pub fn select_to(&mut self, pos: Position) {
        let start = self.selection.map_or(self.cursor, |s| s.start);
        self.selection = Some(Selection::new(start, pos));
        self.cursor = pos;
        self.clamp_cursor();
    }

    /// Select all text
    pub fn select_all(&mut self) {
        let end = Position::new(
            self.lines.len() - 1,
            self.lines.last().map_or(0, |l| l.chars().count()),
        );
        self.selection = Some(Selection::new(Position::default(), end));
        self.cursor = end;
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<String> {
        let sel = self.selection?;
        if sel.is_empty() {
            return None;
        }

        let mut result = String::new();

        for row in sel.start.row..=sel.end.row {
            if row >= self.lines.len() {
                break;
            }

            let line = &self.lines[row];
            let start_col = if row == sel.start.row {
                sel.start.col
            } else {
                0
            };
            let end_col = if row == sel.end.row {
                sel.end.col
            } else {
                line.chars().count()
            };

            let start_byte = char_to_byte_pos(line, start_col);
            let end_byte = char_to_byte_pos(line, end_col);

            if start_byte < line.len() {
                result.push_str(&line[start_byte..end_byte.min(line.len())]);
            }

            if row < sel.end.row {
                result.push('\n');
            }
        }

        Some(result)
    }

    /// Delete selected text (returns true if something was deleted)
    fn delete_selection(&mut self) -> bool {
        let sel = match self.selection {
            Some(s) if !s.is_empty() => s,
            _ => return false,
        };

        // Get text after selection on the end line
        let end_line = &self.lines[sel.end.row];
        let end_byte = char_to_byte_pos(end_line, sel.end.col);
        let after_selection = end_line[end_byte..].to_string();

        // Truncate start line at selection start
        let start_byte = char_to_byte_pos(&self.lines[sel.start.row], sel.start.col);
        self.lines[sel.start.row].truncate(start_byte);

        // Append text after selection
        self.lines[sel.start.row].push_str(&after_selection);

        // Remove lines between start and end
        if sel.end.row > sel.start.row {
            self.lines.drain(sel.start.row + 1..=sel.end.row);
        }

        // Update cursor
        self.cursor = sel.start;
        self.selection = None;

        true
    }

    // ========== Configuration ==========

    /// Set maximum number of lines
    pub fn set_max_lines(&mut self, max: Option<usize>) {
        self.max_lines = max;
    }

    /// Set maximum line length
    pub fn set_max_length(&mut self, max: Option<usize>) {
        self.max_length = max;
    }

    /// Set character limit
    pub fn set_char_limit(&mut self, limit: Option<usize>) {
        self.char_limit = limit;
    }

    /// Set read-only mode
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    /// Check if read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Set placeholder text
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
    }

    /// Get placeholder text
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set viewport height
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        self.ensure_cursor_visible();
    }

    /// Get viewport height
    pub fn viewport_height(&self) -> usize {
        self.viewport_height
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set tab width
    pub fn set_tab_width(&mut self, width: usize) {
        self.tab_width = width.max(1);
    }

    /// Set soft tabs (spaces instead of \t)
    pub fn set_soft_tabs(&mut self, soft: bool) {
        self.soft_tabs = soft;
    }

    /// Enable/disable line numbers
    pub fn set_show_line_numbers(&mut self, show: bool) {
        self.show_line_numbers = show;
    }

    /// Check if line numbers are shown
    pub fn show_line_numbers(&self) -> bool {
        self.show_line_numbers
    }

    // ========== Viewport ==========

    /// Get visible lines for rendering
    pub fn visible_lines(&self) -> impl Iterator<Item = (usize, &str)> {
        self.lines
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.viewport_height)
            .map(|(i, s)| (i, s.as_str()))
    }

    /// Ensure cursor is visible in viewport
    fn ensure_cursor_visible(&mut self) {
        if self.cursor.row < self.scroll_offset {
            self.scroll_offset = self.cursor.row;
        } else if self.cursor.row >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor.row - self.viewport_height + 1;
        }
    }

    // ========== Internal Helpers ==========

    /// Get current line length in characters
    fn current_line_len(&self) -> usize {
        self.lines
            .get(self.cursor.row)
            .map_or(0, |l| l.chars().count())
    }

    /// Clamp cursor to valid position
    fn clamp_cursor(&mut self) {
        self.cursor.row = cmp::min(self.cursor.row, self.lines.len().saturating_sub(1));
        self.clamp_cursor_col();
    }

    /// Clamp cursor column to current line length
    fn clamp_cursor_col(&mut self) {
        self.cursor.col = cmp::min(self.cursor.col, self.current_line_len());
    }
}

/// Convert character position to byte position in a string
fn char_to_byte_pos(s: &str, char_pos: usize) -> usize {
    s.char_indices().nth(char_pos).map_or(s.len(), |(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = TextAreaState::new();
        assert_eq!(state.line_count(), 1);
        assert!(state.is_empty());
        assert_eq!(state.cursor(), Position::default());
    }

    #[test]
    fn test_set_content() {
        let mut state = TextAreaState::new();
        state.set_content("line1\nline2\nline3");

        assert_eq!(state.line_count(), 3);
        assert_eq!(state.line(0), Some("line1"));
        assert_eq!(state.line(1), Some("line2"));
        assert_eq!(state.line(2), Some("line3"));
    }

    #[test]
    fn test_insert_char() {
        let mut state = TextAreaState::new();
        state.insert_char('H');
        state.insert_char('i');

        assert_eq!(state.content(), "Hi");
        assert_eq!(state.cursor(), Position::new(0, 2));
    }

    #[test]
    fn test_insert_newline() {
        let mut state = TextAreaState::new();
        state.insert_string("Hello");
        state.insert_char('\n');
        state.insert_string("World");

        assert_eq!(state.line_count(), 2);
        assert_eq!(state.content(), "Hello\nWorld");
    }

    #[test]
    fn test_delete_before_cursor() {
        let mut state = TextAreaState::new();
        state.insert_string("Hello");
        state.delete_before_cursor();

        assert_eq!(state.content(), "Hell");
    }

    #[test]
    fn test_delete_merge_lines() {
        let mut state = TextAreaState::new();
        state.set_content("Hello\nWorld");
        state.set_cursor(Position::new(1, 0));
        state.delete_before_cursor();

        assert_eq!(state.content(), "HelloWorld");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = TextAreaState::new();
        state.set_content("Hello\nWorld");

        state.move_right();
        assert_eq!(state.cursor(), Position::new(0, 1));

        state.move_to_line_end();
        assert_eq!(state.cursor(), Position::new(0, 5));

        state.move_down();
        assert_eq!(state.cursor(), Position::new(1, 5));

        state.move_to_start();
        assert_eq!(state.cursor(), Position::default());
    }

    #[test]
    fn test_selection() {
        let mut state = TextAreaState::new();
        state.set_content("Hello World");

        state.select_to(Position::new(0, 5));
        assert!(state.has_selection());
        assert_eq!(state.selected_text(), Some("Hello".to_string()));
    }

    #[test]
    fn test_select_all() {
        let mut state = TextAreaState::new();
        state.set_content("Hello\nWorld");

        state.select_all();
        assert_eq!(state.selected_text(), Some("Hello\nWorld".to_string()));
    }

    #[test]
    fn test_delete_selection() {
        let mut state = TextAreaState::new();
        state.set_content("Hello World");
        state.select_to(Position::new(0, 6));
        state.insert_char('X');

        assert_eq!(state.content(), "XWorld");
    }

    #[test]
    fn test_max_lines() {
        let mut state = TextAreaState::new();
        state.set_max_lines(Some(2));
        state.insert_string("Line1\nLine2\nLine3");

        assert_eq!(state.line_count(), 2);
    }

    #[test]
    fn test_word_navigation() {
        let mut state = TextAreaState::new();
        state.set_content("Hello World Test");

        state.move_word_right();
        assert_eq!(state.cursor().col, 6); // After "Hello "

        state.move_word_right();
        assert_eq!(state.cursor().col, 12); // After "World "

        state.move_word_left();
        assert_eq!(state.cursor().col, 6);
    }
}
