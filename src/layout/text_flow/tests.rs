use unicode_segmentation::UnicodeSegmentation;

use super::*;
use crate::core::{Color, Overflow, Style};
use crate::layout::measure::measure_text_width;

fn flow_rows(text: &str, width: usize, wrap: TextWrap) -> Vec<String> {
    flow_text(text, width, wrap).rows().to_vec()
}

#[test]
fn canonical_tokens_preserve_exact_source_and_safe_payload() {
    let source = "e\u{301}\r\n\t\u{1b}";
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new());
    let flow = TextFlow::try_build(&input, &TextFlowOptions::new(8, TextWrap::Wrap))
        .expect("valid exact source should produce an atomic TextFlow");

    assert_eq!(flow.tokens().len(), 4);
    assert_eq!(
        flow.tokens()
            .iter()
            .filter_map(TextFlowToken::source_range)
            .collect::<Vec<_>>(),
        vec![0..3, 3..5, 5..6, 6..7]
    );
    assert_eq!(flow.tokens()[0].safe_text(), "e\u{301}");
    assert_eq!(
        flow.tokens()[1].placement(),
        &TextFlowPlacement::HardBreak { row: 0 }
    );
    assert_eq!(flow.tokens()[2].safe_text(), "    ");
    assert_eq!(flow.tokens()[3].safe_text(), "␛");
    assert_eq!(
        flow.tokens()[3].placement(),
        &TextFlowPlacement::SanitizedControl { row: 1, column: 4 }
    );
}

fn plain_input(source: &str) -> TextFlowInput {
    TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
}

fn build(source: &str, width: usize, wrap: TextWrap) -> TextFlow {
    TextFlow::try_build(&plain_input(source), &TextFlowOptions::new(width, wrap)).unwrap()
}

#[test]
fn text_flow_shared_result() {
    let flow = build("ab cd", 3, TextWrap::Wrap);
    assert_eq!(flow.rows(), &["ab".to_string(), "cd".to_string()]);
    assert_eq!(
        flow.logical_rows
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        ["ab", "cd"]
    );
    for row in &flow.logical_rows {
        for run in &row.runs {
            assert_eq!(run.text, flow.tokens[run.token_index].safe_text);
            assert_eq!(run.row, row.index);
        }
    }

    let mut synthetic = TextFlowToken {
        source: TextFlowSource::Synthetic,
        safe_text: "…".to_string(),
        style: Style::new(),
        display_width: 1,
        placement: TextFlowPlacement::Omitted,
        class: TokenClass::Content,
    };
    let mut synthetic_rows = Vec::new();
    place_row(
        std::slice::from_mut(&mut synthetic),
        &[0],
        4,
        &mut synthetic_rows,
    )
    .unwrap();
    let map = build_position_map(std::slice::from_ref(&synthetic));
    assert_eq!(synthetic.source_range(), None);
    assert_eq!(
        synthetic.placement,
        TextFlowPlacement::Synthetic { row: 0, column: 0 }
    );
    assert_eq!(map[0].source, TextFlowSource::Synthetic);
}

#[test]
fn text_flow_cache_invalidation() {
    let input = plain_input("cache");
    let options = TextFlowOptions::new(8, TextWrap::Wrap);
    let mut cache = TextFlowCache::default();
    cache.get_or_compute(&input, &options).unwrap();

    let mut inputs = Vec::new();
    inputs.push(plain_input("changed"));
    let mut styled = plain_input("cache");
    let mut bold = Style::new();
    bold.bold = true;
    styled.styled_ranges = vec![StyledTextRange {
        range: 0..5,
        style: bold,
    }];
    inputs.push(styled);
    for changed in inputs {
        let before = cache.build_count;
        cache.get_or_compute(&changed, &options).unwrap();
        assert_eq!(cache.build_count, before + 1);
    }

    let mut variants = Vec::new();
    let mut changed = options.clone();
    changed.max_width = 9;
    variants.push(changed);
    let mut changed = options.clone();
    changed.text_wrap = TextWrap::Truncate;
    variants.push(changed);
    let mut changed = options.clone();
    changed.overflow_x = Overflow::Hidden;
    variants.push(changed);
    let mut changed = options.clone();
    changed.overflow_y = Overflow::Scroll;
    variants.push(changed);
    let mut changed = options.clone();
    changed.tab_stop = 8;
    variants.push(changed);
    let mut changed = options.clone();
    changed.ellipsis = "...".to_string();
    variants.push(changed);
    let mut changed = options.clone();
    changed.width_policy.revision = 2;
    variants.push(changed);

    for changed in variants {
        let before = cache.build_count;
        cache.get_or_compute(&input, &changed).unwrap();
        assert_eq!(cache.build_count, before + 1);
    }
}

#[test]
fn text_flow_cache_reuse() {
    let input = plain_input("same");
    let options = TextFlowOptions::new(4, TextWrap::Wrap);
    let mut cache = TextFlowCache::default();
    let first = cache.get_or_compute(&input, &options).unwrap();
    let second = cache.get_or_compute(&input, &options).unwrap();
    let cold = TextFlow::try_build(&input, &options).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(cache.build_count, 1);
    assert_eq!(*first, cold);
}

#[test]
fn text_flow_styled_runs() {
    let mut red = Style::new();
    red.color = Some(Color::Red);
    let mut blue = Style::new();
    blue.color = Some(Color::Blue);
    let input = plain_input("ab界").with_styled_ranges(vec![
        StyledTextRange {
            range: 0..2,
            style: red.clone(),
        },
        StyledTextRange {
            range: 2..5,
            style: blue.clone(),
        },
    ]);
    let flow = TextFlow::try_build(&input, &TextFlowOptions::new(8, TextWrap::Wrap)).unwrap();
    assert_eq!(flow.tokens[0].style, red);
    assert_eq!(flow.tokens[2].style, blue);
    assert_eq!(flow.logical_rows[0].runs[2].text, "界");
}

#[test]
fn text_flow_empty_inputs() {
    let flow = build("", 0, TextWrap::Wrap);
    assert_eq!(flow.rows(), &["".to_string()]);
    assert_eq!(flow.row_count(), 1);
    assert!(flow.tokens.is_empty());
    assert!(flow.position_map.is_empty());
}

#[test]
fn text_flow_graphemes() {
    let source = "e\u{301}👨‍👩‍👧‍👦️";
    let flow = build(source, 2, TextWrap::Wrap);
    assert_eq!(flow.tokens.len(), 2);
    assert_eq!(flow.tokens[0].source_range(), Some(0..3));
    assert_eq!(flow.tokens[0].safe_text, "e\u{301}");
    assert!(
        flow.tokens
            .iter()
            .all(|token| token.safe_text.graphemes(true).count() == 1)
    );
}

#[test]
fn split_combining_and_zwj_style_boundary_normalizes() {
    let family = "👨‍👩‍👧‍👦";
    let source = format!("e\u{301}{family}");
    let family_boundary = 3 + "👨".len();
    let mut first = Style::new();
    first.bold = true;
    let mut later = Style::new();
    later.italic = true;
    let input = TextFlowInput::plain(&source, TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: 0..1,
                style: first.clone(),
            },
            StyledTextRange {
                range: 1..family_boundary,
                style: later.clone(),
            },
            StyledTextRange {
                range: family_boundary..source.len(),
                style: Style::new(),
            },
        ]);
    let flow = TextFlow::try_build(&input, &TextFlowOptions::new(20, TextWrap::Wrap)).unwrap();
    assert_eq!(flow.tokens.len(), 2);
    assert_eq!(flow.tokens[0].style, first);
    assert_eq!(flow.tokens[1].style, later);
    assert!(flow.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        TextFlowDiagnostic::StyleBoundaryNormalized { boundary: 1, .. }
    )));
    assert!(flow.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        TextFlowDiagnostic::StyleBoundaryNormalized { boundary, .. }
            if *boundary == family_boundary
    )));
}

#[test]
fn finalized_non_grapheme_range_is_error() {
    let source = "e\u{301}";
    assert_eq!(validate_finalized_range(source, &(0..3)), Ok(()));
    assert_eq!(
        validate_finalized_range(source, &(0..1)),
        Err(TextFlowError::FinalizedRangeNotGraphemeBoundary { range: 0..1 })
    );
    assert_eq!(
        validate_finalized_range(source, &(1..3)),
        Err(TextFlowError::FinalizedRangeNotGraphemeBoundary { range: 1..3 })
    );

    for range in [2..2, Range { start: 1, end: 0 }] {
        let input = plain_input("a").with_styled_ranges(vec![StyledTextRange {
            range: range.clone(),
            style: Style::new(),
        }]);
        assert_eq!(
            TextFlow::try_build(&input, &TextFlowOptions::new(1, TextWrap::Wrap)),
            Err(TextFlowError::InvalidStyleRange { range })
        );
    }
}

#[test]
fn text_flow_control_replacement() {
    let source = "\0\u{1b}\u{7f}\u{80}\u{9f}";
    let flow = build(source, 20, TextWrap::Wrap);
    assert_eq!(
        flow.tokens
            .iter()
            .map(|token| token.safe_text.as_str())
            .collect::<String>(),
        "␀␛␡��"
    );
    assert!(
        flow.tokens
            .iter()
            .all(|token| matches!(token.placement, TextFlowPlacement::SanitizedControl { .. }))
    );
    assert_eq!(
        flow.tokens
            .iter()
            .filter_map(TextFlowToken::source_range)
            .collect::<Vec<_>>(),
        [0..1, 1..2, 2..3, 3..5, 5..7]
    );

    let all_controls: String = (0..=0x1f)
        .chain(std::iter::once(0x7f))
        .chain(0x80..=0x9f)
        .map(|value| char::from_u32(value).unwrap())
        .collect();
    let all_flow = build(&all_controls, 200, TextWrap::Wrap);
    let mut covered = 0;
    for token in &all_flow.tokens {
        let range = token.source_range().unwrap();
        assert_eq!(range.start, covered);
        covered = range.end;
        match &all_controls[range] {
            "\n" | "\r" => assert!(matches!(
                token.placement,
                TextFlowPlacement::HardBreak { .. }
            )),
            "\t" => assert!(!token.safe_text.contains('\t')),
            _ => assert!(matches!(
                token.placement,
                TextFlowPlacement::SanitizedControl { .. }
            )),
        }
    }
    assert_eq!(covered, all_controls.len());
}

#[test]
fn text_flow_tabs() {
    for tab_stop in [1, 4, 8] {
        let mut options = TextFlowOptions::new(40, TextWrap::Wrap);
        options.tab_stop = tab_stop;
        let flow = TextFlow::try_build(&plain_input("a\tb"), &options).unwrap();
        let tab = &flow.tokens[1];
        let expected = tab_stop - 1 % tab_stop;
        assert_eq!(tab.source_range(), Some(1..2));
        assert_eq!(tab.display_width, expected);
        assert_eq!(tab.safe_text, " ".repeat(expected));
    }
    let mut invalid = TextFlowOptions::new(8, TextWrap::Wrap);
    invalid.tab_stop = 0;
    assert_eq!(
        TextFlow::try_build(&plain_input("\t"), &invalid),
        Err(TextFlowError::InvalidTabStop)
    );
}

#[test]
fn text_flow_wrap() {
    let flow = build("aaaa bbbb cccc", 6, TextWrap::Wrap);
    assert_eq!(
        flow.rows(),
        &["aaaa".to_string(), "bbbb".to_string(), "cccc".to_string()]
    );
    assert!(matches!(
        flow.tokens[4].placement,
        TextFlowPlacement::Omitted
    ));
    assert_eq!(
        flow.tokens
            .iter()
            .filter_map(TextFlowToken::source_range)
            .fold(0, |end, range| {
                assert_eq!(range.start, end);
                range.end
            }),
        "aaaa bbbb cccc".len()
    );
}

#[test]
fn text_flow_truncate() {
    for wrap in [
        TextWrap::Truncate,
        TextWrap::TruncateStart,
        TextWrap::TruncateMiddle,
        TextWrap::TruncateEnd,
    ] {
        let flow = build("abcdef", 3, wrap);
        assert_eq!(flow.rows(), &["abc".to_string()]);
        assert!(
            flow.tokens[3..]
                .iter()
                .all(|token| token.placement == TextFlowPlacement::Truncated)
        );
        assert!(
            flow.tokens
                .iter()
                .all(|token| token.source != TextFlowSource::Synthetic)
        );
    }
}

#[test]
fn text_flow_narrow_width() {
    let zero = build("ab", 0, TextWrap::Wrap);
    assert_eq!(zero.rows(), &["".to_string()]);
    assert!(
        zero.tokens
            .iter()
            .all(|token| token.placement == TextFlowPlacement::Omitted)
    );

    let narrow = build("界a", 1, TextWrap::Wrap);
    assert_eq!(narrow.rows(), &["界".to_string(), "a".to_string()]);
    assert_eq!(narrow.tokens[0].display_width, 2);
    assert_eq!(
        narrow.tokens[0].placement,
        TextFlowPlacement::Positioned { row: 0, column: 0 }
    );

    let zero_width = build("\u{200d}", 1, TextWrap::Wrap);
    assert!(matches!(
        zero_width.tokens[0].placement,
        TextFlowPlacement::ZeroWidth { .. }
    ));
}

#[test]
fn text_flow_interruption() {
    let input = plain_input("interrupt me");
    let options = TextFlowOptions::new(8, TextWrap::Wrap);
    let mut calls = 0;
    let result = TextFlow::try_build_interruptible(&input, &options, || {
        calls += 1;
        calls == 3
    });
    assert_eq!(result, Err(TextFlowError::Interrupted));

    let mut cache = TextFlowCache::default();
    let published = cache.get_or_compute(&input, &options).unwrap();
    let mut invalid = options.clone();
    invalid.tab_stop = 0;
    assert_eq!(
        cache.get_or_compute(&plain_input("new"), &invalid),
        Err(TextFlowError::InvalidTabStop)
    );
    assert!(Arc::ptr_eq(cache.published.as_ref().unwrap(), &published));
}

#[test]
fn wrapped_text_keeps_every_word() {
    assert_eq!(
        flow_rows("aaaa bbbb cccc dddd", 10, TextWrap::Wrap),
        vec!["aaaa bbbb", "cccc dddd"]
    );
}

#[test]
fn measure_and_render_agree_on_row_count() {
    let flow = flow_text("aaaa bbbb cccc dddd", 10, TextWrap::Wrap);
    assert_eq!(flow.row_count(), flow.rows().len());
    assert_eq!(flow.row_count(), 2);
}

#[test]
fn word_longer_than_row_is_broken_not_dropped() {
    assert_eq!(
        flow_rows("abcdefghijkl", 6, TextWrap::Wrap),
        vec!["abcdef", "ghijkl"]
    );
}

#[test]
fn hard_breaks_split_rows_and_crlf_counts_once() {
    assert_eq!(
        flow_rows("a\r\nb\nc\rd", 10, TextWrap::Wrap),
        vec!["a", "b", "c", "d"]
    );
}

#[test]
fn consecutive_hard_breaks_keep_the_blank_row() {
    assert_eq!(flow_rows("a\n\nb", 10, TextWrap::Wrap), vec!["a", "", "b"]);
}

#[test]
fn trailing_whitespace_survives() {
    assert_eq!(flow_rows("ab ", 10, TextWrap::Wrap), vec!["ab "]);
    assert_eq!(flow_rows("a  ", 10, TextWrap::Wrap), vec!["a  "]);
}

#[test]
fn trailing_break_does_not_add_a_final_row() {
    assert_eq!(flow_rows("a\nb\n", 10, TextWrap::Wrap), vec!["a", "b"]);
}

#[test]
fn empty_text_is_one_empty_row() {
    assert_eq!(flow_rows("", 10, TextWrap::Wrap), vec![""]);
    assert_eq!(flow_text("", 10, TextWrap::Wrap).row_count(), 1);
}

#[test]
fn zero_width_keeps_row_count_but_places_no_cells() {
    let flow = flow_text("a\nb", 0, TextWrap::Wrap);
    assert_eq!(flow.row_count(), 2);
    assert_eq!(flow.max_row_width(), 0);
}

#[test]
fn wide_graphemes_are_never_split_across_rows() {
    assert_eq!(
        flow_rows("你好世界", 6, TextWrap::Wrap),
        vec!["你好世", "界"]
    );
}

#[test]
fn a_wide_grapheme_straddling_the_edge_is_not_half_written() {
    let flow = flow_text("你好世", 5, TextWrap::Wrap);
    assert!(flow.rows().iter().all(|row| measure_text_width(row) <= 5));
    assert_eq!(flow.rows().concat(), "你好世");
}

#[test]
fn truncate_keeps_one_row_per_logical_line() {
    assert_eq!(
        flow_rows("aaaa bbbb cccc", 6, TextWrap::Truncate),
        vec!["aaaa b"]
    );
    assert_eq!(flow_rows("ab\ncd", 6, TextWrap::Truncate), vec!["ab", "cd"]);
}

#[test]
fn rows_exceed_the_limit_only_for_a_single_oversized_grapheme() {
    for text in ["aaaa bbbb cccc dddd", "abcdefghijkl", "你好世界 hello"] {
        for width in 1..=12 {
            let flow = flow_text(text, width, TextWrap::Wrap);
            for row in flow.rows() {
                if measure_text_width(row) <= width {
                    continue;
                }
                assert_eq!(
                    row.graphemes(true).count(),
                    1,
                    "text {text:?} at width {width} overflowed with a multi-grapheme row {row:?}"
                );
            }
        }
    }
}

#[test]
fn wrapping_preserves_all_non_whitespace_content() {
    let text = "the quick brown fox jumps over the lazy dog";
    for width in 3..=20 {
        let flow = flow_text(text, width, TextWrap::Wrap);
        let flowed: String = flow.rows().concat().split_whitespace().collect();
        let original: String = text.split_whitespace().collect();
        assert_eq!(flowed, original, "content lost at width {width}");
    }
}
