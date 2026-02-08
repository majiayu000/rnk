//! Shared variant palette for capsule-like display components.

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
        match self {
            Self::Default => (Color::White, Color::Ansi256(240)),
            Self::Primary => (Color::White, Color::Blue),
            Self::Secondary => (Color::White, Color::Ansi256(245)),
            Self::Success => (Color::White, Color::Green),
            Self::Warning => (Color::Black, Color::Yellow),
            Self::Error => (Color::White, Color::Red),
            Self::Info => (Color::White, Color::Cyan),
        }
    }

    pub(crate) fn highlight_colors(self) -> (Color, Color) {
        match self {
            Self::Default => (Color::Black, Color::Yellow),
            Self::Primary => (Color::White, Color::Blue),
            Self::Secondary => (Color::White, Color::Ansi256(245)),
            Self::Success => (Color::White, Color::Green),
            Self::Warning => (Color::Black, Color::Yellow),
            Self::Error => (Color::White, Color::Red),
            Self::Info => (Color::White, Color::Cyan),
        }
    }
}
