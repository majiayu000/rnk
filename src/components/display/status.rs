//! Shared status level styling for components (alert, notification, etc.)

use crate::components::{Theme, get_theme};
use crate::core::Color;

/// Common status levels used across components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Shared styling for a status level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusStyle {
    pub icon: &'static str,
    pub label: &'static str,
    pub fg: Color,
    pub bg: Color,
}

/// Generate `From<$Level> for StatusLevel` for enums with identical Info/Success/Warning/Error variants.
macro_rules! impl_status_level_from {
    ($level_type:ty) => {
        impl From<$level_type> for StatusLevel {
            fn from(level: $level_type) -> Self {
                match level {
                    <$level_type>::Info => StatusLevel::Info,
                    <$level_type>::Success => StatusLevel::Success,
                    <$level_type>::Warning => StatusLevel::Warning,
                    <$level_type>::Error => StatusLevel::Error,
                }
            }
        }
    };
}

pub(crate) use impl_status_level_from;

/// Get shared style data for a status level.
pub fn status_style(level: StatusLevel) -> StatusStyle {
    status_style_with_theme(level, &get_theme())
}

/// Get shared style data for a status level from a specific theme.
pub fn status_style_with_theme(level: StatusLevel, theme: &Theme) -> StatusStyle {
    let tokens = theme.design_tokens();
    let fg = match level {
        StatusLevel::Info => theme.primary,
        StatusLevel::Success => theme.success,
        StatusLevel::Warning => theme.warning,
        StatusLevel::Error => theme.error,
    };
    let bg = theme.background.elevated;

    match level {
        StatusLevel::Info => StatusStyle {
            icon: tokens.symbols.info,
            label: "INFO",
            fg,
            bg,
        },
        StatusLevel::Success => StatusStyle {
            icon: tokens.symbols.success,
            label: "SUCCESS",
            fg,
            bg,
        },
        StatusLevel::Warning => StatusStyle {
            icon: tokens.symbols.warning,
            label: "WARNING",
            fg,
            bg,
        },
        StatusLevel::Error => StatusStyle {
            icon: tokens.symbols.error,
            label: "ERROR",
            fg,
            bg,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_style_info() {
        let style = status_style(StatusLevel::Info);
        let theme = get_theme();

        assert_eq!(style.icon, theme.design_tokens().symbols.info);
        assert_eq!(style.label, "INFO");
        assert_eq!(style.fg, theme.primary);
        assert_eq!(style.bg, theme.background.elevated);
    }
}
