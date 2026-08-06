//! GH-61: the rules that turn float layout into terminal cells.
//!
//! Layout is computed in floats and painted into whole cells. Nothing in the
//! codebase states what happens at that boundary, so the guarantees callers
//! already depend on — that siblings tile their parent exactly, that a border
//! never paints into its neighbour, that a fractional offset lands in one
//! definite cell — hold only by accident of the layout backend's rounding.
//!
//! These tests pin the contract. They are invariants rather than regressions:
//! each one passes today, and each one fails loudly if the quantization rule
//! changes underneath the renderer.
//!
//! The rule, stated once: **a cell is owned by the box whose half-open span
//! `[floor(edge_start), floor(edge_end))` contains it.** Both edges are floored
//! in absolute screen space, on both sides of the origin, so adjacent spans
//! meet exactly and a coordinate below zero stays below zero.

use rnk::core::{BorderStyle, Element};
use rnk::prelude::*;

/// A row of `count` equal-weight children, each filled with its own character.
///
/// Distinct fill characters are what make gaps and overlaps visible: a blank
/// column means no child claimed that cell, and a character in the wrong run
/// means two children claimed it.
fn tiled_row(width: u16, count: usize) -> Element {
    let mut row = Box::new()
        .width(width as f32)
        .height(1.0)
        .flex_direction(FlexDirection::Row)
        .overflow_x(Overflow::Hidden)
        .into_element();

    for index in 0..count {
        let fill = char::from(b'a' + index as u8);
        let mut child = Box::new()
            .flex_grow(1.0)
            .flex_shrink(1.0)
            .height(1.0)
            .child(Text::new(fill.to_string().repeat(width as usize)))
            .into_element();
        child.style.flex_basis = rnk::core::Dimension::Points(0.0);
        child.style.min_width = rnk::core::Dimension::Points(0.0);
        row.add_child(child);
    }
    row
}

fn bordered(width: u16, style: BorderStyle, label: &str) -> Element {
    let mut boxed = Box::new()
        .width(width as f32)
        .height(3.0)
        .child(Text::new(label))
        .into_element();
    boxed.style.border_style = style;
    boxed.style.border_top = true;
    boxed.style.border_bottom = true;
    boxed.style.border_left = true;
    boxed.style.border_right = true;
    boxed
}

fn first_row(element: &Element, width: u16) -> String {
    render_to_string_no_trim(element, width)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn siblings_tile_their_parent_with_no_gap_and_no_overlap() {
    // Widths that do not divide evenly are the interesting ones: 7 across 3
    // children is 2.33 each, so some cell has to be assigned by the rounding
    // rule rather than by arithmetic.
    for width in 4_u16..=24 {
        for count in 2..=4_usize {
            if usize::from(width) < count {
                continue;
            }
            let row = first_row(&tiled_row(width, count), width);
            let cells: Vec<char> = row.chars().collect();

            assert_eq!(
                cells.len(),
                usize::from(width),
                "width {width} / {count} children: painted {} cells, expected {width}",
                cells.len()
            );
            assert!(
                !cells.contains(&' '),
                "width {width} / {count} children: a cell belongs to no child ({row:?})"
            );

            // Each child owns one contiguous run, and the runs appear in order.
            let mut runs: Vec<char> = Vec::new();
            for cell in &cells {
                if runs.last() != Some(cell) {
                    runs.push(*cell);
                }
            }
            let expected: Vec<char> = (0..count).map(|i| char::from(b'a' + i as u8)).collect();
            assert_eq!(
                runs, expected,
                "width {width} / {count} children: cells are interleaved, so spans overlap ({row:?})"
            );
        }
    }
}

#[test]
fn every_sibling_receives_at_least_one_cell_or_none_at_all() {
    // A child rounded down to zero cells must disappear cleanly rather than
    // paint a partial cell over its neighbour.
    for width in 2_u16..=6 {
        let row = first_row(&tiled_row(width, 4), width);
        let distinct: std::collections::BTreeSet<char> = row.chars().collect();
        assert!(
            !distinct.contains(&' ') || distinct.len() > 1,
            "width {width}: the row is entirely blank ({row:?})"
        );
        for cell in row.chars() {
            assert!(
                cell == ' ' || ('a'..='d').contains(&cell),
                "width {width}: an unexpected cell {cell:?} appeared ({row:?})"
            );
        }
    }
}

#[test]
fn a_border_never_paints_into_its_neighbour() {
    // Two bordered boxes side by side. Each border character must sit inside
    // its own box's span; a single cell of overlap shows up as a doubled or
    // missing corner.
    let element = Box::new()
        .width(20.0)
        .height(3.0)
        .flex_direction(FlexDirection::Row)
        .child(bordered(6, BorderStyle::Single, "ab"))
        .child(bordered(6, BorderStyle::Double, "cd"))
        .into_element();

    let rendered = render_to_string_no_trim(&element, 20);
    let lines: Vec<&str> = rendered.lines().collect();
    assert!(lines.len() >= 3, "expected three rows, got {rendered:?}");

    // Slice by character, not by byte: box-drawing glyphs are multi-byte.
    let cells = |line: &str| line.chars().take(12).collect::<String>();

    assert_eq!(
        cells(lines[0]),
        "┌────┐╔════╗",
        "the two borders do not meet exactly at cell 6"
    );
    assert_eq!(cells(lines[2]), "└────┘╚════╝");
    assert!(
        lines[1].starts_with("│ab  │║cd  ║"),
        "content escaped its bordered box: {:?}",
        lines[1]
    );
}

#[test]
fn content_stays_inside_its_own_border() {
    // Text longer than the content rect must be clipped by the border, not
    // painted over it.
    let mut boxed = Box::new()
        .width(8.0)
        .height(3.0)
        .overflow_x(Overflow::Hidden)
        .child(Text::new("abcdefghijkl"))
        .into_element();
    boxed.style.border_style = BorderStyle::Single;
    boxed.style.border_top = true;
    boxed.style.border_bottom = true;
    boxed.style.border_left = true;
    boxed.style.border_right = true;

    let rendered = render_to_string_no_trim(&boxed, 8);
    for line in rendered.lines() {
        let cells: Vec<char> = line.chars().collect();
        if cells.is_empty() {
            continue;
        }
        assert!(
            matches!(cells[0], '┌' | '│' | '└'),
            "content overwrote the left border: {line:?}"
        );
        if cells.len() >= 8 {
            assert!(
                matches!(cells[7], '┐' | '│' | '┘'),
                "content overwrote the right border: {line:?}"
            );
        }
    }
}

/// Render `text` with a raw horizontal padding, which reaches the quantizer
/// unrounded — the layout backend rounds its own output, so a style value is
/// the shortest path to a fractional coordinate.
fn padded(text: &str, padding_left: f32) -> String {
    let mut padded = Text::new(text).into_element();
    padded.style.padding.left = padding_left;
    let element = Box::new()
        .width(12.0)
        .height(1.0)
        .overflow_x(Overflow::Hidden)
        .child(padded)
        .into_element();
    first_row(&element, 12).trim_end().to_owned()
}

#[test]
fn a_fractional_offset_lands_in_the_cell_that_contains_it() {
    // floor, not round and not truncate: every offset in [n, n+1) lands on n.
    assert_eq!(padded("XYZ", 0.0), "XYZ");
    assert_eq!(padded("XYZ", 0.5), "XYZ");
    assert_eq!(padded("XYZ", 0.9), "XYZ");
    assert_eq!(padded("XYZ", 1.0), " XYZ");
    assert_eq!(padded("XYZ", 1.5), " XYZ");
    assert_eq!(padded("XYZ", 2.9), "  XYZ");
}

#[test]
fn a_negative_fractional_offset_stays_negative_and_clips() {
    // Truncation toward zero would fold every offset in (-1.0, 0.0) onto cell
    // 0 and paint content the caller placed off-screen along the edge.
    assert_eq!(padded("XYZ", -0.5), "YZ");
    assert_eq!(padded("XYZ", -0.9), "YZ");
    assert_eq!(padded("XYZ", -1.0), "YZ");
    assert_eq!(padded("XYZ", -1.5), "Z");
}

#[test]
fn a_non_finite_or_overflowing_offset_is_a_typed_error_not_a_guess() {
    use rnk::renderer::{TextCoordinateError, TextRenderError, try_render_to_string};

    for (name, padding, expected) in [
        ("NaN", f32::NAN, TextCoordinateError::NonFinite),
        ("+inf", f32::INFINITY, TextCoordinateError::NonFinite),
        ("-inf", f32::NEG_INFINITY, TextCoordinateError::NonFinite),
        ("f32::MAX", f32::MAX, TextCoordinateError::Overflow),
    ] {
        let mut child = Text::new("XYZ").into_element();
        child.style.padding.left = padding;
        let element = Box::new().width(12.0).child(child).into_element();

        match try_render_to_string(&element, 12) {
            Err(TextRenderError::Coordinate { source, .. }) => assert_eq!(
                source, expected,
                "{name}: reported the wrong coordinate failure"
            ),
            other => panic!("{name}: expected a typed coordinate failure, got {other:?}"),
        }
    }
}

#[test]
fn a_nested_box_cannot_paint_outside_its_ancestor() {
    // A child wider than its clipping ancestor must be cut at the ancestor's
    // edge, whatever its own geometry says.
    let element = Box::new()
        .width(20.0)
        .height(1.0)
        .flex_direction(FlexDirection::Row)
        .child(
            Box::new()
                .width(5.0)
                .height(1.0)
                .overflow_x(Overflow::Hidden)
                .child(
                    Box::new()
                        .width(15.0)
                        .flex_shrink(0.0)
                        .child(Text::new("aaaaaaaaaaaaaaa")),
                ),
        )
        .child(Box::new().width(5.0).height(1.0).child(Text::new("bbbbb")))
        .into_element();

    let row = first_row(&element, 20);
    let cells: Vec<char> = row.chars().collect();

    assert_eq!(
        cells[..5].iter().collect::<String>(),
        "aaaaa",
        "the clipped child did not fill its own span: {row:?}"
    );
    assert_eq!(
        cells[5..10].iter().collect::<String>(),
        "bbbbb",
        "the clipped child painted past its ancestor and over its sibling: {row:?}"
    );
}

#[test]
fn a_zero_or_one_cell_box_has_a_definite_span() {
    // The degenerate widths are where an off-by-one rounding rule shows up
    // first: a zero-width box must claim nothing, a one-cell box exactly one.
    let zero = Box::new()
        .width(0.0)
        .height(1.0)
        .child(Text::new("x"))
        .into_element();
    assert_eq!(first_row(&zero, 4).trim_end(), "");

    let one = Box::new()
        .width(1.0)
        .height(1.0)
        .overflow_x(Overflow::Hidden)
        .child(Text::new("xy"))
        .into_element();
    assert_eq!(first_row(&one, 4).trim_end(), "x");
}
