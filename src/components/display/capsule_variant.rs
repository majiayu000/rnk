//! Shared variant palette for capsule-like display components.

use crate::components::{ComponentState, ComponentVariant, Theme, get_theme};
use crate::core::Color;

/// Shared variant set used by badge/highlight style components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CapsuleVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
    Info,
}

impl CapsuleVariant {
    pub(crate) fn badge_colors(self) -> (Color, Color) {
        self.badge_colors_with_theme(&get_theme())
    }

    pub(crate) fn badge_colors_with_theme(self, theme: &Theme) -> (Color, Color) {
        let style = theme.variant_style(self.into(), ComponentState::Rest);
        (style.fg, style.bg)
    }

    pub(crate) fn highlight_colors(self) -> (Color, Color) {
        self.highlight_colors_with_theme(&get_theme())
    }

    pub(crate) fn highlight_colors_with_theme(self, theme: &Theme) -> (Color, Color) {
        let state = match self {
            Self::Default => ComponentState::Selected,
            _ => ComponentState::Rest,
        };
        let style = theme.variant_style(self.into(), state);
        (style.fg, style.bg)
    }
}

impl From<CapsuleVariant> for ComponentVariant {
    fn from(variant: CapsuleVariant) -> Self {
        match variant {
            CapsuleVariant::Default => ComponentVariant::Default,
            CapsuleVariant::Primary => ComponentVariant::Primary,
            CapsuleVariant::Secondary => ComponentVariant::Secondary,
            CapsuleVariant::Success => ComponentVariant::Success,
            CapsuleVariant::Warning => ComponentVariant::Warning,
            CapsuleVariant::Error => ComponentVariant::Error,
            CapsuleVariant::Info => ComponentVariant::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_colors_resolve_from_theme() {
        let theme = Theme::dark();
        let (fg, bg) = CapsuleVariant::Primary.badge_colors_with_theme(&theme);
        let expected = theme.variant_style(ComponentVariant::Primary, ComponentState::Rest);
        assert_eq!((fg, bg), (expected.fg, expected.bg));
    }
}
