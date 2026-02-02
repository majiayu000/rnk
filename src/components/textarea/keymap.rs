//! Keyboard mapping configuration for textarea
//!
//! Provides customizable key bindings for text editing operations.

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

/// A single key binding
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    /// The key character or special key
    pub key: KeyType,
    /// Required modifier keys
    pub modifiers: Modifiers,
}

/// Type of key
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyType {
    /// Regular character
    Char(char),
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Home
    Home,
    /// End
    End,
    /// Page Up
    PageUp,
    /// Page Down
    PageDown,
    /// Backspace
    Backspace,
    /// Delete
    Delete,
    /// Enter
    Enter,
    /// Tab
    Tab,
    /// Escape
    Escape,
    /// Space
    Space,
}

/// Modifier key flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
    };

    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
    };

    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
    };

    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
    };

    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        alt: false,
        shift: true,
    };
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(key: KeyType, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a key binding for a character without modifiers
    pub fn char(c: char) -> Self {
        Self::new(KeyType::Char(c), Modifiers::NONE)
    }

    /// Create a key binding with Ctrl modifier
    pub fn ctrl(c: char) -> Self {
        Self::new(KeyType::Char(c), Modifiers::CTRL)
    }

    /// Create a key binding with Alt modifier
    pub fn alt(c: char) -> Self {
        Self::new(KeyType::Char(c), Modifiers::ALT)
    }

    /// Create a key binding for a special key
    pub fn special(key: KeyType) -> Self {
        Self::new(key, Modifiers::NONE)
    }

    /// Create a key binding for a special key with Ctrl
    pub fn ctrl_special(key: KeyType) -> Self {
        Self::new(key, Modifiers::CTRL)
    }

    /// Check if this binding matches the given input
    pub fn matches(&self, input: &str, key: &crate::hooks::Key) -> bool {
        // Check modifiers
        if self.modifiers.ctrl != key.ctrl
            || self.modifiers.alt != key.alt
            || self.modifiers.shift != key.shift
        {
            return false;
        }

        // Check key type
        match &self.key {
            KeyType::Char(c) => {
                if input.len() == 1 {
                    input.starts_with(*c)
                } else {
                    false
                }
            }
            KeyType::Up => key.up_arrow,
            KeyType::Down => key.down_arrow,
            KeyType::Left => key.left_arrow,
            KeyType::Right => key.right_arrow,
            KeyType::Home => key.home,
            KeyType::End => key.end,
            KeyType::PageUp => key.page_up,
            KeyType::PageDown => key.page_down,
            KeyType::Backspace => key.backspace,
            KeyType::Delete => key.delete,
            KeyType::Enter => key.return_key,
            KeyType::Tab => key.tab,
            KeyType::Escape => key.escape,
            KeyType::Space => key.space,
        }
    }
}

impl Default for TextAreaKeyMap {
    fn default() -> Self {
        Self {
            // Cursor movement
            left: vec![KeyBinding::special(KeyType::Left)],
            right: vec![KeyBinding::special(KeyType::Right)],
            up: vec![KeyBinding::special(KeyType::Up)],
            down: vec![KeyBinding::special(KeyType::Down)],

            // Word navigation
            word_left: vec![
                KeyBinding::ctrl_special(KeyType::Left),
                KeyBinding::alt('b'),
            ],
            word_right: vec![
                KeyBinding::ctrl_special(KeyType::Right),
                KeyBinding::alt('f'),
            ],

            // Line navigation
            line_start: vec![KeyBinding::special(KeyType::Home), KeyBinding::ctrl('a')],
            line_end: vec![KeyBinding::special(KeyType::End), KeyBinding::ctrl('e')],

            // Document navigation
            doc_start: vec![KeyBinding::ctrl_special(KeyType::Home)],
            doc_end: vec![KeyBinding::ctrl_special(KeyType::End)],

            // Deletion
            delete_before: vec![KeyBinding::special(KeyType::Backspace)],
            delete_after: vec![KeyBinding::special(KeyType::Delete)],
            delete_word_before: vec![
                KeyBinding::ctrl_special(KeyType::Backspace),
                KeyBinding::ctrl('w'),
            ],
            delete_word_after: vec![KeyBinding::ctrl_special(KeyType::Delete)],
            delete_line: vec![KeyBinding::ctrl('k')],

            // Selection
            select_all: vec![KeyBinding::ctrl('a')],

            // Clipboard
            copy: vec![KeyBinding::ctrl('c')],
            cut: vec![KeyBinding::ctrl('x')],
            paste: vec![KeyBinding::ctrl('v')],

            // Undo/Redo
            undo: vec![KeyBinding::ctrl('z')],
            redo: vec![
                KeyBinding::ctrl('y'),
                KeyBinding::new(KeyType::Char('z'), Modifiers::CTRL_SHIFT),
            ],

            // Special
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
        // Check each action's bindings (order matters for priority)

        // Movement
        for binding in &self.left {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveLeft);
            }
        }
        for binding in &self.right {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveRight);
            }
        }
        for binding in &self.up {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveUp);
            }
        }
        for binding in &self.down {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveDown);
            }
        }
        for binding in &self.word_left {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveWordLeft);
            }
        }
        for binding in &self.word_right {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveWordRight);
            }
        }
        for binding in &self.line_start {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveToLineStart);
            }
        }
        for binding in &self.line_end {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveToLineEnd);
            }
        }
        for binding in &self.doc_start {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveToStart);
            }
        }
        for binding in &self.doc_end {
            if binding.matches(input, key) {
                return Some(TextAreaAction::MoveToEnd);
            }
        }

        // Deletion
        for binding in &self.delete_before {
            if binding.matches(input, key) {
                return Some(TextAreaAction::DeleteBefore);
            }
        }
        for binding in &self.delete_after {
            if binding.matches(input, key) {
                return Some(TextAreaAction::DeleteAfter);
            }
        }
        for binding in &self.delete_word_before {
            if binding.matches(input, key) {
                return Some(TextAreaAction::DeleteWordBefore);
            }
        }
        for binding in &self.delete_word_after {
            if binding.matches(input, key) {
                return Some(TextAreaAction::DeleteWordAfter);
            }
        }
        for binding in &self.delete_line {
            if binding.matches(input, key) {
                return Some(TextAreaAction::DeleteLine);
            }
        }

        // Selection
        for binding in &self.select_all {
            if binding.matches(input, key) {
                return Some(TextAreaAction::SelectAll);
            }
        }

        // Clipboard
        for binding in &self.copy {
            if binding.matches(input, key) {
                return Some(TextAreaAction::Copy);
            }
        }
        for binding in &self.cut {
            if binding.matches(input, key) {
                return Some(TextAreaAction::Cut);
            }
        }
        for binding in &self.paste {
            if binding.matches(input, key) {
                return Some(TextAreaAction::Paste);
            }
        }

        // Undo/Redo
        for binding in &self.undo {
            if binding.matches(input, key) {
                return Some(TextAreaAction::Undo);
            }
        }
        for binding in &self.redo {
            if binding.matches(input, key) {
                return Some(TextAreaAction::Redo);
            }
        }

        // Special
        for binding in &self.newline {
            if binding.matches(input, key) {
                return Some(TextAreaAction::InsertNewline);
            }
        }
        for binding in &self.tab {
            if binding.matches(input, key) {
                return Some(TextAreaAction::InsertTab);
            }
        }

        None
    }
}

/// Actions that can be performed on a textarea
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaAction {
    // Movement
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

    // Deletion
    DeleteBefore,
    DeleteAfter,
    DeleteWordBefore,
    DeleteWordAfter,
    DeleteLine,

    // Selection
    SelectAll,

    // Clipboard
    Copy,
    Cut,
    Paste,

    // Undo/Redo
    Undo,
    Redo,

    // Special
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
