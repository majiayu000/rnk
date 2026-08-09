use std::{cell::Cell, rc::Rc, sync::Arc};

use unicode_segmentation::UnicodeSegmentation;

use super::super::style_normalization::{
    NoopNormalizationObserver, NormalizationObserver, NormalizationOperations,
    VALIDATION_POLL_INTERVAL, build_styled_range_plan, checked_add, checked_endpoint_count,
    normalize_source, reserve, validate_styled_ranges,
};
use super::super::{
    StyledTextRange, TextFlow, TextFlowCache, TextFlowDiagnostic, TextFlowError, TextFlowInput,
    TextFlowOptions, TextFlowPlacement, TextFlowSource, TextFlowSourceKind, TextFlowToken,
    TokenClass,
};
use crate::core::{Color, Style, TextWrap};

pub(super) fn assert_text_flow_styled_runs() {
    let mut red = Style::new();
    red.color = Some(Color::Red);
    let mut blue = Style::new();
    blue.color = Some(Color::Blue);
    let input = TextFlowInput::plain("ab界", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
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

pub(super) fn assert_split_combining_and_zwj_style_boundary_normalizes() {
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

#[derive(Debug)]
struct Fixture {
    family: &'static str,
    input: TextFlowInput,
    expected_internal_events: usize,
}

fn observation_tokens(ranges: &[std::ops::Range<usize>]) -> Vec<TextFlowToken> {
    ranges
        .iter()
        .cloned()
        .map(|range| TextFlowToken {
            source: TextFlowSource::Source {
                range,
                kind: TextFlowSourceKind::Exact,
            },
            safe_text: String::new(),
            style: Style::new(),
            display_width: 0,
            placement: TextFlowPlacement::Omitted { row: 0 },
            class: TokenClass::Content,
        })
        .collect()
}

fn operation_bound(graphemes: usize, ranges: usize) -> usize {
    12usize
        .checked_mul(
            graphemes
                .checked_add(ranges)
                .expect("fixture size overflow"),
        )
        .and_then(|bound| bound.checked_add(64))
        .expect("fixture bound overflow")
}

fn assert_operation_bound(
    fixture: &Fixture,
    size: usize,
    graphemes: usize,
    operations: &NormalizationOperations,
    previous: Option<usize>,
) -> Result<usize, String> {
    let observed = operations.total().map_err(|error| error.to_string())?;
    let range_count = fixture.input.styled_ranges.len();
    let endpoint_count = range_count
        .checked_mul(2)
        .expect("fixture endpoint count overflow");
    let expected_plan_construction_steps = range_count
        .checked_mul(3)
        .expect("fixture plan count overflow");
    let last_grapheme_start = fixture
        .input
        .source
        .grapheme_indices(true)
        .next_back()
        .map_or(0, |(start, _)| start);
    let expected_style_range_advances = fixture
        .input
        .styled_ranges
        .iter()
        .filter(|styled| !styled.range.is_empty() && styled.range.end <= last_grapheme_start)
        .count();
    let bound = operation_bound(graphemes, fixture.input.styled_ranges.len());
    let slope_bound = previous.map(|value| value.saturating_mul(2).saturating_add(128));
    let components_are_exact = operations.grapheme_steps == graphemes
        && operations.plan_construction_steps == expected_plan_construction_steps
        && operations.plan_endpoint_visits == endpoint_count
        && operations.style_range_advances == expected_style_range_advances
        && operations.boundary_endpoint_visits == endpoint_count
        && operations.diagnostic_count_visits == endpoint_count
        && operations.diagnostic_offset_preparations == graphemes
        && operations.diagnostic_projections == fixture.expected_internal_events
        && operations.style_applications == graphemes;
    let internal_projection_is_nonzero =
        fixture.expected_internal_events == 0 || operations.diagnostic_projections > 0;
    if !components_are_exact
        || !internal_projection_is_nonzero
        || observed > bound
        || slope_bound.is_some_and(|limit| observed > limit)
    {
        return Err(format!(
            "family={} size={} G={} R={} internal_events={} projected_events={} \
             grapheme_steps={} plan_endpoint_visits={} style_range_advances={} \
             plan_construction_steps={} expected_plan_construction_steps={} \
             expected_style_range_advances={} boundary_endpoint_visits={} \
             diagnostic_count_visits={} diagnostic_offset_preparations={} \
             diagnostic_projections={} style_applications={} observed={} \
             absolute_bound={} previous_operations={:?} slope_bound={:?}",
            fixture.family,
            size,
            graphemes,
            fixture.input.styled_ranges.len(),
            fixture.expected_internal_events,
            operations.diagnostic_projections,
            operations.grapheme_steps,
            operations.plan_endpoint_visits,
            operations.style_range_advances,
            operations.plan_construction_steps,
            expected_plan_construction_steps,
            expected_style_range_advances,
            operations.boundary_endpoint_visits,
            operations.diagnostic_count_visits,
            operations.diagnostic_offset_preparations,
            operations.diagnostic_projections,
            operations.style_applications,
            observed,
            bound,
            previous,
            slope_bound,
        ));
    }
    Ok(observed)
}

fn ascii_fixture(size: usize) -> Fixture {
    let source = "a".repeat(size);
    let ranges = (0..size)
        .map(|index| StyledTextRange {
            range: index..index + 1,
            style: Style::new(),
        })
        .collect();
    Fixture {
        family: "ascii",
        input: TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
            .with_styled_ranges(ranges),
        expected_internal_events: 0,
    }
}

fn internal_boundary_fixture(size: usize) -> Fixture {
    assert_eq!(size % 2, 0);
    let mut source = String::new();
    let mut ranges = Vec::with_capacity(size);
    for index in 0..size / 2 {
        let start = source.len();
        if index % 2 == 0 {
            source.push_str("e\u{301}");
            ranges.push(StyledTextRange {
                range: start..start + 1,
                style: Style::new(),
            });
            ranges.push(StyledTextRange {
                range: start + 1..source.len(),
                style: Style::new(),
            });
        } else {
            source.push_str("👩‍💻");
            ranges.push(StyledTextRange {
                range: start..source.len(),
                style: Style::new(),
            });
            let boundary = start + "👩".len();
            ranges.push(StyledTextRange {
                range: boundary..boundary,
                style: Style::new(),
            });
        }
    }
    Fixture {
        family: "combining_zwj_internal",
        input: TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
            .with_styled_ranges(ranges),
        expected_internal_events: size,
    }
}

fn one_egc_skew_fixture(size: usize) -> Fixture {
    assert_eq!(size % 2, 0);
    let scalar_count = size / 2;
    let mut source = String::from("a");
    source.extend(std::iter::repeat_n('\u{301}', scalar_count - 1));
    assert_eq!(source.graphemes(true).count(), 1);
    let boundaries = source
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let mut ranges = Vec::with_capacity(size);
    for (index, start) in boundaries.iter().copied().enumerate() {
        let end = boundaries.get(index + 1).copied().unwrap_or(source.len());
        ranges.push(StyledTextRange {
            range: start..end,
            style: Style::new(),
        });
    }
    let interior = boundaries[1];
    ranges.extend((0..scalar_count).map(|_| StyledTextRange {
        range: interior..interior,
        style: Style::new(),
    }));
    Fixture {
        family: "one_egc_skew",
        input: TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
            .with_styled_ranges(ranges),
        expected_internal_events: size * 2 - 2,
    }
}

fn observe(fixture: &Fixture) -> (usize, NormalizationOperations) {
    let ranges = fixture
        .input
        .source
        .grapheme_indices(true)
        .map(|(start, grapheme)| start..start + grapheme.len())
        .collect::<Vec<_>>();
    let validated = validate_styled_ranges(&fixture.input).expect("fixture must validate");
    let mut operations = NormalizationOperations::default();
    let plan = build_styled_range_plan(validated, &mut || false, &mut operations)
        .expect("fixture must plan");
    let mut applied = observation_tokens(&ranges);
    let normalized = normalize_source(&plan, &ranges, &mut applied, &mut || false, &mut operations)
        .expect("fixture must normalize");
    assert_eq!(
        normalized.diagnostics.len(),
        fixture.expected_internal_events,
        "{} diagnostic count",
        fixture.family
    );
    (ranges.len(), operations)
}

#[test]
fn styled_boundary_normalization_operation_count_is_linear() {
    for make_fixture in [
        ascii_fixture as fn(usize) -> Fixture,
        internal_boundary_fixture,
        one_egc_skew_fixture,
    ] {
        let mut previous = None;
        for size in [2_000, 4_000, 8_000] {
            let fixture = make_fixture(size);
            let (graphemes, operations) = observe(&fixture);
            let observed = assert_operation_bound(&fixture, size, graphemes, &operations, previous)
                .unwrap_or_else(|message| panic!("{message}"));
            previous = Some(observed);
        }
    }
}

#[test]
fn styled_boundary_operation_bound_failure_reports_complete_diagnostics() {
    let fixture = internal_boundary_fixture(2_000);
    let operations = NormalizationOperations {
        grapheme_steps: 1_000,
        plan_construction_steps: 6_000,
        plan_endpoint_visits: 4_000,
        style_range_advances: 2_000,
        boundary_endpoint_visits: 4_000,
        diagnostic_count_visits: 4_000,
        diagnostic_offset_preparations: 1_000,
        diagnostic_projections: 100_000,
        style_applications: 1_000,
    };
    let message = assert_operation_bound(&fixture, 2_000, 1_000, &operations, Some(20_000))
        .expect_err("synthetic over-bound observation must fail");
    for expected in [
        "family=combining_zwj_internal",
        "size=2000",
        "G=1000",
        "R=2000",
        "internal_events=2000",
        "projected_events=100000",
        "grapheme_steps=1000",
        "plan_endpoint_visits=4000",
        "style_range_advances=2000",
        "plan_construction_steps=6000",
        "expected_plan_construction_steps=6000",
        "expected_style_range_advances=1499",
        "boundary_endpoint_visits=4000",
        "diagnostic_count_visits=4000",
        "diagnostic_offset_preparations=1000",
        "diagnostic_projections=100000",
        "style_applications=1000",
        "observed=123000",
        "absolute_bound=36064",
        "previous_operations=Some(20000)",
        "slope_bound=Some(40128)",
    ] {
        assert!(
            message.contains(expected),
            "missing {expected:?}: {message}"
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NormalizationPhase {
    Start,
    Grapheme,
    PlanConstruction,
    StyleAdvance,
    Boundary,
    DiagnosticCount,
    OffsetPreparation,
    EndpointProjection,
    DiagnosticProjection,
    StyleApplication,
}

struct PhaseObserver {
    phase: Rc<Cell<NormalizationPhase>>,
}

impl PhaseObserver {
    fn mark(&self, phase: NormalizationPhase) -> Result<(), TextFlowError> {
        self.phase.set(phase);
        Ok(())
    }
}

impl NormalizationObserver for PhaseObserver {
    fn grapheme_step(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::Grapheme)
    }

    fn plan_construction_step(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::PlanConstruction)
    }

    fn plan_endpoint_visit(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::EndpointProjection)
    }

    fn style_range_advance(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::StyleAdvance)
    }

    fn boundary_endpoint_visit(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::Boundary)
    }

    fn diagnostic_count_visit(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::DiagnosticCount)
    }

    fn diagnostic_offset_preparation(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::OffsetPreparation)
    }

    fn diagnostic_projection(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::DiagnosticProjection)
    }

    fn style_application(&mut self) -> Result<(), TextFlowError> {
        self.mark(NormalizationPhase::StyleApplication)
    }
}

#[test]
fn valid_style_plan_linear_construction_is_interruptible() {
    let count = VALIDATION_POLL_INTERVAL * 4;
    let sorted = ascii_fixture(count);
    let mut completed_polls = 0usize;
    let mut observer = NoopNormalizationObserver;
    let validated = validate_styled_ranges(&sorted.input).expect("fixture must validate");
    build_styled_range_plan(
        validated,
        &mut || {
            completed_polls += 1;
            false
        },
        &mut observer,
    )
    .expect("linear plan construction must complete");
    assert_eq!(completed_polls, 28, "plan work must be exactly linear");

    let validated = validate_styled_ranges(&sorted.input).expect("fixture must validate");
    let mut construction_polls = 0usize;
    assert_eq!(
        build_styled_range_plan(
            validated,
            &mut || {
                construction_polls += 1;
                construction_polls == 1
            },
            &mut observer,
        )
        .map(|_| ()),
        Err(TextFlowError::Interrupted)
    );

    let empty = TextFlowInput::plain("", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(
            (0..count)
                .map(|_| StyledTextRange {
                    range: 0..0,
                    style: Style::new(),
                })
                .collect(),
        );
    for (target, phase) in [(9, "filtered stream"), (13, "endpoint merge")] {
        let validated = validate_styled_ranges(&empty).expect("empty fixture must validate");
        let mut polls = 0usize;
        assert!(
            matches!(
                build_styled_range_plan(
                    validated,
                    &mut || {
                        polls += 1;
                        polls == target
                    },
                    &mut observer,
                ),
                Err(TextFlowError::Interrupted)
            ),
            "phase={phase}"
        );
        assert_eq!(polls, target, "phase={phase}");
    }
}

#[test]
fn diagnostic_count_offset_projection_and_style_application_are_interruptible() {
    let fixture = internal_boundary_fixture(4_096);
    let ranges = fixture
        .input
        .source
        .grapheme_indices(true)
        .map(|(start, grapheme)| start..start + grapheme.len())
        .collect::<Vec<_>>();
    let validated = validate_styled_ranges(&fixture.input).expect("fixture must validate");
    let mut plan_observer = NoopNormalizationObserver;
    let plan = build_styled_range_plan(validated, &mut || false, &mut plan_observer)
        .expect("fixture must plan");

    for target in [
        NormalizationPhase::StyleApplication,
        NormalizationPhase::DiagnosticCount,
        NormalizationPhase::OffsetPreparation,
        NormalizationPhase::EndpointProjection,
        NormalizationPhase::DiagnosticProjection,
    ] {
        let phase = Rc::new(Cell::new(NormalizationPhase::Start));
        let mut observer = PhaseObserver {
            phase: Rc::clone(&phase),
        };
        let mut applied = observation_tokens(&ranges);
        let result = normalize_source(
            &plan,
            &ranges,
            &mut applied,
            &mut || phase.get() == target,
            &mut observer,
        );
        assert!(
            matches!(result, Err(TextFlowError::Interrupted)),
            "phase={target:?}"
        );
        assert_eq!(phase.get(), target);
    }
}

#[test]
fn style_normalization_capacity_and_arithmetic_seams_are_typed() {
    let mut bytes = Vec::<u8>::new();
    assert_eq!(
        reserve(&mut bytes, usize::MAX),
        Err(TextFlowError::ArithmeticOverflow)
    );
    assert_eq!(
        checked_endpoint_count(usize::MAX),
        Err(TextFlowError::ArithmeticOverflow)
    );
    assert_eq!(
        checked_add(usize::MAX, 1),
        Err(TextFlowError::ArithmeticOverflow)
    );
    let fixture = ascii_fixture(1);
    let validated = validate_styled_ranges(&fixture.input).expect("fixture must validate");
    let mut observer = NoopNormalizationObserver;
    let plan = build_styled_range_plan(validated, &mut || false, &mut observer).unwrap();
    let ranges = std::iter::once(0..1).collect::<Vec<_>>();
    assert_eq!(
        normalize_source(&plan, &ranges, &mut [], &mut || false, &mut observer).map(|_| ()),
        Err(TextFlowError::ArithmeticOverflow)
    );

    let mut operations = NormalizationOperations {
        grapheme_steps: usize::MAX,
        ..NormalizationOperations::default()
    };
    assert_eq!(
        operations.grapheme_step(),
        Err(TextFlowError::ArithmeticOverflow)
    );
    operations.plan_endpoint_visits = 1;
    assert_eq!(operations.total(), Err(TextFlowError::ArithmeticOverflow));
}

#[test]
fn style_boundary_event_order_and_multiplicity_are_stable() {
    let source = "👩‍💻x";
    let zwj_end = "👩‍💻".len();
    let woman_end = "👩".len();
    let joiner_end = woman_end + "\u{200d}".len();
    let mut first_source = Style::new();
    first_source.color = Some(Color::Red);
    let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: zwj_end..source.len(),
                style: Style::new(),
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
                style: Style::new(),
            },
            StyledTextRange {
                range: woman_end..joiner_end,
                style: Style::new(),
            },
        ]);
    let flow = TextFlow::try_build(&input, &TextFlowOptions::new(20, TextWrap::Wrap)).unwrap();
    assert_eq!(flow.tokens()[0].style, first_source);
    assert_eq!(
        flow.diagnostics(),
        &[
            TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary: joiner_end,
                grapheme_range: 0..zwj_end,
            },
            TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary: woman_end,
                grapheme_range: 0..zwj_end,
            },
            TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary: joiner_end,
                grapheme_range: 0..zwj_end,
            },
            TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary: joiner_end,
                grapheme_range: 0..zwj_end,
            },
            TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary: woman_end,
                grapheme_range: 0..zwj_end,
            },
            TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary: joiner_end,
                grapheme_range: 0..zwj_end,
            },
        ]
    );
}

#[test]
fn styled_range_extremes_preserve_typed_errors() {
    let source = "é";
    for range in [
        std::ops::Range { start: 2, end: 1 },
        1..2,
        0..1,
        0..usize::MAX,
        usize::MAX..usize::MAX,
    ] {
        let input = TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
            .with_styled_ranges(vec![StyledTextRange {
                range: range.clone(),
                style: Style::new(),
            }]);
        assert_eq!(
            TextFlow::try_build(&input, &TextFlowOptions::new(8, TextWrap::Wrap)),
            Err(TextFlowError::InvalidStyleRange { range })
        );
    }

    let first = 3..3;
    let second = std::ops::Range { start: 1, end: 0 };
    let input = TextFlowInput::plain("ab", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(vec![
            StyledTextRange {
                range: first.clone(),
                style: Style::new(),
            },
            StyledTextRange {
                range: second,
                style: Style::new(),
            },
        ]);
    assert_eq!(
        TextFlow::try_build(&input, &TextFlowOptions::new(8, TextWrap::Wrap)),
        Err(TextFlowError::InvalidStyleRange { range: first })
    );

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
        TextFlow::try_build(&overlap, &TextFlowOptions::new(8, TextWrap::Wrap)),
        Err(TextFlowError::OverlappingStyleRanges {
            first: 0..3,
            second: 2..4,
        })
    );
}

#[test]
fn styled_normalization_polling_and_cache_count_are_atomic() {
    let options = TextFlowOptions::new(8, TextWrap::Wrap);
    let baseline_input = TextFlowInput::plain("stable", TextFlowSourceKind::Exact, Style::new());
    let mut cache = TextFlowCache::default();
    let published = cache.get_or_compute(&baseline_input, &options).unwrap();

    let interrupted_input = TextFlowInput::plain("", TextFlowSourceKind::Exact, Style::new())
        .with_styled_ranges(
            (0..4_096)
                .map(|_| StyledTextRange {
                    range: 0..0,
                    style: Style::new(),
                })
                .collect(),
        );
    let mut calls = 0usize;
    let result = cache.get_or_compute_interruptible(&interrupted_input, &options, || {
        calls += 1;
        calls == 5
    });
    assert_eq!(result, Err(TextFlowError::Interrupted));
    assert_eq!(cache.build_count, 1);
    assert!(Arc::ptr_eq(cache.published.as_ref().unwrap(), &published));

    let retried = cache.get_or_compute(&interrupted_input, &options).unwrap();
    let cold = TextFlow::try_build(&interrupted_input, &options).unwrap();
    assert_eq!(retried.as_ref(), &cold);
    assert_eq!(cache.build_count, 2);

    let graphemes = 4_096usize;
    let source = "a".repeat(graphemes);
    let style_application_input =
        TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new()).with_styled_ranges(
            vec![StyledTextRange {
                range: 0..graphemes,
                style: Style::new(),
            }],
        );
    let mut style_cache = TextFlowCache::default();
    let style_published = style_cache
        .get_or_compute(&baseline_input, &options)
        .unwrap();
    let first_style_application_poll = 5 * graphemes + 11;
    let mut style_calls = 0usize;
    let style_result =
        style_cache.get_or_compute_interruptible(&style_application_input, &options, || {
            style_calls += 1;
            style_calls == first_style_application_poll
        });
    assert_eq!(style_result, Err(TextFlowError::Interrupted));
    assert_eq!(style_calls, first_style_application_poll);
    assert_eq!(style_cache.build_count, 1);
    assert!(Arc::ptr_eq(
        style_cache.published.as_ref().unwrap(),
        &style_published
    ));
    let style_retry = style_cache
        .get_or_compute(&style_application_input, &options)
        .unwrap();
    let style_cold = TextFlow::try_build(&style_application_input, &options).unwrap();
    assert_eq!(style_retry.as_ref(), &style_cold);
    assert_eq!(style_cache.build_count, 2);

    for (input, expected) in
        [
            (
                TextFlowInput::plain("a", TextFlowSourceKind::Exact, Style::new())
                    .with_styled_ranges(vec![StyledTextRange {
                        range: 2..2,
                        style: Style::new(),
                    }]),
                TextFlowError::InvalidStyleRange { range: 2..2 },
            ),
            (
                TextFlowInput::plain("ab", TextFlowSourceKind::Exact, Style::new())
                    .with_styled_ranges(vec![
                        StyledTextRange {
                            range: 0..2,
                            style: Style::new(),
                        },
                        StyledTextRange {
                            range: 1..2,
                            style: Style::new(),
                        },
                    ]),
                TextFlowError::OverlappingStyleRanges {
                    first: 0..2,
                    second: 1..2,
                },
            ),
        ]
    {
        let mut precedence_calls = 0usize;
        let result = TextFlow::try_build_interruptible(&input, &options, || {
            precedence_calls += 1;
            precedence_calls > 1
        });
        assert_eq!(result, Err(expected));
        assert_eq!(precedence_calls, 1);
    }
}
