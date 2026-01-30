//! Layout utility functions
//!
//! Provides helper functions for common layout operations,
//! inspired by Lip Gloss's layout functions.

use crate::components::{Box as TinkBox, Text};
use crate::core::{AlignItems, Element, FlexDirection, JustifyContent};

/// Position for alignment operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    /// Align to the start (left or top)
    Start,
    /// Align to the center
    Center,
    /// Align to the end (right or bottom)
    End,
    /// Custom position (0.0 = start, 0.5 = center, 1.0 = end)
    At(f32),
}

impl Position {
    /// Convert to a float value (0.0 to 1.0)
    pub fn as_f32(&self) -> f32 {
        match self {
            Position::Start => 0.0,
            Position::Center => 0.5,
            Position::End => 1.0,
            Position::At(v) => v.clamp(0.0, 1.0),
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Position::Start
    }
}

impl From<f32> for Position {
    fn from(v: f32) -> Self {
        Position::At(v)
    }
}

/// Join multiple elements horizontally (in a row)
///
/// # Arguments
/// * `align` - Vertical alignment of elements
/// * `elements` - Elements to join
///
/// # Example
///
/// ```ignore
/// use rnk::layout::{join_horizontal, Position};
///
/// let row = join_horizontal(Position::Center, vec![elem1, elem2, elem3]);
/// ```
pub fn join_horizontal(align: Position, elements: Vec<Element>) -> Element {
    let align_items = match align {
        Position::Start => AlignItems::FlexStart,
        Position::Center => AlignItems::Center,
        Position::End => AlignItems::FlexEnd,
        Position::At(v) if v <= 0.33 => AlignItems::FlexStart,
        Position::At(v) if v >= 0.67 => AlignItems::FlexEnd,
        Position::At(_) => AlignItems::Center,
    };

    let mut container = TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .align_items(align_items);

    for elem in elements {
        container = container.child(elem);
    }

    container.into_element()
}

/// Join multiple elements vertically (in a column)
///
/// # Arguments
/// * `align` - Horizontal alignment of elements
/// * `elements` - Elements to join
///
/// # Example
///
/// ```ignore
/// use rnk::layout::{join_vertical, Position};
///
/// let column = join_vertical(Position::Center, vec![elem1, elem2, elem3]);
/// ```
pub fn join_vertical(align: Position, elements: Vec<Element>) -> Element {
    let align_items = match align {
        Position::Start => AlignItems::FlexStart,
        Position::Center => AlignItems::Center,
        Position::End => AlignItems::FlexEnd,
        Position::At(v) if v <= 0.33 => AlignItems::FlexStart,
        Position::At(v) if v >= 0.67 => AlignItems::FlexEnd,
        Position::At(_) => AlignItems::Center,
    };

    let mut container = TinkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(align_items);

    for elem in elements {
        container = container.child(elem);
    }

    container.into_element()
}

/// Place an element horizontally within a given width
///
/// # Arguments
/// * `width` - Total width to place within
/// * `pos` - Horizontal position
/// * `element` - Element to place
///
/// # Example
///
/// ```ignore
/// use rnk::layout::{place_horizontal, Position};
///
/// let centered = place_horizontal(80, Position::Center, my_element);
/// ```
pub fn place_horizontal(width: u16, pos: Position, element: Element) -> Element {
    let justify = match pos {
        Position::Start => JustifyContent::FlexStart,
        Position::Center => JustifyContent::Center,
        Position::End => JustifyContent::FlexEnd,
        Position::At(v) if v <= 0.33 => JustifyContent::FlexStart,
        Position::At(v) if v >= 0.67 => JustifyContent::FlexEnd,
        Position::At(_) => JustifyContent::Center,
    };

    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(justify)
        .width(width)
        .child(element)
        .into_element()
}

/// Place an element vertically within a given height
///
/// # Arguments
/// * `height` - Total height to place within
/// * `pos` - Vertical position
/// * `element` - Element to place
///
/// # Example
///
/// ```ignore
/// use rnk::layout::{place_vertical, Position};
///
/// let centered = place_vertical(24, Position::Center, my_element);
/// ```
pub fn place_vertical(height: u16, pos: Position, element: Element) -> Element {
    let justify = match pos {
        Position::Start => JustifyContent::FlexStart,
        Position::Center => JustifyContent::Center,
        Position::End => JustifyContent::FlexEnd,
        Position::At(v) if v <= 0.33 => JustifyContent::FlexStart,
        Position::At(v) if v >= 0.67 => JustifyContent::FlexEnd,
        Position::At(_) => JustifyContent::Center,
    };

    TinkBox::new()
        .flex_direction(FlexDirection::Column)
        .justify_content(justify)
        .height(height)
        .child(element)
        .into_element()
}

/// Place an element within a given area
///
/// # Arguments
/// * `width` - Total width
/// * `height` - Total height
/// * `h_pos` - Horizontal position
/// * `v_pos` - Vertical position
/// * `element` - Element to place
///
/// # Example
///
/// ```ignore
/// use rnk::layout::{place, Position};
///
/// let centered = place(80, 24, Position::Center, Position::Center, my_element);
/// ```
pub fn place(
    width: u16,
    height: u16,
    h_pos: Position,
    v_pos: Position,
    element: Element,
) -> Element {
    let h_justify = match h_pos {
        Position::Start => JustifyContent::FlexStart,
        Position::Center => JustifyContent::Center,
        Position::End => JustifyContent::FlexEnd,
        Position::At(v) if v <= 0.33 => JustifyContent::FlexStart,
        Position::At(v) if v >= 0.67 => JustifyContent::FlexEnd,
        Position::At(_) => JustifyContent::Center,
    };

    let v_justify = match v_pos {
        Position::Start => JustifyContent::FlexStart,
        Position::Center => JustifyContent::Center,
        Position::End => JustifyContent::FlexEnd,
        Position::At(v) if v <= 0.33 => JustifyContent::FlexStart,
        Position::At(v) if v >= 0.67 => JustifyContent::FlexEnd,
        Position::At(_) => JustifyContent::Center,
    };

    // Create inner container for horizontal positioning
    let inner = TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(h_justify)
        .width(width)
        .child(element)
        .into_element();

    // Create outer container for vertical positioning
    TinkBox::new()
        .flex_direction(FlexDirection::Column)
        .justify_content(v_justify)
        .width(width)
        .height(height)
        .child(inner)
        .into_element()
}

/// Create a horizontal spacer that fills available space
pub fn h_spacer() -> Element {
    TinkBox::new().flex_grow(1.0).into_element()
}

/// Create a vertical spacer that fills available space
pub fn v_spacer() -> Element {
    TinkBox::new()
        .flex_grow(1.0)
        .flex_direction(FlexDirection::Column)
        .into_element()
}

/// Create a fixed-width horizontal gap
pub fn h_gap(width: u16) -> Element {
    TinkBox::new().width(width).into_element()
}

/// Create a fixed-height vertical gap
pub fn v_gap(height: u16) -> Element {
    TinkBox::new().height(height).into_element()
}

/// Center an element horizontally within a given width
pub fn center_horizontal(width: u16, element: Element) -> Element {
    place_horizontal(width, Position::Center, element)
}

/// Center an element vertically within a given height
pub fn center_vertical(height: u16, element: Element) -> Element {
    place_vertical(height, Position::Center, element)
}

/// Center an element both horizontally and vertically
pub fn center(width: u16, height: u16, element: Element) -> Element {
    place(width, height, Position::Center, Position::Center, element)
}

/// Create a row of elements with equal spacing between them
pub fn space_between(elements: Vec<Element>) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(JustifyContent::SpaceBetween)
        .children(elements)
        .into_element()
}

/// Create a row of elements with equal spacing around them
pub fn space_around(elements: Vec<Element>) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(JustifyContent::SpaceAround)
        .children(elements)
        .into_element()
}

/// Create a row of elements with equal spacing (including edges)
pub fn space_evenly(elements: Vec<Element>) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(JustifyContent::SpaceEvenly)
        .children(elements)
        .into_element()
}

/// Pad text to a specific width with alignment
pub fn pad_to_width(text: &str, width: usize, align: Position) -> String {
    let text_width = unicode_width::UnicodeWidthStr::width(text);
    if text_width >= width {
        return text.to_string();
    }

    let padding = width - text_width;
    match align {
        Position::Start => format!("{}{}", text, " ".repeat(padding)),
        Position::End => format!("{}{}", " ".repeat(padding), text),
        Position::Center => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
        Position::At(v) => {
            let left = ((padding as f32) * v) as usize;
            let right = padding - left;
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_as_f32() {
        assert_eq!(Position::Start.as_f32(), 0.0);
        assert_eq!(Position::Center.as_f32(), 0.5);
        assert_eq!(Position::End.as_f32(), 1.0);
        assert_eq!(Position::At(0.25).as_f32(), 0.25);
    }

    #[test]
    fn test_position_from_f32() {
        let pos: Position = 0.75.into();
        assert_eq!(pos, Position::At(0.75));
    }

    #[test]
    fn test_position_clamp() {
        assert_eq!(Position::At(1.5).as_f32(), 1.0);
        assert_eq!(Position::At(-0.5).as_f32(), 0.0);
    }

    #[test]
    fn test_pad_to_width_start() {
        let result = pad_to_width("hello", 10, Position::Start);
        assert_eq!(result, "hello     ");
    }

    #[test]
    fn test_pad_to_width_end() {
        let result = pad_to_width("hello", 10, Position::End);
        assert_eq!(result, "     hello");
    }

    #[test]
    fn test_pad_to_width_center() {
        let result = pad_to_width("hello", 11, Position::Center);
        assert_eq!(result, "   hello   ");
    }

    #[test]
    fn test_pad_to_width_no_padding_needed() {
        let result = pad_to_width("hello", 3, Position::Center);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_join_horizontal() {
        let elem1 = Text::new("A").into_element();
        let elem2 = Text::new("B").into_element();
        let result = join_horizontal(Position::Center, vec![elem1, elem2]);
        assert!(!result.children.is_empty());
    }

    #[test]
    fn test_join_vertical() {
        let elem1 = Text::new("A").into_element();
        let elem2 = Text::new("B").into_element();
        let result = join_vertical(Position::Center, vec![elem1, elem2]);
        assert!(!result.children.is_empty());
    }

    #[test]
    fn test_place_horizontal() {
        let elem = Text::new("Test").into_element();
        let result = place_horizontal(80, Position::Center, elem);
        assert_eq!(result.style.width, crate::core::Dimension::Points(80.0));
    }

    #[test]
    fn test_place_vertical() {
        let elem = Text::new("Test").into_element();
        let result = place_vertical(24, Position::Center, elem);
        assert_eq!(result.style.height, crate::core::Dimension::Points(24.0));
    }

    #[test]
    fn test_place() {
        let elem = Text::new("Test").into_element();
        let result = place(80, 24, Position::Center, Position::Center, elem);
        assert_eq!(result.style.width, crate::core::Dimension::Points(80.0));
        assert_eq!(result.style.height, crate::core::Dimension::Points(24.0));
    }

    #[test]
    fn test_center() {
        let elem = Text::new("Test").into_element();
        let result = center(80, 24, elem);
        assert_eq!(result.style.width, crate::core::Dimension::Points(80.0));
        assert_eq!(result.style.height, crate::core::Dimension::Points(24.0));
    }

    #[test]
    fn test_h_gap() {
        let gap = h_gap(10);
        assert_eq!(gap.style.width, crate::core::Dimension::Points(10.0));
    }

    #[test]
    fn test_v_gap() {
        let gap = v_gap(5);
        assert_eq!(gap.style.height, crate::core::Dimension::Points(5.0));
    }

    #[test]
    fn test_space_between() {
        let elem1 = Text::new("A").into_element();
        let elem2 = Text::new("B").into_element();
        let result = space_between(vec![elem1, elem2]);
        assert_eq!(result.style.justify_content, JustifyContent::SpaceBetween);
    }

    #[test]
    fn test_space_around() {
        let elem1 = Text::new("A").into_element();
        let elem2 = Text::new("B").into_element();
        let result = space_around(vec![elem1, elem2]);
        assert_eq!(result.style.justify_content, JustifyContent::SpaceAround);
    }

    #[test]
    fn test_space_evenly() {
        let elem1 = Text::new("A").into_element();
        let elem2 = Text::new("B").into_element();
        let result = space_evenly(vec![elem1, elem2]);
        assert_eq!(result.style.justify_content, JustifyContent::SpaceEvenly);
    }
}
