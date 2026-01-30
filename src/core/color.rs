//! Color types for terminal styling

use crossterm::style::Color as CrosstermColor;
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
        if let Some(bg) = colorfgbg.split(';').last() {
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

/// Adaptive color that changes based on terminal background
///
/// This allows specifying different colors for light and dark backgrounds,
/// automatically selecting the appropriate one at runtime.
///
/// # Example
///
/// ```
/// use rnk::core::{AdaptiveColor, Color};
///
/// let color = AdaptiveColor::new(Color::Black, Color::White);
/// // On dark background: returns White
/// // On light background: returns Black
/// ```
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

    /// Create an adaptive color from hex strings
    pub fn from_hex(light: &str, dark: &str) -> Self {
        Self {
            light: Color::hex(light),
            dark: Color::hex(dark),
        }
    }

    /// Resolve to the appropriate color based on current background
    pub fn resolve(&self) -> Color {
        if is_dark_background() {
            self.dark
        } else {
            self.light
        }
    }

    /// Create a color that's visible on both backgrounds
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

/// Predefined adaptive color schemes
pub mod adaptive_colors {
    use super::{AdaptiveColor, Color};

    /// Primary text color (black on light, white on dark)
    pub fn text() -> AdaptiveColor {
        AdaptiveColor::new(Color::Black, Color::White)
    }

    /// Secondary/muted text color
    pub fn muted() -> AdaptiveColor {
        AdaptiveColor::new(Color::BrightBlack, Color::BrightBlack)
    }

    /// Success color (green)
    pub fn success() -> AdaptiveColor {
        AdaptiveColor::new(Color::Green, Color::BrightGreen)
    }

    /// Error color (red)
    pub fn error() -> AdaptiveColor {
        AdaptiveColor::new(Color::Red, Color::BrightRed)
    }

    /// Warning color (yellow)
    pub fn warning() -> AdaptiveColor {
        AdaptiveColor::new(Color::Yellow, Color::BrightYellow)
    }

    /// Info color (blue)
    pub fn info() -> AdaptiveColor {
        AdaptiveColor::new(Color::Blue, Color::BrightBlue)
    }

    /// Accent color (cyan)
    pub fn accent() -> AdaptiveColor {
        AdaptiveColor::new(Color::Cyan, Color::BrightCyan)
    }

    /// Highlight color (magenta)
    pub fn highlight() -> AdaptiveColor {
        AdaptiveColor::new(Color::Magenta, Color::BrightMagenta)
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

    #[test]
    fn test_adaptive_color_creation() {
        let adaptive = AdaptiveColor::new(Color::Black, Color::White);
        assert_eq!(adaptive.light, Color::Black);
        assert_eq!(adaptive.dark, Color::White);
    }

    #[test]
    fn test_adaptive_color_from_hex() {
        let adaptive = AdaptiveColor::from_hex("#000000", "#ffffff");
        assert_eq!(adaptive.light, Color::Rgb(0, 0, 0));
        assert_eq!(adaptive.dark, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_adaptive_color_resolve_dark() {
        set_dark_background(true);
        let adaptive = AdaptiveColor::new(Color::Black, Color::White);
        assert_eq!(adaptive.resolve(), Color::White);
    }

    #[test]
    fn test_adaptive_color_resolve_light() {
        set_dark_background(false);
        let adaptive = AdaptiveColor::new(Color::Black, Color::White);
        assert_eq!(adaptive.resolve(), Color::Black);
        // Reset to default
        set_dark_background(true);
    }

    #[test]
    fn test_adaptive_color_universal() {
        let adaptive = AdaptiveColor::universal(Color::Cyan);
        assert_eq!(adaptive.light, Color::Cyan);
        assert_eq!(adaptive.dark, Color::Cyan);
    }

    #[test]
    fn test_adaptive_color_into_color() {
        set_dark_background(true);
        let adaptive = AdaptiveColor::new(Color::Black, Color::White);
        let color: Color = adaptive.into();
        assert_eq!(color, Color::White);
    }

    #[test]
    fn test_color_into_adaptive() {
        let color = Color::Cyan;
        let adaptive: AdaptiveColor = color.into();
        assert_eq!(adaptive.light, Color::Cyan);
        assert_eq!(adaptive.dark, Color::Cyan);
    }

    #[test]
    fn test_is_dark_background() {
        set_dark_background(true);
        assert!(is_dark_background());

        set_dark_background(false);
        assert!(!is_dark_background());

        // Reset to default
        set_dark_background(true);
    }

    #[test]
    fn test_adaptive_colors_presets() {
        let text = adaptive_colors::text();
        assert_eq!(text.light, Color::Black);
        assert_eq!(text.dark, Color::White);

        let success = adaptive_colors::success();
        assert_eq!(success.light, Color::Green);
        assert_eq!(success.dark, Color::BrightGreen);

        let error = adaptive_colors::error();
        assert_eq!(error.light, Color::Red);
        assert_eq!(error.dark, Color::BrightRed);
    }
}
