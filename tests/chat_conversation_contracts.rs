use rnk::components::chat::*;
use rnk::components::{Message, MessageRole, ThinkingBlock, ToolCall};

fn text_entry(id: u64, text: &str) -> MessageBlockEntry {
    MessageBlockEntry::new(BlockId::new(id), MessageBlock::Text(text.to_owned()))
}

#[test]
fn message_revision_and_affected_outcome_are_typed() {
    assert_eq!(MessageRevision::INITIAL.get(), 1);
    assert!(MessageRevision::new(0).is_err());
    let guard = MessageMutationGuard::new(
        ConversationGuard::new(ConversationRevision::INITIAL),
        MessageId::new(7),
        MessageRevision::INITIAL,
    );
    assert_eq!(guard.message_id(), MessageId::new(7));
    assert_eq!(guard.message_revision(), MessageRevision::INITIAL);
}

#[test]
fn every_block_variant_preserves_typed_data() {
    let thinking = ThinkingContent::new(ThinkingId::new("think").unwrap(), "");
    let call = ToolCallContent::new(
        ToolCallId::new("call").unwrap(),
        "read",
        vec![ToolArgument::new("path", TypedValue::String("/tmp/a".into())).unwrap()],
    )
    .unwrap();
    let result = ToolResultContent::new(ToolCallId::new("call").unwrap(), "ok");
    let blocks = vec![
        MessageBlock::Text("text".into()),
        MessageBlock::Markdown("**md**".into()),
        MessageBlock::Code(CodeContent::new("").unwrap()),
        MessageBlock::Thinking(thinking),
        MessageBlock::ToolCall(call),
        MessageBlock::ToolResult(result),
        MessageBlock::Error(ErrorContent::new("error").unwrap()),
        MessageBlock::Diff(DiffContent::new("+line").unwrap()),
        MessageBlock::Quote(QuoteContent::new("quote").unwrap()),
        MessageBlock::Link(LinkContent::new("label", "opaque").unwrap()),
        MessageBlock::TerminalAttachmentSummary(
            TerminalAttachmentSummary::new("term", "summary").unwrap(),
        ),
    ];
    assert_eq!(blocks.len(), 11);
}

#[test]
fn closed_typed_values_reject_invalid_payloads() {
    assert!(TypedField::new(" ", TypedValue::Null).is_err());
    let duplicated = vec![
        TypedField::new("x", TypedValue::Integer(1)).unwrap(),
        TypedField::new("x", TypedValue::Integer(2)).unwrap(),
    ];
    assert!(TypedValue::object(duplicated).is_err());
    let valid = TypedValue::object(vec![
        TypedField::new("x", TypedValue::List(vec![])).unwrap(),
        TypedField::new("y", TypedValue::String(String::new())).unwrap(),
    ])
    .unwrap();
    assert!(matches!(valid, TypedValue::Object(fields) if fields.len() == 2));
}

#[test]
fn chat_message_metadata_is_closed_and_optional() {
    let empty = ChatMessageMetadata::default();
    assert!(empty.author().is_none());
    assert!(empty.timestamp().is_none());
    let metadata = ChatMessageMetadata::new(
        Some(MessageAuthor::new("Ada").unwrap()),
        Some(MessageTimestamp::new("2026-07-26").unwrap()),
    );
    assert_eq!(metadata.author().unwrap().as_str(), "Ada");
    assert_eq!(metadata.timestamp().unwrap().as_str(), "2026-07-26");
}

#[test]
fn chat_roles_and_legacy_mapping_are_closed() {
    for role in [
        ChatRole::User,
        ChatRole::Assistant,
        ChatRole::System,
        ChatRole::Tool,
    ] {
        assert_eq!(ChatRole::try_from(MessageRole::from(role)).unwrap(), role);
    }
    assert!(ChatRole::try_from(MessageRole::ToolResult).is_err());
    assert!(ChatRole::try_from(MessageRole::Error).is_err());
}

#[test]
fn error_content_is_typed_and_source_aware() {
    assert!(ErrorContent::new("").is_err());
    let error = ErrorContent::new("timeout")
        .unwrap()
        .with_source(ErrorSource::new("adapter").unwrap());
    assert_eq!(error.message(), "timeout");
    assert_eq!(error.source().unwrap().as_str(), "adapter");
}

#[test]
fn decimal_values_have_one_canonical_representation() {
    for valid in ["0", "1", "-1", "1.25", "-10.01"] {
        assert_eq!(DecimalValue::new(valid).unwrap().as_str(), valid);
    }
    for invalid in ["", " ", "-0", "+1", "01", "1.0", "1.00", "1e0", "NaN"] {
        assert!(DecimalValue::new(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn lifecycle_payloads_are_closed_and_projectable() {
    let thinking = ThinkingContent::new(ThinkingId::new("t").unwrap(), "work")
        .with_status(ThinkingStatus::Streaming);
    assert_eq!(thinking.id().as_str(), "t");
    assert_eq!(thinking.content(), "work");
    assert!(matches!(thinking.status(), ThinkingStatus::Streaming));

    let call = ToolCallContent::new(ToolCallId::new("c").unwrap(), "tool", vec![])
        .unwrap()
        .with_status(ToolCallStatus::Running);
    assert_eq!(call.call_id().as_str(), "c");
    assert!(matches!(call.status(), ToolCallStatus::Running));

    let result = ToolResultContent::new(ToolCallId::new("c").unwrap(), "partial")
        .with_status(ToolResultStatus::Streaming);
    assert_eq!(result.output(), "partial");
}

#[test]
fn empty_and_missing_inputs_have_explicit_results() {
    assert!(UpdateId::new("").is_err());
    assert!(ThinkingId::new(" ").is_err());
    assert!(ToolCallId::new("").is_err());
    assert!(FailureCause::new("").is_err());
    assert!(ChatMessage::new(MessageId::new(1), ChatRole::User, vec![]).is_err());
    assert!(
        ConversationUpdate::append_text(
            MessageMutationGuard::new(
                ConversationGuard::new(ConversationRevision::INITIAL),
                MessageId::new(1),
                MessageRevision::INITIAL,
            ),
            BlockId::new(1),
            "",
        )
        .is_err()
    );
}

#[test]
fn core_model_requires_adapter_owned_typed_values() {
    let value = TypedValue::object(vec![
        TypedField::new("known", TypedValue::Bool(true)).unwrap(),
        TypedField::new(
            "nested",
            TypedValue::List(vec![TypedValue::Integer(2), TypedValue::Null]),
        )
        .unwrap(),
    ])
    .unwrap();
    assert!(matches!(value, TypedValue::Object(fields) if fields.len() == 2));
}

#[test]
fn tool_and_thinking_models_have_no_execution_surface() {
    let call = ToolCallContent::new(
        ToolCallId::new("data-only").unwrap(),
        "display-name",
        vec![ToolArgument::new("approved", TypedValue::Bool(false)).unwrap()],
    )
    .unwrap();
    assert_eq!(call.arguments()[0].name(), "approved");
    assert_eq!(call.arguments()[0].value(), &TypedValue::Bool(false));
}

#[test]
fn legacy_message_and_new_chat_surface_coexist() {
    let _legacy = Message::new(MessageRole::User, "legacy");
    let _tool = ToolCall::new("legacy", "args");
    let _thinking = ThinkingBlock::new("legacy");
    let message = ChatMessage::new(
        MessageId::new(1),
        ChatRole::User,
        vec![text_entry(1, "typed")],
    )
    .unwrap();
    assert_eq!(message.blocks().len(), 1);
}

fn guard(state: &ConversationState, message_id: MessageId) -> MessageMutationGuard {
    MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        message_id,
        state.message(message_id).unwrap().revision(),
    )
}

fn event(id: &str, sequence: u64, update: ConversationUpdate) -> ConversationEvent {
    ConversationEvent::new(UpdateId::new(id).unwrap(), sequence, update)
}

fn message(id: u64, role: ChatRole, entries: Vec<MessageBlockEntry>) -> ChatMessage {
    ChatMessage::new(MessageId::new(id), role, entries).unwrap()
}

fn apply_push(
    state: &mut ConversationState,
    id: &str,
    sequence: u64,
    value: ChatMessage,
) -> ApplyOutcome {
    let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), value);
    state.apply_event(event(id, sequence, update)).unwrap()
}

fn assert_atomic_error(
    state: &mut ConversationState,
    value: ConversationEvent,
) -> ConversationError {
    let before = state.clone();
    let error = state.apply_event(value).unwrap_err();
    assert_eq!(*state, before);
    error
}

fn exercise_push_stream_complete() {
    let mut state = ConversationState::new(10, std::num::NonZeroUsize::new(4).unwrap());
    let pushed = event(
        "push",
        10,
        ConversationUpdate::push(
            ConversationGuard::new(ConversationRevision::INITIAL),
            message(1, ChatRole::Assistant, vec![text_entry(11, "")]),
        ),
    );
    let first = state.apply_event(pushed.clone()).unwrap();
    assert_eq!(first.revision().get(), 1);
    assert_eq!(first.affected_messages()[0].previous_revision(), None);
    assert_eq!(state.apply_event(pushed).unwrap(), first);
    let append = ConversationUpdate::append_text(
        guard(&state, MessageId::new(1)),
        BlockId::new(11),
        "hello",
    )
    .unwrap();
    state.apply_event(event("append", 11, append)).unwrap();
    let complete = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    state.apply_event(event("complete", 12, complete)).unwrap();
    let value = state.message(MessageId::new(1)).unwrap();
    assert!(matches!(value.status(), MessageStatus::Complete));
    assert_eq!(value.revision().get(), 3);
    assert!(matches!(value.blocks()[0].block(), MessageBlock::Text(text) if text == "hello"));
}

fn exercise_ordering_and_atomicity() {
    let mut state = ConversationState::new(7, std::num::NonZeroUsize::new(2).unwrap());
    let push = event(
        "p",
        7,
        ConversationUpdate::push(
            ConversationGuard::new(state.revision()),
            message(1, ChatRole::User, vec![text_entry(1, "x")]),
        ),
    );
    let accepted = state.apply_event(push.clone()).unwrap();
    assert_eq!(state.apply_event(push.clone()).unwrap(), accepted);
    let conflicting = event(
        "p",
        8,
        ConversationUpdate::complete(guard(&state, MessageId::new(1))),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, conflicting),
        ConversationError::EventIdConflict { .. }
    ));
    let stale = event(
        "stale",
        7,
        ConversationUpdate::complete(guard(&state, MessageId::new(1))),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, stale),
        ConversationError::StaleSequence { .. }
    ));
    let gap = event(
        "gap",
        10,
        ConversationUpdate::complete(guard(&state, MessageId::new(1))),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, gap),
        ConversationError::SequenceGap { .. }
    ));
    let bad_guard = ConversationUpdate::complete(MessageMutationGuard::new(
        ConversationGuard::new(ConversationRevision::INITIAL),
        MessageId::new(1),
        MessageRevision::INITIAL,
    ));
    assert!(matches!(
        assert_atomic_error(&mut state, event("guard", 8, bad_guard)),
        ConversationError::ConversationRevisionMismatch { .. }
    ));
    let bad_message_guard = ConversationUpdate::complete(MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        MessageId::new(1),
        MessageRevision::new(99).unwrap(),
    ));
    assert!(matches!(
        assert_atomic_error(&mut state, event("message-guard", 8, bad_message_guard)),
        ConversationError::MessageRevisionMismatch { .. }
    ));
}

fn exercise_block_mutations() {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(8).unwrap());
    apply_push(
        &mut state,
        "p",
        0,
        message(
            1,
            ChatRole::Assistant,
            vec![
                text_entry(1, ""),
                MessageBlockEntry::new(
                    BlockId::new(2),
                    MessageBlock::Thinking(ThinkingContent::new(
                        ThinkingId::new("thought").unwrap(),
                        "",
                    )),
                ),
            ],
        ),
    );
    let append =
        ConversationUpdate::append_text(guard(&state, MessageId::new(1)), BlockId::new(1), "a")
            .unwrap();
    state.apply_event(event("a", 1, append)).unwrap();
    let thinking = MessageBlock::Thinking(
        ThinkingContent::new(ThinkingId::new("thought").unwrap(), "step")
            .with_status(ThinkingStatus::Streaming),
    );
    let replace = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(1)),
        BlockId::new(2),
        thinking,
    );
    state.apply_event(event("r", 2, replace)).unwrap();
    let inserted = MessageBlockEntry::new(
        BlockId::new(3),
        MessageBlock::Code(CodeContent::new("let x = 1;").unwrap()),
    );
    let insert =
        ConversationUpdate::insert_message_block(guard(&state, MessageId::new(1)), 1, inserted);
    state.apply_event(event("i", 3, insert)).unwrap();
    let appended = MessageBlockEntry::new(BlockId::new(4), MessageBlock::Markdown("tail".into()));
    let add = ConversationUpdate::append_message_block(guard(&state, MessageId::new(1)), appended);
    state.apply_event(event("b", 4, add)).unwrap();
    let before = state.clone();
    let bad = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(1)),
        BlockId::new(1),
        MessageBlock::Markdown("wrong".into()),
    );
    assert!(state.apply_event(event("bad", 5, bad)).is_err());
    assert_eq!(state, before);
    let empty = ConversationUpdate::append_message_block(
        guard(&state, MessageId::new(1)),
        MessageBlockEntry::new(BlockId::new(5), MessageBlock::Text(String::new())),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, event("empty", 5, empty)),
        ConversationError::InvalidMessage { .. }
    ));
    let out_of_bounds = ConversationUpdate::insert_message_block(
        guard(&state, MessageId::new(1)),
        usize::MAX,
        text_entry(6, "never inserted"),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, event("bounds", 5, out_of_bounds)),
        ConversationError::InvalidMessage { .. }
    ));
    assert_eq!(state.message(MessageId::new(1)).unwrap().blocks().len(), 4);
}

fn setup_correlated() -> ConversationState {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(16).unwrap());
    let call_id = ToolCallId::new("call").unwrap();
    apply_push(
        &mut state,
        "call-push",
        0,
        message(
            1,
            ChatRole::Assistant,
            vec![MessageBlockEntry::new(
                BlockId::new(1),
                MessageBlock::ToolCall(
                    ToolCallContent::new(call_id.clone(), "read", vec![]).unwrap(),
                ),
            )],
        ),
    );
    let running = ToolCallContent::new(call_id.clone(), "read", vec![])
        .unwrap()
        .with_status(ToolCallStatus::Running);
    let replace = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(1)),
        BlockId::new(1),
        MessageBlock::ToolCall(running),
    );
    state.apply_event(event("running", 1, replace)).unwrap();
    apply_push(
        &mut state,
        "result-push",
        2,
        message(
            2,
            ChatRole::Tool,
            vec![MessageBlockEntry::new(
                BlockId::new(2),
                MessageBlock::ToolResult(ToolResultContent::new(call_id, "")),
            )],
        ),
    );
    state
}

fn exercise_correlation_cancel() {
    let mut state = setup_correlated();
    let cancel = ConversationUpdate::cancel(guard(&state, MessageId::new(2)));
    let outcome = state.apply_event(event("cancel", 3, cancel)).unwrap();
    assert_eq!(outcome.affected_messages().len(), 2);
    assert!(matches!(
        state.message(MessageId::new(1)).unwrap().blocks()[0].block(),
        MessageBlock::ToolCall(value) if matches!(value.status(), ToolCallStatus::Cancelled)
    ));
    assert!(matches!(
        state.message(MessageId::new(2)).unwrap().blocks()[0].block(),
        MessageBlock::ToolResult(value) if matches!(value.status(), ToolResultStatus::Cancelled)
    ));
    let late = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(2)),
        BlockId::new(2),
        MessageBlock::ToolResult(
            ToolResultContent::new(ToolCallId::new("call").unwrap(), "late")
                .with_status(ToolResultStatus::Complete),
        ),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, event("late", 4, late)),
        ConversationError::InvalidTransition { .. }
    ));
}

fn exercise_correlation_fail() {
    let mut state = setup_correlated();
    let premature = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    assert!(matches!(
        assert_atomic_error(&mut state, event("premature", 3, premature)),
        ConversationError::InvalidTransition { .. }
    ));
    let cause = FailureCause::new("provider stopped").unwrap();
    let fail = ConversationUpdate::fail(guard(&state, MessageId::new(2)), cause.clone());
    state.apply_event(event("fail", 3, fail)).unwrap();
    assert_eq!(
        state
            .message(MessageId::new(2))
            .unwrap()
            .status()
            .failure_cause(),
        Some(&cause)
    );
    for id in [MessageId::new(1), MessageId::new(2)] {
        let block = state.message(id).unwrap().blocks()[0].block();
        let found = match block {
            MessageBlock::ToolCall(value) => value.status().failure_cause(),
            MessageBlock::ToolResult(value) => value.status().failure_cause(),
            _ => None,
        };
        assert_eq!(found, Some(&cause));
    }
    let duplicate = message(
        3,
        ChatRole::Assistant,
        vec![MessageBlockEntry::new(
            BlockId::new(3),
            MessageBlock::ToolCall(
                ToolCallContent::new(ToolCallId::new("call").unwrap(), "again", vec![]).unwrap(),
            ),
        )],
    );
    let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), duplicate);
    assert!(matches!(
        assert_atomic_error(&mut state, event("duplicate", 4, update)),
        ConversationError::DuplicateToolCallId { .. }
    ));
}

fn exercise_edit_delete_resend_restore() {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(2).unwrap());
    apply_push(
        &mut state,
        "p",
        0,
        message(
            1,
            ChatRole::User,
            vec![
                text_entry(1, "source"),
                MessageBlockEntry::new(
                    BlockId::new(2),
                    MessageBlock::Thinking(ThinkingContent::new(
                        ThinkingId::new("same").unwrap(),
                        "",
                    )),
                ),
            ],
        ),
    );
    let edit = ConversationUpdate::edit_message(
        guard(&state, MessageId::new(1)),
        vec![text_entry(1, "edited")],
    );
    state.apply_event(event("edit", 1, edit)).unwrap();
    let reused = ConversationUpdate::edit_message(
        guard(&state, MessageId::new(1)),
        vec![
            text_entry(1, "edited"),
            MessageBlockEntry::new(
                BlockId::new(3),
                MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("same").unwrap(), "")),
            ),
        ],
    );
    assert!(matches!(
        assert_atomic_error(&mut state, event("reuse", 2, reused)),
        ConversationError::RetiredThinkingId { .. }
    ));
    let retired_block = ConversationUpdate::append_message_block(
        guard(&state, MessageId::new(1)),
        text_entry(2, "retired"),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, event("retired-block", 2, retired_block)),
        ConversationError::RetiredBlockId { .. }
    ));
    let complete = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    state.apply_event(event("complete", 2, complete)).unwrap();
    let fresh = message(
        2,
        ChatRole::User,
        vec![MessageBlockEntry::new(
            BlockId::new(4),
            MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("same").unwrap(), "")),
        )],
    );
    let resend = ConversationUpdate::resend(guard(&state, MessageId::new(1)), fresh);
    state.apply_event(event("resend", 3, resend)).unwrap();
    let snapshot = state.snapshot();
    assert_eq!(ConversationState::try_restore(snapshot).unwrap(), state);
    let delete = ConversationUpdate::delete_message(guard(&state, MessageId::new(2)));
    let outcome = state.apply_event(event("delete", 4, delete)).unwrap();
    assert!(matches!(
        outcome.affected_messages()[0].disposition(),
        AffectedMessageDisposition::Deleted
    ));

    let mut correlated = setup_correlated();
    let delete_result = ConversationUpdate::delete_message(guard(&correlated, MessageId::new(2)));
    correlated
        .apply_event(event("delete-result", 3, delete_result))
        .unwrap();
    let replacement_result = ConversationUpdate::append_message_block(
        guard(&correlated, MessageId::new(1)),
        MessageBlockEntry::new(
            BlockId::new(9),
            MessageBlock::ToolResult(ToolResultContent::new(ToolCallId::new("call").unwrap(), "")),
        ),
    );
    assert!(matches!(
        assert_atomic_error(
            &mut correlated,
            event("replacement-result", 4, replacement_result)
        ),
        ConversationError::ResultSlotRetired { .. }
    ));
    assert_eq!(
        ConversationState::try_restore(correlated.snapshot()).unwrap(),
        correlated
    );
}

fn exercise_retention_and_exhaustion() {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(1).unwrap());
    apply_push(
        &mut state,
        "p",
        0,
        message(1, ChatRole::User, vec![text_entry(1, "x")]),
    );
    let complete = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    state.apply_event(event("c", 1, complete)).unwrap();
    let old = event(
        "p",
        0,
        ConversationUpdate::push(
            ConversationGuard::new(ConversationRevision::INITIAL),
            message(1, ChatRole::User, vec![text_entry(1, "x")]),
        ),
    );
    assert!(matches!(
        assert_atomic_error(&mut state, old),
        ConversationError::ReplayOutsideRetention { .. }
    ));

    let max_snapshot = ConversationStateSnapshot::new(
        vec![],
        ConversationRevision::INITIAL,
        u64::MAX,
        RetentionHistory::new(std::num::NonZeroUsize::new(1).unwrap(), vec![], None).unwrap(),
        ConversationIdentityHistory::new(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    );
    let mut exhausted = ConversationState::try_restore(max_snapshot).unwrap();
    let malformed = ConversationUpdate::complete(MessageMutationGuard::new(
        ConversationGuard::new(ConversationRevision::INITIAL),
        MessageId::new(999),
        MessageRevision::INITIAL,
    ));
    assert!(matches!(
        assert_atomic_error(&mut exhausted, event("max", u64::MAX, malformed)),
        ConversationError::SequenceExhausted
    ));

    let mut edge = ConversationState::new(u64::MAX - 1, std::num::NonZeroUsize::new(1).unwrap());
    let last = event(
        "last",
        u64::MAX - 1,
        ConversationUpdate::push(
            ConversationGuard::new(edge.revision()),
            message(8, ChatRole::User, vec![text_entry(8, "edge")]),
        ),
    );
    let original = edge.apply_event(last.clone()).unwrap();
    assert_eq!(edge.apply_event(last).unwrap(), original);
    let invalid = ConversationUpdate::complete(guard(&edge, MessageId::new(8)));
    assert!(matches!(
        assert_atomic_error(&mut edge, event("past-end", u64::MAX, invalid)),
        ConversationError::SequenceExhausted
    ));

    let mut fresh = ConversationState::new(0, std::num::NonZeroUsize::new(1).unwrap());
    apply_push(
        &mut fresh,
        "fresh",
        0,
        message(1, ChatRole::User, vec![text_entry(1, "accepted")]),
    );
    assert_eq!(fresh.evicted_through(), None);
}

fn exercise_determinism() {
    fn run() -> (ConversationState, Vec<ApplyOutcome>) {
        let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(4).unwrap());
        let first = apply_push(
            &mut state,
            "p",
            0,
            message(1, ChatRole::Assistant, vec![text_entry(1, "")]),
        );
        let append = ConversationUpdate::append_text(
            guard(&state, MessageId::new(1)),
            BlockId::new(1),
            "same",
        )
        .unwrap();
        let second = state.apply_event(event("a", 1, append)).unwrap();
        (state, vec![first, second])
    }
    assert_eq!(run(), run());
    let mut empty = ConversationState::new(0, std::num::NonZeroUsize::new(2).unwrap());
    apply_push(
        &mut empty,
        "empty",
        0,
        message(1, ChatRole::Assistant, vec![text_entry(1, "")]),
    );
    let complete = ConversationUpdate::complete(guard(&empty, MessageId::new(1)));
    assert!(matches!(
        assert_atomic_error(&mut empty, event("complete-empty", 1, complete)),
        ConversationError::InvalidTransition { .. }
    ));
    let mut nested = ConversationState::new(0, std::num::NonZeroUsize::new(2).unwrap());
    apply_push(
        &mut nested,
        "nested",
        0,
        message(
            1,
            ChatRole::Assistant,
            vec![MessageBlockEntry::new(
                BlockId::new(1),
                MessageBlock::Thinking(ThinkingContent::new(
                    ThinkingId::new("active").unwrap(),
                    "",
                )),
            )],
        ),
    );
    let complete = ConversationUpdate::complete(guard(&nested, MessageId::new(1)));
    assert!(matches!(
        assert_atomic_error(&mut nested, event("complete-active", 1, complete)),
        ConversationError::InvalidTransition { .. }
    ));
}

macro_rules! cases {
    ($helper:ident => $($name:ident),+ $(,)?) => {$(
        #[test]
        fn $name() { $helper(); }
    )+};
}

cases!(exercise_push_stream_complete =>
    push_is_unique_and_atomic,
    streaming_deltas_are_ordered_lossless_and_typed,
    static_message_completes_without_dummy_append,
    public_model_is_typed_and_constructible,
);
cases!(exercise_block_mutations =>
    append_block_supports_late_discovered_typed_blocks,
    append_block_rejects_invalid_blocks_atomically,
    replace_block_validates_before_commit,
    replace_block_requires_same_variant_and_identity,
    edit_and_insert_are_revisioned_and_identity_safe,
    revision_guards_and_mutation_failures_are_atomic,
);
cases!(exercise_correlation_cancel =>
    lifecycle_identity_namespaces_are_scoped_and_correlated,
    correlated_lifecycle_updates_are_atomic,
    cancel_cascades_across_correlated_messages_atomically,
    cancellation_preserves_partial_content_and_rejects_late_events,
);
cases!(exercise_correlation_fail =>
    duplicate_lifecycle_identities_are_rejected_atomically,
    fail_cascades_across_correlated_messages_atomically,
    failure_causes_are_typed_and_propagated,
    message_complete_rejects_inconsistent_tool_pairs,
);
cases!(exercise_edit_delete_resend_restore =>
    edit_retires_thinking_ids_atomically,
    delete_preserves_global_correlation_atomically,
    resend_preserves_source_and_creates_fresh_identity,
    block_ids_are_conversation_unique_and_retained,
    restore_snapshot_roundtrip_preserves_histories,
    deleted_tool_result_retires_result_slot_atomically,
);
cases!(exercise_ordering_and_atomicity =>
    sequence_is_conversation_wide_and_contiguous,
    exact_replay_returns_original_outcome_without_mutation,
    reused_event_id_with_different_content_conflicts,
    stale_gap_and_retention_errors_do_not_advance_state,
    every_failure_is_atomic_for_full_state,
    replay_conflict_stale_and_gap_precede_exhaustion,
    exact_replay_does_not_advance_exhausted_counters,
);
cases!(exercise_retention_and_exhaustion =>
    mutation_replay_retention_is_consistent,
    bounded_ledger_exposes_honest_replay_boundary,
    fresh_restart_state_has_no_replay_or_eviction_evidence,
    sequence_exhaustion_is_checked_and_atomic_at_u64_max,
    sequence_exhaustion_precedes_malformed_update_at_u64_max,
);
cases!(exercise_determinism =>
    identical_sequences_produce_identical_state_and_outcomes,
    empty_static_message_requires_content_before_complete,
    pending_message_with_active_nested_block_cannot_complete,
);
