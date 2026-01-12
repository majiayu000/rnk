//! Test data generators for property-based testing
//!
//! Provides generators for creating random but valid Element trees
//! for use with property-based testing frameworks like proptest.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Dimension, Element};

/// Generate a random color for testing
pub fn random_color(seed: u8) -> Color {
    match seed % 10 {
        0 => Color::Red,
        1 => Color::Green,
        2 => Color::Blue,
        3 => Color::Yellow,
        4 => Color::Cyan,
        5 => Color::Magenta,
        6 => Color::White,
        7 => Color::Black,
        8 => Color::Ansi256(seed),
        _ => Color::Rgb(seed, seed.wrapping_mul(2), seed.wrapping_mul(3)),
    }
}

/// Sample texts for testing including edge cases
pub const SAMPLE_TEXTS: &[&str] = &[
    // ASCII
    "Hello World",
    "The quick brown fox",
    "",
    " ",
    "   ",
    // CJK
    "你好世界",
    "こんにちは",
    "안녕하세요",
    // Mixed
    "Hello 世界",
    "Test テスト 测试",
    // Emoji
    "👋 Hello",
    "🎉🎊🎁",
    // Box drawing (borders)
    "─│┌┐└┘├┤┬┴┼",
    "═║╔╗╚╝╠╣╦╩╬",
    "╭╮╰╯",
    // Special
    "Tab\there",
    "Line1\nLine2",
    // Long text
    "This is a very long line of text that might need to be wrapped or truncated depending on the terminal width",
    // Numbers
    "123456789",
    "3.14159",
    // Symbols
    "!@#$%^&*()",
    "+-*/=<>[]{}",
];

/// Sample terminal dimensions for testing
pub const SAMPLE_DIMENSIONS: &[(u16, u16)] = &[
    (80, 24),  // Standard
    (120, 40), // Large
    (40, 10),  // Small
    (1, 1),    // Minimal
    (200, 60), // Very large
    (80, 1),   // Single line
    (1, 24),   // Single column
];

/// Generate a simple text element
pub fn gen_text(text: &str) -> Element {
    Text::new(text).into_element()
}

/// Generate a styled text element
pub fn gen_styled_text(text: &str, seed: u8) -> Element {
    let mut t = Text::new(text);

    if seed & 1 != 0 {
        t = t.bold();
    }
    if seed & 2 != 0 {
        t = t.italic();
    }
    if seed & 4 != 0 {
        t = t.underline();
    }
    if seed & 8 != 0 {
        t = t.color(random_color(seed));
    }

    t.into_element()
}

/// Generate a simple box element
pub fn gen_box(width: u16, height: u16) -> Element {
    TinkBox::new()
        .width(Dimension::Points(width as f32))
        .height(Dimension::Points(height as f32))
        .into_element()
}

/// Generate a box with children
pub fn gen_box_with_children(children: Vec<Element>) -> Element {
    let mut b = TinkBox::new();
    for child in children {
        b = b.child(child);
    }
    b.into_element()
}

/// Generate a nested box structure for stress testing
pub fn gen_nested_boxes(depth: usize) -> Element {
    if depth == 0 {
        return Text::new("Leaf").into_element();
    }

    TinkBox::new()
        .padding(1)
        .child(gen_nested_boxes(depth - 1))
        .into_element()
}

/// Generate a row of elements
pub fn gen_row(children: Vec<Element>) -> Element {
    use crate::core::FlexDirection;

    let mut b = TinkBox::new().flex_direction(FlexDirection::Row);
    for child in children {
        b = b.child(child);
    }
    b.into_element()
}

/// Generate a column of elements
pub fn gen_column(children: Vec<Element>) -> Element {
    use crate::core::FlexDirection;

    let mut b = TinkBox::new().flex_direction(FlexDirection::Column);
    for child in children {
        b = b.child(child);
    }
    b.into_element()
}

/// Test case for unicode width validation
pub struct UnicodeWidthTestCase {
    pub text: &'static str,
    pub expected_width: usize,
}

/// Unicode width test cases
pub const UNICODE_WIDTH_CASES: &[UnicodeWidthTestCase] = &[
    // ASCII
    UnicodeWidthTestCase {
        text: "a",
        expected_width: 1,
    },
    UnicodeWidthTestCase {
        text: "hello",
        expected_width: 5,
    },
    UnicodeWidthTestCase {
        text: " ",
        expected_width: 1,
    },
    // CJK (each character is 2 cells wide)
    UnicodeWidthTestCase {
        text: "中",
        expected_width: 2,
    },
    UnicodeWidthTestCase {
        text: "你好",
        expected_width: 4,
    },
    UnicodeWidthTestCase {
        text: "日本語",
        expected_width: 6,
    },
    // Mixed
    UnicodeWidthTestCase {
        text: "a中b",
        expected_width: 4,
    },
    UnicodeWidthTestCase {
        text: "Hello 世界",
        expected_width: 10,
    },
    // Box drawing (1 cell each)
    UnicodeWidthTestCase {
        text: "─",
        expected_width: 1,
    },
    UnicodeWidthTestCase {
        text: "│",
        expected_width: 1,
    },
    UnicodeWidthTestCase {
        text: "╭",
        expected_width: 1,
    },
    UnicodeWidthTestCase {
        text: "├─┤",
        expected_width: 3,
    },
    // Zero-width
    UnicodeWidthTestCase {
        text: "",
        expected_width: 0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::measure::measure_text_width;

    #[test]
    fn test_unicode_width_cases() {
        for case in UNICODE_WIDTH_CASES {
            let actual = measure_text_width(case.text);
            assert_eq!(
                actual, case.expected_width,
                "Text '{}' should have width {}, got {}",
                case.text, case.expected_width, actual
            );
        }
    }

    #[test]
    fn test_gen_nested_boxes() {
        let element = gen_nested_boxes(3);
        // Just verify it doesn't panic
        assert!(!element.children.is_empty() || element.text_content.is_some());
    }

    #[test]
    fn test_gen_row() {
        let row = gen_row(vec![gen_text("A"), gen_text("B"), gen_text("C")]);
        assert_eq!(row.children.len(), 3);
    }
}
