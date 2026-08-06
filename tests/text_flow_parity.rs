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
fn unicode_graphemes_render_intact() {
    // One flow decides both the reserved height and the painted cells, so a
    // grapheme cluster must survive that trip whole. Splitting one mid-cluster
    // is what produced replacement characters and stray fragments before.
    let cases = [
        ("CJK", "你好世界"),
        ("emoji", "🙂🙃😀😁"),
        ("ZWJ family", "👨‍👩‍👧‍👦 end"),
        ("combining", "e\u{0301}a\u{0300}o\u{0302}"),
        ("mixed", "hi 你好 🙂 e\u{0301}"),
    ];

    for (name, text) in cases {
        for width in 4..=12 {
            let (reserved, painted) = measure_and_render(text, width, TextWrap::Wrap);
            let joined = painted.concat();

            assert!(
                !joined.contains('\u{fffd}'),
                "{name} at width {width}: a cluster was split into a replacement char: {painted:?}"
            );
            assert_eq!(
                reserved,
                painted.len(),
                "{name} at width {width}: reserved {reserved} rows but painted {} ({painted:?})",
                painted.len()
            );

            let source_clusters: String = text.split_whitespace().collect();
            let painted_clusters: String = joined.split_whitespace().collect();
            assert_eq!(
                painted_clusters, source_clusters,
                "{name} at width {width}: content was lost or reordered ({painted:?})"
            );
        }
    }
}

#[test]
fn source_controls_are_not_terminal_sequences() {
    // Source text is data, never instructions. An ESC, a C0/C1 byte or DEL
    // arriving in user content must be replaced at the Output trust boundary,
    // not forwarded to the terminal where it would move the cursor, clear the
    // screen, or open an OSC.
    let payloads = [
        ("ESC + CSI clear", "safe\u{1b}[2Jtail"),
        ("ESC + cursor home", "safe\u{1b}[Htail"),
        ("OSC open", "safe\u{1b}]0;title\u{7}tail"),
        ("bare C0", "safe\u{7}\u{8}tail"),
        ("DEL", "safe\u{7f}tail"),
        ("C1 CSI", "safe\u{9b}2Jtail"),
    ];

    for (name, payload) in payloads {
        let element = Box::new()
            .width(40.0)
            .child(Text::new(payload))
            .into_element();
        let raw = render_to_string_raw(&element, 40);

        assert!(
            !raw.contains('\u{1b}'),
            "{name}: an ESC from source text reached the terminal stream: {raw:?}"
        );
        assert!(
            !raw.contains('\u{9b}') && !raw.contains('\u{7}') && !raw.contains('\u{7f}'),
            "{name}: a control scalar from source text reached the terminal stream: {raw:?}"
        );
        assert!(
            raw.contains("safe") && raw.contains("tail"),
            "{name}: sanitizing dropped the surrounding text: {raw:?}"
        );
    }
}

#[test]
fn viewport_projection_tracks_overflow_scroll_and_clip() {
    // The two axes are independent: hiding one must not clip the other, and a
    // scroll offset must move content by whole cells in the scrolled axis only.
    let content = || {
        Box::new()
            .flex_direction(FlexDirection::Column)
            .child(Box::new().height(1.0).child(Text::new("row-one-long")))
            .child(Box::new().height(1.0).child(Text::new("row-two-long")))
            .child(Box::new().height(1.0).child(Text::new("row-three")))
    };

    // Vertical scroll with vertical clipping: the first row is scrolled out.
    let scrolled = Box::new()
        .width(12.0)
        .height(2.0)
        .overflow_y(Overflow::Scroll)
        .scroll_offset_y(1)
        .child(content())
        .into_element();
    let painted = render_to_string_no_trim(&scrolled, 12);
    assert!(
        !painted.contains("row-one-long"),
        "a row scrolled out of view was still painted: {painted:?}"
    );
    assert!(
        painted.contains("row-two-long"),
        "the scrolled-to row was not painted: {painted:?}"
    );

    // Hiding the horizontal axis must not truncate rows vertically.
    let horizontal_only = Box::new()
        .width(6.0)
        .height(3.0)
        .overflow_x(Overflow::Hidden)
        .child(content())
        .into_element();
    let painted = render_to_string_no_trim(&horizontal_only, 6);
    let rows = painted.lines().filter(|row| !row.trim().is_empty()).count();
    assert_eq!(
        rows, 3,
        "hiding overflow_x clipped the vertical axis too: {painted:?}"
    );
    for row in painted.lines().filter(|row| !row.trim().is_empty()) {
        assert!(
            row.chars().count() <= 6,
            "overflow_x Hidden let a row exceed the content rect: {row:?}"
        );
    }
}

#[test]
fn overflow_change_recomputes_flow_and_projection() {
    // Overflow is part of the flow's cache identity. Changing only the overflow
    // mode must miss the cache; reusing the previous flow would keep painting
    // the old clip and silently show or hide the wrong cells.
    // The inner box is narrower than its content and narrower than the frame,
    // so the clip is what decides the painted width. A self-truncating wrap
    // mode would settle it inside the flow and never exercise the projection.
    let build = |overflow| {
        Box::new()
            .width(20.0)
            .child(
                Box::new()
                    .width(6.0)
                    .height(1.0)
                    .overflow_x(overflow)
                    .child(
                        Box::new()
                            .width(12.0)
                            .flex_shrink(0.0)
                            // 12 cells of text in a 12-cell box: it never wraps, so
                            // the clip alone decides how much is painted.
                            .child(Text::new("abcdefghijkl").wrap(TextWrap::Wrap)),
                    ),
            )
            .into_element()
    };

    let visible = render_to_string_no_trim(&build(Overflow::Visible), 20);
    let hidden = render_to_string_no_trim(&build(Overflow::Hidden), 20);

    assert_ne!(
        visible.trim_end(),
        hidden.trim_end(),
        "changing overflow_x reused a stale flow: {visible:?}"
    );
    assert!(
        visible.contains("abcdefghijkl"),
        "overflow_x Visible clipped content it should have painted: {visible:?}"
    );
    for row in hidden.lines() {
        assert!(
            row.trim_end().chars().count() <= 6,
            "the reflowed projection still painted past the clip: {row:?}"
        );
    }
}

#[test]
fn resize_reflows_or_reprojects_before_render() {
    // Width is part of the flow's cache identity. After a resize the next frame
    // must be laid out and painted at the new width, with reserved rows and
    // painted rows still agreeing.
    let text = "alpha beta gamma delta";
    let mut previous: Option<(u16, Vec<String>)> = None;

    for width in [30_u16, 12, 7, 20, 30] {
        let (reserved, painted) = measure_and_render(text, width, TextWrap::Wrap);

        assert_eq!(
            reserved,
            painted.len(),
            "width {width}: resize left reserved {reserved} rows against {} painted ({painted:?})",
            painted.len()
        );
        for row in &painted {
            assert!(
                row.chars().count() <= width as usize,
                "width {width}: a row survived from a wider frame: {row:?}"
            );
        }

        let rendered: String = painted.concat().split_whitespace().collect();
        let original: String = text.split_whitespace().collect();
        assert_eq!(rendered, original, "width {width}: resize lost content");

        if let Some((previous_width, previous_painted)) = previous.replace((width, painted.clone()))
        {
            if previous_width != width {
                assert_ne!(
                    previous_painted, painted,
                    "resize from {previous_width} to {width} reused the previous frame"
                );
            }
        }
    }
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
