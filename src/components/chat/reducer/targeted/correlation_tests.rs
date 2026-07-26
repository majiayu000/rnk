use super::*;
use std::num::NonZeroUsize;

fn event(state: &ConversationState, id: &str, update: ConversationUpdate) -> ConversationEvent {
    ConversationEvent::new(UpdateId::new(id).unwrap(), state.expected_sequence, update)
}

fn guard(state: &ConversationState, id: MessageId) -> MessageMutationGuard {
    let position = state.message_position(id).unwrap();
    MessageMutationGuard::new(
        ConversationGuard::new(state.revision),
        id,
        state.messages[position].revision,
    )
}

fn message(id: u64, role: ChatRole, block: MessageBlock) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(id),
        role,
        vec![MessageBlockEntry::new(BlockId::new(id), block)],
    )
    .unwrap()
}

fn apply(state: &mut ConversationState, id: &str, update: ConversationUpdate) -> ApplyOutcome {
    state.apply_event(event(state, id, update)).unwrap()
}

fn push(state: &mut ConversationState, id: &str, message: ChatMessage) {
    let update = ConversationUpdate::push(ConversationGuard::new(state.revision), message);
    apply(state, id, update);
}

fn correlated_state() -> ConversationState {
    let mut state = ConversationState::new(0, NonZeroUsize::new(16).unwrap());
    let call_id = ToolCallId::new("call").unwrap();
    push(
        &mut state,
        "call",
        message(
            1,
            ChatRole::Assistant,
            MessageBlock::ToolCall(ToolCallContent::new(call_id.clone(), "tool", vec![]).unwrap()),
        ),
    );
    let running = ToolCallContent::new(call_id.clone(), "tool", vec![])
        .unwrap()
        .with_status(ToolCallStatus::Running);
    let update = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(1)),
        BlockId::new(1),
        MessageBlock::ToolCall(running),
    );
    apply(&mut state, "running", update);
    push(
        &mut state,
        "result",
        message(
            2,
            ChatRole::Tool,
            MessageBlock::ToolResult(ToolResultContent::new(call_id, "")),
        ),
    );
    push(
        &mut state,
        "unrelated",
        message(3, ChatRole::User, MessageBlock::Text("unrelated".into())),
    );
    state
}

fn terminate_cost(fail: bool) -> (ReducerCost, Vec<MessageId>) {
    let mut state = correlated_state();
    let guard = guard(&state, MessageId::new(1));
    let update = if fail {
        ConversationUpdate::fail(guard, FailureCause::new("failed").unwrap())
    } else {
        ConversationUpdate::cancel(guard)
    };
    reset_cost();
    let outcome = apply(&mut state, if fail { "fail" } else { "cancel" }, update);
    (
        cost(),
        outcome
            .affected_messages
            .iter()
            .map(|affected| affected.message_id)
            .collect(),
    )
}

fn correlation_ready_to_complete() -> ConversationState {
    let mut state = correlated_state();
    let call_id = ToolCallId::new("call").unwrap();
    let succeeded = ToolCallContent::new(call_id.clone(), "tool", vec![])
        .unwrap()
        .with_status(ToolCallStatus::Succeeded);
    let update = ConversationUpdate::replace_block(
        guard(&state, MessageId::new(1)),
        BlockId::new(1),
        MessageBlock::ToolCall(succeeded),
    );
    apply(&mut state, "succeeded", update);
    for (id, status) in [
        ("result-streaming", ToolResultStatus::Streaming),
        ("result-complete", ToolResultStatus::Complete),
    ] {
        let result = ToolResultContent::new(call_id.clone(), "done").with_status(status);
        let update = ConversationUpdate::replace_block(
            guard(&state, MessageId::new(2)),
            BlockId::new(2),
            MessageBlock::ToolResult(result),
        );
        apply(&mut state, id, update);
    }
    state
}

fn pending_correlation_state() -> ConversationState {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    push(
        &mut state,
        "call",
        message(
            1,
            ChatRole::Assistant,
            MessageBlock::ToolCall(
                ToolCallContent::new(ToolCallId::new("pending").unwrap(), "tool", vec![]).unwrap(),
            ),
        ),
    );
    push(
        &mut state,
        "unrelated",
        message(2, ChatRole::User, MessageBlock::Text("unrelated".into())),
    );
    state
}

#[test]
fn cancel_and_fail_count_each_correlation_visit() {
    let cancel = terminate_cost(false);
    let fail = terminate_cost(true);
    assert_eq!(cancel, fail);
    assert_eq!(cancel.1, vec![MessageId::new(1), MessageId::new(2)]);
    assert_eq!(
        cancel.0,
        ReducerCost {
            message_visits: 21,
            target_lookups: 3,
            block_visits: 12,
            global_validations: 1,
            backup_captures: 1,
        },
    );
}

#[test]
fn correlated_complete_counts_each_fallback_visit() {
    let mut state = correlation_ready_to_complete();
    let update = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    reset_cost();
    let outcome = apply(&mut state, "complete", update);
    assert_eq!(
        outcome
            .affected_messages
            .iter()
            .map(|affected| affected.message_id)
            .collect::<Vec<_>>(),
        vec![MessageId::new(1)],
    );
    assert_eq!(
        cost(),
        ReducerCost {
            message_visits: 12,
            target_lookups: 3,
            block_visits: 6,
            global_validations: 0,
            backup_captures: 0,
        },
    );
}

#[test]
fn pending_correlated_complete_counts_rejected_readiness_scan() {
    let mut state = pending_correlation_state();
    let before = state.clone();
    let update = ConversationUpdate::complete(guard(&state, MessageId::new(1)));
    reset_cost();
    let rejected = event(&state, "complete", update);
    assert!(matches!(
        state.apply_event(rejected),
        Err(ConversationError::InvalidTransition {
            kind: "message",
            ..
        })
    ));
    assert_eq!(state, before);
    assert_eq!(
        cost(),
        ReducerCost {
            message_visits: 4,
            target_lookups: 2,
            block_visits: 2,
            global_validations: 0,
            backup_captures: 0,
        },
    );
}

#[test]
fn target_revision_exhaustion_is_atomic_and_locally_counted() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
    state.messages = vec![message(
        1,
        ChatRole::Assistant,
        MessageBlock::Text("ready".into()),
    )];
    state.messages[0].revision = MessageRevision::new(u64::MAX).unwrap();
    state.seen_messages.insert(MessageId::new(1));
    state.seen_blocks.insert(BlockId::new(1));
    state.rebuild_message_index();
    let before = state.clone();
    let update =
        ConversationUpdate::append_text(guard(&state, MessageId::new(1)), BlockId::new(1), "x")
            .unwrap();
    reset_cost();
    let overflow = event(&state, "overflow", update);
    assert_eq!(
        state.apply_event(overflow),
        Err(ConversationError::MessageRevisionExhausted {
            message_id: MessageId::new(1),
        }),
    );
    assert_eq!(state, before);
    assert_eq!(
        cost(),
        ReducerCost {
            target_lookups: 1,
            ..ReducerCost::default()
        }
    );
}
