//! Color Picker component for selecting colors
//!
//! Provides a color picker UI for terminal applications.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::ColorPicker;
//!
//! fn app() -> Element {
//!     let picker = ColorPicker::new()
//!         .selected(Color::Blue);
//!
//!     picker.into_element()
//! }
//! ```

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};

/// Predefined color palette
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// Colors in the palette
    pub colors: Vec<Color>,
    /// Palette name
    pub name: String,
}

impl ColorPalette {
    /// Create a new palette
    pub fn new(name: impl Into<String>, colors: Vec<Color>) -> Self {
        Self {
            name: name.into(),
            colors,
        }
    }

    /// Basic 16 ANSI colors
    pub fn basic() -> Self {
        Self::new(
            "Basic",
            vec![
                Color::Black,
                Color::Red,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
                Color::White,
                Color::BrightBlack,
                Color::BrightRed,
                Color::BrightGreen,
                Color::BrightYellow,
                Color::BrightBlue,
                Color::BrightMagenta,
                Color::BrightCyan,
                Color::BrightWhite,
            ],
        )
    }

    /// Grayscale palette
    pub fn grayscale() -> Self {
        let colors: Vec<Color> = (0..24)
            .map(|i| {
                let gray = 8 + i * 10;
                Color::Rgb(gray, gray, gray)
            })
            .collect();
        Self::new("Grayscale", colors)
    }

    /// Rainbow palette
    pub fn rainbow() -> Self {
        Self::new(
            "Rainbow",
            vec![
                Color::Rgb(255, 0, 0),   // Red
                Color::Rgb(255, 127, 0), // Orange
                Color::Rgb(255, 255, 0), // Yellow
                Color::Rgb(0, 255, 0),   // Green
                Color::Rgb(0, 255, 255), // Cyan
                Color::Rgb(0, 0, 255),   // Blue
                Color::Rgb(127, 0, 255), // Purple
                Color::Rgb(255, 0, 255), // Magenta
            ],
        )
    }

    /// Pastel palette
    pub fn pastel() -> Self {
        Self::new(
            "Pastel",
            vec![
                Color::Rgb(255, 179, 186), // Pink
                Color::Rgb(255, 223, 186), // Peach
                Color::Rgb(255, 255, 186), // Yellow
                Color::Rgb(186, 255, 201), // Mint
                Color::Rgb(186, 225, 255), // Sky
                Color::Rgb(218, 186, 255), // Lavender
            ],
        )
    }

    /// Material design colors
    pub fn material() -> Self {
        Self::new(
            "Material",
            vec![
                Color::Rgb(244, 67, 54),  // Red
                Color::Rgb(233, 30, 99),  // Pink
                Color::Rgb(156, 39, 176), // Purple
                Color::Rgb(103, 58, 183), // Deep Purple
                Color::Rgb(63, 81, 181),  // Indigo
                Color::Rgb(33, 150, 243), // Blue
                Color::Rgb(3, 169, 244),  // Light Blue
                Color::Rgb(0, 188, 212),  // Cyan
                Color::Rgb(0, 150, 136),  // Teal
                Color::Rgb(76, 175, 80),  // Green
                Color::Rgb(139, 195, 74), // Light Green
                Color::Rgb(205, 220, 57), // Lime
                Color::Rgb(255, 235, 59), // Yellow
                Color::Rgb(255, 193, 7),  // Amber
                Color::Rgb(255, 152, 0),  // Orange
                Color::Rgb(255, 87, 34),  // Deep Orange
            ],
        )
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::basic()
    }
}

/// Color picker state
#[derive(Debug, Clone, Default)]
pub struct ColorPickerState {
    /// Selected color index
    pub selected: usize,
    /// Whether the picker is open
    pub open: bool,
}

impl ColorPickerState {
    /// Create a new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the picker
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close the picker
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Toggle the picker
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Move selection
    pub fn select(&mut self, index: usize) {
        self.selected = index;
    }

    /// Move selection left
    pub fn select_prev(&mut self, max: usize) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if max > 0 {
            self.selected = max - 1;
        }
    }

    /// Move selection right
    pub fn select_next(&mut self, max: usize) {
        if max > 0 && self.selected < max - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }
}

/// Color picker style
#[derive(Debug, Clone)]
pub struct ColorPickerStyle {
    /// Colors per row
    pub colors_per_row: usize,
    /// Show color names
    pub show_names: bool,
    /// Show hex values
    pub show_hex: bool,
    /// Selection indicator
    pub selection_indicator: String,
    /// Border color
    pub border_color: Color,
}

impl Default for ColorPickerStyle {
    fn default() -> Self {
        Self {
            colors_per_row: 8,
            show_names: false,
            show_hex: false,
            selection_indicator: "▼".to_string(),
            border_color: Color::White,
        }
    }
}

impl ColorPickerStyle {
    /// Create a new style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set colors per row
    pub fn colors_per_row(mut self, count: usize) -> Self {
        self.colors_per_row = count;
        self
    }

    /// Show color names
    pub fn show_names(mut self, show: bool) -> Self {
        self.show_names = show;
        self
    }

    /// Show hex values
    pub fn show_hex(mut self, show: bool) -> Self {
        self.show_hex = show;
        self
    }

    /// Compact style
    pub fn compact() -> Self {
        Self::new()
            .colors_per_row(16)
            .show_names(false)
            .show_hex(false)
    }

    /// Detailed style
    pub fn detailed() -> Self {
        Self::new()
            .colors_per_row(4)
            .show_names(true)
            .show_hex(true)
    }
}

/// Color picker component
#[derive(Debug)]
pub struct ColorPicker {
    /// Color palette
    palette: ColorPalette,
    /// Current state
    state: ColorPickerState,
    /// Style
    style: ColorPickerStyle,
    /// Title
    title: Option<String>,
}

impl ColorPicker {
    /// Create a new color picker
    pub fn new() -> Self {
        Self {
            palette: ColorPalette::basic(),
            state: ColorPickerState::new(),
            style: ColorPickerStyle::default(),
            title: None,
        }
    }

    /// Set the palette
    pub fn palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Set the state
    pub fn state(mut self, state: ColorPickerState) -> Self {
        self.state = state;
        self
    }

    /// Set the style
    pub fn style(mut self, style: ColorPickerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set selected color by index
    pub fn selected(mut self, index: usize) -> Self {
        self.state.selected = index;
        self
    }

    /// Get the selected color
    pub fn selected_color(&self) -> Option<Color> {
        self.palette.colors.get(self.state.selected).copied()
    }

    /// Convert color to hex string
    fn color_to_hex(color: &Color) -> String {
        match color {
            Color::Rgb(r, g, b) => format!("#{:02X}{:02X}{:02X}", r, g, b),
            _ => "#??????".to_string(),
        }
    }

    /// Render a color swatch
    fn render_swatch(&self, color: &Color, is_selected: bool) -> String {
        let block = "██";
        let indicator = if is_selected {
            &self.style.selection_indicator
        } else {
            "  "
        };

        format!("{}{}{}{}", color.to_ansi_fg(), block, "\x1b[0m", indicator)
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut container = Box::new().flex_direction(FlexDirection::Column);

        // Title
        if let Some(title) = &self.title {
            container = container.child(Text::new(title).into_element());
        }

        // Color grid
        let colors = &self.palette.colors;
        let per_row = self.style.colors_per_row;

        for (row_idx, chunk) in colors.chunks(per_row).enumerate() {
            let mut row_str = String::new();

            for (col_idx, color) in chunk.iter().enumerate() {
                let idx = row_idx * per_row + col_idx;
                let is_selected = idx == self.state.selected;
                row_str.push_str(&self.render_swatch(color, is_selected));
            }

            container = container.child(Text::new(row_str).into_element());
        }

        // Selected color info
        if self.style.show_hex || self.style.show_names {
            if let Some(color) = self.selected_color() {
                let mut info = String::new();

                if self.style.show_hex {
                    info.push_str(&Self::color_to_hex(&color));
                }

                if !info.is_empty() {
                    container = container.child(Text::new(info).into_element());
                }
            }
        }

        container.into_element()
    }
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_palette_basic() {
        let palette = ColorPalette::basic();
        assert_eq!(palette.colors.len(), 16);
    }

    #[test]
    fn test_color_palette_grayscale() {
        let palette = ColorPalette::grayscale();
        assert_eq!(palette.colors.len(), 24);
    }

    #[test]
    fn test_color_palette_presets() {
        let _ = ColorPalette::rainbow();
        let _ = ColorPalette::pastel();
        let _ = ColorPalette::material();
    }

    #[test]
    fn test_color_picker_state() {
        let mut state = ColorPickerState::new();
        assert!(!state.open);

        state.open();
        assert!(state.open);

        state.close();
        assert!(!state.open);
    }

    #[test]
    fn test_color_picker_state_navigation() {
        let mut state = ColorPickerState::new();
        state.selected = 0;

        state.select_next(5);
        assert_eq!(state.selected, 1);

        state.select_prev(5);
        assert_eq!(state.selected, 0);

        // Wrap around
        state.select_prev(5);
        assert_eq!(state.selected, 4);
    }

    #[test]
    fn test_color_picker_creation() {
        let picker = ColorPicker::new();
        assert_eq!(picker.palette.colors.len(), 16);
    }

    #[test]
    fn test_color_picker_selected_color() {
        let picker = ColorPicker::new().selected(0);
        let color = picker.selected_color();
        assert!(color.is_some());
    }

    #[test]
    fn test_color_picker_style() {
        let style = ColorPickerStyle::new().colors_per_row(4).show_hex(true);

        assert_eq!(style.colors_per_row, 4);
        assert!(style.show_hex);
    }

    #[test]
    fn test_color_picker_into_element() {
        let picker = ColorPicker::new();
        let _ = picker.into_element();
    }

    #[test]
    fn test_color_to_hex() {
        let hex = ColorPicker::color_to_hex(&Color::Rgb(255, 0, 128));
        assert_eq!(hex, "#FF0080");
    }
}
