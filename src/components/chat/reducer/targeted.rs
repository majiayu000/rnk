//! Single-target reducer path and deterministic cost evidence.

use super::super::state::{message_active, nested_active, nested_terminal, static_complete_ready};
use super::super::*;
use std::collections::BTreeSet;

#[path = "targeted/replay.rs"]
mod replay;
pub(super) fn replay_matches(left: &ConversationEvent, right: &ConversationEvent) -> bool {
    replay::replay_matches(left, right)
}

#[derive(Clone, Copy)]
pub(super) enum TargetedUpdate {
    AppendText,
    Complete,
}

pub(super) fn target_guard(update: &ConversationUpdate) -> Option<MessageMutationGuard> {
    match update {
        ConversationUpdate::Push(_) => None,
        ConversationUpdate::AppendText(value) => Some(value.guard),
        ConversationUpdate::AppendMessageBlock(value) => Some(value.guard),
        ConversationUpdate::InsertMessageBlock(value) => Some(value.guard),
        ConversationUpdate::ReplaceBlock(value) => Some(value.guard),
        ConversationUpdate::Complete(value)
        | ConversationUpdate::Cancel(value)
        | ConversationUpdate::DeleteMessage(value) => Some(value.guard),
        ConversationUpdate::Fail(value) => Some(value.guard),
        ConversationUpdate::EditMessage(value) => Some(value.guard),
        ConversationUpdate::Resend(value) => Some(value.source_guard),
    }
}

pub(super) fn affected_existing(
    state: &ConversationState,
    update: &ConversationUpdate,
) -> BTreeSet<MessageId> {
    let Some(guard) = target_guard(update) else {
        return BTreeSet::new();
    };
    if matches!(update, ConversationUpdate::Resend(_)) {
        return BTreeSet::new();
    }
    let mut ids = BTreeSet::from([guard.message_id]);
    if !matches!(
        update,
        ConversationUpdate::Cancel(_) | ConversationUpdate::Fail(_)
    ) {
        return ids;
    }
    record_target_lookup();
    let Some(target) = state.message(guard.message_id) else {
        return ids;
    };
    let correlations = target
        .blocks
        .iter()
        .filter_map(|entry| {
            record_block_visit();
            match &entry.block {
                MessageBlock::ToolCall(value) => Some(value.call_id.clone()),
                MessageBlock::ToolResult(value) => Some(value.call_id.clone()),
                _ => None,
            }
        })
        .collect::<BTreeSet<_>>();
    for message in &state.messages {
        record_message_visits(1);
        if message.blocks.iter().any(|entry| {
            record_block_visit();
            match &entry.block {
                MessageBlock::ToolCall(value) if correlations.contains(&value.call_id) => {
                    nested_active(&entry.block)
                }
                MessageBlock::ToolResult(value) if correlations.contains(&value.call_id) => {
                    nested_active(&entry.block)
                }
                _ => false,
            }
        }) {
            ids.insert(message.id);
        }
    }
    ids
}

pub(super) fn requires_global_validation(update: &ConversationUpdate) -> bool {
    match update {
        ConversationUpdate::AppendText(_) | ConversationUpdate::Complete(_) => false,
        ConversationUpdate::Push(_)
        | ConversationUpdate::AppendMessageBlock(_)
        | ConversationUpdate::InsertMessageBlock(_)
        | ConversationUpdate::ReplaceBlock(_)
        | ConversationUpdate::Cancel(_)
        | ConversationUpdate::Fail(_)
        | ConversationUpdate::EditMessage(_)
        | ConversationUpdate::DeleteMessage(_)
        | ConversationUpdate::Resend(_) => true,
    }
}

pub(super) fn new_affected(message_id: MessageId) -> AffectedMessage {
    AffectedMessage {
        message_id,
        previous: None,
        applied: MessageRevision::INITIAL,
        disposition: AffectedMessageDisposition::Present,
    }
}

pub(super) fn commit_event(
    state: &mut ConversationState,
    event: ConversationEvent,
    next_sequence: u64,
    next_revision: ConversationRevision,
    affected_messages: Vec<AffectedMessage>,
) -> ApplyOutcome {
    let outcome = ApplyOutcome {
        revision: next_revision,
        affected_messages,
    };
    state.revision = next_revision;
    state.expected_sequence = next_sequence;
    let previous = state
        .ledger
        .back()
        .and_then(|record| record.proof.as_ref())
        .map_or([0; 4], |proof| proof.record);
    while state.ledger.len() >= state.ledger_capacity.get() {
        if let Some(record) = state.ledger.pop_front() {
            state.evicted_through = Some(record.event.sequence);
        }
    }
    state.ledger.push_back(ProcessedEventRecord::proven(
        event,
        outcome.clone(),
        previous,
    ));
    outcome
}

pub(super) fn classify(
    update: &ConversationUpdate,
    target: &ChatMessage,
) -> Option<TargetedUpdate> {
    match update {
        ConversationUpdate::AppendText(_) => Some(TargetedUpdate::AppendText),
        ConversationUpdate::Complete(_) => {
            for entry in &target.blocks {
                record_block_visit();
                if matches!(
                    entry.block,
                    MessageBlock::ToolCall(_) | MessageBlock::ToolResult(_)
                ) {
                    return None;
                }
            }
            Some(TargetedUpdate::Complete)
        }
        _ => None,
    }
}

pub(super) fn apply_targeted(
    state: &mut ConversationState,
    position: usize,
    update: &ConversationUpdate,
    kind: TargetedUpdate,
) -> Result<AffectedMessage, ConversationError> {
    let message_id = state.messages[position].id;
    let previous = state.messages[position].revision;
    let applied = previous.checked_next(message_id)?;
    match (kind, update) {
        (TargetedUpdate::AppendText, ConversationUpdate::AppendText(value)) => {
            append_text_at(&mut state.messages[position], value.block_id, &value.delta)?;
        }
        (TargetedUpdate::Complete, ConversationUpdate::Complete(_)) => {
            complete_at(&mut state.messages[position])?;
        }
        _ => unreachable!("targeted update classification must match its payload"),
    }
    state.messages[position].revision = applied;
    Ok(AffectedMessage {
        message_id,
        previous: Some(previous),
        applied,
        disposition: AffectedMessageDisposition::Present,
    })
}

fn append_text_at(
    message: &mut ChatMessage,
    block_id: BlockId,
    delta: &str,
) -> Result<(), ConversationError> {
    if delta.is_empty() {
        return Err(ConversationError::InvalidValue {
            field: "delta",
            reason: "must be non-empty",
        });
    }
    require_active(message)?;
    let mut found = None;
    for (position, entry) in message.blocks.iter().enumerate() {
        record_block_visit();
        if entry.id == block_id {
            found = Some(position);
            break;
        }
    }
    let position = found.ok_or(ConversationError::UnknownBlock {
        message_id: message.id,
        block_id,
    })?;
    match &mut message.blocks[position].block {
        MessageBlock::Text(value) | MessageBlock::Markdown(value) => value.push_str(delta),
        MessageBlock::Code(value) => value.append_text(delta),
        MessageBlock::Thinking(value) => match value.status {
            ThinkingStatus::Pending => {
                value.content.push_str(delta);
                value.status = ThinkingStatus::Streaming;
            }
            ThinkingStatus::Streaming => value.content.push_str(delta),
            _ => {
                return targeted_transition_error("thinking", "cannot append to terminal thinking");
            }
        },
        _ => return targeted_transition_error("block", "block is not appendable"),
    }
    if message.status == MessageStatus::Pending {
        message.status = MessageStatus::Streaming;
    }
    Ok(())
}

fn complete_at(message: &mut ChatMessage) -> Result<(), ConversationError> {
    let ready = match message.status {
        MessageStatus::Pending => static_complete_ready(message, record_block_visit),
        MessageStatus::Streaming => {
            let mut ready = true;
            for entry in &message.blocks {
                record_block_visit();
                ready &= nested_terminal(&entry.block);
            }
            ready
        }
        MessageStatus::Complete | MessageStatus::Cancelled | MessageStatus::Failed(_) => {
            return targeted_transition_error("message", "message is terminal");
        }
    };
    if !ready {
        return targeted_transition_error("message", "message content is not ready to complete");
    }
    message.status = MessageStatus::Complete;
    Ok(())
}

fn require_active(message: &ChatMessage) -> Result<(), ConversationError> {
    if message_active(&message.status) {
        Ok(())
    } else {
        targeted_transition_error("message", "message is terminal")
    }
}

fn targeted_transition_error<T>(
    kind: &'static str,
    reason: &'static str,
) -> Result<T, ConversationError> {
    Err(ConversationError::InvalidTransition { kind, reason })
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ReducerCost {
    message_visits: usize,
    target_lookups: usize,
    block_visits: usize,
    global_validations: usize,
    backup_captures: usize,
}

#[cfg(test)]
thread_local! {
    static REDUCER_COST: std::cell::Cell<ReducerCost> =
        const { std::cell::Cell::new(ReducerCost {
            message_visits: 0,
            target_lookups: 0,
            block_visits: 0,
            global_validations: 0,
            backup_captures: 0,
        }) };
}

#[cfg(test)]
fn update_cost(update: impl FnOnce(&mut ReducerCost)) {
    REDUCER_COST.with(|cost| {
        let mut value = cost.get();
        update(&mut value);
        cost.set(value);
    });
}

pub(super) fn record_message_visits(count: usize) {
    #[cfg(test)]
    update_cost(|cost| cost.message_visits += count);
    #[cfg(not(test))]
    let _ = count;
}

pub(super) fn record_target_lookup() {
    #[cfg(test)]
    update_cost(|cost| cost.target_lookups += 1);
}

fn record_block_visit() {
    #[cfg(test)]
    update_cost(|cost| cost.block_visits += 1);
}

pub(super) fn record_block_visits(count: usize) {
    #[cfg(test)]
    update_cost(|cost| cost.block_visits += count);
    #[cfg(not(test))]
    let _ = count;
}

pub(super) fn record_global_validation() {
    #[cfg(test)]
    update_cost(|cost| cost.global_validations += 1);
}

pub(super) fn record_backup_capture() {
    #[cfg(test)]
    update_cost(|cost| cost.backup_captures += 1);
}

#[cfg(test)]
fn reset_cost() {
    REDUCER_COST.set(ReducerCost::default());
}

#[cfg(test)]
fn cost() -> ReducerCost {
    REDUCER_COST.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    fn text_message(id: u64, text: &str) -> ChatMessage {
        ChatMessage::new(
            MessageId::new(id),
            ChatRole::User,
            vec![MessageBlockEntry::new(
                BlockId::new(id),
                MessageBlock::Text(text.into()),
            )],
        )
        .unwrap()
    }

    fn direct_state(count: usize) -> ConversationState {
        let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
        state.messages = (1..=count)
            .map(|id| text_message(id as u64, "ready"))
            .collect();
        state
            .seen_messages
            .extend(state.messages.iter().map(|message| message.id));
        state.seen_blocks.extend(
            state
                .messages
                .iter()
                .flat_map(|message| message.blocks.iter().map(|entry| entry.id)),
        );
        state.rebuild_message_index(|| {});
        state
    }

    fn target_guard(state: &ConversationState, id: MessageId) -> MessageMutationGuard {
        let position = state.message_position(id).unwrap();
        MessageMutationGuard::new(
            ConversationGuard::new(state.revision),
            id,
            state.messages[position].revision,
        )
    }

    fn append_cost(count: usize, target: MessageId) -> ReducerCost {
        let mut state = direct_state(count);
        let update = ConversationUpdate::append_text(
            target_guard(&state, target),
            BlockId::new(target.get()),
            "x",
        )
        .unwrap();
        reset_cost();
        state
            .apply_event(ConversationEvent::new(
                UpdateId::new("append").unwrap(),
                0,
                update,
            ))
            .unwrap();
        cost()
    }

    fn complete_cost(count: usize, target: MessageId) -> ReducerCost {
        let mut state = direct_state(count);
        let update = ConversationUpdate::complete(target_guard(&state, target));
        reset_cost();
        state
            .apply_event(ConversationEvent::new(
                UpdateId::new("complete").unwrap(),
                0,
                update,
            ))
            .unwrap();
        cost()
    }

    #[test]
    fn append_cost_is_transcript_independent() {
        let small = append_cost(1, MessageId::new(1));
        let large = append_cost(10_001, MessageId::new(1));
        assert_eq!(small.message_visits, 0);
        assert_eq!(large.message_visits, 0);
        assert_eq!(small.target_lookups, 1);
        assert_eq!(large.target_lookups, 1);
    }

    #[test]
    fn complete_cost_is_transcript_independent() {
        let small = complete_cost(1, MessageId::new(1));
        let large = complete_cost(10_001, MessageId::new(1));
        assert_eq!(small.message_visits, 0);
        assert_eq!(large.message_visits, 0);
        assert_eq!(small.target_lookups, 1);
        assert_eq!(large.target_lookups, 1);
    }

    #[test]
    fn front_and_end_targets_have_equal_cost() {
        assert_eq!(
            append_cost(10_001, MessageId::new(1)),
            append_cost(10_001, MessageId::new(10_001)),
        );
        assert_eq!(
            complete_cost(10_001, MessageId::new(1)),
            complete_cost(10_001, MessageId::new(10_001)),
        );
    }

    #[test]
    fn correlated_complete_records_global_visits() {
        let call_id = ToolCallId::new("call").unwrap();
        let call = ToolCallContent::new(call_id.clone(), "tool", vec![])
            .unwrap()
            .with_status(ToolCallStatus::Succeeded);
        let result =
            ToolResultContent::new(call_id, "done").with_status(ToolResultStatus::Complete);
        let mut call_message = ChatMessage::new(
            MessageId::new(1),
            ChatRole::Assistant,
            vec![MessageBlockEntry::new(
                BlockId::new(1),
                MessageBlock::ToolCall(call),
            )],
        )
        .unwrap();
        call_message.status = MessageStatus::Streaming;
        let mut result_message = ChatMessage::new(
            MessageId::new(2),
            ChatRole::Tool,
            vec![MessageBlockEntry::new(
                BlockId::new(2),
                MessageBlock::ToolResult(result),
            )],
        )
        .unwrap();
        result_message.status = MessageStatus::Complete;
        let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
        state.messages = vec![call_message, result_message];
        state.rebuild_message_index(|| {});
        let update = ConversationUpdate::complete(target_guard(&state, MessageId::new(1)));
        reset_cost();
        state
            .apply_event(ConversationEvent::new(
                UpdateId::new("complete").unwrap(),
                0,
                update,
            ))
            .unwrap();
        assert!(cost().message_visits > 0);
    }

    #[test]
    fn cost_dimensions_are_independent() {
        let mut state = ConversationState::new(0, NonZeroUsize::new(4).unwrap());
        let update =
            ConversationUpdate::push(ConversationGuard::new(state.revision), text_message(1, "x"));
        reset_cost();
        state
            .apply_event(ConversationEvent::new(
                UpdateId::new("push").unwrap(),
                0,
                update,
            ))
            .unwrap();
        let cost = cost();
        assert_eq!(cost.target_lookups, 1);
        assert_eq!(cost.global_validations, 1);
        assert_eq!(cost.backup_captures, 1);
        assert!(cost.message_visits > 0);
    }

    #[test]
    fn local_paths_skip_global_work() {
        for cost in [
            append_cost(100, MessageId::new(50)),
            complete_cost(100, MessageId::new(50)),
        ] {
            assert_eq!(cost.global_validations, 0);
            assert_eq!(cost.backup_captures, 0);
        }
    }

    #[test]
    fn global_validation_scope_is_exhaustive() {
        let conversation = ConversationGuard::new(ConversationRevision::INITIAL);
        let guard =
            MessageMutationGuard::new(conversation, MessageId::new(1), MessageRevision::INITIAL);
        let entry = MessageBlockEntry::new(BlockId::new(1), MessageBlock::Text("x".into()));
        let message = text_message(2, "y");
        let updates = vec![
            ConversationUpdate::push(conversation, message.clone()),
            ConversationUpdate::append_text(guard, BlockId::new(1), "x").unwrap(),
            ConversationUpdate::append_message_block(guard, entry.clone()),
            ConversationUpdate::insert_message_block(guard, 0, entry.clone()),
            ConversationUpdate::replace_block(
                guard,
                BlockId::new(1),
                MessageBlock::Text("y".into()),
            ),
            ConversationUpdate::complete(guard),
            ConversationUpdate::cancel(guard),
            ConversationUpdate::fail(guard, FailureCause::new("x").unwrap()),
            ConversationUpdate::edit_message(guard, vec![entry]),
            ConversationUpdate::delete_message(guard),
            ConversationUpdate::resend(guard, message),
        ];
        assert_eq!(
            updates
                .iter()
                .map(requires_global_validation)
                .collect::<Vec<_>>(),
            vec![
                true, false, true, true, true, false, true, true, true, true, true
            ],
        );
    }

    #[test]
    fn rejected_local_updates_remain_atomic_without_backup() {
        let mut state = direct_state(1);
        let complete = ConversationUpdate::complete(target_guard(&state, MessageId::new(1)));
        state
            .apply_event(ConversationEvent::new(
                UpdateId::new("complete").unwrap(),
                0,
                complete,
            ))
            .unwrap();
        let before = state.clone();
        let append = ConversationUpdate::append_text(
            target_guard(&state, MessageId::new(1)),
            BlockId::new(1),
            "late",
        )
        .unwrap();
        reset_cost();
        assert!(
            state
                .apply_event(ConversationEvent::new(
                    UpdateId::new("late").unwrap(),
                    1,
                    append,
                ))
                .is_err()
        );
        assert_eq!(state, before);
        assert_eq!(cost().global_validations, 0);
        assert_eq!(cost().backup_captures, 0);
    }

    #[test]
    fn missing_affected_target_stays_single_target() {
        let state = direct_state(0);
        let guard = MessageMutationGuard::new(
            ConversationGuard::new(state.revision),
            MessageId::new(1),
            MessageRevision::INITIAL,
        );
        let update = ConversationUpdate::cancel(guard);
        assert_eq!(
            affected_existing(&state, &update),
            BTreeSet::from([MessageId::new(1)]),
        );
    }

    #[test]
    #[should_panic(expected = "targeted update classification must match its payload")]
    fn targeted_kind_payload_mismatch_fails_loudly() {
        let mut state = direct_state(1);
        let update = ConversationUpdate::append_text(
            target_guard(&state, MessageId::new(1)),
            BlockId::new(1),
            "x",
        )
        .unwrap();
        apply_targeted(&mut state, 0, &update, TargetedUpdate::Complete).unwrap();
    }

    #[test]
    fn append_rejects_empty_terminal_and_nonappendable_payloads() {
        let mut text = text_message(1, "ready");
        assert!(matches!(
            append_text_at(&mut text, BlockId::new(1), ""),
            Err(ConversationError::InvalidValue { field: "delta", .. })
        ));

        let thinking = ThinkingContent::new(ThinkingId::new("thought").unwrap(), "step")
            .with_status(ThinkingStatus::Complete);
        let mut terminal_thinking = ChatMessage::new(
            MessageId::new(2),
            ChatRole::Assistant,
            vec![MessageBlockEntry::new(
                BlockId::new(2),
                MessageBlock::Thinking(thinking),
            )],
        )
        .unwrap();
        assert!(matches!(
            append_text_at(&mut terminal_thinking, BlockId::new(2), "x"),
            Err(ConversationError::InvalidTransition {
                kind: "thinking",
                ..
            })
        ));

        let mut error = ChatMessage::new(
            MessageId::new(3),
            ChatRole::Assistant,
            vec![MessageBlockEntry::new(
                BlockId::new(3),
                MessageBlock::Error(ErrorContent::new("error").unwrap()),
            )],
        )
        .unwrap();
        assert!(matches!(
            append_text_at(&mut error, BlockId::new(3), "x"),
            Err(ConversationError::InvalidTransition { kind: "block", .. })
        ));
    }

    #[test]
    fn streaming_thinking_append_and_complete_paths_are_covered() {
        let thinking = ThinkingContent::new(ThinkingId::new("thought").unwrap(), "step")
            .with_status(ThinkingStatus::Streaming);
        let mut streaming_thinking = ChatMessage::new(
            MessageId::new(1),
            ChatRole::Assistant,
            vec![MessageBlockEntry::new(
                BlockId::new(1),
                MessageBlock::Thinking(thinking),
            )],
        )
        .unwrap();
        streaming_thinking.status = MessageStatus::Streaming;
        append_text_at(&mut streaming_thinking, BlockId::new(1), "+").unwrap();
        assert!(matches!(
            &streaming_thinking.blocks[0].block,
            MessageBlock::Thinking(value) if value.content() == "step+"
        ));

        let mut streaming = text_message(2, "ready");
        streaming.status = MessageStatus::Streaming;
        complete_at(&mut streaming).unwrap();
        assert_eq!(streaming.status, MessageStatus::Complete);
    }

    #[test]
    fn pending_static_complete_handles_code_and_inert_content() {
        for (id, block) in [
            (1, MessageBlock::Code(CodeContent::new("code").unwrap())),
            (2, MessageBlock::Error(ErrorContent::new("error").unwrap())),
        ] {
            let mut message = ChatMessage::new(
                MessageId::new(id),
                ChatRole::Assistant,
                vec![MessageBlockEntry::new(BlockId::new(id), block)],
            )
            .unwrap();
            complete_at(&mut message).unwrap();
            assert_eq!(message.status, MessageStatus::Complete);
        }
    }

    #[test]
    fn complete_rejects_terminal_message() {
        let mut message = text_message(1, "ready");
        message.status = MessageStatus::Complete;
        assert!(matches!(
            complete_at(&mut message),
            Err(ConversationError::InvalidTransition {
                kind: "message",
                ..
            })
        ));
    }
}

#[cfg(test)]
#[path = "targeted/correlation_tests.rs"]
mod correlation_tests;
