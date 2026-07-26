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

mod error;
mod model;
mod reducer;
mod state;

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
        affected: &BTreeSet<MessageId>) -> Self {
        let mut message_ids = affected.clone(); let mut block_ids = BTreeSet::new();
        let mut thinking_ids = BTreeSet::new(); let mut call_ids = BTreeSet::new();
        let mut capture_entry = |message_id: MessageId, entry: &MessageBlockEntry| {
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
        for message in state.messages.iter().filter(|message| affected.contains(&message.id)) {
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
