//! Element to string rendering
//!
//! This module provides utilities for rendering elements to strings
//! outside of the main application runtime.

use crate::core::Element;
use crate::layout::LayoutEngine;
use crate::renderer::{Output, Terminal};

/// Render an element to a string with specified width.
///
/// This is useful for rendering elements outside the runtime,
/// such as in CLI tools or for testing.
///
/// # Arguments
///
/// * `element` - The element to render
/// * `width` - The maximum width for rendering
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// let element = Box::new()
///     .border_style(BorderStyle::Round)
///     .child(Text::new("Hello!").into_element())
///     .into_element();
///
/// let output = rnk::render_to_string(&element, 80);
/// println!("{}", output);
/// ```
pub fn render_to_string(element: &Element, width: u16) -> String {
    render_to_string_impl(element, width, true)
}

/// Render an element to a string without trimming trailing spaces.
///
/// This is useful when you need to preserve exact spacing.
///
/// # Arguments
///
/// * `element` - The element to render
/// * `width` - The maximum width for rendering
pub fn render_to_string_no_trim(element: &Element, width: u16) -> String {
    render_to_string_impl(element, width, false)
}

/// Render an element to a string with automatic width detection.
///
/// This detects the terminal width and uses it for rendering.
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// let element = Text::new("Hello, world!").into_element();
/// let output = rnk::render_to_string_auto(&element);
/// println!("{}", output);
/// ```
pub fn render_to_string_auto(element: &Element) -> String {
    let (width, _) = Terminal::size().unwrap_or((80, 24));
    render_to_string(element, width)
}

/// Internal implementation for rendering element to string
fn render_to_string_impl(element: &Element, width: u16, trim: bool) -> String {
    let helper = RenderHelper;
    helper.render_element_to_string_impl(element, width, trim)
}

/// Helper struct for rendering elements outside the app runtime
struct RenderHelper;

impl RenderHelper {
    fn render_element_to_string_impl(&self, element: &Element, width: u16, trim: bool) -> String {
        let mut engine = LayoutEngine::new();

        // Use the passed width directly for layout computation
        // This respects the caller's intended width (e.g., terminal width)
        let layout_width = width;

        // Calculate actual height considering text wrapping
        let height = self.calculate_element_height(element, layout_width, &mut engine);

        // Compute layout with the specified width
        engine.compute(element, layout_width, height.max(1000));

        // Get layout dimensions (layout is used to confirm computation completed)
        let _layout = engine.get_layout(element.id).unwrap_or_default();
        // IMPORTANT: Use the full layout_width for the output buffer, not the computed root width.
        // Taffy computes child positions relative to the container width (layout_width),
        // so we need the output buffer to match this width for correct positioning.
        let render_width = layout_width;
        let content_height = height.max(1);

        // Render to output buffer
        let mut output = Output::new(render_width, content_height);
        self.render_element_to_output(element, &engine, &mut output, 0.0, 0.0);

        let rendered = output.render();

        // Normalize line endings to LF and trim trailing spaces if requested
        // output.render() uses CRLF for raw mode, but render_to_string should use LF
        let normalized = rendered.replace("\r\n", "\n");

        // Trim trailing spaces from each line if requested
        if trim {
            normalized
                .lines()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            normalized
        }
    }

    #[allow(dead_code)]
    fn calculate_content_width(&self, element: &Element) -> u16 {
        use unicode_width::UnicodeWidthStr;

        // If element has explicit width set, use it
        if let crate::core::Dimension::Points(w) = element.style.width {
            return w as u16;
        }

        let mut width = 0u16;

        // Calculate text content width
        if let Some(text) = &element.text_content {
            width = width.max(text.width() as u16);
        }

        // Calculate spans width
        if let Some(lines) = &element.spans {
            for line in lines {
                let line_width: usize = line.spans.iter().map(|span| span.width()).sum();
                width = width.max(line_width as u16);
            }
        }

        // Recursively check children for row layout
        if element.style.flex_direction == crate::core::FlexDirection::Row {
            let mut child_width_sum = 0u16;
            for child in &element.children {
                let child_width = self.calculate_content_width(child);
                child_width_sum = child_width_sum.saturating_add(child_width);
            }
            width = width.max(child_width_sum);
        } else {
            // For column layout, take the maximum child width
            for child in &element.children {
                let child_width = self.calculate_content_width(child);
                width = width.max(child_width);
            }
        }

        // Add border width
        if element.style.has_border() {
            width = width.saturating_add(2);
        }

        // Add padding width
        let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
        width = width.saturating_add(padding_h);

        width.max(1)
    }

    fn calculate_element_height(
        &self,
        element: &Element,
        max_width: u16,
        _engine: &mut LayoutEngine,
    ) -> u16 {
        use crate::layout::measure::wrap_text;

        let mut height = 1u16;

        // Calculate available width for text
        let available_width = if element.style.has_border() {
            max_width.saturating_sub(2)
        } else {
            max_width
        };
        let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
        let available_width = available_width.saturating_sub(padding_h).max(1);

        // Check for multiline spans with wrapping
        if let Some(lines) = &element.spans {
            let mut total_lines = 0usize;
            for line in lines {
                let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
                let wrapped = wrap_text(&line_text, available_width as usize);
                total_lines += wrapped.len();
            }
            height = height.max(total_lines as u16);
        }

        // Check text_content with wrapping
        if let Some(text) = &element.text_content {
            let wrapped = wrap_text(text, available_width as usize);
            height = height.max(wrapped.len() as u16);
        }

        // Add border height
        if element.style.has_border() {
            height = height.saturating_add(2);
        }

        // Add padding height
        let padding_v = (element.style.padding.top + element.style.padding.bottom) as u16;
        height = height.saturating_add(padding_v);

        // Recursively check children and accumulate height based on layout direction
        if !element.children.is_empty() {
            let mut child_height_sum = 0u16;
            let mut child_height_max = 0u16;
            for child in &element.children {
                let child_height = self.calculate_element_height(child, max_width, _engine);
                child_height_sum = child_height_sum.saturating_add(child_height);
                child_height_max = child_height_max.max(child_height);
            }
            // Column layout: sum heights; Row layout: take max height
            if element.style.flex_direction == crate::core::FlexDirection::Column {
                height = height.saturating_add(child_height_sum);
            } else {
                height = height.max(child_height_max);
            }
        }

        height
    }

    fn render_element_to_output(
        &self,
        element: &Element,
        engine: &LayoutEngine,
        output: &mut Output,
        offset_x: f32,
        offset_y: f32,
    ) {
        // Skip elements with display: none
        if element.style.display == crate::core::Display::None {
            return;
        }

        let layout = engine.get_layout(element.id).unwrap_or_default();

        let x = (offset_x + layout.x) as u16;
        let y = (offset_y + layout.y) as u16;
        let width = layout.width as u16;
        let height = layout.height as u16;

        if element.style.background_color.is_some() {
            output.fill_rect(x, y, width, height, ' ', &element.style);
        }

        if element.style.has_border() {
            self.render_border(element, output, x, y, width, height);
        }

        if let Some(text) = &element.text_content {
            let text_x = x
                + if element.style.has_border() { 1 } else { 0 }
                + element.style.padding.left as u16;
            let text_y = y
                + if element.style.has_border() { 1 } else { 0 }
                + element.style.padding.top as u16;
            output.write(text_x, text_y, text, &element.style);
        }

        let child_offset_x = offset_x + layout.x;
        let child_offset_y = offset_y + layout.y;

        for child in &element.children {
            self.render_element_to_output(child, engine, output, child_offset_x, child_offset_y);
        }
    }

    fn render_border(
        &self,
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
            output.write_char(x, y, tl.chars().next().unwrap(), &top_style);
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, y, h.chars().next().unwrap(), &top_style);
            }
            if width > 1 {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::core::BorderStyle;

    #[test]
    fn test_render_to_string_simple() {
        let element = Text::new("Hello").into_element();
        let output = render_to_string(&element, 80);
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_render_to_string_with_border() {
        let element = Box::new()
            .border_style(BorderStyle::Single)
            .child(Text::new("Test").into_element())
            .into_element();
        let output = render_to_string(&element, 80);
        assert!(output.contains("Test"));
        assert!(output.contains("─")); // Border character
    }

    #[test]
    fn test_render_to_string_no_trim() {
        let element = Text::new("Hi").into_element();
        let trimmed = render_to_string(&element, 80);
        let not_trimmed = render_to_string_no_trim(&element, 80);
        // Both should contain the text
        assert!(trimmed.contains("Hi"));
        assert!(not_trimmed.contains("Hi"));
    }
}
