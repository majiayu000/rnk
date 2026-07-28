use rnk::components::chat::{
    BlockId, ChatMessage, ChatRole, CodeContent, ConversationError, ConversationEvent,
    ConversationGuard, ConversationState, ConversationUpdate, MessageBlock, MessageBlockEntry,
    MessageId, MessageMutationGuard, MessageRevision, MessageStatus, ThinkingContent, ThinkingId,
    ThinkingStatus, UpdateId,
};
use std::num::NonZeroUsize;

fn fixture_message(id: u64, role: ChatRole, block_id: u64, block: MessageBlock) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(id),
        role,
        vec![MessageBlockEntry::new(BlockId::new(block_id), block)],
    )
    .unwrap()
}

fn fixture_guard(state: &ConversationState, id: MessageId) -> MessageMutationGuard {
    let message = state.message(id).unwrap();
    MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        id,
        message.revision(),
    )
}

fn apply_fixture_event(
    state: &mut ConversationState,
    event_id: &str,
    update: ConversationUpdate,
) -> rnk::components::chat::ApplyOutcome {
    state
        .apply_event(ConversationEvent::new(
            UpdateId::new(event_id).unwrap(),
            state.expected_sequence(),
            update,
        ))
        .unwrap()
}

fn push_fixture(state: &mut ConversationState, event_id: &str, message: ChatMessage) {
    let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), message);
    apply_fixture_event(state, event_id, update);
}

fn append_fixture(
    state: &mut ConversationState,
    event_id: &str,
    message_id: MessageId,
    block_id: BlockId,
    delta: &str,
) -> rnk::components::chat::ApplyOutcome {
    let update =
        ConversationUpdate::append_text(fixture_guard(state, message_id), block_id, delta).unwrap();
    apply_fixture_event(state, event_id, update)
}

#[test]
fn append_supported_blocks_preserves_typed_payloads() {
    let cases = [
        MessageBlock::Text("a".into()),
        MessageBlock::Markdown("**a".into()),
        MessageBlock::Code(
            CodeContent::new("fn")
                .unwrap()
                .with_language("rust")
                .unwrap(),
        ),
        MessageBlock::Thinking(ThinkingContent::new(
            ThinkingId::new("thought").unwrap(),
            "step",
        )),
    ];

    for (case, block) in cases.into_iter().enumerate() {
        let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
        push_fixture(
            &mut state,
            "push",
            fixture_message(1, ChatRole::Assistant, 1, block),
        );
        append_fixture(
            &mut state,
            "append",
            MessageId::new(1),
            BlockId::new(1),
            "+",
        );
        match (
            case,
            state.message(MessageId::new(1)).unwrap().blocks()[0].block(),
        ) {
            (0, MessageBlock::Text(value)) => assert_eq!(value, "a+"),
            (1, MessageBlock::Markdown(value)) => assert_eq!(value, "**a+"),
            (2, MessageBlock::Code(value)) => {
                assert_eq!(value.content(), "fn+");
                assert_eq!(value.language(), Some("rust"));
            }
            (3, MessageBlock::Thinking(value)) => {
                assert_eq!(value.content(), "step+");
                assert_eq!(value.status(), &ThinkingStatus::Streaming);
                assert_eq!(value.id().as_str(), "thought");
            }
            _ => panic!("append changed the target block kind"),
        }
    }
}

#[test]
fn targeted_outcome_advances_only_target() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
    push_fixture(
        &mut state,
        "target",
        fixture_message(1, ChatRole::Assistant, 1, MessageBlock::Text(String::new())),
    );
    push_fixture(
        &mut state,
        "other",
        fixture_message(2, ChatRole::User, 2, MessageBlock::Text("unchanged".into())),
    );
    let target_before = state.message(MessageId::new(1)).unwrap().revision();
    let other_before = state.message(MessageId::new(2)).unwrap().clone();
    let outcome = append_fixture(
        &mut state,
        "append",
        MessageId::new(1),
        BlockId::new(1),
        "chunk",
    );

    assert_eq!(outcome.affected_messages().len(), 1);
    assert_eq!(
        outcome.affected_messages()[0].message_id(),
        MessageId::new(1)
    );
    assert_eq!(
        outcome.affected_messages()[0].previous_revision(),
        Some(target_before),
    );
    assert_eq!(
        outcome.affected_messages()[0].applied_revision(),
        state.message(MessageId::new(1)).unwrap().revision(),
    );
    assert_eq!(state.message(MessageId::new(2)), Some(&other_before));
}

#[test]
fn rejected_targeted_updates_are_fully_atomic() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
    push_fixture(
        &mut state,
        "push",
        fixture_message(1, ChatRole::Assistant, 1, MessageBlock::Text("done".into())),
    );
    let complete = ConversationUpdate::complete(fixture_guard(&state, MessageId::new(1)));
    apply_fixture_event(&mut state, "complete", complete);

    let before_terminal = state.clone();
    let late = ConversationUpdate::append_text(
        fixture_guard(&state, MessageId::new(1)),
        BlockId::new(1),
        "late",
    )
    .unwrap();
    assert!(
        state
            .apply_event(ConversationEvent::new(
                UpdateId::new("late").unwrap(),
                state.expected_sequence(),
                late,
            ))
            .is_err()
    );
    assert_eq!(state, before_terminal);

    let mut active = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
    push_fixture(
        &mut active,
        "push",
        fixture_message(1, ChatRole::Assistant, 1, MessageBlock::Text(String::new())),
    );
    let before_missing = active.clone();
    let missing = ConversationUpdate::append_text(
        fixture_guard(&active, MessageId::new(1)),
        BlockId::new(999),
        "x",
    )
    .unwrap();
    assert!(
        active
            .apply_event(ConversationEvent::new(
                UpdateId::new("missing").unwrap(),
                active.expected_sequence(),
                missing,
            ))
            .is_err()
    );
    assert_eq!(active, before_missing);
}

#[test]
fn push_delete_resend_keep_lookup_and_order_consistent() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    for id in 1..=3 {
        push_fixture(
            &mut state,
            &format!("push-{id}"),
            fixture_message(
                id,
                ChatRole::User,
                id,
                MessageBlock::Text(format!("message-{id}")),
            ),
        );
    }
    let complete = ConversationUpdate::complete(fixture_guard(&state, MessageId::new(1)));
    apply_fixture_event(&mut state, "complete", complete);
    let resend = ConversationUpdate::resend(
        fixture_guard(&state, MessageId::new(1)),
        fixture_message(4, ChatRole::User, 4, MessageBlock::Text("resend".into())),
    );
    apply_fixture_event(&mut state, "resend", resend);
    let delete = ConversationUpdate::delete_message(fixture_guard(&state, MessageId::new(2)));
    apply_fixture_event(&mut state, "delete", delete);

    assert!(state.message(MessageId::new(2)).is_none());
    assert_eq!(
        state
            .messages()
            .iter()
            .map(ChatMessage::id)
            .collect::<Vec<_>>(),
        vec![MessageId::new(1), MessageId::new(3), MessageId::new(4)],
    );
    for (id, expected) in [(1, "message-1"), (3, "message-3"), (4, "resend")] {
        assert!(matches!(
            state.message(MessageId::new(id)).unwrap().blocks()[0].block(),
            MessageBlock::Text(value) if value == expected
        ));
    }
}

#[test]
fn snapshot_restore_rebuilds_target_lookup() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    for id in 1..=4 {
        push_fixture(
            &mut state,
            &format!("push-{id}"),
            fixture_message(
                id,
                ChatRole::Assistant,
                id,
                MessageBlock::Text(format!("message-{id}")),
            ),
        );
    }
    let restored = ConversationState::try_restore(state.snapshot()).unwrap();
    assert_eq!(restored, state);
    for id in 1..=4 {
        assert_eq!(
            restored.message(MessageId::new(id)).unwrap().id(),
            MessageId::new(id),
        );
    }
    assert!(restored.message(MessageId::new(99)).is_none());
    assert_eq!(restored.messages().len(), 4);
    assert_ne!(restored.messages()[0].status(), &MessageStatus::Complete);
}

#[test]
fn append_rejection_matrix_is_fully_atomic() {
    let blocks = [
        MessageBlock::Text("text".into()),
        MessageBlock::Markdown("markdown".into()),
        MessageBlock::Code(CodeContent::new("code").unwrap()),
        MessageBlock::Thinking(ThinkingContent::new(
            ThinkingId::new("thought").unwrap(),
            "thinking",
        )),
    ];
    for block in blocks {
        let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
        push_fixture(
            &mut state,
            "push",
            fixture_message(1, ChatRole::Assistant, 1, block),
        );
        let cancel = ConversationUpdate::cancel(fixture_guard(&state, MessageId::new(1)));
        apply_fixture_event(&mut state, "cancel", cancel);
        let before = state.clone();
        let append = ConversationUpdate::append_text(
            fixture_guard(&state, MessageId::new(1)),
            BlockId::new(1),
            "late",
        )
        .unwrap();
        let event = ConversationEvent::new(
            UpdateId::new("late").unwrap(),
            state.expected_sequence(),
            append,
        );
        assert!(matches!(
            state.apply_event(event),
            Err(ConversationError::InvalidTransition {
                kind: "message",
                ..
            })
        ));
        assert_eq!(state, before);
    }
}

#[test]
fn targeted_preflight_errors_preserve_the_complete_state() {
    let mut unknown = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    let unknown_before = unknown.clone();
    let unknown_update = ConversationUpdate::append_text(
        MessageMutationGuard::new(
            ConversationGuard::new(unknown.revision()),
            MessageId::new(99),
            MessageRevision::INITIAL,
        ),
        BlockId::new(99),
        "x",
    )
    .unwrap();
    let unknown_event =
        ConversationEvent::new(UpdateId::new("unknown").unwrap(), 0, unknown_update);
    assert_eq!(
        unknown.apply_event(unknown_event),
        Err(ConversationError::UnknownMessage {
            message_id: MessageId::new(99),
        }),
    );
    assert_eq!(unknown, unknown_before);

    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    push_fixture(
        &mut state,
        "push",
        fixture_message(1, ChatRole::Assistant, 1, MessageBlock::Text("text".into())),
    );
    let before_missing = state.clone();
    let missing = ConversationUpdate::append_text(
        fixture_guard(&state, MessageId::new(1)),
        BlockId::new(999),
        "x",
    )
    .unwrap();
    let missing_event = ConversationEvent::new(
        UpdateId::new("missing").unwrap(),
        state.expected_sequence(),
        missing,
    );
    assert_eq!(
        state.apply_event(missing_event),
        Err(ConversationError::UnknownBlock {
            message_id: MessageId::new(1),
            block_id: BlockId::new(999),
        }),
    );
    assert_eq!(state, before_missing);

    let stale_revision = state.message(MessageId::new(1)).unwrap().revision();
    append_fixture(
        &mut state,
        "first",
        MessageId::new(1),
        BlockId::new(1),
        " accepted",
    );
    let before_stale = state.clone();
    let stale = ConversationUpdate::append_text(
        MessageMutationGuard::new(
            ConversationGuard::new(state.revision()),
            MessageId::new(1),
            stale_revision,
        ),
        BlockId::new(1),
        " stale",
    )
    .unwrap();
    let stale_event = ConversationEvent::new(
        UpdateId::new("stale").unwrap(),
        state.expected_sequence(),
        stale,
    );
    assert!(matches!(
        state.apply_event(stale_event),
        Err(ConversationError::MessageRevisionMismatch {
            message_id,
            actual,
            ..
        }) if message_id == MessageId::new(1) && actual == stale_revision
    ));
    assert_eq!(state, before_stale);

    let before_empty = state.clone();
    assert!(
        ConversationUpdate::append_text(
            fixture_guard(&state, MessageId::new(1)),
            BlockId::new(1),
            "",
        )
        .is_err()
    );
    assert_eq!(state, before_empty);
}
