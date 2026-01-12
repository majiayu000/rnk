//! Style system for elements

use crate::core::Color;

/// Flex direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl From<FlexDirection> for taffy::FlexDirection {
    fn from(dir: FlexDirection) -> Self {
        match dir {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        }
    }
}

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

impl From<AlignItems> for taffy::AlignItems {
    fn from(align: AlignItems) -> Self {
        match align {
            AlignItems::Stretch => taffy::AlignItems::Stretch,
            AlignItems::FlexStart => taffy::AlignItems::FlexStart,
            AlignItems::FlexEnd => taffy::AlignItems::FlexEnd,
            AlignItems::Center => taffy::AlignItems::Center,
            AlignItems::Baseline => taffy::AlignItems::Baseline,
        }
    }
}

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

impl From<JustifyContent> for taffy::JustifyContent {
    fn from(justify: JustifyContent) -> Self {
        match justify {
            JustifyContent::FlexStart => taffy::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
        }
    }
}

/// Display type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Flex,
    None,
}

impl From<Display> for taffy::Display {
    fn from(display: Display) -> Self {
        match display {
            Display::Flex => taffy::Display::Flex,
            Display::None => taffy::Display::None,
        }
    }
}

/// Position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

impl From<Position> for taffy::Position {
    fn from(pos: Position) -> Self {
        match pos {
            Position::Relative => taffy::Position::Relative,
            Position::Absolute => taffy::Position::Absolute,
        }
    }
}

/// Overflow behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

impl From<Overflow> for taffy::Overflow {
    fn from(overflow: Overflow) -> Self {
        match overflow {
            Overflow::Visible => taffy::Overflow::Visible,
            Overflow::Hidden => taffy::Overflow::Hidden,
            Overflow::Scroll => taffy::Overflow::Scroll,
        }
    }
}

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

impl From<u16> for Dimension {
    fn from(v: u16) -> Self {
        Dimension::Points(v as f32)
    }
}

impl From<i32> for Dimension {
    fn from(v: i32) -> Self {
        Dimension::Points(v as f32)
    }
}

impl From<f32> for Dimension {
    fn from(v: f32) -> Self {
        Dimension::Points(v)
    }
}

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

impl From<f32> for Edges {
    fn from(v: f32) -> Self {
        Edges::all(v)
    }
}

impl From<u16> for Edges {
    fn from(v: u16) -> Self {
        Edges::all(v as f32)
    }
}

impl From<i32> for Edges {
    fn from(v: i32) -> Self {
        Edges::all(v as f32)
    }
}

/// Complete style definition
#[derive(Debug, Clone, PartialEq, Default)]
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

impl Style {
    pub fn new() -> Self {
        Self {
            flex_shrink: 1.0,
            border_top: true,
            border_bottom: true,
            border_left: true,
            border_right: true,
            ..Default::default()
        }
    }

    /// Convert to taffy style
    pub fn to_taffy(&self) -> taffy::Style {
        taffy::Style {
            display: self.display.into(),
            position: self.position.into(),
            inset: taffy::Rect {
                top: self
                    .top
                    .map(taffy::LengthPercentageAuto::Length)
                    .unwrap_or(taffy::LengthPercentageAuto::Auto),
                right: self
                    .right
                    .map(taffy::LengthPercentageAuto::Length)
                    .unwrap_or(taffy::LengthPercentageAuto::Auto),
                bottom: self
                    .bottom
                    .map(taffy::LengthPercentageAuto::Length)
                    .unwrap_or(taffy::LengthPercentageAuto::Auto),
                left: self
                    .left
                    .map(taffy::LengthPercentageAuto::Length)
                    .unwrap_or(taffy::LengthPercentageAuto::Auto),
            },
            flex_direction: self.flex_direction.into(),
            flex_wrap: if self.flex_wrap {
                taffy::FlexWrap::Wrap
            } else {
                taffy::FlexWrap::NoWrap
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.into(),
            align_items: Some(self.align_items.into()),
            align_self: self.align_self.into(),
            justify_content: Some(self.justify_content.into()),
            padding: taffy::Rect {
                top: taffy::LengthPercentage::Length(self.padding.top),
                right: taffy::LengthPercentage::Length(self.padding.right),
                bottom: taffy::LengthPercentage::Length(self.padding.bottom),
                left: taffy::LengthPercentage::Length(self.padding.left),
            },
            margin: taffy::Rect {
                top: taffy::LengthPercentageAuto::Length(self.margin.top),
                right: taffy::LengthPercentageAuto::Length(self.margin.right),
                bottom: taffy::LengthPercentageAuto::Length(self.margin.bottom),
                left: taffy::LengthPercentageAuto::Length(self.margin.left),
            },
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(self.column_gap.unwrap_or(self.gap)),
                height: taffy::LengthPercentage::Length(self.row_gap.unwrap_or(self.gap)),
            },
            size: taffy::Size {
                width: self.width.into(),
                height: self.height.into(),
            },
            min_size: taffy::Size {
                width: self.min_width.into(),
                height: self.min_height.into(),
            },
            max_size: taffy::Size {
                width: self.max_width.into(),
                height: self.max_height.into(),
            },
            border: if self.border_style.is_visible() {
                taffy::Rect {
                    top: taffy::LengthPercentage::Length(if self.border_top { 1.0 } else { 0.0 }),
                    right: taffy::LengthPercentage::Length(if self.border_right {
                        1.0
                    } else {
                        0.0
                    }),
                    bottom: taffy::LengthPercentage::Length(if self.border_bottom {
                        1.0
                    } else {
                        0.0
                    }),
                    left: taffy::LengthPercentage::Length(if self.border_left { 1.0 } else { 0.0 }),
                }
            } else {
                taffy::Rect::zero()
            },
            overflow: taffy::Point {
                x: self.overflow_x.into(),
                y: self.overflow_y.into(),
            },
            ..Default::default()
        }
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
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let style = Style::new();
        assert_eq!(style.flex_direction, FlexDirection::Row);
        assert_eq!(style.flex_shrink, 1.0);
        assert_eq!(style.display, Display::Flex);
    }

    #[test]
    fn test_edges() {
        let edges = Edges::all(5.0);
        assert_eq!(edges.top, 5.0);
        assert_eq!(edges.right, 5.0);
        assert_eq!(edges.bottom, 5.0);
        assert_eq!(edges.left, 5.0);
    }

    #[test]
    fn test_border_chars() {
        let chars = BorderStyle::Single.chars();
        assert_eq!(chars.0, "┌");
        assert_eq!(chars.4, "─");
    }

    #[test]
    fn test_dimension_conversion() {
        let dim: Dimension = 10u16.into();
        assert_eq!(dim, Dimension::Points(10.0));

        let dim: Dimension = 20i32.into();
        assert_eq!(dim, Dimension::Points(20.0));
    }
}
