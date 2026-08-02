use super::{Dimension, Edges, Style};

impl PartialEq for Style {
    fn eq(&self, other: &Self) -> bool {
        self.semantically_eq(other)
    }
}

impl Style {
    pub(crate) fn semantically_eq(&self, other: &Self) -> bool {
        self.display == other.display
            && self.position == other.position
            && same_optional_float(self.top, other.top)
            && same_optional_float(self.right, other.right)
            && same_optional_float(self.bottom, other.bottom)
            && same_optional_float(self.left, other.left)
            && self.flex_direction == other.flex_direction
            && self.flex_wrap == other.flex_wrap
            && same_float(self.flex_grow, other.flex_grow)
            && same_float(self.flex_shrink, other.flex_shrink)
            && same_dimension(self.flex_basis, other.flex_basis)
            && self.align_items == other.align_items
            && self.align_self == other.align_self
            && self.justify_content == other.justify_content
            && same_edges(self.padding, other.padding)
            && same_edges(self.margin, other.margin)
            && same_float(self.gap, other.gap)
            && same_optional_float(self.row_gap, other.row_gap)
            && same_optional_float(self.column_gap, other.column_gap)
            && same_dimension(self.width, other.width)
            && same_dimension(self.height, other.height)
            && same_dimension(self.min_width, other.min_width)
            && same_dimension(self.min_height, other.min_height)
            && same_dimension(self.max_width, other.max_width)
            && same_dimension(self.max_height, other.max_height)
            && self.border_style == other.border_style
            && self.border_color == other.border_color
            && self.border_top_color == other.border_top_color
            && self.border_right_color == other.border_right_color
            && self.border_bottom_color == other.border_bottom_color
            && self.border_left_color == other.border_left_color
            && self.border_dim == other.border_dim
            && self.border_top == other.border_top
            && self.border_bottom == other.border_bottom
            && self.border_left == other.border_left
            && self.border_right == other.border_right
            && self.color == other.color
            && self.background_color == other.background_color
            && self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.strikethrough == other.strikethrough
            && self.dim == other.dim
            && self.inverse == other.inverse
            && self.text_wrap == other.text_wrap
            && self.overflow_x == other.overflow_x
            && self.overflow_y == other.overflow_y
            && self.is_static == other.is_static
    }
}

fn same_edges(left: Edges, right: Edges) -> bool {
    same_float(left.top, right.top)
        && same_float(left.right, right.right)
        && same_float(left.bottom, right.bottom)
        && same_float(left.left, right.left)
}

fn same_dimension(left: Dimension, right: Dimension) -> bool {
    match (left, right) {
        (Dimension::Auto, Dimension::Auto) => true,
        (Dimension::Points(left), Dimension::Points(right))
        | (Dimension::Percent(left), Dimension::Percent(right)) => same_float(left, right),
        _ => false,
    }
}

fn same_optional_float(left: Option<f32>, right: Option<f32>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => same_float(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn same_float(left: f32, right: f32) -> bool {
    left == right || (left.is_nan() && right.is_nan())
}
