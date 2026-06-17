//! Design tokens and variant resolvers derived from a theme.

use crate::core::{BorderStyle, Color};

use super::Theme;

/// Shared component variants used by theme-aware components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentVariant {
    /// Neutral/default component variant.
    #[default]
    Default,
    /// Primary/accent component variant.
    Primary,
    /// Secondary component variant.
    Secondary,
    /// Success component variant.
    Success,
    /// Warning component variant.
    Warning,
    /// Error/destructive component variant.
    Error,
    /// Informational component variant.
    Info,
}

/// Shared visual states used by theme-aware components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentState {
    /// Normal state.
    #[default]
    Rest,
    /// Focused state.
    Focused,
    /// Selected state.
    Selected,
    /// Disabled state.
    Disabled,
    /// Active/pressed state.
    Active,
}

/// Resolved style for a variant and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantStyle {
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Border color.
    pub border: Color,
    /// Whether text should be bold.
    pub bold: bool,
}

/// Density preset for tokenized component spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    /// Compact spacing for dense tools.
    Compact,
    /// Default spacing for terminal applications.
    #[default]
    Comfortable,
    /// More generous spacing for low-density screens.
    Spacious,
}

/// Spacing tokens measured in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpacingTokens {
    /// Extra-small spacing.
    pub xs: u16,
    /// Small spacing.
    pub sm: u16,
    /// Medium/default spacing.
    pub md: u16,
    /// Large spacing.
    pub lg: u16,
}

/// Density tokens used by layout defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityTokens {
    /// Active density preset.
    pub mode: Density,
    /// Default gap between related inline controls.
    pub gap: u16,
    /// Default block padding.
    pub padding: u16,
    /// Default inline control padding.
    pub inline_padding: u16,
}

/// Border tokens for panels and controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderTokens {
    /// Border style for panels.
    pub panel: BorderStyle,
    /// Border style for dialogs.
    pub dialog: BorderStyle,
    /// Border style for controls.
    pub control: BorderStyle,
    /// Border style used to emphasize focus.
    pub focus: BorderStyle,
}

/// Focus presentation tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusTokens {
    /// Prefix marker for focused rows or actions.
    pub marker: &'static str,
    /// Whether focused labels should be bold.
    pub bold: bool,
}

/// State presentation tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTokens {
    /// Whether selected labels should be bold.
    pub selected_bold: bool,
    /// Whether disabled labels should be dimmed.
    pub disabled_dim: bool,
    /// Whether active labels should be bold.
    pub active_bold: bool,
}

/// Common symbols used by component variants and states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolTokens {
    /// Selected/check symbol.
    pub selected: &'static str,
    /// Unselected/empty symbol.
    pub unselected: &'static str,
    /// Success symbol.
    pub success: &'static str,
    /// Warning symbol.
    pub warning: &'static str,
    /// Error symbol.
    pub error: &'static str,
    /// Info symbol.
    pub info: &'static str,
    /// Disclosure-open symbol.
    pub disclosure_open: &'static str,
    /// Disclosure-closed symbol.
    pub disclosure_closed: &'static str,
}

/// Non-color design token bundle resolved from a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DesignTokens {
    /// Terminal-cell spacing scale.
    pub spacing: SpacingTokens,
    /// Density defaults.
    pub density: DensityTokens,
    /// Border defaults.
    pub borders: BorderTokens,
    /// Focus defaults.
    pub focus: FocusTokens,
    /// State defaults.
    pub states: StateTokens,
    /// Symbol defaults.
    pub symbols: SymbolTokens,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            xs: 0,
            sm: 1,
            md: 1,
            lg: 2,
        }
    }
}

impl DensityTokens {
    /// Create tokens for a density preset.
    pub fn for_density(mode: Density) -> Self {
        match mode {
            Density::Compact => Self {
                mode,
                gap: 1,
                padding: 0,
                inline_padding: 0,
            },
            Density::Comfortable => Self {
                mode,
                gap: 2,
                padding: 1,
                inline_padding: 1,
            },
            Density::Spacious => Self {
                mode,
                gap: 3,
                padding: 2,
                inline_padding: 2,
            },
        }
    }
}

impl Default for DensityTokens {
    fn default() -> Self {
        Self::for_density(Density::Comfortable)
    }
}

impl Default for BorderTokens {
    fn default() -> Self {
        Self {
            panel: BorderStyle::Single,
            dialog: BorderStyle::Round,
            control: BorderStyle::Single,
            focus: BorderStyle::Bold,
        }
    }
}

impl Default for FocusTokens {
    fn default() -> Self {
        Self {
            marker: ">",
            bold: true,
        }
    }
}

impl Default for StateTokens {
    fn default() -> Self {
        Self {
            selected_bold: true,
            disabled_dim: true,
            active_bold: true,
        }
    }
}

impl Default for SymbolTokens {
    fn default() -> Self {
        Self {
            selected: "✓",
            unselected: "○",
            success: "✓",
            warning: "⚠",
            error: "✗",
            info: "ℹ",
            disclosure_open: "▾",
            disclosure_closed: "▸",
        }
    }
}

impl Theme {
    /// Resolve non-color design tokens for this theme.
    ///
    /// Tokens are derived rather than stored on `Theme` so adding this API does
    /// not change the public field layout of `Theme`.
    pub fn design_tokens(&self) -> DesignTokens {
        DesignTokens::default()
    }

    /// Resolve colors and emphasis for a shared component variant/state pair.
    pub fn variant_style(&self, variant: ComponentVariant, state: ComponentState) -> VariantStyle {
        let (fg, bg, border) = match variant {
            ComponentVariant::Default => (
                self.text.primary,
                self.background.elevated,
                self.border.default,
            ),
            ComponentVariant::Primary => (
                self.components.button.primary_text,
                self.components.button.primary_bg,
                self.primary,
            ),
            ComponentVariant::Secondary => (
                self.components.button.secondary_text,
                self.components.button.secondary_bg,
                self.secondary,
            ),
            ComponentVariant::Success => (self.text.inverted, self.success, self.success),
            ComponentVariant::Warning => (self.text.inverted, self.warning, self.warning),
            ComponentVariant::Error => (
                self.components.button.danger_text,
                self.components.button.danger_bg,
                self.error,
            ),
            ComponentVariant::Info => (self.text.inverted, self.info, self.info),
        };

        match state {
            ComponentState::Rest => VariantStyle {
                fg,
                bg,
                border,
                bold: false,
            },
            ComponentState::Focused => VariantStyle {
                fg,
                bg,
                border: self.border.focused,
                bold: self.design_tokens().focus.bold,
            },
            ComponentState::Selected => VariantStyle {
                fg,
                bg,
                border,
                bold: self.design_tokens().states.selected_bold,
            },
            ComponentState::Disabled => VariantStyle {
                fg: self.text.disabled,
                bg: self.background.disabled,
                border: self.border.disabled,
                bold: false,
            },
            ComponentState::Active => VariantStyle {
                fg,
                bg,
                border,
                bold: self.design_tokens().states.active_bold,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_density_tokens_for_density() {
        assert_eq!(DensityTokens::for_density(Density::Compact).padding, 0);
        assert_eq!(DensityTokens::for_density(Density::Spacious).gap, 3);
    }

    #[test]
    fn test_theme_design_tokens_are_derived_without_theme_fields() {
        let theme = Theme::dark();
        let tokens = theme.design_tokens();
        assert_eq!(tokens.borders.dialog, BorderStyle::Round);
        assert_eq!(tokens.focus.marker, ">");
    }

    #[test]
    fn test_variant_style_uses_theme_colors() {
        let theme = Theme::light();
        let style = theme.variant_style(ComponentVariant::Primary, ComponentState::Focused);
        assert_eq!(style.bg, theme.components.button.primary_bg);
        assert_eq!(style.border, theme.border.focused);
        assert!(style.bold);
    }
}
