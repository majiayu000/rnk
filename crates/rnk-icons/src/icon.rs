//! Icon type with optional styling

use std::fmt;

/// An icon with optional color
#[derive(Debug, Clone)]
pub struct Icon {
    /// The icon character(s)
    pub glyph: &'static str,
    /// Optional hex color (e.g., "#ff0000")
    pub color: Option<String>,
}

impl Icon {
    /// Create a new icon from a glyph
    pub fn new(glyph: &'static str) -> Self {
        Self { glyph, color: None }
    }

    /// Set the icon color (hex format)
    pub fn colored(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Get the glyph string
    pub fn glyph(&self) -> &'static str {
        self.glyph
    }

    /// Check if icon has a color
    pub fn has_color(&self) -> bool {
        self.color.is_some()
    }

    /// Get the color if set
    pub fn get_color(&self) -> Option<&str> {
        self.color.as_deref()
    }
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.glyph)
    }
}

impl From<&'static str> for Icon {
    fn from(glyph: &'static str) -> Self {
        Self::new(glyph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_creation() {
        let icon = Icon::new("");
        assert_eq!(icon.glyph(), "");
        assert!(!icon.has_color());
    }

    #[test]
    fn test_icon_with_color() {
        let icon = Icon::new("").colored("#ff0000");
        assert!(icon.has_color());
        assert_eq!(icon.get_color(), Some("#ff0000"));
    }

    #[test]
    fn test_icon_display() {
        let icon = Icon::new("");
        assert_eq!(format!("{}", icon), "");
    }
}
