//! Shared element tree renderer.
//!
//! This module centralizes recursive element rendering so all call sites
//! (runtime, render_to_string, static content, tests) use one code path.

use crate::components::text::Line;
use crate::core::{Display, Element, Overflow, Style};
use crate::layout::LayoutEngine;
use crate::layout::text_flow::flow_text;
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

    let needs_clip = matches!(
        element.style.overflow_x,
        Overflow::Hidden | Overflow::Scroll
    ) || matches!(
        element.style.overflow_y,
        Overflow::Hidden | Overflow::Scroll
    );

    let clip_x = screen_coord(raw_x + f32::from(content_rect.x));
    let clip_y = screen_coord(raw_y + f32::from(content_rect.y));
    let clip_width = content_rect.width;
    let clip_height = content_rect.height;

    let mut clip_pushed = false;
    if needs_clip && let (Some(clip_x), Some(clip_y)) = (clip_x, clip_y) {
        output.clip(ClipRegion {
            x1: clip_x,
            y1: clip_y,
            x2: clip_x.saturating_add(clip_width),
            y2: clip_y.saturating_add(clip_height),
        });
        clip_pushed = true;
    }

    if let (Some(x), Some(y)) = (x, y) {
        let text_x = x
            .saturating_add(content_rect.x)
            .saturating_add(element.style.padding.left as u16);
        let text_y = y
            .saturating_add(content_rect.y)
            .saturating_add(element.style.padding.top as u16);

        if let Some(spans) = &element.spans {
            render_spans(spans, output, text_x, text_y);
        } else if let Some(text) = &element.text_content {
            // Draw the same rows layout reserved height for. Writing the raw
            // string instead stops at the first hard break or at the right
            // edge, silently dropping everything after it.
            let content_width = content_rect
                .width
                .saturating_sub(element.style.padding.left as u16)
                .saturating_sub(element.style.padding.right as u16);
            let flow = flow_text(text, content_width as usize, element.style.text_wrap);
            for (row_idx, row) in flow.rows().iter().enumerate() {
                let Ok(offset) = u16::try_from(row_idx) else {
                    break;
                };
                let Some(row_y) = text_y.checked_add(offset) else {
                    break;
                };
                output.write(text_x, row_y, row, &element.style);
            }
        }
    }

    let scroll_offset_x = element.scroll_offset_x.unwrap_or(0) as f32;
    let scroll_offset_y = element.scroll_offset_y.unwrap_or(0) as f32;
    let child_offset_x = offset_x + layout.x - scroll_offset_x;
    let child_offset_y = offset_y + layout.y - scroll_offset_y;

    for child in &element.children {
        render_element_tree(child, layout_engine, output, child_offset_x, child_offset_y);
    }

    if clip_pushed {
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

fn render_spans(lines: &[Line], output: &mut Output, start_x: u16, start_y: u16) {
    for (line_idx, line) in lines.iter().enumerate() {
        let y = start_y + line_idx as u16;
        let mut x = start_x;

        for span in &line.spans {
            output.write(x, y, &span.content, &span.style);
            x += span.width() as u16;
        }
    }
}

fn border_char(raw: &str) -> char {
    raw.chars().next().unwrap_or(' ')
}

#[cfg(test)]
mod tests;
