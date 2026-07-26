//! Shared element tree renderer.
//!
//! This module centralizes recursive element rendering so all call sites
//! (runtime, render_to_string, static content, tests) use one code path.

use crate::core::{Display, Element, Overflow, Style};
use crate::layout::LayoutEngine;
use crate::renderer::Output;
use crate::renderer::output::ClipRegion;

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

/// Convert a float screen coordinate to u16.
///
/// Negative coordinates are outside the visible viewport. They must not be
/// clamped to 0, otherwise scrolled-out content is painted at the top edge.
#[inline]
fn screen_coord(v: f32) -> Option<u16> {
    if v < 0.0 {
        None
    } else if v >= u16::MAX as f32 {
        Some(u16::MAX)
    } else {
        Some(v as u16)
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

/// Clamp a positive extent to u16 range.
#[inline]
fn clamp_extent(v: f32) -> u16 {
    if v <= 0.0 {
        0
    } else if v >= u16::MAX as f32 {
        u16::MAX
    } else {
        v as u16
    }
}

/// Render an element tree into the provided output buffer.
pub(crate) fn render_element_tree(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) {
    render_element_tree_with_clip(element, layout_engine, output, offset_x, offset_y, None);
}

fn render_element_tree_with_clip(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
    inherited_clip: Option<ClipBounds>,
) {
    if element.style.display == Display::None {
        return;
    }

    let layout = layout_engine.get_layout(element.id).unwrap_or_default();

    let raw_x = offset_x + layout.x;
    let raw_y = offset_y + layout.y;
    let x = screen_coord(raw_x);
    let y = screen_coord(raw_y);
    let width = clamp_extent(layout.width);
    let height = clamp_extent(layout.height);
    let content_rect = ContentRect::from_border(&element.style, width, height);

    if let (Some(x), Some(y)) = (x, y)
        && element.style.background_color.is_some()
    {
        output.fill_rect(x, y, width, height, ' ', &element.style);
    }

    let own_clip = ClipBounds::from_overflow(&element.style, raw_x, raw_y, content_rect);
    let clip_to_push =
        own_clip.map(|clip| inherited_clip.map_or(clip, |ancestor| ancestor.intersect(clip)));
    let effective_clip = clip_to_push.or(inherited_clip);
    if let Some(clip) = clip_to_push {
        output.clip(clip.into_region());
    }

    if let (Some(x), Some(y)) = (x, y) {
        let text_x = x
            .saturating_add(content_rect.x)
            .saturating_add(element.style.padding.left as u16);
        let text_y = y
            .saturating_add(content_rect.y)
            .saturating_add(element.style.padding.top as u16);

        if element.spans.is_some() || element.text_content.is_some() {
            render_published_text_flow(element, layout_engine, output, text_x, text_y);
        }
    }

    let scroll_offset_x = element.scroll_offset_x.unwrap_or(0) as f32;
    let scroll_offset_y = element.scroll_offset_y.unwrap_or(0) as f32;
    let child_offset_x = offset_x + layout.x - scroll_offset_x;
    let child_offset_y = offset_y + layout.y - scroll_offset_y;

    for child in &element.children {
        render_element_tree_with_clip(
            child,
            layout_engine,
            output,
            child_offset_x,
            child_offset_y,
            effective_clip,
        );
    }

    if clip_to_push.is_some() {
        output.unclip();
    }

    // Borders are the final paint owner of their enabled cells. This keeps
    // visible overflow available through disabled sides without letting
    // content or descendants overwrite an enabled border.
    if let (Some(x), Some(y)) = (x, y)
        && element.style.has_border()
    {
        render_border(element, output, x, y, width, height);
    }
}

fn render_border(element: &Element, output: &mut Output, x: u16, y: u16, width: u16, height: u16) {
    if width == 0 || height == 0 {
        return;
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

    let right_x = x.saturating_add(width - 1);
    let bottom_y = y.saturating_add(height - 1);

    if element.style.border_top {
        style.color = element.style.get_border_top_color();
        output.write_char(x, y, tl, &style);
        if width > 2 {
            for col_offset in 1..(width - 1) {
                output.write_char(x.saturating_add(col_offset), y, h, &style);
            }
        }
        if width > 1 {
            output.write_char(right_x, y, tr, &style);
        }
    }

    // Horizontal rows own shared cells. On rows without a horizontal border,
    // the right side writes after the left side when width is one.
    let first_vertical_row = u16::from(element.style.border_top);
    let vertical_end = height.saturating_sub(u16::from(element.style.border_bottom));
    for row_offset in first_vertical_row..vertical_end {
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

    // Paint bottom last so it deterministically wins when top and bottom
    // occupy the same row (for example, a one-cell-high layout).
    if element.style.border_bottom {
        style.color = element.style.get_border_bottom_color();
        output.write_char(x, bottom_y, bl, &style);
        if width > 2 {
            for col_offset in 1..(width - 1) {
                output.write_char(x.saturating_add(col_offset), bottom_y, h, &style);
            }
        }
        if width > 1 {
            output.write_char(right_x, bottom_y, br, &style);
        }
    }
}

fn render_published_text_flow(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    start_x: u16,
    start_y: u16,
) {
    let flow = layout_engine
        .current_text_flow(element.id)
        .expect("text element must have a published TextFlow before rendering");

    for run in flow.logical_rows().iter().flat_map(|row| &row.runs) {
        let column =
            u16::try_from(run.column).expect("published TextFlow column must fit output geometry");
        let row = u16::try_from(run.row).expect("published TextFlow row must fit output geometry");
        let x = start_x
            .checked_add(column)
            .expect("published TextFlow column must fit screen geometry");
        let y = start_y
            .checked_add(row)
            .expect("published TextFlow row must fit screen geometry");

        output.write(x, y, &run.text, &run.style);
    }
}

fn border_char(raw: &str) -> char {
    raw.chars().next().unwrap_or(' ')
}

#[cfg(test)]
mod tests;
