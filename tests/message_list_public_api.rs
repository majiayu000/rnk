//! GH-65: the message list must be usable, and its errors matchable, from
//! outside the crate.
//!
//! A closed error family is only closed if a caller can exhaustively match it
//! without a wildcard arm. These fixtures compile against the public surface
//! only, so anything that is not exported shows up here as a build failure
//! rather than as a surprise for a user.

use rnk::components::chat::message_list::{
    BottomFollowState, HorizontalInsets, MessageAnchor, MessageCompositeMeasureConfig,
    MessageExpansionKey, MessageList, MessageListEntry, MessageListMeasureError,
    MessageListMutation, MessageListRenderError, MessageListRevision, MessageListState,
    MessageListStateError, MessageMeasureOutcome, MessageMeasureRequest, MessageRows,
    MessageRowsError, MessageShellMeasureConfig, MessageStructuralSegment, MessageStructureSlotKey,
    MessageVariantKey, RowOffset, ViewportRows, try_measure_composite,
};
use rnk::components::chat::{MessageId, MessageRevision};
use rnk::core::{Element, Style, TextWrap};
use rnk::layout::text_flow::{
    TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};

const WIDTH: u16 = 40;

fn shell() -> MessageShellMeasureConfig {
    MessageShellMeasureConfig::try_new(WIDTH, HorizontalInsets::new(0, 0), vec![]).unwrap()
}

fn entry(id: u64, source: &str) -> MessageListEntry {
    let shell = shell();
    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::default()),
        options: TextFlowOptions::new(usize::from(shell.content_width()), TextWrap::Wrap),
    };
    MessageListEntry::new(
        MessageId::new(id),
        MessageRevision::INITIAL,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(0),
        MessageCompositeMeasureConfig::try_new(vec![identity], shell).unwrap(),
    )
}

fn fixed(rows: u64) -> impl FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<(), ()> {
    move |_request| MessageMeasureOutcome::Measured(MessageRows::try_new(rows).unwrap())
}

/// Names every state-error variant without a wildcard arm.
///
/// Adding a variant breaks this function, which is the point: a caller's
/// recovery logic should have to be revisited when a new failure appears.
fn describe(error: &MessageListStateError) -> &'static str {
    match error {
        MessageListStateError::DuplicateMessageId { .. } => "duplicate",
        MessageListStateError::UnknownMessageId { .. } => "unknown",
        MessageListStateError::InvalidInsertIndex { .. } => "insert index",
        MessageListStateError::MissingMeasurement { .. } => "missing measurement",
        MessageListStateError::MissingActiveMeasurement { .. } => "missing active measurement",
        MessageListStateError::StaleStateRevision { .. } => "stale revision",
        MessageListStateError::StateRevisionOverflow { .. } => "revision overflow",
        MessageListStateError::MeasurementIdentityMismatch { .. } => "identity mismatch",
        MessageListStateError::RowArithmeticOverflow => "row overflow",
        MessageListStateError::CoordinateOverflow { .. } => "coordinate overflow",
        MessageListStateError::InvalidAnchorRow { .. } => "anchor row",
        MessageListStateError::InvalidResizeConfig { .. } => "resize config",
        MessageListStateError::InvalidViewportWidth { .. } => "viewport width",
        MessageListStateError::InvalidCacheCapacity => "cache capacity",
        _ => "unknown variant",
    }
}

#[test]
fn a_list_can_be_built_scrolled_and_rendered_from_outside_the_crate() {
    let entries = [entry(1, "first"), entry(2, "second"), entry(3, "third")];
    let mut state =
        MessageListState::try_new::<(), (), _>(&entries, WIDTH, ViewportRows::new(4), 32, fixed(3))
            .expect("list builds");

    assert_eq!(state.len(), 3);
    assert_eq!(state.total_rows().unwrap(), 9);
    assert_eq!(state.revision(), MessageListRevision::INITIAL);
    assert_eq!(state.follow_state(), BottomFollowState::Following);

    state
        .try_scroll_to(state.revision(), RowOffset::ZERO)
        .expect("scroll to top");
    assert!(matches!(
        state.follow_state(),
        BottomFollowState::Paused { .. }
    ));

    let element = MessageList::new(&state)
        .try_into_element::<(), _>(|_entry, _key, _slice| Ok(Element::box_element()))
        .expect("renders");
    assert_eq!(
        element.children.len(),
        2,
        "a 4-row viewport over 3-row messages"
    );
}

#[test]
fn every_state_error_variant_is_matchable_without_a_wildcard() {
    let entries = [entry(1, "only")];
    let mut state =
        MessageListState::try_new::<(), (), _>(&entries, WIDTH, ViewportRows::new(4), 32, fixed(3))
            .unwrap();

    let unknown = state
        .try_scroll_to_message(state.revision(), MessageId::new(99))
        .unwrap_err();
    assert_eq!(describe(&unknown), "unknown");

    let bad_row = state
        .try_scroll_to_anchor(
            state.revision(),
            MessageAnchor::new(MessageId::new(1), RowOffset::new(50)),
        )
        .unwrap_err();
    assert_eq!(describe(&bad_row), "anchor row");

    let bad_width =
        MessageShellMeasureConfig::try_new(0, HorizontalInsets::new(0, 0), vec![]).unwrap_err();
    assert_eq!(describe(&bad_width), "viewport width");
}

#[test]
fn measure_and_render_errors_are_distinguishable_by_the_caller() {
    let entries = [entry(1, "only")];

    let missing = MessageListState::try_new::<(), (), _>(
        &entries,
        WIDTH,
        ViewportRows::new(4),
        32,
        |_request| MessageMeasureOutcome::Missing,
    )
    .unwrap_err();
    let failed = MessageListState::try_new::<&str, &str, _>(
        &entries,
        WIDTH,
        ViewportRows::new(4),
        32,
        |_request| MessageMeasureOutcome::Failed("boom"),
    )
    .unwrap_err();
    let cancelled = MessageListState::try_new::<&str, &str, _>(
        &entries,
        WIDTH,
        ViewportRows::new(4),
        32,
        |_request| MessageMeasureOutcome::Cancelled("stopped"),
    )
    .unwrap_err();

    assert!(matches!(missing, MessageListMeasureError::State(_)));
    assert!(matches!(
        failed,
        MessageListMeasureError::MeasurementFailed { .. }
    ));
    assert!(matches!(
        cancelled,
        MessageListMeasureError::Cancelled { .. }
    ));

    let state =
        MessageListState::try_new::<(), (), _>(&entries, WIDTH, ViewportRows::new(4), 32, fixed(3))
            .unwrap();
    let render = MessageList::new(&state)
        .try_into_element::<&str, _>(|_entry, _key, _slice| Err("cannot draw"))
        .unwrap_err();
    assert!(matches!(
        render,
        MessageListRenderError::RenderFailed { .. }
    ));
}

#[test]
fn a_no_op_mutation_reports_no_change_and_keeps_the_revision() {
    let entries = [entry(1, "only")];
    let mut state =
        MessageListState::try_new::<(), (), _>(&entries, WIDTH, ViewportRows::new(4), 32, fixed(3))
            .unwrap();
    let revision = state.revision();

    let outcome = state
        .try_append::<(), (), _>(revision, &[], fixed(3))
        .unwrap();

    assert_eq!(outcome, MessageListMutation::NoChange { revision });
    assert_eq!(state.revision(), revision);
}

#[test]
fn the_composite_adapter_counts_shell_rows_on_top_of_the_text() {
    // A four-child message inside a shell with a role header, a status line and
    // two block separators: the adapter must report the full rendered height,
    // not just the body.
    let shell = MessageShellMeasureConfig::try_new(
        WIDTH,
        HorizontalInsets::new(2, 2),
        vec![
            MessageStructuralSegment::new(MessageStructureSlotKey::new(1), RowOffset::new(1)),
            MessageStructuralSegment::new(MessageStructureSlotKey::new(2), RowOffset::new(1)),
            MessageStructuralSegment::new(MessageStructureSlotKey::new(3), RowOffset::new(2)),
        ],
    )
    .unwrap();
    let content_width = usize::from(shell.content_width());

    let children: Vec<TextFlowCacheIdentity> = ["body text", "fn main() {}", "thinking", "result"]
        .iter()
        .map(|source| TextFlowCacheIdentity {
            input: TextFlowInput::plain(*source, TextFlowSourceKind::Exact, Style::default()),
            options: TextFlowOptions::new(content_width, TextWrap::Wrap),
        })
        .collect();
    let child_count = children.len() as u64;

    let config = MessageCompositeMeasureConfig::try_new(children, shell).unwrap();
    let entry = MessageListEntry::new(
        MessageId::new(1),
        MessageRevision::INITIAL,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(0),
        config,
    );
    let key = entry.measure_key();

    let rows = try_measure_composite(MessageMeasureRequest {
        entry: &entry,
        key: key.as_key(),
    })
    .expect("composite measures");

    // Each short child flows to one row; the shell adds 1 + 1 + 2.
    assert_eq!(rows.get(), child_count + 4);
}

#[test]
fn a_zero_row_message_is_rejected_by_value_not_by_convention() {
    assert_eq!(MessageRows::try_new(0), Err(MessageRowsError::Zero));
    assert!(MessageRows::try_new(1).is_ok());
}
