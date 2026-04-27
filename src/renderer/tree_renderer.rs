//! Shared element tree renderer.
//!
//! This module centralizes recursive element rendering so all call sites
//! (runtime, render_to_string, static content, tests) use one code path.

use crate::components::text::Line;
use crate::core::{Display, Element, Overflow};
use crate::layout::LayoutEngine;
use crate::renderer::Output;
use crate::renderer::output::ClipRegion;

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

    if let (Some(x), Some(y)) = (x, y) {
        if element.style.background_color.is_some() {
            output.fill_rect(x, y, width, height, ' ', &element.style);
        }

        if element.style.has_border() {
            render_border(element, output, x, y, width, height);
        }

        let text_x =
            x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
        let text_y =
            y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;

        if let Some(spans) = &element.spans {
            render_spans(spans, output, text_x, text_y);
        } else if let Some(text) = &element.text_content {
            output.write(text_x, text_y, text, &element.style);
        }
    }

    let needs_clip = matches!(
        element.style.overflow_x,
        Overflow::Hidden | Overflow::Scroll
    ) || matches!(
        element.style.overflow_y,
        Overflow::Hidden | Overflow::Scroll
    );

    let clip_x = screen_coord(raw_x + if element.style.has_border() { 1.0 } else { 0.0 });
    let clip_y = screen_coord(raw_y + if element.style.has_border() { 1.0 } else { 0.0 });
    let clip_width = width.saturating_sub(if element.style.has_border() { 2 } else { 0 });
    let clip_height = height.saturating_sub(if element.style.has_border() { 2 } else { 0 });

    let mut clip_pushed = false;
    if needs_clip && clip_width > 0 && clip_height > 0 {
        if let (Some(clip_x), Some(clip_y)) = (clip_x, clip_y) {
            output.clip(ClipRegion {
                x1: clip_x,
                y1: clip_y,
                x2: clip_x.saturating_add(clip_width),
                y2: clip_y.saturating_add(clip_height),
            });
            clip_pushed = true;
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
}

fn render_border(element: &Element, output: &mut Output, x: u16, y: u16, width: u16, height: u16) {
    let (tl, tr, bl, br, h, v) = element.style.border_style.chars();
    let tl = border_char(tl);
    let tr = border_char(tr);
    let bl = border_char(bl);
    let br = border_char(br);
    let h = border_char(h);
    let v = border_char(v);

    let mut style = element.style.clone();
    style.dim = element.style.border_dim;

    if element.style.border_top && height > 0 && width > 0 {
        style.color = element.style.get_border_top_color();
        output.write_char(x, y, tl, &style);
        if width > 2 {
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, y, h, &style);
            }
        }
        if width > 1 {
            output.write_char(x + width - 1, y, tr, &style);
        }
    }

    if element.style.border_bottom && height > 1 && width > 0 {
        style.color = element.style.get_border_bottom_color();
        let bottom_y = y + height - 1;
        output.write_char(x, bottom_y, bl, &style);
        if width > 2 {
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, bottom_y, h, &style);
            }
        }
        if width > 1 {
            output.write_char(x + width - 1, bottom_y, br, &style);
        }
    }

    if element.style.border_left && height > 1 {
        style.color = element.style.get_border_left_color();
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x, row, v, &style);
        }
    }

    if element.style.border_right && width > 1 && height > 1 {
        style.color = element.style.get_border_right_color();
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x + width - 1, row, v, &style);
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
mod tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::core::Overflow;

    #[test]
    fn scrolled_out_negative_rows_do_not_paint_at_top() {
        let element = Box::new()
            .flex_direction(crate::core::FlexDirection::Column)
            .width(12)
            .height(1)
            .overflow_y(Overflow::Hidden)
            .scroll_offset_y(1)
            .child(
                Box::new()
                    .height(1)
                    .flex_shrink(0.0)
                    .child(Text::new("hiddenxxxxx").into_element())
                    .into_element(),
            )
            .child(
                Box::new()
                    .height(1)
                    .flex_shrink(0.0)
                    .child(Text::new("ok").into_element())
                    .into_element(),
            )
            .into_element();

        let mut engine = LayoutEngine::new();
        engine.compute(&element, 12, 1);

        let mut output = Output::new(12, 1);
        render_element_tree(&element, &engine, &mut output, 0.0, 0.0);

        assert_eq!(output.render(), "ok");
    }
}
