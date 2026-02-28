//! use_keyboard_shortcut hook for handling keyboard shortcuts
//!
//! Provides a convenient way to register and handle keyboard shortcuts.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     use_keyboard_shortcut(Shortcut::ctrl('s'), || {
//!         println!("Save triggered!");
//!     });
//!
//!     use_keyboard_shortcut(Shortcut::ctrl_shift('z'), || {
//!         println!("Redo triggered!");
//!     });
//!
//!     Text::new("Press Ctrl+S to save").into_element()
//! }
//! ```

use crate::hooks::use_input::{Key, use_input};

/// Modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Control key pressed
    pub ctrl: bool,
    /// Alt key pressed
    pub alt: bool,
    /// Shift key pressed
    pub shift: bool,
}

impl Modifiers {
    /// No modifiers
    pub fn none() -> Self {
        Self::default()
    }

    /// Control modifier
    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            ..Default::default()
        }
    }

    /// Alt modifier
    pub fn alt() -> Self {
        Self {
            alt: true,
            ..Default::default()
        }
    }

    /// Shift modifier
    pub fn shift() -> Self {
        Self {
            shift: true,
            ..Default::default()
        }
    }

    /// Control + Shift modifiers
    pub fn ctrl_shift() -> Self {
        Self {
            ctrl: true,
            shift: true,
            ..Default::default()
        }
    }

    /// Control + Alt modifiers
    pub fn ctrl_alt() -> Self {
        Self {
            ctrl: true,
            alt: true,
            ..Default::default()
        }
    }

    /// Alt + Shift modifiers
    pub fn alt_shift() -> Self {
        Self {
            alt: true,
            shift: true,
            ..Default::default()
        }
    }

    /// All modifiers
    pub fn all() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: true,
        }
    }

    /// Check if modifiers match a Key
    pub fn matches(&self, key: &Key) -> bool {
        self.ctrl == key.ctrl && self.alt == key.alt && self.shift == key.shift
    }
}

/// A keyboard shortcut definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// The key
    pub key: ShortcutKey,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// Key for a shortcut
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutKey {
    /// A character key
    Char(char),
    /// Function key (F1-F12)
    F(u8),
    /// Enter key
    Enter,
    /// Escape key
    Escape,
    /// Tab key
    Tab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Arrow up
    Up,
    /// Arrow down
    Down,
    /// Arrow left
    Left,
    /// Arrow right
    Right,
    /// Home key
    Home,
    /// End key
    End,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Insert key
    Insert,
    /// Space key
    Space,
}

impl Shortcut {
    /// Create a new shortcut
    pub fn new(key: ShortcutKey, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a shortcut with just a character
    pub fn char(c: char) -> Self {
        Self::new(ShortcutKey::Char(c), Modifiers::none())
    }

    /// Create a Ctrl+key shortcut
    pub fn ctrl(c: char) -> Self {
        Self::new(ShortcutKey::Char(c), Modifiers::ctrl())
    }

    /// Create an Alt+key shortcut
    pub fn alt(c: char) -> Self {
        Self::new(ShortcutKey::Char(c), Modifiers::alt())
    }

    /// Create a Shift+key shortcut
    pub fn shift(c: char) -> Self {
        Self::new(ShortcutKey::Char(c), Modifiers::shift())
    }

    /// Create a Ctrl+Shift+key shortcut
    pub fn ctrl_shift(c: char) -> Self {
        Self::new(ShortcutKey::Char(c), Modifiers::ctrl_shift())
    }

    /// Create a Ctrl+Alt+key shortcut
    pub fn ctrl_alt(c: char) -> Self {
        Self::new(ShortcutKey::Char(c), Modifiers::ctrl_alt())
    }

    /// Create a function key shortcut
    pub fn f(n: u8) -> Self {
        Self::new(ShortcutKey::F(n), Modifiers::none())
    }

    /// Create a Ctrl+F key shortcut
    pub fn ctrl_f(n: u8) -> Self {
        Self::new(ShortcutKey::F(n), Modifiers::ctrl())
    }

    /// Create an Enter shortcut
    pub fn enter() -> Self {
        Self::new(ShortcutKey::Enter, Modifiers::none())
    }

    /// Create an Escape shortcut
    pub fn escape() -> Self {
        Self::new(ShortcutKey::Escape, Modifiers::none())
    }

    /// Create a Tab shortcut
    pub fn tab() -> Self {
        Self::new(ShortcutKey::Tab, Modifiers::none())
    }

    /// Create a Shift+Tab shortcut
    pub fn shift_tab() -> Self {
        Self::new(ShortcutKey::Tab, Modifiers::shift())
    }

    /// Create a Space shortcut
    pub fn space() -> Self {
        Self::new(ShortcutKey::Space, Modifiers::none())
    }

    /// Create an arrow up shortcut
    pub fn up() -> Self {
        Self::new(ShortcutKey::Up, Modifiers::none())
    }

    /// Create an arrow down shortcut
    pub fn down() -> Self {
        Self::new(ShortcutKey::Down, Modifiers::none())
    }

    /// Create an arrow left shortcut
    pub fn left() -> Self {
        Self::new(ShortcutKey::Left, Modifiers::none())
    }

    /// Create an arrow right shortcut
    pub fn right() -> Self {
        Self::new(ShortcutKey::Right, Modifiers::none())
    }

    /// Check if a Key matches this shortcut
    pub fn matches(&self, input: &str, key: &Key) -> bool {
        // Check modifiers first
        if !self.modifiers.matches(key) {
            return false;
        }

        // Check the key
        match &self.key {
            ShortcutKey::Char(c) => {
                let mut chars = input.chars();
                match (chars.next(), chars.next()) {
                    (Some(input_char), None) => input_char.eq_ignore_ascii_case(c),
                    _ => false,
                }
            }
            ShortcutKey::F(1) => key.f1,
            ShortcutKey::F(2) => key.f2,
            ShortcutKey::F(3) => key.f3,
            ShortcutKey::F(4) => key.f4,
            ShortcutKey::F(5) => key.f5,
            ShortcutKey::F(6) => key.f6,
            ShortcutKey::F(7) => key.f7,
            ShortcutKey::F(8) => key.f8,
            ShortcutKey::F(9) => key.f9,
            ShortcutKey::F(10) => key.f10,
            ShortcutKey::F(11) => key.f11,
            ShortcutKey::F(12) => key.f12,
            ShortcutKey::F(_) => false,
            ShortcutKey::Enter => key.return_key,
            ShortcutKey::Escape => key.escape,
            ShortcutKey::Tab => key.tab,
            ShortcutKey::Backspace => key.backspace,
            ShortcutKey::Delete => key.delete,
            ShortcutKey::Up => key.up_arrow,
            ShortcutKey::Down => key.down_arrow,
            ShortcutKey::Left => key.left_arrow,
            ShortcutKey::Right => key.right_arrow,
            ShortcutKey::Home => key.home,
            ShortcutKey::End => key.end,
            ShortcutKey::PageUp => key.page_up,
            ShortcutKey::PageDown => key.page_down,
            ShortcutKey::Insert => key.insert,
            ShortcutKey::Space => key.space,
        }
    }

    /// Get a human-readable description of the shortcut
    pub fn description(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.alt {
            parts.push("Alt".to_string());
        }
        if self.modifiers.shift {
            parts.push("Shift".to_string());
        }

        let key_str = match &self.key {
            ShortcutKey::Char(c) => c.to_uppercase().to_string(),
            ShortcutKey::F(n) => format!("F{}", n),
            ShortcutKey::Enter => "Enter".to_string(),
            ShortcutKey::Escape => "Esc".to_string(),
            ShortcutKey::Tab => "Tab".to_string(),
            ShortcutKey::Backspace => "Backspace".to_string(),
            ShortcutKey::Delete => "Delete".to_string(),
            ShortcutKey::Up => "↑".to_string(),
            ShortcutKey::Down => "↓".to_string(),
            ShortcutKey::Left => "←".to_string(),
            ShortcutKey::Right => "→".to_string(),
            ShortcutKey::Home => "Home".to_string(),
            ShortcutKey::End => "End".to_string(),
            ShortcutKey::PageUp => "PgUp".to_string(),
            ShortcutKey::PageDown => "PgDn".to_string(),
            ShortcutKey::Insert => "Insert".to_string(),
            ShortcutKey::Space => "Space".to_string(),
        };

        parts.push(key_str);
        parts.join("+")
    }
}

/// Hook to handle a keyboard shortcut
///
/// Calls the callback when the shortcut is pressed.
pub fn use_keyboard_shortcut<F>(shortcut: Shortcut, callback: F)
where
    F: Fn() + 'static,
{
    use_input(move |input, key| {
        if shortcut.matches(input, key) {
            callback();
        }
    });
}

/// Hook to handle multiple keyboard shortcuts
///
/// Takes a list of (shortcut, callback) pairs.
pub fn use_keyboard_shortcuts<F>(shortcuts: Vec<(Shortcut, F)>)
where
    F: Fn() + 'static,
{
    use_input(move |input, key| {
        for (shortcut, callback) in &shortcuts {
            if shortcut.matches(input, key) {
                callback();
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifiers() {
        let none = Modifiers::none();
        assert!(!none.ctrl && !none.alt && !none.shift);

        let ctrl = Modifiers::ctrl();
        assert!(ctrl.ctrl && !ctrl.alt && !ctrl.shift);

        let ctrl_shift = Modifiers::ctrl_shift();
        assert!(ctrl_shift.ctrl && !ctrl_shift.alt && ctrl_shift.shift);
    }

    #[test]
    fn test_shortcut_creation() {
        let s = Shortcut::ctrl('s');
        assert_eq!(s.key, ShortcutKey::Char('s'));
        assert!(s.modifiers.ctrl);

        let f1 = Shortcut::f(1);
        assert_eq!(f1.key, ShortcutKey::F(1));
    }

    #[test]
    fn test_shortcut_matches_ctrl_char() {
        let shortcut = Shortcut::ctrl('s');
        let key = Key {
            ctrl: true,
            ..Key::default()
        };

        assert!(shortcut.matches("s", &key));
        assert!(shortcut.matches("S", &key));

        let key = Key::default();
        assert!(!shortcut.matches("s", &key));
    }

    #[test]
    fn test_shortcut_matches_plain_char() {
        let shortcut = Shortcut::char('a');
        let key = Key::default();

        assert!(shortcut.matches("a", &key));
        assert!(shortcut.matches("A", &key));
        assert!(!shortcut.matches("b", &key));
        assert!(!shortcut.matches("", &key));
        assert!(!shortcut.matches("ab", &key));
    }

    #[test]
    fn test_shortcut_matches_function_key() {
        let shortcut = Shortcut::f(1);
        let key = Key {
            f1: true,
            ..Key::default()
        };

        assert!(shortcut.matches("", &key));

        let key = Key {
            f2: true,
            ..Key::default()
        };
        assert!(!shortcut.matches("", &key));
    }

    #[test]
    fn test_shortcut_matches_special_keys() {
        let key = Key {
            return_key: true,
            ..Key::default()
        };
        assert!(Shortcut::enter().matches("", &key));

        let key = Key {
            escape: true,
            ..Key::default()
        };
        assert!(Shortcut::escape().matches("", &key));

        let key = Key {
            tab: true,
            ..Key::default()
        };
        assert!(Shortcut::tab().matches("", &key));

        let key = Key {
            up_arrow: true,
            ..Key::default()
        };
        assert!(Shortcut::up().matches("", &key));

        let key = Key {
            down_arrow: true,
            ..Key::default()
        };
        assert!(Shortcut::down().matches("", &key));

        let key = Key {
            left_arrow: true,
            ..Key::default()
        };
        assert!(Shortcut::left().matches("", &key));

        let key = Key {
            right_arrow: true,
            ..Key::default()
        };
        assert!(Shortcut::right().matches("", &key));
    }

    #[test]
    fn test_shortcut_description() {
        assert_eq!(Shortcut::ctrl('s').description(), "Ctrl+S");
        assert_eq!(Shortcut::ctrl_shift('z').description(), "Ctrl+Shift+Z");
        assert_eq!(Shortcut::f(1).description(), "F1");
        assert_eq!(Shortcut::escape().description(), "Esc");
        assert_eq!(Shortcut::shift_tab().description(), "Shift+Tab");
    }

    #[test]
    fn test_modifiers_matches() {
        let key = Key {
            ctrl: true,
            shift: true,
            ..Key::default()
        };

        assert!(Modifiers::ctrl_shift().matches(&key));
        assert!(!Modifiers::ctrl().matches(&key));
        assert!(!Modifiers::none().matches(&key));
    }

    #[test]
    fn test_use_keyboard_shortcut_compiles() {
        fn _test() {
            use_keyboard_shortcut(Shortcut::ctrl('s'), || {});
        }
    }
}
