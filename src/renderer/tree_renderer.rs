//! Shared element tree renderer.
//!
//! This module centralizes recursive element rendering so all call sites
//! (runtime, render_to_string, static content, tests) use one code path.

use crate::components::text::Line;
use crate::core::{Display, Element, Overflow};
use crate::layout::LayoutEngine;
use crate::renderer::Output;
use crate::renderer::output::ClipRegion;

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

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let width = layout.width as u16;
    let height = layout.height as u16;

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

    let needs_clip = matches!(
        element.style.overflow_x,
        Overflow::Hidden | Overflow::Scroll
    ) || matches!(
        element.style.overflow_y,
        Overflow::Hidden | Overflow::Scroll
    );

    let clip_x = x + if element.style.has_border() { 1 } else { 0 };
    let clip_y = y + if element.style.has_border() { 1 } else { 0 };
    let clip_width = width.saturating_sub(if element.style.has_border() { 2 } else { 0 });
    let clip_height = height.saturating_sub(if element.style.has_border() { 2 } else { 0 });

    if needs_clip && clip_width > 0 && clip_height > 0 {
        output.clip(ClipRegion {
            x1: clip_x,
            y1: clip_y,
            x2: clip_x.saturating_add(clip_width),
            y2: clip_y.saturating_add(clip_height),
        });
    }

    let scroll_offset_x = element.scroll_offset_x.unwrap_or(0) as f32;
    let scroll_offset_y = element.scroll_offset_y.unwrap_or(0) as f32;
    let child_offset_x = offset_x + layout.x - scroll_offset_x;
    let child_offset_y = offset_y + layout.y - scroll_offset_y;

    for child in &element.children {
        render_element_tree(child, layout_engine, output, child_offset_x, child_offset_y);
    }

    if needs_clip && clip_width > 0 && clip_height > 0 {
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

    let mut base_style = element.style.clone();
    base_style.dim = element.style.border_dim;

    let mut top_style = base_style.clone();
    top_style.color = element.style.get_border_top_color();

    let mut right_style = base_style.clone();
    right_style.color = element.style.get_border_right_color();

    let mut bottom_style = base_style.clone();
    bottom_style.color = element.style.get_border_bottom_color();

    let mut left_style = base_style.clone();
    left_style.color = element.style.get_border_left_color();

    if element.style.border_top && height > 0 && width > 0 {
        output.write_char(x, y, tl, &top_style);
        if width > 2 {
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, y, h, &top_style);
            }
        }
        if width > 1 {
            output.write_char(x + width - 1, y, tr, &top_style);
        }
    }

    if element.style.border_bottom && height > 1 && width > 0 {
        let bottom_y = y + height - 1;
        output.write_char(x, bottom_y, bl, &bottom_style);
        if width > 2 {
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, bottom_y, h, &bottom_style);
            }
        }
        if width > 1 {
            output.write_char(x + width - 1, bottom_y, br, &bottom_style);
        }
    }

    if element.style.border_left && height > 1 {
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x, row, v, &left_style);
        }
    }

    if element.style.border_right && width > 1 && height > 1 {
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x + width - 1, row, v, &right_style);
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
