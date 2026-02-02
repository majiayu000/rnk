//! Keyboard mapping configuration for viewport navigation
//!
//! Provides customizable key bindings for viewport scrolling and navigation.

/// Key binding configuration for viewport navigation
#[derive(Debug, Clone)]
pub struct ViewportKeyMap {
    // Vertical navigation
    /// Scroll up one line (default: up arrow, k)
    pub up: Vec<KeyBinding>,
    /// Scroll down one line (default: down arrow, j)
    pub down: Vec<KeyBinding>,
    /// Page up (default: PageUp, Ctrl+B)
    pub page_up: Vec<KeyBinding>,
    /// Page down (default: PageDown, Ctrl+F, Space)
    pub page_down: Vec<KeyBinding>,
    /// Half page up (default: Ctrl+U)
    pub half_page_up: Vec<KeyBinding>,
    /// Half page down (default: Ctrl+D)
    pub half_page_down: Vec<KeyBinding>,
    /// Go to top (default: Home, g)
    pub goto_top: Vec<KeyBinding>,
    /// Go to bottom (default: End, G)
    pub goto_bottom: Vec<KeyBinding>,

    // Horizontal navigation
    /// Scroll left (default: left arrow, h)
    pub left: Vec<KeyBinding>,
    /// Scroll right (default: right arrow, l)
    pub right: Vec<KeyBinding>,
    /// Go to left edge (default: Home with Shift, 0)
    pub goto_left: Vec<KeyBinding>,
    /// Go to right edge (default: End with Shift, $)
    pub goto_right: Vec<KeyBinding>,
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
    /// Page Up
    PageUp,
    /// Page Down
    PageDown,
    /// Home
    Home,
    /// End
    End,
    /// Space
    Space,
    /// Enter
    Enter,
    /// Escape
    Escape,
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

    /// Create a key binding for a special key
    pub fn special(key: KeyType) -> Self {
        Self::new(key, Modifiers::NONE)
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
            KeyType::PageUp => key.page_up,
            KeyType::PageDown => key.page_down,
            KeyType::Home => key.home,
            KeyType::End => key.end,
            KeyType::Space => key.space,
            KeyType::Enter => key.return_key,
            KeyType::Escape => key.escape,
        }
    }
}

impl Default for ViewportKeyMap {
    fn default() -> Self {
        Self {
            // Vertical navigation
            up: vec![KeyBinding::special(KeyType::Up), KeyBinding::char('k')],
            down: vec![KeyBinding::special(KeyType::Down), KeyBinding::char('j')],
            page_up: vec![KeyBinding::special(KeyType::PageUp), KeyBinding::ctrl('b')],
            page_down: vec![
                KeyBinding::special(KeyType::PageDown),
                KeyBinding::ctrl('f'),
                KeyBinding::special(KeyType::Space),
            ],
            half_page_up: vec![KeyBinding::ctrl('u')],
            half_page_down: vec![KeyBinding::ctrl('d')],
            goto_top: vec![KeyBinding::special(KeyType::Home), KeyBinding::char('g')],
            goto_bottom: vec![KeyBinding::special(KeyType::End), KeyBinding::char('G')],

            // Horizontal navigation
            left: vec![KeyBinding::special(KeyType::Left), KeyBinding::char('h')],
            right: vec![KeyBinding::special(KeyType::Right), KeyBinding::char('l')],
            goto_left: vec![KeyBinding::char('0')],
            goto_right: vec![KeyBinding::char('$')],
        }
    }
}

impl ViewportKeyMap {
    /// Create a new keymap with default bindings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a minimal keymap with only arrow keys
    pub fn arrows_only() -> Self {
        Self {
            up: vec![KeyBinding::special(KeyType::Up)],
            down: vec![KeyBinding::special(KeyType::Down)],
            page_up: vec![KeyBinding::special(KeyType::PageUp)],
            page_down: vec![KeyBinding::special(KeyType::PageDown)],
            half_page_up: vec![],
            half_page_down: vec![],
            goto_top: vec![KeyBinding::special(KeyType::Home)],
            goto_bottom: vec![KeyBinding::special(KeyType::End)],
            left: vec![KeyBinding::special(KeyType::Left)],
            right: vec![KeyBinding::special(KeyType::Right)],
            goto_left: vec![],
            goto_right: vec![],
        }
    }

    /// Create a vim-style keymap
    pub fn vim() -> Self {
        Self {
            up: vec![KeyBinding::char('k')],
            down: vec![KeyBinding::char('j')],
            page_up: vec![KeyBinding::ctrl('b')],
            page_down: vec![KeyBinding::ctrl('f')],
            half_page_up: vec![KeyBinding::ctrl('u')],
            half_page_down: vec![KeyBinding::ctrl('d')],
            goto_top: vec![KeyBinding::char('g')],
            goto_bottom: vec![KeyBinding::char('G')],
            left: vec![KeyBinding::char('h')],
            right: vec![KeyBinding::char('l')],
            goto_left: vec![KeyBinding::char('0')],
            goto_right: vec![KeyBinding::char('$')],
        }
    }

    /// Disable all key bindings
    pub fn disabled() -> Self {
        Self {
            up: vec![],
            down: vec![],
            page_up: vec![],
            page_down: vec![],
            half_page_up: vec![],
            half_page_down: vec![],
            goto_top: vec![],
            goto_bottom: vec![],
            left: vec![],
            right: vec![],
            goto_left: vec![],
            goto_right: vec![],
        }
    }

    /// Check which action matches the input, if any
    pub fn match_action(&self, input: &str, key: &crate::hooks::Key) -> Option<ViewportAction> {
        // Check each action's bindings
        for binding in &self.up {
            if binding.matches(input, key) {
                return Some(ViewportAction::ScrollUp);
            }
        }
        for binding in &self.down {
            if binding.matches(input, key) {
                return Some(ViewportAction::ScrollDown);
            }
        }
        for binding in &self.page_up {
            if binding.matches(input, key) {
                return Some(ViewportAction::PageUp);
            }
        }
        for binding in &self.page_down {
            if binding.matches(input, key) {
                return Some(ViewportAction::PageDown);
            }
        }
        for binding in &self.half_page_up {
            if binding.matches(input, key) {
                return Some(ViewportAction::HalfPageUp);
            }
        }
        for binding in &self.half_page_down {
            if binding.matches(input, key) {
                return Some(ViewportAction::HalfPageDown);
            }
        }
        for binding in &self.goto_top {
            if binding.matches(input, key) {
                return Some(ViewportAction::GotoTop);
            }
        }
        for binding in &self.goto_bottom {
            if binding.matches(input, key) {
                return Some(ViewportAction::GotoBottom);
            }
        }
        for binding in &self.left {
            if binding.matches(input, key) {
                return Some(ViewportAction::ScrollLeft);
            }
        }
        for binding in &self.right {
            if binding.matches(input, key) {
                return Some(ViewportAction::ScrollRight);
            }
        }
        for binding in &self.goto_left {
            if binding.matches(input, key) {
                return Some(ViewportAction::GotoLeft);
            }
        }
        for binding in &self.goto_right {
            if binding.matches(input, key) {
                return Some(ViewportAction::GotoRight);
            }
        }

        None
    }
}

/// Actions that can be performed on a viewport
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportAction {
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    GotoTop,
    GotoBottom,
    ScrollLeft,
    ScrollRight,
    GotoLeft,
    GotoRight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keymap() {
        let keymap = ViewportKeyMap::default();
        assert!(!keymap.up.is_empty());
        assert!(!keymap.down.is_empty());
    }

    #[test]
    fn test_vim_keymap() {
        let keymap = ViewportKeyMap::vim();
        assert!(keymap.up.iter().any(|b| b.key == KeyType::Char('k')));
        assert!(keymap.down.iter().any(|b| b.key == KeyType::Char('j')));
    }

    #[test]
    fn test_disabled_keymap() {
        let keymap = ViewportKeyMap::disabled();
        assert!(keymap.up.is_empty());
        assert!(keymap.down.is_empty());
    }
}
