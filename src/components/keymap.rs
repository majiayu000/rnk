//! Shared key binding types for component keymaps.
//!
//! Provides `KeyBinding`, `KeyType`, and `Modifiers` used by both
//! `textarea::keymap` and `viewport::keymap`.

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
        if self.modifiers.ctrl != key.ctrl
            || self.modifiers.alt != key.alt
            || self.modifiers.shift != key.shift
        {
            return false;
        }

        match &self.key {
            KeyType::Char(c) => input.len() == 1 && input.starts_with(*c),
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
