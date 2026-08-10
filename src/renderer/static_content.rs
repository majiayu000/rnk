//! Static content handling for inline mode
//!
//! This module handles the extraction and rendering of `Static` elements,
//! which are elements that persist in the terminal history (like Ink's `<Static>`).

use crate::core::{Display, Element, ElementType};
use crate::layout::{LayoutEngine, TransactionalLayoutError};
use crate::renderer::{
    CheckedRenderError, Output, TextRenderError, legacy_snapshot_coordinate_error,
    try_render_element_snapshot_checked,
};

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
    #[allow(dead_code)]
    pub(crate) fn extract_static_content(&self, element: &Element, width: u16) -> Vec<String> {
        self.try_extract_static_content(element, width)
            .unwrap_or_else(|error| panic!("static text render failed: {error}"))
    }

    pub(crate) fn try_extract_static_content(
        &self,
        element: &Element,
        width: u16,
    ) -> Result<Vec<String>, TextRenderError> {
        self.try_extract_static_content_checked(element, width)
            .map_err(|error| match error {
                CheckedRenderError::Text(source) => source,
                CheckedRenderError::LayoutBuild(TransactionalLayoutError::Snapshot(source)) => {
                    legacy_snapshot_coordinate_error(element, &source).unwrap_or_else(|| {
                        panic!("legacy static renderer cannot represent snapshot error: {source}")
                    })
                }
                CheckedRenderError::LayoutBuild(TransactionalLayoutError::RecoveredSnapshot(
                    source,
                )) => legacy_snapshot_coordinate_error(element, source.snapshot_failure())
                    .unwrap_or_else(|| {
                        panic!("legacy static renderer cannot represent snapshot error: {source}")
                    }),
                other => panic!("legacy static renderer cannot represent checked error: {other}"),
            })
    }

    pub(crate) fn try_extract_static_content_checked(
        &self,
        element: &Element,
        width: u16,
    ) -> Result<Vec<String>, CheckedRenderError> {
        let mut lines = Vec::new();
        self.try_extract_recursive_checked(element, width, &mut lines)?;
        Ok(lines)
    }

    /// Recursive helper for extracting static content
    fn try_extract_recursive_checked(
        &self,
        element: &Element,
        width: u16,
        lines: &mut Vec<String>,
    ) -> Result<(), CheckedRenderError> {
        if element.style.display == Display::None
            || element.element_type == ElementType::VirtualText
        {
            return Ok(());
        }
        if element.style.is_static {
            // Only render if the static element has children (new items)
            // Empty Static elements mean all items have already been rendered
            if !element.children.is_empty() {
                // Render static element to get its content
                let prepared =
                    LayoutEngine::new().prepare_element_incremental(element, None, width, 100)?;
                let bounds = prepared.snapshot().root().border_bounds();
                let render_width = u16::try_from(bounds.width().max(1)).unwrap_or(width);
                let render_height = u16::try_from(bounds.height().max(1)).unwrap_or(100);
                let mut output = Output::new(render_width, render_height);
                let clip_depth_before = output.clip_depth();
                try_render_element_snapshot_checked(
                    element,
                    prepared.prepared_snapshot(),
                    &mut output,
                    0.0,
                    0.0,
                )?;
                debug_assert_eq!(
                    output.clip_depth(),
                    clip_depth_before,
                    "static content render left an unbalanced clip stack"
                );

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
            self.try_extract_recursive_checked(child, width, lines)?;
        }
        Ok(())
    }

    /// Publish lines after the terminal frame has succeeded.
    pub(crate) fn commit_prepared_lines(&mut self, new_lines: Vec<String>) {
        self.committed_lines.extend(new_lines);
    }

    #[cfg(test)]
    pub(crate) fn committed_lines(&self) -> &[String] {
        &self.committed_lines
    }

    /// Filter out static elements from the tree
    ///
    /// Returns a new element tree with all static elements removed,
    /// leaving only dynamic content for rendering.
    pub(crate) fn filter_static_elements(&self, element: &Element) -> Element {
        let mut new_element = element.clone();
        // This is a projection of the current frame, not a new logical tree.
        // Preserve source IDs so layout aliases and runtime measurements refer
        // to the Elements that the component actually produced.
        new_element.id = element.id;

        // Remove static children
        new_element.children = element
            .children
            .iter()
            .filter(|child| !child.style.is_static)
            .map(|child| self.filter_static_elements(child))
            .collect();

        new_element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::renderer::TextCoordinateError;

    #[test]
    fn test_static_renderer_creation() {
        let renderer = StaticRenderer::new();
        assert_eq!(renderer.committed_lines.len(), 0);
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

    #[test]
    fn static_render_failure_returns_no_partial_candidate() {
        let renderer = StaticRenderer::new();
        let mut valid = Box::new()
            .child(Text::new("valid").into_element())
            .into_element();
        valid.style.is_static = true;
        let mut invalid_text = Text::new("invalid").into_element();
        invalid_text.style.padding.left = f32::NAN;
        let mut invalid = Box::new().child(invalid_text).into_element();
        invalid.style.is_static = true;
        let tree = Box::new().children([valid, invalid]).into_element();

        assert!(matches!(
            renderer.try_extract_static_content(&tree, 20),
            Err(TextRenderError::Coordinate {
                source: TextCoordinateError::NonFinite,
                ..
            })
        ));
        assert!(renderer.committed_lines.is_empty());
    }

    #[test]
    fn hidden_and_virtual_static_roots_are_filtered_before_layout() {
        let renderer = StaticRenderer::new();
        let mut hidden = Box::new()
            .child(Text::new("hidden").into_element())
            .into_element();
        hidden.style.is_static = true;
        hidden.style.display = crate::core::Display::None;
        let mut virtual_text = Element::new(crate::core::ElementType::VirtualText);
        virtual_text.style.is_static = true;
        virtual_text.add_child(Text::new("virtual").into_element());

        assert!(
            renderer
                .try_extract_static_content_checked(&hidden, 20)
                .expect("hidden static root is filtered")
                .is_empty()
        );
        assert!(
            renderer
                .try_extract_static_content_checked(&virtual_text, 20)
                .expect("VirtualText static root is filtered")
                .is_empty()
        );
    }
}
