//! Provider-independent typed conversation data.
//!
//! This module contains data only. It does not parse provider payloads, perform
//! network requests, execute tools, or write to a terminal.
//!
//! ```rust
//! use rnk::components::chat::{
//!     BlockId, ChatMessage, ChatRole, ConversationEvent, ConversationGuard,
//!     ConversationState, ConversationUpdate, MessageBlock, MessageBlockEntry, MessageId,
//!     MessageMutationGuard, UpdateId,
//! };
//! use std::num::NonZeroUsize;
//!
//! let mut state = ConversationState::new(0, NonZeroUsize::new(8).unwrap());
//! let message = ChatMessage::new(
//!     MessageId::new(1),
//!     ChatRole::Assistant,
//!     vec![MessageBlockEntry::new(BlockId::new(1), MessageBlock::Text(String::new()))],
//! )?;
//! let push = ConversationUpdate::push(ConversationGuard::new(state.revision()), message);
//! state.apply_event(ConversationEvent::new(UpdateId::new("push")?, 0, push))?;
//!
//! let message = state.message(MessageId::new(1)).unwrap();
//! let guard = MessageMutationGuard::new(
//!     ConversationGuard::new(state.revision()),
//!     message.id(),
//!     message.revision(),
//! );
//! let append = ConversationUpdate::append_text(guard, BlockId::new(1), "hello")?;
//! state.apply_event(ConversationEvent::new(UpdateId::new("append")?, 1, append))?;
//!
//! let message = state.message(MessageId::new(1)).unwrap();
//! let guard = MessageMutationGuard::new(
//!     ConversationGuard::new(state.revision()),
//!     message.id(),
//!     message.revision(),
//! );
//! let complete = ConversationUpdate::complete(guard);
//! state.apply_event(ConversationEvent::new(UpdateId::new("complete")?, 2, complete))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(missing_docs)]

pub mod composer;
mod error;
mod model;
mod reducer;
mod state;
pub mod view;

pub use composer::{
    ChatComposerKeyMap, ChatComposerState, ComposerAction, ComposerError, ComposerProjection,
    ComposerRevision, PendingSubmission, SubmissionToken, handle_key,
};
pub use view::{
    ChatBlockRef, ChatBlockRenderer, ChatMessageView, ChatMessageViewOptions, ChatMessageViewStyle,
    ChatRenderContext, ChatRenderOverride, MessageViewVariant, StreamingIndicatorFrame,
    ThinkingDisclosure,
};

/// Opaque evidence binding a retained event to its accepted history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionProof {
    pub(in crate::components::chat) previous: [u64; 4],
    pub(in crate::components::chat) record: [u64; 4],
}

/// Opaque evidence binding a snapshot's content to its retained history tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotProof {
    pub(in crate::components::chat) retention_tail: [u64; 4],
    pub(in crate::components::chat) content: [u64; 4],
}

impl RetentionProof {
    /// Returns opaque adapter-owned persistence parts without defining a wire encoding.
    pub const fn parts(&self) -> ([u64; 4], [u64; 4]) {
        (self.previous, self.record)
    }
}

impl SnapshotProof {
    /// Returns opaque adapter-owned persistence parts without defining a wire encoding.
    pub const fn parts(&self) -> ([u64; 4], [u64; 4]) {
        (self.retention_tail, self.content)
    }
}

#[rustfmt::skip]
mod rollback {
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct IdentityBackup {
    messages: Vec<(MessageId, bool, bool)>, blocks: Vec<(BlockId, bool, bool)>,
    thinking_maps: Vec<(MessageId, bool, bool)>,
    thinking_ids: Vec<(MessageId, ThinkingId, bool, bool)>,
    calls: Vec<(ToolCallId, bool, bool, Option<ToolResultSlot>)>,
}
impl IdentityBackup {
    pub(super) fn capture(state: &ConversationState, update: &ConversationUpdate,
        affected: &BTreeSet<MessageId>, mut message_visit: impl FnMut(),
        mut block_visit: impl FnMut()) -> Self {
        let mut message_ids = affected.clone(); let mut block_ids = BTreeSet::new();
        let mut thinking_ids = BTreeSet::new(); let mut call_ids = BTreeSet::new();
        let mut capture_entry = |message_id: MessageId, entry: &MessageBlockEntry| {
            block_visit();
            block_ids.insert(entry.id);
            match &entry.block {
                MessageBlock::Thinking(value) => {
                    thinking_ids.insert((message_id, value.id.clone()));
                }
                MessageBlock::ToolCall(value) => { call_ids.insert(value.call_id.clone()); }
                MessageBlock::ToolResult(value) => { call_ids.insert(value.call_id.clone()); }
                _ => {}
            }
        };
        for message in &state.messages {
            message_visit();
            if !affected.contains(&message.id) { continue; }
            message_ids.insert(message.id);
            for entry in &message.blocks { capture_entry(message.id, entry); }
        }
        match update {
            ConversationUpdate::Push(value) => { message_ids.insert(value.message.id);
                for entry in &value.message.blocks { capture_entry(value.message.id, entry); } }
            ConversationUpdate::Resend(value) => { message_ids.insert(value.message.id);
                for entry in &value.message.blocks { capture_entry(value.message.id, entry); } }
            ConversationUpdate::AppendMessageBlock(value) =>
                capture_entry(value.guard.message_id, &value.entry),
            ConversationUpdate::InsertMessageBlock(value) =>
                capture_entry(value.guard.message_id, &value.entry),
            ConversationUpdate::EditMessage(value) =>
                for entry in &value.entries { capture_entry(value.guard.message_id, entry); },
            _ => {}
        }
        let messages = message_ids.iter().map(|id| (*id, state.seen_messages.contains(id),
            state.retired_messages.contains(id))).collect();
        let blocks = block_ids.iter().map(|id| (*id, state.seen_blocks.contains(id),
            state.retired_blocks.contains(id))).collect();
        let thinking_maps = message_ids.into_iter().map(|id| (id,
            state.thinking_seen.contains_key(&id), state.thinking_retired.contains_key(&id))).collect();
        let thinking_ids = thinking_ids.into_iter().map(|(message_id, id)| {
            let seen = state.thinking_seen.get(&message_id).is_some_and(|set| set.contains(&id));
            let retired = state.thinking_retired.get(&message_id).is_some_and(|set| set.contains(&id));
            (message_id, id, seen, retired)
        }).collect();
        let calls = call_ids.into_iter().map(|id| { let seen = state.seen_tool_calls.contains(&id);
            let retired = state.retired_tool_calls.contains(&id);
            let slot = state.result_slots.get(&id).cloned(); (id, seen, retired, slot) }).collect();
        Self { messages, blocks, thinking_maps, thinking_ids, calls }
    }
    pub(super) fn restore(self, state: &mut ConversationState) {
        for (id, seen, retired) in self.messages {
            set_membership(&mut state.seen_messages, id, seen);
            set_membership(&mut state.retired_messages, id, retired);
        }
        for (id, seen, retired) in self.blocks {
            set_membership(&mut state.seen_blocks, id, seen);
            set_membership(&mut state.retired_blocks, id, retired);
        }
        for (message_id, id, seen, retired) in self.thinking_ids {
            set_nested_membership(&mut state.thinking_seen, message_id, id.clone(), seen);
            set_nested_membership(&mut state.thinking_retired, message_id, id, retired);
        }
        for (id, seen, retired) in self.thinking_maps {
            restore_map_presence(&mut state.thinking_seen, id, seen);
            restore_map_presence(&mut state.thinking_retired, id, retired);
        }
        for (id, seen, retired, slot) in self.calls {
            set_membership(&mut state.seen_tool_calls, id.clone(), seen);
            set_membership(&mut state.retired_tool_calls, id.clone(), retired);
            restore_map(&mut state.result_slots, id, slot);
        }
    }
}
fn set_membership<T: Ord>(set: &mut BTreeSet<T>, value: T, present: bool) {
    if present { set.insert(value); } else { set.remove(&value); }
}
fn set_nested_membership<K: Ord, V: Ord>(map: &mut BTreeMap<K, BTreeSet<V>>,
    key: K, value: V, present: bool) {
    if present { map.entry(key).or_default().insert(value); }
    else if let Some(values) = map.get_mut(&key) { values.remove(&value); }
}
fn restore_map_presence<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, present: bool)
    where V: Default {
    if present { map.entry(key).or_default(); } else { map.remove(&key); }
}
fn restore_map<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, value: Option<V>) {
    if let Some(value) = value { map.insert(key, value); } else { map.remove(&key); }
}
}

#[rustfmt::skip]
mod proof {
use super::*;
use std::{collections::{BTreeMap, BTreeSet}, fmt::Debug};

fn fingerprint(value: &impl Debug) -> [u64; 4] {
    let bytes = format!("{value:?}"); let mut found = [
        0xcbf29ce484222325_u64, 0x84222325cbf29ce4, 0x9e3779b97f4a7c15, 0x517cc1b727220a95];
    for byte in bytes.bytes() {
        for (index, hash) in found.iter_mut().enumerate() {
            *hash ^= u64::from(byte).wrapping_add(index as u64 * 0x9d);
            *hash = hash.wrapping_mul(0x100000001b3).rotate_left((index * 11 + 5) as u32);
        }
    }
    found
}
pub(super) fn record_fingerprint(previous: [u64; 4], event: &ConversationEvent,
    outcome: &ApplyOutcome) -> [u64; 4] {
    fingerprint(&(previous, event, outcome))
}
fn snapshot_state_fingerprint(retention_tail: [u64; 4],
    value: &ConversationStateSnapshot) -> [u64; 4] {
    let identities = &value.identities;
    let seen_messages = identities.seen_messages().iter().copied().collect::<BTreeSet<_>>();
    let retired_messages = identities.retired_messages().iter().copied().collect::<BTreeSet<_>>();
    let seen_blocks = identities.seen_blocks().iter().copied().collect::<BTreeSet<_>>();
    let retired_blocks = identities.retired_blocks().iter().copied().collect::<BTreeSet<_>>();
    let thinking_seen = identities.thinking().iter().map(|history| (history.message_id(),
        history.seen().iter().cloned().collect::<BTreeSet<_>>())).collect::<BTreeMap<_, _>>();
    let thinking_retired = identities.thinking().iter().map(|history| (history.message_id(),
        history.retired().iter().cloned().collect::<BTreeSet<_>>())).collect::<BTreeMap<_, _>>();
    let seen_calls = identities.seen_tool_calls().iter().cloned().collect::<BTreeSet<_>>();
    let retired_calls = identities.retired_tool_calls().iter().cloned().collect::<BTreeSet<_>>();
    let result_slots = identities.result_slots().iter().cloned().collect::<BTreeMap<_, _>>();
    fingerprint(&(retention_tail, (&value.messages, value.revision, value.expected_sequence,
        value.retention.capacity, value.retention.evicted_through), (seen_messages,
        retired_messages, seen_blocks, retired_blocks, thinking_seen, thinking_retired,
        seen_calls, retired_calls, result_slots)))
}
pub(super) fn snapshot_proof(value: &ConversationStateSnapshot) -> SnapshotProof {
    let retention_tail = value.retention.records.last().and_then(|record| record.proof.as_ref())
        .map_or([0; 4], |proof| proof.record);
    SnapshotProof { retention_tail, content: snapshot_state_fingerprint(retention_tail, value) }
}
pub(super) fn evicted_proofs_are_valid(value: &ConversationStateSnapshot) -> bool {
    let mut prior = None;
    for record in &value.retention.records {
        let Some(proof) = &record.proof else { return false; };
        if proof.record != record_fingerprint(proof.previous, &record.event,
            &record.outcome)
            || prior.is_some_and(|previous| proof.previous != previous) { return false; }
        prior = Some(proof.record);
    }
    value.proof.as_ref().is_some_and(|proof| proof.retention_tail == prior.unwrap_or([0; 4])
        && proof.content == snapshot_state_fingerprint(proof.retention_tail, value))
}
}

pub use error::ConversationError;
pub use model::{
    AffectedMessage, AffectedMessageDisposition, AppendTextUpdate, ApplyOutcome, BlockId,
    BlockUpdate, ChatMessage, ChatMessageMetadata, ChatRole, CodeContent, ConversationEvent,
    ConversationGuard, ConversationRevision, ConversationUpdate, DecimalValue, DiffContent,
    EditMessageUpdate, ErrorContent, ErrorSource, FailUpdate, FailureCause, GuardedUpdate,
    InsertBlockUpdate, LegacyRoleConversionError, LinkContent, MessageAuthor, MessageBlock,
    MessageBlockEntry, MessageId, MessageMutationGuard, MessageRevision, MessageStatus,
    MessageTimestamp, PushUpdate, QuoteContent, ReplaceBlockUpdate, ResendUpdate,
    TerminalAttachmentSummary, ThinkingContent, ThinkingId, ThinkingStatus, ToolArgument,
    ToolCallContent, ToolCallId, ToolCallStatus, ToolResultContent, ToolResultStatus, TypedField,
    TypedValue, UpdateId,
};
pub use state::{
    ConversationIdentityHistory, ConversationState, ConversationStateSnapshot,
    ProcessedEventRecord, RetentionHistory, ThinkingIdentityHistory, ToolResultLocation,
    ToolResultSlot,
};

impl AffectedMessage {
    /// Reconstructs one persisted affected-message revision after local validation.
    pub fn try_restore(
        message_id: MessageId,
        previous: Option<MessageRevision>,
        applied: MessageRevision,
        disposition: AffectedMessageDisposition,
    ) -> Result<Self, ConversationError> {
        let expected = match previous {
            Some(value) => value.checked_next(message_id)?,
            None => MessageRevision::INITIAL,
        };
        if applied != expected
            || previous.is_none() && disposition == AffectedMessageDisposition::Deleted
        {
            return Err(ConversationError::InvalidSnapshot {
                reason: "affected message revisions are contradictory",
            });
        }
        Ok(Self {
            message_id,
            previous,
            applied,
            disposition,
        })
    }
}

impl ApplyOutcome {
    /// Reconstructs one persisted successful outcome after local validation.
    pub fn try_restore(
        revision: ConversationRevision,
        affected_messages: Vec<AffectedMessage>,
    ) -> Result<Self, ConversationError> {
        let unique = affected_messages
            .iter()
            .map(AffectedMessage::message_id)
            .collect::<std::collections::BTreeSet<_>>();
        if revision == ConversationRevision::INITIAL
            || affected_messages.is_empty()
            || unique.len() != affected_messages.len()
        {
            return Err(ConversationError::InvalidSnapshot {
                reason: "apply outcome is contradictory",
            });
        }
        Ok(Self {
            revision,
            affected_messages,
        })
    }
}

impl ProcessedEventRecord {
    /// Reconstructs a proven record after authenticating its opaque proof parts.
    pub fn try_new_with_proof_parts(
        event: ConversationEvent,
        outcome: ApplyOutcome,
        previous: [u64; 4],
        record: [u64; 4],
    ) -> Result<Self, ConversationError> {
        if proof::record_fingerprint(previous, &event, &outcome) != record {
            return Err(ConversationError::InvalidSnapshot {
                reason: "retained record proof is contradictory",
            });
        }
        Ok(Self::new_with_proof(
            event,
            outcome,
            RetentionProof { previous, record },
        ))
    }
}

impl ConversationStateSnapshot {
    /// Reconstructs a proven snapshot after authenticating its opaque proof parts.
    pub fn try_new_with_proof_parts(
        messages: Vec<ChatMessage>,
        revision: ConversationRevision,
        expected_sequence: u64,
        retention: RetentionHistory,
        identities: ConversationIdentityHistory,
        retention_tail: [u64; 4],
        content: [u64; 4],
    ) -> Result<Self, ConversationError> {
        let supplied = SnapshotProof {
            retention_tail,
            content,
        };
        let value = Self::new_with_proof(
            messages,
            revision,
            expected_sequence,
            retention,
            identities,
            supplied.clone(),
        );
        if proof::snapshot_proof(&value) != supplied {
            return Err(ConversationError::InvalidSnapshot {
                reason: "snapshot proof is contradictory",
            });
        }
        Ok(value)
    }
}

macro_rules! failure_cause_accessor {
    ($ty:ty) => {
        impl $ty {
            /// Returns the typed cause when this status is failed.
            pub fn failure_cause(&self) -> Option<&FailureCause> {
                match self {
                    Self::Failed(cause) => Some(cause),
                    _ => None,
                }
            }
        }
    };
}
failure_cause_accessor!(MessageStatus);
failure_cause_accessor!(ThinkingStatus);
failure_cause_accessor!(ToolCallStatus);
failure_cause_accessor!(ToolResultStatus);
