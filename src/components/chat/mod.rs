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
