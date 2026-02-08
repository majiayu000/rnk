//! Context Menu component for right-click style menus
//!
//! Provides a context menu UI for displaying actions.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::ContextMenu;
//!
//! fn app() -> Element {
//!     let items = vec![
//!         MenuItem::new("cut", "Cut").shortcut("Ctrl+X"),
//!         MenuItem::new("copy", "Copy").shortcut("Ctrl+C"),
//!         MenuItem::separator(),
//!         MenuItem::new("paste", "Paste").shortcut("Ctrl+V"),
//!     ];
//!
//!     ContextMenu::new(items).into_element()
//! }
//! ```

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};

/// A menu item
#[derive(Debug, Clone)]
pub enum MenuItem {
    /// A regular action item
    Action {
        /// Unique ID
        id: String,
        /// Display label
        label: String,
        /// Optional keyboard shortcut
        shortcut: Option<String>,
        /// Whether the item is disabled
        disabled: bool,
        /// Optional icon
        icon: Option<String>,
    },
    /// A separator line
    Separator,
    /// A submenu
    Submenu {
        /// Display label
        label: String,
        /// Submenu items
        items: Vec<MenuItem>,
    },
}

impl MenuItem {
    /// Create a new action item
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Action {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            disabled: false,
            icon: None,
        }
    }

    /// Create a separator
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Create a submenu
    pub fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self::Submenu {
            label: label.into(),
            items,
        }
    }

    /// Set keyboard shortcut
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Self::Action {
            shortcut: ref mut s,
            ..
        } = self
        {
            *s = Some(shortcut.into());
        }
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        if let Self::Action {
            disabled: ref mut d,
            ..
        } = self
        {
            *d = disabled;
        }
        self
    }

    /// Set icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        if let Self::Action {
            icon: ref mut i, ..
        } = self
        {
            *i = Some(icon.into());
        }
        self
    }

    /// Check if this is a separator
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator)
    }

    /// Check if this is a submenu
    pub fn is_submenu(&self) -> bool {
        matches!(self, Self::Submenu { .. })
    }

    /// Get the ID if this is an action
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Action { id, .. } => Some(id),
            _ => None,
        }
    }

    /// Get the label
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Action { label, .. } | Self::Submenu { label, .. } => Some(label),
            Self::Separator => None,
        }
    }
}

/// Context menu state
#[derive(Debug, Clone, Default)]
pub struct ContextMenuState {
    /// Whether the menu is open
    pub open: bool,
    /// Selected index
    pub selected: usize,
    /// Position (x, y)
    pub position: (u16, u16),
}

impl ContextMenuState {
    /// Create a new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the menu at a position
    pub fn open_at(&mut self, x: u16, y: u16) {
        self.open = true;
        self.position = (x, y);
        self.selected = 0;
    }

    /// Close the menu
    pub fn close(&mut self) {
        self.open = false;
        self.selected = 0;
    }

    /// Toggle the menu
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Move selection up
    pub fn select_prev(&mut self, items: &[MenuItem]) {
        let selectable_count = items.iter().filter(|i| !i.is_separator()).count();
        if selectable_count == 0 {
            return;
        }

        loop {
            if self.selected > 0 {
                self.selected -= 1;
            } else {
                self.selected = items.len() - 1;
            }
            if !items[self.selected].is_separator() {
                break;
            }
        }
    }

    /// Move selection down
    pub fn select_next(&mut self, items: &[MenuItem]) {
        let selectable_count = items.iter().filter(|i| !i.is_separator()).count();
        if selectable_count == 0 {
            return;
        }

        loop {
            if self.selected < items.len() - 1 {
                self.selected += 1;
            } else {
                self.selected = 0;
            }
            if !items[self.selected].is_separator() {
                break;
            }
        }
    }
}

/// Context menu style
#[derive(Debug, Clone)]
pub struct ContextMenuStyle {
    /// Border color
    pub border_color: Color,
    /// Background color
    pub background: Color,
    /// Text color
    pub text_color: Color,
    /// Selected background
    pub selected_bg: Color,
    /// Selected text color
    pub selected_fg: Color,
    /// Disabled color
    pub disabled_color: Color,
    /// Shortcut color
    pub shortcut_color: Color,
    /// Separator color
    pub separator_color: Color,
    /// Width
    pub width: usize,
    /// Padding
    pub padding: usize,
}

impl Default for ContextMenuStyle {
    fn default() -> Self {
        Self {
            border_color: Color::White,
            background: Color::Black,
            text_color: Color::White,
            selected_bg: Color::Blue,
            selected_fg: Color::White,
            disabled_color: Color::BrightBlack,
            shortcut_color: Color::BrightBlack,
            separator_color: Color::BrightBlack,
            width: 30,
            padding: 1,
        }
    }
}

impl ContextMenuStyle {
    /// Create a new style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set width
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Set selected background
    pub fn selected_bg(mut self, color: Color) -> Self {
        self.selected_bg = color;
        self
    }
}

/// Context menu component
#[derive(Debug)]
pub struct ContextMenu {
    /// Menu items
    items: Vec<MenuItem>,
    /// Current state
    state: ContextMenuState,
    /// Style
    style: ContextMenuStyle,
}

impl ContextMenu {
    /// Create a new context menu
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            items,
            state: ContextMenuState::new(),
            style: ContextMenuStyle::default(),
        }
    }

    /// Set the state
    pub fn state(mut self, state: ContextMenuState) -> Self {
        self.state = state;
        self
    }

    /// Set the style
    pub fn style(mut self, style: ContextMenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Get the selected item
    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.state.selected)
    }

    /// Render a menu item
    fn render_item(&self, item: &MenuItem, is_selected: bool, index: usize) -> Element {
        let padding = " ".repeat(self.style.padding);

        match item {
            MenuItem::Separator => {
                let line = "─".repeat(self.style.width - 2);
                Text::new(format!(" {} ", line))
                    .color(self.style.separator_color)
                    .into_element()
            }
            MenuItem::Action {
                label,
                shortcut,
                disabled,
                icon,
                ..
            } => {
                let mut line = String::new();

                // Selection indicator
                if is_selected && index == self.state.selected {
                    line.push('>');
                } else {
                    line.push(' ');
                }

                line.push_str(&padding);

                // Icon
                if let Some(icon) = icon {
                    line.push_str(icon);
                    line.push(' ');
                }

                // Label
                line.push_str(label);

                // Shortcut (right-aligned)
                if let Some(shortcut) = shortcut {
                    let current_len = line.len();
                    let shortcut_space = self
                        .style
                        .width
                        .saturating_sub(current_len + shortcut.len() + 2);
                    line.push_str(&" ".repeat(shortcut_space));
                    line.push_str(shortcut);
                }

                line.push_str(&padding);

                // Truncate if needed
                if line.len() > self.style.width {
                    line.truncate(self.style.width - 3);
                    line.push_str("...");
                }

                // Pad to width
                while line.len() < self.style.width {
                    line.push(' ');
                }

                let (fg, bg) = if is_selected && index == self.state.selected {
                    (self.style.selected_fg, self.style.selected_bg)
                } else if *disabled {
                    (self.style.disabled_color, self.style.background)
                } else {
                    (self.style.text_color, self.style.background)
                };

                Text::new(line).color(fg).background(bg).into_element()
            }
            MenuItem::Submenu { label, .. } => {
                let mut line = String::new();

                // Selection indicator
                if is_selected && index == self.state.selected {
                    line.push('>');
                } else {
                    line.push(' ');
                }

                line.push_str(&padding);
                line.push_str(label);

                // Submenu arrow (right-aligned)
                let arrow = "▶";
                let current_len = line.len();
                let arrow_space = self.style.width.saturating_sub(current_len + 2);
                line.push_str(&" ".repeat(arrow_space));
                line.push_str(arrow);

                line.push_str(&padding);

                let (fg, bg) = if is_selected && index == self.state.selected {
                    (self.style.selected_fg, self.style.selected_bg)
                } else {
                    (self.style.text_color, self.style.background)
                };

                Text::new(line).color(fg).background(bg).into_element()
            }
        }
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        if !self.state.open {
            return Box::new().into_element();
        }

        let mut container = Box::new().flex_direction(FlexDirection::Column);

        // Top border
        let top_border = format!("┌{}┐", "─".repeat(self.style.width - 2));
        container = container.child(
            Text::new(top_border)
                .color(self.style.border_color)
                .into_element(),
        );

        // Items
        for (i, item) in self.items.iter().enumerate() {
            let is_selected = i == self.state.selected;
            container = container.child(self.render_item(item, is_selected, i));
        }

        // Bottom border
        let bottom_border = format!("└{}┘", "─".repeat(self.style.width - 2));
        container = container.child(
            Text::new(bottom_border)
                .color(self.style.border_color)
                .into_element(),
        );

        container.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_item_action() {
        let item = MenuItem::new("test", "Test Item");
        assert_eq!(item.id(), Some("test"));
        assert_eq!(item.label(), Some("Test Item"));
        assert!(!item.is_separator());
    }

    #[test]
    fn test_menu_item_separator() {
        let item = MenuItem::separator();
        assert!(item.is_separator());
        assert!(item.id().is_none());
    }

    #[test]
    fn test_menu_item_submenu() {
        let item = MenuItem::submenu("Edit", vec![MenuItem::new("cut", "Cut")]);
        assert!(item.is_submenu());
        assert_eq!(item.label(), Some("Edit"));
    }

    #[test]
    fn test_menu_item_builder() {
        let item = MenuItem::new("copy", "Copy")
            .shortcut("Ctrl+C")
            .icon("📋")
            .disabled(false);

        if let MenuItem::Action {
            shortcut,
            icon,
            disabled,
            ..
        } = item
        {
            assert_eq!(shortcut, Some("Ctrl+C".to_string()));
            assert_eq!(icon, Some("📋".to_string()));
            assert!(!disabled);
        }
    }

    #[test]
    fn test_context_menu_state() {
        let mut state = ContextMenuState::new();
        assert!(!state.open);

        state.open_at(10, 20);
        assert!(state.open);
        assert_eq!(state.position, (10, 20));

        state.close();
        assert!(!state.open);
    }

    #[test]
    fn test_context_menu_state_navigation() {
        let items = vec![
            MenuItem::new("a", "A"),
            MenuItem::separator(),
            MenuItem::new("b", "B"),
            MenuItem::new("c", "C"),
        ];

        let mut state = ContextMenuState::new();
        state.selected = 0;

        // Should skip separator
        state.select_next(&items);
        assert_eq!(state.selected, 2);

        state.select_next(&items);
        assert_eq!(state.selected, 3);

        state.select_prev(&items);
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn test_context_menu_creation() {
        let items = vec![MenuItem::new("cut", "Cut"), MenuItem::new("copy", "Copy")];

        let menu = ContextMenu::new(items);
        assert_eq!(menu.items.len(), 2);
    }

    #[test]
    fn test_context_menu_into_element() {
        let items = vec![MenuItem::new("test", "Test")];
        let mut state = ContextMenuState::new();
        state.open = true;

        let menu = ContextMenu::new(items).state(state);
        let _ = menu.into_element();
    }

    #[test]
    fn test_context_menu_style() {
        let style = ContextMenuStyle::new().width(40).background(Color::Blue);

        assert_eq!(style.width, 40);
        assert_eq!(style.background, Color::Blue);
    }
}
