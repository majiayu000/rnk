//! Gradient text support
//!
//! Provides gradient text rendering for terminal UI.

use crate::core::Color;

/// A color gradient for text
#[derive(Debug, Clone)]
pub struct Gradient {
    /// Colors in the gradient
    colors: Vec<Color>,
}

impl Gradient {
    /// Create a new gradient from colors
    pub fn new(colors: Vec<Color>) -> Self {
        Self { colors }
    }

    /// Create a gradient from two colors
    pub fn from_two(start: Color, end: Color) -> Self {
        Self {
            colors: vec![start, end],
        }
    }

    /// Create a rainbow gradient
    pub fn rainbow() -> Self {
        Self {
            colors: vec![
                Color::Red,
                Color::Yellow,
                Color::Green,
                Color::Cyan,
                Color::Blue,
                Color::Magenta,
            ],
        }
    }

    /// Create a warm gradient (red to yellow)
    pub fn warm() -> Self {
        Self {
            colors: vec![
                Color::Rgb(255, 0, 0),
                Color::Rgb(255, 128, 0),
                Color::Rgb(255, 255, 0),
            ],
        }
    }

    /// Create a cool gradient (cyan to blue to purple)
    pub fn cool() -> Self {
        Self {
            colors: vec![
                Color::Rgb(0, 255, 255),
                Color::Rgb(0, 128, 255),
                Color::Rgb(128, 0, 255),
            ],
        }
    }

    /// Create a pastel rainbow gradient
    pub fn pastel() -> Self {
        Self {
            colors: vec![
                Color::Rgb(255, 179, 186), // Pink
                Color::Rgb(255, 223, 186), // Peach
                Color::Rgb(255, 255, 186), // Yellow
                Color::Rgb(186, 255, 201), // Mint
                Color::Rgb(186, 225, 255), // Sky
                Color::Rgb(218, 186, 255), // Lavender
            ],
        }
    }

    /// Create a sunset gradient
    pub fn sunset() -> Self {
        Self {
            colors: vec![
                Color::Rgb(255, 94, 77),  // Coral
                Color::Rgb(255, 154, 0),  // Orange
                Color::Rgb(255, 206, 84), // Gold
            ],
        }
    }

    /// Create an ocean gradient
    pub fn ocean() -> Self {
        Self {
            colors: vec![
                Color::Rgb(0, 105, 148),   // Deep blue
                Color::Rgb(0, 168, 198),   // Teal
                Color::Rgb(127, 219, 255), // Light blue
            ],
        }
    }

    /// Create a forest gradient
    pub fn forest() -> Self {
        Self {
            colors: vec![
                Color::Rgb(34, 139, 34),   // Forest green
                Color::Rgb(50, 205, 50),   // Lime green
                Color::Rgb(144, 238, 144), // Light green
            ],
        }
    }

    /// Get the color at a specific position (0.0 to 1.0)
    pub fn color_at(&self, position: f32) -> Color {
        if self.colors.is_empty() {
            return Color::Reset;
        }
        if self.colors.len() == 1 {
            return self.colors[0];
        }

        let position = position.clamp(0.0, 1.0);
        let segment_count = self.colors.len() - 1;
        let segment_size = 1.0 / segment_count as f32;

        let segment_index = ((position / segment_size).floor() as usize).min(segment_count - 1);
        let segment_position = (position - segment_index as f32 * segment_size) / segment_size;

        let start_color = &self.colors[segment_index];
        let end_color = &self.colors[segment_index + 1];

        interpolate_color(start_color, end_color, segment_position)
    }

    /// Apply gradient to text, returning a vector of (char, color) pairs
    pub fn apply(&self, text: &str) -> Vec<(char, Color)> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();

        if len == 0 {
            return vec![];
        }

        chars
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let position = if len == 1 {
                    0.0
                } else {
                    i as f32 / (len - 1) as f32
                };
                (c, self.color_at(position))
            })
            .collect()
    }

    /// Apply gradient and return ANSI-colored string
    pub fn render(&self, text: &str) -> String {
        use std::fmt::Write;

        let colored_chars = self.apply(text);
        let mut result = String::new();

        for (c, color) in colored_chars {
            let ansi_color = color.to_ansi_fg();
            // Writing to String never fails, but we handle it properly
            let _ = write!(result, "{}{}", ansi_color, c);
        }

        // Reset at the end
        result.push_str("\x1b[0m");
        result
    }

    /// Get the number of colors in the gradient
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Check if the gradient is empty
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Add a color to the gradient
    pub fn push(mut self, color: Color) -> Self {
        self.colors.push(color);
        self
    }

    /// Reverse the gradient
    pub fn reverse(mut self) -> Self {
        self.colors.reverse();
        self
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Self::rainbow()
    }
}

/// Interpolate between two colors
fn interpolate_color(start: &Color, end: &Color, t: f32) -> Color {
    let (r1, g1, b1) = color_to_rgb(start);
    let (r2, g2, b2) = color_to_rgb(end);

    let r = lerp(r1 as f32, r2 as f32, t) as u8;
    let g = lerp(g1 as f32, g2 as f32, t) as u8;
    let b = lerp(b1 as f32, b2 as f32, t) as u8;

    Color::Rgb(r, g, b)
}

/// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Convert a Color to RGB values
fn color_to_rgb(color: &Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (*r, *g, *b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::White => (229, 229, 229),
        Color::BrightBlack => (127, 127, 127),
        Color::BrightRed => (255, 0, 0),
        Color::BrightGreen => (0, 255, 0),
        Color::BrightYellow => (255, 255, 0),
        Color::BrightBlue => (92, 92, 255),
        Color::BrightMagenta => (255, 0, 255),
        Color::BrightCyan => (0, 255, 255),
        Color::BrightWhite => (255, 255, 255),
        Color::Ansi256(code) => ansi256_to_rgb(*code),
        Color::Reset => (255, 255, 255),
    }
}

/// Convert ANSI 256 color code to RGB
fn ansi256_to_rgb(code: u8) -> (u8, u8, u8) {
    match code {
        0..=15 => {
            // Standard colors
            let standard = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];
            standard[code as usize]
        }
        16..=231 => {
            // 6x6x6 color cube
            let code = code - 16;
            let r = (code / 36) % 6;
            let g = (code / 6) % 6;
            let b = code % 6;
            let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (to_rgb(r), to_rgb(g), to_rgb(b))
        }
        232..=255 => {
            // Grayscale
            let gray = 8 + (code - 232) * 10;
            (gray, gray, gray)
        }
    }
}

/// Apply a rainbow gradient to text
pub fn rainbow(text: &str) -> String {
    Gradient::rainbow().render(text)
}

/// Apply a gradient between two colors to text
pub fn gradient(text: &str, start: Color, end: Color) -> String {
    Gradient::from_two(start, end).render(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_creation() {
        let g = Gradient::new(vec![Color::Red, Color::Blue]);
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn test_gradient_from_two() {
        let g = Gradient::from_two(Color::Red, Color::Blue);
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn test_gradient_rainbow() {
        let g = Gradient::rainbow();
        assert_eq!(g.len(), 6);
    }

    #[test]
    fn test_gradient_presets() {
        assert!(!Gradient::warm().is_empty());
        assert!(!Gradient::cool().is_empty());
        assert!(!Gradient::pastel().is_empty());
        assert!(!Gradient::sunset().is_empty());
        assert!(!Gradient::ocean().is_empty());
        assert!(!Gradient::forest().is_empty());
    }

    #[test]
    fn test_gradient_color_at() {
        let g = Gradient::from_two(Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255));

        let start = g.color_at(0.0);
        assert_eq!(start, Color::Rgb(0, 0, 0));

        let end = g.color_at(1.0);
        assert_eq!(end, Color::Rgb(255, 255, 255));

        let mid = g.color_at(0.5);
        if let Color::Rgb(r, g, b) = mid {
            assert!((r as i32 - 127).abs() <= 1);
            assert!((g as i32 - 127).abs() <= 1);
            assert!((b as i32 - 127).abs() <= 1);
        } else {
            panic!("Expected RGB color");
        }
    }

    #[test]
    fn test_gradient_apply() {
        let g = Gradient::from_two(Color::Red, Color::Blue);
        let result = g.apply("ABC");

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 'A');
        assert_eq!(result[1].0, 'B');
        assert_eq!(result[2].0, 'C');
    }

    #[test]
    fn test_gradient_apply_empty() {
        let g = Gradient::rainbow();
        let result = g.apply("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_gradient_apply_single_char() {
        let g = Gradient::rainbow();
        let result = g.apply("X");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_gradient_render() {
        let g = Gradient::from_two(Color::Red, Color::Blue);
        let result = g.render("Hi");

        // Should contain ANSI escape codes
        assert!(result.contains("\x1b["));
        // Should end with reset
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_gradient_push() {
        let g = Gradient::new(vec![Color::Red]).push(Color::Blue);
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn test_gradient_reverse() {
        let g = Gradient::new(vec![Color::Red, Color::Blue]).reverse();
        assert_eq!(g.colors[0], Color::Blue);
        assert_eq!(g.colors[1], Color::Red);
    }

    #[test]
    fn test_rainbow_function() {
        let result = rainbow("Hello");
        assert!(result.contains("\x1b["));
    }

    #[test]
    fn test_gradient_function() {
        let result = gradient("Hello", Color::Red, Color::Blue);
        assert!(result.contains("\x1b["));
    }

    #[test]
    fn test_interpolate_color() {
        let start = Color::Rgb(0, 0, 0);
        let end = Color::Rgb(100, 100, 100);
        let mid = interpolate_color(&start, &end, 0.5);

        if let Color::Rgb(r, g, b) = mid {
            assert_eq!(r, 50);
            assert_eq!(g, 50);
            assert_eq!(b, 50);
        }
    }

    #[test]
    fn test_color_to_rgb() {
        assert_eq!(color_to_rgb(&Color::Black), (0, 0, 0));
        assert_eq!(color_to_rgb(&Color::Rgb(100, 150, 200)), (100, 150, 200));
    }

    #[test]
    fn test_ansi256_to_rgb() {
        // Black
        assert_eq!(ansi256_to_rgb(0), (0, 0, 0));
        // White
        assert_eq!(ansi256_to_rgb(15), (255, 255, 255));
        // Grayscale
        let (r, g, b) = ansi256_to_rgb(240);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn test_empty_gradient() {
        let g = Gradient::new(vec![]);
        assert!(g.is_empty());
        assert_eq!(g.color_at(0.5), Color::Reset);
    }

    #[test]
    fn test_single_color_gradient() {
        let g = Gradient::new(vec![Color::Red]);
        assert_eq!(g.color_at(0.0), Color::Red);
        assert_eq!(g.color_at(0.5), Color::Red);
        assert_eq!(g.color_at(1.0), Color::Red);
    }
}
