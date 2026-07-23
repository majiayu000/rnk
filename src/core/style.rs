//! Style system for elements

use crate::core::Color;

mod taffy;

/// Generate `impl From<LocalEnum> for taffy::TaffyEnum` for enums with matching variant names.
macro_rules! impl_taffy_from {
    ($local:ident => $taffy:ty { $($variant:ident),+ $(,)? }) => {
        impl From<$local> for $taffy {
            fn from(v: $local) -> Self {
                match v {
                    $( $local::$variant => <$taffy>::$variant, )+
                }
            }
        }
    };
}

/// Flex direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl_taffy_from!(FlexDirection => taffy::FlexDirection {
    Row, Column, RowReverse, ColumnReverse,
});

/// Align items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl_taffy_from!(AlignItems => taffy::AlignItems {
    Stretch, FlexStart, FlexEnd, Center, Baseline,
});

/// Align self
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl From<AlignSelf> for Option<taffy::AlignItems> {
    fn from(align: AlignSelf) -> Self {
        match align {
            AlignSelf::Auto => None,
            AlignSelf::Stretch => Some(taffy::AlignItems::Stretch),
            AlignSelf::FlexStart => Some(taffy::AlignItems::FlexStart),
            AlignSelf::FlexEnd => Some(taffy::AlignItems::FlexEnd),
            AlignSelf::Center => Some(taffy::AlignItems::Center),
            AlignSelf::Baseline => Some(taffy::AlignItems::Baseline),
        }
    }
}

/// Justify content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl_taffy_from!(JustifyContent => taffy::JustifyContent {
    FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly,
});

/// Display type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Flex,
    None,
}

impl_taffy_from!(Display => taffy::Display { Flex, None });

/// Position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

impl_taffy_from!(Position => taffy::Position { Relative, Absolute });

/// Overflow behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

impl_taffy_from!(Overflow => taffy::Overflow { Visible, Hidden, Scroll });

/// Text wrapping behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrap {
    #[default]
    Wrap,
    Truncate,
    TruncateStart,
    TruncateMiddle,
    TruncateEnd,
}

/// Border style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    #[default]
    None,
    Single,
    Double,
    Round,
    Bold,
    SingleDouble,
    DoubleSingle,
    Classic,
}

impl BorderStyle {
    /// Get border characters: (top_left, top_right, bottom_left, bottom_right, horizontal, vertical)
    pub fn chars(
        &self,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        match self {
            BorderStyle::None => (" ", " ", " ", " ", " ", " "),
            BorderStyle::Single => ("┌", "┐", "└", "┘", "─", "│"),
            BorderStyle::Double => ("╔", "╗", "╚", "╝", "═", "║"),
            BorderStyle::Round => ("╭", "╮", "╰", "╯", "─", "│"),
            BorderStyle::Bold => ("┏", "┓", "┗", "┛", "━", "┃"),
            BorderStyle::SingleDouble => ("╓", "╖", "╙", "╜", "─", "║"),
            BorderStyle::DoubleSingle => ("╒", "╕", "╘", "╛", "═", "│"),
            BorderStyle::Classic => ("+", "+", "+", "+", "-", "|"),
        }
    }

    /// Check if border style is visible
    pub fn is_visible(&self) -> bool {
        !matches!(self, BorderStyle::None)
    }
}

/// Dimension type for width/height
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Dimension {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

impl From<Dimension> for taffy::Dimension {
    fn from(dim: Dimension) -> Self {
        match dim {
            Dimension::Auto => taffy::Dimension::Auto,
            Dimension::Points(v) => taffy::Dimension::Length(v),
            Dimension::Percent(v) => taffy::Dimension::Percent(v / 100.0),
        }
    }
}

macro_rules! impl_numeric_from {
    ($target:ident :: $variant:ident, $($num:ty),+ $(,)?) => {
        $(
            impl From<$num> for $target {
                fn from(v: $num) -> Self {
                    $target::$variant(v as f32)
                }
            }
        )+
    };
}

impl_numeric_from!(Dimension::Points, u16, i32, f32);

/// Edge values for padding/margin
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn horizontal(value: f32) -> Self {
        Self {
            top: 0.0,
            right: value,
            bottom: 0.0,
            left: value,
        }
    }

    pub fn vertical(value: f32) -> Self {
        Self {
            top: value,
            right: 0.0,
            bottom: value,
            left: 0.0,
        }
    }
}

macro_rules! impl_edges_from {
    ($($num:ty),+ $(,)?) => {
        $(
            impl From<$num> for Edges {
                fn from(v: $num) -> Self {
                    Edges::all(v as f32)
                }
            }
        )+
    };
}

impl_edges_from!(f32, u16, i32);

/// Complete style definition
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // Display
    pub display: Display,

    // Positioning
    pub position: Position,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,

    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: bool,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub justify_content: JustifyContent,

    // Spacing
    pub padding: Edges,
    pub margin: Edges,
    pub gap: f32,
    pub row_gap: Option<f32>,
    pub column_gap: Option<f32>,

    // Size
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,

    // Border
    pub border_style: BorderStyle,
    pub border_color: Option<Color>,
    pub border_top_color: Option<Color>,
    pub border_right_color: Option<Color>,
    pub border_bottom_color: Option<Color>,
    pub border_left_color: Option<Color>,
    pub border_dim: bool,
    pub border_top: bool,
    pub border_bottom: bool,
    pub border_left: bool,
    pub border_right: bool,

    // Colors
    pub color: Option<Color>,
    pub background_color: Option<Color>,

    // Text styles
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub inverse: bool,
    pub text_wrap: TextWrap,

    // Overflow
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    // Static output marker (internal use)
    #[doc(hidden)]
    pub is_static: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    pub fn new() -> Self {
        Self {
            display: Display::default(),
            position: Position::default(),
            top: None,
            right: None,
            bottom: None,
            left: None,
            flex_direction: FlexDirection::default(),
            flex_wrap: false,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::default(),
            align_items: AlignItems::default(),
            align_self: AlignSelf::default(),
            justify_content: JustifyContent::default(),
            padding: Edges::default(),
            margin: Edges::default(),
            gap: 0.0,
            row_gap: None,
            column_gap: None,
            width: Dimension::default(),
            height: Dimension::default(),
            min_width: Dimension::default(),
            min_height: Dimension::default(),
            max_width: Dimension::default(),
            max_height: Dimension::default(),
            border_style: BorderStyle::default(),
            border_color: None,
            border_top_color: None,
            border_right_color: None,
            border_bottom_color: None,
            border_left_color: None,
            border_dim: false,
            border_top: true,
            border_bottom: true,
            border_left: true,
            border_right: true,
            color: None,
            background_color: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            inverse: false,
            text_wrap: TextWrap::default(),
            overflow_x: Overflow::default(),
            overflow_y: Overflow::default(),
            is_static: false,
        }
    }

    // ========== Color Methods ==========

    /// Set foreground color
    pub fn fg(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set background color
    pub fn bg(mut self, color: Color) -> Self {
        self.background_color = Some(color);
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
    pub fn p(mut self, value: impl Into<f32>) -> Self {
        self.padding = Edges::all(value.into());
        self
    }

    /// Set horizontal padding (left and right)
    pub fn px(mut self, value: impl Into<f32>) -> Self {
        let v = value.into();
        self.padding.left = v;
        self.padding.right = v;
        self
    }

    /// Set vertical padding (top and bottom)
    pub fn py(mut self, value: impl Into<f32>) -> Self {
        let v = value.into();
        self.padding.top = v;
        self.padding.bottom = v;
        self
    }

    /// Set top padding
    pub fn pt(mut self, value: impl Into<f32>) -> Self {
        self.padding.top = value.into();
        self
    }

    /// Set right padding
    pub fn pr(mut self, value: impl Into<f32>) -> Self {
        self.padding.right = value.into();
        self
    }

    /// Set bottom padding
    pub fn pb(mut self, value: impl Into<f32>) -> Self {
        self.padding.bottom = value.into();
        self
    }

    /// Set left padding
    pub fn pl(mut self, value: impl Into<f32>) -> Self {
        self.padding.left = value.into();
        self
    }

    // ========== Margin Methods ==========

    /// Set all margin
    pub fn m(mut self, value: impl Into<f32>) -> Self {
        self.margin = Edges::all(value.into());
        self
    }

    /// Set horizontal margin (left and right)
    pub fn mx(mut self, value: impl Into<f32>) -> Self {
        let v = value.into();
        self.margin.left = v;
        self.margin.right = v;
        self
    }

    /// Set vertical margin (top and bottom)
    pub fn my(mut self, value: impl Into<f32>) -> Self {
        let v = value.into();
        self.margin.top = v;
        self.margin.bottom = v;
        self
    }

    /// Set top margin
    pub fn mt(mut self, value: impl Into<f32>) -> Self {
        self.margin.top = value.into();
        self
    }

    /// Set right margin
    pub fn mr(mut self, value: impl Into<f32>) -> Self {
        self.margin.right = value.into();
        self
    }

    /// Set bottom margin
    pub fn mb(mut self, value: impl Into<f32>) -> Self {
        self.margin.bottom = value.into();
        self
    }

    /// Set left margin
    pub fn ml(mut self, value: impl Into<f32>) -> Self {
        self.margin.left = value.into();
        self
    }

    // ========== Border Methods ==========

    /// Set border style
    pub fn border(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set border color
    pub fn border_fg(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set rounded border (shortcut for BorderStyle::Round)
    pub fn rounded(mut self) -> Self {
        self.border_style = BorderStyle::Round;
        self
    }

    // ========== Size Methods ==========

    /// Set width
    pub fn w(mut self, value: impl Into<Dimension>) -> Self {
        self.width = value.into();
        self
    }

    /// Set height
    pub fn h(mut self, value: impl Into<Dimension>) -> Self {
        self.height = value.into();
        self
    }

    /// Set min width
    pub fn min_w(mut self, value: impl Into<Dimension>) -> Self {
        self.min_width = value.into();
        self
    }

    /// Set min height
    pub fn min_h(mut self, value: impl Into<Dimension>) -> Self {
        self.min_height = value.into();
        self
    }

    /// Set max width
    pub fn max_w(mut self, value: impl Into<Dimension>) -> Self {
        self.max_width = value.into();
        self
    }

    /// Set max height
    pub fn max_h(mut self, value: impl Into<Dimension>) -> Self {
        self.max_height = value.into();
        self
    }

    // ========== Flexbox Methods ==========

    /// Set flex direction
    pub fn direction(mut self, dir: FlexDirection) -> Self {
        self.flex_direction = dir;
        self
    }

    /// Set flex grow
    pub fn grow(mut self, value: f32) -> Self {
        self.flex_grow = value;
        self
    }

    /// Set flex shrink
    pub fn shrink(mut self, value: f32) -> Self {
        self.flex_shrink = value;
        self
    }

    /// Set gap between children
    pub fn gap_size(mut self, value: f32) -> Self {
        self.gap = value;
        self
    }

    /// Set align items
    pub fn align(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    /// Set justify content
    pub fn justify(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    // ========== Style Combination ==========

    /// Merge another style into this one (other takes precedence for set values)
    pub fn merge(mut self, other: &Style) -> Self {
        // Colors
        if other.color.is_some() {
            self.color = other.color;
        }
        if other.background_color.is_some() {
            self.background_color = other.background_color;
        }

        // Text styles (only override if true)
        if other.bold {
            self.bold = true;
        }
        if other.italic {
            self.italic = true;
        }
        if other.underline {
            self.underline = true;
        }
        if other.strikethrough {
            self.strikethrough = true;
        }
        if other.dim {
            self.dim = true;
        }
        if other.inverse {
            self.inverse = true;
        }

        // Border
        if other.border_style != BorderStyle::None {
            self.border_style = other.border_style;
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color;
        }

        self
    }

    // ========== Preset Styles ==========

    /// Create an error style (red foreground)
    pub fn error() -> Self {
        Self::new().fg(Color::Red).bold()
    }

    /// Create a success style (green foreground)
    pub fn success() -> Self {
        Self::new().fg(Color::Green)
    }

    /// Create a warning style (yellow foreground)
    pub fn warning() -> Self {
        Self::new().fg(Color::Yellow)
    }

    /// Create an info style (cyan foreground)
    pub fn info() -> Self {
        Self::new().fg(Color::Cyan)
    }

    /// Create a muted/secondary style (dim text)
    pub fn muted() -> Self {
        Self::new().dim()
    }

    /// Create a highlighted style (inverse colors)
    pub fn highlight() -> Self {
        Self::new().inverse()
    }

    /// Check if element has visible border
    pub fn has_border(&self) -> bool {
        self.border_style.is_visible()
            && (self.border_top || self.border_bottom || self.border_left || self.border_right)
    }

    /// Get effective top border color
    pub fn get_border_top_color(&self) -> Option<Color> {
        self.border_top_color.or(self.border_color)
    }

    /// Get effective right border color
    pub fn get_border_right_color(&self) -> Option<Color> {
        self.border_right_color.or(self.border_color)
    }

    /// Get effective bottom border color
    pub fn get_border_bottom_color(&self) -> Option<Color> {
        self.border_bottom_color.or(self.border_color)
    }

    /// Get effective left border color
    pub fn get_border_left_color(&self) -> Option<Color> {
        self.border_left_color.or(self.border_color)
    }
}

#[cfg(test)]
mod tests;
