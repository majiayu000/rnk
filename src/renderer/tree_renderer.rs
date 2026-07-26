//! Shared element tree renderer.
//!
//! This module centralizes recursive element rendering so all call sites
//! (runtime, render_to_string, static content, tests) use one code path.

use crate::core::{Display, Element, Overflow, Style};
use crate::layout::LayoutEngine;
use crate::renderer::Output;
use crate::renderer::TextRenderError;
use crate::renderer::output::ClipRegion;

mod projection;

use projection::{ProjectionError, StagedFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl ContentRect {
    fn from_border(style: &Style, width: u16, height: u16) -> Self {
        let visible = style.border_style.is_visible();
        let left = u16::from(visible && style.border_left);
        let right = u16::from(visible && style.border_right);
        let top = u16::from(visible && style.border_top);
        let bottom = u16::from(visible && style.border_bottom);

        Self {
            x: left.min(width),
            y: top.min(height),
            width: width.saturating_sub(left).saturating_sub(right),
            height: height.saturating_sub(top).saturating_sub(bottom),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClipBounds {
    x1: u16,
    y1: u16,
    x2: u16,
    y2: u16,
}

impl ClipBounds {
    fn from_overflow(
        style: &Style,
        raw_x: f32,
        raw_y: f32,
        content_rect: ContentRect,
    ) -> Option<Self> {
        let clips_x = matches!(style.overflow_x, Overflow::Hidden | Overflow::Scroll);
        let clips_y = matches!(style.overflow_y, Overflow::Hidden | Overflow::Scroll);
        if !clips_x && !clips_y {
            return None;
        }

        let content_x = raw_x + f32::from(content_rect.x);
        let content_y = raw_y + f32::from(content_rect.y);
        Some(Self {
            x1: if clips_x { clip_bound(content_x) } else { 0 },
            y1: if clips_y { clip_bound(content_y) } else { 0 },
            x2: if clips_x {
                clip_bound(content_x + f32::from(content_rect.width))
            } else {
                u16::MAX
            },
            y2: if clips_y {
                clip_bound(content_y + f32::from(content_rect.height))
            } else {
                u16::MAX
            },
        })
    }

    fn intersect(self, other: Self) -> Self {
        let x1 = self.x1.max(other.x1);
        let y1 = self.y1.max(other.y1);
        Self {
            x1,
            y1,
            x2: self.x2.min(other.x2).max(x1),
            y2: self.y2.min(other.y2).max(y1),
        }
    }

    fn into_region(self) -> ClipRegion {
        ClipRegion {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: self.y2,
        }
    }
}

/// Clamp a screen-space clip boundary to the representable output range.
#[inline]
fn clip_bound(v: f32) -> u16 {
    if v <= 0.0 {
        0
    } else if v >= u16::MAX as f32 {
        u16::MAX
    } else {
        v as u16
    }
}

/// Clamp a finite positive extent to u16 range.
#[inline]
fn clamp_extent(v: f32) -> Result<u16, ProjectionError> {
    if !v.is_finite() {
        return Err(ProjectionError::NonFiniteCoordinate);
    }
    if v <= 0.0 {
        Ok(0)
    } else if v >= u16::MAX as f32 {
        Ok(u16::MAX)
    } else {
        Ok(v as u16)
    }
}

/// Render an element tree into the provided output buffer.
#[allow(dead_code)]
pub(crate) fn render_element_tree(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) {
    try_render_element_tree(element, layout_engine, output, offset_x, offset_y)
        .unwrap_or_else(|error| panic!("text render failed: {error}"));
}

pub(crate) fn try_render_element_tree(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) -> Result<(), TextRenderError> {
    projection::try_render_tree(element, layout_engine, output, offset_x, offset_y)
        .map(|_| ())
        .map_err(|error| error.into_text_render_error(element.id))
}

fn render_element_tree_staged(
    element: &Element,
    layout_engine: &LayoutEngine,
    staged: &mut StagedFrame,
    offset_x: f32,
    offset_y: f32,
    inherited_clip: Option<ClipBounds>,
) -> Result<(), ProjectionError> {
    if element.style.display == Display::None {
        return Ok(());
    }

    let layout = layout_engine
        .get_layout(element.id)
        .ok_or(ProjectionError::MissingLayout(element.id))?;

    let raw_x = offset_x + layout.x;
    let raw_y = offset_y + layout.y;
    let x = signed_coord(raw_x)?;
    let y = signed_coord(raw_y)?;
    let width = clamp_extent(layout.width)?;
    let height = clamp_extent(layout.height)?;
    let content_rect = ContentRect::from_border(&element.style, width, height);

    if element.style.background_color.is_some() {
        staged.fill_rect(x, y, width, height, &element.style)?;
    }

    let own_clip = ClipBounds::from_overflow(&element.style, raw_x, raw_y, content_rect);
    let clip_to_push =
        own_clip.map(|clip| inherited_clip.map_or(clip, |ancestor| ancestor.intersect(clip)));
    let effective_clip = clip_to_push.or(inherited_clip);
    if let Some(clip) = clip_to_push {
        staged.clip(clip.into_region());
    }

    if element.spans.is_some() || element.text_content.is_some() {
        let scroll_x = i64::from(
            matches!(element.style.overflow_x, Overflow::Scroll)
                .then_some(element.scroll_offset_x.unwrap_or(0))
                .unwrap_or(0),
        );
        let scroll_y = i64::from(
            matches!(element.style.overflow_y, Overflow::Scroll)
                .then_some(element.scroll_offset_y.unwrap_or(0))
                .unwrap_or(0),
        );
        let padding_left = signed_coord(element.style.padding.left)?;
        let padding_top = signed_coord(element.style.padding.top)?;
        let text_x = x
            .checked_add(i64::from(content_rect.x))
            .and_then(|value| value.checked_add(padding_left))
            .and_then(|value| value.checked_sub(scroll_x))
            .ok_or(ProjectionError::CoordinateOverflow)?;
        let text_y = y
            .checked_add(i64::from(content_rect.y))
            .and_then(|value| value.checked_add(padding_top))
            .and_then(|value| value.checked_sub(scroll_y))
            .ok_or(ProjectionError::CoordinateOverflow)?;
        let flow = layout_engine
            .current_text_flow(element.id)
            .ok_or(ProjectionError::MissingCurrentFlow(element.id))?;
        staged.project_flow(element.id, &flow, text_x, text_y)?;
    }

    let scroll_offset_x = element.scroll_offset_x.unwrap_or(0) as f32;
    let scroll_offset_y = element.scroll_offset_y.unwrap_or(0) as f32;
    let child_offset_x = offset_x + layout.x - scroll_offset_x;
    let child_offset_y = offset_y + layout.y - scroll_offset_y;

    for child in &element.children {
        render_element_tree_staged(
            child,
            layout_engine,
            staged,
            child_offset_x,
            child_offset_y,
            effective_clip,
        )?;
    }

    if clip_to_push.is_some() {
        staged.unclip();
    }

    // Borders are the final paint owner of their enabled cells. This keeps
    // visible overflow available through disabled sides without letting
    // content or descendants overwrite an enabled border.
    if element.style.has_border() {
        render_border_staged(element, staged, x, y, width, height)?;
    }
    Ok(())
}

fn render_border_staged(
    element: &Element,
    staged: &mut StagedFrame,
    x: i64,
    y: i64,
    width: u16,
    height: u16,
) -> Result<(), ProjectionError> {
    if width == 0 || height == 0 {
        return Ok(());
    }

    let (tl, tr, bl, br, h, v) = element.style.border_style.chars();
    let tl = border_char(tl);
    let tr = border_char(tr);
    let bl = border_char(bl);
    let br = border_char(br);
    let h = border_char(h);
    let v = border_char(v);

    let mut style = element.style.clone();
    style.dim = element.style.border_dim;

    let right_x = x
        .checked_add(i64::from(width - 1))
        .ok_or(ProjectionError::CoordinateOverflow)?;
    let bottom_y = y
        .checked_add(i64::from(height - 1))
        .ok_or(ProjectionError::CoordinateOverflow)?;

    if element.style.border_top {
        style.color = element.style.get_border_top_color();
        paint_char(staged, x, y, tl, &style)?;
        if width > 2 {
            for col_offset in 1..(width - 1) {
                let column = x
                    .checked_add(i64::from(col_offset))
                    .ok_or(ProjectionError::CoordinateOverflow)?;
                paint_char(staged, column, y, h, &style)?;
            }
        }
        if width > 1 {
            paint_char(staged, right_x, y, tr, &style)?;
        }
    }

    // Horizontal rows own shared cells. On rows without a horizontal border,
    // the right side writes after the left side when width is one.
    let first_vertical_row = u16::from(element.style.border_top);
    let vertical_end = height.saturating_sub(u16::from(element.style.border_bottom));
    for row_offset in first_vertical_row..vertical_end {
        let row = y
            .checked_add(i64::from(row_offset))
            .ok_or(ProjectionError::CoordinateOverflow)?;
        if element.style.border_left {
            style.color = element.style.get_border_left_color();
            paint_char(staged, x, row, v, &style)?;
        }
        if element.style.border_right {
            style.color = element.style.get_border_right_color();
            paint_char(staged, right_x, row, v, &style)?;
        }
    }

    // Paint bottom last so it deterministically wins when top and bottom
    // occupy the same row (for example, a one-cell-high layout).
    if element.style.border_bottom {
        style.color = element.style.get_border_bottom_color();
        paint_char(staged, x, bottom_y, bl, &style)?;
        if width > 2 {
            for col_offset in 1..(width - 1) {
                let column = x
                    .checked_add(i64::from(col_offset))
                    .ok_or(ProjectionError::CoordinateOverflow)?;
                paint_char(staged, column, bottom_y, h, &style)?;
            }
        }
        if width > 1 {
            paint_char(staged, right_x, bottom_y, br, &style)?;
        }
    }
    Ok(())
}

fn paint_char(
    staged: &mut StagedFrame,
    x: i64,
    y: i64,
    ch: char,
    style: &Style,
) -> Result<(), ProjectionError> {
    let mut buffer = [0; 4];
    staged.paint_grapheme(x, y, ch.encode_utf8(&mut buffer), style)
}

fn signed_coord(value: f32) -> Result<i64, ProjectionError> {
    if !value.is_finite() {
        return Err(ProjectionError::NonFiniteCoordinate);
    }
    if value < i64::MIN as f32 || value > i64::MAX as f32 {
        return Err(ProjectionError::CoordinateOverflow);
    }
    Ok(value as i64)
}

fn border_char(raw: &str) -> char {
    raw.chars().next().unwrap_or(' ')
}

#[cfg(test)]
fn render_border(element: &Element, output: &mut Output, x: u16, y: u16, width: u16, height: u16) {
    if width == 0 || height == 0 {
        return;
    }

    let (tl, tr, bl, br, h, v) = element.style.border_style.chars();
    let (tl, tr, bl, br, h, v) = (
        border_char(tl),
        border_char(tr),
        border_char(bl),
        border_char(br),
        border_char(h),
        border_char(v),
    );
    let mut style = element.style.clone();
    style.dim = element.style.border_dim;
    let right_x = x.saturating_add(width - 1);
    let bottom_y = y.saturating_add(height - 1);

    if element.style.border_top {
        style.color = element.style.get_border_top_color();
        output.write_char(x, y, tl, &style);
        for col_offset in 1..width.saturating_sub(1) {
            output.write_char(x.saturating_add(col_offset), y, h, &style);
        }
        if width > 1 {
            output.write_char(right_x, y, tr, &style);
        }
    }
    for row_offset in u16::from(element.style.border_top)
        ..height.saturating_sub(u16::from(element.style.border_bottom))
    {
        let row = y.saturating_add(row_offset);
        if element.style.border_left {
            style.color = element.style.get_border_left_color();
            output.write_char(x, row, v, &style);
        }
        if element.style.border_right {
            style.color = element.style.get_border_right_color();
            output.write_char(right_x, row, v, &style);
        }
    }
    if element.style.border_bottom {
        style.color = element.style.get_border_bottom_color();
        output.write_char(x, bottom_y, bl, &style);
        for col_offset in 1..width.saturating_sub(1) {
            output.write_char(x.saturating_add(col_offset), bottom_y, h, &style);
        }
        if width > 1 {
            output.write_char(right_x, bottom_y, br, &style);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod typed_error_tests {
    use std::error::Error;

    use super::*;
    use crate::components::{Box, Text};
    use crate::layout::TextFlowError;
    use crate::renderer::{TextCoordinateError, TextProjectionError};

    #[test]
    fn text_flow_error_preserves_source_and_commits_no_partial_output() {
        let mut tree = Box::new()
            .width(10)
            .height(2)
            .child(Text::new("first").into_element())
            .into_element();
        let mut engine = LayoutEngine::new();
        engine.try_compute(&tree, 10, 2).unwrap();
        let missing = Text::new("missing").into_element();
        let missing_id = missing.id;
        tree.add_child(missing);

        let mut output = Output::new(10, 2);
        output.write(0, 0, "stable", &Style::default());
        let before = output.render();
        let failure = try_render_element_tree(&tree, &engine, &mut output, 0.0, 0.0);
        assert!(matches!(
            failure,
            Err(TextRenderError::MissingCurrentFlow { element_id })
                if element_id == missing_id
        ));
        assert_eq!(output.render(), before);
        assert!(!output.render().starts_with("first"));

        let flow_error = TextRenderError::flow(tree.id, TextFlowError::InvalidTabStop);
        assert!(matches!(
            flow_error
                .source()
                .and_then(|source| { source.downcast_ref::<TextFlowError>() }),
            Some(TextFlowError::InvalidTabStop)
        ));

        let coordinate_tree = Element::text("coordinate");
        let mut coordinate_engine = LayoutEngine::new();
        coordinate_engine
            .try_compute(&coordinate_tree, 10, 2)
            .unwrap();
        let mut coordinate_output = Output::new(10, 2);
        coordinate_output.write(0, 0, "stable", &Style::default());
        let coordinate_before = coordinate_output.render();
        assert!(matches!(
            try_render_element_tree(
                &coordinate_tree,
                &coordinate_engine,
                &mut coordinate_output,
                f32::NAN,
                0.0,
            ),
            Err(TextRenderError::Coordinate {
                source: TextCoordinateError::NonFinite,
                ..
            })
        ));
        assert_eq!(coordinate_output.render(), coordinate_before);

        assert!(matches!(
            try_render_element_tree(
                &coordinate_tree,
                &coordinate_engine,
                &mut coordinate_output,
                f32::MAX,
                0.0,
            ),
            Err(TextRenderError::Coordinate {
                source: TextCoordinateError::Overflow,
                ..
            })
        ));
        assert_eq!(coordinate_output.render(), coordinate_before);

        let injected = projection::try_render_tree_with_options(
            &coordinate_tree,
            &coordinate_engine,
            &mut coordinate_output,
            0.0,
            0.0,
            projection::ProjectionOptions {
                fail_after_writes: Some(0),
                validation_rows: None,
            },
        )
        .unwrap_err()
        .into_text_render_error(coordinate_tree.id);
        assert!(matches!(
            injected,
            TextRenderError::Projection {
                source: TextProjectionError::InjectedFailure,
                ..
            }
        ));
        assert_eq!(coordinate_output.render(), coordinate_before);

        let malformed =
            projection::ProjectionError::MalformedProjection("test source map is incomplete")
                .into_text_render_error(coordinate_tree.id);
        assert!(matches!(
            malformed,
            TextRenderError::IncompleteSourceMap { element_id }
                if element_id == coordinate_tree.id
        ));
    }
}
