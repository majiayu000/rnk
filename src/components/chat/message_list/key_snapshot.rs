//! Bit-pattern mirror of a measurement key, used for all key equality.
//!
//! A measurement key has to include the styles a message is drawn with, and
//! `Style` holds `f32`. Float `PartialEq` is not reflexive: a key containing
//! NaN would not equal itself, so it would miss its own cache entry on every
//! lookup and the list would remeasure that message forever. Comparing bit
//! patterns instead makes every key equal to itself and lets the key implement
//! `Eq` and `Hash` soundly.
//!
//! A visible consequence: `+0.0` and `-0.0` have different bits and so key
//! different measurements. That costs one extra measurement in a case that does
//! not arise from normal layout, and it buys reflexivity everywhere else.

use super::types::{HorizontalInsets, MessageCompositeMeasureConfig, MessageStructuralSegment};
use crate::components::chat::{MessageId, MessageRevision};
use crate::core::{
    AlignItems, AlignSelf, BorderStyle, Color, Dimension, Display, Edges, FlexDirection,
    JustifyContent, Overflow, Position, Style, TextWrap,
};
use crate::layout::text_flow::{TextFlowCacheIdentity, TextFlowSourceKind, UnicodeWidthPolicy};

/// An `f32` compared by its bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TotalF32Bits(u32);

impl TotalF32Bits {
    fn of(value: f32) -> Self {
        Self(value.to_bits())
    }

    fn of_option(value: Option<f32>) -> Option<Self> {
        value.map(Self::of)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DimensionSnapshot {
    Auto,
    Points(TotalF32Bits),
    Percent(TotalF32Bits),
}

impl DimensionSnapshot {
    fn of(value: Dimension) -> Self {
        match value {
            Dimension::Auto => Self::Auto,
            Dimension::Points(points) => Self::Points(TotalF32Bits::of(points)),
            Dimension::Percent(percent) => Self::Percent(TotalF32Bits::of(percent)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgesSnapshot {
    top: TotalF32Bits,
    right: TotalF32Bits,
    bottom: TotalF32Bits,
    left: TotalF32Bits,
}

impl EdgesSnapshot {
    fn of(value: Edges) -> Self {
        let Edges {
            top,
            right,
            bottom,
            left,
        } = value;
        Self {
            top: TotalF32Bits::of(top),
            right: TotalF32Bits::of(right),
            bottom: TotalF32Bits::of(bottom),
            left: TotalF32Bits::of(left),
        }
    }
}

/// A `Style` mirrored field by field, with every float held as bits.
///
/// The destructuring in `of` is exhaustive on purpose. Adding a field to
/// `Style` breaks this file, which forces a decision about whether the new
/// field changes a message's height, instead of letting it silently drop out
/// of the cache key and leave stale heights on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct StyleSnapshot {
    display: Display,
    position: Position,
    top: Option<TotalF32Bits>,
    right: Option<TotalF32Bits>,
    bottom: Option<TotalF32Bits>,
    left: Option<TotalF32Bits>,
    flex_direction: FlexDirection,
    flex_wrap: bool,
    flex_grow: TotalF32Bits,
    flex_shrink: TotalF32Bits,
    flex_basis: DimensionSnapshot,
    align_items: AlignItems,
    align_self: AlignSelf,
    justify_content: JustifyContent,
    padding: EdgesSnapshot,
    margin: EdgesSnapshot,
    gap: TotalF32Bits,
    row_gap: Option<TotalF32Bits>,
    column_gap: Option<TotalF32Bits>,
    width: DimensionSnapshot,
    height: DimensionSnapshot,
    min_width: DimensionSnapshot,
    min_height: DimensionSnapshot,
    max_width: DimensionSnapshot,
    max_height: DimensionSnapshot,
    border_style: BorderStyle,
    border_color: Option<Color>,
    border_top_color: Option<Color>,
    border_right_color: Option<Color>,
    border_bottom_color: Option<Color>,
    border_left_color: Option<Color>,
    border_dim: bool,
    border_top: bool,
    border_bottom: bool,
    border_left: bool,
    border_right: bool,
    color: Option<Color>,
    background_color: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    dim: bool,
    inverse: bool,
    text_wrap: TextWrap,
    overflow_x: Overflow,
    overflow_y: Overflow,
    is_static: bool,
}

impl StyleSnapshot {
    fn of(style: &Style) -> Self {
        let Style {
            display,
            position,
            top,
            right,
            bottom,
            left,
            flex_direction,
            flex_wrap,
            flex_grow,
            flex_shrink,
            flex_basis,
            align_items,
            align_self,
            justify_content,
            padding,
            margin,
            gap,
            row_gap,
            column_gap,
            width,
            height,
            min_width,
            min_height,
            max_width,
            max_height,
            border_style,
            border_color,
            border_top_color,
            border_right_color,
            border_bottom_color,
            border_left_color,
            border_dim,
            border_top,
            border_bottom,
            border_left,
            border_right,
            color,
            background_color,
            bold,
            italic,
            underline,
            strikethrough,
            dim,
            inverse,
            text_wrap,
            overflow_x,
            overflow_y,
            is_static,
        } = style;

        Self {
            display: *display,
            position: *position,
            top: TotalF32Bits::of_option(*top),
            right: TotalF32Bits::of_option(*right),
            bottom: TotalF32Bits::of_option(*bottom),
            left: TotalF32Bits::of_option(*left),
            flex_direction: *flex_direction,
            flex_wrap: *flex_wrap,
            flex_grow: TotalF32Bits::of(*flex_grow),
            flex_shrink: TotalF32Bits::of(*flex_shrink),
            flex_basis: DimensionSnapshot::of(*flex_basis),
            align_items: *align_items,
            align_self: *align_self,
            justify_content: *justify_content,
            padding: EdgesSnapshot::of(*padding),
            margin: EdgesSnapshot::of(*margin),
            gap: TotalF32Bits::of(*gap),
            row_gap: TotalF32Bits::of_option(*row_gap),
            column_gap: TotalF32Bits::of_option(*column_gap),
            width: DimensionSnapshot::of(*width),
            height: DimensionSnapshot::of(*height),
            min_width: DimensionSnapshot::of(*min_width),
            min_height: DimensionSnapshot::of(*min_height),
            max_width: DimensionSnapshot::of(*max_width),
            max_height: DimensionSnapshot::of(*max_height),
            border_style: *border_style,
            border_color: *border_color,
            border_top_color: *border_top_color,
            border_right_color: *border_right_color,
            border_bottom_color: *border_bottom_color,
            border_left_color: *border_left_color,
            border_dim: *border_dim,
            border_top: *border_top,
            border_bottom: *border_bottom,
            border_left: *border_left,
            border_right: *border_right,
            color: *color,
            background_color: *background_color,
            bold: *bold,
            italic: *italic,
            underline: *underline,
            strikethrough: *strikethrough,
            dim: *dim,
            inverse: *inverse,
            text_wrap: *text_wrap,
            overflow_x: *overflow_x,
            overflow_y: *overflow_y,
            is_static: *is_static,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StyledRangeSnapshot {
    range: core::ops::Range<usize>,
    style: StyleSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextFlowIdentitySnapshot {
    source: String,
    source_kind: TextFlowSourceKind,
    default_style: StyleSnapshot,
    styled_ranges: Vec<StyledRangeSnapshot>,
    max_width: usize,
    text_wrap: TextWrap,
    overflow_x: Overflow,
    overflow_y: Overflow,
    tab_stop: usize,
    ellipsis: String,
    width_policy: UnicodeWidthPolicy,
}

impl TextFlowIdentitySnapshot {
    fn of(identity: &TextFlowCacheIdentity) -> Self {
        Self {
            source: identity.input.source.clone(),
            source_kind: identity.input.source_kind,
            default_style: StyleSnapshot::of(&identity.input.default_style),
            styled_ranges: identity
                .input
                .styled_ranges
                .iter()
                .map(|styled| StyledRangeSnapshot {
                    range: styled.range.clone(),
                    style: StyleSnapshot::of(&styled.style),
                })
                .collect(),
            max_width: identity.options.max_width,
            text_wrap: identity.options.text_wrap,
            overflow_x: identity.options.overflow_x,
            overflow_y: identity.options.overflow_y,
            tab_stop: identity.options.tab_stop,
            ellipsis: identity.options.ellipsis.clone(),
            width_policy: identity.options.width_policy,
        }
    }
}

/// The whole measurement key, mirrored for total equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeySnapshot {
    message_id: MessageId,
    content_revision: u64,
    variant: u64,
    expansion: u64,
    text_flows: Vec<TextFlowIdentitySnapshot>,
    outer_width: u16,
    horizontal_insets: HorizontalInsets,
    structural_segments: Vec<MessageStructuralSegment>,
}

/// Hashing uses the subset of the snapshot that is cheap and discriminating:
/// identity, revision, variant, expansion, widths and source text. Styles are
/// deliberately left out — the core style enums are not `Hash`, and a hash only
/// has to agree for equal keys, not to separate unequal ones. Equality still
/// compares the whole snapshot, so a bucket collision cannot return the wrong
/// measurement.
impl std::hash::Hash for KeySnapshot {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.message_id.hash(hasher);
        self.content_revision.hash(hasher);
        self.variant.hash(hasher);
        self.expansion.hash(hasher);
        self.outer_width.hash(hasher);
        self.horizontal_insets.hash(hasher);
        self.structural_segments.hash(hasher);
        for flow in &self.text_flows {
            flow.source.hash(hasher);
            flow.max_width.hash(hasher);
            flow.tab_stop.hash(hasher);
            flow.ellipsis.hash(hasher);
            flow.styled_ranges.len().hash(hasher);
        }
    }
}

impl KeySnapshot {
    pub(super) fn of(
        message_id: MessageId,
        content_revision: MessageRevision,
        variant: u64,
        expansion: u64,
        config: &MessageCompositeMeasureConfig,
    ) -> Self {
        Self {
            message_id,
            content_revision: content_revision.get(),
            variant,
            expansion,
            text_flows: config
                .text_flows()
                .iter()
                .map(TextFlowIdentitySnapshot::of)
                .collect(),
            outer_width: config.shell().outer_width(),
            horizontal_insets: config.shell().horizontal_insets(),
            structural_segments: config.shell().structural_segments().to_vec(),
        }
    }
}
