//! Atomic reducer implementation for ordered conversation events.

#[rustfmt::skip]
mod compact {
use super::super::*;
use super::super::state::{
    message_active, message_terminal, nested_active, nested_terminal, same_block_identity,
    static_complete_ready, valid_call_result, valid_call_transition, valid_message_transition,
    valid_result_transition, valid_thinking_transition,
};
use std::collections::{BTreeMap, BTreeSet};

impl ConversationState {
    /// Applies one ordered event atomically or returns a typed error without mutation.
    pub fn apply_event(&mut self, event: ConversationEvent)
        -> Result<ApplyOutcome, ConversationError> {
        if let Some(record) = self.ledger.iter().find(|record|
            record.event.event_id == event.event_id) {
            return if record.event == event { Ok(record.outcome.clone()) } else {
                Err(ConversationError::EventIdConflict { event_id: event.event_id })
            };
        }
        if event.sequence < self.expected_sequence {
            return if self.evicted_through.is_some_and(|boundary| event.sequence <= boundary) {
                Err(ConversationError::ReplayOutsideRetention {
                    sequence: event.sequence, evicted_through: self.evicted_through.unwrap(),
                })
            } else {
                Err(ConversationError::StaleSequence {
                    expected: self.expected_sequence, actual: event.sequence,
                })
            };
        }
        if event.sequence > self.expected_sequence {
            return Err(ConversationError::SequenceGap {
                expected: self.expected_sequence, actual: event.sequence,
            });
        }
        let next_sequence = self.expected_sequence.checked_add(1)
            .ok_or(ConversationError::SequenceExhausted)?;
        let next_revision = self.revision.checked_next()?;
        let supplied_revision = event.update.conversation_guard().expected();
        if supplied_revision != self.revision {
            return Err(ConversationError::ConversationRevisionMismatch {
                expected: self.revision, actual: supplied_revision,
            });
        }
        let target_guard = target_guard(&event.update);
        if let Some(guard) = target_guard {
            let message = self.message(guard.message_id)
                .ok_or(ConversationError::UnknownMessage { message_id: guard.message_id })?;
            if guard.message_revision != message.revision {
                return Err(ConversationError::MessageRevisionMismatch {
                    message_id: guard.message_id, expected: message.revision,
                    actual: guard.message_revision,
                });
            }
        }
        let affected_ids = affected_existing(self, &event.update);
        let mut revisions = BTreeMap::new();
        for message in &self.messages {
            if affected_ids.contains(&message.id) {
                revisions.insert(message.id, (message.revision,
                    message.revision.checked_next(message.id)?));
            }
        }
        let mut staged = self.clone();
        apply_update(&mut staged, &event.update)?;
        validate_conversation(&staged)?;
        for message in &mut staged.messages {
            if let Some((_, next)) = revisions.get(&message.id) { message.revision = *next; }
        }
        let mut affected_messages = self.messages.iter().filter_map(|message| {
            revisions.get(&message.id).map(|(previous, applied)| AffectedMessage {
                message_id: message.id, previous: Some(*previous), applied: *applied,
                disposition: if matches!(event.update, ConversationUpdate::DeleteMessage(_))
                    && target_guard.is_some_and(|guard| guard.message_id == message.id) {
                    AffectedMessageDisposition::Deleted
                } else { AffectedMessageDisposition::Present },
            })
        }).collect::<Vec<_>>();
        match &event.update {
            ConversationUpdate::Push(value) => affected_messages.push(new_affected(value.message.id)),
            ConversationUpdate::Resend(value) => affected_messages.push(new_affected(value.message.id)),
            _ => {}
        }
        let outcome = ApplyOutcome { revision: next_revision, affected_messages };
        staged.revision = next_revision;
        staged.expected_sequence = next_sequence;
        staged.ledger.push_back(ProcessedEventRecord::new(event, outcome.clone()));
        while staged.ledger.len() > staged.ledger_capacity.get() {
            if let Some(record) = staged.ledger.pop_front() {
                staged.evicted_through = Some(record.event.sequence);
            }
        }
        *self = staged;
        Ok(outcome)
    }
}

fn new_affected(message_id: MessageId) -> AffectedMessage {
    AffectedMessage { message_id, previous: None, applied: MessageRevision::INITIAL,
        disposition: AffectedMessageDisposition::Present }
}

fn target_guard(update: &ConversationUpdate) -> Option<MessageMutationGuard> {
    match update {
        ConversationUpdate::Push(_) => None,
        ConversationUpdate::AppendText(value) => Some(value.guard),
        ConversationUpdate::AppendMessageBlock(value) => Some(value.guard),
        ConversationUpdate::InsertMessageBlock(value) => Some(value.guard),
        ConversationUpdate::ReplaceBlock(value) => Some(value.guard),
        ConversationUpdate::Complete(value) | ConversationUpdate::Cancel(value)
        | ConversationUpdate::DeleteMessage(value) => Some(value.guard),
        ConversationUpdate::Fail(value) => Some(value.guard),
        ConversationUpdate::EditMessage(value) => Some(value.guard),
        ConversationUpdate::Resend(value) => Some(value.source_guard),
    }
}

fn affected_existing(state: &ConversationState, update: &ConversationUpdate) -> BTreeSet<MessageId> {
    let Some(guard) = target_guard(update) else { return BTreeSet::new(); };
    let mut ids = BTreeSet::from([guard.message_id]);
    if !matches!(update, ConversationUpdate::Cancel(_) | ConversationUpdate::Fail(_)) {
        return ids;
    }
    let Some(target) = state.message(guard.message_id) else { return ids; };
    let correlations = target.blocks.iter().filter_map(|entry| match &entry.block {
        MessageBlock::ToolCall(value) => Some(value.call_id.clone()),
        MessageBlock::ToolResult(value) => Some(value.call_id.clone()),
        _ => None,
    }).collect::<BTreeSet<_>>();
    for message in &state.messages {
        if message.blocks.iter().any(|entry| match &entry.block {
            MessageBlock::ToolCall(value) if correlations.contains(&value.call_id) => nested_active(&entry.block),
            MessageBlock::ToolResult(value) if correlations.contains(&value.call_id) => nested_active(&entry.block),
            _ => false,
        }) { ids.insert(message.id); }
    }
    ids
}

fn apply_update(state: &mut ConversationState, update: &ConversationUpdate)
    -> Result<(), ConversationError> {
    match update {
        ConversationUpdate::Push(value) => push(state, &value.message, None),
        ConversationUpdate::AppendText(value) =>
            append_text(state, value.guard.message_id, value.block_id, &value.delta),
        ConversationUpdate::AppendMessageBlock(value) =>
            insert_block(state, value.guard.message_id, None, &value.entry),
        ConversationUpdate::InsertMessageBlock(value) =>
            insert_block(state, value.guard.message_id, Some(value.position), &value.entry),
        ConversationUpdate::ReplaceBlock(value) =>
            replace_block(state, value.guard.message_id, value.block_id, &value.replacement),
        ConversationUpdate::Complete(value) => complete(state, value.guard.message_id),
        ConversationUpdate::Cancel(value) => terminate(state, value.guard.message_id, None),
        ConversationUpdate::Fail(value) =>
            terminate(state, value.guard.message_id, Some(value.cause.clone())),
        ConversationUpdate::EditMessage(value) =>
            edit_message(state, value.guard.message_id, &value.entries),
        ConversationUpdate::DeleteMessage(value) => delete_message(state, value.guard.message_id),
        ConversationUpdate::Resend(value) => {
            let source = state.message(value.source_guard.message_id)
                .ok_or(ConversationError::UnknownMessage {
                    message_id: value.source_guard.message_id,
                })?;
            if !message_terminal(&source.status) {
                return Err(ConversationError::ResendRequiresTerminal { message_id: source.id });
            }
            if source.role != value.message.role {
                return Err(ConversationError::InvalidMessage {
                    message_id: value.message.id, reason: "resend role must match source",
                });
            }
            push(state, &value.message, Some(source.id))
        }
    }
}

fn push(state: &mut ConversationState, message: &ChatMessage, _resend_source: Option<MessageId>)
    -> Result<(), ConversationError> {
    if state.seen_messages.contains(&message.id) {
        return Err(ConversationError::DuplicateMessageId { message_id: message.id });
    }
    if message.status != MessageStatus::Pending || message.revision != MessageRevision::INITIAL {
        return Err(ConversationError::InvalidMessage {
            message_id: message.id, reason: "new message must be pending at initial revision",
        });
    }
    if message.blocks.is_empty() {
        return Err(ConversationError::InvalidMessage {
            message_id: message.id, reason: "message must contain at least one block",
        });
    }
    let permitted_result_calls = live_call_ids(state);
    state.seen_messages.insert(message.id);
    state.thinking_seen.entry(message.id).or_default();
    state.thinking_retired.entry(message.id).or_default();
    for entry in &message.blocks {
        register_new_entry(state, message.id, entry, false, &permitted_result_calls)?;
    }
    state.messages.push(message.clone());
    Ok(())
}

fn register_new_entry(state: &mut ConversationState, message_id: MessageId,
    entry: &MessageBlockEntry, require_static_payload: bool,
    permitted_result_calls: &BTreeSet<ToolCallId>) -> Result<(), ConversationError> {
    if state.retired_blocks.contains(&entry.id) {
        return Err(ConversationError::RetiredBlockId { block_id: entry.id });
    }
    if state.seen_blocks.contains(&entry.id) {
        return Err(ConversationError::DuplicateBlockId { block_id: entry.id });
    }
    match &entry.block {
        MessageBlock::Thinking(value) => {
            if value.status != ThinkingStatus::Pending {
                return invalid_transition("thinking", "new thinking block must be pending");
            }
            let retired = state.thinking_retired.entry(message_id).or_default();
            if retired.contains(&value.id) {
                return Err(ConversationError::RetiredThinkingId {
                    message_id, thinking_id: value.id.as_str().to_owned(),
                });
            }
            let seen = state.thinking_seen.entry(message_id).or_default();
            if !seen.insert(value.id.clone()) {
                return Err(ConversationError::DuplicateThinkingId {
                    message_id, thinking_id: value.id.as_str().to_owned(),
                });
            }
        }
        MessageBlock::ToolCall(value) => {
            if value.status != ToolCallStatus::Pending {
                return invalid_transition("tool_call", "new tool call must be pending");
            }
            if state.seen_tool_calls.contains(&value.call_id) {
                return Err(ConversationError::DuplicateToolCallId {
                    call_id: value.call_id.as_str().to_owned(),
                });
            }
            state.seen_tool_calls.insert(value.call_id.clone());
            state.result_slots.insert(value.call_id.clone(), ToolResultSlot::Vacant);
        }
        MessageBlock::ToolResult(value) => {
            if value.status != ToolResultStatus::Pending {
                return invalid_transition("tool_result", "new tool result must be pending");
            }
            if !permitted_result_calls.contains(&value.call_id) {
                return Err(ConversationError::OrphanToolResult {
                    call_id: value.call_id.as_str().to_owned(),
                });
            }
            match state.result_slots.get(&value.call_id) {
                Some(ToolResultSlot::Vacant) => {}
                Some(ToolResultSlot::Retired) => return Err(ConversationError::ResultSlotRetired {
                    call_id: value.call_id.as_str().to_owned(),
                }),
                Some(ToolResultSlot::Occupied(_)) => return Err(ConversationError::InvalidCorrelation {
                    call_id: value.call_id.as_str().to_owned(), reason: "tool result already exists",
                }),
                None => return Err(ConversationError::OrphanToolResult {
                    call_id: value.call_id.as_str().to_owned(),
                }),
            }
            state.result_slots.insert(value.call_id.clone(),
                ToolResultSlot::Occupied(ToolResultLocation::new(message_id, entry.id)));
        }
        block if require_static_payload && !static_payload_nonempty(block) => {
            return Err(ConversationError::InvalidMessage {
                message_id, reason: "late-discovered static block must have non-empty payload",
            });
        }
        _ => {}
    }
    state.seen_blocks.insert(entry.id);
    Ok(())
}

fn static_payload_nonempty(block: &MessageBlock) -> bool {
    match block {
        MessageBlock::Text(value) | MessageBlock::Markdown(value) => !value.is_empty(),
        MessageBlock::Code(value) => !value.content().is_empty(),
        MessageBlock::Thinking(_) | MessageBlock::ToolCall(_) | MessageBlock::ToolResult(_) => true,
        _ => true,
    }
}

fn append_text(state: &mut ConversationState, message_id: MessageId, block_id: BlockId,
    delta: &str) -> Result<(), ConversationError> {
    if delta.is_empty() {
        return Err(ConversationError::InvalidValue { field: "delta", reason: "must be non-empty" });
    }
    let message = message_mut(state, message_id)?;
    require_active(message)?;
    let entry = message.blocks.iter_mut().find(|entry| entry.id == block_id)
        .ok_or(ConversationError::UnknownBlock { message_id, block_id })?;
    match &mut entry.block {
        MessageBlock::Text(value) | MessageBlock::Markdown(value) => value.push_str(delta),
        MessageBlock::Code(value) => {
            let mut replacement = CodeContent::new(format!("{}{}", value.content(), delta))?;
            if let Some(language) = value.language() {
                replacement = replacement.with_language(language)?;
            }
            entry.block = MessageBlock::Code(replacement);
        }
        MessageBlock::Thinking(value) => match value.status {
            ThinkingStatus::Pending => {
                value.content.push_str(delta); value.status = ThinkingStatus::Streaming;
            }
            ThinkingStatus::Streaming => value.content.push_str(delta),
            _ => return invalid_transition("thinking", "cannot append to terminal thinking"),
        },
        _ => return invalid_transition("block", "block is not appendable"),
    }
    if message.status == MessageStatus::Pending { message.status = MessageStatus::Streaming; }
    Ok(())
}

fn insert_block(state: &mut ConversationState, message_id: MessageId, position: Option<usize>,
    entry: &MessageBlockEntry) -> Result<(), ConversationError> {
    let message = state.message(message_id)
        .ok_or(ConversationError::UnknownMessage { message_id })?;
    require_active(message)?;
    let actual_position = position.unwrap_or(message.blocks.len());
    if actual_position > message.blocks.len() {
        return Err(ConversationError::InvalidMessage {
            message_id, reason: "insert position is out of bounds",
        });
    }
    let permitted = live_call_ids(state);
    register_new_entry(state, message_id, entry, true, &permitted)?;
    let message = message_mut(state, message_id)?;
    message.blocks.insert(actual_position, entry.clone());
    if message.status == MessageStatus::Pending { message.status = MessageStatus::Streaming; }
    Ok(())
}

fn replace_block(state: &mut ConversationState, message_id: MessageId, block_id: BlockId,
    replacement: &MessageBlock) -> Result<(), ConversationError> {
    let message = message_mut(state, message_id)?;
    require_active(message)?;
    let entry = message.blocks.iter_mut().find(|entry| entry.id == block_id)
        .ok_or(ConversationError::UnknownBlock { message_id, block_id })?;
    if !same_block_identity(&entry.block, replacement) {
        return Err(ConversationError::InvalidReplacement {
            block_id, reason: "replacement must preserve block kind and lifecycle identity",
        });
    }
    validate_replacement(&entry.block, replacement, block_id)?;
    let starts_nested = match (&entry.block, replacement) {
        (MessageBlock::Thinking(old), MessageBlock::Thinking(new)) =>
            old.status == ThinkingStatus::Pending && new.status == ThinkingStatus::Streaming,
        (MessageBlock::ToolCall(old), MessageBlock::ToolCall(new)) =>
            old.status == ToolCallStatus::Pending && new.status == ToolCallStatus::Running,
        (MessageBlock::ToolResult(old), MessageBlock::ToolResult(new)) =>
            old.status == ToolResultStatus::Pending && new.status == ToolResultStatus::Streaming,
        _ => false,
    };
    entry.block = replacement.clone();
    if starts_nested && message.status == MessageStatus::Pending {
        message.status = MessageStatus::Streaming;
    }
    Ok(())
}

fn validate_replacement(old: &MessageBlock, new: &MessageBlock, block_id: BlockId)
    -> Result<(), ConversationError> {
    match (old, new) {
        (MessageBlock::Thinking(old), MessageBlock::Thinking(new))
            if old.status != new.status && !valid_thinking_transition(&old.status, &new.status) =>
            invalid_transition("thinking", "invalid thinking status transition"),
        (MessageBlock::ToolCall(old), MessageBlock::ToolCall(new))
            if old.status != new.status && !valid_call_transition(&old.status, &new.status) =>
            invalid_transition("tool_call", "invalid tool-call status transition"),
        (MessageBlock::ToolResult(old), MessageBlock::ToolResult(new))
            if old.status != new.status && !valid_result_transition(&old.status, &new.status) =>
            invalid_transition("tool_result", "invalid tool-result status transition"),
        _ if old == new => Err(ConversationError::InvalidReplacement {
            block_id, reason: "replacement must change content or status",
        }),
        _ => Ok(()),
    }
}

fn complete(state: &mut ConversationState, message_id: MessageId)
    -> Result<(), ConversationError> {
    let message = message_mut(state, message_id)?;
    require_active(message)?;
    let ready = match message.status {
        MessageStatus::Pending => static_complete_ready(message),
        MessageStatus::Streaming => message.blocks.iter().all(|entry| nested_terminal(&entry.block)),
        _ => false,
    };
    if !ready
        || !valid_message_transition(&message.status, &MessageStatus::Complete, ready) {
        return invalid_transition("message", "message content is not ready to complete");
    }
    message.status = MessageStatus::Complete;
    Ok(())
}

fn terminate(state: &mut ConversationState, target_id: MessageId,
    cause: Option<FailureCause>) -> Result<(), ConversationError> {
    let target = state.message(target_id)
        .ok_or(ConversationError::UnknownMessage { message_id: target_id })?;
    require_active(target)?;
    let correlations = target.blocks.iter().filter_map(|entry| match &entry.block {
        MessageBlock::ToolCall(value) => Some(value.call_id.clone()),
        MessageBlock::ToolResult(value) => Some(value.call_id.clone()),
        _ => None,
    }).collect::<BTreeSet<_>>();
    for message in &mut state.messages {
        let is_target = message.id == target_id;
        for entry in &mut message.blocks {
            let correlated = match &entry.block {
                MessageBlock::ToolCall(value) => correlations.contains(&value.call_id),
                MessageBlock::ToolResult(value) => correlations.contains(&value.call_id),
                _ => false,
            };
            if nested_active(&entry.block) && (is_target || correlated) {
                terminate_block(&mut entry.block, cause.as_ref());
            }
        }
        if is_target {
            message.status = cause.as_ref().map_or(MessageStatus::Cancelled,
                |value| MessageStatus::Failed(value.clone()));
        }
    }
    Ok(())
}

fn terminate_block(block: &mut MessageBlock, cause: Option<&FailureCause>) {
    match block {
        MessageBlock::Thinking(value) => value.status = cause.map_or(ThinkingStatus::Cancelled,
            |cause| ThinkingStatus::Failed(cause.clone())),
        MessageBlock::ToolCall(value) => value.status = cause.map_or(ToolCallStatus::Cancelled,
            |cause| ToolCallStatus::Failed(cause.clone())),
        MessageBlock::ToolResult(value) => value.status = cause.map_or(ToolResultStatus::Cancelled,
            |cause| ToolResultStatus::Failed(cause.clone())),
        _ => {}
    }
}

fn edit_message(state: &mut ConversationState, message_id: MessageId,
    entries: &[MessageBlockEntry]) -> Result<(), ConversationError> {
    if entries.is_empty() {
        return Err(ConversationError::InvalidMessage {
            message_id, reason: "edited message must contain at least one block",
        });
    }
    let current = state.message(message_id)
        .ok_or(ConversationError::UnknownMessage { message_id })?.clone();
    let old = current.blocks.iter().map(|entry| (entry.id, entry))
        .collect::<BTreeMap<_, _>>();
    let candidate = entries.iter().map(|entry| (entry.id, entry))
        .collect::<BTreeMap<_, _>>();
    if candidate.len() != entries.len() {
        return Err(ConversationError::DuplicateBlockId {
            block_id: entries.iter().map(|entry| entry.id)
                .find(|id| entries.iter().filter(|entry| entry.id == *id).count() > 1).unwrap(),
        });
    }
    for (id, replacement) in &candidate {
        if let Some(previous) = old.get(id) {
            if !same_block_identity(&previous.block, &replacement.block) {
                return Err(ConversationError::InvalidReplacement {
                    block_id: *id, reason: "edit must preserve retained block identity",
                });
            }
            if lifecycle_status_changed(&previous.block, &replacement.block) {
                return Err(ConversationError::InvalidReplacement {
                    block_id: *id, reason: "edit cannot change retained lifecycle status",
                });
            }
        } else if message_terminal(&current.status)
            && matches!(replacement.block, MessageBlock::Thinking(_)
                | MessageBlock::ToolCall(_) | MessageBlock::ToolResult(_)) {
            return Err(ConversationError::InvalidMessage {
                message_id, reason: "terminal message may only add static blocks",
            });
        }
    }
    for entry in current.blocks.iter().filter(|entry| !candidate.contains_key(&entry.id)) {
        retire_entry(state, message_id, entry);
    }
    let permitted = live_call_ids(state);
    for entry in entries.iter().filter(|entry| !old.contains_key(&entry.id)) {
        register_new_entry(state, message_id, entry, true, &permitted)?;
    }
    message_mut(state, message_id)?.blocks = entries.to_vec();
    Ok(())
}

fn lifecycle_status_changed(old: &MessageBlock, new: &MessageBlock) -> bool {
    match (old, new) {
        (MessageBlock::Thinking(a), MessageBlock::Thinking(b)) => a.status != b.status,
        (MessageBlock::ToolCall(a), MessageBlock::ToolCall(b)) => a.status != b.status,
        (MessageBlock::ToolResult(a), MessageBlock::ToolResult(b)) => a.status != b.status,
        _ => false,
    }
}

fn delete_message(state: &mut ConversationState, message_id: MessageId)
    -> Result<(), ConversationError> {
    let position = state.messages.iter().position(|message| message.id == message_id)
        .ok_or(ConversationError::UnknownMessage { message_id })?;
    let message = state.messages[position].clone();
    for entry in &message.blocks { retire_entry(state, message_id, entry); }
    state.messages.remove(position);
    state.retired_messages.insert(message_id);
    Ok(())
}

fn retire_entry(state: &mut ConversationState, message_id: MessageId,
    entry: &MessageBlockEntry) {
    state.retired_blocks.insert(entry.id);
    match &entry.block {
        MessageBlock::Thinking(value) => {
            state.thinking_retired.entry(message_id).or_default().insert(value.id.clone());
        }
        MessageBlock::ToolCall(value) => {
            state.retired_tool_calls.insert(value.call_id.clone());
            state.result_slots.insert(value.call_id.clone(), ToolResultSlot::Retired);
        }
        MessageBlock::ToolResult(value) => {
            state.result_slots.insert(value.call_id.clone(), ToolResultSlot::Retired);
        }
        _ => {}
    }
}

fn live_call_ids(state: &ConversationState) -> BTreeSet<ToolCallId> {
    state.messages.iter().flat_map(|message| message.blocks.iter()).filter_map(|entry| {
        if let MessageBlock::ToolCall(value) = &entry.block {
            Some(value.call_id.clone())
        } else { None }
    }).collect()
}

fn validate_conversation(state: &ConversationState) -> Result<(), ConversationError> {
    let mut calls = BTreeMap::<ToolCallId, &ToolCallStatus>::new();
    let mut results = BTreeMap::<ToolCallId, (&ToolResultStatus, ToolResultLocation)>::new();
    for message in &state.messages {
        if message_terminal(&message.status)
            && message.blocks.iter().any(|entry| nested_active(&entry.block)) {
            return Err(ConversationError::InvalidMessage {
                message_id: message.id, reason: "terminal message contains active nested block",
            });
        }
        for entry in &message.blocks {
            match &entry.block {
                MessageBlock::ToolCall(value) => {
                    if calls.insert(value.call_id.clone(), &value.status).is_some() {
                        return Err(ConversationError::DuplicateToolCallId {
                            call_id: value.call_id.as_str().to_owned(),
                        });
                    }
                }
                MessageBlock::ToolResult(value) => {
                    if results.insert(value.call_id.clone(), (&value.status,
                        ToolResultLocation::new(message.id, entry.id))).is_some() {
                        return Err(ConversationError::InvalidCorrelation {
                            call_id: value.call_id.as_str().to_owned(),
                            reason: "tool call has more than one result",
                        });
                    }
                }
                _ => {}
            }
        }
    }
    for (call_id, (result, location)) in &results {
        let call = calls.get(call_id).ok_or_else(|| ConversationError::OrphanToolResult {
            call_id: call_id.as_str().to_owned(),
        })?;
        if !valid_call_result(call, Some(result)) {
            return Err(ConversationError::InvalidCorrelation {
                call_id: call_id.as_str().to_owned(), reason: "call/result status matrix rejected",
            });
        }
        if !matches!(state.result_slots.get(call_id),
            Some(ToolResultSlot::Occupied(found)) if found == location) {
            return Err(ConversationError::InvalidCorrelation {
                call_id: call_id.as_str().to_owned(), reason: "result slot location is inconsistent",
            });
        }
    }
    for (call_id, call) in calls {
        if !valid_call_result(call, results.get(&call_id).map(|(status, _)| *status)) {
            return Err(ConversationError::InvalidCorrelation {
                call_id: call_id.as_str().to_owned(), reason: "call/result status matrix rejected",
            });
        }
    }
    Ok(())
}

fn message_mut(state: &mut ConversationState, message_id: MessageId)
    -> Result<&mut ChatMessage, ConversationError> {
    state.messages.iter_mut().find(|message| message.id == message_id)
        .ok_or(ConversationError::UnknownMessage { message_id })
}

fn require_active(message: &ChatMessage) -> Result<(), ConversationError> {
    if message_active(&message.status) { Ok(()) } else {
        invalid_transition("message", "message is terminal")
    }
}

fn invalid_transition<T>(kind: &'static str, reason: &'static str)
    -> Result<T, ConversationError> {
    Err(ConversationError::InvalidTransition { kind, reason })
}

macro_rules! failure_cause {
    ($ty:ty) => {
        impl $ty {
            /// Returns the typed cause when this status is failed.
            pub fn failure_cause(&self) -> Option<&FailureCause> {
                match self { Self::Failed(cause) => Some(cause), _ => None }
            }
        }
    };
}
failure_cause!(MessageStatus);
failure_cause!(ThinkingStatus);
failure_cause!(ToolCallStatus);
failure_cause!(ToolResultStatus);

impl PushUpdate {
    /// Returns the expected conversation revision.
    pub const fn guard(&self) -> ConversationGuard { self.guard }
    /// Returns the new message.
    pub const fn message(&self) -> &ChatMessage { &self.message }
}
impl AppendTextUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
    /// Returns the append target.
    pub const fn block_id(&self) -> BlockId { self.block_id }
    /// Returns the non-empty delta.
    pub fn delta(&self) -> &str { &self.delta }
}
impl BlockUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
    /// Returns the new entry.
    pub const fn entry(&self) -> &MessageBlockEntry { &self.entry }
}
impl InsertBlockUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
    /// Returns the checked insertion position.
    pub const fn position(&self) -> usize { self.position }
    /// Returns the new entry.
    pub const fn entry(&self) -> &MessageBlockEntry { &self.entry }
}
impl ReplaceBlockUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
    /// Returns the replacement target.
    pub const fn block_id(&self) -> BlockId { self.block_id }
    /// Returns the same-kind replacement.
    pub const fn replacement(&self) -> &MessageBlock { &self.replacement }
}
impl GuardedUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
}
impl FailUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
    /// Returns the typed failure cause.
    pub const fn cause(&self) -> &FailureCause { &self.cause }
}
impl EditMessageUpdate {
    /// Returns the mutation guard.
    pub const fn guard(&self) -> MessageMutationGuard { self.guard }
    /// Returns the complete replacement entry list.
    pub fn entries(&self) -> &[MessageBlockEntry] { &self.entries }
}
impl ResendUpdate {
    /// Returns the terminal source guard.
    pub const fn source_guard(&self) -> MessageMutationGuard { self.source_guard }
    /// Returns the fresh message.
    pub const fn message(&self) -> &ChatMessage { &self.message }
}
}

#[cfg(test)]
#[rustfmt::skip]
mod coverage_tests {
    use super::super::*; use std::num::NonZeroUsize;
    fn message() -> ChatMessage {
        ChatMessage::new(
            MessageId::new(1),
            ChatRole::User,
            vec![MessageBlockEntry::new(
                BlockId::new(1),
                MessageBlock::Text("text".into()),
            )],
        )
        .unwrap()
    }
    #[test]
    fn public_payload_and_snapshot_accessors_are_live() {
        let conversation = ConversationGuard::new(ConversationRevision::INITIAL);
        let mutation =
            MessageMutationGuard::new(conversation, MessageId::new(1), MessageRevision::INITIAL);
        let entry = MessageBlockEntry::new(BlockId::new(2), MessageBlock::Markdown("md".into()));
        if let ConversationUpdate::Push(value) = ConversationUpdate::push(conversation, message()) {
            assert_eq!(value.guard(), conversation);
            assert_eq!(value.message().id(), MessageId::new(1));
        }
        if let ConversationUpdate::AppendText(value) =
            ConversationUpdate::append_text(mutation, BlockId::new(1), "delta").unwrap()
        {
            assert_eq!(value.guard(), mutation);
            assert_eq!(value.block_id(), BlockId::new(1));
            assert_eq!(value.delta(), "delta");
        }
        if let ConversationUpdate::AppendMessageBlock(value) =
            ConversationUpdate::append_message_block(mutation, entry.clone())
        {
            assert_eq!(value.guard(), mutation);
            assert_eq!(value.entry(), &entry);
        }
        if let ConversationUpdate::InsertMessageBlock(value) =
            ConversationUpdate::insert_message_block(mutation, 0, entry.clone())
        {
            assert_eq!(value.guard(), mutation);
            assert_eq!(value.position(), 0);
            assert_eq!(value.entry(), &entry);
        }
        if let ConversationUpdate::ReplaceBlock(value) = ConversationUpdate::replace_block(
            mutation,
            BlockId::new(1),
            MessageBlock::Text("next".into()),
        ) {
            assert_eq!(value.guard(), mutation);
            assert_eq!(value.block_id(), BlockId::new(1));
            assert!(matches!(value.replacement(), MessageBlock::Text(_)));
        }
        if let ConversationUpdate::Complete(value) = ConversationUpdate::complete(mutation) {
            assert_eq!(value.guard(), mutation);
        }
        let cause = FailureCause::new("cause").unwrap();
        if let ConversationUpdate::Fail(value) = ConversationUpdate::fail(mutation, cause.clone()) {
            assert_eq!(value.guard(), mutation);
            assert_eq!(value.cause(), &cause);
        }
        if let ConversationUpdate::EditMessage(value) =
            ConversationUpdate::edit_message(mutation, vec![entry.clone()])
        {
            assert_eq!(value.guard(), mutation);
            assert_eq!(value.entries(), &[entry.clone()]);
        }
        if let ConversationUpdate::Resend(value) = ConversationUpdate::resend(mutation, message()) {
            assert_eq!(value.source_guard(), mutation);
            assert_eq!(value.message().id(), MessageId::new(1));
        }
        assert_eq!(
            MessageStatus::Failed(cause.clone()).failure_cause(),
            Some(&cause)
        );
        assert_eq!(
            ThinkingStatus::Failed(cause.clone()).failure_cause(),
            Some(&cause)
        );
        assert_eq!(
            ToolCallStatus::Failed(cause.clone()).failure_cause(),
            Some(&cause)
        );
        assert_eq!(
            ToolResultStatus::Failed(cause.clone()).failure_cause(),
            Some(&cause)
        );

        let state = ConversationState::new(4, NonZeroUsize::MIN);
        let snapshot = state.snapshot();
        assert!(snapshot.messages().is_empty());
        assert_eq!(snapshot.revision(), ConversationRevision::INITIAL);
        assert_eq!(snapshot.expected_sequence(), 4);
        assert_eq!(snapshot.retention().capacity(), NonZeroUsize::MIN);
        assert!(snapshot.retention().records().is_empty());
        assert_eq!(snapshot.retention().evicted_through(), None);
        let identities = snapshot.identities();
        assert!(identities.seen_messages().is_empty());
        assert!(identities.retired_messages().is_empty());
        assert!(identities.seen_blocks().is_empty());
        assert!(identities.retired_blocks().is_empty());
        assert!(identities.thinking().is_empty());
        assert!(identities.seen_tool_calls().is_empty());
        assert!(identities.retired_tool_calls().is_empty());
        assert!(identities.result_slots().is_empty());
        assert!(!ConversationError::SequenceExhausted.to_string().is_empty());
    }
}
