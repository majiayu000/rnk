//! Shared element tree renderer.
//!
//! This module centralizes recursive element rendering so all call sites
//! (runtime, render_to_string, static content, tests) use one code path.

use crate::core::{Display, Element, ElementType, Overflow, Style};
use crate::layout::{AxisClip, LayoutEngine, PreparedSnapshotFrame};
use crate::renderer::Output;
use crate::renderer::TextRenderError;
use crate::renderer::output::ClipRegion;

mod projection;

pub(crate) use projection::ProjectionError;
use projection::StagedFrame;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

#[cfg(test)]
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
    fn from_snapshot(clip: AxisClip, offset_x: i64, offset_y: i64) -> Self {
        Self {
            x1: clip_bound_i64(i64::from(clip.x().start()) + offset_x),
            y1: clip_bound_i64(i64::from(clip.y().start()) + offset_y),
            x2: clip_bound_i64(i64::from(clip.x().end()) + offset_x),
            y2: clip_bound_i64(i64::from(clip.y().end()) + offset_y),
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

#[inline]
fn clip_bound_i64(value: i64) -> u16 {
    if value <= 0 {
        0
    } else if value >= i64::from(u16::MAX) {
        u16::MAX
    } else {
        match u16::try_from(value) {
            Ok(value) => value,
            Err(_) => unreachable!("clip bound was checked as a terminal coordinate"),
        }
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
    if element.style.display == Display::None || element.element_type == ElementType::VirtualText {
        return Ok(());
    }
    projection::try_render_tree(element, layout_engine, output, offset_x, offset_y)
        .map(|_| ())
        .map_err(|error| error.into_text_render_error(element.id))
}

pub(crate) fn try_render_element_snapshot(
    element: &Element,
    snapshot: &PreparedSnapshotFrame,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) -> Result<(), ProjectionError> {
    projection::try_render_snapshot(element, snapshot, output, offset_x, offset_y).map(|_| ())
}

/// Render one element and its descendants, naming the element that failed.
///
/// Recursion runs through this wrapper so a coordinate failure is labelled by
/// the frame that raised it. A child's own frame attributes first, and
/// [`ProjectionError::attributed_to`] leaves an already-named error alone, so
/// ancestors unwinding past it cannot claim the failure for themselves.
fn render_element_tree_staged(
    element: &Element,
    snapshot: &PreparedSnapshotFrame,
    staged: &mut StagedFrame,
    offset_x: f32,
    offset_y: f32,
) -> Result<(), ProjectionError> {
    let offset_x = signed_coord(offset_x)?;
    let offset_y = signed_coord(offset_y)?;
    render_element_subtree_staged(element, snapshot, staged, offset_x, offset_y)
        .map_err(|error| error.attributed_to(element.id))
}

fn render_element_subtree_staged(
    element: &Element,
    snapshot: &PreparedSnapshotFrame,
    staged: &mut StagedFrame,
    offset_x: i64,
    offset_y: i64,
) -> Result<(), ProjectionError> {
    if element.style.display == Display::None || element.element_type == ElementType::VirtualText {
        return Ok(());
    }

    let node = snapshot
        .node_for_element(element.id)
        .map_err(ProjectionError::Alias)?;
    let bounds = node.border_bounds();
    let x1 = i64::from(bounds.left())
        .checked_add(offset_x)
        .ok_or(ProjectionError::CoordinateOverflow(None))?;
    let y1 = i64::from(bounds.top())
        .checked_add(offset_y)
        .ok_or(ProjectionError::CoordinateOverflow(None))?;
    let x2 = i64::from(bounds.right())
        .checked_add(offset_x)
        .ok_or(ProjectionError::CoordinateOverflow(None))?;
    let y2 = i64::from(bounds.bottom())
        .checked_add(offset_y)
        .ok_or(ProjectionError::CoordinateOverflow(None))?;

    if element.style.background_color.is_some() {
        staged.fill_rect_edges(x1, y1, x2, y2, &element.style)?;
    }

    let clip = ClipBounds::from_snapshot(node.effective_clip(), offset_x, offset_y);
    staged.clip(clip.into_region());

    if element.spans.is_some() || element.text_content.is_some() {
        let text_origin = node.text_origin();
        let scroll = node.scroll_transform();
        let text_scroll_x = if element.style.overflow_x == Overflow::Scroll {
            i64::from(scroll.dx())
        } else {
            0
        };
        let text_scroll_y = if element.style.overflow_y == Overflow::Scroll {
            i64::from(scroll.dy())
        } else {
            0
        };
        let text_x = i64::from(text_origin.x())
            .checked_add(offset_x)
            .and_then(|value| value.checked_add(text_scroll_x))
            .ok_or(ProjectionError::CoordinateOverflow(None))?;
        let text_y = i64::from(text_origin.y())
            .checked_add(offset_y)
            .and_then(|value| value.checked_add(text_scroll_y))
            .ok_or(ProjectionError::CoordinateOverflow(None))?;
        let flow = node
            .text_flow()
            .ok_or(ProjectionError::MissingCurrentFlow(element.id))?;
        staged.project_flow(element.id, flow.flow(), text_x, text_y)?;
    }

    for child in &element.children {
        render_element_subtree_staged(child, snapshot, staged, offset_x, offset_y)
            .map_err(|error| error.attributed_to(child.id))?;
    }

    staged.unclip();

    // Borders are the final paint owner of their enabled cells. This keeps
    // visible overflow available through disabled sides without letting
    // content or descendants overwrite an enabled border.
    if element.style.has_border() {
        render_border_staged(element, staged, x1, y1, x2, y2)?;
    }
    Ok(())
}

fn render_border_staged(
    element: &Element,
    staged: &mut StagedFrame,
    x: i64,
    y: i64,
    x2: i64,
    y2: i64,
) -> Result<(), ProjectionError> {
    if x >= x2 || y >= y2 {
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

    let right_x = x2 - 1;
    let bottom_y = y2 - 1;
    let (visible_x1, visible_y1, visible_x2, visible_y2) = staged.visible_bounds();

    if element.style.border_top {
        style.color = element.style.get_border_top_color();
        paint_char(staged, x, y, tl, &style)?;
        if x2 - x > 2 && y >= visible_y1 && y < visible_y2 {
            for column in (x + 1).max(visible_x1)..right_x.min(visible_x2) {
                paint_char(staged, column, y, h, &style)?;
            }
        }
        if x2 - x > 1 {
            paint_char(staged, right_x, y, tr, &style)?;
        }
    }

    // Horizontal rows own shared cells. On rows without a horizontal border,
    // the right side writes after the left side when width is one.
    let first_vertical_row = y + i64::from(element.style.border_top);
    let vertical_end = y2 - i64::from(element.style.border_bottom);
    for row in first_vertical_row.max(visible_y1)..vertical_end.min(visible_y2) {
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
        if x2 - x > 2 && bottom_y >= visible_y1 && bottom_y < visible_y2 {
            for column in (x + 1).max(visible_x1)..right_x.min(visible_x2) {
                paint_char(staged, column, bottom_y, h, &style)?;
            }
        }
        if x2 - x > 1 {
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

/// Project a screen-space coordinate onto the cell grid it falls in.
///
/// A cell owns the half-open span from its own coordinate to the next, so the
/// containing cell is `floor(value)` on both sides of the origin. Casting
/// instead truncates toward zero, which folds every coordinate in `(-1.0, 0.0)`
/// onto cell `0` and paints off-screen content along the viewport edge.
fn signed_coord(value: f32) -> Result<i64, ProjectionError> {
    if !value.is_finite() {
        return Err(ProjectionError::NonFiniteCoordinate(None));
    }
    let floored = value.floor();
    // `i64::MAX as f32` rounds up to 2^63, so the upper bound is exclusive:
    // a value that reaches it has no i64 representation.
    if floored < i64::MIN as f32 || floored >= i64::MAX as f32 {
        return Err(ProjectionError::CoordinateOverflow(None));
    }
    Ok(floored as i64)
}

/// Add two coordinate operands without collapsing arithmetic overflow into an
/// already-invalid input value. The result remains fractional until
/// [`signed_coord`] applies the containing-cell floor.
#[inline]
#[cfg(test)]
fn checked_coordinate_add(left: f32, right: f32) -> Result<f32, ProjectionError> {
    if !left.is_finite() || !right.is_finite() {
        return Err(ProjectionError::NonFiniteCoordinate(None));
    }
    let result = left + right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ProjectionError::CoordinateOverflow(None))
    }
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
    fn finite_coordinate_arithmetic_overflow_is_not_non_finite() {
        assert_eq!(
            checked_coordinate_add(f32::MAX, f32::MAX),
            Err(ProjectionError::CoordinateOverflow(None))
        );
        assert_eq!(
            checked_coordinate_add(f32::NAN, 1.0),
            Err(ProjectionError::NonFiniteCoordinate(None))
        );
        assert_eq!(
            signed_coord(checked_coordinate_add(-0.75, 0.5).unwrap()).unwrap(),
            -1
        );
    }

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
        tree.add_child(missing);

        let mut output = Output::new(10, 2);
        output.write(0, 0, "stable", &Style::default());
        let before = output.render();
        let failure = try_render_element_tree(&tree, &engine, &mut output, 0.0, 0.0);
        assert!(matches!(
            failure,
            Err(TextRenderError::Projection {
                element_id,
                source: TextProjectionError::MissingLayout,
            }) if element_id == tree.id
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
