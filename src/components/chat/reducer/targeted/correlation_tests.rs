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

fn text_blocks_message(id: u64, blocks: &[(u64, &str)]) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(id),
        ChatRole::User,
        blocks
            .iter()
            .map(|(block_id, text)| {
                MessageBlockEntry::new(BlockId::new(*block_id), MessageBlock::Text((*text).into()))
            })
            .collect(),
    )
    .unwrap()
}

fn three_block_state() -> ConversationState {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    push(
        &mut state,
        "seed",
        text_blocks_message(1, &[(1, "one"), (2, "two"), (3, "three")]),
    );
    state
}

fn apply(state: &mut ConversationState, id: &str, update: ConversationUpdate) -> ApplyOutcome {
    state.apply_event(event(state, id, update)).unwrap()
}

fn assert_replay_match(
    left: ConversationUpdate,
    right: ConversationUpdate,
    expected: bool,
    block_visits: usize,
) {
    let event_id = UpdateId::new("compare").unwrap();
    let left = ConversationEvent::new(event_id.clone(), 0, left);
    let right = ConversationEvent::new(event_id, 0, right);
    reset_cost();
    assert_eq!(replay_matches(&left, &right), expected);
    assert_eq!(
        cost(),
        ReducerCost {
            block_visits,
            ..ReducerCost::default()
        }
    );
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
            message_visits: 27,
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
fn push_counts_index_rebuild_validation_and_identity_backup_visits() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    push(
        &mut state,
        "first",
        message(1, ChatRole::User, MessageBlock::Text("first".into())),
    );
    push(
        &mut state,
        "second",
        message(2, ChatRole::User, MessageBlock::Text("second".into())),
    );
    reset_cost();
    push(
        &mut state,
        "third",
        text_blocks_message(3, &[(3, "third"), (4, "fourth"), (5, "fifth")]),
    );
    assert_eq!(
        cost(),
        ReducerCost {
            message_visits: 25,
            target_lookups: 1,
            block_visits: 13,
            global_validations: 1,
            backup_captures: 1,
        },
    );
}

#[test]
fn generic_mutations_count_each_block_traversal() {
    let mut replace_state = three_block_state();
    let replace = ConversationUpdate::replace_block(
        guard(&replace_state, MessageId::new(1)),
        BlockId::new(3),
        MessageBlock::Text("changed".into()),
    );
    reset_cost();
    apply(&mut replace_state, "replace", replace);
    assert_eq!(cost().block_visits, 6);

    let mut edit_state = three_block_state();
    let edit = ConversationUpdate::edit_message(
        guard(&edit_state, MessageId::new(1)),
        vec![
            MessageBlockEntry::new(BlockId::new(1), MessageBlock::Text("changed".into())),
            MessageBlockEntry::new(BlockId::new(2), MessageBlock::Text("two".into())),
            MessageBlockEntry::new(BlockId::new(3), MessageBlock::Text("three".into())),
        ],
    );
    reset_cost();
    apply(&mut edit_state, "edit", edit);
    assert_eq!(cost().block_visits, 29);

    let mut delete_state = three_block_state();
    let delete = ConversationUpdate::delete_message(guard(&delete_state, MessageId::new(1)));
    reset_cost();
    apply(&mut delete_state, "delete", delete);
    assert_eq!(cost().block_visits, 6);
}

#[test]
fn no_op_edit_counts_each_compared_block() {
    let mut state = three_block_state();
    let entries = state.messages[0].blocks.clone();
    let edit = ConversationUpdate::edit_message(guard(&state, MessageId::new(1)), entries);
    reset_cost();
    assert!(matches!(
        state.apply_event(event(&state, "no-op-edit", edit)),
        Err(ConversationError::NoOpEdit { .. })
    ));
    assert_eq!(
        cost(),
        ReducerCost {
            target_lookups: 1,
            block_visits: 6,
            ..ReducerCost::default()
        }
    );
}

#[test]
fn replay_equality_counts_each_compared_block() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    let original = event(
        &state,
        "replay",
        ConversationUpdate::push(
            ConversationGuard::new(state.revision),
            text_blocks_message(1, &[(1, "one"), (2, "two"), (3, "three")]),
        ),
    );
    let exact = original.clone();
    let conflict = event(
        &state,
        "replay",
        ConversationUpdate::push(
            ConversationGuard::new(state.revision),
            text_blocks_message(1, &[(1, "one"), (2, "two"), (3, "changed")]),
        ),
    );
    let outcome = state.apply_event(original).unwrap();

    reset_cost();
    assert_eq!(state.apply_event(exact).unwrap(), outcome);
    assert_eq!(
        cost(),
        ReducerCost {
            block_visits: 6,
            ..ReducerCost::default()
        }
    );

    reset_cost();
    assert!(matches!(
        state.apply_event(conflict),
        Err(ConversationError::EventIdConflict { .. })
    ));
    assert_eq!(
        cost(),
        ReducerCost {
            block_visits: 6,
            ..ReducerCost::default()
        }
    );
}

#[test]
fn replay_equality_covers_every_update_payload_shape() {
    let conversation = ConversationGuard::new(ConversationRevision::INITIAL);
    let guard =
        MessageMutationGuard::new(conversation, MessageId::new(1), MessageRevision::INITIAL);
    let entry = MessageBlockEntry::new(BlockId::new(4), MessageBlock::Text("four".into()));
    let message = text_blocks_message(1, &[(1, "one"), (2, "two"), (3, "three")]);
    let updates = vec![
        (ConversationUpdate::push(conversation, message.clone()), 6),
        (
            ConversationUpdate::append_text(guard, BlockId::new(1), "x").unwrap(),
            0,
        ),
        (
            ConversationUpdate::append_message_block(guard, entry.clone()),
            2,
        ),
        (
            ConversationUpdate::insert_message_block(guard, 1, entry.clone()),
            2,
        ),
        (
            ConversationUpdate::replace_block(
                guard,
                BlockId::new(1),
                MessageBlock::Text("changed".into()),
            ),
            2,
        ),
        (ConversationUpdate::complete(guard), 0),
        (ConversationUpdate::cancel(guard), 0),
        (
            ConversationUpdate::fail(guard, FailureCause::new("failed").unwrap()),
            0,
        ),
        (
            ConversationUpdate::edit_message(guard, message.blocks.clone()),
            6,
        ),
        (ConversationUpdate::delete_message(guard), 0),
        (ConversationUpdate::resend(guard, message.clone()), 6),
    ];
    for (update, block_visits) in updates {
        assert_replay_match(update.clone(), update.clone(), true, block_visits);
        let mismatch = if matches!(&update, ConversationUpdate::Complete(_)) {
            ConversationUpdate::cancel(guard)
        } else {
            ConversationUpdate::complete(guard)
        };
        assert_replay_match(update, mismatch, false, 0);
    }
}

#[test]
fn append_block_counts_existing_and_candidate_identity_backup_visits() {
    let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
    push(
        &mut state,
        "first",
        message(1, ChatRole::User, MessageBlock::Text("first".into())),
    );
    push(
        &mut state,
        "second",
        message(2, ChatRole::User, MessageBlock::Text("second".into())),
    );
    let update = ConversationUpdate::append_message_block(
        guard(&state, MessageId::new(1)),
        MessageBlockEntry::new(BlockId::new(3), MessageBlock::Text("third".into())),
    );
    reset_cost();
    apply(&mut state, "append-block", update);
    assert_eq!(
        cost(),
        ReducerCost {
            message_visits: 18,
            target_lookups: 3,
            block_visits: 7,
            global_validations: 1,
            backup_captures: 1,
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
    state.rebuild_message_index(|| {});
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
