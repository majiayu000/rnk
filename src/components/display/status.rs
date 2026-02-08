//! Shared status level styling for components (alert, notification, etc.)

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

/// Get shared style data for a status level.
pub fn status_style(level: StatusLevel) -> StatusStyle {
    match level {
        StatusLevel::Info => StatusStyle {
            icon: "ℹ",
            label: "INFO",
            fg: Color::Cyan,
            bg: Color::Ansi256(23),
        },
        StatusLevel::Success => StatusStyle {
            icon: "✓",
            label: "SUCCESS",
            fg: Color::Green,
            bg: Color::Ansi256(22),
        },
        StatusLevel::Warning => StatusStyle {
            icon: "⚠",
            label: "WARNING",
            fg: Color::Yellow,
            bg: Color::Ansi256(58),
        },
        StatusLevel::Error => StatusStyle {
            icon: "✗",
            label: "ERROR",
            fg: Color::Red,
            bg: Color::Ansi256(52),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_style_info() {
        let style = status_style(StatusLevel::Info);
        assert_eq!(style.icon, "ℹ");
        assert_eq!(style.label, "INFO");
        assert_eq!(style.fg, Color::Cyan);
        assert_eq!(style.bg, Color::Ansi256(23));
    }
}
