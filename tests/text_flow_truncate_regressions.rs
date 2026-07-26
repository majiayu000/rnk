use rnk::{
    core::{Style, TextWrap},
    layout::{TextFlow, TextFlowInput, TextFlowOptions, TextFlowPlacement, TextFlowSourceKind},
};

fn build_truncated_flow(
    source: &str,
    width: usize,
    wrap: TextWrap,
    tab_stop: usize,
    ellipsis: &str,
) -> TextFlow {
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new());
    let mut options = TextFlowOptions::new(width, wrap);
    options.tab_stop = tab_stop;
    options.ellipsis = ellipsis.to_string();
    TextFlow::try_build(&input, &options).expect("valid truncation input must build")
}

fn assert_source_dispositions_are_total(flow: &TextFlow, source_tokens: usize) {
    assert_eq!(flow.tokens()[..source_tokens].len(), source_tokens);
    assert!(flow.tokens()[..source_tokens].iter().all(|token| {
        matches!(
            token.placement(),
            TextFlowPlacement::Positioned { .. }
                | TextFlowPlacement::ZeroWidth { .. }
                | TextFlowPlacement::SanitizedControl { .. }
                | TextFlowPlacement::Truncated { .. }
        )
    }));
}

#[test]
fn middle_truncation_measures_tab_suffix_at_its_eventual_column() {
    let cases = [
        (4, 10, "LLLLMabc\td", "LLLL…bc d"),
        (4, 9, "LLLLMabc\td", "LLLL…bc d"),
        (8, 12, "LLLLMabc\td", "LLLLM…c d"),
    ];

    for (tab_stop, width, source, expected) in cases {
        let flow = build_truncated_flow(source, width, TextWrap::TruncateMiddle, tab_stop, "…");
        assert_eq!(flow.rows(), &[expected.to_string()]);
        assert!(flow.logical_rows()[0].width <= width);
        assert!(flow.rows()[0].ends_with('d'), "true right suffix was lost");
        assert_source_dispositions_are_total(&flow, source.chars().count());
    }
}

#[test]
fn tab_ellipsis_is_measured_after_the_retained_prefix() {
    let cases = [
        ("abcdx", 4, 4, TextWrap::Truncate, "abc ", 3, 1),
        ("abcdx", 4, 4, TextWrap::TruncateEnd, "abc ", 3, 1),
        ("abcdx", 4, 4, TextWrap::TruncateStart, "    ", 0, 4),
        ("abcdx", 4, 4, TextWrap::TruncateMiddle, "abc ", 3, 1),
        ("abcdxy", 5, 4, TextWrap::TruncateStart, "    y", 0, 4),
        ("abcdxy", 5, 4, TextWrap::TruncateMiddle, "abc y", 3, 1),
        ("abcdefghx", 8, 8, TextWrap::TruncateEnd, "abcdefg ", 7, 1),
        ("abcdefghx", 8, 8, TextWrap::TruncateStart, "        ", 0, 8),
        (
            "abcdefghxy",
            9,
            8,
            TextWrap::TruncateMiddle,
            "abcdefg y",
            7,
            1,
        ),
    ];

    for (source, width, tab_stop, wrap, expected, ellipsis_column, ellipsis_width) in cases {
        let flow = build_truncated_flow(source, width, wrap, tab_stop, "\t");
        assert_eq!(flow.rows(), &[expected.to_string()]);
        assert!(flow.logical_rows()[0].width <= width);
        let source_tokens = source.chars().count();
        let synthetic = flow.logical_rows()[0]
            .runs
            .iter()
            .find(|run| run.token_index >= source_tokens)
            .expect("tab ellipsis must be placed");
        assert_eq!(
            (synthetic.column, synthetic.width),
            (ellipsis_column, ellipsis_width)
        );
        assert_source_dispositions_are_total(&flow, source_tokens);
    }
}

fn interrupt_calls_for(size: usize, wrap: TextWrap) -> usize {
    let input = TextFlowInput::plain("a".repeat(size), TextFlowSourceKind::Exact, Style::new());
    let options = TextFlowOptions::new(size / 2, wrap);
    let mut calls = 0usize;
    TextFlow::try_build_interruptible(&input, &options, || {
        calls += 1;
        false
    })
    .expect("uncancelled truncation must build");
    calls
}

fn assert_linear_operation_count(wrap: TextWrap) {
    let mut previous = None;
    for size in [1_024usize, 2_048, 4_096, 8_192, 16_384] {
        let calls = interrupt_calls_for(size, wrap);
        assert!(
            calls <= 6 * size + 64,
            "{wrap:?} used {calls} interruptible operations for {size} tokens"
        );
        if let Some((previous_size, previous_calls)) = previous {
            assert!(
                calls * previous_size <= previous_calls * size + size,
                "{wrap:?} operation density grew from {previous_calls}/{previous_size} \
                 to {calls}/{size}"
            );
        }
        previous = Some((size, calls));
    }
}

#[test]
fn compat_truncate_operation_count_is_linear() {
    assert_linear_operation_count(TextWrap::Truncate);
}

#[test]
fn end_truncate_operation_count_is_linear() {
    assert_linear_operation_count(TextWrap::TruncateEnd);
}

#[test]
fn start_truncate_operation_count_is_linear() {
    assert_linear_operation_count(TextWrap::TruncateStart);
}

#[test]
fn middle_truncate_operation_count_is_linear() {
    assert_linear_operation_count(TextWrap::TruncateMiddle);
}
