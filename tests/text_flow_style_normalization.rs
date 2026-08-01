use std::sync::Arc;

use rnk::{
    core::{Color, Style, TextWrap},
    layout::{
        StyledTextRange, TextFlow, TextFlowCache, TextFlowDiagnostic, TextFlowError, TextFlowInput,
        TextFlowOptions, TextFlowSourceKind,
    },
};

fn color(value: Color) -> Style {
    let mut style = Style::new();
    style.color = Some(value);
    style
}

fn options() -> TextFlowOptions {
    TextFlowOptions::new(40, TextWrap::Wrap)
}

fn split_fixture(source: &str, boundary: usize) -> (TextFlowInput, Style, Style) {
    let first = color(Color::Red);
    let later = color(Color::Blue);
    (
        TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new()).with_styled_ranges(
            vec![
                StyledTextRange {
                    range: 0..boundary,
                    style: first.clone(),
                },
                StyledTextRange {
                    range: boundary..source.len(),
                    style: later.clone(),
                },
            ],
        ),
        first,
        later,
    )
}

fn empty_range_input(count: usize) -> TextFlowInput {
    TextFlowInput::plain("", TextFlowSourceKind::Exact, Style::new()).with_styled_ranges(
        (0..count)
            .map(|_| StyledTextRange {
                range: 0..0,
                style: Style::new(),
            })
            .collect(),
    )
}

#[test]
fn public_styled_flow_preserves_first_source_style_and_diagnostics() {
    for (source, boundary) in [("e\u{301}", 1), ("👩‍💻", "👩".len())] {
        let (input, first, _) = split_fixture(source, boundary);
        let flow = TextFlow::try_build(&input, &options()).unwrap();
        assert_eq!(flow.tokens().len(), 1);
        assert_eq!(flow.tokens()[0].style, first);
        assert_eq!(flow.tokens()[0].source_range(), Some(0..source.len()));
        assert_eq!(
            flow.diagnostics(),
            &[
                TextFlowDiagnostic::StyleBoundaryNormalized {
                    boundary,
                    grapheme_range: 0..source.len(),
                },
                TextFlowDiagnostic::StyleBoundaryNormalized {
                    boundary,
                    grapheme_range: 0..source.len(),
                },
            ]
        );
        assert_eq!(flow.cache_identity().input, input);
    }
}

#[test]
fn public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges() {
    let source = "👩‍💻xy";
    let zwj_end = "👩‍💻".len();
    let woman_end = "👩".len();
    let joiner_end = woman_end + "\u{200d}".len();
    let first_source = color(Color::Red);
    let x_style = color(Color::Blue);
    let mut default_style = Style::new();
    default_style.bold = true;
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, default_style.clone())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: zwj_end..zwj_end + 1,
                style: x_style.clone(),
            },
            StyledTextRange {
                range: joiner_end..zwj_end,
                style: Style::new(),
            },
            StyledTextRange {
                range: 0..woman_end,
                style: first_source.clone(),
            },
            StyledTextRange {
                range: joiner_end..joiner_end,
                style: color(Color::Green),
            },
            StyledTextRange {
                range: woman_end..joiner_end,
                style: Style::new(),
            },
        ]);
    let flow = TextFlow::try_build(&input, &options()).unwrap();
    assert_eq!(flow.tokens()[0].style, first_source);
    assert_eq!(flow.tokens()[1].style, x_style);
    assert_eq!(flow.tokens()[2].style, default_style);
    assert_eq!(
        flow.diagnostics()
            .iter()
            .map(|diagnostic| match diagnostic {
                TextFlowDiagnostic::StyleBoundaryNormalized { boundary, .. } => *boundary,
            })
            .collect::<Vec<_>>(),
        vec![
            joiner_end, woman_end, joiner_end, joiner_end, woman_end, joiner_end
        ]
    );
    assert_eq!(
        flow.cache_identity().input.styled_ranges,
        input.styled_ranges
    );

    let empty = empty_range_input(1);
    let empty_flow = TextFlow::try_build(&empty, &options()).unwrap();
    assert_eq!(empty_flow.rows(), &[String::new()]);
    assert!(empty_flow.tokens().is_empty());
    assert!(empty_flow.diagnostics().is_empty());
    assert_eq!(empty_flow.cache_identity().input, empty);
}

#[test]
fn public_styled_flow_preserves_typed_failures() {
    for (source, range) in [
        ("a", 2..2),
        ("a", std::ops::Range { start: 1, end: 0 }),
        ("é", 1..2),
        ("é", 0..1),
        ("a", 0..usize::MAX),
        ("a", usize::MAX..usize::MAX),
    ] {
        let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
            .with_styled_ranges(vec![StyledTextRange {
                range: range.clone(),
                style: Style::new(),
            }]);
        let result = std::panic::catch_unwind(|| TextFlow::try_build(&input, &options()));
        assert_eq!(
            result.expect("invalid styled range must not panic"),
            Err(TextFlowError::InvalidStyleRange { range })
        );
    }

    let overlap = TextFlowInput::plain("abcd", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: 2..4,
                style: Style::new(),
            },
            StyledTextRange {
                range: 0..3,
                style: Style::new(),
            },
        ]);
    assert_eq!(
        TextFlow::try_build(&overlap, &options()),
        Err(TextFlowError::OverlappingStyleRanges {
            first: 0..3,
            second: 2..4,
        })
    );
}

#[test]
fn public_styled_flow_preserves_complete_flow_identity() {
    let source = "e\u{301}\t界\nx";
    let first = color(Color::Red);
    let second = color(Color::Blue);
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: 0..1,
                style: first.clone(),
            },
            StyledTextRange {
                range: 1..source.len(),
                style: second,
            },
        ]);
    let direct = TextFlow::try_build(&input, &options()).unwrap();
    let repeated = TextFlow::try_build(&input, &options()).unwrap();
    assert_eq!(direct, repeated);
    assert_eq!(
        direct.cache_identity().input.source.as_bytes(),
        source.as_bytes()
    );
    assert_eq!(direct.tokens()[0].style, first);
    assert_eq!(direct.tokens()[0].safe_text(), "e\u{301}");
    assert_eq!(direct.tokens().len(), direct.position_map().len());
    for (index, entry) in direct.position_map().iter().enumerate() {
        assert_eq!(entry.token_index, index);
        assert_eq!(entry.source, direct.tokens()[index].source);
        assert_eq!(entry.placement, *direct.tokens()[index].placement());
    }
    let reconstructed = direct
        .tokens()
        .iter()
        .filter_map(|token| token.source_range())
        .map(|range| &source[range])
        .collect::<String>();
    assert_eq!(reconstructed, source);
}

#[test]
fn public_styled_flow_preserves_exact_cache_identity() {
    let (baseline, _, _) = split_fixture("ab", 1);
    let options = options();
    let mut cache = TextFlowCache::default();
    let first = cache.get_or_compute(&baseline, &options).unwrap();
    let second = cache.get_or_compute(&baseline, &options).unwrap();
    assert!(Arc::ptr_eq(&first, &second));

    let assert_miss = |changed_input: TextFlowInput, changed_options: TextFlowOptions| {
        let mut isolated = TextFlowCache::default();
        let original = isolated.get_or_compute(&baseline, &options).unwrap();
        let changed = isolated
            .get_or_compute(&changed_input, &changed_options)
            .unwrap();
        assert!(!Arc::ptr_eq(&original, &changed));
        assert_eq!(changed.cache_identity().input, changed_input);
        assert_eq!(changed.cache_identity().options, changed_options);
    };

    let mut reversed = baseline.clone();
    reversed.styled_ranges.reverse();
    assert_miss(reversed, options.clone());
    let mut changed_style = baseline.clone();
    changed_style.styled_ranges[0].style.bold = true;
    assert_miss(changed_style, options.clone());
    let mut changed_endpoint = baseline.clone();
    changed_endpoint.styled_ranges.insert(
        0,
        StyledTextRange {
            range: 0..0,
            style: Style::new(),
        },
    );
    assert_miss(changed_endpoint, options.clone());
    let mut changed_kind = baseline.clone();
    changed_kind.source_kind = TextFlowSourceKind::Canonical;
    assert_miss(changed_kind, options.clone());
    let mut changed_default = baseline.clone();
    changed_default.default_style.italic = true;
    assert_miss(changed_default, options.clone());
    let mut changed_options = options.clone();
    changed_options.max_width += 1;
    assert_miss(baseline.clone(), changed_options);
}

#[test]
fn public_styled_flow_failure_precedence_is_stable() {
    let invalid_range = 2..2;
    let invalid = TextFlowInput::plain("a", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![StyledTextRange {
            range: invalid_range.clone(),
            style: Style::new(),
        }]);
    assert_eq!(
        TextFlow::try_build_interruptible(&invalid, &options(), || true),
        Err(TextFlowError::Interrupted)
    );
    let mut invalid_calls = 0usize;
    assert_eq!(
        TextFlow::try_build_interruptible(&invalid, &options(), || {
            invalid_calls += 1;
            invalid_calls > 1
        }),
        Err(TextFlowError::InvalidStyleRange {
            range: invalid_range,
        })
    );
    assert_eq!(invalid_calls, 1);

    let overlap = TextFlowInput::plain("ab", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: 0..2,
                style: Style::new(),
            },
            StyledTextRange {
                range: 1..2,
                style: Style::new(),
            },
        ]);
    let mut overlap_calls = 0usize;
    assert_eq!(
        TextFlow::try_build_interruptible(&overlap, &options(), || {
            overlap_calls += 1;
            overlap_calls > 1
        }),
        Err(TextFlowError::OverlappingStyleRanges {
            first: 0..2,
            second: 1..2,
        })
    );
    assert_eq!(overlap_calls, 1);
}

#[test]
fn public_large_late_invalid_range_precedes_later_cancellation() {
    let invalid_range = 1..1;
    let mut ranges = (0..2_048)
        .map(|_| StyledTextRange {
            range: 0..0,
            style: Style::new(),
        })
        .collect::<Vec<_>>();
    ranges.push(StyledTextRange {
        range: invalid_range.clone(),
        style: Style::new(),
    });
    let input = TextFlowInput::plain("", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(ranges);
    let mut calls = 0usize;
    assert_eq!(
        TextFlow::try_build_interruptible(&input, &options(), || {
            calls += 1;
            calls > 1
        }),
        Err(TextFlowError::InvalidStyleRange {
            range: invalid_range,
        })
    );
    assert_eq!(calls, 1, "typed validation must not poll after entry");
}

#[test]
fn public_large_late_overlap_precedes_later_cancellation() {
    let source = "a".repeat(2_049);
    let first = 2_047..2_048;
    let second = 2_047..2_049;
    let mut ranges = (0..2_048)
        .map(|index| StyledTextRange {
            range: index..index + 1,
            style: Style::new(),
        })
        .collect::<Vec<_>>();
    ranges.push(StyledTextRange {
        range: second.clone(),
        style: Style::new(),
    });
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(ranges);
    let mut calls = 0usize;
    assert_eq!(
        TextFlow::try_build_interruptible(&input, &options(), || {
            calls += 1;
            calls > 1
        }),
        Err(TextFlowError::OverlappingStyleRanges { first, second })
    );
    assert_eq!(calls, 1, "typed validation must not poll after entry");
}

#[test]
fn public_styled_flow_failures_and_interruption_are_atomic() {
    let options = options();
    let stable_input = TextFlowInput::plain("stable", TextFlowSourceKind::Exact, Style::new());
    let mut cache = TextFlowCache::default();
    let published = cache.get_or_compute(&stable_input, &options).unwrap();
    let rows = published.rows().to_vec();
    let tokens = published.tokens().to_vec();
    let map = published.position_map().to_vec();

    let interrupted_input = empty_range_input(4_096);
    let mut calls = 0usize;
    let result = cache.get_or_compute_interruptible(&interrupted_input, &options, || {
        calls += 1;
        calls == 5
    });
    assert_eq!(result, Err(TextFlowError::Interrupted));

    let still_published = cache.get_or_compute(&stable_input, &options).unwrap();
    assert!(Arc::ptr_eq(&published, &still_published));
    assert_eq!(still_published.rows(), rows);
    assert_eq!(still_published.tokens(), tokens);
    assert_eq!(still_published.position_map(), map);
    assert_eq!(still_published.cache_identity().input, stable_input);
}

#[test]
fn public_styled_flow_retry_matches_cold_build() {
    let options = options();
    let stable_input = TextFlowInput::plain("stable", TextFlowSourceKind::Exact, Style::new());
    let interrupted_input = empty_range_input(4_096);
    let mut cache = TextFlowCache::default();
    let stable = cache.get_or_compute(&stable_input, &options).unwrap();
    let mut calls = 0usize;
    assert_eq!(
        cache.get_or_compute_interruptible(&interrupted_input, &options, || {
            calls += 1;
            calls == 5
        }),
        Err(TextFlowError::Interrupted)
    );
    assert!(Arc::ptr_eq(
        &stable,
        &cache.get_or_compute(&stable_input, &options).unwrap()
    ));

    let retried = cache.get_or_compute(&interrupted_input, &options).unwrap();
    let cold = TextFlow::try_build(&interrupted_input, &options).unwrap();
    assert_eq!(retried.as_ref(), &cold);
    assert_eq!(retried.cache_identity().input, interrupted_input);
}
