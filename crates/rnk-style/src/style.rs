//! Style definition and builder

use rnk_style_core::{BorderStyle, Color};

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// Style for terminal text rendering
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // Colors
    pub(crate) fg: Color,
    pub(crate) bg: Color,

    // Text attributes
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) dim: bool,
    pub(crate) inverse: bool,

    // Padding (top, right, bottom, left)
    pub(crate) padding_top: u16,
    pub(crate) padding_right: u16,
    pub(crate) padding_bottom: u16,
    pub(crate) padding_left: u16,

    // Margin (top, right, bottom, left)
    pub(crate) margin_top: u16,
    pub(crate) margin_right: u16,
    pub(crate) margin_bottom: u16,
    pub(crate) margin_left: u16,

    // Border
    pub(crate) border_style: BorderStyle,
    pub(crate) border_fg: Color,

    // Layout
    pub(crate) width: Option<u16>,
    pub(crate) height: Option<u16>,
    pub(crate) align: Align,
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    /// Create a new empty style
    pub fn new() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            inverse: false,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
            padding_left: 0,
            margin_top: 0,
            margin_right: 0,
            margin_bottom: 0,
            margin_left: 0,
            border_style: BorderStyle::None,
            border_fg: Color::Reset,
            width: None,
            height: None,
            align: Align::Left,
        }
    }

    // ========== Color Methods ==========

    /// Set foreground color
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Set background color
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    // ========== Text Style Methods ==========

    /// Set bold text
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Set italic text
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Set underline text
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Set strikethrough text
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// Set dim text
    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    /// Set inverse colors
    pub fn inverse(mut self) -> Self {
        self.inverse = true;
        self
    }

    // ========== Padding Methods ==========

    /// Set all padding
    pub fn padding(mut self, vertical: u16, horizontal: u16) -> Self {
        self.padding_top = vertical;
        self.padding_bottom = vertical;
        self.padding_left = horizontal;
        self.padding_right = horizontal;
        self
    }

    /// Set all padding to same value
    pub fn p(mut self, value: u16) -> Self {
        self.padding_top = value;
        self.padding_right = value;
        self.padding_bottom = value;
        self.padding_left = value;
        self
    }

    /// Set horizontal padding
    pub fn px(mut self, value: u16) -> Self {
        self.padding_left = value;
        self.padding_right = value;
        self
    }

    /// Set vertical padding
    pub fn py(mut self, value: u16) -> Self {
        self.padding_top = value;
        self.padding_bottom = value;
        self
    }

    /// Set top padding
    pub fn pt(mut self, value: u16) -> Self {
        self.padding_top = value;
        self
    }

    /// Set right padding
    pub fn pr(mut self, value: u16) -> Self {
        self.padding_right = value;
        self
    }

    /// Set bottom padding
    pub fn pb(mut self, value: u16) -> Self {
        self.padding_bottom = value;
        self
    }

    /// Set left padding
    pub fn pl(mut self, value: u16) -> Self {
        self.padding_left = value;
        self
    }

    // ========== Margin Methods ==========

    /// Set all margin
    pub fn margin(mut self, vertical: u16, horizontal: u16) -> Self {
        self.margin_top = vertical;
        self.margin_bottom = vertical;
        self.margin_left = horizontal;
        self.margin_right = horizontal;
        self
    }

    /// Set all margin to same value
    pub fn m(mut self, value: u16) -> Self {
        self.margin_top = value;
        self.margin_right = value;
        self.margin_bottom = value;
        self.margin_left = value;
        self
    }

    /// Set horizontal margin
    pub fn mx(mut self, value: u16) -> Self {
        self.margin_left = value;
        self.margin_right = value;
        self
    }

    /// Set vertical margin
    pub fn my(mut self, value: u16) -> Self {
        self.margin_top = value;
        self.margin_bottom = value;
        self
    }

    /// Set top margin
    pub fn mt(mut self, value: u16) -> Self {
        self.margin_top = value;
        self
    }

    /// Set right margin
    pub fn mr(mut self, value: u16) -> Self {
        self.margin_right = value;
        self
    }

    /// Set bottom margin
    pub fn mb(mut self, value: u16) -> Self {
        self.margin_bottom = value;
        self
    }

    /// Set left margin
    pub fn ml(mut self, value: u16) -> Self {
        self.margin_left = value;
        self
    }

    // ========== Border Methods ==========

    /// Set border style
    pub fn border(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set border foreground color
    pub fn border_fg(mut self, color: Color) -> Self {
        self.border_fg = color;
        self
    }

    // ========== Layout Methods ==========

    /// Set fixed width
    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    /// Set fixed height
    pub fn height(mut self, h: u16) -> Self {
        self.height = Some(h);
        self
    }

    /// Set text alignment
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Center text
    pub fn center(mut self) -> Self {
        self.align = Align::Center;
        self
    }

    /// Right-align text
    pub fn right(mut self) -> Self {
        self.align = Align::Right;
        self
    }

    // ========== Preset Styles ==========

    /// Create an error style (red, bold)
    pub fn error() -> Self {
        Self::new().fg(Color::Red).bold()
    }

    /// Create a success style (green)
    pub fn success() -> Self {
        Self::new().fg(Color::Green)
    }

    /// Create a warning style (yellow)
    pub fn warning() -> Self {
        Self::new().fg(Color::Yellow)
    }

    /// Create an info style (cyan)
    pub fn info() -> Self {
        Self::new().fg(Color::Cyan)
    }

    /// Create a muted style (dim)
    pub fn muted() -> Self {
        Self::new().dim()
    }

    /// Create a highlight style (inverse)
    pub fn highlight() -> Self {
        Self::new().inverse()
    }

    // ========== Rendering ==========

    /// Render text with this style
    pub fn render(&self, text: &str) -> String {
        crate::render::render(self, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_builder() {
        let style = Style::new()
            .fg(Color::Red)
            .bg(Color::Black)
            .bold()
            .padding(1, 2);

        assert_eq!(style.fg, Color::Red);
        assert_eq!(style.bg, Color::Black);
        assert!(style.bold);
        assert_eq!(style.padding_top, 1);
        assert_eq!(style.padding_left, 2);
    }

    #[test]
    fn test_preset_styles() {
        let error = Style::error();
        assert_eq!(error.fg, Color::Red);
        assert!(error.bold);

        let success = Style::success();
        assert_eq!(success.fg, Color::Green);
    }

    #[test]
    fn test_padding_shortcuts() {
        let style = Style::new().p(2);
        assert_eq!(style.padding_top, 2);
        assert_eq!(style.padding_right, 2);
        assert_eq!(style.padding_bottom, 2);
        assert_eq!(style.padding_left, 2);

        let style = Style::new().px(3).py(1);
        assert_eq!(style.padding_left, 3);
        assert_eq!(style.padding_right, 3);
        assert_eq!(style.padding_top, 1);
        assert_eq!(style.padding_bottom, 1);
    }

    #[test]
    fn test_margin_shortcuts() {
        let style = Style::new().m(2);
        assert_eq!(style.margin_top, 2);
        assert_eq!(style.margin_right, 2);
        assert_eq!(style.margin_bottom, 2);
        assert_eq!(style.margin_left, 2);
    }

    #[test]
    fn test_alignment() {
        let style = Style::new().center();
        assert_eq!(style.align, Align::Center);

        let style = Style::new().right();
        assert_eq!(style.align, Align::Right);
    }
}
