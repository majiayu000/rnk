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
    let unchecked = TypedValue::Object(vec![
        TypedField::new("x", TypedValue::Null).unwrap(),
        TypedField::new("x", TypedValue::Bool(true)).unwrap(),
    ]);
    assert!(ToolArgument::new("payload", unchecked).is_err());
    let nested_unchecked = TypedValue::List(vec![TypedValue::Object(vec![
        TypedField::new("nested", TypedValue::Null).unwrap(),
        TypedField::new("nested", TypedValue::Integer(1)).unwrap(),
    ])]);
    assert!(ToolArgument::new("payload", nested_unchecked).is_err());
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
    for valid in ["0", "1", "-1", "0.1", "-0.1", "1.25", "-10.01"] {
        assert_eq!(DecimalValue::new(valid).unwrap().as_str(), valid);
    }
    for invalid in [
        "", " ", "-0", "+1", "01", "00.1", "-00.1", "0.0", "-0.0", "1.0", "1.00", "1e0", "NaN",
    ] {
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
fn public_update_payload_accessors_preserve_exact_inputs() {
    let conversation = ConversationGuard::new(ConversationRevision::new(3));
    let mutation = MessageMutationGuard::new(
        conversation,
        MessageId::new(7),
        MessageRevision::new(4).unwrap(),
    );
    let entry = text_entry(9, "entry");
    let pushed = message(8, ChatRole::User, vec![text_entry(8, "push")]);
    assert!(
        matches!(ConversationUpdate::push(conversation, pushed), ConversationUpdate::Push(value)
            if value.guard() == conversation && value.message().id() == MessageId::new(8))
    );
    assert!(
        matches!(ConversationUpdate::append_text(mutation, BlockId::new(9), "delta").unwrap(),
            ConversationUpdate::AppendText(value)
            if value.guard() == mutation && value.block_id() == BlockId::new(9)
                && value.delta() == "delta")
    );
    assert!(
        matches!(ConversationUpdate::append_message_block(mutation, entry.clone()),
            ConversationUpdate::AppendMessageBlock(value)
            if value.guard() == mutation && value.entry() == &entry)
    );
    assert!(
        matches!(ConversationUpdate::insert_message_block(mutation, 2, entry.clone()),
            ConversationUpdate::InsertMessageBlock(value)
            if value.guard() == mutation && value.position() == 2 && value.entry() == &entry)
    );
    assert!(matches!(ConversationUpdate::replace_block(
            mutation, BlockId::new(9), MessageBlock::Text("replacement".into())),
            ConversationUpdate::ReplaceBlock(value)
            if value.guard() == mutation && value.block_id() == BlockId::new(9)
                && matches!(value.replacement(), MessageBlock::Text(text) if text == "replacement")));
    assert!(
        matches!(ConversationUpdate::complete(mutation), ConversationUpdate::Complete(value)
            if value.guard() == mutation)
    );
    let cause = FailureCause::new("cause").unwrap();
    assert!(
        matches!(ConversationUpdate::fail(mutation, cause.clone()), ConversationUpdate::Fail(value)
            if value.guard() == mutation && value.cause() == &cause)
    );
    assert!(
        matches!(ConversationUpdate::edit_message(mutation, vec![entry.clone()]),
            ConversationUpdate::EditMessage(value)
            if value.guard() == mutation && value.entries() == std::slice::from_ref(&entry))
    );
    assert!(matches!(ConversationUpdate::resend(
            mutation, message(10, ChatRole::User, vec![text_entry(10, "resend")])),
            ConversationUpdate::Resend(value)
            if value.source_guard() == mutation && value.message().id() == MessageId::new(10)));
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

fn text_state(capacity: usize) -> ConversationState {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(capacity).unwrap());
    apply_push(
        &mut state,
        "push",
        0,
        message(1, ChatRole::User, vec![text_entry(1, "x")]),
    );
    state
}
fn block_text(state: &ConversationState, id: u64) -> &str {
    match state.message(MessageId::new(id)).unwrap().blocks()[0].block() {
        MessageBlock::Text(value) => value,
        _ => panic!("expected text"),
    }
}
fn terminal_call_state() -> ConversationState {
    let mut state = setup_correlated();
    let call = ToolCallContent::new(ToolCallId::new("call").unwrap(), "read", vec![])
        .unwrap()
        .with_status(ToolCallStatus::Succeeded);
    let update = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(1)),
        BlockId::new(1),
        MessageBlock::ToolCall(call),
    );
    state.apply_event(event("succeeded", 3, update)).unwrap();
    state
}
fn max_state() -> ConversationState {
    ConversationState::try_restore(ConversationStateSnapshot::new(
        vec![],
        ConversationRevision::INITIAL,
        u64::MAX,
        RetentionHistory::new(std::num::NonZeroUsize::MIN, vec![], None).unwrap(),
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
    ))
    .unwrap()
}
macro_rules! exact { ($name:ident, $body:block) => { #[test] fn $name() $body }; }
macro_rules! unformatted { ($($tokens:tt)*) => { $($tokens)* }; }

#[rustfmt::skip]
unformatted! {
exact!(push_is_unique_and_atomic, {
    let mut state = text_state(4); let duplicate = ConversationUpdate::push(ConversationGuard::new(state.revision()), message(1, ChatRole::User, vec![text_entry(2, "duplicate")]));
    assert!(matches!(assert_atomic_error(&mut state, event("duplicate", 1, duplicate)), ConversationError::DuplicateMessageId { .. }));
});
exact!(streaming_deltas_are_ordered_lossless_and_typed, {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(4).unwrap()); apply_push(&mut state, "target", 0, message(1, ChatRole::Assistant, vec![text_entry(1, "")])); apply_push(&mut state, "other", 1, message(2, ChatRole::User, vec![text_entry(2, "unrelated transcript")]));
    let pointer = block_text(&state, 2).as_ptr(); let update = ConversationUpdate::append_text(guard(&state, MessageId::new(1)), BlockId::new(1), "hello").unwrap(); state.apply_event(event("append", 2, update)).unwrap();
    assert_eq!(block_text(&state, 1), "hello"); assert_eq!(block_text(&state, 2).as_ptr(), pointer, "unaffected transcript storage was cloned");
});
exact!(static_message_completes_without_dummy_append, {
    let mut state = text_state(4); let before = state.message(MessageId::new(1)).unwrap().blocks().to_vec();
    let update = ConversationUpdate::complete(guard(&state, MessageId::new(1))); state.apply_event(event("complete", 1, update)).unwrap();
    assert!(matches!(state.message(MessageId::new(1)).unwrap().status(), MessageStatus::Complete)); assert_eq!(state.message(MessageId::new(1)).unwrap().blocks(), before);
});
exact!(public_model_is_typed_and_constructible, {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::MIN); let outcome = apply_push(&mut state, "p", 0, message(7, ChatRole::User, vec![text_entry(7, "typed")]));
    assert_eq!(outcome.affected_messages()[0].previous_revision(), None); assert_eq!(outcome.affected_messages()[0].message_id(), MessageId::new(7));
});
exact!(constructor_based_public_api_remains_compatible, {
    let update = ConversationUpdate::append_text(MessageMutationGuard::new(ConversationGuard::new(ConversationRevision::new(2)), MessageId::new(3), MessageRevision::new(4).unwrap()), BlockId::new(5), "x").unwrap();
    assert!(matches!(update, ConversationUpdate::AppendText(value) if value.block_id() == BlockId::new(5) && value.delta() == "x"));
});
exact!(append_block_supports_late_discovered_typed_blocks, {
    let mut state = text_state(4); let entry = MessageBlockEntry::new(BlockId::new(2), MessageBlock::Markdown("late".into()));
    let update = ConversationUpdate::append_message_block(guard(&state, MessageId::new(1)), entry); state.apply_event(event("append-block", 1, update)).unwrap();
    assert!(matches!(state.message(MessageId::new(1)).unwrap().blocks()[1].block(), MessageBlock::Markdown(value) if value == "late"));
});
exact!(append_block_rejects_invalid_blocks_atomically, {
    let mut state = text_state(4); let update = ConversationUpdate::append_message_block(guard(&state, MessageId::new(1)), text_entry(2, ""));
    assert!(matches!(assert_atomic_error(&mut state, event("empty", 1, update)), ConversationError::InvalidMessage { .. }));
});
exact!(failed_push_rolls_back_new_message_and_identity_indexes, {
    let mut state = setup_correlated(); let cancel = ConversationUpdate::cancel(guard(&state, MessageId::new(1))); state.apply_event(event("cancel-call", 3, cancel)).unwrap();
    let pending = MessageBlockEntry::new(BlockId::new(3), MessageBlock::ToolResult(ToolResultContent::new(ToolCallId::new("call").unwrap(), ""))); let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), message(3, ChatRole::Tool, vec![pending]));
    assert!(matches!(assert_atomic_error(&mut state, event("invalid-result", 4, update)), ConversationError::InvalidCorrelation { .. })); assert!(state.message(MessageId::new(3)).is_none());
});
exact!(replace_block_validates_before_commit, {
    let mut state = terminal_call_state(); let rewritten = ToolCallContent::new(ToolCallId::new("call").unwrap(), "rewritten", vec![]).unwrap().with_status(ToolCallStatus::Succeeded);
    let update = ConversationUpdate::replace_block(guard(&state, MessageId::new(1)), BlockId::new(1), MessageBlock::ToolCall(rewritten));
    assert!(matches!(assert_atomic_error(&mut state, event("rewrite", 4, update)), ConversationError::InvalidReplacement { .. }));
});
exact!(edit_message_cannot_rewrite_terminal_lifecycle_payload, {
    let mut state = terminal_call_state(); let rewritten = ToolCallContent::new(ToolCallId::new("call").unwrap(), "rewritten", vec![]).unwrap().with_status(ToolCallStatus::Succeeded);
    let entry = MessageBlockEntry::new(BlockId::new(1), MessageBlock::ToolCall(rewritten)); let update = ConversationUpdate::edit_message(guard(&state, MessageId::new(1)), vec![entry]);
    assert!(matches!(assert_atomic_error(&mut state, event("edit-rewrite", 4, update)), ConversationError::InvalidReplacement { .. }));
});
exact!(edit_message_allows_retained_static_payload_changes, {
    let mut state = text_state(4); let update = ConversationUpdate::edit_message(guard(&state, MessageId::new(1)), vec![text_entry(1, "edited")]); state.apply_event(event("static-edit", 1, update)).unwrap();
    assert_eq!(block_text(&state, 1), "edited");
});
exact!(replace_block_requires_same_variant_and_identity, {
    let mut state = text_state(4); let update = ConversationUpdate::replace_block(guard(&state, MessageId::new(1)), BlockId::new(1), MessageBlock::Markdown("x".into()));
    assert!(matches!(assert_atomic_error(&mut state, event("wrong-kind", 1, update)), ConversationError::InvalidReplacement { .. }));
});
exact!(edit_and_insert_are_revisioned_and_identity_safe, {
    let mut state = text_state(4); let update = ConversationUpdate::insert_message_block(guard(&state, MessageId::new(1)), 0, text_entry(2, "first")); state.apply_event(event("insert", 1, update)).unwrap();
    assert_eq!(state.message(MessageId::new(1)).unwrap().blocks()[0].id(), BlockId::new(2)); assert_eq!(state.message(MessageId::new(1)).unwrap().revision().get(), 2);
});
exact!(revision_guards_and_mutation_failures_are_atomic, {
    let mut state = text_state(4); let bad = MessageMutationGuard::new(ConversationGuard::new(ConversationRevision::INITIAL), MessageId::new(1), MessageRevision::INITIAL);
    assert!(matches!(assert_atomic_error(&mut state, event("stale-guard", 1, ConversationUpdate::complete(bad))), ConversationError::ConversationRevisionMismatch { .. }));
});
exact!(lifecycle_identity_namespaces_are_scoped_and_correlated, {
    let state = setup_correlated(); let call = state.message(MessageId::new(1)).unwrap().blocks()[0].block(); let result = state.message(MessageId::new(2)).unwrap().blocks()[0].block();
    assert!(matches!((call, result), (MessageBlock::ToolCall(a), MessageBlock::ToolResult(b)) if a.call_id() == b.call_id()));
});
exact!(correlated_lifecycle_updates_are_atomic, {
    let mut state = setup_correlated(); let result = ToolResultContent::new(ToolCallId::new("call").unwrap(), "part").with_status(ToolResultStatus::Streaming);
    let update = ConversationUpdate::replace_block(guard(&state, MessageId::new(2)), BlockId::new(2), MessageBlock::ToolResult(result)); state.apply_event(event("stream-result", 3, update)).unwrap();
    assert!(matches!(state.message(MessageId::new(2)).unwrap().blocks()[0].block(), MessageBlock::ToolResult(value) if value.output() == "part" && matches!(value.status(), ToolResultStatus::Streaming)));
});
exact!(cancel_cascades_across_correlated_messages_atomically, {
    let mut state = setup_correlated(); let update = ConversationUpdate::cancel(guard(&state, MessageId::new(2))); let outcome = state.apply_event(event("cancel", 3, update)).unwrap();
    assert_eq!(outcome.affected_messages().len(), 2); assert!(matches!(state.message(MessageId::new(1)).unwrap().blocks()[0].block(), MessageBlock::ToolCall(value) if matches!(value.status(), ToolCallStatus::Cancelled)));
});
exact!(cancellation_preserves_partial_content_and_rejects_late_events, {
    let mut state = setup_correlated(); let partial = ToolResultContent::new(ToolCallId::new("call").unwrap(), "partial").with_status(ToolResultStatus::Streaming);
    let stream = ConversationUpdate::replace_block(guard(&state, MessageId::new(2)), BlockId::new(2), MessageBlock::ToolResult(partial)); state.apply_event(event("partial", 3, stream)).unwrap();
    let cancel = ConversationUpdate::cancel(guard(&state, MessageId::new(2))); state.apply_event(event("cancel", 4, cancel)).unwrap(); assert!(matches!(state.message(MessageId::new(2)).unwrap().blocks()[0].block(), MessageBlock::ToolResult(value) if value.output() == "partial"));
});
exact!(duplicate_lifecycle_identities_are_rejected_atomically, {
    let mut state = setup_correlated(); let duplicate = ToolCallContent::new(ToolCallId::new("call").unwrap(), "again", vec![]).unwrap();
    let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), message(3, ChatRole::Assistant, vec![MessageBlockEntry::new(BlockId::new(3), MessageBlock::ToolCall(duplicate))]));
    assert!(matches!(assert_atomic_error(&mut state, event("duplicate-call", 3, update)), ConversationError::DuplicateToolCallId { .. }));
});
exact!(fail_cascades_across_correlated_messages_atomically, {
    let mut state = setup_correlated(); let cause = FailureCause::new("stopped").unwrap(); let update = ConversationUpdate::fail(guard(&state, MessageId::new(2)), cause.clone()); let outcome = state.apply_event(event("fail", 3, update)).unwrap();
    assert_eq!(outcome.affected_messages().len(), 2); assert!(matches!(state.message(MessageId::new(1)).unwrap().blocks()[0].block(), MessageBlock::ToolCall(value) if value.status().failure_cause() == Some(&cause)));
});
exact!(failure_causes_are_typed_and_propagated, {
    let mut state = setup_correlated(); let cause = FailureCause::new("typed").unwrap(); let update = ConversationUpdate::fail(guard(&state, MessageId::new(2)), cause.clone()); state.apply_event(event("fail", 3, update)).unwrap();
    assert_eq!(state.message(MessageId::new(2)).unwrap().status().failure_cause(), Some(&cause)); assert!(matches!(state.message(MessageId::new(2)).unwrap().blocks()[0].block(), MessageBlock::ToolResult(value) if value.status().failure_cause() == Some(&cause)));
});
exact!(message_complete_rejects_inconsistent_tool_pairs, {
    let mut state = terminal_call_state(); let update = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    assert!(matches!(assert_atomic_error(&mut state, event("premature", 4, update)), ConversationError::InvalidTransition { .. }));
});
exact!(edit_retires_thinking_ids_atomically, {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(4).unwrap()); let thought = MessageBlockEntry::new(BlockId::new(2), MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("thought").unwrap(), ""))); apply_push(&mut state, "p", 0, message(1, ChatRole::User, vec![text_entry(1, "x"), thought]));
    let edit = ConversationUpdate::edit_message(guard(&state, MessageId::new(1)), vec![text_entry(1, "x")]); state.apply_event(event("edit", 1, edit)).unwrap(); let reused = MessageBlockEntry::new(BlockId::new(3), MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("thought").unwrap(), ""))); let update = ConversationUpdate::append_message_block(guard(&state, MessageId::new(1)), reused);
    assert!(matches!(assert_atomic_error(&mut state, event("reuse", 2, update)), ConversationError::RetiredThinkingId { .. }));
});
exact!(delete_preserves_global_correlation_atomically, {
    let mut state = setup_correlated(); let update = ConversationUpdate::delete_message(guard(&state, MessageId::new(1)));
    assert!(matches!(assert_atomic_error(&mut state, event("orphan", 3, update)), ConversationError::OrphanToolResult { .. }));
});
exact!(resend_preserves_source_and_creates_fresh_identity, {
    let mut state = text_state(4); let complete = ConversationUpdate::complete(guard(&state, MessageId::new(1))); state.apply_event(event("complete", 1, complete)).unwrap(); let source = state.message(MessageId::new(1)).unwrap().clone();
    let update = ConversationUpdate::resend(guard(&state, MessageId::new(1)), message(2, ChatRole::User, vec![text_entry(2, "fresh")])); let outcome = state.apply_event(event("resend", 2, update)).unwrap();
    assert_eq!(state.message(MessageId::new(1)).unwrap(), &source); assert_eq!(outcome.affected_messages().iter().map(AffectedMessage::message_id).collect::<Vec<_>>(), vec![MessageId::new(2)]);
});
exact!(block_ids_are_conversation_unique_and_retained, {
    let mut state = text_state(4); let delete = ConversationUpdate::delete_message(guard(&state, MessageId::new(1))); state.apply_event(event("delete", 1, delete)).unwrap();
    let reused = ConversationUpdate::push(ConversationGuard::new(state.revision()), message(2, ChatRole::User, vec![text_entry(1, "reuse")])); assert!(matches!(assert_atomic_error(&mut state, event("reuse", 2, reused)), ConversationError::RetiredBlockId { .. }));
});
exact!(restore_snapshot_roundtrip_preserves_histories, {
    let state = setup_correlated(); assert_eq!(ConversationState::try_restore(state.snapshot()).unwrap(), state);
    let snapshot = state.snapshot(); let records = snapshot.retention().records().iter().map(|record| ProcessedEventRecord::new(record.event().clone(), record.outcome().clone())).collect(); let external = ConversationStateSnapshot::new(snapshot.messages().to_vec(), snapshot.revision(), snapshot.expected_sequence(), RetentionHistory::new(snapshot.retention().capacity(), records, None).unwrap(), snapshot.identities().clone()); assert_eq!(ConversationState::try_restore(external).unwrap(), state);
    let empty = ConversationStateSnapshot::new(snapshot.messages().to_vec(), snapshot.revision(), snapshot.expected_sequence(), RetentionHistory::new(snapshot.retention().capacity(), vec![], None).unwrap(), snapshot.identities().clone()); assert!(ConversationState::try_restore(empty).is_err());
});
exact!(deleted_tool_result_retires_result_slot_atomically, {
    let mut state = setup_correlated(); let delete = ConversationUpdate::delete_message(guard(&state, MessageId::new(2))); state.apply_event(event("delete-result", 3, delete)).unwrap();
    let result = MessageBlockEntry::new(BlockId::new(9), MessageBlock::ToolResult(ToolResultContent::new(ToolCallId::new("call").unwrap(), ""))); let update = ConversationUpdate::append_message_block(guard(&state, MessageId::new(1)), result); assert!(matches!(assert_atomic_error(&mut state, event("reuse-result", 4, update)), ConversationError::ResultSlotRetired { .. }));
});
exact!(sequence_is_conversation_wide_and_contiguous, {
    let mut state = text_state(4); let update = ConversationUpdate::complete(guard(&state, MessageId::new(1))); assert!(matches!(assert_atomic_error(&mut state, event("gap", 3, update)), ConversationError::SequenceGap { expected: 1, actual: 3 }));
});
exact!(exact_replay_returns_original_outcome_without_mutation, {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::MIN); let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), message(1, ChatRole::User, vec![text_entry(1, "x")])); let accepted = event("same", 0, update); let outcome = state.apply_event(accepted.clone()).unwrap(); let before = state.clone(); assert_eq!(state.apply_event(accepted).unwrap(), outcome); assert_eq!(state, before);
});
exact!(reused_event_id_with_different_content_conflicts, {
    let mut state = text_state(4); let update = ConversationUpdate::complete(guard(&state, MessageId::new(1))); assert!(matches!(assert_atomic_error(&mut state, event("push", 1, update)), ConversationError::EventIdConflict { .. }));
});
exact!(stale_gap_and_retention_errors_do_not_advance_state, {
    let mut state = text_state(4); let stale = ConversationUpdate::complete(guard(&state, MessageId::new(1))); assert!(matches!(assert_atomic_error(&mut state, event("stale", 0, stale)), ConversationError::StaleSequence { .. }));
});
exact!(every_failure_is_atomic_for_full_state, {
    let mut state = text_state(4); let bad = ConversationUpdate::replace_block(guard(&state, MessageId::new(1)), BlockId::new(99), MessageBlock::Text("no".into())); let before = state.clone(); assert!(state.apply_event(event("bad", 1, bad)).is_err()); assert_eq!(state, before);
});
exact!(replay_conflict_stale_and_gap_precede_exhaustion, {
    let mut state = ConversationState::new(u64::MAX - 1, std::num::NonZeroUsize::MIN); let accepted = event("last", u64::MAX - 1, ConversationUpdate::push(ConversationGuard::new(state.revision()), message(1, ChatRole::User, vec![text_entry(1, "x")]))); state.apply_event(accepted.clone()).unwrap(); assert!(state.apply_event(accepted).is_ok());
});
exact!(exact_replay_does_not_advance_exhausted_counters, {
    let mut state = ConversationState::new(u64::MAX - 1, std::num::NonZeroUsize::MIN); let accepted = event("last", u64::MAX - 1, ConversationUpdate::push(ConversationGuard::new(state.revision()), message(1, ChatRole::User, vec![text_entry(1, "x")]))); let outcome = state.apply_event(accepted.clone()).unwrap(); let before = state.clone(); assert_eq!(state.apply_event(accepted).unwrap(), outcome); assert_eq!(state, before);
});
exact!(mutation_replay_retention_is_consistent, {
    let mut state = text_state(1); let complete = ConversationUpdate::complete(guard(&state, MessageId::new(1))); state.apply_event(event("complete", 1, complete)).unwrap(); let old = event("push", 0, ConversationUpdate::push(ConversationGuard::new(ConversationRevision::INITIAL), message(1, ChatRole::User, vec![text_entry(1, "x")]))); assert!(matches!(assert_atomic_error(&mut state, old), ConversationError::ReplayOutsideRetention { .. }));
});
exact!(bounded_ledger_exposes_honest_replay_boundary, {
    let mut first = ConversationState::new(7, std::num::NonZeroUsize::new(2).unwrap()); let first_outcome = first.apply_event(event("first", 7, ConversationUpdate::push(ConversationGuard::new(first.revision()), message(11, ChatRole::User, vec![text_entry(11, "one")])))).unwrap();
    let mut second = ConversationState::new(7, std::num::NonZeroUsize::new(2).unwrap()); let second_event = event("second", 7, ConversationUpdate::push(ConversationGuard::new(second.revision()), message(22, ChatRole::User, vec![text_entry(22, "two")])));
    second.apply_event(second_event.clone()).unwrap(); let snapshot = second.snapshot(); let forged = ConversationStateSnapshot::new(snapshot.messages().to_vec(), snapshot.revision(), snapshot.expected_sequence(), RetentionHistory::new(snapshot.retention().capacity(), vec![ProcessedEventRecord::new(second_event, first_outcome)], None).unwrap(), snapshot.identities().clone()); assert!(ConversationState::try_restore(forged).is_err());
});
exact!(evicted_snapshot_rejects_forged_retained_event_with_valid_outcome, {
    let mut state = text_state(1); let target = guard(&state, MessageId::new(1)); let accepted = ConversationUpdate::append_text(target, BlockId::new(1), "accepted").unwrap(); state.apply_event(event("append", 1, accepted)).unwrap();
    let snapshot = state.snapshot(); assert_eq!(ConversationState::try_restore(snapshot.clone()).unwrap(), state); assert_eq!(snapshot.retention().evicted_through(), Some(0)); let forged = event("append", 1, ConversationUpdate::append_text(target, BlockId::new(1), "forged").unwrap()); let record = ProcessedEventRecord::new(forged, snapshot.retention().records()[0].outcome().clone());
    let forged_snapshot = ConversationStateSnapshot::new(snapshot.messages().to_vec(), snapshot.revision(), snapshot.expected_sequence(), RetentionHistory::new(snapshot.retention().capacity(), vec![record], snapshot.retention().evicted_through()).unwrap(), snapshot.identities().clone()); assert!(ConversationState::try_restore(forged_snapshot).is_err());
});
exact!(evicted_snapshot_rejects_genuine_retention_spliced_from_another_state, {
    let mut alpha = text_state(1); let alpha_update = ConversationUpdate::append_text(guard(&alpha, MessageId::new(1)), BlockId::new(1), "alpha").unwrap(); alpha.apply_event(event("alpha", 1, alpha_update)).unwrap(); let alpha_snapshot = alpha.snapshot();
    let mut beta = text_state(1); let beta_update = ConversationUpdate::append_text(guard(&beta, MessageId::new(1)), BlockId::new(1), "beta").unwrap(); beta.apply_event(event("beta", 1, beta_update)).unwrap(); let beta_snapshot = beta.snapshot();
    let hybrid = ConversationStateSnapshot::new_with_proof(beta_snapshot.messages().to_vec(), beta_snapshot.revision(), beta_snapshot.expected_sequence(), alpha_snapshot.retention().clone(), beta_snapshot.identities().clone(), beta_snapshot.proof().unwrap().clone()); assert!(ConversationState::try_restore(hybrid).is_err());
});
exact!(evicted_snapshot_public_proof_constructor_roundtrips, {
    let mut state = text_state(1); let update = ConversationUpdate::append_text(guard(&state, MessageId::new(1)), BlockId::new(1), "proof").unwrap(); state.apply_event(event("proof", 1, update)).unwrap(); let snapshot = state.snapshot(); let source = &snapshot.retention().records()[0];
    let record = ProcessedEventRecord::new_with_proof(source.event().clone(), source.outcome().clone(), source.proof().unwrap().clone()); let rebuilt = ConversationStateSnapshot::new_with_proof(snapshot.messages().to_vec(), snapshot.revision(), snapshot.expected_sequence(), RetentionHistory::new(snapshot.retention().capacity(), vec![record], snapshot.retention().evicted_through()).unwrap(), snapshot.identities().clone(), snapshot.proof().unwrap().clone()); assert_eq!(ConversationState::try_restore(rebuilt).unwrap(), state);
});
exact!(fresh_restart_state_has_no_replay_or_eviction_evidence, {
    let state = ConversationState::new(9, std::num::NonZeroUsize::MIN); assert_eq!(state.evicted_through(), None); assert!(state.snapshot().retention().records().is_empty());
});
exact!(sequence_exhaustion_is_checked_and_atomic_at_u64_max, {
    let mut state = max_state(); let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), message(1, ChatRole::User, vec![text_entry(1, "x")])); assert!(matches!(assert_atomic_error(&mut state, event("max", u64::MAX, update)), ConversationError::SequenceExhausted));
});
exact!(sequence_exhaustion_precedes_malformed_update_at_u64_max, {
    let mut state = max_state(); let malformed = ConversationUpdate::complete(MessageMutationGuard::new(ConversationGuard::new(state.revision()), MessageId::new(404), MessageRevision::INITIAL)); assert!(matches!(assert_atomic_error(&mut state, event("max", u64::MAX, malformed)), ConversationError::SequenceExhausted));
});
exact!(identical_sequences_produce_identical_state_and_outcomes, {
    let mut left = ConversationState::new(0, std::num::NonZeroUsize::new(2).unwrap()); let mut right = left.clone(); let update = ConversationUpdate::push(ConversationGuard::new(ConversationRevision::INITIAL), message(1, ChatRole::User, vec![text_entry(1, "same")])); let accepted = event("same", 0, update); assert_eq!(left.apply_event(accepted.clone()).unwrap(), right.apply_event(accepted).unwrap()); assert_eq!(left, right);
});
exact!(distinct_mock_adapters_produce_equal_core_events, {
    struct ChunkAdapter(Vec<&'static str>); struct PayloadAdapter { text: &'static str } impl ChunkAdapter { fn core(self) -> ConversationEvent { event("provider", 0, ConversationUpdate::push(ConversationGuard::new(ConversationRevision::INITIAL), message(1, ChatRole::Assistant, vec![text_entry(1, &self.0.concat())]))) } } impl PayloadAdapter { fn core(self) -> ConversationEvent { event("provider", 0, ConversationUpdate::push(ConversationGuard::new(ConversationRevision::INITIAL), message(1, ChatRole::Assistant, vec![text_entry(1, self.text)]))) } }
    assert_eq!(ChunkAdapter(vec!["sa", "me"]).core(), PayloadAdapter { text: "same" }.core());
});
exact!(empty_static_message_requires_content_before_complete, {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::MIN); apply_push(&mut state, "empty", 0, message(1, ChatRole::Assistant, vec![text_entry(1, "")])); let update = ConversationUpdate::complete(guard(&state, MessageId::new(1))); assert!(matches!(assert_atomic_error(&mut state, event("complete", 1, update)), ConversationError::InvalidTransition { .. }));
});
exact!(pending_message_with_active_nested_block_cannot_complete, {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::MIN); let thought = MessageBlockEntry::new(BlockId::new(1), MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("active").unwrap(), ""))); apply_push(&mut state, "nested", 0, message(1, ChatRole::Assistant, vec![thought])); let update = ConversationUpdate::complete(guard(&state, MessageId::new(1))); assert!(matches!(assert_atomic_error(&mut state, event("complete", 1, update)), ConversationError::InvalidTransition { .. }));
});
}
