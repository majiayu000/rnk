//! Box component - Flexbox container

use crate::core::{
    AlignItems, AlignSelf, BorderStyle, Color, Dimension, Display, Edges, Element, ElementType,
    FlexDirection, JustifyContent, Overflow, Position, Style,
};

/// Box component builder
#[derive(Debug, Clone, Default)]
pub struct Box {
    style: Style,
    children: Vec<Element>,
    key: Option<String>,
    scroll_offset_x: Option<u16>,
    scroll_offset_y: Option<u16>,
}

impl Box {
    /// Create a new Box
    pub fn new() -> Self {
        Self {
            style: Style::new(),
            children: Vec::new(),
            key: None,
            scroll_offset_x: None,
            scroll_offset_y: None,
        }
    }

    /// Set key for reconciliation
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    // === Display ===

    /// Set display type
    pub fn display(mut self, display: Display) -> Self {
        self.style.display = display;
        self
    }

    /// Hide this element (display: none)
    pub fn hidden(mut self) -> Self {
        self.style.display = Display::None;
        self
    }

    /// Show this element (display: flex)
    pub fn visible(mut self) -> Self {
        self.style.display = Display::Flex;
        self
    }

    // === Flexbox properties ===

    /// Set flex direction
    pub fn flex_direction(mut self, direction: FlexDirection) -> Self {
        self.style.flex_direction = direction;
        self
    }

    /// Set flex wrap
    pub fn flex_wrap(mut self, wrap: bool) -> Self {
        self.style.flex_wrap = wrap;
        self
    }

    /// Set flex grow
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self
    }

    /// Set flex shrink
    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style.flex_shrink = shrink;
        self
    }

    /// Set flex (shorthand for grow and shrink)
    pub fn flex(mut self, value: f32) -> Self {
        self.style.flex_grow = value;
        self.style.flex_shrink = 1.0;
        self
    }

    /// Set flex basis
    pub fn flex_basis(mut self, basis: impl Into<Dimension>) -> Self {
        self.style.flex_basis = basis.into();
        self
    }

    /// Set align items
    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.style.align_items = align;
        self
    }

    /// Set align self
    pub fn align_self(mut self, align: AlignSelf) -> Self {
        self.style.align_self = align;
        self
    }

    /// Set justify content
    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.style.justify_content = justify;
        self
    }

    // === Spacing ===

    /// Set padding (all sides)
    pub fn padding(mut self, value: impl Into<Edges>) -> Self {
        self.style.padding = value.into();
        self
    }

    /// Set padding top
    pub fn padding_top(mut self, value: f32) -> Self {
        self.style.padding.top = value;
        self
    }

    /// Set padding right
    pub fn padding_right(mut self, value: f32) -> Self {
        self.style.padding.right = value;
        self
    }

    /// Set padding bottom
    pub fn padding_bottom(mut self, value: f32) -> Self {
        self.style.padding.bottom = value;
        self
    }

    /// Set padding left
    pub fn padding_left(mut self, value: f32) -> Self {
        self.style.padding.left = value;
        self
    }

    /// Set horizontal padding (left and right)
    pub fn padding_x(mut self, value: f32) -> Self {
        self.style.padding.left = value;
        self.style.padding.right = value;
        self
    }

    /// Set vertical padding (top and bottom)
    pub fn padding_y(mut self, value: f32) -> Self {
        self.style.padding.top = value;
        self.style.padding.bottom = value;
        self
    }

    /// Set margin (all sides)
    pub fn margin(mut self, value: impl Into<Edges>) -> Self {
        self.style.margin = value.into();
        self
    }

    /// Set margin top
    pub fn margin_top(mut self, value: f32) -> Self {
        self.style.margin.top = value;
        self
    }

    /// Set margin right
    pub fn margin_right(mut self, value: f32) -> Self {
        self.style.margin.right = value;
        self
    }

    /// Set margin bottom
    pub fn margin_bottom(mut self, value: f32) -> Self {
        self.style.margin.bottom = value;
        self
    }

    /// Set margin left
    pub fn margin_left(mut self, value: f32) -> Self {
        self.style.margin.left = value;
        self
    }

    /// Set horizontal margin (left and right)
    pub fn margin_x(mut self, value: f32) -> Self {
        self.style.margin.left = value;
        self.style.margin.right = value;
        self
    }

    /// Set vertical margin (top and bottom)
    pub fn margin_y(mut self, value: f32) -> Self {
        self.style.margin.top = value;
        self.style.margin.bottom = value;
        self
    }

    /// Set gap between children
    pub fn gap(mut self, value: f32) -> Self {
        self.style.gap = value;
        self
    }

    /// Set column gap
    pub fn column_gap(mut self, value: f32) -> Self {
        self.style.column_gap = Some(value);
        self
    }

    /// Set row gap
    pub fn row_gap(mut self, value: f32) -> Self {
        self.style.row_gap = Some(value);
        self
    }

    // === Size ===

    /// Set width
    pub fn width(mut self, value: impl Into<Dimension>) -> Self {
        self.style.width = value.into();
        self
    }

    /// Set height
    pub fn height(mut self, value: impl Into<Dimension>) -> Self {
        self.style.height = value.into();
        self
    }

    /// Set min width
    pub fn min_width(mut self, value: impl Into<Dimension>) -> Self {
        self.style.min_width = value.into();
        self
    }

    /// Set min height
    pub fn min_height(mut self, value: impl Into<Dimension>) -> Self {
        self.style.min_height = value.into();
        self
    }

    /// Set max width
    pub fn max_width(mut self, value: impl Into<Dimension>) -> Self {
        self.style.max_width = value.into();
        self
    }

    /// Set max height
    pub fn max_height(mut self, value: impl Into<Dimension>) -> Self {
        self.style.max_height = value.into();
        self
    }

    // === Border ===

    /// Set border style
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.style.border_style = style;
        self
    }

    /// Set border color (all sides)
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    /// Set top border color
    pub fn border_top_color(mut self, color: Color) -> Self {
        self.style.border_top_color = Some(color);
        self
    }

    /// Set right border color
    pub fn border_right_color(mut self, color: Color) -> Self {
        self.style.border_right_color = Some(color);
        self
    }

    /// Set bottom border color
    pub fn border_bottom_color(mut self, color: Color) -> Self {
        self.style.border_bottom_color = Some(color);
        self
    }

    /// Set left border color
    pub fn border_left_color(mut self, color: Color) -> Self {
        self.style.border_left_color = Some(color);
        self
    }

    /// Set border dim
    pub fn border_dim(mut self, dim: bool) -> Self {
        self.style.border_dim = dim;
        self
    }

    /// Set border on specific sides
    pub fn border(mut self, top: bool, right: bool, bottom: bool, left: bool) -> Self {
        self.style.border_top = top;
        self.style.border_right = right;
        self.style.border_bottom = bottom;
        self.style.border_left = left;
        self
    }

    // === Colors ===

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }

    /// Alias for background
    pub fn bg(self, color: Color) -> Self {
        self.background(color)
    }

    // === Overflow ===

    /// Set overflow behavior
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style.overflow_x = overflow;
        self.style.overflow_y = overflow;
        self
    }

    /// Set horizontal overflow
    pub fn overflow_x(mut self, overflow: Overflow) -> Self {
        self.style.overflow_x = overflow;
        self
    }

    /// Set vertical overflow
    pub fn overflow_y(mut self, overflow: Overflow) -> Self {
        self.style.overflow_y = overflow;
        self
    }

    // === Scroll Offset ===

    /// Set horizontal scroll offset (for scrollable content)
    pub fn scroll_offset_x(mut self, offset: u16) -> Self {
        self.scroll_offset_x = Some(offset);
        self
    }

    /// Set vertical scroll offset (for scrollable content)
    pub fn scroll_offset_y(mut self, offset: u16) -> Self {
        self.scroll_offset_y = Some(offset);
        self
    }

    /// Set both scroll offsets
    pub fn scroll_offset(mut self, x: u16, y: u16) -> Self {
        self.scroll_offset_x = Some(x);
        self.scroll_offset_y = Some(y);
        self
    }

    // === Positioning ===

    /// Set position type
    pub fn position(mut self, position: Position) -> Self {
        self.style.position = position;
        self
    }

    /// Set position to absolute
    pub fn position_absolute(mut self) -> Self {
        self.style.position = Position::Absolute;
        self
    }

    /// Set top position
    pub fn top(mut self, value: f32) -> Self {
        self.style.top = Some(value);
        self
    }

    /// Set right position
    pub fn right(mut self, value: f32) -> Self {
        self.style.right = Some(value);
        self
    }

    /// Set bottom position
    pub fn bottom(mut self, value: f32) -> Self {
        self.style.bottom = Some(value);
        self
    }

    /// Set left position
    pub fn left(mut self, value: f32) -> Self {
        self.style.left = Some(value);
        self
    }

    // === Children ===

    /// Add a child element
    pub fn child(mut self, element: Element) -> Self {
        self.children.push(element);
        self
    }

    /// Add multiple children
    pub fn children(mut self, elements: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(elements);
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut element = Element::new(ElementType::Box);
        element.style = self.style;
        element.key = self.key;
        element.scroll_offset_x = self.scroll_offset_x;
        element.scroll_offset_y = self.scroll_offset_y;
        for child in self.children {
            element.add_child(child);
        }
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_builder() {
        let element = Box::new()
            .padding(1)
            .flex_direction(FlexDirection::Column)
            .into_element();

        assert_eq!(element.style.padding.top, 1.0);
        assert_eq!(element.style.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn test_box_with_children() {
        let element = Box::new()
            .child(Element::text("Hello"))
            .child(Element::text("World"))
            .into_element();

        assert_eq!(element.children.len(), 2);
    }

    #[test]
    fn test_box_border() {
        let element = Box::new()
            .border_style(BorderStyle::Round)
            .border_color(Color::Cyan)
            .into_element();

        assert_eq!(element.style.border_style, BorderStyle::Round);
        assert_eq!(element.style.border_color, Some(Color::Cyan));
    }
}
