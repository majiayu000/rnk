//! Static content handling for inline mode
//!
//! This module handles the extraction and rendering of `Static` elements,
//! which are elements that persist in the terminal history (like Ink's `<Static>`).

use crate::core::Element;
use crate::layout::LayoutEngine;
use crate::renderer::{Output, Terminal};

/// Static content renderer for inline mode
///
/// Handles the extraction, rendering, and committing of static content
/// that should persist in terminal history.
pub(crate) struct StaticRenderer {
    /// Lines of static content that have been committed
    committed_lines: Vec<String>,
}

impl StaticRenderer {
    /// Create a new static renderer
    pub(crate) fn new() -> Self {
        Self {
            committed_lines: Vec::new(),
        }
    }

    /// Extract static content from the element tree
    ///
    /// Only extracts content from Static elements that have actual children
    /// (new items to render). Empty Static elements are skipped.
    pub(crate) fn extract_static_content(&self, element: &Element, width: u16) -> Vec<String> {
        let mut lines = Vec::new();
        self.extract_recursive(element, width, &mut lines);
        lines
    }

    /// Recursive helper for extracting static content
    fn extract_recursive(&self, element: &Element, width: u16, lines: &mut Vec<String>) {
        if element.style.is_static {
            // Only render if the static element has children (new items)
            // Empty Static elements mean all items have already been rendered
            if !element.children.is_empty() {
                // Render static element to get its content
                let mut engine = LayoutEngine::new();
                engine.compute(element, width, 100); // Use large height for static content

                let layout = engine.get_layout(element.id).unwrap_or_default();
                // Ensure we have valid dimensions
                let render_width = (layout.width as u16).max(1);
                let render_height = (layout.height as u16).max(1);
                let mut output = Output::new(render_width, render_height);
                Self::render_element_to_output(element, &engine, &mut output, 0.0, 0.0);

                let rendered = output.render();
                for line in rendered.lines() {
                    // Skip empty lines to avoid clutter
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        lines.push(line.to_string());
                    }
                }
            }
        }

        // Check children for static content (non-static elements might contain static children)
        for child in &element.children {
            self.extract_recursive(child, width, lines);
        }
    }

    /// Commit static content to the terminal (write permanently)
    ///
    /// This follows the Ink/Bubbletea pattern:
    /// 1. Clear the current dynamic UI
    /// 2. Write the static content (which will persist)
    /// 3. The dynamic UI will be re-rendered below
    pub(crate) fn commit_static_content(
        &mut self,
        new_lines: &[String],
        terminal: &mut Terminal,
    ) -> std::io::Result<()> {
        use std::io::{Write, stdout};

        // Skip if no lines to commit
        if new_lines.is_empty() {
            return Ok(());
        }

        // Clear current dynamic UI first (like Ink's log.clear())
        terminal.clear()?;

        let mut stdout = stdout();
        for line in new_lines {
            // Write the line with erase-to-end-of-line to ensure clean output
            writeln!(stdout, "{}\x1b[K", line)?;
            self.committed_lines.push(line.clone());
        }
        stdout.flush()?;

        // Force a full repaint of the dynamic UI
        terminal.repaint();

        Ok(())
    }

    /// Filter out static elements from the tree
    ///
    /// Returns a new element tree with all static elements removed,
    /// leaving only dynamic content for rendering.
    pub(crate) fn filter_static_elements(&self, element: &Element) -> Element {
        let mut new_element = element.clone();

        // Remove static children
        new_element.children = element
            .children
            .iter()
            .filter(|child| !child.style.is_static)
            .map(|child| self.filter_static_elements(child))
            .collect();

        new_element
    }

    /// Get the number of committed lines
    #[allow(dead_code)]
    pub(crate) fn committed_line_count(&self) -> usize {
        self.committed_lines.len()
    }

    /// Render element to output buffer (helper for static content)
    fn render_element_to_output(
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
            Self::render_border(element, output, x, y, width, height);
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
            Self::render_element_to_output(child, engine, output, child_offset_x, child_offset_y);
        }
    }

    /// Render border for an element
    fn render_border(
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

    #[test]
    fn test_static_renderer_creation() {
        let renderer = StaticRenderer::new();
        assert_eq!(renderer.committed_line_count(), 0);
    }

    #[test]
    fn test_extract_empty_element() {
        let renderer = StaticRenderer::new();
        let element = Text::new("Hello").into_element();
        let lines = renderer.extract_static_content(&element, 80);
        assert!(lines.is_empty()); // Non-static elements return empty
    }

    #[test]
    fn test_filter_static_elements() {
        let renderer = StaticRenderer::new();

        // Create element with static child
        let mut static_child = Text::new("Static").into_element();
        static_child.style.is_static = true;

        let dynamic_child = Text::new("Dynamic").into_element();

        let parent = Box::new()
            .child(static_child)
            .child(dynamic_child)
            .into_element();

        let filtered = renderer.filter_static_elements(&parent);

        // Should only have the dynamic child
        assert_eq!(filtered.children.len(), 1);
        assert!(!filtered.children.get(0).unwrap().style.is_static);
    }

    #[test]
    fn test_extract_static_with_children() {
        let renderer = StaticRenderer::new();

        // Create a static element with children
        let mut static_element = Box::new()
            .child(Text::new("Line 1").into_element())
            .into_element();
        static_element.style.is_static = true;

        let lines = renderer.extract_static_content(&static_element, 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_filter_nested_static() {
        let renderer = StaticRenderer::new();

        // Create nested structure with static element
        let mut static_child = Text::new("Static").into_element();
        static_child.style.is_static = true;

        let inner_box = Box::new().child(static_child).into_element();

        let outer_box = Box::new()
            .child(inner_box)
            .child(Text::new("Dynamic").into_element())
            .into_element();

        let filtered = renderer.filter_static_elements(&outer_box);

        // Outer should have 2 children, but inner should have 0 (static filtered out)
        assert_eq!(filtered.children.len(), 2);
        assert_eq!(filtered.children.get(0).unwrap().children.len(), 0);
    }
}
