//! Typed failures for conversation model validation and reduction.

use super::{BlockId, ConversationRevision, MessageId, MessageRevision, UpdateId};
use std::error::Error;
use std::fmt;

/// A typed failure produced while constructing or reducing conversation data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConversationError {
    /// A public value failed validation.
    InvalidValue {
        /// Name of the invalid field.
        field: &'static str,
        /// Stable explanation of the failed rule.
        reason: &'static str,
    },
    /// A message is structurally invalid.
    InvalidMessage {
        /// Message whose structure is invalid.
        message_id: MessageId,
        /// Stable explanation of the failed rule.
        reason: &'static str,
    },
    /// A message identity was already used.
    DuplicateMessageId {
        /// Duplicated message identity.
        message_id: MessageId,
    },
    /// A target message does not exist.
    UnknownMessage {
        /// Missing or retired message identity.
        message_id: MessageId,
    },
    /// A target block does not exist in the selected message.
    UnknownBlock {
        /// Target message identity.
        message_id: MessageId,
        /// Missing block identity.
        block_id: BlockId,
    },
    /// A block identity is duplicated.
    DuplicateBlockId {
        /// Duplicated block identity.
        block_id: BlockId,
    },
    /// A retired block identity was reused.
    RetiredBlockId {
        /// Retired block identity.
        block_id: BlockId,
    },
    /// A thinking identity is duplicated within one message lifetime.
    DuplicateThinkingId {
        /// Message containing the duplicate.
        message_id: MessageId,
        /// Duplicated thinking identity.
        thinking_id: String,
    },
    /// A retired thinking identity was reused in the same message lifetime.
    RetiredThinkingId {
        /// Message containing the retired identity.
        message_id: MessageId,
        /// Retired thinking identity.
        thinking_id: String,
    },
    /// A tool-call identity is duplicated in the conversation.
    DuplicateToolCallId {
        /// Duplicated tool-call identity.
        call_id: String,
    },
    /// A tool result does not have one live call.
    OrphanToolResult {
        /// Unmatched tool-call identity.
        call_id: String,
    },
    /// A tool result slot was permanently retired.
    ResultSlotRetired {
        /// Tool-call identity owning the slot.
        call_id: String,
    },
    /// A lifecycle transition is not allowed.
    InvalidTransition {
        /// Stable state-machine category.
        kind: &'static str,
        /// Stable transition explanation.
        reason: &'static str,
    },
    /// A replacement changed a stable kind or lifecycle identity.
    InvalidReplacement {
        /// Entry being replaced.
        block_id: BlockId,
        /// Stable replacement explanation.
        reason: &'static str,
    },
    /// Tool-call and tool-result states are inconsistent.
    InvalidCorrelation {
        /// Tool-call identity.
        call_id: String,
        /// Stable matrix explanation.
        reason: &'static str,
    },
    /// A retained event ID was reused with different content.
    EventIdConflict {
        /// Conflicting event identity.
        event_id: UpdateId,
    },
    /// A new event sequence is older than the expected sequence.
    StaleSequence {
        /// Expected next sequence.
        expected: u64,
        /// Received sequence.
        actual: u64,
    },
    /// A new event sequence skipped one or more values.
    SequenceGap {
        /// Expected next sequence.
        expected: u64,
        /// Received sequence.
        actual: u64,
    },
    /// An old event fell beyond the retained replay proof.
    ReplayOutsideRetention {
        /// Old sequence that can no longer be proven.
        sequence: u64,
        /// Highest sequence known to have been evicted.
        evicted_through: u64,
    },
    /// The expected sequence cannot be advanced.
    SequenceExhausted,
    /// The conversation revision cannot be advanced.
    RevisionExhausted,
    /// A message revision cannot be advanced.
    MessageRevisionExhausted {
        /// Message whose revision is exhausted.
        message_id: MessageId,
    },
    /// The caller supplied a stale conversation revision.
    ConversationRevisionMismatch {
        /// Current conversation revision.
        expected: ConversationRevision,
        /// Caller-provided conversation revision.
        actual: ConversationRevision,
    },
    /// The caller supplied a stale message revision.
    MessageRevisionMismatch {
        /// Target message.
        message_id: MessageId,
        /// Current message revision.
        expected: MessageRevision,
        /// Caller-provided message revision.
        actual: MessageRevision,
    },
    /// A snapshot or history record is incomplete or contradictory.
    InvalidSnapshot {
        /// Stable snapshot validation explanation.
        reason: &'static str,
    },
    /// Resend requires a terminal source message.
    ResendRequiresTerminal {
        /// Non-terminal source identity.
        message_id: MessageId,
    },
}

impl fmt::Display for ConversationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for ConversationError {}
