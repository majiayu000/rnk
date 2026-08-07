//! Deterministic fixtures for the message list.

pub(crate) mod support;

use super::*;
use support::{entry_with_rows, measure_from_table, sized_state};

fn rows(value: u64) -> MessageRows {
    MessageRows::try_new(value).expect("non-zero")
}

fn described(
    state: &MessageListState,
) -> Vec<(usize, core::ops::Range<u64>, core::ops::Range<u64>)> {
    state
        .visible_range()
        .unwrap()
        .slices
        .iter()
        .map(|slice| {
            (
                slice.message_index,
                slice.message_rows.clone(),
                slice.viewport_rows.clone(),
            )
        })
        .collect()
}

#[test]
fn a_new_list_opens_at_the_bottom_of_the_transcript() {
    // A chat opens on the newest messages, so the initial offset is the last
    // full screen rather than row zero.
    let state = sized_state(&[3, 5, 2], 6, 10);

    assert_eq!(state.follow_state(), BottomFollowState::Following);
    assert_eq!(
        state.scroll_offset().get(),
        4,
        "10 rows less a 6-row viewport"
    );
    assert_eq!(state.visible_range().unwrap().total_rows, 10);
}

#[test]
fn visible_range_includes_partly_visible_first_and_last_messages() {
    // Heights 3, 5, 2 opened at the bottom: the viewport covers rows 4..10, so
    // it starts partway through the second message and ends on the third. An
    // item-count scroll cannot express either boundary.
    let state = sized_state(&[3, 5, 2], 6, 10);

    assert_eq!(
        described(&state),
        vec![(1, 1..5, 0..4), (2, 0..2, 4..6)],
        "slices did not cover the viewport exactly"
    );
}

#[test]
fn scrolling_to_the_top_shows_the_first_messages_whole() {
    let mut state = sized_state(&[3, 5, 2], 6, 10);
    state
        .try_scroll_to(state.revision(), RowOffset::ZERO)
        .unwrap();

    assert_eq!(described(&state), vec![(0, 0..3, 0..3), (1, 0..3, 3..6)]);
}

#[test]
fn a_partly_scrolled_first_message_reports_its_own_row_range() {
    let mut state = sized_state(&[3, 5, 2], 6, 10);
    state
        .try_scroll_to(state.revision(), RowOffset::new(2))
        .unwrap();

    assert_eq!(described(&state), vec![(0, 2..3, 0..1), (1, 0..5, 1..6)]);
}

#[test]
fn a_zero_row_viewport_shows_nothing_but_keeps_its_place() {
    let mut state = sized_state(&[3, 5], 4, 8);
    let anchor_before = state.stored_anchor();

    state
        .try_set_viewport_rows(state.revision(), ViewportRows::new(0))
        .unwrap();

    assert!(state.visible_range().unwrap().slices.is_empty());
    assert!(state.stored_anchor().is_some());
    assert_eq!(state.follow_state(), BottomFollowState::Following);
    let _ = anchor_before;
}

#[test]
fn prepending_history_keeps_the_reader_on_the_same_message_row() {
    let mut state = sized_state(&[4, 4, 4], 4, 12);
    state
        .try_scroll_to(state.revision(), RowOffset::new(5))
        .unwrap();

    let anchor = state.stored_anchor().expect("paused list is anchored");
    let offset_before = state.scroll_offset();

    let older = [entry_with_rows(90, 7), entry_with_rows(91, 3)];
    let table = [(90_u64, 7_u64), (91, 3)];
    state
        .try_prepend::<(), (), _>(state.revision(), &older, measure_from_table(&table))
        .unwrap();

    assert_eq!(
        state.stored_anchor(),
        Some(anchor),
        "prepending history moved the anchor"
    );
    assert_eq!(
        state.scroll_offset().get(),
        offset_before.get() + 10,
        "the anchored row did not shift by the height of the prepended history"
    );
}

#[test]
fn streaming_below_a_paused_reader_raises_the_new_content_flag() {
    let mut state = sized_state(&[3, 3, 3], 3, 9);
    state
        .try_scroll_to(state.revision(), RowOffset::new(0))
        .unwrap();
    assert!(!state.observation().new_content_below);

    let table = [(1_u64, 3_u64), (2, 3), (3, 3), (4, 5)];
    state
        .try_append::<(), (), _>(
            state.revision(),
            &[entry_with_rows(4, 5)],
            measure_from_table(&table),
        )
        .unwrap();

    let observation = state.observation();
    assert!(
        observation.new_content_below,
        "content arrived below the viewport without raising the indicator"
    );
    assert_eq!(state.scroll_offset().get(), 0, "a paused reader was moved");
}

#[test]
fn a_following_list_stays_at_the_bottom_as_content_arrives() {
    let mut state = sized_state(&[3, 3], 3, 6);
    assert_eq!(state.follow_state(), BottomFollowState::Following);
    assert_eq!(state.scroll_offset().get(), 3);

    let table = [(1_u64, 3_u64), (2, 3), (3, 4)];
    state
        .try_append::<(), (), _>(
            state.revision(),
            &[entry_with_rows(3, 4)],
            measure_from_table(&table),
        )
        .unwrap();

    assert_eq!(state.follow_state(), BottomFollowState::Following);
    assert_eq!(state.scroll_offset().get(), 7);
    assert!(!state.observation().new_content_below);
}

#[test]
fn returning_to_the_bottom_resumes_following_and_clears_the_flag() {
    let mut state = sized_state(&[3, 3, 3], 3, 9);
    state
        .try_scroll_to(state.revision(), RowOffset::new(0))
        .unwrap();
    let table = [(1_u64, 3_u64), (2, 3), (3, 3), (4, 3)];
    state
        .try_append::<(), (), _>(
            state.revision(),
            &[entry_with_rows(4, 3)],
            measure_from_table(&table),
        )
        .unwrap();
    assert!(state.observation().new_content_below);

    state.jump_to_bottom(state.revision()).unwrap();

    assert_eq!(state.follow_state(), BottomFollowState::Following);
    assert!(!state.observation().new_content_below);
    assert_eq!(state.scroll_offset().get(), 9);
}

#[test]
fn an_unmeasured_message_is_a_typed_error_not_a_one_row_guess() {
    let entries = [entry_with_rows(1, 3)];
    let result = MessageListState::try_new::<(), (), _>(
        &entries,
        40,
        ViewportRows::new(4),
        16,
        |_request| MessageMeasureOutcome::Missing,
    );

    assert!(matches!(
        result,
        Err(MessageListMeasureError::State(
            MessageListStateError::MissingMeasurement { .. }
        ))
    ));
}

#[test]
fn a_zero_height_measurement_cannot_be_constructed() {
    assert_eq!(MessageRows::try_new(0), Err(MessageRowsError::Zero));
    assert_eq!(rows(1).get(), 1);
}

#[test]
fn a_failed_measurement_leaves_the_list_exactly_as_it_was() {
    let mut state = sized_state(&[3, 3], 3, 6);
    let before = state.visible_range().unwrap();
    let revision_before = state.revision();

    let result = state.try_append::<&'static str, (), _>(
        state.revision(),
        &[entry_with_rows(9, 4)],
        |_request| MessageMeasureOutcome::Failed("no measurement available"),
    );

    assert!(matches!(
        result,
        Err(MessageListMeasureError::MeasurementFailed { .. })
    ));
    assert_eq!(state.revision(), revision_before);
    assert_eq!(state.visible_range().unwrap(), before);
}

#[test]
fn a_cancelled_measurement_is_distinct_from_a_failed_one() {
    let mut state = sized_state(&[3], 3, 3);

    let result = state.try_append::<&'static str, &'static str, _>(
        state.revision(),
        &[entry_with_rows(9, 4)],
        |_request| MessageMeasureOutcome::Cancelled("the reader navigated away"),
    );

    assert!(
        matches!(result, Err(MessageListMeasureError::Cancelled { .. })),
        "cancellation was reported as a failure"
    );
}

#[test]
fn a_stale_revision_is_rejected_before_anything_is_measured() {
    // The list must be tall enough that scrolling actually moves it; otherwise
    // the scroll is a no-op, the revision does not advance, and the test would
    // not be holding a stale revision at all.
    let mut state = sized_state(&[3, 3], 3, 6);
    let stale = state.revision();
    let scrolled = state
        .try_scroll_to(state.revision(), RowOffset::ZERO)
        .unwrap();
    assert!(
        matches!(scrolled, MessageListMutation::Applied(_)),
        "the fixture scroll did not advance the revision"
    );

    let mut calls = 0;
    let result = state.try_append::<(), (), _>(stale, &[entry_with_rows(9, 4)], |_request| {
        calls += 1;
        MessageMeasureOutcome::Measured(rows(4))
    });

    assert!(matches!(
        result,
        Err(MessageListMeasureError::State(
            MessageListStateError::StaleStateRevision { .. }
        ))
    ));
    assert_eq!(calls, 0, "a stale mutation still ran the measure callback");
}

#[test]
fn an_out_of_range_insert_is_rejected_before_any_callback() {
    let mut state = sized_state(&[3], 3, 3);
    let mut calls = 0;

    let result =
        state.try_insert::<(), (), _>(state.revision(), 7, entry_with_rows(9, 1), |_request| {
            calls += 1;
            MessageMeasureOutcome::Measured(rows(1))
        });

    assert!(matches!(
        result,
        Err(MessageListMeasureError::State(
            MessageListStateError::InvalidInsertIndex { index: 7, len: 1 }
        ))
    ));
    assert_eq!(calls, 0);
}

#[test]
fn navigating_to_an_unknown_message_changes_nothing() {
    let mut state = sized_state(&[3, 3], 3, 6);
    let before = state.observation();

    let result = state.try_scroll_to_message(
        state.revision(),
        crate::components::chat::MessageId::new(404),
    );

    assert!(matches!(
        result,
        Err(MessageListStateError::UnknownMessageId { .. })
    ));
    assert_eq!(state.observation(), before);
}

#[test]
fn navigating_past_the_end_of_a_message_is_rejected_not_clamped() {
    let mut state = sized_state(&[3, 3], 3, 6);
    let before = state.observation();

    let result = state.try_scroll_to_anchor(
        state.revision(),
        MessageAnchor::new(
            crate::components::chat::MessageId::new(1),
            RowOffset::new(9),
        ),
    );

    assert!(matches!(
        result,
        Err(MessageListStateError::InvalidAnchorRow { .. })
    ));
    assert_eq!(state.observation(), before);
}

#[test]
fn an_unchanged_entry_does_not_remeasure_or_advance_the_revision() {
    let mut state = sized_state(&[3, 3], 3, 6);
    let revision_before = state.revision();
    let mut calls = 0;

    let outcome = state
        .try_update::<(), (), _>(state.revision(), entry_with_rows(1, 3), |_request| {
            calls += 1;
            MessageMeasureOutcome::Measured(rows(3))
        })
        .unwrap();

    assert_eq!(
        outcome,
        MessageListMutation::NoChange {
            revision: revision_before
        }
    );
    assert_eq!(calls, 0, "an unchanged message was re-measured");
}

#[test]
fn deleting_the_anchored_message_falls_to_the_next_survivor() {
    let mut state = sized_state(&[3, 3, 3], 3, 9);
    state
        .try_scroll_to(state.revision(), RowOffset::new(3))
        .unwrap();
    assert_eq!(
        state.stored_anchor().map(|anchor| anchor.message_id()),
        Some(crate::components::chat::MessageId::new(2))
    );

    state
        .try_remove(state.revision(), crate::components::chat::MessageId::new(2))
        .unwrap();

    assert_eq!(
        state.stored_anchor().map(|anchor| anchor.message_id()),
        Some(crate::components::chat::MessageId::new(3)),
        "the view did not fall through to the message that took its place"
    );
}

#[test]
fn a_shrinking_message_clamps_its_anchor_and_says_so() {
    let mut state = sized_state(&[3, 6], 4, 9);
    state
        .try_scroll_to_anchor(
            state.revision(),
            MessageAnchor::new(
                crate::components::chat::MessageId::new(2),
                RowOffset::new(4),
            ),
        )
        .unwrap();

    let table = [(1_u64, 3_u64), (2, 2)];
    let outcome = state
        .try_update::<(), (), _>(
            state.revision(),
            entry_with_rows(2, 2),
            measure_from_table(&table),
        )
        .unwrap();

    match outcome {
        MessageListMutation::Applied(update) => assert!(
            update.anchor_clamped,
            "the anchor moved without reporting a clamp"
        ),
        MessageListMutation::NoChange { .. } => panic!("shrinking a message changed nothing"),
    }
    assert_eq!(
        state
            .stored_anchor()
            .map(|anchor| anchor.intra_message_row().get()),
        Some(1)
    );
}

#[test]
fn a_height_only_resize_skips_both_callbacks() {
    let mut state = sized_state(&[3, 3], 3, 6);
    let mut rebuilds = 0;
    let mut measures = 0;

    state
        .try_resize::<(), (), _, _>(
            state.revision(),
            state.width(),
            ViewportRows::new(5),
            |_request| {
                rebuilds += 1;
                unreachable!("a height-only resize rebuilt a config")
            },
            |_request| {
                measures += 1;
                unreachable!("a height-only resize re-measured")
            },
        )
        .unwrap();

    assert_eq!((rebuilds, measures), (0, 0));
    assert_eq!(state.viewport_rows(), ViewportRows::new(5));
}

#[test]
fn an_identical_resize_is_a_no_op() {
    let mut state = sized_state(&[3], 3, 3);
    let revision = state.revision();

    let outcome = state
        .try_resize::<(), (), _, _>(
            revision,
            state.width(),
            state.viewport_rows(),
            |_| unreachable!(),
            |_| unreachable!(),
        )
        .unwrap();

    assert_eq!(outcome, MessageListMutation::NoChange { revision });
}

#[test]
fn a_failed_config_rebuild_leaves_the_old_width_in_place() {
    let mut state = sized_state(&[3, 3], 3, 6);
    let width_before = state.width();
    let revision_before = state.revision();

    let result = state.try_resize::<&'static str, (), _, _>(
        state.revision(),
        width_before + 10,
        state.viewport_rows(),
        |_request| MessageResizeConfigOutcome::Failed("cannot reflow"),
        |_request| unreachable!("measurement ran after a failed rebuild"),
    );

    assert!(matches!(
        result,
        Err(MessageListMeasureError::ConfigRebuildFailed { .. })
    ));
    assert_eq!(state.width(), width_before);
    assert_eq!(state.revision(), revision_before);
}

#[test]
fn the_key_rejects_a_config_that_does_not_fit_its_shell() {
    // A child flowed at a width other than the shell's content width would
    // measure rows the renderer never paints.
    let shell =
        MessageShellMeasureConfig::try_new(40, HorizontalInsets::new(2, 2), vec![]).unwrap();
    assert_eq!(shell.content_width(), 36);

    let identity = support::text_identity("hello", 12);
    assert!(matches!(
        MessageCompositeMeasureConfig::try_new(vec![identity], shell),
        Err(MessageListStateError::InvalidViewportWidth { .. })
    ));
}

#[test]
fn a_key_containing_nan_is_equal_to_itself() {
    // Float PartialEq is not reflexive over NaN. If the key delegated to it,
    // this message would miss its own cache entry on every single lookup.
    let entry = support::entry_with_nan_style(1);
    let first = entry.measure_key();
    let second = entry.measure_key();

    assert_eq!(first, first, "a key was not equal to itself");
    assert_eq!(first, second);
}

#[test]
fn structural_rows_are_counted_on_top_of_the_text() {
    let shell = MessageShellMeasureConfig::try_new(
        40,
        HorizontalInsets::new(0, 0),
        vec![
            MessageStructuralSegment::new(MessageStructureSlotKey::new(1), RowOffset::new(1)),
            MessageStructuralSegment::new(MessageStructureSlotKey::new(2), RowOffset::new(2)),
        ],
    )
    .unwrap();

    assert_eq!(shell.structural_rows().unwrap(), 3);
}

#[test]
fn a_repeated_structural_slot_is_rejected() {
    let duplicate =
        MessageStructuralSegment::new(MessageStructureSlotKey::new(1), RowOffset::new(1));
    assert!(matches!(
        MessageShellMeasureConfig::try_new(
            40,
            HorizontalInsets::new(0, 0),
            vec![duplicate.clone(), duplicate],
        ),
        Err(MessageListStateError::InvalidViewportWidth { .. })
    ));
}

#[test]
fn rendering_calls_the_closure_once_per_visible_slice() {
    // Opened at the bottom, the viewport covers the tail of message 2 and all
    // of message 3, so the closure runs exactly twice with those row ranges.
    let state = sized_state(&[3, 5, 2], 6, 10);
    let mut seen = Vec::new();

    MessageList::new(&state)
        .try_into_element::<(), _>(|entry, key, slice| {
            assert_eq!(key.as_key().message_id(), entry.message_id());
            seen.push((entry.message_id(), slice.message_rows.clone()));
            Ok(Element::box_element())
        })
        .unwrap();

    assert_eq!(
        seen,
        vec![
            (crate::components::chat::MessageId::new(2), 1..5),
            (crate::components::chat::MessageId::new(3), 0..2),
        ]
    );
}

#[test]
fn a_render_failure_names_the_message_and_returns_no_partial_element() {
    let state = sized_state(&[3], 3, 3);

    let result = MessageList::new(&state)
        .try_into_element::<&'static str, _>(|_entry, _key, _slice| Err("cannot draw"));

    match result {
        Err(MessageListRenderError::RenderFailed {
            message_id, source, ..
        }) => {
            assert_eq!(message_id, crate::components::chat::MessageId::new(1));
            assert_eq!(source, "cannot draw");
        }
        other => panic!("expected a render failure, got {other:?}"),
    }
}
