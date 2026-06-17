//! Confirm component for yes/no confirmation dialogs
//!
//! A simple confirmation dialog component for getting user confirmation.
//!
//! # Features
//!
//! - Yes/No confirmation
//! - Customizable button labels
//! - Keyboard navigation (Tab, Enter, Y/N)
//! - Customizable styling
//!
//! # Example
//!
//! ```ignore
//! use rnk::components::{Confirm, ConfirmState};
//! use rnk::hooks::{use_signal, use_input};
//!
//! fn app() -> Element {
//!     let state = use_signal(|| ConfirmState::new("Delete this file?"));
//!
//!     use_input(move |input, key| {
//!         let mut s = state.get();
//!         if handle_confirm_input(&mut s, input, key) {
//!             state.set(s);
//!         }
//!     });
//!
//!     if let Some(confirmed) = state.get().result() {
//!         if confirmed {
//!             // User confirmed
//!         }
//!     }
//!
//!     Confirm::new(&state.get()).into_element()
//! }
//! ```

use crate::components::{
    ActionButton, ActionRole, ActionShape, ActionState, Box as RnkBox, InteractionMode,
    InteractionOutcome, Text, Theme, get_theme,
};
use crate::core::{Color, Element, FlexDirection};

/// Confirm dialog state
#[derive(Debug, Clone)]
pub struct ConfirmState {
    /// The prompt message
    prompt: String,
    /// Currently focused button (true = yes, false = no)
    focused_yes: bool,
    /// Result (None = not answered, Some(true) = yes, Some(false) = no)
    result: Option<bool>,
    /// Default selection (true = yes, false = no)
    default: bool,
}

impl ConfirmState {
    /// Create a new confirm state with a prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            focused_yes: false, // Default focus on "No" for safety
            result: None,
            default: false,
        }
    }

    /// Create with default set to Yes
    pub fn default_yes(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            focused_yes: true,
            result: None,
            default: true,
        }
    }

    /// Create with default set to No
    pub fn default_no(prompt: impl Into<String>) -> Self {
        Self::new(prompt)
    }

    /// Get the prompt message
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Set the prompt message
    pub fn set_prompt(&mut self, prompt: impl Into<String>) {
        self.prompt = prompt.into();
    }

    /// Check if Yes is focused
    pub fn is_yes_focused(&self) -> bool {
        self.focused_yes
    }

    /// Check if No is focused
    pub fn is_no_focused(&self) -> bool {
        !self.focused_yes
    }

    /// Focus Yes button
    pub fn focus_yes(&mut self) {
        self.focused_yes = true;
    }

    /// Focus No button
    pub fn focus_no(&mut self) {
        self.focused_yes = false;
    }

    /// Toggle focus between Yes and No
    pub fn toggle_focus(&mut self) {
        self.focused_yes = !self.focused_yes;
    }

    /// Confirm with Yes
    pub fn confirm(&mut self) {
        self.result = Some(true);
    }

    /// Confirm with No (cancel)
    pub fn cancel(&mut self) {
        self.result = Some(false);
    }

    /// Submit the currently focused option
    pub fn submit(&mut self) {
        self.result = Some(self.focused_yes);
    }

    /// Get the result (None if not answered yet)
    pub fn result(&self) -> Option<bool> {
        self.result
    }

    /// Check if confirmed (Yes was selected)
    pub fn is_confirmed(&self) -> bool {
        self.result == Some(true)
    }

    /// Check if cancelled (No was selected)
    pub fn is_cancelled(&self) -> bool {
        self.result == Some(false)
    }

    /// Check if answered (either Yes or No)
    pub fn is_answered(&self) -> bool {
        self.result.is_some()
    }

    /// Reset the state (clear result, reset focus)
    pub fn reset(&mut self) {
        self.result = None;
        self.focused_yes = self.default;
    }

    /// Get the default selection
    pub fn default(&self) -> bool {
        self.default
    }
}

/// Style configuration for the confirm dialog
#[derive(Debug, Clone)]
pub struct ConfirmStyle {
    /// Yes button label
    pub yes_label: String,
    /// No button label
    pub no_label: String,
    /// Separator between buttons
    pub separator: String,
    /// Color for focused button
    pub focused_color: Option<Color>,
    /// Background for focused button
    pub focused_bg: Option<Color>,
    /// Color for unfocused button
    pub unfocused_color: Option<Color>,
    /// Color for prompt text
    pub prompt_color: Option<Color>,
    /// Show keyboard hints (Y/N)
    pub show_hints: bool,
    /// Button style (brackets, etc.)
    pub button_style: ButtonStyle,
}

/// Button display style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonStyle {
    /// `[Yes] [No]`
    #[default]
    Brackets,
    /// `<Yes> <No>`
    Angles,
    /// `(Yes) (No)`
    Parens,
    /// `Yes | No`
    Plain,
    /// `[ Yes ] [ No ]` with padding
    Padded,
}

impl Default for ConfirmStyle {
    fn default() -> Self {
        Self::from_theme(&Theme::dark())
    }
}

impl From<ButtonStyle> for ActionShape {
    fn from(style: ButtonStyle) -> Self {
        match style {
            ButtonStyle::Brackets => ActionShape::Brackets,
            ButtonStyle::Angles => ActionShape::Angles,
            ButtonStyle::Parens => ActionShape::Parens,
            ButtonStyle::Plain => ActionShape::Plain,
            ButtonStyle::Padded => ActionShape::Padded,
        }
    }
}

impl ConfirmStyle {
    /// Create a style from the resolved theme tokens.
    pub fn from_theme(theme: &Theme) -> Self {
        let tokens = theme.design_tokens();
        let focused = theme.action_style(ActionRole::Primary, ActionState::Focused);
        let rest = theme.action_style(ActionRole::Secondary, ActionState::Rest);
        Self {
            yes_label: "Yes".to_string(),
            no_label: "No".to_string(),
            separator: " ".repeat(tokens.density.gap as usize),
            focused_color: Some(focused.fg),
            focused_bg: focused.bg,
            unfocused_color: Some(rest.fg),
            prompt_color: Some(theme.text.primary),
            show_hints: true,
            button_style: ButtonStyle::Brackets,
        }
    }

    /// Create a new style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set Yes label
    pub fn yes_label(mut self, label: impl Into<String>) -> Self {
        self.yes_label = label.into();
        self
    }

    /// Set No label
    pub fn no_label(mut self, label: impl Into<String>) -> Self {
        self.no_label = label.into();
        self
    }

    /// Set both labels
    pub fn labels(mut self, yes: impl Into<String>, no: impl Into<String>) -> Self {
        self.yes_label = yes.into();
        self.no_label = no.into();
        self
    }

    /// Set separator
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Set focused color
    pub fn focused_color(mut self, color: Color) -> Self {
        self.focused_color = Some(color);
        self
    }

    /// Set focused background
    pub fn focused_bg(mut self, color: Color) -> Self {
        self.focused_bg = Some(color);
        self
    }

    /// Set unfocused color
    pub fn unfocused_color(mut self, color: Color) -> Self {
        self.unfocused_color = Some(color);
        self
    }

    /// Set prompt color
    pub fn prompt_color(mut self, color: Color) -> Self {
        self.prompt_color = Some(color);
        self
    }

    /// Show/hide keyboard hints
    pub fn show_hints(mut self, show: bool) -> Self {
        self.show_hints = show;
        self
    }

    /// Set button style
    pub fn button_style(mut self, style: ButtonStyle) -> Self {
        self.button_style = style;
        self
    }

    /// Format a button label
    fn format_button(&self, label: &str, hint: Option<char>) -> String {
        ActionShape::from(self.button_style).format_label(label, hint, self.show_hints)
    }

    fn action_text(&self, label: &str, hint: char, role: ActionRole, state: ActionState) -> Text {
        let mut text = ActionButton::new(label)
            .role(role)
            .state(state)
            .shape(self.button_style.into())
            .hint(Some(hint))
            .show_hint(self.show_hints)
            .into_text();

        if state == ActionState::Focused {
            if let Some(color) = self.focused_color {
                text = text.color(color);
            }
            if let Some(bg) = self.focused_bg {
                text = text.background(bg);
            }
            text = text.bold();
        } else if let Some(color) = self.unfocused_color {
            text = text.color(color);
        }

        text
    }

    /// Confirm/Cancel style
    pub fn confirm_cancel() -> Self {
        Self::default().labels("Confirm", "Cancel")
    }

    /// OK/Cancel style
    pub fn ok_cancel() -> Self {
        Self::default().labels("OK", "Cancel")
    }

    /// Save/Discard style
    pub fn save_discard() -> Self {
        Self::default().labels("Save", "Discard")
    }

    /// Delete/Keep style
    pub fn delete_keep() -> Self {
        Self::default().labels("Delete", "Keep")
    }
}

/// Confirm dialog component
#[derive(Debug, Clone)]
pub struct Confirm<'a> {
    /// State reference
    state: &'a ConfirmState,
    /// Style configuration
    style: ConfirmStyle,
    /// Whether the dialog is focused
    focused: bool,
}

impl<'a> Confirm<'a> {
    /// Create a new confirm dialog
    pub fn new(state: &'a ConfirmState) -> Self {
        Self {
            state,
            style: ConfirmStyle::from_theme(&get_theme()),
            focused: true,
        }
    }

    /// Set the style
    pub fn style(mut self, style: ConfirmStyle) -> Self {
        self.style = style;
        self
    }

    /// Set whether the dialog is focused
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set Yes label
    pub fn yes_label(mut self, label: impl Into<String>) -> Self {
        self.style.yes_label = label.into();
        self
    }

    /// Set No label
    pub fn no_label(mut self, label: impl Into<String>) -> Self {
        self.style.no_label = label.into();
        self
    }

    /// Render as string (for non-TUI usage)
    pub fn render(&self) -> String {
        let yes_btn = self.style.format_button(&self.style.yes_label, Some('Y'));
        let no_btn = self.style.format_button(&self.style.no_label, Some('N'));

        format!(
            "{} {}{}{}",
            self.state.prompt, yes_btn, self.style.separator, no_btn
        )
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

        // Prompt
        let mut prompt_text = Text::new(&self.state.prompt);
        if let Some(color) = self.style.prompt_color {
            prompt_text = prompt_text.color(color);
        }
        container = container.child(prompt_text.into_element());

        // Buttons row
        let mut buttons = RnkBox::new().flex_direction(FlexDirection::Row);

        let yes_state = if self.focused && self.state.is_yes_focused() {
            ActionState::Focused
        } else {
            ActionState::Rest
        };
        let yes_text =
            self.style
                .action_text(&self.style.yes_label, 'Y', ActionRole::Primary, yes_state);
        buttons = buttons.child(yes_text.into_element());

        // Separator
        buttons = buttons.child(Text::new(&self.style.separator).into_element());

        let no_state = if self.focused && self.state.is_no_focused() {
            ActionState::Focused
        } else {
            ActionState::Rest
        };
        let no_text =
            self.style
                .action_text(&self.style.no_label, 'N', ActionRole::Secondary, no_state);
        buttons = buttons.child(no_text.into_element());

        container = container.child(buttons.into_element());

        container.into_element()
    }
}

/// Handle confirm dialog input
pub fn handle_confirm_input(
    state: &mut ConfirmState,
    input: &str,
    key: &crate::hooks::Key,
) -> bool {
    handle_confirm_input_with_mode(state, input, key, InteractionMode::Enabled).is_handled()
}

/// Handle confirm dialog input with explicit disabled/read-only behavior.
pub fn handle_confirm_input_with_mode(
    state: &mut ConfirmState,
    input: &str,
    key: &crate::hooks::Key,
    mode: InteractionMode,
) -> InteractionOutcome<bool> {
    if mode.is_disabled() {
        return InteractionOutcome::Ignored;
    }

    // Already answered
    if state.is_answered() {
        return InteractionOutcome::Ignored;
    }

    // Tab or arrow keys to toggle
    if key.tab || key.left_arrow || key.right_arrow {
        state.toggle_focus();
        return InteractionOutcome::Handled;
    }

    if key.escape || input.eq_ignore_ascii_case("n") {
        if mode.is_read_only() {
            return InteractionOutcome::Cancelled;
        }
        state.cancel();
        return InteractionOutcome::Cancelled;
    }

    if mode.is_read_only() {
        return InteractionOutcome::Ignored;
    }

    // Enter to submit
    if key.return_key || key.space {
        state.submit();
        return InteractionOutcome::Submitted(state.result().unwrap_or(false));
    }
    // Y for yes
    if input.eq_ignore_ascii_case("y") {
        state.confirm();
        return InteractionOutcome::Submitted(true);
    }

    InteractionOutcome::Ignored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_state_new() {
        let state = ConfirmState::new("Delete?");
        assert_eq!(state.prompt(), "Delete?");
        assert!(!state.is_yes_focused());
        assert!(state.is_no_focused());
        assert!(!state.is_answered());
    }

    #[test]
    fn test_confirm_state_default_yes() {
        let state = ConfirmState::default_yes("Continue?");
        assert!(state.is_yes_focused());
        assert!(!state.is_no_focused());
    }

    #[test]
    fn test_confirm_state_toggle() {
        let mut state = ConfirmState::new("Test?");
        assert!(state.is_no_focused());

        state.toggle_focus();
        assert!(state.is_yes_focused());

        state.toggle_focus();
        assert!(state.is_no_focused());
    }

    #[test]
    fn test_confirm_state_confirm() {
        let mut state = ConfirmState::new("Test?");
        state.confirm();

        assert!(state.is_answered());
        assert!(state.is_confirmed());
        assert!(!state.is_cancelled());
        assert_eq!(state.result(), Some(true));
    }

    #[test]
    fn test_confirm_state_cancel() {
        let mut state = ConfirmState::new("Test?");
        state.cancel();

        assert!(state.is_answered());
        assert!(!state.is_confirmed());
        assert!(state.is_cancelled());
        assert_eq!(state.result(), Some(false));
    }

    #[test]
    fn test_confirm_state_submit() {
        let mut state = ConfirmState::new("Test?");
        state.focus_yes();
        state.submit();

        assert!(state.is_confirmed());

        let mut state = ConfirmState::new("Test?");
        state.focus_no();
        state.submit();

        assert!(state.is_cancelled());
    }

    #[test]
    fn test_confirm_state_reset() {
        let mut state = ConfirmState::default_yes("Test?");
        state.focus_no();
        state.confirm();

        state.reset();

        assert!(!state.is_answered());
        assert!(state.is_yes_focused()); // Reset to default
    }

    #[test]
    fn test_confirm_style_presets() {
        let _default = ConfirmStyle::default();
        let _confirm_cancel = ConfirmStyle::confirm_cancel();
        let _ok_cancel = ConfirmStyle::ok_cancel();
        let _save_discard = ConfirmStyle::save_discard();
        let _delete_keep = ConfirmStyle::delete_keep();
    }

    #[test]
    fn test_confirm_style_from_theme() {
        let theme = Theme::dark();
        let style = ConfirmStyle::from_theme(&theme);
        let expected = theme.action_style(ActionRole::Primary, ActionState::Focused);

        assert_eq!(style.separator, "  ");
        assert_eq!(style.focused_color, Some(expected.fg));
        assert_eq!(style.focused_bg, expected.bg);
        assert_eq!(style.prompt_color, Some(theme.text.primary));
    }

    #[test]
    fn test_confirm_render() {
        let state = ConfirmState::new("Delete file?");
        let confirm = Confirm::new(&state);
        let rendered = confirm.render();

        assert!(rendered.contains("Delete file?"));
        assert!(rendered.contains("Yes"));
        assert!(rendered.contains("No"));
    }

    #[test]
    fn test_handle_confirm_input_with_mode() {
        let mut state = ConfirmState::new("Delete?");
        let outcome = handle_confirm_input_with_mode(
            &mut state,
            "",
            &crate::hooks::Key {
                tab: true,
                ..Default::default()
            },
            InteractionMode::ReadOnly,
        );
        assert_eq!(outcome, InteractionOutcome::Handled);
        assert!(state.is_yes_focused());

        let outcome = handle_confirm_input_with_mode(
            &mut state,
            "y",
            &crate::hooks::Key::default(),
            InteractionMode::ReadOnly,
        );
        assert_eq!(outcome, InteractionOutcome::Ignored);
        assert!(!state.is_answered());

        let outcome = handle_confirm_input_with_mode(
            &mut state,
            "y",
            &crate::hooks::Key::default(),
            InteractionMode::Enabled,
        );
        assert_eq!(outcome, InteractionOutcome::Submitted(true));
        assert!(state.is_confirmed());
    }
}
