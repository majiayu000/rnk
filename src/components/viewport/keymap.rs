//! Keyboard mapping configuration for viewport navigation
//!
//! Provides customizable key bindings for viewport scrolling and navigation.

pub use crate::components::keymap::{KeyBinding, KeyType, Modifiers};

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

impl Default for ViewportKeyMap {
    fn default() -> Self {
        Self {
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
        let checks: &[(&[KeyBinding], ViewportAction)] = &[
            (&self.up, ViewportAction::ScrollUp),
            (&self.down, ViewportAction::ScrollDown),
            (&self.page_up, ViewportAction::PageUp),
            (&self.page_down, ViewportAction::PageDown),
            (&self.half_page_up, ViewportAction::HalfPageUp),
            (&self.half_page_down, ViewportAction::HalfPageDown),
            (&self.goto_top, ViewportAction::GotoTop),
            (&self.goto_bottom, ViewportAction::GotoBottom),
            (&self.left, ViewportAction::ScrollLeft),
            (&self.right, ViewportAction::ScrollRight),
            (&self.goto_left, ViewportAction::GotoLeft),
            (&self.goto_right, ViewportAction::GotoRight),
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
