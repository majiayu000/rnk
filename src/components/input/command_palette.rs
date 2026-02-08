//! Command Palette component for searchable command lists
//!
//! Provides a command palette UI similar to VS Code's Ctrl+Shift+P.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::CommandPalette;
//!
//! fn app() -> Element {
//!     let commands = vec![
//!         Command::new("file.open", "Open File"),
//!         Command::new("file.save", "Save File"),
//!         Command::new("edit.undo", "Undo"),
//!     ];
//!
//!     CommandPalette::new(commands)
//!         .placeholder("Type a command...")
//!         .into_element()
//! }
//! ```

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};

/// A command in the palette
#[derive(Debug, Clone)]
pub struct Command {
    /// Unique command ID
    pub id: String,
    /// Display label
    pub label: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional keyboard shortcut
    pub shortcut: Option<String>,
    /// Optional category/group
    pub category: Option<String>,
    /// Whether the command is disabled
    pub disabled: bool,
}

impl Command {
    /// Create a new command
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            shortcut: None,
            category: None,
            disabled: false,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set keyboard shortcut
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set category
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Check if command matches a query
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let query_lower = query.to_lowercase();
        let label_lower = self.label.to_lowercase();
        let id_lower = self.id.to_lowercase();

        // Check label
        if label_lower.contains(&query_lower) {
            return true;
        }

        // Check ID
        if id_lower.contains(&query_lower) {
            return true;
        }

        // Check description
        if let Some(desc) = &self.description {
            if desc.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        // Check category
        if let Some(cat) = &self.category {
            if cat.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        // Fuzzy match on label
        fuzzy_match(&label_lower, &query_lower)
    }

    /// Calculate match score for sorting
    pub fn match_score(&self, query: &str) -> i32 {
        if query.is_empty() {
            return 0;
        }

        let query_lower = query.to_lowercase();
        let label_lower = self.label.to_lowercase();

        // Exact match
        if label_lower == query_lower {
            return 100;
        }

        // Starts with
        if label_lower.starts_with(&query_lower) {
            return 80;
        }

        // Contains
        if label_lower.contains(&query_lower) {
            return 60;
        }

        // ID match
        if self.id.to_lowercase().contains(&query_lower) {
            return 40;
        }

        // Fuzzy match
        if fuzzy_match(&label_lower, &query_lower) {
            return 20;
        }

        0
    }
}

/// Simple fuzzy matching
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    for c in text.chars() {
        if let Some(&p) = pattern_chars.peek() {
            if c == p {
                pattern_chars.next();
            }
        }
    }
    pattern_chars.peek().is_none()
}

/// Command palette state
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    /// Current search query
    pub query: String,
    /// Selected index
    pub selected: usize,
    /// Whether the palette is open
    pub open: bool,
}

impl CommandPaletteState {
    /// Create a new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the palette
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
    }

    /// Close the palette
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
    }

    /// Toggle the palette
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Set the query
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.selected = 0;
    }

    /// Move selection up
    pub fn select_prev(&mut self, max: usize) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if max > 0 {
            self.selected = max - 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self, max: usize) {
        if max > 0 && self.selected < max - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }
}

/// Command palette style
#[derive(Debug, Clone)]
pub struct CommandPaletteStyle {
    /// Border color
    pub border_color: Color,
    /// Background color
    pub background: Color,
    /// Text color
    pub text_color: Color,
    /// Selected item background
    pub selected_bg: Color,
    /// Selected item text color
    pub selected_fg: Color,
    /// Disabled item color
    pub disabled_color: Color,
    /// Shortcut color
    pub shortcut_color: Color,
    /// Description color
    pub description_color: Color,
    /// Maximum visible items
    pub max_visible: usize,
    /// Width
    pub width: usize,
}

impl Default for CommandPaletteStyle {
    fn default() -> Self {
        Self {
            border_color: Color::White,
            background: Color::Black,
            text_color: Color::White,
            selected_bg: Color::Blue,
            selected_fg: Color::White,
            disabled_color: Color::BrightBlack,
            shortcut_color: Color::Cyan,
            description_color: Color::BrightBlack,
            max_visible: 10,
            width: 60,
        }
    }
}

impl CommandPaletteStyle {
    /// Create a new style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Set text color
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set selected background
    pub fn selected_bg(mut self, color: Color) -> Self {
        self.selected_bg = color;
        self
    }

    /// Set max visible items
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Set width
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }
}

/// Command palette component
#[derive(Debug)]
pub struct CommandPalette {
    /// Available commands
    commands: Vec<Command>,
    /// Current state
    state: CommandPaletteState,
    /// Style
    style: CommandPaletteStyle,
    /// Placeholder text
    placeholder: String,
    /// Title
    title: Option<String>,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new(commands: Vec<Command>) -> Self {
        Self {
            commands,
            state: CommandPaletteState::new(),
            style: CommandPaletteStyle::default(),
            placeholder: "> Type to search...".to_string(),
            title: None,
        }
    }

    /// Set the state
    pub fn state(mut self, state: CommandPaletteState) -> Self {
        self.state = state;
        self
    }

    /// Set the style
    pub fn style(mut self, style: CommandPaletteStyle) -> Self {
        self.style = style;
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Get filtered and sorted commands
    pub fn filtered_commands(&self) -> Vec<&Command> {
        let mut filtered: Vec<_> = self
            .commands
            .iter()
            .filter(|cmd| cmd.matches(&self.state.query))
            .collect();

        // Sort by match score
        filtered.sort_by(|a, b| {
            b.match_score(&self.state.query)
                .cmp(&a.match_score(&self.state.query))
        });

        filtered
    }

    /// Get the currently selected command
    pub fn selected_command(&self) -> Option<&Command> {
        let filtered = self.filtered_commands();
        filtered.get(self.state.selected).copied()
    }

    /// Render a single command item
    fn render_item(&self, cmd: &Command, is_selected: bool) -> String {
        let mut line = String::new();

        // Selection indicator
        if is_selected {
            line.push_str("> ");
        } else {
            line.push_str("  ");
        }

        // Label
        line.push_str(&cmd.label);

        // Shortcut (right-aligned)
        if let Some(shortcut) = &cmd.shortcut {
            let padding = self
                .style
                .width
                .saturating_sub(line.len() + shortcut.len() + 2);
            line.push_str(&" ".repeat(padding));
            line.push_str(shortcut);
        }

        // Truncate if too long
        if line.len() > self.style.width {
            line.truncate(self.style.width - 3);
            line.push_str("...");
        }

        line
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        if !self.state.open {
            return Box::new().into_element();
        }

        let filtered = self.filtered_commands();
        let visible_count = filtered.len().min(self.style.max_visible);

        let mut container = Box::new().flex_direction(FlexDirection::Column);

        // Title
        if let Some(title) = &self.title {
            container =
                container.child(Text::new(title).color(self.style.text_color).into_element());
        }

        // Search input
        let input_text = if self.state.query.is_empty() {
            self.placeholder.clone()
        } else {
            format!("> {}", self.state.query)
        };
        container = container.child(
            Text::new(input_text)
                .color(self.style.text_color)
                .into_element(),
        );

        // Separator
        container = container.child(
            Text::new("─".repeat(self.style.width))
                .color(self.style.border_color)
                .into_element(),
        );

        // Command list
        for (i, cmd) in filtered.iter().take(visible_count).enumerate() {
            let is_selected = i == self.state.selected;
            let line = self.render_item(cmd, is_selected);

            let (fg, bg) = if is_selected {
                (self.style.selected_fg, self.style.selected_bg)
            } else if cmd.disabled {
                (self.style.disabled_color, self.style.background)
            } else {
                (self.style.text_color, self.style.background)
            };

            container = container.child(Text::new(line).color(fg).background(bg).into_element());
        }

        // Show count if more items
        if filtered.len() > visible_count {
            let more = filtered.len() - visible_count;
            container = container.child(
                Text::new(format!("  ... and {} more", more))
                    .color(self.style.description_color)
                    .into_element(),
            );
        }

        // Empty state
        if filtered.is_empty() {
            container = container.child(
                Text::new("  No commands found")
                    .color(self.style.description_color)
                    .into_element(),
            );
        }

        container.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = Command::new("test.cmd", "Test Command");
        assert_eq!(cmd.id, "test.cmd");
        assert_eq!(cmd.label, "Test Command");
    }

    #[test]
    fn test_command_builder() {
        let cmd = Command::new("file.open", "Open File")
            .description("Open a file from disk")
            .shortcut("Ctrl+O")
            .category("File");

        assert_eq!(cmd.description, Some("Open a file from disk".to_string()));
        assert_eq!(cmd.shortcut, Some("Ctrl+O".to_string()));
        assert_eq!(cmd.category, Some("File".to_string()));
    }

    #[test]
    fn test_command_matches() {
        let cmd = Command::new("file.open", "Open File");

        assert!(cmd.matches(""));
        assert!(cmd.matches("open"));
        assert!(cmd.matches("Open"));
        assert!(cmd.matches("file"));
        assert!(cmd.matches("of")); // fuzzy
        assert!(!cmd.matches("xyz"));
    }

    #[test]
    fn test_command_match_score() {
        let cmd = Command::new("file.open", "Open File");

        assert!(cmd.match_score("Open File") > cmd.match_score("Open"));
        assert!(cmd.match_score("Open") > cmd.match_score("pen"));
        assert!(cmd.match_score("pen") > cmd.match_score("xyz"));
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("open file", "of"));
        assert!(fuzzy_match("open file", "opfl"));
        assert!(fuzzy_match("command palette", "cmdp"));
        assert!(!fuzzy_match("open", "xyz"));
    }

    #[test]
    fn test_palette_state() {
        let mut state = CommandPaletteState::new();
        assert!(!state.open);

        state.open();
        assert!(state.open);
        assert!(state.query.is_empty());

        state.set_query("test");
        assert_eq!(state.query, "test");

        state.close();
        assert!(!state.open);
    }

    #[test]
    fn test_palette_state_navigation() {
        let mut state = CommandPaletteState::new();
        state.selected = 0;

        state.select_next(5);
        assert_eq!(state.selected, 1);

        state.select_next(5);
        assert_eq!(state.selected, 2);

        state.select_prev(5);
        assert_eq!(state.selected, 1);

        state.select_prev(5);
        assert_eq!(state.selected, 0);

        // Wrap around
        state.select_prev(5);
        assert_eq!(state.selected, 4);
    }

    #[test]
    fn test_palette_filtered_commands() {
        let commands = vec![
            Command::new("file.open", "Open File"),
            Command::new("file.save", "Save File"),
            Command::new("edit.undo", "Undo"),
        ];

        let palette = CommandPalette::new(commands);
        let filtered = palette.filtered_commands();
        assert_eq!(filtered.len(), 3);

        let mut state = CommandPaletteState::new();
        state.set_query("file");
        let palette = CommandPalette::new(vec![
            Command::new("file.open", "Open File"),
            Command::new("file.save", "Save File"),
            Command::new("edit.undo", "Undo"),
        ])
        .state(state);

        let filtered = palette.filtered_commands();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_palette_into_element() {
        let commands = vec![Command::new("test", "Test")];
        let mut state = CommandPaletteState::new();
        state.open();

        let palette = CommandPalette::new(commands).state(state);
        let _ = palette.into_element();
    }

    #[test]
    fn test_palette_style() {
        let style = CommandPaletteStyle::new()
            .width(80)
            .max_visible(15)
            .selected_bg(Color::Green);

        assert_eq!(style.width, 80);
        assert_eq!(style.max_visible, 15);
        assert_eq!(style.selected_bg, Color::Green);
    }
}
