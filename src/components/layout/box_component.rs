//! Box component - Flexbox container

use crate::core::{
    AlignItems, AlignSelf, BorderStyle, Color, Dimension, Display, Edges, Element, ElementType,
    FlexDirection, JustifyContent, Overflow, Position, Style,
};

/// Generate a Box style setter that directly assigns a value.
macro_rules! style_setter {
    ($(#[doc = $doc:literal])* $fn_name:ident($value_ty:ty) => $field:ident) => {
        $(#[doc = $doc])*
        pub fn $fn_name(mut self, value: $value_ty) -> Self {
            self.style.$field = value;
            self
        }
    };
}

/// Generate a Box style setter that uses `impl Into<T>`.
macro_rules! style_setter_into {
    ($(#[doc = $doc:literal])* $fn_name:ident($value_ty:ty) => $field:ident) => {
        $(#[doc = $doc])*
        pub fn $fn_name(mut self, value: impl Into<$value_ty>) -> Self {
            self.style.$field = value.into();
            self
        }
    };
}

/// Generate a Box style setter that wraps the value in `Some()`.
macro_rules! style_setter_some {
    ($(#[doc = $doc:literal])* $fn_name:ident($value_ty:ty) => $field:ident) => {
        $(#[doc = $doc])*
        pub fn $fn_name(mut self, value: $value_ty) -> Self {
            self.style.$field = Some(value);
            self
        }
    };
}

/// Generate a Box style setter for a sub-field (e.g., `padding.top`).
macro_rules! style_setter_sub {
    ($(#[doc = $doc:literal])* $fn_name:ident($value_ty:ty) => $parent:ident . $field:ident) => {
        $(#[doc = $doc])*
        pub fn $fn_name(mut self, value: $value_ty) -> Self {
            self.style.$parent.$field = value;
            self
        }
    };
}

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

    style_setter!(/// Set display type
        display(Display) => display);

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

    style_setter!(/// Set flex direction
        flex_direction(FlexDirection) => flex_direction);
    style_setter!(/// Set flex wrap
        flex_wrap(bool) => flex_wrap);
    style_setter!(/// Set flex grow
        flex_grow(f32) => flex_grow);
    style_setter!(/// Set flex shrink
        flex_shrink(f32) => flex_shrink);

    /// Set flex (shorthand for grow and shrink)
    pub fn flex(mut self, value: f32) -> Self {
        self.style.flex_grow = value;
        self.style.flex_shrink = 1.0;
        self
    }

    style_setter_into!(/// Set flex basis
        flex_basis(Dimension) => flex_basis);
    style_setter!(/// Set align items
        align_items(AlignItems) => align_items);
    style_setter!(/// Set align self
        align_self(AlignSelf) => align_self);
    style_setter!(/// Set justify content
        justify_content(JustifyContent) => justify_content);

    // === Spacing ===

    style_setter_into!(/// Set padding (all sides)
        padding(Edges) => padding);
    style_setter_sub!(/// Set padding top
        padding_top(f32) => padding.top);
    style_setter_sub!(/// Set padding right
        padding_right(f32) => padding.right);
    style_setter_sub!(/// Set padding bottom
        padding_bottom(f32) => padding.bottom);
    style_setter_sub!(/// Set padding left
        padding_left(f32) => padding.left);

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

    style_setter_into!(/// Set margin (all sides)
        margin(Edges) => margin);
    style_setter_sub!(/// Set margin top
        margin_top(f32) => margin.top);
    style_setter_sub!(/// Set margin right
        margin_right(f32) => margin.right);
    style_setter_sub!(/// Set margin bottom
        margin_bottom(f32) => margin.bottom);
    style_setter_sub!(/// Set margin left
        margin_left(f32) => margin.left);

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

    style_setter!(/// Set gap between children
        gap(f32) => gap);
    style_setter_some!(/// Set column gap
        column_gap(f32) => column_gap);
    style_setter_some!(/// Set row gap
        row_gap(f32) => row_gap);

    // === Size ===

    style_setter_into!(/// Set width
        width(Dimension) => width);
    style_setter_into!(/// Set height
        height(Dimension) => height);
    style_setter_into!(/// Set min width
        min_width(Dimension) => min_width);
    style_setter_into!(/// Set min height
        min_height(Dimension) => min_height);
    style_setter_into!(/// Set max width
        max_width(Dimension) => max_width);
    style_setter_into!(/// Set max height
        max_height(Dimension) => max_height);

    // === Border ===

    style_setter!(/// Set border style
        border_style(BorderStyle) => border_style);
    style_setter_some!(/// Set border color (all sides)
        border_color(Color) => border_color);
    style_setter_some!(/// Set top border color
        border_top_color(Color) => border_top_color);
    style_setter_some!(/// Set right border color
        border_right_color(Color) => border_right_color);
    style_setter_some!(/// Set bottom border color
        border_bottom_color(Color) => border_bottom_color);
    style_setter_some!(/// Set left border color
        border_left_color(Color) => border_left_color);
    style_setter!(/// Set border dim
        border_dim(bool) => border_dim);

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

    style_setter!(/// Set horizontal overflow
        overflow_x(Overflow) => overflow_x);
    style_setter!(/// Set vertical overflow
        overflow_y(Overflow) => overflow_y);

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

    style_setter!(/// Set position type
        position(Position) => position);

    /// Set position to absolute
    pub fn position_absolute(mut self) -> Self {
        self.style.position = Position::Absolute;
        self
    }

    style_setter_some!(/// Set top position
        top(f32) => top);
    style_setter_some!(/// Set right position
        right(f32) => right);
    style_setter_some!(/// Set bottom position
        bottom(f32) => bottom);
    style_setter_some!(/// Set left position
        left(f32) => left);

    // === Children ===

    /// Add a child element
    pub fn child(mut self, element: impl Into<Element>) -> Self {
        self.children.push(element.into());
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
