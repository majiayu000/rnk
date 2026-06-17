//! Shared action/button primitives used by higher-level components.

use crate::core::{Color, Element};

use super::display::Text;
use super::theme::{ComponentState, ComponentVariant, Theme, get_theme};

/// Semantic role for button-like actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionRole {
    /// Main affirmative action.
    #[default]
    Primary,
    /// Secondary or cancel-style action.
    Secondary,
    /// Destructive action.
    Destructive,
}

impl ActionRole {
    pub(crate) fn variant(self) -> ComponentVariant {
        match self {
            Self::Primary => ComponentVariant::Primary,
            Self::Secondary => ComponentVariant::Secondary,
            Self::Destructive => ComponentVariant::Error,
        }
    }
}

/// Visual state for button-like actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionState {
    /// Normal action state.
    #[default]
    Rest,
    /// Focused action state.
    Focused,
    /// Disabled action state.
    Disabled,
}

impl From<ActionState> for ComponentState {
    fn from(state: ActionState) -> Self {
        match state {
            ActionState::Rest => ComponentState::Rest,
            ActionState::Focused => ComponentState::Focused,
            ActionState::Disabled => ComponentState::Disabled,
        }
    }
}

/// Shape used when formatting an action label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionShape {
    /// `[Yes]`
    #[default]
    Brackets,
    /// `<Yes>`
    Angles,
    /// `(Yes)`
    Parens,
    /// `Yes`
    Plain,
    /// `[ Yes ]`
    Padded,
}

impl ActionShape {
    /// Format a label and optional key hint with this shape.
    pub fn format_label(self, label: &str, hint: Option<char>, show_hint: bool) -> String {
        let hint = if show_hint {
            hint.map(|c| format!("({})", c)).unwrap_or_default()
        } else {
            String::new()
        };

        match self {
            Self::Brackets => format!("[{}]{}", label, hint),
            Self::Angles => format!("<{}>{}", label, hint),
            Self::Parens => format!("({}){}", label, hint),
            Self::Plain => format!("{}{}", label, hint),
            Self::Padded => format!("[ {} ]{}", label, hint),
        }
    }
}

/// Resolved terminal style for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionStyle {
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Option<Color>,
    /// Whether the label should be bold.
    pub bold: bool,
    /// Whether the label should be dimmed.
    pub dim: bool,
}

/// Shared action label builder.
#[derive(Debug, Clone)]
pub struct ActionButton {
    label: String,
    role: ActionRole,
    state: ActionState,
    shape: ActionShape,
    hint: Option<char>,
    show_hint: bool,
}

impl ActionButton {
    /// Create a new action button label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            role: ActionRole::Primary,
            state: ActionState::Rest,
            shape: ActionShape::Brackets,
            hint: None,
            show_hint: false,
        }
    }

    /// Set the semantic action role.
    pub fn role(mut self, role: ActionRole) -> Self {
        self.role = role;
        self
    }

    /// Set the visual action state.
    pub fn state(mut self, state: ActionState) -> Self {
        self.state = state;
        self
    }

    /// Set the label shape.
    pub fn shape(mut self, shape: ActionShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set an optional key hint.
    pub fn hint(mut self, hint: Option<char>) -> Self {
        self.hint = hint;
        self
    }

    /// Show or hide the configured key hint.
    pub fn show_hint(mut self, show: bool) -> Self {
        self.show_hint = show;
        self
    }

    /// Resolve this action against the current theme.
    pub fn style(&self) -> ActionStyle {
        self.style_with_theme(&get_theme())
    }

    /// Resolve this action against a specific theme.
    pub fn style_with_theme(&self, theme: &Theme) -> ActionStyle {
        theme.action_style(self.role, self.state)
    }

    /// Format the action label without resolving colors.
    pub fn formatted_label(&self) -> String {
        self.shape
            .format_label(&self.label, self.hint, self.show_hint)
    }

    /// Convert to a styled text node with the current theme.
    pub fn into_text(self) -> Text {
        let theme = get_theme();
        self.into_text_with_theme(&theme)
    }

    /// Convert to a styled text node with a specific theme.
    pub fn into_text_with_theme(self, theme: &Theme) -> Text {
        let style = self.style_with_theme(theme);
        let mut text = Text::new(self.formatted_label()).color(style.fg);
        if let Some(bg) = style.bg {
            text = text.background(bg);
        }
        if style.bold {
            text = text.bold();
        }
        if style.dim {
            text = text.dim();
        }
        text
    }

    /// Convert to an element with the current theme.
    pub fn into_element(self) -> Element {
        self.into_text().into_element()
    }
}

impl Theme {
    /// Resolve the style for a button-like action.
    pub fn action_style(&self, role: ActionRole, state: ActionState) -> ActionStyle {
        let variant = self.variant_style(role.variant(), state.into());
        let disabled = state == ActionState::Disabled;
        ActionStyle {
            fg: variant.fg,
            bg: Some(variant.bg),
            bold: variant.bold,
            dim: disabled && self.design_tokens().states.disabled_dim,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Theme;

    #[test]
    fn test_action_shape_formats_hints() {
        assert_eq!(
            ActionShape::Padded.format_label("Save", Some('S'), true),
            "[ Save ](S)"
        );
        assert_eq!(
            ActionShape::Plain.format_label("Cancel", Some('C'), false),
            "Cancel"
        );
    }

    #[test]
    fn test_action_button_resolves_theme_tokens() {
        let theme = Theme::dark();
        let style = ActionButton::new("Delete")
            .role(ActionRole::Destructive)
            .state(ActionState::Focused)
            .style_with_theme(&theme);

        assert_eq!(style.fg, theme.components.button.danger_text);
        assert_eq!(style.bg, Some(theme.error));
        assert!(style.bold);
    }
}
