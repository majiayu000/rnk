use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use rnk::{
    core::{Style, TextWrap},
    layout::{
        TextFlow, TextFlowCache, TextFlowError, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
    },
};

fn wrap_input(source: String) -> TextFlowInput {
    TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::new())
}

fn expect_interrupted_after(
    input: &TextFlowInput,
    options: &TextFlowOptions,
    allowed_calls: usize,
) {
    let mut calls = 0usize;
    let result = TextFlow::try_build_interruptible(input, options, || {
        calls += 1;
        calls > allowed_calls
    });

    assert!(matches!(result, Err(TextFlowError::Interrupted)));
    assert_eq!(
        calls,
        allowed_calls + 1,
        "wrapping did not stop at the first armed interruption poll"
    );
}

#[test]
fn cancellation_after_tokenization_stops_a_100k_word_promptly() {
    const TOKENS: usize = 100_000;
    let input = wrap_input("a".repeat(TOKENS));
    let options = TextFlowOptions::new(80, TextWrap::Wrap);

    // One build-entry poll, one poll per tokenization token, and one wrap-entry poll.
    // Cancellation is armed for the first long-word collection operation.
    expect_interrupted_after(&input, &options, TOKENS + 2);
}

#[test]
fn wide_width_and_tab_or_whitespace_placement_poll_for_interruption() {
    const TOKENS: usize = 4_096;
    let fixtures = [
        (
            wrap_input("界".repeat(TOKENS)),
            TextFlowOptions::new(TOKENS * 2, TextWrap::Wrap),
            2 * TOKENS + 2,
        ),
        (
            wrap_input("\t".repeat(TOKENS)),
            TextFlowOptions::new(TOKENS * 4, TextWrap::Wrap),
            2 * TOKENS + 1,
        ),
        (
            wrap_input(" ".repeat(TOKENS)),
            TextFlowOptions::new(TOKENS, TextWrap::Wrap),
            3 * TOKENS + 1,
        ),
    ];

    for (input, options, allowed_calls) in fixtures {
        expect_interrupted_after(&input, &options, allowed_calls);
    }
}

#[test]
fn cancellation_armed_after_placement_starts_interrupts_the_next_token() {
    const TOKENS: usize = 4_096;
    // Before placement: build entry + tokenization + wrap scanning + append_wrapped.
    // The first placement poll is allowed to construct one token; cancellation is
    // armed only when the second placement token is about to be constructed.
    const SECOND_PLACEMENT_POLL: usize = 3 * TOKENS + 3;
    let (placement_started_tx, placement_started_rx) = mpsc::sync_channel(0);
    let (cancel_tx, cancel_rx) = mpsc::sync_channel(0);

    let build = thread::spawn(move || {
        let input = wrap_input(" ".repeat(TOKENS));
        let options = TextFlowOptions::new(TOKENS, TextWrap::Wrap);
        let mut calls = 0usize;
        let result = TextFlow::try_build_interruptible(&input, &options, || {
            calls += 1;
            if calls != SECOND_PLACEMENT_POLL {
                return false;
            }
            placement_started_tx
                .send(())
                .expect("test receiver must observe placement progress");
            cancel_rx
                .recv()
                .expect("test coordinator must arm cancellation after placement starts");
            true
        });
        (result, calls)
    });

    placement_started_rx
        .recv()
        .expect("placement must poll again after constructing its first token");
    cancel_tx
        .send(())
        .expect("builder must still be waiting for cancellation");
    let (result, calls) = build.join().expect("builder thread must not panic");

    assert!(matches!(result, Err(TextFlowError::Interrupted)));
    assert_eq!(calls, SECOND_PLACEMENT_POLL);
}

#[test]
fn interruption_publishes_no_partial_cache_rows_or_position_map() {
    let stable_input = wrap_input("keep\t界界  together\t終".to_string());
    let options = TextFlowOptions::new(9, TextWrap::Wrap);
    let completed_direct =
        TextFlow::try_build(&stable_input, &options).expect("baseline flow must build");
    let mut cache = TextFlowCache::default();
    let published = cache
        .get_or_compute(&stable_input, &options)
        .expect("baseline flow must publish");
    assert_eq!(published.as_ref(), &completed_direct);

    let rows_before = published.rows().to_vec();
    let logical_rows_before = published.logical_rows().to_vec();
    let map_before = published.position_map().to_vec();
    let tokens_before = published.tokens().to_vec();

    const TOKENS: usize = 8_192;
    let interrupted_input = wrap_input(" ".repeat(TOKENS));
    let mut calls = 0usize;
    let result = cache.get_or_compute_interruptible(&interrupted_input, &options, || {
        calls += 1;
        // Cache entry, build entry, tokenization, wrap scanning, append_wrapped,
        // one completed placement token, then the interrupting placement poll.
        calls > 3 * TOKENS + 3
    });
    assert!(matches!(result, Err(TextFlowError::Interrupted)));
    assert_eq!(calls, 3 * TOKENS + 4);

    let still_published = cache
        .get_or_compute(&stable_input, &options)
        .expect("interruption must retain the previously published flow");
    assert!(Arc::ptr_eq(&published, &still_published));
    assert_eq!(still_published.rows(), rows_before);
    assert_eq!(still_published.logical_rows(), logical_rows_before);
    assert_eq!(still_published.position_map(), map_before);
    assert_eq!(still_published.tokens(), tokens_before);

    let completed_after_interruption = cache
        .get_or_compute(&interrupted_input, &options)
        .expect("the same input must complete without cancellation");
    let expected = TextFlow::try_build(&interrupted_input, &options)
        .expect("ordinary completed output must remain valid");
    assert_eq!(completed_after_interruption.as_ref(), &expected);
}
