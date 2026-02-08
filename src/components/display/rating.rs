//! Rating component for star ratings
//!
//! Provides a rating display component.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Rating;
//!
//! fn app() -> Element {
//!     Rating::new(4.5)
//!         .max(5)
//!         .into_element()
//! }
//! ```

use crate::components::Text;
use crate::core::{Color, Element};

/// Rating symbol style
#[derive(Debug, Clone)]
pub struct RatingSymbols {
    /// Filled symbol
    pub filled: String,
    /// Half-filled symbol
    pub half: String,
    /// Empty symbol
    pub empty: String,
}

impl Default for RatingSymbols {
    fn default() -> Self {
        Self {
            filled: "★".to_string(),
            half: "☆".to_string(),
            empty: "☆".to_string(),
        }
    }
}

impl RatingSymbols {
    /// Create new symbols
    pub fn new(
        filled: impl Into<String>,
        half: impl Into<String>,
        empty: impl Into<String>,
    ) -> Self {
        Self {
            filled: filled.into(),
            half: half.into(),
            empty: empty.into(),
        }
    }

    /// Star symbols (default)
    pub fn stars() -> Self {
        Self::default()
    }

    /// Heart symbols
    pub fn hearts() -> Self {
        Self::new("♥", "♡", "♡")
    }

    /// Circle symbols
    pub fn circles() -> Self {
        Self::new("●", "◐", "○")
    }

    /// Square symbols
    pub fn squares() -> Self {
        Self::new("■", "◧", "□")
    }

    /// Emoji stars
    pub fn emoji() -> Self {
        Self::new("⭐", "✨", "☆")
    }

    /// Simple ASCII
    pub fn ascii() -> Self {
        Self::new("*", "+", "-")
    }
}

/// Rating style
#[derive(Debug, Clone)]
pub struct RatingStyle {
    /// Symbols to use
    pub symbols: RatingSymbols,
    /// Filled color
    pub filled_color: Color,
    /// Half color
    pub half_color: Color,
    /// Empty color
    pub empty_color: Color,
    /// Show numeric value
    pub show_value: bool,
    /// Value format (e.g., "{:.1}")
    pub value_format: String,
    /// Spacing between symbols
    pub spacing: usize,
}

impl Default for RatingStyle {
    fn default() -> Self {
        Self {
            symbols: RatingSymbols::default(),
            filled_color: Color::Yellow,
            half_color: Color::Yellow,
            empty_color: Color::BrightBlack,
            show_value: false,
            value_format: "{:.1}".to_string(),
            spacing: 0,
        }
    }
}

impl RatingStyle {
    /// Create a new style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set symbols
    pub fn symbols(mut self, symbols: RatingSymbols) -> Self {
        self.symbols = symbols;
        self
    }

    /// Set filled color
    pub fn filled_color(mut self, color: Color) -> Self {
        self.filled_color = color;
        self
    }

    /// Set empty color
    pub fn empty_color(mut self, color: Color) -> Self {
        self.empty_color = color;
        self
    }

    /// Show numeric value
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Set spacing
    pub fn spacing(mut self, spacing: usize) -> Self {
        self.spacing = spacing;
        self
    }

    /// Colorful style
    pub fn colorful() -> Self {
        Self::new()
            .filled_color(Color::BrightYellow)
            .empty_color(Color::BrightBlack)
    }

    /// Minimal style
    pub fn minimal() -> Self {
        Self::new()
            .symbols(RatingSymbols::ascii())
            .filled_color(Color::White)
            .empty_color(Color::BrightBlack)
    }
}

/// Rating component
#[derive(Debug, Clone)]
pub struct Rating {
    /// Current value
    value: f32,
    /// Maximum value
    max: u8,
    /// Style
    style: RatingStyle,
    /// Label (optional)
    label: Option<String>,
}

impl Rating {
    /// Create a new rating
    pub fn new(value: f32) -> Self {
        Self {
            value,
            max: 5,
            style: RatingStyle::default(),
            label: None,
        }
    }

    /// Set maximum value
    pub fn max(mut self, max: u8) -> Self {
        self.max = max;
        self
    }

    /// Set style
    pub fn style(mut self, style: RatingStyle) -> Self {
        self.style = style;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Get the value
    pub fn get_value(&self) -> f32 {
        self.value
    }

    /// Get the max
    pub fn get_max(&self) -> u8 {
        self.max
    }

    /// Get percentage
    pub fn percentage(&self) -> f32 {
        (self.value / self.max as f32) * 100.0
    }

    /// Render the rating as a string
    pub fn render(&self) -> String {
        let mut result = String::new();

        // Label
        if let Some(label) = &self.label {
            result.push_str(label);
            result.push(' ');
        }

        // Stars
        let spacing = " ".repeat(self.style.spacing);
        let full_stars = self.value.floor() as u8;
        let has_half = (self.value - self.value.floor()) >= 0.5;

        for i in 0..self.max {
            if i > 0 && self.style.spacing > 0 {
                result.push_str(&spacing);
            }

            if i < full_stars {
                result.push_str(&format!(
                    "{}{}{}",
                    self.style.filled_color.to_ansi_fg(),
                    self.style.symbols.filled,
                    "\x1b[0m"
                ));
            } else if i == full_stars && has_half {
                result.push_str(&format!(
                    "{}{}{}",
                    self.style.half_color.to_ansi_fg(),
                    self.style.symbols.half,
                    "\x1b[0m"
                ));
            } else {
                result.push_str(&format!(
                    "{}{}{}",
                    self.style.empty_color.to_ansi_fg(),
                    self.style.symbols.empty,
                    "\x1b[0m"
                ));
            }
        }

        // Numeric value
        if self.style.show_value {
            result.push_str(&format!(" ({:.1}/{})", self.value, self.max));
        }

        result
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        Text::new(self.render()).into_element()
    }
}

impl Default for Rating {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Create a rating
pub fn rating(value: f32) -> Rating {
    Rating::new(value)
}

/// Create a rating with max
pub fn rating_of(value: f32, max: u8) -> Rating {
    Rating::new(value).max(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_creation() {
        let r = Rating::new(4.5);
        assert_eq!(r.value, 4.5);
        assert_eq!(r.max, 5);
    }

    #[test]
    fn test_rating_max() {
        let r = Rating::new(8.0).max(10);
        assert_eq!(r.max, 10);
    }

    #[test]
    fn test_rating_percentage() {
        let r = Rating::new(2.5).max(5);
        assert_eq!(r.percentage(), 50.0);
    }

    #[test]
    fn test_rating_render() {
        let r = Rating::new(3.0).max(5);
        let rendered = r.render();
        assert!(rendered.contains("★"));
    }

    #[test]
    fn test_rating_with_label() {
        let r = Rating::new(4.0).label("Quality:");
        let rendered = r.render();
        assert!(rendered.contains("Quality:"));
    }

    #[test]
    fn test_rating_symbols() {
        let symbols = RatingSymbols::hearts();
        assert_eq!(symbols.filled, "♥");
        assert_eq!(symbols.empty, "♡");
    }

    #[test]
    fn test_rating_style() {
        let style = RatingStyle::new()
            .symbols(RatingSymbols::circles())
            .show_value(true);

        assert!(style.show_value);
        assert_eq!(style.symbols.filled, "●");
    }

    #[test]
    fn test_rating_into_element() {
        let r = Rating::new(4.0);
        let _ = r.into_element();
    }

    #[test]
    fn test_rating_helpers() {
        let r1 = rating(3.5);
        assert_eq!(r1.value, 3.5);

        let r2 = rating_of(7.0, 10);
        assert_eq!(r2.value, 7.0);
        assert_eq!(r2.max, 10);
    }

    #[test]
    fn test_rating_symbols_presets() {
        let _ = RatingSymbols::stars();
        let _ = RatingSymbols::hearts();
        let _ = RatingSymbols::circles();
        let _ = RatingSymbols::squares();
        let _ = RatingSymbols::emoji();
        let _ = RatingSymbols::ascii();
    }

    #[test]
    fn test_rating_style_presets() {
        let _ = RatingStyle::colorful();
        let _ = RatingStyle::minimal();
    }
}
