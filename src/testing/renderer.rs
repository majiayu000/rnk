//! Test renderer for unit testing
//!
//! Provides a side-effect-free renderer that can be used to verify
//! layout and rendering output without terminal interaction.

use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;

use crate::core::{Element, ElementId};
use crate::layout::{Layout, LayoutEngine};
use crate::renderer::Output;
use crate::renderer::tree_renderer::render_element_tree;

/// Test renderer configuration
#[derive(Debug, Clone)]
pub struct TestRenderer {
    width: u16,
    height: u16,
}

impl TestRenderer {
    /// Create a new test renderer with specified dimensions
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// Create a standard 80x24 terminal renderer
    pub fn standard() -> Self {
        Self::new(80, 24)
    }

    /// Get the width
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Get the height
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Render element and return plain text (no ANSI codes)
    pub fn render_to_plain(&self, element: &Element) -> String {
        let ansi = self.render_to_ansi(element);
        strip_ansi_codes(&ansi)
    }

    /// Render element and return string with ANSI codes
    pub fn render_to_ansi(&self, element: &Element) -> String {
        let engine = self.compute_layout(element);

        let mut output = Output::new(self.width, self.height);
        let clip_depth_before = output.clip_depth();
        render_element_tree(element, &engine, &mut output, 0.0, 0.0);
        assert_eq!(
            output.clip_depth(),
            clip_depth_before,
            "test renderer left an unbalanced clip stack"
        );
        output.render()
    }

    /// Get computed layouts for all elements
    pub fn get_layouts(&self, element: &Element) -> HashMap<ElementId, Layout> {
        self.compute_layout(element).get_all_layouts()
    }

    /// Get layout for a specific element
    pub fn get_layout(&self, element: &Element) -> Option<Layout> {
        self.compute_layout(element).get_layout(element.id)
    }

    /// Compute layout for an element tree
    fn compute_layout(&self, element: &Element) -> LayoutEngine {
        let mut engine = LayoutEngine::new();
        engine.compute(element, self.width, self.height);
        engine
    }

    /// Validate layout constraints
    pub fn validate_layout(&self, element: &Element) -> Result<(), LayoutError> {
        let layouts = self.get_layouts(element);

        for (id, layout) in &layouts {
            // Check non-negative coordinates
            if layout.x < 0.0 {
                return Err(LayoutError::NegativeCoordinate {
                    element_id: *id,
                    axis: "x",
                    value: layout.x,
                });
            }
            if layout.y < 0.0 {
                return Err(LayoutError::NegativeCoordinate {
                    element_id: *id,
                    axis: "y",
                    value: layout.y,
                });
            }

            // Check non-negative dimensions
            if layout.width < 0.0 {
                return Err(LayoutError::NegativeDimension {
                    element_id: *id,
                    dimension: "width",
                    value: layout.width,
                });
            }
            if layout.height < 0.0 {
                return Err(LayoutError::NegativeDimension {
                    element_id: *id,
                    dimension: "height",
                    value: layout.height,
                });
            }

            // Check bounds within terminal
            if layout.x + layout.width > self.width as f32 + 0.5 {
                return Err(LayoutError::OutOfBounds {
                    element_id: *id,
                    axis: "x",
                    position: layout.x + layout.width,
                    limit: self.width as f32,
                });
            }
            if layout.y + layout.height > self.height as f32 + 0.5 {
                return Err(LayoutError::OutOfBounds {
                    element_id: *id,
                    axis: "y",
                    position: layout.y + layout.height,
                    limit: self.height as f32,
                });
            }
        }

        Ok(())
    }
}

impl Default for TestRenderer {
    fn default() -> Self {
        Self::standard()
    }
}

/// Layout validation error
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutError {
    NegativeCoordinate {
        element_id: ElementId,
        axis: &'static str,
        value: f32,
    },
    NegativeDimension {
        element_id: ElementId,
        dimension: &'static str,
        value: f32,
    },
    OutOfBounds {
        element_id: ElementId,
        axis: &'static str,
        position: f32,
        limit: f32,
    },
    ChildOutsideParent {
        child_id: ElementId,
        parent_id: ElementId,
    },
    InvalidUnicodeWidth {
        text: String,
        expected: usize,
        actual: usize,
    },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NegativeCoordinate {
                element_id,
                axis,
                value,
            } => {
                write!(
                    f,
                    "Element {:?} has negative {} coordinate: {}",
                    element_id, axis, value
                )
            }
            Self::NegativeDimension {
                element_id,
                dimension,
                value,
            } => {
                write!(
                    f,
                    "Element {:?} has negative {}: {}",
                    element_id, dimension, value
                )
            }
            Self::OutOfBounds {
                element_id,
                axis,
                position,
                limit,
            } => {
                write!(
                    f,
                    "Element {:?} {} position {} exceeds limit {}",
                    element_id, axis, position, limit
                )
            }
            Self::ChildOutsideParent {
                child_id,
                parent_id,
            } => {
                write!(
                    f,
                    "Child {:?} is outside parent {:?} bounds",
                    child_id, parent_id
                )
            }
            Self::InvalidUnicodeWidth {
                text,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Text '{}' has width {} but expected {}",
                    text, actual, expected
                )
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Strip ANSI escape codes from a string
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Calculate display width of text accounting for Unicode
pub fn display_width(s: &str) -> usize {
    s.chars().filter_map(|c| c.width()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box as RnkBox, Text};

    #[test]
    fn test_strip_ansi_codes() {
        assert_eq!(strip_ansi_codes("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi_codes("plain text"), "plain text");
        assert_eq!(
            strip_ansi_codes("\x1b[1;32mbold green\x1b[0m"),
            "bold green"
        );
    }

    #[test]
    fn test_display_width() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("你好"), 4); // CJK characters are 2 wide
        assert_eq!(display_width("hello 世界"), 10);
    }

    #[test]
    fn test_render_to_plain() {
        let renderer = TestRenderer::new(80, 24);
        let element = Text::new("Hello World").into_element();
        let output = renderer.render_to_plain(&element);
        assert!(output.contains("Hello World"));
    }

    #[test]
    fn test_layout_validation() {
        let renderer = TestRenderer::new(80, 24);
        let element = RnkBox::new()
            .width(20)
            .height(5)
            .child(Text::new("Test").into_element())
            .into_element();

        assert!(renderer.validate_layout(&element).is_ok());
    }

    #[test]
    fn test_get_layouts() {
        let renderer = TestRenderer::new(80, 24);
        let element = RnkBox::new().width(20).height(5).into_element();

        let layouts = renderer.get_layouts(&element);
        assert!(!layouts.is_empty());

        let layout = layouts.get(&element.id).unwrap();
        assert_eq!(layout.width, 20.0);
        assert_eq!(layout.height, 5.0);
    }

    #[test]
    fn test_render_to_plain_applies_scroll_offset() {
        let renderer = TestRenderer::new(20, 3);
        let element = RnkBox::new()
            .padding_left(4.0)
            .scroll_offset_x(2)
            .child(Text::new("X").into_element())
            .into_element();

        let output = renderer.render_to_plain(&element);
        let first_line = output.lines().next().unwrap_or_default();
        let x_pos = first_line.find('X').unwrap_or(usize::MAX);

        assert_eq!(x_pos, 2);
    }
}
