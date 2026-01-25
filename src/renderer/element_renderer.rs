//! Element rendering utilities
//!
//! This module provides functions for rendering elements to an output buffer.

use crate::components::text::Line;
use crate::core::Element;
use crate::layout::LayoutEngine;
use crate::renderer::Output;
use crate::renderer::output::ClipRegion;

/// Render an element tree to an output buffer
pub(crate) fn render_element(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) {
    // Skip elements with display: none
    if element.style.display == crate::core::Display::None {
        return;
    }

    // Get layout for this element
    let layout = layout_engine.get_layout(element.id).unwrap_or_default();

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let width = layout.width as u16;
    let height = layout.height as u16;

    // Render background if set
    if element.style.background_color.is_some() {
        output.fill_rect(x, y, width, height, ' ', &element.style);
    }

    // Render border if set
    if element.style.has_border() {
        render_border(element, output, x, y, width, height);
    }

    // Render text content (simple or rich text with spans)
    let text_x =
        x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
    let text_y =
        y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;

    if let Some(spans) = &element.spans {
        // Rich text with multiple spans
        render_spans(spans, output, text_x, text_y);
    } else if let Some(text) = &element.text_content {
        // Simple text
        output.write(text_x, text_y, text, &element.style);
    }

    // Check if overflow clipping is needed for children
    let needs_clip = element.style.overflow_x == crate::core::Overflow::Hidden
        || element.style.overflow_x == crate::core::Overflow::Scroll
        || element.style.overflow_y == crate::core::Overflow::Hidden
        || element.style.overflow_y == crate::core::Overflow::Scroll;

    // Calculate content area for clipping (inside border and padding)
    let clip_x = x + if element.style.has_border() { 1 } else { 0 };
    let clip_y = y + if element.style.has_border() { 1 } else { 0 };
    let clip_width = width.saturating_sub(if element.style.has_border() { 2 } else { 0 });
    let clip_height = height.saturating_sub(if element.style.has_border() { 2 } else { 0 });

    // Apply clip region if overflow is hidden or scroll
    if needs_clip && clip_width > 0 && clip_height > 0 {
        output.clip(ClipRegion {
            x1: clip_x,
            y1: clip_y,
            x2: clip_x + clip_width,
            y2: clip_y + clip_height,
        });
    }

    // Render children - Taffy already includes border/padding in child positions
    // Apply scroll offset if element has scroll_offset set
    let scroll_offset_x = element.scroll_offset_x.unwrap_or(0) as f32;
    let scroll_offset_y = element.scroll_offset_y.unwrap_or(0) as f32;

    let child_offset_x = offset_x + layout.x - scroll_offset_x;
    let child_offset_y = offset_y + layout.y - scroll_offset_y;

    for child in &element.children {
        render_element(child, layout_engine, output, child_offset_x, child_offset_y);
    }

    // Remove clip region
    if needs_clip && clip_width > 0 && clip_height > 0 {
        output.unclip();
    }
}

/// Render border for an element
pub(crate) fn render_border(
    element: &Element,
    output: &mut Output,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    let (tl, tr, bl, br, h, v) = element.style.border_style.chars();

    // Create base style for borders
    let mut base_style = element.style.clone();
    base_style.dim = element.style.border_dim;

    // Create per-side styles with their respective colors
    let mut top_style = base_style.clone();
    top_style.color = element.style.get_border_top_color();

    let mut right_style = base_style.clone();
    right_style.color = element.style.get_border_right_color();

    let mut bottom_style = base_style.clone();
    bottom_style.color = element.style.get_border_bottom_color();

    let mut left_style = base_style.clone();
    left_style.color = element.style.get_border_left_color();

    // Top border
    if element.style.border_top && height > 0 {
        // Top-left corner uses left color if no top color, or top color
        output.write_char(x, y, tl.chars().next().unwrap(), &top_style);
        for col in (x + 1)..(x + width - 1) {
            output.write_char(col, y, h.chars().next().unwrap(), &top_style);
        }
        if width > 1 {
            // Top-right corner uses top color
            output.write_char(x + width - 1, y, tr.chars().next().unwrap(), &top_style);
        }
    }

    // Bottom border
    if element.style.border_bottom && height > 1 {
        let bottom_y = y + height - 1;
        output.write_char(x, bottom_y, bl.chars().next().unwrap(), &bottom_style);
        for col in (x + 1)..(x + width - 1) {
            output.write_char(col, bottom_y, h.chars().next().unwrap(), &bottom_style);
        }
        if width > 1 {
            output.write_char(
                x + width - 1,
                bottom_y,
                br.chars().next().unwrap(),
                &bottom_style,
            );
        }
    }

    // Left border
    if element.style.border_left {
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x, row, v.chars().next().unwrap(), &left_style);
        }
    }

    // Right border
    if element.style.border_right && width > 1 {
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x + width - 1, row, v.chars().next().unwrap(), &right_style);
        }
    }
}

/// Render rich text spans
pub(crate) fn render_spans(lines: &[Line], output: &mut Output, start_x: u16, start_y: u16) {
    for (line_idx, line) in lines.iter().enumerate() {
        let y = start_y + line_idx as u16;
        let mut x = start_x;

        for span in &line.spans {
            output.write(x, y, &span.content, &span.style);
            x += span.width() as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::core::BorderStyle;

    #[test]
    fn test_render_simple_text() {
        let element = Text::new("Hello").into_element();
        let mut engine = LayoutEngine::new();
        engine.compute(&element, 80, 24);

        let mut output = Output::new(80, 24);
        render_element(&element, &engine, &mut output, 0.0, 0.0);

        let rendered = output.render();
        assert!(rendered.contains("Hello"));
    }

    #[test]
    fn test_render_with_border() {
        let element = Box::new()
            .border_style(BorderStyle::Single)
            .child(Text::new("Test").into_element())
            .into_element();

        let mut engine = LayoutEngine::new();
        engine.compute(&element, 80, 24);

        let mut output = Output::new(80, 24);
        render_element(&element, &engine, &mut output, 0.0, 0.0);

        let rendered = output.render();
        assert!(rendered.contains("Test"));
        assert!(rendered.contains("─")); // Border character
    }
}
