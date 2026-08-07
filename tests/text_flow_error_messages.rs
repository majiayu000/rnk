//! GH-58: what a `TextFlowError` says when it reaches a person.
//!
//! These messages are the whole diagnostic surface for a text flow failure —
//! the crate returns typed errors rather than logging, so `Display` is what a
//! caller prints and what ends up in a bug report. None of it was covered: a
//! format string that names the wrong field, or reports a range backwards,
//! would ship silently and send whoever reads it to the wrong place.
//!
//! Each case asserts the numbers actually reach the message, not merely that
//! some text came out.

use std::error::Error;

use rnk::core::{Style, TextWrap};
use rnk::layout::text_flow::{StyledTextRange, TextFlowSourceKind};
use rnk::layout::{TextFlow, TextFlowError, TextFlowInput, TextFlowOptions};

fn message(error: &TextFlowError) -> String {
    error.to_string()
}

#[test]
fn every_variant_says_which_failure_it_is() {
    let cases = [
        (
            TextFlowError::InvalidTabStop,
            vec!["tab stop", "greater than zero"],
        ),
        (
            TextFlowError::TabExpansionTooLarge { requested: 9001 },
            vec!["9001", "4096"],
        ),
        (
            TextFlowError::InvalidStyleRange { range: 7..3 },
            vec!["7", "3"],
        ),
        (
            TextFlowError::OverlappingStyleRanges {
                first: 0..5,
                second: 3..9,
            },
            vec!["overlap", "5", "9"],
        ),
        (
            TextFlowError::FinalizedRangeNotGraphemeBoundary { range: 2..4 },
            vec!["grapheme", "2", "4"],
        ),
        (
            TextFlowError::IncompleteSourceCoverage {
                expected: 40,
                covered: 12,
            },
            vec!["40", "12"],
        ),
        (TextFlowError::ArithmeticOverflow, vec!["overflow"]),
        (TextFlowError::Interrupted, vec!["interrupted"]),
    ];

    for (error, fragments) in cases {
        let rendered = message(&error);
        assert!(!rendered.is_empty(), "{error:?} rendered an empty message");
        for fragment in fragments {
            assert!(
                rendered.contains(fragment),
                "{error:?} rendered {rendered:?}, which does not mention {fragment:?}"
            );
        }
    }
}

#[test]
fn the_maximum_is_named_alongside_the_value_that_exceeded_it() {
    // A limit error is only actionable if it says what the limit was.
    let error = TextFlowError::TabExpansionTooLarge {
        requested: TextFlowOptions::MAX_TAB_EXPANSION + 1,
    };
    let rendered = message(&error);

    assert!(rendered.contains(&(TextFlowOptions::MAX_TAB_EXPANSION + 1).to_string()));
    assert!(
        rendered.contains(&TextFlowOptions::MAX_TAB_EXPANSION.to_string()),
        "the message reports the offending value without the limit: {rendered:?}"
    );
}

#[test]
fn coverage_errors_report_both_sides_of_the_shortfall() {
    // "covers N, expected M" is only useful with both numbers.
    let error = TextFlowError::IncompleteSourceCoverage {
        expected: 128,
        covered: 100,
    };
    let rendered = message(&error);

    assert!(rendered.contains("128"));
    assert!(rendered.contains("100"));
    assert_ne!(
        rendered.find("100"),
        rendered.find("128"),
        "the two counts collapsed into one number: {rendered:?}"
    );
}

#[test]
fn a_failure_from_a_real_build_prints_its_own_message() {
    // Not a hand-built variant: the error a caller actually receives.
    let input = TextFlowInput::plain("hello", TextFlowSourceKind::Exact, Style::default());
    let mut options = TextFlowOptions::new(20, TextWrap::Wrap);
    options.tab_stop = 0;

    let error = TextFlow::try_build(&input, &options).expect_err("tab stop zero must fail");
    assert_eq!(error, TextFlowError::InvalidTabStop);
    assert!(message(&error).contains("tab stop"));
    assert!(
        error.source().is_none(),
        "a leaf failure should not claim a cause"
    );
}

#[test]
fn an_overlapping_style_range_reports_both_ranges_from_a_real_build() {
    let mut input = TextFlowInput::plain("abcdefgh", TextFlowSourceKind::Exact, Style::default());
    input.styled_ranges = vec![
        StyledTextRange {
            range: 0..5,
            style: Style::default(),
        },
        StyledTextRange {
            range: 3..8,
            style: Style::default(),
        },
    ];
    let options = TextFlowOptions::new(20, TextWrap::Wrap);

    let error = TextFlow::try_build(&input, &options).expect_err("overlapping ranges must fail");
    let rendered = message(&error);
    assert!(
        rendered.contains('5') && rendered.contains('8'),
        "the overlap message does not identify both ranges: {rendered:?}"
    );
}

#[test]
fn an_interrupted_build_reports_interruption_and_nothing_else() {
    let input = TextFlowInput::plain("hello", TextFlowSourceKind::Exact, Style::default());
    let options = TextFlowOptions::new(20, TextWrap::Wrap);

    let error = TextFlow::try_build_interruptible(&input, &options, || true)
        .expect_err("an immediately interrupted build must fail");

    assert_eq!(error, TextFlowError::Interrupted);
    assert!(
        !message(&error).contains("tab"),
        "interruption was reported as a configuration problem"
    );
}
