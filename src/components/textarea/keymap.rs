//! Keyboard mapping configuration for textarea
//!
//! Provides customizable key bindings for text editing operations.

pub use crate::components::keymap::{KeyBinding, KeyType, Modifiers};

/// Key binding configuration for textarea
#[derive(Debug, Clone)]
pub struct TextAreaKeyMap {
    // Cursor movement
    /// Move cursor left (default: left arrow)
    pub left: Vec<KeyBinding>,
    /// Move cursor right (default: right arrow)
    pub right: Vec<KeyBinding>,
    /// Move cursor up (default: up arrow)
    pub up: Vec<KeyBinding>,
    /// Move cursor down (default: down arrow)
    pub down: Vec<KeyBinding>,

    // Word navigation
    /// Move to previous word (default: Ctrl+Left, Alt+B)
    pub word_left: Vec<KeyBinding>,
    /// Move to next word (default: Ctrl+Right, Alt+F)
    pub word_right: Vec<KeyBinding>,

    // Line navigation
    /// Move to start of line (default: Home, Ctrl+A)
    pub line_start: Vec<KeyBinding>,
    /// Move to end of line (default: End, Ctrl+E)
    pub line_end: Vec<KeyBinding>,

    // Document navigation
    /// Move to start of document (default: Ctrl+Home)
    pub doc_start: Vec<KeyBinding>,
    /// Move to end of document (default: Ctrl+End)
    pub doc_end: Vec<KeyBinding>,

    // Deletion
    /// Delete character before cursor (default: Backspace)
    pub delete_before: Vec<KeyBinding>,
    /// Delete character after cursor (default: Delete)
    pub delete_after: Vec<KeyBinding>,
    /// Delete word before cursor (default: Ctrl+Backspace, Ctrl+W)
    pub delete_word_before: Vec<KeyBinding>,
    /// Delete word after cursor (default: Ctrl+Delete)
    pub delete_word_after: Vec<KeyBinding>,
    /// Delete entire line (default: Ctrl+K)
    pub delete_line: Vec<KeyBinding>,

    // Selection
    /// Select all (default: Ctrl+A)
    pub select_all: Vec<KeyBinding>,

    // Clipboard (handled externally, but we define the keys)
    /// Copy selection (default: Ctrl+C)
    pub copy: Vec<KeyBinding>,
    /// Cut selection (default: Ctrl+X)
    pub cut: Vec<KeyBinding>,
    /// Paste (default: Ctrl+V)
    pub paste: Vec<KeyBinding>,

    // Undo/Redo (if implemented)
    /// Undo (default: Ctrl+Z)
    pub undo: Vec<KeyBinding>,
    /// Redo (default: Ctrl+Y, Ctrl+Shift+Z)
    pub redo: Vec<KeyBinding>,

    // Special
    /// Insert newline (default: Enter)
    pub newline: Vec<KeyBinding>,
    /// Insert tab (default: Tab)
    pub tab: Vec<KeyBinding>,
}

impl Default for TextAreaKeyMap {
    fn default() -> Self {
        Self {
            left: vec![KeyBinding::special(KeyType::Left)],
            right: vec![KeyBinding::special(KeyType::Right)],
            up: vec![KeyBinding::special(KeyType::Up)],
            down: vec![KeyBinding::special(KeyType::Down)],
            word_left: vec![
                KeyBinding::ctrl_special(KeyType::Left),
                KeyBinding::alt('b'),
            ],
            word_right: vec![
                KeyBinding::ctrl_special(KeyType::Right),
                KeyBinding::alt('f'),
            ],
            line_start: vec![KeyBinding::special(KeyType::Home), KeyBinding::ctrl('a')],
            line_end: vec![KeyBinding::special(KeyType::End), KeyBinding::ctrl('e')],
            doc_start: vec![KeyBinding::ctrl_special(KeyType::Home)],
            doc_end: vec![KeyBinding::ctrl_special(KeyType::End)],
            delete_before: vec![KeyBinding::special(KeyType::Backspace)],
            delete_after: vec![KeyBinding::special(KeyType::Delete)],
            delete_word_before: vec![
                KeyBinding::ctrl_special(KeyType::Backspace),
                KeyBinding::ctrl('w'),
            ],
            delete_word_after: vec![KeyBinding::ctrl_special(KeyType::Delete)],
            delete_line: vec![KeyBinding::ctrl('k')],
            select_all: vec![KeyBinding::ctrl('a')],
            copy: vec![KeyBinding::ctrl('c')],
            cut: vec![KeyBinding::ctrl('x')],
            paste: vec![KeyBinding::ctrl('v')],
            undo: vec![KeyBinding::ctrl('z')],
            redo: vec![
                KeyBinding::ctrl('y'),
                KeyBinding::new(KeyType::Char('z'), Modifiers::CTRL_SHIFT),
            ],
            newline: vec![KeyBinding::special(KeyType::Enter)],
            tab: vec![KeyBinding::special(KeyType::Tab)],
        }
    }
}

impl TextAreaKeyMap {
    /// Create a new keymap with default bindings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a minimal keymap (basic editing only)
    pub fn minimal() -> Self {
        Self {
            left: vec![KeyBinding::special(KeyType::Left)],
            right: vec![KeyBinding::special(KeyType::Right)],
            up: vec![KeyBinding::special(KeyType::Up)],
            down: vec![KeyBinding::special(KeyType::Down)],
            word_left: vec![],
            word_right: vec![],
            line_start: vec![KeyBinding::special(KeyType::Home)],
            line_end: vec![KeyBinding::special(KeyType::End)],
            doc_start: vec![],
            doc_end: vec![],
            delete_before: vec![KeyBinding::special(KeyType::Backspace)],
            delete_after: vec![KeyBinding::special(KeyType::Delete)],
            delete_word_before: vec![],
            delete_word_after: vec![],
            delete_line: vec![],
            select_all: vec![],
            copy: vec![],
            cut: vec![],
            paste: vec![],
            undo: vec![],
            redo: vec![],
            newline: vec![KeyBinding::special(KeyType::Enter)],
            tab: vec![KeyBinding::special(KeyType::Tab)],
        }
    }

    /// Disable all key bindings
    pub fn disabled() -> Self {
        Self {
            left: vec![],
            right: vec![],
            up: vec![],
            down: vec![],
            word_left: vec![],
            word_right: vec![],
            line_start: vec![],
            line_end: vec![],
            doc_start: vec![],
            doc_end: vec![],
            delete_before: vec![],
            delete_after: vec![],
            delete_word_before: vec![],
            delete_word_after: vec![],
            delete_line: vec![],
            select_all: vec![],
            copy: vec![],
            cut: vec![],
            paste: vec![],
            undo: vec![],
            redo: vec![],
            newline: vec![],
            tab: vec![],
        }
    }

    /// Check which action matches the input, if any
    pub fn match_action(&self, input: &str, key: &crate::hooks::Key) -> Option<TextAreaAction> {
        let checks: &[(&[KeyBinding], TextAreaAction)] = &[
            (&self.left, TextAreaAction::MoveLeft),
            (&self.right, TextAreaAction::MoveRight),
            (&self.up, TextAreaAction::MoveUp),
            (&self.down, TextAreaAction::MoveDown),
            (&self.word_left, TextAreaAction::MoveWordLeft),
            (&self.word_right, TextAreaAction::MoveWordRight),
            (&self.line_start, TextAreaAction::MoveToLineStart),
            (&self.line_end, TextAreaAction::MoveToLineEnd),
            (&self.doc_start, TextAreaAction::MoveToStart),
            (&self.doc_end, TextAreaAction::MoveToEnd),
            (&self.delete_before, TextAreaAction::DeleteBefore),
            (&self.delete_after, TextAreaAction::DeleteAfter),
            (&self.delete_word_before, TextAreaAction::DeleteWordBefore),
            (&self.delete_word_after, TextAreaAction::DeleteWordAfter),
            (&self.delete_line, TextAreaAction::DeleteLine),
            (&self.select_all, TextAreaAction::SelectAll),
            (&self.copy, TextAreaAction::Copy),
            (&self.cut, TextAreaAction::Cut),
            (&self.paste, TextAreaAction::Paste),
            (&self.undo, TextAreaAction::Undo),
            (&self.redo, TextAreaAction::Redo),
            (&self.newline, TextAreaAction::InsertNewline),
            (&self.tab, TextAreaAction::InsertTab),
        ];
        for (bindings, action) in checks {
            for binding in *bindings {
                if binding.matches(input, key) {
                    return Some(*action);
                }
            }
        }
        None
    }
}

/// Actions that can be performed on a textarea
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaAction {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordLeft,
    MoveWordRight,
    MoveToLineStart,
    MoveToLineEnd,
    MoveToStart,
    MoveToEnd,
    DeleteBefore,
    DeleteAfter,
    DeleteWordBefore,
    DeleteWordAfter,
    DeleteLine,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    InsertNewline,
    InsertTab,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keymap() {
        let keymap = TextAreaKeyMap::default();
        assert!(!keymap.left.is_empty());
        assert!(!keymap.delete_before.is_empty());
    }

    #[test]
    fn test_minimal_keymap() {
        let keymap = TextAreaKeyMap::minimal();
        assert!(!keymap.left.is_empty());
        assert!(keymap.word_left.is_empty());
    }

    #[test]
    fn test_disabled_keymap() {
        let keymap = TextAreaKeyMap::disabled();
        assert!(keymap.left.is_empty());
        assert!(keymap.delete_before.is_empty());
    }
}
