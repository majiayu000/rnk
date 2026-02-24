//! Element to string rendering
//!
//! This module provides utilities for rendering elements to strings
//! outside of the main application runtime.

use crate::core::Element;
use crate::layout::LayoutEngine;
use crate::renderer::tree_renderer::render_element_tree;
use crate::renderer::{Output, Terminal};

/// Options for controlling render-to-string behavior.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Whether to trim trailing whitespace from each line (default: true)
    pub trim: bool,
    /// Whether to normalize CRLF to LF (default: true).
    /// Set to false for raw terminal mode where CRLF is needed.
    pub normalize_line_endings: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            trim: true,
            normalize_line_endings: true,
        }
    }
}

/// Render an element to a string with full control over options.
pub fn render_to_string_with_options(
    element: &Element,
    width: u16,
    options: &RenderOptions,
) -> String {
    let raw = RenderHelper.render_to_output(element, width);

    if !options.normalize_line_endings {
        return raw;
    }

    let normalized = raw.replace("\r\n", "\n");

    if options.trim {
        normalized
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        normalized
    }
}

/// Render an element to a string with specified width.
///
/// Trims trailing whitespace and normalizes line endings to LF.
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
    render_to_string_with_options(element, width, &RenderOptions::default())
}

/// Render an element to a string without trimming trailing spaces.
pub fn render_to_string_no_trim(element: &Element, width: u16) -> String {
    render_to_string_with_options(
        element,
        width,
        &RenderOptions {
            trim: false,
            ..Default::default()
        },
    )
}

/// Render an element to a string with CRLF line endings for raw mode.
///
/// Use this when writing to a terminal in raw mode, where `\n` alone
/// does not perform a carriage return.
pub fn render_to_string_raw(element: &Element, width: u16) -> String {
    render_to_string_with_options(
        element,
        width,
        &RenderOptions {
            trim: false,
            normalize_line_endings: false,
        },
    )
}

/// Render an element to a string with automatic width detection.
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

/// Helper struct for rendering elements outside the app runtime
struct RenderHelper;

impl RenderHelper {
    fn render_to_output(&self, element: &Element, width: u16) -> String {
        let mut engine = LayoutEngine::new();
        let layout_width = width;
        let height = self.calculate_element_height(element, layout_width, &mut engine);
        engine.compute(element, layout_width, height.max(1000));

        let _layout = engine.get_layout(element.id).unwrap_or_default();
        let render_width = layout_width;
        let content_height = height.max(1);

        let mut output = Output::new(render_width, content_height);
        let clip_depth_before = output.clip_depth();
        render_element_tree(element, &engine, &mut output, 0.0, 0.0);
        debug_assert_eq!(
            output.clip_depth(),
            clip_depth_before,
            "render_to_string left an unbalanced clip stack"
        );

        output.render()
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
                total_lines += wrapped.lines().count().max(1);
            }
            height = height.max(total_lines as u16);
        }

        // Check text_content with wrapping
        if let Some(text) = &element.text_content {
            let wrapped = wrap_text(text, available_width as usize);
            height = height.max(wrapped.lines().count().max(1) as u16);
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

    #[test]
    fn test_render_to_string_applies_scroll_offset() {
        let element = Box::new()
            .padding_left(4.0)
            .scroll_offset_x(2)
            .child(Text::new("X").into_element())
            .into_element();

        let output = render_to_string(&element, 20);
        let first_line = output.lines().next().unwrap_or_default();
        let x_pos = first_line.find('X').unwrap_or(usize::MAX);

        assert_eq!(x_pos, 2);
    }
}
