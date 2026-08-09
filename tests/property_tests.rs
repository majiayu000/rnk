//! Property-based tests for Tink
//!
//! Uses proptest to find edge cases through random input generation.

use proptest::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

use rnk::components::{Box as RnkBox, Text};
use rnk::core::{Dimension, Element, FlexDirection, Style, TextWrap};
use rnk::layout::measure::{measure_text_width, truncate_middle, truncate_start, truncate_text};
use rnk::layout::{
    TextFlow, TextFlowInput, TextFlowOptions, TextFlowPlacement, TextFlowSource, TextFlowSourceKind,
};
use rnk::testing::{TestRenderer, display_width};

// ============================================================================
// Unicode Width Property Tests
// ============================================================================

proptest! {
    /// Unicode width measurement must be consistent
    #[test]
    fn unicode_width_consistency(s in "[ -~]{0,100}") {
        // ASCII characters should each have width 1
        let width = measure_text_width(&s);
        prop_assert_eq!(width, s.len());
    }

    /// CJK characters should have width 2
    #[test]
    fn cjk_width(s in "[一-龥]{1,10}") {
        let width = measure_text_width(&s);
        let char_count = s.chars().count();
        prop_assert_eq!(width, char_count * 2);
    }

    /// Mixed text width should be sum of individual widths
    #[test]
    fn mixed_width_additive(ascii in "[a-z]{1,10}", cjk in "[一-龥]{1,5}") {
        let combined = format!("{}{}", ascii, cjk);
        let combined_width = measure_text_width(&combined);
        let ascii_width = measure_text_width(&ascii);
        let cjk_width = measure_text_width(&cjk);

        prop_assert_eq!(combined_width, ascii_width + cjk_width);
    }

    /// Canonical source ranges cover the exact input once and only on EGC boundaries.
    #[test]
    fn text_flow_logical_source_round_trip(
        scalars in proptest::collection::vec(any::<char>(), 0..80),
        width in 0usize..20
    ) {
        let source: String = scalars.into_iter().collect();
        let input = TextFlowInput::plain(
            source.clone(),
            TextFlowSourceKind::Exact,
            Style::new(),
        );
        let flow = TextFlow::try_build(
            &input,
            &TextFlowOptions::new(width, TextWrap::Wrap),
        ).expect("valid UTF-8 input must produce a complete logical map");

        let grapheme_ranges: Vec<_> = source
            .grapheme_indices(true)
            .map(|(start, grapheme)| start..start + grapheme.len())
            .collect();
        let source_ranges: Vec<_> = flow
            .tokens()
            .iter()
            .filter_map(|token| token.source_range())
            .collect();

        prop_assert_eq!(&source_ranges, &grapheme_ranges);
        let mut covered = 0usize;
        for range in source_ranges {
            prop_assert_eq!(range.start, covered);
            prop_assert!(source.is_char_boundary(range.start));
            prop_assert!(source.is_char_boundary(range.end));
            covered = range.end;
        }
        prop_assert_eq!(covered, source.len());

        prop_assert_eq!(flow.position_map().len(), flow.tokens().len());
        for (token_index, entry) in flow.position_map().iter().enumerate() {
            prop_assert_eq!(entry.token_index, token_index);
            prop_assert_eq!(&entry.source, &flow.tokens()[token_index].source);
            prop_assert_eq!(
                &entry.placement,
                &flow.tokens()[token_index].placement
            );
            if let Some(range) = flow.tokens()[entry.token_index].source_range() {
                prop_assert!(grapheme_ranges.contains(&range));
                let begins_inside_another = grapheme_ranges.iter().any(|other| {
                    other.start < range.start && range.start < other.end
                });
                prop_assert!(!begins_inside_another);
            }
        }

        let expected_non_break: String = source
            .graphemes(true)
            .filter(|grapheme| !matches!(*grapheme, "\n" | "\r" | "\r\n"))
            .collect();
        for wrap_width in 1..=grapheme_ranges.len().max(1) {
            let wrapped = TextFlow::try_build(
                &input,
                &TextFlowOptions::new(wrap_width, TextWrap::Wrap),
            ).expect("every positive width must preserve row-backed source order");
            let mut reconstructed = String::new();
            let mut last_token_index = None;
            for (row_index, row) in wrapped.logical_rows().iter().enumerate() {
                prop_assert_eq!(row.index, row_index);
                prop_assert_eq!(&wrapped.rows()[row_index], &row.text);
                let run_text = row
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>();
                prop_assert_eq!(&run_text, &row.text);
                for run in &row.runs {
                    if let Some(previous) = last_token_index {
                        prop_assert!(previous < run.token_index);
                    }
                    last_token_index = Some(run.token_index);
                    let range = wrapped.tokens()[run.token_index]
                        .source_range()
                        .expect("wrap rows must contain source-backed runs");
                    let source_grapheme = &source[range];
                    prop_assert!(!matches!(source_grapheme, "\n" | "\r" | "\r\n"));
                    reconstructed.push_str(source_grapheme);
                }
            }
            prop_assert_eq!(&reconstructed, &expected_non_break);
        }
    }

    /// Truncation preserves a total source map and never exceeds its cell budget.
    #[test]
    fn text_flow_truncate_map_is_total(
        graphemes in proptest::collection::vec(
            prop_oneof![
                8 => Just("a"),
                4 => Just("b"),
                3 => Just(" "),
                2 => Just("界"),
                1 => Just("e\u{301}"),
                1 => Just("👨‍👩‍👧‍👦"),
                1 => Just("\n"),
            ],
            0..60,
        ),
        width in prop_oneof![Just(0usize), Just(1usize), 2usize..20],
        wrap in prop_oneof![
            Just(TextWrap::Truncate),
            Just(TextWrap::TruncateStart),
            Just(TextWrap::TruncateMiddle),
            Just(TextWrap::TruncateEnd),
        ],
        ellipsis in prop::sample::select(vec!["…", "..", "界", "e\u{301}", ""])
    ) {
        let source = graphemes.concat();
        let input = TextFlowInput::plain(
            source.clone(),
            TextFlowSourceKind::Exact,
            Style::new(),
        );
        let mut options = TextFlowOptions::new(width, wrap);
        options.ellipsis = ellipsis.to_string();
        let flow = TextFlow::try_build(&input, &options)
            .expect("valid UTF-8 truncation must produce a complete logical map");

        let mut logical_lines: Vec<_> = source.split_terminator('\n').collect();
        if logical_lines.is_empty() {
            logical_lines.push("");
        }
        let expected_rows: Vec<_> = logical_lines
            .iter()
            .map(|line| match wrap {
                TextWrap::Truncate | TextWrap::TruncateEnd => {
                    truncate_text(line, width, ellipsis)
                }
                TextWrap::TruncateStart => truncate_start(line, width, ellipsis),
                TextWrap::TruncateMiddle => truncate_middle(line, width, ellipsis),
                TextWrap::Wrap => unreachable!("strategy only generates truncate modes"),
            })
            .collect();
        prop_assert_eq!(flow.rows(), expected_rows.as_slice());
        prop_assert_eq!(flow.logical_rows().len(), expected_rows.len());

        let grapheme_ranges: Vec<_> = source
            .grapheme_indices(true)
            .map(|(start, grapheme)| start..start + grapheme.len())
            .collect();
        let source_ranges: Vec<_> = flow
            .tokens()
            .iter()
            .filter_map(|token| token.source_range())
            .collect();
        prop_assert_eq!(source_ranges, grapheme_ranges);
        prop_assert_eq!(flow.position_map().len(), flow.tokens().len());

        let mut run_counts = vec![0usize; flow.tokens().len()];
        let mut last_source_token = None;
        for (row_index, row) in flow.logical_rows().iter().enumerate() {
            prop_assert_eq!(row.index, row_index);
            prop_assert!(row.width <= width);
            let mut column = 0usize;
            let mut reconstructed = String::new();
            let mut source_before_synthetic = false;
            let mut synthetic_seen = false;
            let mut source_after_synthetic = false;

            for run in &row.runs {
                prop_assert!(run.token_index < flow.tokens().len());
                prop_assert_eq!(run.row, row_index);
                prop_assert_eq!(run.column, column);
                prop_assert_eq!(measure_text_width(&run.text), run.width);
                let token = &flow.tokens()[run.token_index];
                prop_assert_eq!(run.text.as_str(), token.safe_text());
                let placement_matches_run = match token.placement() {
                    TextFlowPlacement::Positioned { row, column }
                    | TextFlowPlacement::ZeroWidth { row, column }
                    | TextFlowPlacement::SanitizedControl { row, column }
                    | TextFlowPlacement::Synthetic { row, column } => {
                        *row == row_index && *column == run.column
                    }
                    _ => false,
                };
                prop_assert!(placement_matches_run);

                run_counts[run.token_index] += 1;
                prop_assert_eq!(run_counts[run.token_index], 1);
                match token.source {
                    TextFlowSource::Source { .. } => {
                        let range = token
                            .source_range()
                            .expect("source-backed run must retain its range");
                        prop_assert_eq!(&source[range], &run.text);
                        if synthetic_seen {
                            source_after_synthetic = true;
                        } else {
                            source_before_synthetic = true;
                        }
                        if let Some(previous) = last_source_token {
                            prop_assert!(previous < run.token_index);
                        }
                        last_source_token = Some(run.token_index);
                    }
                    TextFlowSource::Synthetic => {
                        prop_assert_eq!(token.source_range(), None);
                        prop_assert!(!source_after_synthetic);
                        synthetic_seen = true;
                    }
                }
                reconstructed.push_str(&run.text);
                column += run.width;
                prop_assert!(column <= width);
            }

            if synthetic_seen {
                match wrap {
                    TextWrap::Truncate | TextWrap::TruncateEnd => {
                        prop_assert!(!source_after_synthetic);
                    }
                    TextWrap::TruncateStart => {
                        prop_assert!(!source_before_synthetic);
                    }
                    TextWrap::TruncateMiddle => {}
                    TextWrap::Wrap => unreachable!("strategy only generates truncate modes"),
                }
            }
            prop_assert_eq!(&reconstructed, &row.text);
            prop_assert_eq!(column, row.width);
            prop_assert_eq!(measure_text_width(&row.text), row.width);
        }

        for (token_index, token) in flow.tokens().iter().enumerate() {
            prop_assert_eq!(flow.position_map()[token_index].token_index, token_index);
            prop_assert_eq!(&flow.position_map()[token_index].source, &token.source);
            prop_assert_eq!(
                &flow.position_map()[token_index].placement,
                token.placement()
            );
            match token.source {
                TextFlowSource::Synthetic => {
                    prop_assert_eq!(token.source_range(), None);
                    prop_assert_eq!(run_counts[token_index], 1);
                    let synthetic_placed = matches!(
                        token.placement(),
                        TextFlowPlacement::Synthetic { row, .. }
                            if *row < flow.logical_rows().len()
                    );
                    prop_assert!(synthetic_placed);
                }
                TextFlowSource::Source { .. } if run_counts[token_index] == 1 => {
                    let range = token
                        .source_range()
                        .expect("source token must retain its range");
                    let expected_row = source[..range.start].matches('\n').count();
                    let source_placed = matches!(
                        token.placement(),
                        TextFlowPlacement::Positioned { row, .. }
                            | TextFlowPlacement::ZeroWidth { row, .. }
                            | TextFlowPlacement::SanitizedControl { row, .. }
                            if *row == expected_row
                    );
                    prop_assert!(source_placed);
                }
                TextFlowSource::Source { .. } => {
                    let range = token
                        .source_range()
                        .expect("source token must retain its range");
                    let expected_row = source[..range.start].matches('\n').count();
                    let source_absent_with_row = match token.placement() {
                        TextFlowPlacement::HardBreak { row } => *row == expected_row,
                        TextFlowPlacement::Omitted { row } if width == 0 => {
                            *row == expected_row
                        }
                        TextFlowPlacement::Truncated { row } if width > 0 => {
                            *row == expected_row
                        }
                        _ => false,
                    };
                    prop_assert!(source_absent_with_row);
                }
            }
        }
    }
}

// ============================================================================
// Layout Property Tests
// ============================================================================

proptest! {
    /// Layout dimensions must be non-negative
    #[test]
    fn layout_dimensions_non_negative(
        width in 1u16..200,
        height in 1u16..100
    ) {
        let element = RnkBox::new()
            .width(Dimension::Points(width as f32))
            .height(Dimension::Points(height as f32))
            .into_element();

        let renderer = TestRenderer::new(500, 200);
        let layout = renderer.get_layout(&element).unwrap();

        prop_assert!(layout.width >= 0.0, "Width should be non-negative");
        prop_assert!(layout.height >= 0.0, "Height should be non-negative");
        prop_assert!(layout.x >= 0.0, "X should be non-negative");
        prop_assert!(layout.y >= 0.0, "Y should be non-negative");
    }

    /// Layout validation should pass for valid elements
    #[test]
    fn layout_validation_valid_elements(
        width in 10u16..100,
        height in 5u16..50,
        term_width in 100u16..500,
        term_height in 50u16..200
    ) {
        prop_assume!(width < term_width);
        prop_assume!(height < term_height);

        let element = RnkBox::new()
            .width(Dimension::Points(width as f32))
            .height(Dimension::Points(height as f32))
            .into_element();

        let renderer = TestRenderer::new(term_width, term_height);
        let result = renderer.validate_layout(&element);

        prop_assert!(result.is_ok(), "Layout should be valid: {:?}", result);
    }

    /// Nested boxes should have valid child positions
    #[test]
    fn nested_boxes_valid(depth in 1usize..5) {
        fn create_nested(depth: usize) -> Element {
            if depth == 0 {
                return Text::new("leaf").into_element();
            }
            RnkBox::new()
                .padding(1)
                .child(create_nested(depth - 1))
                .into_element()
        }

        let element = create_nested(depth);
        let renderer = TestRenderer::new(100, 50);
        let result = renderer.validate_layout(&element);

        prop_assert!(result.is_ok(), "Nested layout should be valid: {:?}", result);
    }
}

// ============================================================================
// Rendering Property Tests
// ============================================================================

proptest! {
    /// Text should appear in render output
    #[test]
    fn text_appears_in_output(s in "[a-zA-Z0-9]{1,30}") {
        let element = Text::new(&s).into_element();
        let renderer = TestRenderer::new(80, 24);
        let output = renderer.render_to_plain(&element);

        prop_assert!(
            output.contains(&s),
            "Output should contain text '{}', got: {}",
            s, output
        );
    }

    /// Render output should not exceed terminal dimensions
    #[test]
    fn render_within_bounds(
        term_width in 40u16..200,
        term_height in 10u16..100
    ) {
        let element = RnkBox::new()
            .width(Dimension::Percent(100.0))
            .height(Dimension::Percent(100.0))
            .child(Text::new("Content").into_element())
            .into_element();

        let renderer = TestRenderer::new(term_width, term_height);
        let output = renderer.render_to_plain(&element);

        for line in output.lines() {
            let line_width = display_width(line);
            prop_assert!(
                line_width <= term_width as usize,
                "Line width {} exceeds terminal width {}",
                line_width, term_width
            );
        }

        let line_count = output.lines().count();
        prop_assert!(
            line_count <= term_height as usize,
            "Line count {} exceeds terminal height {}",
            line_count, term_height
        );
    }
}

// ============================================================================
// Component Property Tests
// ============================================================================

proptest! {
    /// Box children should be in correct order
    #[test]
    fn box_children_order(children_count in 1usize..5) {
        let texts: Vec<String> = (0..children_count)
            .map(|i| format!("child{}", i))
            .collect();

        let mut builder = RnkBox::new()
            .flex_direction(FlexDirection::Column);

        for text in &texts {
            builder = builder.child(Text::new(text).into_element());
        }

        let element = builder.into_element();

        prop_assert_eq!(element.children.len(), children_count);

        for (i, child) in element.children.iter().enumerate() {
            let expected = format!("child{}", i);
            prop_assert_eq!(
                child.text_content.as_deref(),
                Some(expected.as_str()),
                "Child {} should have text '{}'",
                i, expected
            );
        }
    }

    /// Text styling should be preserved
    #[test]
    fn text_styling_preserved(
        use_bold in any::<bool>(),
        use_italic in any::<bool>(),
        use_underline in any::<bool>()
    ) {
        let mut text = Text::new("styled");

        if use_bold {
            text = text.bold();
        }
        if use_italic {
            text = text.italic();
        }
        if use_underline {
            text = text.underline();
        }

        let element = text.into_element();

        prop_assert_eq!(element.style.bold, use_bold);
        prop_assert_eq!(element.style.italic, use_italic);
        prop_assert_eq!(element.style.underline, use_underline);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

proptest! {
    /// Empty strings should not crash
    #[test]
    fn empty_string_safe(_dummy in Just(())) {
        let element = Text::new("").into_element();
        let renderer = TestRenderer::new(80, 24);

        // Should not panic
        let _ = renderer.render_to_plain(&element);
        let _ = renderer.get_layout(&element);
    }

    /// Very long strings should not crash
    #[test]
    fn long_string_safe(s in ".{100,500}") {
        let element = Text::new(&s).into_element();
        let renderer = TestRenderer::new(80, 24);

        // Should not panic
        let _ = renderer.render_to_plain(&element);
        let _ = renderer.get_layout(&element);
    }

    /// Zero dimensions should be handled
    #[test]
    fn zero_dimensions_safe(_dummy in Just(())) {
        let element = RnkBox::new()
            .width(Dimension::Points(0.0))
            .height(Dimension::Points(0.0))
            .into_element();

        let renderer = TestRenderer::new(80, 24);

        // Should not panic
        let _ = renderer.render_to_plain(&element);
        let result = renderer.validate_layout(&element);
        prop_assert!(result.is_ok());
    }

    /// Minimal terminal size should work
    #[test]
    fn minimal_terminal_safe(_dummy in Just(())) {
        let element = Text::new("x").into_element();
        let renderer = TestRenderer::new(1, 1);

        // Should not panic
        let _ = renderer.render_to_plain(&element);
    }
}
