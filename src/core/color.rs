//! Color types for terminal styling

use crossterm::style::Color as CrosstermColor;

/// Color type supporting various color formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// Default terminal color
    #[default]
    Reset,

    // Basic colors
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,

    // Bright colors
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,

    /// 256-color palette (0-255)
    Ansi256(u8),

    /// RGB color (24-bit)
    Rgb(u8, u8, u8),
}

impl Color {
    /// Create a color from a hex string (e.g., "#ff0000" or "ff0000")
    ///
    /// Returns `Color::Reset` for invalid hex strings. Use `try_hex` for
    /// explicit error handling.
    ///
    /// # Examples
    ///
    /// ```
    /// use rnk::core::Color;
    ///
    /// // Valid hex codes
    /// assert_eq!(Color::hex("#ff0000"), Color::Rgb(255, 0, 0));
    /// assert_eq!(Color::hex("00ff00"), Color::Rgb(0, 255, 0));
    ///
    /// // Invalid hex codes return Reset
    /// assert_eq!(Color::hex("invalid"), Color::Reset);
    /// assert_eq!(Color::hex("#fff"), Color::Reset); // 3-char not supported
    /// ```
    pub fn hex(hex: &str) -> Self {
        Self::try_hex(hex).unwrap_or(Color::Reset)
    }

    /// Try to create a color from a hex string, returning `None` on invalid input
    ///
    /// # Examples
    ///
    /// ```
    /// use rnk::core::Color;
    ///
    /// assert_eq!(Color::try_hex("#ff0000"), Some(Color::Rgb(255, 0, 0)));
    /// assert_eq!(Color::try_hex("invalid"), None);
    /// assert_eq!(Color::try_hex("#gg0000"), None); // invalid hex chars
    /// ```
    pub fn try_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Color::Rgb(r, g, b))
    }

    /// Create an RGB color
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb(r, g, b)
    }

    /// Create a 256-color palette color
    pub fn ansi256(code: u8) -> Self {
        Color::Ansi256(code)
    }
}

impl From<Color> for CrosstermColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Reset => CrosstermColor::Reset,
            Color::Black => CrosstermColor::Black,
            Color::Red => CrosstermColor::DarkRed,
            Color::Green => CrosstermColor::DarkGreen,
            Color::Yellow => CrosstermColor::DarkYellow,
            Color::Blue => CrosstermColor::DarkBlue,
            Color::Magenta => CrosstermColor::DarkMagenta,
            Color::Cyan => CrosstermColor::DarkCyan,
            Color::White => CrosstermColor::Grey,
            Color::BrightBlack => CrosstermColor::DarkGrey,
            Color::BrightRed => CrosstermColor::Red,
            Color::BrightGreen => CrosstermColor::Green,
            Color::BrightYellow => CrosstermColor::Yellow,
            Color::BrightBlue => CrosstermColor::Blue,
            Color::BrightMagenta => CrosstermColor::Magenta,
            Color::BrightCyan => CrosstermColor::Cyan,
            Color::BrightWhite => CrosstermColor::White,
            Color::Ansi256(code) => CrosstermColor::AnsiValue(code),
            Color::Rgb(r, g, b) => CrosstermColor::Rgb { r, g, b },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_color() {
        assert_eq!(Color::hex("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(Color::hex("00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(Color::hex("#0000ff"), Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_hex_invalid_returns_reset() {
        assert_eq!(Color::hex("invalid"), Color::Reset);
        assert_eq!(Color::hex("#fff"), Color::Reset); // too short
        assert_eq!(Color::hex("#gg0000"), Color::Reset); // invalid hex chars
    }

    #[test]
    fn test_try_hex_valid() {
        assert_eq!(Color::try_hex("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(Color::try_hex("00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(Color::try_hex("#AABBCC"), Some(Color::Rgb(170, 187, 204)));
    }

    #[test]
    fn test_try_hex_invalid() {
        assert_eq!(Color::try_hex("invalid"), None);
        assert_eq!(Color::try_hex("#fff"), None); // too short
        assert_eq!(Color::try_hex("#gg0000"), None); // invalid hex chars
        assert_eq!(Color::try_hex(""), None);
    }

    #[test]
    fn test_rgb_color() {
        assert_eq!(Color::rgb(128, 64, 32), Color::Rgb(128, 64, 32));
    }

    #[test]
    fn test_ansi256_color() {
        assert_eq!(Color::ansi256(196), Color::Ansi256(196));
    }

    #[test]
    fn test_crossterm_conversion() {
        let color = Color::Green;
        let ct_color: CrosstermColor = color.into();
        assert_eq!(ct_color, CrosstermColor::DarkGreen);
    }
}
