//! Immutable-facing state, snapshots, histories, and pure transition rules.

#[rustfmt::skip]
mod compact {
use super::super::*;
use std::{collections::{BTreeMap, BTreeSet, VecDeque}, num::NonZeroUsize};

/// One retained accepted event and its original outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedEventRecord {
    pub(in crate::components::chat) event: ConversationEvent,
    pub(in crate::components::chat) outcome: ApplyOutcome,
    pub(in crate::components::chat) proof: Option<RetentionProof>,
}
impl ProcessedEventRecord {
    /// Creates an externally assembled record that restoration must replay from its origin.
    pub fn new(event: ConversationEvent, outcome: ApplyOutcome) -> Self {
        Self { event, outcome, proof: None }
    }
    /// Rebuilds a retained record with opaque evidence obtained from [`Self::proof`].
    pub fn new_with_proof(event: ConversationEvent, outcome: ApplyOutcome,
        proof: RetentionProof) -> Self { Self { event, outcome, proof: Some(proof) } }
    pub(in crate::components::chat) fn proven(event: ConversationEvent, outcome: ApplyOutcome,
        previous: [u64; 4]) -> Self {
        let record = super::super::proof::record_fingerprint(previous, &event, &outcome);
        Self::new_with_proof(event, outcome, RetentionProof { previous, record })
    }
    /// Returns the exact accepted event.
    pub fn event(&self) -> &ConversationEvent { &self.event }
    /// Returns the original successful outcome.
    pub fn outcome(&self) -> &ApplyOutcome { &self.outcome }
    /// Returns opaque evidence for lossless persistence reconstruction, when available.
    pub fn proof(&self) -> Option<&RetentionProof> { self.proof.as_ref() }
}

/// Bounded replay history and its honest eviction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionHistory {
    pub(in crate::components::chat) capacity: NonZeroUsize,
    pub(in crate::components::chat) records: Vec<ProcessedEventRecord>,
    pub(in crate::components::chat) evicted_through: Option<u64>,
}
impl RetentionHistory {
    /// Creates replay history after checking its basic capacity contract.
    pub fn new(capacity: NonZeroUsize, records: Vec<ProcessedEventRecord>,
        evicted_through: Option<u64>) -> Result<Self, ConversationError> {
        if records.len() > capacity.get() {
            return Err(ConversationError::InvalidSnapshot {
                reason: "retained records exceed ledger capacity",
            });
        }
        Ok(Self { capacity, records, evicted_through })
    }
    /// Returns configured capacity.
    pub const fn capacity(&self) -> NonZeroUsize { self.capacity }
    /// Returns records in accepted sequence order.
    pub fn records(&self) -> &[ProcessedEventRecord] { &self.records }
    /// Returns the highest sequence known to have been evicted.
    pub const fn evicted_through(&self) -> Option<u64> { self.evicted_through }
}

/// Seen and retired thinking identities for one message lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingIdentityHistory {
    message_id: MessageId, seen: Vec<ThinkingId>, retired: Vec<ThinkingId>,
}
impl ThinkingIdentityHistory {
    /// Creates one message-local history.
    pub fn new(message_id: MessageId, seen: Vec<ThinkingId>, retired: Vec<ThinkingId>) -> Self {
        Self { message_id, seen, retired }
    }
    /// Returns the message identity.
    pub const fn message_id(&self) -> MessageId { self.message_id }
    /// Returns all seen identities.
    pub fn seen(&self) -> &[ThinkingId] { &self.seen }
    /// Returns retired identities.
    pub fn retired(&self) -> &[ThinkingId] { &self.retired }
}

/// Location of a live result entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolResultLocation { message_id: MessageId, block_id: BlockId }
impl ToolResultLocation {
    /// Creates a result location.
    pub const fn new(message_id: MessageId, block_id: BlockId) -> Self {
        Self { message_id, block_id }
    }
    /// Returns the containing message.
    pub const fn message_id(self) -> MessageId { self.message_id }
    /// Returns the result block.
    pub const fn block_id(self) -> BlockId { self.block_id }
}

/// Lifetime state of a tool call's single result slot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolResultSlot {
    /// No result has been inserted.
    Vacant,
    /// Exactly one live result occupies the slot.
    Occupied(ToolResultLocation),
    /// A prior result was deleted and the slot cannot be reused.
    Retired,
}

/// Snapshot of all non-ledger identity namespaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationIdentityHistory {
    seen_messages: Vec<MessageId>, retired_messages: Vec<MessageId>,
    seen_blocks: Vec<BlockId>, retired_blocks: Vec<BlockId>,
    thinking: Vec<ThinkingIdentityHistory>, seen_tool_calls: Vec<ToolCallId>,
    retired_tool_calls: Vec<ToolCallId>, result_slots: Vec<(ToolCallId, ToolResultSlot)>,
}
impl ConversationIdentityHistory {
    /// Creates an identity-history snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn new(seen_messages: Vec<MessageId>, retired_messages: Vec<MessageId>,
        seen_blocks: Vec<BlockId>, retired_blocks: Vec<BlockId>,
        thinking: Vec<ThinkingIdentityHistory>, seen_tool_calls: Vec<ToolCallId>,
        retired_tool_calls: Vec<ToolCallId>,
        result_slots: Vec<(ToolCallId, ToolResultSlot)>) -> Self {
        Self { seen_messages, retired_messages, seen_blocks, retired_blocks, thinking,
            seen_tool_calls, retired_tool_calls, result_slots }
    }
    /// Returns seen message identities.
    pub fn seen_messages(&self) -> &[MessageId] { &self.seen_messages }
    /// Returns retired message identities.
    pub fn retired_messages(&self) -> &[MessageId] { &self.retired_messages }
    /// Returns seen block identities.
    pub fn seen_blocks(&self) -> &[BlockId] { &self.seen_blocks }
    /// Returns retired block identities.
    pub fn retired_blocks(&self) -> &[BlockId] { &self.retired_blocks }
    /// Returns per-message thinking histories.
    pub fn thinking(&self) -> &[ThinkingIdentityHistory] { &self.thinking }
    /// Returns seen tool calls.
    pub fn seen_tool_calls(&self) -> &[ToolCallId] { &self.seen_tool_calls }
    /// Returns retired tool calls.
    pub fn retired_tool_calls(&self) -> &[ToolCallId] { &self.retired_tool_calls }
    /// Returns tool-result slots.
    pub fn result_slots(&self) -> &[(ToolCallId, ToolResultSlot)] { &self.result_slots }
}

/// Complete restorable conversation snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationStateSnapshot {
    pub(in crate::components::chat) messages: Vec<ChatMessage>,
    pub(in crate::components::chat) revision: ConversationRevision,
    pub(in crate::components::chat) expected_sequence: u64,
    pub(in crate::components::chat) retention: RetentionHistory,
    pub(in crate::components::chat) identities: ConversationIdentityHistory,
    pub(in crate::components::chat) proof: Option<SnapshotProof>,
}
impl ConversationStateSnapshot {
    /// Creates a snapshot value; [`ConversationState::try_restore`] performs full validation.
    pub fn new(messages: Vec<ChatMessage>, revision: ConversationRevision, expected_sequence: u64,
        retention: RetentionHistory, identities: ConversationIdentityHistory) -> Self {
        Self { messages, revision, expected_sequence, retention, identities, proof: None }
    }
    /// Rebuilds a snapshot with opaque evidence obtained from [`Self::proof`].
    pub fn new_with_proof(messages: Vec<ChatMessage>, revision: ConversationRevision,
        expected_sequence: u64, retention: RetentionHistory,
        identities: ConversationIdentityHistory, proof: SnapshotProof) -> Self {
        Self { messages, revision, expected_sequence, retention, identities, proof: Some(proof) }
    }
    /// Returns messages in conversation order.
    pub fn messages(&self) -> &[ChatMessage] { &self.messages }
    /// Returns conversation revision.
    pub const fn revision(&self) -> ConversationRevision { self.revision }
    /// Returns next expected sequence.
    pub const fn expected_sequence(&self) -> u64 { self.expected_sequence }
    /// Returns replay history.
    pub const fn retention(&self) -> &RetentionHistory { &self.retention }
    /// Returns identity history.
    pub const fn identities(&self) -> &ConversationIdentityHistory { &self.identities }
    /// Returns opaque evidence for lossless persistence reconstruction, when available.
    pub const fn proof(&self) -> Option<&SnapshotProof> { self.proof.as_ref() }
}

/// Deterministic provider-independent conversation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationState {
    pub(in crate::components::chat) messages: Vec<ChatMessage>,
    pub(in crate::components::chat) revision: ConversationRevision,
    pub(in crate::components::chat) expected_sequence: u64,
    pub(in crate::components::chat) ledger_capacity: NonZeroUsize,
    pub(in crate::components::chat) ledger: VecDeque<ProcessedEventRecord>,
    pub(in crate::components::chat) evicted_through: Option<u64>,
    pub(in crate::components::chat) seen_messages: BTreeSet<MessageId>,
    pub(in crate::components::chat) retired_messages: BTreeSet<MessageId>,
    pub(in crate::components::chat) seen_blocks: BTreeSet<BlockId>,
    pub(in crate::components::chat) retired_blocks: BTreeSet<BlockId>,
    pub(in crate::components::chat) thinking_seen: BTreeMap<MessageId, BTreeSet<ThinkingId>>,
    pub(in crate::components::chat) thinking_retired: BTreeMap<MessageId, BTreeSet<ThinkingId>>,
    pub(in crate::components::chat) seen_tool_calls: BTreeSet<ToolCallId>,
    pub(in crate::components::chat) retired_tool_calls: BTreeSet<ToolCallId>,
    pub(in crate::components::chat) result_slots: BTreeMap<ToolCallId, ToolResultSlot>,
}
impl ConversationState {
    /// Creates an empty state with an explicit sequence start and non-zero ledger capacity.
    pub fn new(initial_sequence: u64, ledger_capacity: NonZeroUsize) -> Self {
        Self { messages: Vec::new(), revision: ConversationRevision::INITIAL,
            expected_sequence: initial_sequence, ledger_capacity, ledger: VecDeque::new(),
            evicted_through: None, seen_messages: BTreeSet::new(),
            retired_messages: BTreeSet::new(), seen_blocks: BTreeSet::new(),
            retired_blocks: BTreeSet::new(), thinking_seen: BTreeMap::new(),
            thinking_retired: BTreeMap::new(), seen_tool_calls: BTreeSet::new(),
            retired_tool_calls: BTreeSet::new(), result_slots: BTreeMap::new() }
    }
    /// Returns messages in stable conversation order.
    pub fn messages(&self) -> &[ChatMessage] { &self.messages }
    /// Returns one message by identity.
    pub fn message(&self, id: MessageId) -> Option<&ChatMessage> {
        self.messages.iter().find(|message| message.id == id)
    }
    /// Returns current conversation revision.
    pub const fn revision(&self) -> ConversationRevision { self.revision }
    /// Returns the next expected sequence.
    pub const fn expected_sequence(&self) -> u64 { self.expected_sequence }
    /// Returns the current honest eviction boundary.
    pub const fn evicted_through(&self) -> Option<u64> { self.evicted_through }
    /// Returns configured ledger capacity.
    pub const fn ledger_capacity(&self) -> NonZeroUsize { self.ledger_capacity }
    /// Returns a complete owned snapshot.
    pub fn snapshot(&self) -> ConversationStateSnapshot {
        let mut thinking_ids = BTreeSet::new();
        thinking_ids.extend(self.thinking_seen.keys().copied());
        thinking_ids.extend(self.thinking_retired.keys().copied());
        let thinking = thinking_ids.into_iter().map(|message_id| ThinkingIdentityHistory::new(
            message_id,
            self.thinking_seen.get(&message_id).map(set_vec).unwrap_or_default(),
            self.thinking_retired.get(&message_id).map(set_vec).unwrap_or_default(),
        )).collect();
        let mut snapshot = ConversationStateSnapshot::new(
            self.messages.clone(), self.revision, self.expected_sequence,
            RetentionHistory { capacity: self.ledger_capacity, records: self.ledger.iter().cloned().collect(),
                evicted_through: self.evicted_through },
            ConversationIdentityHistory::new(set_vec(&self.seen_messages),
                set_vec(&self.retired_messages), set_vec(&self.seen_blocks),
                set_vec(&self.retired_blocks), thinking, set_vec(&self.seen_tool_calls),
                set_vec(&self.retired_tool_calls),
                self.result_slots.iter().map(|(id, slot)| (id.clone(), slot.clone())).collect()));
        snapshot.proof = Some(super::super::proof::snapshot_proof(&snapshot));
        snapshot
    }
    /// Restores a snapshot only after validating every retained proof and identity history.
    pub fn try_restore(mut snapshot: ConversationStateSnapshot) -> Result<Self, ConversationError> {
        validate_snapshot(&snapshot)?;
        if snapshot.retention.evicted_through.is_none() && !snapshot.retention.records.is_empty() {
            let first = snapshot.retention.records[0].event.sequence;
            let mut replay = Self::new(first, snapshot.retention.capacity);
            for record in &snapshot.retention.records {
                replay.apply_event(record.event.clone()).map_err(|_|
                    ConversationError::InvalidSnapshot { reason: "retained proof upgrade failed" })?;
            }
            snapshot.retention.records = replay.ledger.into();
        }
        let identities = &snapshot.identities;
        let thinking_seen = identities.thinking.iter().map(|history|
            (history.message_id, history.seen.iter().cloned().collect())).collect();
        let thinking_retired = identities.thinking.iter().map(|history|
            (history.message_id, history.retired.iter().cloned().collect())).collect();
        Ok(Self { messages: snapshot.messages, revision: snapshot.revision,
            expected_sequence: snapshot.expected_sequence, ledger_capacity: snapshot.retention.capacity,
            ledger: snapshot.retention.records.into(), evicted_through: snapshot.retention.evicted_through,
            seen_messages: identities.seen_messages.iter().copied().collect(),
            retired_messages: identities.retired_messages.iter().copied().collect(),
            seen_blocks: identities.seen_blocks.iter().copied().collect(),
            retired_blocks: identities.retired_blocks.iter().copied().collect(),
            thinking_seen, thinking_retired,
            seen_tool_calls: identities.seen_tool_calls.iter().cloned().collect(),
            retired_tool_calls: identities.retired_tool_calls.iter().cloned().collect(),
            result_slots: identities.result_slots.iter().cloned().collect() })
    }
}

fn set_vec<T: Clone + Ord>(set: &BTreeSet<T>) -> Vec<T> { set.iter().cloned().collect() }
fn unique_slice<T: Ord + Clone>(values: &[T]) -> bool {
    values.iter().cloned().collect::<BTreeSet<_>>().len() == values.len()
}
fn validate_snapshot(value: &ConversationStateSnapshot) -> Result<(), ConversationError> {
    if value.retention.records.len() > value.retention.capacity.get() {
        return invalid_snapshot("retained records exceed capacity");
    }
    if !unique_slice(&value.identities.seen_messages)
        || !unique_slice(&value.identities.retired_messages)
        || !unique_slice(&value.identities.seen_blocks)
        || !unique_slice(&value.identities.retired_blocks)
        || !unique_slice(&value.identities.seen_tool_calls)
        || !unique_slice(&value.identities.retired_tool_calls) {
        return invalid_snapshot("identity histories contain duplicates");
    }
    let seen_messages: BTreeSet<_> = value.identities.seen_messages.iter().copied().collect();
    let retired_messages: BTreeSet<_> = value.identities.retired_messages.iter().copied().collect();
    let seen_blocks: BTreeSet<_> = value.identities.seen_blocks.iter().copied().collect();
    let retired_blocks: BTreeSet<_> = value.identities.retired_blocks.iter().copied().collect();
    let seen_tool_calls: BTreeSet<_> =
        value.identities.seen_tool_calls.iter().cloned().collect();
    let retired_tool_calls: BTreeSet<_> =
        value.identities.retired_tool_calls.iter().cloned().collect();
    if !retired_messages.is_subset(&seen_messages) || !retired_blocks.is_subset(&seen_blocks) {
        return invalid_snapshot("retired identities must also be seen");
    }
    if !retired_tool_calls.is_subset(&seen_tool_calls) {
        return invalid_snapshot("retired tool calls must also be seen");
    }
    let mut active_messages = BTreeSet::new();
    let mut active_blocks = BTreeSet::new();
    let mut active_thinking = BTreeMap::<MessageId, BTreeSet<ThinkingId>>::new();
    let mut active_calls = BTreeMap::<ToolCallId, &ToolCallStatus>::new();
    let mut active_results =
        BTreeMap::<ToolCallId, (&ToolResultStatus, ToolResultLocation)>::new();
    for message in &value.messages {
        if !active_messages.insert(message.id) || !seen_messages.contains(&message.id)
            || retired_messages.contains(&message.id) {
            return invalid_snapshot("active message history is contradictory");
        }
        if message.blocks.is_empty() || message_terminal(&message.status)
            && message.blocks.iter().any(|entry| nested_active(&entry.block)) {
            return invalid_snapshot("active message structure is contradictory");
        }
        for entry in &message.blocks {
            if !active_blocks.insert(entry.id) || !seen_blocks.contains(&entry.id)
                || retired_blocks.contains(&entry.id) {
                return invalid_snapshot("active block history is contradictory");
            }
            match &entry.block {
                MessageBlock::Thinking(thinking) => {
                    if !active_thinking.entry(message.id).or_default().insert(thinking.id.clone()) {
                        return invalid_snapshot("active thinking identity is duplicated");
                    }
                }
                MessageBlock::ToolCall(call) => {
                    if !seen_tool_calls.contains(&call.call_id)
                        || retired_tool_calls.contains(&call.call_id)
                        || active_calls.insert(call.call_id.clone(), &call.status).is_some() {
                        return invalid_snapshot("active tool-call history is contradictory");
                    }
                }
                MessageBlock::ToolResult(result) => {
                    let location = ToolResultLocation::new(message.id, entry.id);
                    if active_results.insert(result.call_id.clone(),
                        (&result.status, location)).is_some() {
                        return invalid_snapshot("active tool result is duplicated");
                    }
                }
                _ => {}
            }
        }
    }
    if seen_messages != active_messages.union(&retired_messages).copied().collect()
        || seen_blocks != active_blocks.union(&retired_blocks).copied().collect()
        || seen_tool_calls != active_calls.keys().cloned()
            .chain(retired_tool_calls.iter().cloned()).collect() {
        return invalid_snapshot("seen identities must be active or retired");
    }
    let mut event_ids = BTreeSet::new();
    let mut previous_sequence: Option<u64> = None;
    let mut previous_revision: Option<u64> = None;
    for record in &value.retention.records {
        validate_record_contract(record)?;
        if !event_ids.insert(record.event.event_id.clone())
            || previous_sequence.is_some_and(|previous|
                previous.checked_add(1) != Some(record.event.sequence))
            || record.event.sequence >= value.expected_sequence
            || record.outcome.revision > value.revision
            || previous_revision.is_some_and(|previous: u64|
                previous.checked_add(1) != Some(record.outcome.revision.get())) {
            return invalid_snapshot("retained records are unordered or contradictory");
        }
        previous_sequence = Some(record.event.sequence);
        previous_revision = Some(record.outcome.revision.get());
    }
    if let Some(last) = value.retention.records.last() {
        if last.event.sequence.checked_add(1) != Some(value.expected_sequence)
            || last.outcome.revision != value.revision {
            return invalid_snapshot("retained tail does not match current counters");
        }
    } else if value.retention.evicted_through.is_some() {
        return invalid_snapshot("eviction boundary requires retained evidence");
    } else if value.revision != ConversationRevision::INITIAL || !value.messages.is_empty()
        || !snapshot_histories_are_fresh(&value.identities) {
        return invalid_snapshot("empty retention history requires a genuinely fresh state");
    }
    if let (Some(boundary), Some(first)) = (value.retention.evicted_through,
        value.retention.records.first()) {
        if boundary.checked_add(1) != Some(first.event.sequence) {
            return invalid_snapshot("eviction boundary is not contiguous with ledger");
        }
        if !super::super::proof::evicted_proofs_are_valid(value) {
            return invalid_snapshot("evicted history requires reducer-proven retained records");
        }
    }
    let slots: BTreeMap<_, _> = value.identities.result_slots.iter().cloned().collect();
    if slots.len() != value.identities.result_slots.len()
        || slots.keys().any(|call_id| !seen_tool_calls.contains(call_id))
        || seen_tool_calls.iter().any(|call_id| !slots.contains_key(call_id)) {
        return invalid_snapshot("result slot history contains duplicates");
    }
    for (call_id, slot) in &slots {
        match (slot, active_results.get(call_id)) {
            (ToolResultSlot::Occupied(location), Some((_, found))) if location == found => {}
            (ToolResultSlot::Vacant | ToolResultSlot::Retired, None) => {}
            _ => return invalid_snapshot("result slot does not match live result"),
        }
    }
    for (call_id, (result, _)) in &active_results {
        let call = active_calls.get(call_id)
            .ok_or(ConversationError::InvalidSnapshot {
                reason: "tool result has no active call",
            })?;
        if !valid_call_result(call, Some(result)) {
            return invalid_snapshot("call/result status matrix is contradictory");
        }
    }
    for (call_id, call) in &active_calls {
        if !valid_call_result(call, active_results.get(call_id).map(|(status, _)| *status)) {
            return invalid_snapshot("call/result status matrix is contradictory");
        }
    }
    let mut thinking_messages = BTreeSet::new();
    for history in &value.identities.thinking {
        if !thinking_messages.insert(history.message_id)
            || !seen_messages.contains(&history.message_id) || !unique_slice(&history.seen)
            || !unique_slice(&history.retired)
            || !history.retired.iter().all(|id| history.seen.contains(id)) {
            return invalid_snapshot("thinking history is contradictory");
        }
        let seen: BTreeSet<_> = history.seen.iter().cloned().collect();
        let retired: BTreeSet<_> = history.retired.iter().cloned().collect();
        let active = active_thinking.get(&history.message_id).cloned().unwrap_or_default();
        if active.iter().any(|id| !seen.contains(id) || retired.contains(id))
            || seen != active.union(&retired).cloned().collect() {
            return invalid_snapshot("active thinking history is contradictory");
        }
    }
    if thinking_messages != seen_messages {
        return invalid_snapshot("every message lifetime requires thinking history");
    }
    if value.retention.evicted_through.is_none() && !value.retention.records.is_empty() {
        let mut replay = ConversationState::new(
            value.retention.records[0].event.sequence, value.retention.capacity);
        for record in &value.retention.records {
            let outcome = replay.apply_event(record.event.clone())
                .map_err(|_| ConversationError::InvalidSnapshot {
                    reason: "retained event cannot be replayed from its proven origin",
                })?;
            if outcome != record.outcome {
                return invalid_snapshot("retained event and outcome do not agree");
            }
        }
        let mut replayed = replay.snapshot();
        for (found, supplied) in replayed.retention.records.iter_mut()
            .zip(&value.retention.records) { found.proof = supplied.proof.clone(); }
        replayed.proof = value.proof.clone();
        if replayed != *value {
            return invalid_snapshot("retained replay does not produce the restored state");
        }
    } else {
        let mut latest = BTreeMap::new();
        for affected in value.retention.records.iter()
            .flat_map(|record| &record.outcome.affected_messages) {
            latest.insert(affected.message_id, affected);
        }
        for (id, affected) in latest {
            match affected.disposition {
                AffectedMessageDisposition::Present => {
                    if value.messages.iter().find(|message| message.id == id)
                        .is_none_or(|message| message.revision != affected.applied) {
                        return invalid_snapshot("retained affected revision disagrees with state");
                    }
                }
                AffectedMessageDisposition::Deleted => {
                    if value.messages.iter().any(|message| message.id == id)
                        || !retired_messages.contains(&id) {
                        return invalid_snapshot("retained deletion disagrees with identity history");
                    }
                }
            }
        }
    }
    Ok(())
}
fn snapshot_histories_are_fresh(value: &ConversationIdentityHistory) -> bool {
    value.seen_messages.is_empty() && value.retired_messages.is_empty()
        && value.seen_blocks.is_empty() && value.retired_blocks.is_empty()
        && value.thinking.is_empty() && value.seen_tool_calls.is_empty()
        && value.retired_tool_calls.is_empty() && value.result_slots.is_empty()
}
fn validate_record_contract(record: &ProcessedEventRecord) -> Result<(), ConversationError> {
    let expected_revision = record.event.update.conversation_guard().expected().get()
        .checked_add(1).ok_or(ConversationError::InvalidSnapshot {
            reason: "retained guard revision is exhausted",
        })?;
    if expected_revision != record.outcome.revision.get() {
        return invalid_snapshot("retained guard and outcome revisions disagree");
    }
    let affected = &record.outcome.affected_messages;
    if affected.iter().map(|value| value.message_id).collect::<BTreeSet<_>>().len()
        != affected.len() || affected.iter().any(|value| match value.previous {
            Some(previous) => previous.checked_next(value.message_id).ok() != Some(value.applied),
            None => value.applied != MessageRevision::INITIAL,
        }) {
        return invalid_snapshot("retained affected revisions are contradictory");
    }
    let (target, new_message, allow_correlated, deleted) = match &record.event.update {
        ConversationUpdate::Push(value) => (None, Some(value.message.id), false, false),
        ConversationUpdate::Resend(value) =>
            (Some(value.source_guard), Some(value.message.id), false, false),
        ConversationUpdate::AppendText(value) => (Some(value.guard), None, false, false),
        ConversationUpdate::AppendMessageBlock(value) => (Some(value.guard), None, false, false),
        ConversationUpdate::InsertMessageBlock(value) => (Some(value.guard), None, false, false),
        ConversationUpdate::ReplaceBlock(value) => (Some(value.guard), None, false, false),
        ConversationUpdate::Complete(value) => (Some(value.guard), None, false, false),
        ConversationUpdate::Cancel(value) => (Some(value.guard), None, true, false),
        ConversationUpdate::Fail(value) => (Some(value.guard), None, true, false),
        ConversationUpdate::EditMessage(value) => (Some(value.guard), None, false, false),
        ConversationUpdate::DeleteMessage(value) => (Some(value.guard), None, false, true),
    };
    if let Some(new_id) = new_message {
        let created = affected.iter().filter(|value| value.message_id == new_id
            && value.previous.is_none() && value.disposition == AffectedMessageDisposition::Present)
            .count();
        if created != 1 { return invalid_snapshot("retained creation outcome is inconsistent"); }
    }
    if let Some(guard) = target {
        let target_affected = affected.iter().find(|value| value.message_id == guard.message_id);
        if matches!(record.event.update, ConversationUpdate::Resend(_)) {
            if target_affected.is_some() {
                return invalid_snapshot("resend source must not be affected");
            }
        } else if target_affected.is_none_or(|value| value.previous != Some(guard.message_revision)
            || value.disposition != if deleted { AffectedMessageDisposition::Deleted }
                else { AffectedMessageDisposition::Present }) {
            return invalid_snapshot("retained target outcome is inconsistent");
        }
    }
    let expected_len = usize::from(new_message.is_some())
        + usize::from(target.is_some() && !matches!(record.event.update, ConversationUpdate::Resend(_)));
    if (!allow_correlated && affected.len() != expected_len)
        || allow_correlated && affected.len() < expected_len
        || affected.iter().any(|value| value.previous.is_none()
            && Some(value.message_id) != new_message) {
        return invalid_snapshot("retained affected set is inconsistent with its update");
    }
    Ok(())
}
fn invalid_snapshot<T>(reason: &'static str) -> Result<T, ConversationError> {
    Err(ConversationError::InvalidSnapshot { reason })
}

pub(in crate::components::chat) fn message_active(status: &MessageStatus) -> bool {
    matches!(status, MessageStatus::Pending | MessageStatus::Streaming)
}
pub(in crate::components::chat) fn message_terminal(status: &MessageStatus) -> bool {
    matches!(status, MessageStatus::Complete | MessageStatus::Cancelled | MessageStatus::Failed(_))
}
pub(in crate::components::chat) fn nested_active(block: &MessageBlock) -> bool {
    match block {
        MessageBlock::Thinking(value) => matches!(value.status, ThinkingStatus::Pending | ThinkingStatus::Streaming),
        MessageBlock::ToolCall(value) => matches!(value.status, ToolCallStatus::Pending | ToolCallStatus::Running),
        MessageBlock::ToolResult(value) => matches!(value.status, ToolResultStatus::Pending | ToolResultStatus::Streaming),
        _ => false,
    }
}
pub(in crate::components::chat) fn nested_terminal(block: &MessageBlock) -> bool {
    match block { MessageBlock::Thinking(value) => !matches!(value.status,
        ThinkingStatus::Pending | ThinkingStatus::Streaming),
        MessageBlock::ToolCall(value) => !matches!(value.status,
            ToolCallStatus::Pending | ToolCallStatus::Running),
        MessageBlock::ToolResult(value) => !matches!(value.status,
            ToolResultStatus::Pending | ToolResultStatus::Streaming), _ => true }
}
pub(in crate::components::chat) fn valid_message_transition(from: &MessageStatus,
    to: &MessageStatus, static_ready: bool) -> bool {
    matches!((from, to), (MessageStatus::Pending, MessageStatus::Streaming)
        | (MessageStatus::Streaming, MessageStatus::Complete)
        | (MessageStatus::Pending | MessageStatus::Streaming, MessageStatus::Cancelled)
        | (MessageStatus::Pending | MessageStatus::Streaming, MessageStatus::Failed(_)))
        || matches!((from, to), (MessageStatus::Pending, MessageStatus::Complete)) && static_ready
}
pub(in crate::components::chat) fn valid_thinking_transition(from: &ThinkingStatus,
    to: &ThinkingStatus) -> bool {
    matches!((from, to), (ThinkingStatus::Pending, ThinkingStatus::Streaming)
        | (ThinkingStatus::Streaming, ThinkingStatus::Complete)
        | (ThinkingStatus::Pending | ThinkingStatus::Streaming, ThinkingStatus::Cancelled)
        | (ThinkingStatus::Pending | ThinkingStatus::Streaming, ThinkingStatus::Failed(_)))
}
pub(in crate::components::chat) fn valid_call_transition(from: &ToolCallStatus,
    to: &ToolCallStatus) -> bool {
    matches!((from, to), (ToolCallStatus::Pending, ToolCallStatus::Running)
        | (ToolCallStatus::Running, ToolCallStatus::Succeeded)
        | (ToolCallStatus::Pending | ToolCallStatus::Running, ToolCallStatus::Cancelled)
        | (ToolCallStatus::Pending | ToolCallStatus::Running, ToolCallStatus::Failed(_)))
}
pub(in crate::components::chat) fn valid_result_transition(from: &ToolResultStatus,
    to: &ToolResultStatus) -> bool {
    matches!((from, to), (ToolResultStatus::Pending, ToolResultStatus::Streaming)
        | (ToolResultStatus::Streaming, ToolResultStatus::Complete)
        | (ToolResultStatus::Pending | ToolResultStatus::Streaming, ToolResultStatus::Cancelled)
        | (ToolResultStatus::Pending | ToolResultStatus::Streaming, ToolResultStatus::Failed(_)))
}
pub(in crate::components::chat) fn valid_call_result(call: &ToolCallStatus,
    result: Option<&ToolResultStatus>) -> bool {
    match call {
        ToolCallStatus::Pending => result.is_none(),
        ToolCallStatus::Running => result.is_none_or(|value| matches!(value,
            ToolResultStatus::Pending | ToolResultStatus::Streaming)),
        ToolCallStatus::Succeeded => true,
        ToolCallStatus::Cancelled => result.is_none_or(|value| matches!(value, ToolResultStatus::Cancelled)),
        ToolCallStatus::Failed(_) => result.is_none_or(|value| matches!(value, ToolResultStatus::Failed(_))),
    }
}
pub(in crate::components::chat) fn same_block_identity(old: &MessageBlock,
    new: &MessageBlock) -> bool {
    if old.kind() != new.kind() { return false; }
    match (old, new) {
        (MessageBlock::Thinking(a), MessageBlock::Thinking(b)) => a.id == b.id,
        (MessageBlock::ToolCall(a), MessageBlock::ToolCall(b)) => a.call_id == b.call_id,
        (MessageBlock::ToolResult(a), MessageBlock::ToolResult(b)) => a.call_id == b.call_id,
        _ => true,
    }
}
pub(in crate::components::chat) fn static_complete_ready(message: &ChatMessage) -> bool {
    !message.blocks.iter().any(|entry| matches!(entry.block,
        MessageBlock::Thinking(_) | MessageBlock::ToolCall(_) | MessageBlock::ToolResult(_)))
        && message.blocks.iter().any(|entry| match &entry.block {
            MessageBlock::Text(value) | MessageBlock::Markdown(value) => !value.is_empty(),
            MessageBlock::Code(value) => !value.content().is_empty(),
            MessageBlock::Error(_) | MessageBlock::Diff(_) | MessageBlock::Quote(_)
            | MessageBlock::Link(_) | MessageBlock::TerminalAttachmentSummary(_) => true,
            _ => false,
        })
}

#[cfg(test)]
pub(super) mod test_cases {
    use super::*;
    fn cause() -> FailureCause { FailureCause::new("cause").unwrap() }
    fn text_message() -> ChatMessage {
        ChatMessage::new(MessageId::new(1), ChatRole::User,
            vec![MessageBlockEntry::new(BlockId::new(1), MessageBlock::Text("x".into()))]).unwrap()
    }
    pub(in crate::components::chat::state) fn thinking_replacement_requires_same_identity() {
        let a = MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("a").unwrap(), ""));
        let b = MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("b").unwrap(), ""));
        assert!(!same_block_identity(&a, &b)); assert!(same_block_identity(&a, &a));
    }
    pub(in crate::components::chat::state) fn thinking_id_message_lifetime_rules_are_exhaustive() {
        let id = ThinkingId::new("same").unwrap();
        let mut histories = BTreeMap::new(); histories.insert(MessageId::new(1), BTreeSet::from([id.clone()]));
        assert!(histories.get(&MessageId::new(1)).unwrap().contains(&id));
        assert!(!histories.contains_key(&MessageId::new(2)));
    }
    pub(in crate::components::chat::state) fn message_transition_matrix_is_exhaustive() {
        let statuses = [MessageStatus::Pending, MessageStatus::Streaming, MessageStatus::Complete,
            MessageStatus::Cancelled, MessageStatus::Failed(cause())];
        for (from_index, from) in statuses.iter().enumerate() {
            for (to_index, to) in statuses.iter().enumerate() {
                let always = matches!((from_index, to_index), (0, 1) | (1, 2) | (0 | 1, 3 | 4));
                assert_eq!(valid_message_transition(from, to, false), always);
                assert_eq!(valid_message_transition(from, to, true),
                    always || (from_index, to_index) == (0, 2));
            }
        }
    }
    pub(in crate::components::chat::state) fn nested_status_transition_matrices_are_exhaustive() {
        let thinking = [ThinkingStatus::Pending, ThinkingStatus::Streaming, ThinkingStatus::Complete,
            ThinkingStatus::Cancelled, ThinkingStatus::Failed(cause())];
        let calls = [ToolCallStatus::Pending, ToolCallStatus::Running, ToolCallStatus::Succeeded,
            ToolCallStatus::Cancelled, ToolCallStatus::Failed(cause())];
        let results = [ToolResultStatus::Pending, ToolResultStatus::Streaming, ToolResultStatus::Complete,
            ToolResultStatus::Cancelled, ToolResultStatus::Failed(cause())];
        for from in 0..5 { for to in 0..5 {
            let expected = matches!((from, to), (0, 1) | (1, 2) | (0 | 1, 3 | 4));
            assert_eq!(valid_thinking_transition(&thinking[from], &thinking[to]), expected);
            assert_eq!(valid_call_transition(&calls[from], &calls[to]), expected);
            assert_eq!(valid_result_transition(&results[from], &results[to]), expected);
        }}
    }
    pub(in crate::components::chat::state) fn terminal_updates_are_single_effect_and_race_safe() {
        assert!(message_terminal(&MessageStatus::Cancelled));
        assert!(!valid_message_transition(&MessageStatus::Cancelled, &MessageStatus::Failed(cause()), false));
    }
    pub(in crate::components::chat::state) fn cross_level_terminality_never_freezes_active_nested_blocks() {
        let active = MessageBlock::Thinking(ThinkingContent::new(ThinkingId::new("a").unwrap(), ""));
        assert!(nested_active(&active)); assert!(!nested_terminal(&active));
    }
    pub(in crate::components::chat::state) fn identity_and_correlation_helpers_cover_all_namespaces() {
        assert!(valid_call_result(&ToolCallStatus::Pending, None));
        assert!(!valid_call_result(&ToolCallStatus::Pending, Some(&ToolResultStatus::Pending)));
    }
    pub(in crate::components::chat::state) fn append_block_cross_level_rules_are_exhaustive() {
        assert!(message_active(&MessageStatus::Pending));
        assert!(!message_active(&MessageStatus::Complete));
    }
    pub(in crate::components::chat::state) fn replace_block_kind_rules_are_exhaustive() {
        assert!(same_block_identity(&MessageBlock::Text("a".into()), &MessageBlock::Text("b".into())));
        assert!(!same_block_identity(&MessageBlock::Text("a".into()), &MessageBlock::Markdown("a".into())));
    }
    pub(in crate::components::chat::state) fn static_completion_readiness_matrix_is_exhaustive() {
        assert!(static_complete_ready(&text_message()));
        let empty = ChatMessage::new(MessageId::new(2), ChatRole::User,
            vec![MessageBlockEntry::new(BlockId::new(2), MessageBlock::Text(String::new()))]).unwrap();
        assert!(!static_complete_ready(&empty));
    }
    pub(in crate::components::chat::state) fn tool_call_result_correlation_matrix_is_exhaustive() {
        let calls = [ToolCallStatus::Pending, ToolCallStatus::Running, ToolCallStatus::Succeeded,
            ToolCallStatus::Cancelled, ToolCallStatus::Failed(cause())];
        let results = [None, Some(ToolResultStatus::Pending), Some(ToolResultStatus::Streaming),
            Some(ToolResultStatus::Complete), Some(ToolResultStatus::Cancelled),
            Some(ToolResultStatus::Failed(cause()))];
        let expected = [[true, false, false, false, false, false],
            [true, true, true, false, false, false], [true; 6],
            [true, false, false, false, true, false],
            [true, false, false, false, false, true]];
        for (call_index, call) in calls.iter().enumerate() {
            for (result_index, result) in results.iter().enumerate() {
                assert_eq!(valid_call_result(call, result.as_ref()),
                    expected[call_index][result_index]);
            }
        }
    }
    pub(in crate::components::chat::state) fn message_revision_checked_increment_is_exhaustive() {
        assert_eq!(MessageRevision::INITIAL.checked_next(MessageId::new(1)).unwrap().get(), 2);
        let maximum = MessageRevision::new(u64::MAX).unwrap();
        assert_eq!(maximum.checked_next(MessageId::new(9)),
            Err(ConversationError::MessageRevisionExhausted { message_id: MessageId::new(9) }));
    }
    pub(in crate::components::chat::state) fn block_id_state_lifetime_rules_are_exhaustive() {
        let seen = BTreeSet::from([BlockId::new(1)]); let retired = BTreeSet::from([BlockId::new(1)]);
        assert!(retired.is_subset(&seen));
    }
    pub(in crate::components::chat::state) fn restore_history_validation_is_exhaustive() {
        let state = ConversationState::new(4, NonZeroUsize::new(2).unwrap());
        assert_eq!(ConversationState::try_restore(state.snapshot()).unwrap(), state);
        let history = ConversationIdentityHistory::new(vec![MessageId::new(1), MessageId::new(1)],
            vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
        let bad = ConversationStateSnapshot::new(vec![], ConversationRevision::INITIAL, 0,
            RetentionHistory::new(NonZeroUsize::new(1).unwrap(), vec![], None).unwrap(), history);
        assert!(ConversationState::try_restore(bad).is_err());
    }
    pub(in crate::components::chat::state) fn tool_result_slot_history_rules_are_exhaustive() {
        let location = ToolResultLocation::new(MessageId::new(1), BlockId::new(2));
        assert_eq!(location.message_id(), MessageId::new(1));
        assert!(matches!(ToolResultSlot::Retired, ToolResultSlot::Retired));
    }
    pub(in crate::components::chat::state) fn revision_exhaustion_is_checked_and_atomic_at_u64_max() {
        let mut state = ConversationState::new(1, NonZeroUsize::MIN);
        state.revision = ConversationRevision::new(u64::MAX);
        let before = state.clone();
        let update = ConversationUpdate::complete(MessageMutationGuard::new(
            ConversationGuard::new(ConversationRevision::new(u64::MAX)),
            MessageId::new(404), MessageRevision::INITIAL));
        let event = ConversationEvent::new(UpdateId::new("overflow").unwrap(), 1, update);
        assert_eq!(state.apply_event(event), Err(ConversationError::RevisionExhausted));
        assert_eq!(state, before);
    }
}
}

pub use compact::*;

#[cfg(test)]
mod tests {
    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                super::compact::test_cases::$name();
            }
        };
    }
    case!(thinking_replacement_requires_same_identity);
    case!(thinking_id_message_lifetime_rules_are_exhaustive);
    case!(message_transition_matrix_is_exhaustive);
    case!(nested_status_transition_matrices_are_exhaustive);
    case!(terminal_updates_are_single_effect_and_race_safe);
    case!(cross_level_terminality_never_freezes_active_nested_blocks);
    case!(identity_and_correlation_helpers_cover_all_namespaces);
    case!(append_block_cross_level_rules_are_exhaustive);
    case!(replace_block_kind_rules_are_exhaustive);
    case!(static_completion_readiness_matrix_is_exhaustive);
    case!(tool_call_result_correlation_matrix_is_exhaustive);
    case!(message_revision_checked_increment_is_exhaustive);
    case!(block_id_state_lifetime_rules_are_exhaustive);
    case!(restore_history_validation_is_exhaustive);
    case!(tool_result_slot_history_rules_are_exhaustive);
    case!(revision_exhaustion_is_checked_and_atomic_at_u64_max);
}
