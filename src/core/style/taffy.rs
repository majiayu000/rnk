pub use ::taffy::*;

use super::Style;

impl Style {
    /// Convert to taffy style
    pub fn to_taffy(&self) -> ::taffy::Style {
        ::taffy::Style {
            display: self.display.into(),
            position: self.position.into(),
            inset: ::taffy::Rect {
                top: self
                    .top
                    .map(::taffy::LengthPercentageAuto::Length)
                    .unwrap_or(::taffy::LengthPercentageAuto::Auto),
                right: self
                    .right
                    .map(::taffy::LengthPercentageAuto::Length)
                    .unwrap_or(::taffy::LengthPercentageAuto::Auto),
                bottom: self
                    .bottom
                    .map(::taffy::LengthPercentageAuto::Length)
                    .unwrap_or(::taffy::LengthPercentageAuto::Auto),
                left: self
                    .left
                    .map(::taffy::LengthPercentageAuto::Length)
                    .unwrap_or(::taffy::LengthPercentageAuto::Auto),
            },
            flex_direction: self.flex_direction.into(),
            flex_wrap: if self.flex_wrap {
                ::taffy::FlexWrap::Wrap
            } else {
                ::taffy::FlexWrap::NoWrap
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.into(),
            align_items: Some(self.align_items.into()),
            align_self: self.align_self.into(),
            justify_content: Some(self.justify_content.into()),
            padding: ::taffy::Rect {
                top: ::taffy::LengthPercentage::Length(self.padding.top),
                right: ::taffy::LengthPercentage::Length(self.padding.right),
                bottom: ::taffy::LengthPercentage::Length(self.padding.bottom),
                left: ::taffy::LengthPercentage::Length(self.padding.left),
            },
            margin: ::taffy::Rect {
                top: ::taffy::LengthPercentageAuto::Length(self.margin.top),
                right: ::taffy::LengthPercentageAuto::Length(self.margin.right),
                bottom: ::taffy::LengthPercentageAuto::Length(self.margin.bottom),
                left: ::taffy::LengthPercentageAuto::Length(self.margin.left),
            },
            gap: ::taffy::Size {
                width: ::taffy::LengthPercentage::Length(self.column_gap.unwrap_or(self.gap)),
                height: ::taffy::LengthPercentage::Length(self.row_gap.unwrap_or(self.gap)),
            },
            size: ::taffy::Size {
                width: self.width.into(),
                height: self.height.into(),
            },
            min_size: ::taffy::Size {
                width: self.min_width.into(),
                height: self.min_height.into(),
            },
            max_size: ::taffy::Size {
                width: self.max_width.into(),
                height: self.max_height.into(),
            },
            border: if self.border_style.is_visible() {
                ::taffy::Rect {
                    top: ::taffy::LengthPercentage::Length(if self.border_top { 1.0 } else { 0.0 }),
                    right: ::taffy::LengthPercentage::Length(if self.border_right {
                        1.0
                    } else {
                        0.0
                    }),
                    bottom: ::taffy::LengthPercentage::Length(if self.border_bottom {
                        1.0
                    } else {
                        0.0
                    }),
                    left: ::taffy::LengthPercentage::Length(if self.border_left {
                        1.0
                    } else {
                        0.0
                    }),
                }
            } else {
                ::taffy::Rect::zero()
            },
            overflow: ::taffy::Point {
                x: self.overflow_x.into(),
                y: self.overflow_y.into(),
            },
            ..Default::default()
        }
    }
}
