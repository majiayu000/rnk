//! Layout alignment correctness tests
//!
//! These tests verify that elements are positioned correctly in the rendered output.

use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;

/// Strip ANSI escape codes from a string
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Get the starting column (number of leading spaces) of each line
fn get_line_starts(output: &str) -> Vec<usize> {
    output
        .lines()
        .map(|line| {
            let stripped = strip_ansi(line);
            stripped.len() - stripped.trim_start().len()
        })
        .collect()
}

/// Check if all non-empty lines start at column 0 (left-aligned)
fn is_left_aligned(output: &str) -> bool {
    for line in output.lines() {
        let stripped = strip_ansi(line);
        if !stripped.is_empty() && !stripped.chars().next().map(|c| c != ' ').unwrap_or(true) {
            // Line starts with space - check if it's intentional padding or wrong alignment
            let leading_spaces = stripped.len() - stripped.trim_start().len();
            if leading_spaces > 0 {
                return false;
            }
        }
    }
    true
}

#[test]
fn test_simple_column_layout_left_aligned() {
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::FlexStart)
        .width(80)
        .child(Text::new("Line 1").into_element())
        .child(Text::new("Line 2").into_element())
        .child(Text::new("Line 3").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, 80);
    let starts = get_line_starts(&output);

    println!("Output:\n{}", output);
    println!("Line starts: {:?}", starts);

    // All lines should start at column 0
    for (i, &start) in starts.iter().enumerate() {
        assert_eq!(start, 0, "Line {} should start at column 0, but starts at {}", i, start);
    }
}

#[test]
fn test_nested_column_layout_left_aligned() {
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::FlexStart)
        .width(80)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .align_items(AlignItems::FlexStart)
                .child(Text::new("Title").bold().into_element())
                .child(Text::new("Subtitle").dim().into_element())
                .into_element(),
        )
        .child(Text::new("Content").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, 80);
    let starts = get_line_starts(&output);

    println!("Output:\n{}", output);
    println!("Line starts: {:?}", starts);

    // All lines should start at column 0
    for (i, &start) in starts.iter().enumerate() {
        assert_eq!(start, 0, "Line {} should start at column 0, but starts at {}", i, start);
    }
}

#[test]
fn test_row_layout_on_same_line() {
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .width(80)
        .child(Text::new("Left").into_element())
        .child(Text::new(" Right").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, 80);
    let lines: Vec<&str> = output.lines().collect();

    println!("Output:\n{}", output);
    println!("Number of lines: {}", lines.len());

    // Row layout should produce a single line
    assert_eq!(lines.len(), 1, "Row layout should produce 1 line, got {}", lines.len());

    let stripped = strip_ansi(lines[0]);
    assert!(stripped.contains("Left"), "Should contain 'Left'");
    assert!(stripped.contains("Right"), "Should contain 'Right'");
}

#[test]
fn test_explicit_width_respected() {
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(100)
        .child(Text::new("Test").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, 100);

    println!("Output:\n{}", output);

    // Text should start at column 0
    let stripped = strip_ansi(&output);
    let first_line = stripped.lines().next().unwrap_or("");
    let leading_spaces = first_line.len() - first_line.trim_start().len();

    assert_eq!(leading_spaces, 0, "Text should start at column 0, but has {} leading spaces", leading_spaces);
}

#[test]
fn test_full_width_separator() {
    let width = 80u16;
    let separator = "─".repeat(width as usize);

    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .child(Text::new("Content").into_element())
        .child(Text::new(&separator).into_element())
        .child(Text::new("Footer").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, width);
    let starts = get_line_starts(&output);

    println!("Output:\n{}", output);
    println!("Line starts: {:?}", starts);

    // All lines should start at column 0
    for (i, &start) in starts.iter().enumerate() {
        assert_eq!(start, 0, "Line {} should start at column 0, but starts at {}", i, start);
    }
}

#[test]
fn test_sage_like_layout() {
    // Simulate the sage-cli UI layout
    let term_width = 80u16;
    let separator = "─".repeat(term_width as usize);

    let welcome = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Sage Agent").color(Color::Cyan).bold().into_element())
        .child(Text::new("Rust-based LLM Agent").dim().into_element())
        .into_element();

    let bottom = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(&separator).dim().into_element())
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
                .child(Text::new("Type your message...").into_element())
                .into_element(),
        )
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("▸▸").into_element())
                .child(Text::new(" permissions required").dim().into_element())
                .into_element(),
        )
        .into_element();

    let root = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::FlexStart)
        .width(term_width as i32)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .align_items(AlignItems::FlexStart)
                .flex_grow(1.0)
                .child(welcome)
                .into_element(),
        )
        .child(bottom)
        .into_element();

    let output = rnk::render_to_string(&root, term_width);
    let starts = get_line_starts(&output);

    println!("=== Sage-like Layout Test ===");
    for (i, line) in output.lines().enumerate() {
        let stripped = strip_ansi(line);
        println!("{:2}: [{}] starts at col {}", i, stripped, starts.get(i).unwrap_or(&999));
    }
    println!("=== End ===");

    // Critical: All lines must start at column 0
    for (i, &start) in starts.iter().enumerate() {
        assert_eq!(start, 0, "Line {} should start at column 0, but starts at column {}", i, start);
    }
}
