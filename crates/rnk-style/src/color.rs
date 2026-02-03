//! Color types for terminal styling

/// Terminal color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Default terminal color
    Default,
    /// Black (ANSI 0)
    Black,
    /// Red (ANSI 1)
    Red,
    /// Green (ANSI 2)
    Green,
    /// Yellow (ANSI 3)
    Yellow,
    /// Blue (ANSI 4)
    Blue,
    /// Magenta (ANSI 5)
    Magenta,
    /// Cyan (ANSI 6)
    Cyan,
    /// White (ANSI 7)
    White,
    /// Bright Black (ANSI 8)
    BrightBlack,
    /// Bright Red (ANSI 9)
    BrightRed,
    /// Bright Green (ANSI 10)
    BrightGreen,
    /// Bright Yellow (ANSI 11)
    BrightYellow,
    /// Bright Blue (ANSI 12)
    BrightBlue,
    /// Bright Magenta (ANSI 13)
    BrightMagenta,
    /// Bright Cyan (ANSI 14)
    BrightCyan,
    /// Bright White (ANSI 15)
    BrightWhite,
    /// ANSI 256 color (0-255)
    Ansi256(u8),
    /// RGB color
    Rgb(u8, u8, u8),
}

impl Color {
    /// Create a color from a hex string (e.g., "#ff6b6b" or "ff6b6b")
    pub fn hex(s: &str) -> Self {
        let s = s.trim_start_matches('#');
        if s.len() != 6 {
            return Color::Default;
        }

        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);

        Color::Rgb(r, g, b)
    }

    /// Create an RGB color
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb(r, g, b)
    }

    /// Get the ANSI escape code for foreground color
    pub fn fg_code(&self) -> String {
        match self {
            Color::Default => String::new(),
            Color::Black => "\x1b[30m".to_string(),
            Color::Red => "\x1b[31m".to_string(),
            Color::Green => "\x1b[32m".to_string(),
            Color::Yellow => "\x1b[33m".to_string(),
            Color::Blue => "\x1b[34m".to_string(),
            Color::Magenta => "\x1b[35m".to_string(),
            Color::Cyan => "\x1b[36m".to_string(),
            Color::White => "\x1b[37m".to_string(),
            Color::BrightBlack => "\x1b[90m".to_string(),
            Color::BrightRed => "\x1b[91m".to_string(),
            Color::BrightGreen => "\x1b[92m".to_string(),
            Color::BrightYellow => "\x1b[93m".to_string(),
            Color::BrightBlue => "\x1b[94m".to_string(),
            Color::BrightMagenta => "\x1b[95m".to_string(),
            Color::BrightCyan => "\x1b[96m".to_string(),
            Color::BrightWhite => "\x1b[97m".to_string(),
            Color::Ansi256(n) => format!("\x1b[38;5;{}m", n),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        }
    }

    /// Get the ANSI escape code for background color
    pub fn bg_code(&self) -> String {
        match self {
            Color::Default => String::new(),
            Color::Black => "\x1b[40m".to_string(),
            Color::Red => "\x1b[41m".to_string(),
            Color::Green => "\x1b[42m".to_string(),
            Color::Yellow => "\x1b[43m".to_string(),
            Color::Blue => "\x1b[44m".to_string(),
            Color::Magenta => "\x1b[45m".to_string(),
            Color::Cyan => "\x1b[46m".to_string(),
            Color::White => "\x1b[47m".to_string(),
            Color::BrightBlack => "\x1b[100m".to_string(),
            Color::BrightRed => "\x1b[101m".to_string(),
            Color::BrightGreen => "\x1b[102m".to_string(),
            Color::BrightYellow => "\x1b[103m".to_string(),
            Color::BrightBlue => "\x1b[104m".to_string(),
            Color::BrightMagenta => "\x1b[105m".to_string(),
            Color::BrightCyan => "\x1b[106m".to_string(),
            Color::BrightWhite => "\x1b[107m".to_string(),
            Color::Ansi256(n) => format!("\x1b[48;5;{}m", n),
            Color::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_color() {
        assert_eq!(Color::hex("#ff6b6b"), Color::Rgb(255, 107, 107));
        assert_eq!(Color::hex("2d3436"), Color::Rgb(45, 52, 54));
        assert_eq!(Color::hex("#000000"), Color::Rgb(0, 0, 0));
        assert_eq!(Color::hex("#ffffff"), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_fg_code() {
        assert_eq!(Color::Red.fg_code(), "\x1b[31m");
        assert_eq!(Color::Ansi256(196).fg_code(), "\x1b[38;5;196m");
        assert_eq!(Color::Rgb(255, 0, 0).fg_code(), "\x1b[38;2;255;0;0m");
    }

    #[test]
    fn test_bg_code() {
        assert_eq!(Color::Blue.bg_code(), "\x1b[44m");
        assert_eq!(Color::Ansi256(21).bg_code(), "\x1b[48;5;21m");
        assert_eq!(Color::Rgb(0, 0, 255).bg_code(), "\x1b[48;2;0;0;255m");
    }
}
