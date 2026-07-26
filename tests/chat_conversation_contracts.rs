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
