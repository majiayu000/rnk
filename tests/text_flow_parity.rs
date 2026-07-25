//! GH-58: layout height and rendered rows must describe the same text.
//!
//! Before the shared flow, a wrapped block reserved N rows in layout but the
//! renderer wrote only what fit before the first break, so everything after it
//! vanished with no error.

use rnk::layout::LayoutEngine;
use rnk::prelude::*;

/// Rows layout reserved, and rows the renderer actually painted.
fn measure_and_render(text: &str, width: u16, wrap: TextWrap) -> (usize, Vec<String>) {
    let element = Box::new()
        .width(width as f32)
        .child(Text::new(text).wrap(wrap))
        .into_element();

    let mut engine = LayoutEngine::new();
    engine.compute(&element, width, 40);
    let reserved = engine
        .get_layout(element.id)
        .map(|layout| layout.height as usize)
        .unwrap_or(0);

    let painted = render_to_string_no_trim(&element, width)
        .lines()
        .map(str::to_owned)
        .filter(|row| !row.trim().is_empty())
        .collect();

    (reserved, painted)
}

#[test]
fn measure_rows_equal_rendered_rows() {
    let (reserved, painted) = measure_and_render("aaaa bbbb cccc dddd", 10, TextWrap::Wrap);
    assert_eq!(reserved, 2, "layout should reserve two rows");
    assert_eq!(painted, vec!["aaaa bbbb", "cccc dddd"]);
}

#[test]
fn wrapped_content_past_the_first_row_is_not_dropped() {
    let text = "the quick brown fox jumps over the lazy dog";
    let (_, painted) = measure_and_render(text, 12, TextWrap::Wrap);

    let rendered: String = painted.concat().split_whitespace().collect();
    let original: String = text.split_whitespace().collect();
    assert_eq!(rendered, original, "wrapping lost content: {painted:?}");
}

#[test]
fn a_word_longer_than_the_box_wraps_instead_of_truncating() {
    let (reserved, painted) = measure_and_render("abcdefghijkl", 6, TextWrap::Wrap);
    assert_eq!(reserved, 2);
    assert_eq!(painted, vec!["abcdef", "ghijkl"]);
}

#[test]
fn wide_graphemes_stay_intact_across_the_wrap_point() {
    let (_, painted) = measure_and_render("你好世界", 6, TextWrap::Wrap);
    assert_eq!(painted.concat(), "你好世界");
    for row in &painted {
        assert!(
            !row.contains('\u{fffd}'),
            "a wide grapheme was split: {painted:?}"
        );
    }
}

#[test]
fn explicit_newlines_still_render_every_line() {
    let (reserved, painted) = measure_and_render("aaa\nbbb\nccc", 10, TextWrap::Wrap);
    assert_eq!(reserved, 3);
    assert_eq!(painted, vec!["aaa", "bbb", "ccc"]);
}

#[test]
fn reserved_and_painted_rows_agree_across_widths() {
    let text = "alpha beta gamma delta epsilon";
    for width in 6..=30 {
        let (reserved, painted) = measure_and_render(text, width, TextWrap::Wrap);
        assert_eq!(
            reserved,
            painted.len(),
            "width {width}: reserved {reserved} rows but painted {} ({painted:?})",
            painted.len()
        );
    }
}
