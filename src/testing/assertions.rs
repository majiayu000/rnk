//! Assertion helpers for testing Tink components
//!
//! Provides convenient assertion macros and functions for verifying
//! layout and rendering correctness.

use crate::core::Element;
use crate::layout::Layout;
use super::renderer::{TestRenderer, display_width};

/// Assert that an element renders to the expected plain text
pub fn assert_renders_to(element: &Element, expected: &str) {
    let renderer = TestRenderer::standard();
    let output = renderer.render_to_plain(element);
    let trimmed = output.trim();

    assert_eq!(
        trimmed, expected,
        "\nExpected:\n{}\n\nActual:\n{}\n",
        expected, trimmed
    );
}

/// Assert that an element renders containing the expected text
pub fn assert_renders_containing(element: &Element, expected: &str) {
    let renderer = TestRenderer::standard();
    let output = renderer.render_to_plain(element);

    assert!(
        output.contains(expected),
        "\nExpected output to contain:\n{}\n\nActual output:\n{}\n",
        expected, output
    );
}

/// Assert that layout has expected dimensions
pub fn assert_layout_dimensions(layout: &Layout, expected_width: f32, expected_height: f32) {
    assert!(
        (layout.width - expected_width).abs() < 0.5,
        "Expected width {}, got {}",
        expected_width,
        layout.width
    );
    assert!(
        (layout.height - expected_height).abs() < 0.5,
        "Expected height {}, got {}",
        expected_height,
        layout.height
    );
}

/// Assert that layout has expected position
pub fn assert_layout_position(layout: &Layout, expected_x: f32, expected_y: f32) {
    assert!(
        (layout.x - expected_x).abs() < 0.5,
        "Expected x {}, got {}",
        expected_x,
        layout.x
    );
    assert!(
        (layout.y - expected_y).abs() < 0.5,
        "Expected y {}, got {}",
        expected_y,
        layout.y
    );
}

/// Assert that text has expected display width
pub fn assert_text_width(text: &str, expected: usize) {
    let actual = display_width(text);
    assert_eq!(
        actual, expected,
        "Text '{}' has display width {}, expected {}",
        text, actual, expected
    );
}

/// Assert that element layout is valid
pub fn assert_layout_valid(element: &Element, width: u16, height: u16) {
    let renderer = TestRenderer::new(width, height);
    if let Err(e) = renderer.validate_layout(element) {
        panic!("Layout validation failed: {}", e);
    }
}

/// Assert that two rendered outputs are visually identical
pub fn assert_renders_equal(element1: &Element, element2: &Element) {
    let renderer = TestRenderer::standard();
    let output1 = renderer.render_to_plain(element1);
    let output2 = renderer.render_to_plain(element2);

    assert_eq!(
        output1, output2,
        "\nFirst element:\n{}\n\nSecond element:\n{}\n",
        output1, output2
    );
}

/// Trait for asserting element properties
pub trait ElementAssertions {
    fn assert_renders_to(&self, expected: &str);
    fn assert_renders_containing(&self, expected: &str);
    fn assert_layout_valid(&self);
    fn assert_dimensions(&self, width: f32, height: f32);
}

impl ElementAssertions for Element {
    fn assert_renders_to(&self, expected: &str) {
        assert_renders_to(self, expected);
    }

    fn assert_renders_containing(&self, expected: &str) {
        assert_renders_containing(self, expected);
    }

    fn assert_layout_valid(&self) {
        assert_layout_valid(self, 80, 24);
    }

    fn assert_dimensions(&self, width: f32, height: f32) {
        let renderer = TestRenderer::standard();
        let layout = renderer.get_layout(self).expect("Element should have layout");
        assert_layout_dimensions(&layout, width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;

    #[test]
    fn test_assert_text_width() {
        assert_text_width("hello", 5);
        assert_text_width("你好", 4);
        assert_text_width("a你b", 4);
    }

    #[test]
    fn test_assert_renders_containing() {
        let element = Text::new("Hello World").into_element();
        assert_renders_containing(&element, "Hello");
        assert_renders_containing(&element, "World");
    }
}
