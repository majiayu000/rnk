//! Color types for terminal styling

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for dark background detection
static DARK_BACKGROUND: AtomicBool = AtomicBool::new(true);

/// Set whether the terminal has a dark background
///
/// This affects how `AdaptiveColor` resolves its colors.
pub fn set_dark_background(dark: bool) {
    DARK_BACKGROUND.store(dark, Ordering::SeqCst);
}

/// Check if the terminal has a dark background
pub fn is_dark_background() -> bool {
    DARK_BACKGROUND.load(Ordering::SeqCst)
}

/// Detect terminal background from environment
///
/// Checks common environment variables to determine if the terminal
/// has a dark or light background. Returns `None` if detection fails.
pub fn detect_background() -> Option<bool> {
    // Check COLORFGBG (format: "fg;bg" where bg < 7 is dark)
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg) = colorfgbg.split(';').next_back() {
            if let Ok(bg_num) = bg.parse::<u8>() {
                return Some(bg_num < 7);
            }
        }
    }

    // Check terminal-specific variables
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        // Most modern terminals default to dark
        if term_program.contains("iTerm")
            || term_program.contains("Alacritty")
            || term_program.contains("kitty")
        {
            return Some(true);
        }
    }

    // Default assumption: dark background
    None
}

/// Initialize background detection
///
/// Call this at application startup to auto-detect terminal background.
pub fn init_background_detection() {
    if let Some(dark) = detect_background() {
        set_dark_background(dark);
    }
}

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
    /// use rnk_style_core::Color;
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
    /// use rnk_style_core::Color;
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

    /// Convert to ANSI foreground escape code
    ///
    /// # Examples
    ///
    /// ```
    /// use rnk_style_core::Color;
    ///
    /// assert_eq!(Color::Red.to_ansi_fg(), "\x1b[31m");
    /// assert_eq!(Color::Rgb(255, 0, 0).to_ansi_fg(), "\x1b[38;2;255;0;0m");
    /// ```
    pub fn to_ansi_fg(&self) -> String {
        match self {
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
            Color::Ansi256(code) => format!("\x1b[38;5;{}m", code),
            Color::Reset => "\x1b[0m".to_string(),
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
        }
    }

    /// Convert to ANSI background escape code
    ///
    /// # Examples
    ///
    /// ```
    /// use rnk_style_core::Color;
    ///
    /// assert_eq!(Color::Red.to_ansi_bg(), "\x1b[41m");
    /// assert_eq!(Color::Rgb(255, 0, 0).to_ansi_bg(), "\x1b[48;2;255;0;0m");
    /// ```
    pub fn to_ansi_bg(&self) -> String {
        match self {
            Color::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b),
            Color::Ansi256(code) => format!("\x1b[48;5;{}m", code),
            Color::Reset => "\x1b[0m".to_string(),
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
        }
    }
}

/// Adaptive color that depends on terminal background
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptiveColor {
    /// Color to use on light backgrounds
    pub light: Color,
    /// Color to use on dark backgrounds
    pub dark: Color,
}

impl AdaptiveColor {
    /// Create a new adaptive color
    pub fn new(light: Color, dark: Color) -> Self {
        Self { light, dark }
    }

    /// Create from hex strings
    pub fn from_hex(light: &str, dark: &str) -> Self {
        Self {
            light: Color::hex(light),
            dark: Color::hex(dark),
        }
    }

    /// Resolve to a concrete color based on background
    pub fn resolve(&self) -> Color {
        if is_dark_background() {
            self.dark
        } else {
            self.light
        }
    }

    /// Create a universal adaptive color (same on both backgrounds)
    pub fn universal(color: Color) -> Self {
        Self {
            light: color,
            dark: color,
        }
    }
}

impl Default for AdaptiveColor {
    fn default() -> Self {
        Self {
            light: Color::Black,
            dark: Color::White,
        }
    }
}

impl From<AdaptiveColor> for Color {
    fn from(adaptive: AdaptiveColor) -> Self {
        adaptive.resolve()
    }
}

impl From<Color> for AdaptiveColor {
    fn from(color: Color) -> Self {
        AdaptiveColor::universal(color)
    }
}

/// Convenience adaptive colors
pub mod adaptive_colors {
    use super::{AdaptiveColor, Color};

    pub fn text() -> AdaptiveColor {
        AdaptiveColor::new(Color::Black, Color::White)
    }

    pub fn muted() -> AdaptiveColor {
        AdaptiveColor::new(Color::BrightBlack, Color::BrightBlack)
    }

    pub fn success() -> AdaptiveColor {
        AdaptiveColor::new(Color::Green, Color::BrightGreen)
    }

    pub fn error() -> AdaptiveColor {
        AdaptiveColor::new(Color::Red, Color::BrightRed)
    }

    pub fn warning() -> AdaptiveColor {
        AdaptiveColor::new(Color::Yellow, Color::BrightYellow)
    }

    pub fn info() -> AdaptiveColor {
        AdaptiveColor::new(Color::Blue, Color::BrightBlue)
    }

    pub fn accent() -> AdaptiveColor {
        AdaptiveColor::new(Color::Cyan, Color::BrightCyan)
    }

    pub fn highlight() -> AdaptiveColor {
        AdaptiveColor::new(Color::Magenta, Color::BrightMagenta)
    }
}

#[cfg(test)]
mod tests {
    use super::{AdaptiveColor, Color};

    #[test]
    fn test_hex_color() {
        assert_eq!(Color::hex("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(Color::hex("00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(Color::hex("#0000ff"), Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_hex_invalid() {
        assert_eq!(Color::hex("invalid"), Color::Reset);
        assert_eq!(Color::hex("#fff"), Color::Reset); // too short
        assert_eq!(Color::hex("#gg0000"), Color::Reset); // invalid hex chars
    }

    #[test]
    fn test_try_hex() {
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
    fn test_rgb() {
        assert_eq!(Color::rgb(128, 64, 32), Color::Rgb(128, 64, 32));
    }

    #[test]
    fn test_ansi256() {
        assert_eq!(Color::ansi256(196), Color::Ansi256(196));
    }

    #[test]
    fn test_adaptive() {
        let adaptive = AdaptiveColor::new(Color::Black, Color::White);
        assert_eq!(adaptive.light, Color::Black);
        assert_eq!(adaptive.dark, Color::White);
    }

    #[test]
    fn test_adaptive_from_hex() {
        let adaptive = AdaptiveColor::from_hex("#000000", "#ffffff");
        assert_eq!(adaptive.light, Color::Rgb(0, 0, 0));
        assert_eq!(adaptive.dark, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_adaptive_universal() {
        let adaptive = AdaptiveColor::universal(Color::Cyan);
        assert_eq!(adaptive.light, Color::Cyan);
        assert_eq!(adaptive.dark, Color::Cyan);
    }

    #[test]
    fn test_to_ansi() {
        assert_eq!(Color::Red.to_ansi_fg(), "\x1b[31m");
        assert_eq!(Color::BrightRed.to_ansi_fg(), "\x1b[91m");
        assert_eq!(Color::Ansi256(196).to_ansi_fg(), "\x1b[38;5;196m");
        assert_eq!(Color::Reset.to_ansi_fg(), "\x1b[0m");
    }
}
