//! Provider-independent typed conversation data.
//!
//! This module contains data only. It does not parse provider payloads, perform
//! network requests, execute tools, or write to a terminal.

#![forbid(missing_docs)]

mod error;
mod model;
mod reducer;
mod state;

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
