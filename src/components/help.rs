//! Help component for displaying keyboard shortcuts
//!
//! Provides a component to display keybindings and help text,
//! similar to Bubbles' help component.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// A single key binding with its description
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// The key(s) to press (e.g., "↑/↓", "j/k", "Ctrl+C")
    pub key: String,
    /// Description of what the key does
    pub description: String,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
        }
    }
}

/// Style configuration for the Help component
#[derive(Debug, Clone)]
pub struct HelpStyle {
    /// Color for the key text
    pub key_color: Option<Color>,
    /// Color for the description text
    pub description_color: Option<Color>,
    /// Separator between key and description
    pub separator: String,
    /// Separator between different bindings (for single-line mode)
    pub binding_separator: String,
    /// Whether to show keys in bold
    pub key_bold: bool,
    /// Whether to dim the description
    pub description_dim: bool,
}

impl Default for HelpStyle {
    fn default() -> Self {
        Self {
            key_color: Some(Color::Cyan),
            description_color: Some(Color::BrightBlack),
            separator: " ".to_string(),
            binding_separator: "  •  ".to_string(),
            key_bold: true,
            description_dim: true,
        }
    }
}

impl HelpStyle {
    /// Create a new style with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the key color
    pub fn key_color(mut self, color: Color) -> Self {
        self.key_color = Some(color);
        self
    }

    /// Set the description color
    pub fn description_color(mut self, color: Color) -> Self {
        self.description_color = Some(color);
        self
    }

    /// Set the separator between key and description
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Set the separator between bindings
    pub fn binding_separator(mut self, sep: impl Into<String>) -> Self {
        self.binding_separator = sep.into();
        self
    }

    /// Set whether keys are bold
    pub fn key_bold(mut self, bold: bool) -> Self {
        self.key_bold = bold;
        self
    }

    /// Set whether descriptions are dimmed
    pub fn description_dim(mut self, dim: bool) -> Self {
        self.description_dim = dim;
        self
    }
}

/// Display mode for the Help component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpMode {
    /// Display all bindings on a single line
    #[default]
    SingleLine,
    /// Display each binding on its own line
    MultiLine,
    /// Display bindings in two columns
    TwoColumn,
}

/// Help component for displaying keyboard shortcuts
///
/// # Example
///
/// ```ignore
/// use rnk::components::{Help, KeyBinding};
///
/// let bindings = vec![
///     KeyBinding::new("↑/↓", "Navigate"),
///     KeyBinding::new("Enter", "Select"),
///     KeyBinding::new("q", "Quit"),
/// ];
///
/// Help::new(bindings).into_element()
/// ```
#[derive(Clone)]
pub struct Help {
    /// Key bindings to display
    bindings: Vec<KeyBinding>,
    /// Display mode
    mode: HelpMode,
    /// Style configuration
    style: HelpStyle,
    /// Maximum width (for wrapping in single-line mode)
    max_width: Option<usize>,
    /// Whether the help is visible
    visible: bool,
}

impl Help {
    /// Create a new Help component with bindings
    pub fn new(bindings: Vec<KeyBinding>) -> Self {
        Self {
            bindings,
            mode: HelpMode::default(),
            style: HelpStyle::default(),
            max_width: None,
            visible: true,
        }
    }

    /// Create from an iterator of (key, description) tuples
    pub fn from_tuples<I, K, D>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, D)>,
        K: Into<String>,
        D: Into<String>,
    {
        let bindings = iter
            .into_iter()
            .map(|(k, d)| KeyBinding::new(k, d))
            .collect();
        Self::new(bindings)
    }

    /// Add a key binding
    pub fn binding(mut self, key: impl Into<String>, description: impl Into<String>) -> Self {
        self.bindings.push(KeyBinding::new(key, description));
        self
    }

    /// Set the display mode
    pub fn mode(mut self, mode: HelpMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set to single-line mode
    pub fn single_line(mut self) -> Self {
        self.mode = HelpMode::SingleLine;
        self
    }

    /// Set to multi-line mode
    pub fn multi_line(mut self) -> Self {
        self.mode = HelpMode::MultiLine;
        self
    }

    /// Set to two-column mode
    pub fn two_column(mut self) -> Self {
        self.mode = HelpMode::TwoColumn;
        self
    }

    /// Set the style configuration
    pub fn style(mut self, style: HelpStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the maximum width
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the number of bindings
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        if !self.visible || self.bindings.is_empty() {
            return TinkBox::new().into_element();
        }

        match self.mode {
            HelpMode::SingleLine => self.render_single_line(),
            HelpMode::MultiLine => self.render_multi_line(),
            HelpMode::TwoColumn => self.render_two_column(),
        }
    }

    fn render_single_line(self) -> Element {
        let mut parts = Vec::new();

        for (i, binding) in self.bindings.iter().enumerate() {
            if i > 0 {
                parts.push(self.style.binding_separator.clone());
            }
            parts.push(binding.key.clone());
            parts.push(self.style.separator.clone());
            parts.push(binding.description.clone());
        }

        let text = parts.join("");
        let mut text_elem = Text::new(&text);

        if self.style.description_dim {
            text_elem = text_elem.dim();
        }

        text_elem.into_element()
    }

    fn render_multi_line(self) -> Element {
        let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

        for binding in &self.bindings {
            let line = format!(
                "{}{}{}",
                binding.key, self.style.separator, binding.description
            );

            let mut text = Text::new(&line);

            if self.style.description_dim {
                text = text.dim();
            }

            container = container.child(text.into_element());
        }

        container.into_element()
    }

    fn render_two_column(self) -> Element {
        let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

        // Find the maximum key width for alignment
        let max_key_width = self
            .bindings
            .iter()
            .map(|b| b.key.chars().count())
            .max()
            .unwrap_or(0);

        for binding in &self.bindings {
            let padded_key = format!("{:width$}", binding.key, width = max_key_width);
            let line = format!(
                "{}{}{}",
                padded_key, self.style.separator, binding.description
            );

            let mut text = Text::new(&line);

            if self.style.description_dim {
                text = text.dim();
            }

            container = container.child(text.into_element());
        }

        container.into_element()
    }
}

impl Default for Help {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Create common navigation help bindings
pub fn navigation_help() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("↑/↓", "Navigate"),
        KeyBinding::new("Enter", "Select"),
        KeyBinding::new("Esc", "Cancel"),
    ]
}

/// Create common vim-style navigation help bindings
pub fn vim_navigation_help() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("j/k", "Navigate"),
        KeyBinding::new("Enter", "Select"),
        KeyBinding::new("q", "Quit"),
    ]
}

/// Create common editor help bindings
pub fn editor_help() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("Ctrl+S", "Save"),
        KeyBinding::new("Ctrl+Q", "Quit"),
        KeyBinding::new("Ctrl+Z", "Undo"),
        KeyBinding::new("Ctrl+Y", "Redo"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_binding_creation() {
        let binding = KeyBinding::new("Ctrl+C", "Copy");
        assert_eq!(binding.key, "Ctrl+C");
        assert_eq!(binding.description, "Copy");
    }

    #[test]
    fn test_help_creation() {
        let bindings = vec![
            KeyBinding::new("↑/↓", "Navigate"),
            KeyBinding::new("Enter", "Select"),
        ];
        let help = Help::new(bindings);
        assert_eq!(help.len(), 2);
        assert!(!help.is_empty());
    }

    #[test]
    fn test_help_from_tuples() {
        let help = Help::from_tuples([("↑/↓", "Navigate"), ("Enter", "Select"), ("q", "Quit")]);
        assert_eq!(help.len(), 3);
    }

    #[test]
    fn test_help_builder() {
        let help = Help::new(vec![])
            .binding("↑/↓", "Navigate")
            .binding("Enter", "Select")
            .binding("q", "Quit");
        assert_eq!(help.len(), 3);
    }

    #[test]
    fn test_help_mode() {
        let help = Help::new(vec![]).single_line();
        assert_eq!(help.mode, HelpMode::SingleLine);

        let help = Help::new(vec![]).multi_line();
        assert_eq!(help.mode, HelpMode::MultiLine);

        let help = Help::new(vec![]).two_column();
        assert_eq!(help.mode, HelpMode::TwoColumn);
    }

    #[test]
    fn test_help_visibility() {
        let mut help = Help::new(vec![]);
        assert!(help.is_visible());

        help = help.visible(false);
        assert!(!help.is_visible());

        help.toggle();
        assert!(help.is_visible());
    }

    #[test]
    fn test_help_style() {
        let style = HelpStyle::new()
            .key_color(Color::Green)
            .description_color(Color::White)
            .separator(": ")
            .key_bold(false);

        assert_eq!(style.key_color, Some(Color::Green));
        assert_eq!(style.separator, ": ");
        assert!(!style.key_bold);
    }

    #[test]
    fn test_navigation_help() {
        let bindings = navigation_help();
        assert_eq!(bindings.len(), 3);
    }

    #[test]
    fn test_vim_navigation_help() {
        let bindings = vim_navigation_help();
        assert_eq!(bindings.len(), 3);
    }

    #[test]
    fn test_editor_help() {
        let bindings = editor_help();
        assert_eq!(bindings.len(), 4);
    }

    #[test]
    fn test_help_empty() {
        let help = Help::new(vec![]);
        assert!(help.is_empty());
        assert_eq!(help.len(), 0);
    }
}
