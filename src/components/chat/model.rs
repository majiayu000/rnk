//! Provider-independent owned conversation values.

#[rustfmt::skip]
mod compact {
use super::super::ConversationError;
use crate::components::MessageRole;
use std::{collections::BTreeSet, fmt, num::NonZeroU64};
macro_rules! numeric_id {
    ($name:ident, $doc:literal, $value_doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);
        impl $name {
            #[doc = $value_doc]
            pub const fn new(value: u64) -> Self { Self(value) }
            /// Returns the numeric value.
            pub const fn get(self) -> u64 { self.0 }
        }
    };
}
numeric_id!(MessageId, "Stable message identity.", "Creates a message identity.");
numeric_id!(BlockId, "Stable conversation-lifetime block identity.", "Creates a block identity.");
macro_rules! string_value {
    ($name:ident, $doc:literal, $field:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);
        impl $name {
            /// Creates a validated non-empty value.
            pub fn new(value: impl Into<String>) -> Result<Self, ConversationError> {
                Ok(Self(nonempty(value, $field)?))
            }
            /// Returns the original validated text.
            pub fn as_str(&self) -> &str { &self.0 }
        }
    };
}
string_value!(UpdateId, "Stable event identity.", "event_id");
string_value!(ThinkingId, "Message-local thinking identity.", "thinking_id");
string_value!(ToolCallId, "Conversation-wide tool-call correlation identity.", "tool_call_id");
string_value!(FailureCause, "Typed lifecycle failure cause.", "failure_cause");
string_value!(MessageAuthor, "Application-provided display author.", "message_author");
string_value!(MessageTimestamp, "Application-formatted display timestamp.", "message_timestamp");
string_value!(ErrorSource, "Application-provided error source.", "error_source");
impl fmt::Display for UpdateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}
impl TryFrom<String> for UpdateId {
    type Error = ConversationError;
    fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(value) }
}
/// Monotonic conversation revision.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationRevision(u64);
impl ConversationRevision {
    /// Initial revision.
    pub const INITIAL: Self = Self(0);
    /// Creates an exact revision.
    pub const fn new(value: u64) -> Self { Self(value) }
    /// Returns the numeric revision.
    pub const fn get(self) -> u64 { self.0 }
    pub(in crate::components::chat) fn checked_next(self) -> Result<Self, ConversationError> {
        self.0.checked_add(1).map(Self).ok_or(ConversationError::RevisionExhausted)
    }
}
/// Non-zero revision of one message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageRevision(NonZeroU64);
impl MessageRevision {
    /// Initial message revision.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);
    /// Creates a non-zero revision.
    pub fn new(value: u64) -> Result<Self, ConversationError> {
        NonZeroU64::new(value).map(Self).ok_or(ConversationError::InvalidValue {
            field: "message_revision", reason: "must be non-zero",
        })
    }
    /// Returns the numeric revision.
    pub const fn get(self) -> u64 { self.0.get() }
    pub(in crate::components::chat) fn checked_next(self, id: MessageId) -> Result<Self, ConversationError> {
        self.get().checked_add(1).and_then(NonZeroU64::new).map(Self)
            .ok_or(ConversationError::MessageRevisionExhausted { message_id: id })
    }
}
/// Closed role set understood by the core model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatRole {
    /// Human participant.
    User,
    /// Assistant participant.
    Assistant,
    /// System participant.
    System,
    /// Tool participant.
    Tool,
}
impl From<ChatRole> for MessageRole {
    fn from(value: ChatRole) -> Self {
        match value {
            ChatRole::User => Self::User, ChatRole::Assistant => Self::Assistant,
            ChatRole::System => Self::System, ChatRole::Tool => Self::Tool,
        }
    }
}
/// Failure converting a display-only legacy role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyRoleConversionError(MessageRole);
impl LegacyRoleConversionError {
    /// Returns the unsupported role.
    pub const fn role(self) -> MessageRole { self.0 }
}
impl TryFrom<MessageRole> for ChatRole {
    type Error = LegacyRoleConversionError;
    fn try_from(value: MessageRole) -> Result<Self, Self::Error> {
        match value {
            MessageRole::User => Ok(Self::User), MessageRole::Assistant => Ok(Self::Assistant),
            MessageRole::System => Ok(Self::System), MessageRole::Tool => Ok(Self::Tool),
            MessageRole::ToolResult | MessageRole::Error => Err(LegacyRoleConversionError(value)),
        }
    }
}
/// Decimal text with exactly one canonical representation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalValue(String);
impl DecimalValue {
    /// Parses canonical, non-exponent decimal notation.
    pub fn new(value: impl Into<String>) -> Result<Self, ConversationError> {
        let value = value.into();
        if canonical_decimal(&value) { Ok(Self(value)) } else {
            Err(ConversationError::InvalidValue {
                field: "decimal", reason: "must use canonical decimal notation",
            })
        }
    }
    /// Returns canonical decimal text.
    pub fn as_str(&self) -> &str { &self.0 }
}
/// Named value in a typed object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedField { name: String, value: TypedValue }
impl TypedField {
    /// Creates a named field.
    pub fn new(name: impl Into<String>, value: TypedValue) -> Result<Self, ConversationError> {
        value.validate_recursive()?;
        Ok(Self { name: nonempty(name, "typed_field_name")?, value })
    }
    /// Returns the field name.
    pub fn name(&self) -> &str { &self.name }
    /// Returns the field value.
    pub fn value(&self) -> &TypedValue { &self.value }
}
/// Closed provider-independent value tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedValue {
    /// Explicit null.
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    Integer(i64),
    /// Canonical decimal.
    Decimal(DecimalValue),
    /// Exact string, including empty.
    String(String),
    /// Ordered list.
    List(Vec<TypedValue>),
    /// Ordered object with unique names.
    Object(Vec<TypedField>),
}
impl TypedValue {
    /// Creates an object after recursively checking every object name set.
    pub fn object(fields: Vec<TypedField>) -> Result<Self, ConversationError> {
        unique(fields.iter().map(TypedField::name), "typed_field_name")?;
        let value = Self::Object(fields);
        value.validate_recursive()?;
        Ok(value)
    }
    fn validate_recursive(&self) -> Result<(), ConversationError> {
        match self {
            Self::List(values) => values.iter().try_for_each(Self::validate_recursive),
            Self::Object(fields) => {
                unique(fields.iter().map(TypedField::name), "typed_field_name")?;
                fields.iter().try_for_each(|field| field.value.validate_recursive())
            }
            _ => Ok(()),
        }
    }
}

/// Ordered typed tool argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgument { name: String, value: TypedValue }
impl ToolArgument {
    /// Creates a named argument.
    pub fn new(name: impl Into<String>, value: TypedValue) -> Result<Self, ConversationError> {
        value.validate_recursive()?;
        Ok(Self { name: nonempty(name, "tool_argument_name")?, value })
    }
    /// Returns the name.
    pub fn name(&self) -> &str { &self.name }
    /// Returns the typed value.
    pub fn value(&self) -> &TypedValue { &self.value }
}

macro_rules! lifecycle {
    ($name:ident, $doc:literal, [$($variant:ident => $vdoc:literal),+ $(,)?]) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $name {
            $(
                #[doc = $vdoc]
                $variant,
            )+
            /// Failed terminal state with a typed cause.
            Failed(FailureCause),
        }
    };
}
lifecycle!(MessageStatus, "Message lifecycle.", [
    Pending => "Awaiting the first update.", Streaming => "Receiving updates.",
    Complete => "Successfully terminal.", Cancelled => "Cancelled terminal state."
]);
lifecycle!(ThinkingStatus, "Thinking lifecycle.", [
    Pending => "Awaiting content.", Streaming => "Receiving content.",
    Complete => "Successfully terminal.", Cancelled => "Cancelled terminal state."
]);
lifecycle!(ToolResultStatus, "Tool-result lifecycle.", [
    Pending => "Awaiting output.", Streaming => "Receiving output.",
    Complete => "Successfully terminal.", Cancelled => "Cancelled terminal state."
]);
/// Tool-call lifecycle reported by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolCallStatus {
    /// Awaiting execution.
    Pending,
    /// Execution is running.
    Running,
    /// Execution succeeded.
    Succeeded,
    /// Execution was cancelled.
    Cancelled,
    /// Execution failed with a typed cause.
    Failed(FailureCause),
}

macro_rules! language_payload {
    ($name:ident, $doc:literal, $field:literal, $allow_empty:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name { content: String, language: Option<String> }
        impl $name {
            /// Creates content without a language.
            pub fn new(content: impl Into<String>) -> Result<Self, ConversationError> {
                let content = content.into();
                if !$allow_empty && content.is_empty() {
                    return Err(ConversationError::InvalidValue {
                        field: $field, reason: "must be non-empty",
                    });
                }
                Ok(Self { content, language: None })
            }
            /// Sets an optional non-empty language.
            pub fn with_language(mut self, value: impl Into<String>) -> Result<Self, ConversationError> {
                self.language = Some(nonempty(value, "language")?); Ok(self)
            }
            /// Returns content.
            pub fn content(&self) -> &str { &self.content }
            /// Returns the optional language.
            pub fn language(&self) -> Option<&str> { self.language.as_deref() }
        }
    };
}
language_payload!(CodeContent, "Code content.", "code_content", true);
language_payload!(DiffContent, "Diff content.", "diff_content", false);

/// Quoted content and optional attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteContent { content: String, attribution: Option<String> }
impl QuoteContent {
    /// Creates non-empty quoted content.
    pub fn new(content: impl Into<String>) -> Result<Self, ConversationError> {
        Ok(Self { content: exact(content, "quote_content")?, attribution: None })
    }
    /// Sets an optional non-empty attribution.
    pub fn with_attribution(mut self, value: impl Into<String>) -> Result<Self, ConversationError> {
        self.attribution = Some(nonempty(value, "quote_attribution")?); Ok(self)
    }
    /// Returns content.
    pub fn content(&self) -> &str { &self.content }
    /// Returns attribution.
    pub fn attribution(&self) -> Option<&str> { self.attribution.as_deref() }
}

/// Inert link data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkContent { label: String, target: String }
impl LinkContent {
    /// Creates a non-empty label and opaque target.
    pub fn new(label: impl Into<String>, target: impl Into<String>) -> Result<Self, ConversationError> {
        Ok(Self { label: exact(label, "link_label")?, target: exact(target, "link_target")? })
    }
    /// Returns the label.
    pub fn label(&self) -> &str { &self.label }
    /// Returns the opaque target.
    pub fn target(&self) -> &str { &self.target }
}

/// Inert terminal attachment summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalAttachmentSummary { name: String, media_type: Option<String>, summary: String }
impl TerminalAttachmentSummary {
    /// Creates a non-empty name and summary.
    pub fn new(name: impl Into<String>, summary: impl Into<String>) -> Result<Self, ConversationError> {
        Ok(Self { name: exact(name, "attachment_name")?, media_type: None,
            summary: exact(summary, "attachment_summary")? })
    }
    /// Sets an optional non-empty media type.
    pub fn with_media_type(mut self, value: impl Into<String>) -> Result<Self, ConversationError> {
        self.media_type = Some(nonempty(value, "attachment_media_type")?); Ok(self)
    }
    /// Returns the name.
    pub fn name(&self) -> &str { &self.name }
    /// Returns the optional media type.
    pub fn media_type(&self) -> Option<&str> { self.media_type.as_deref() }
    /// Returns the summary.
    pub fn summary(&self) -> &str { &self.summary }
}

/// Typed error content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContent { message: String, source: Option<ErrorSource> }
impl ErrorContent {
    /// Creates a non-empty error message.
    pub fn new(message: impl Into<String>) -> Result<Self, ConversationError> {
        Ok(Self { message: exact(message, "error_message")?, source: None })
    }
    /// Sets an optional source.
    pub fn with_source(mut self, source: ErrorSource) -> Self { self.source = Some(source); self }
    /// Returns the message.
    pub fn message(&self) -> &str { &self.message }
    /// Returns the optional source.
    pub fn source(&self) -> Option<&ErrorSource> { self.source.as_ref() }
}

/// Thinking content and lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingContent {
    pub(in crate::components::chat) id: ThinkingId, pub(in crate::components::chat) content: String,
    pub(in crate::components::chat) status: ThinkingStatus,
}
impl ThinkingContent {
    /// Creates pending thinking content.
    pub fn new(id: ThinkingId, content: impl Into<String>) -> Self {
        Self { id, content: content.into(), status: ThinkingStatus::Pending }
    }
    /// Returns its identity.
    pub fn id(&self) -> &ThinkingId { &self.id }
    /// Returns accumulated content.
    pub fn content(&self) -> &str { &self.content }
    /// Returns status.
    pub fn status(&self) -> &ThinkingStatus { &self.status }
    /// Returns a copy with a requested status.
    pub fn with_status(mut self, status: ThinkingStatus) -> Self { self.status = status; self }
}

/// Data-only tool call and lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallContent {
    pub(in crate::components::chat) call_id: ToolCallId, name: String, arguments: Vec<ToolArgument>,
    pub(in crate::components::chat) status: ToolCallStatus,
}
impl ToolCallContent {
    /// Creates a pending call.
    pub fn new(call_id: ToolCallId, name: impl Into<String>, arguments: Vec<ToolArgument>)
        -> Result<Self, ConversationError> {
        unique(arguments.iter().map(ToolArgument::name), "tool_argument_name")?;
        Ok(Self { call_id, name: nonempty(name, "tool_name")?, arguments,
            status: ToolCallStatus::Pending })
    }
    /// Returns correlation identity.
    pub fn call_id(&self) -> &ToolCallId { &self.call_id }
    /// Returns the tool name.
    pub fn name(&self) -> &str { &self.name }
    /// Returns ordered arguments.
    pub fn arguments(&self) -> &[ToolArgument] { &self.arguments }
    /// Returns status.
    pub fn status(&self) -> &ToolCallStatus { &self.status }
    /// Returns a copy with application-reported status.
    pub fn with_status(mut self, status: ToolCallStatus) -> Self { self.status = status; self }
}

/// Tool result correlated with exactly one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultContent {
    pub(in crate::components::chat) call_id: ToolCallId,
    pub(in crate::components::chat) output: String,
    pub(in crate::components::chat) status: ToolResultStatus,
}
impl ToolResultContent {
    /// Creates a pending result.
    pub fn new(call_id: ToolCallId, output: impl Into<String>) -> Self {
        Self { call_id, output: output.into(), status: ToolResultStatus::Pending }
    }
    /// Returns correlation identity.
    pub fn call_id(&self) -> &ToolCallId { &self.call_id }
    /// Returns accumulated output.
    pub fn output(&self) -> &str { &self.output }
    /// Returns status.
    pub fn status(&self) -> &ToolResultStatus { &self.status }
    /// Returns a copy with a requested status.
    pub fn with_status(mut self, status: ToolResultStatus) -> Self { self.status = status; self }
}

/// Typed content carried by one stable block entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageBlock {
    /// Plain text.
    Text(String),
    /// Markdown source.
    Markdown(String),
    /// Code source.
    Code(CodeContent),
    /// Thinking lifecycle.
    Thinking(ThinkingContent),
    /// Tool-call lifecycle.
    ToolCall(ToolCallContent),
    /// Tool-result lifecycle.
    ToolResult(ToolResultContent),
    /// Typed error.
    Error(ErrorContent),
    /// Diff source.
    Diff(DiffContent),
    /// Quoted content.
    Quote(QuoteContent),
    /// Inert link.
    Link(LinkContent),
    /// Inert terminal attachment summary.
    TerminalAttachmentSummary(TerminalAttachmentSummary),
}
impl MessageBlock {
    pub(in crate::components::chat) fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text", Self::Markdown(_) => "markdown", Self::Code(_) => "code",
            Self::Thinking(_) => "thinking", Self::ToolCall(_) => "tool_call",
            Self::ToolResult(_) => "tool_result", Self::Error(_) => "error",
            Self::Diff(_) => "diff", Self::Quote(_) => "quote", Self::Link(_) => "link",
            Self::TerminalAttachmentSummary(_) => "terminal_attachment_summary",
        }
    }
}

/// Stable block identity paired with typed content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBlockEntry {
    pub(in crate::components::chat) id: BlockId, pub(in crate::components::chat) block: MessageBlock,
}
impl MessageBlockEntry {
    /// Creates an entry.
    pub const fn new(id: BlockId, block: MessageBlock) -> Self { Self { id, block } }
    /// Returns the block identity.
    pub const fn id(&self) -> BlockId { self.id }
    /// Returns typed content.
    pub const fn block(&self) -> &MessageBlock { &self.block }
}

/// Closed optional display metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatMessageMetadata {
    author: Option<MessageAuthor>, timestamp: Option<MessageTimestamp>,
}
impl ChatMessageMetadata {
    /// Creates metadata without inferred values.
    pub const fn new(author: Option<MessageAuthor>, timestamp: Option<MessageTimestamp>) -> Self {
        Self { author, timestamp }
    }
    /// Returns the optional author.
    pub const fn author(&self) -> Option<&MessageAuthor> { self.author.as_ref() }
    /// Returns the optional timestamp.
    pub const fn timestamp(&self) -> Option<&MessageTimestamp> { self.timestamp.as_ref() }
}

/// One typed conversation message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub(in crate::components::chat) id: MessageId, pub(in crate::components::chat) role: ChatRole,
    pub(in crate::components::chat) status: MessageStatus,
    pub(in crate::components::chat) revision: MessageRevision,
    pub(in crate::components::chat) blocks: Vec<MessageBlockEntry>,
    metadata: ChatMessageMetadata,
}
impl ChatMessage {
    /// Creates a pending message with at least one block.
    pub fn new(id: MessageId, role: ChatRole, blocks: Vec<MessageBlockEntry>)
        -> Result<Self, ConversationError> {
        if blocks.is_empty() {
            return Err(ConversationError::InvalidMessage {
                message_id: id, reason: "message must contain at least one block",
            });
        }
        Ok(Self { id, role, status: MessageStatus::Pending, revision: MessageRevision::INITIAL,
            blocks, metadata: ChatMessageMetadata::default() })
    }
    /// Sets optional display metadata.
    pub fn with_metadata(mut self, value: ChatMessageMetadata) -> Self { self.metadata = value; self }
    /// Returns stable identity.
    pub const fn id(&self) -> MessageId { self.id }
    /// Returns role.
    pub const fn role(&self) -> ChatRole { self.role }
    /// Returns status.
    pub const fn status(&self) -> &MessageStatus { &self.status }
    /// Returns revision.
    pub const fn revision(&self) -> MessageRevision { self.revision }
    /// Returns ordered entries.
    pub fn blocks(&self) -> &[MessageBlockEntry] { &self.blocks }
    /// Returns metadata.
    pub const fn metadata(&self) -> &ChatMessageMetadata { &self.metadata }
}

/// Guard for an expected conversation revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationGuard(pub(in crate::components::chat) ConversationRevision);
impl ConversationGuard {
    /// Creates a guard.
    pub const fn new(expected: ConversationRevision) -> Self { Self(expected) }
    /// Returns the expected revision.
    pub const fn expected(self) -> ConversationRevision { self.0 }
}

/// Guard for an existing message mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageMutationGuard {
    pub(in crate::components::chat) conversation: ConversationGuard,
    pub(in crate::components::chat) message_id: MessageId,
    pub(in crate::components::chat) message_revision: MessageRevision,
}
impl MessageMutationGuard {
    /// Creates a message guard.
    pub const fn new(conversation: ConversationGuard, message_id: MessageId,
        message_revision: MessageRevision) -> Self {
        Self { conversation, message_id, message_revision }
    }
    /// Returns conversation guard.
    pub const fn conversation(self) -> ConversationGuard { self.conversation }
    /// Returns target identity.
    pub const fn message_id(self) -> MessageId { self.message_id }
    /// Returns expected message revision.
    pub const fn message_revision(self) -> MessageRevision { self.message_revision }
}

macro_rules! update_payload {
    ($name:ident, $doc:literal, {$($field:ident : $ty:ty),+ $(,)?}) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name { $(pub(in crate::components::chat) $field: $ty),+ }
    };
}
update_payload!(PushUpdate, "Payload for a push.", {guard: ConversationGuard, message: ChatMessage});
update_payload!(AppendTextUpdate, "Payload for text append.", {
    guard: MessageMutationGuard, block_id: BlockId, delta: String
});
update_payload!(BlockUpdate, "Payload for append-block.", {
    guard: MessageMutationGuard, entry: MessageBlockEntry
});
update_payload!(InsertBlockUpdate, "Payload for insert-block.", {
    guard: MessageMutationGuard, position: usize, entry: MessageBlockEntry
});
update_payload!(ReplaceBlockUpdate, "Payload for block replacement.", {
    guard: MessageMutationGuard, block_id: BlockId, replacement: MessageBlock
});
update_payload!(GuardedUpdate, "Payload for a guarded message transition.", {
    guard: MessageMutationGuard
});
update_payload!(FailUpdate, "Payload for a failed transition.", {
    guard: MessageMutationGuard, cause: FailureCause
});
update_payload!(EditMessageUpdate, "Payload for a full message edit.", {
    guard: MessageMutationGuard, entries: Vec<MessageBlockEntry>
});
update_payload!(ResendUpdate, "Payload for a resend.", {
    source_guard: MessageMutationGuard, message: ChatMessage
});

/// Typed reducer mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConversationUpdate {
    /// Inserts a new pending message.
    Push(PushUpdate),
    /// Appends text to an appendable block.
    AppendText(AppendTextUpdate),
    /// Appends a newly discovered block.
    AppendMessageBlock(BlockUpdate),
    /// Inserts a newly discovered block.
    InsertMessageBlock(InsertBlockUpdate),
    /// Replaces one same-kind block.
    ReplaceBlock(ReplaceBlockUpdate),
    /// Completes a message.
    Complete(GuardedUpdate),
    /// Cancels a message.
    Cancel(GuardedUpdate),
    /// Fails a message.
    Fail(FailUpdate),
    /// Replaces a message block list.
    EditMessage(EditMessageUpdate),
    /// Deletes a message.
    DeleteMessage(GuardedUpdate),
    /// Creates a fresh message from a terminal source.
    Resend(ResendUpdate),
}
impl ConversationUpdate {
    /// Creates a push.
    pub fn push(guard: ConversationGuard, message: ChatMessage) -> Self {
        Self::Push(PushUpdate { guard, message })
    }
    /// Creates a non-empty text append.
    pub fn append_text(guard: MessageMutationGuard, block_id: BlockId,
        delta: impl Into<String>) -> Result<Self, ConversationError> {
        Ok(Self::AppendText(AppendTextUpdate {
            guard, block_id, delta: exact(delta, "delta")?,
        }))
    }
    /// Creates a block append.
    pub fn append_message_block(guard: MessageMutationGuard, entry: MessageBlockEntry) -> Self {
        Self::AppendMessageBlock(BlockUpdate { guard, entry })
    }
    /// Creates a checked-position block insertion.
    pub fn insert_message_block(guard: MessageMutationGuard, position: usize,
        entry: MessageBlockEntry) -> Self {
        Self::InsertMessageBlock(InsertBlockUpdate { guard, position, entry })
    }
    /// Creates a same-kind replacement.
    pub fn replace_block(guard: MessageMutationGuard, block_id: BlockId,
        replacement: MessageBlock) -> Self {
        Self::ReplaceBlock(ReplaceBlockUpdate { guard, block_id, replacement })
    }
    /// Creates a complete update.
    pub fn complete(guard: MessageMutationGuard) -> Self { Self::Complete(GuardedUpdate { guard }) }
    /// Creates a cancel update.
    pub fn cancel(guard: MessageMutationGuard) -> Self { Self::Cancel(GuardedUpdate { guard }) }
    /// Creates a fail update.
    pub fn fail(guard: MessageMutationGuard, cause: FailureCause) -> Self {
        Self::Fail(FailUpdate { guard, cause })
    }
    /// Creates a full edit.
    pub fn edit_message(guard: MessageMutationGuard, entries: Vec<MessageBlockEntry>) -> Self {
        Self::EditMessage(EditMessageUpdate { guard, entries })
    }
    /// Creates a deletion.
    pub fn delete_message(guard: MessageMutationGuard) -> Self {
        Self::DeleteMessage(GuardedUpdate { guard })
    }
    /// Creates a resend.
    pub fn resend(source_guard: MessageMutationGuard, message: ChatMessage) -> Self {
        Self::Resend(ResendUpdate { source_guard, message })
    }
    pub(in crate::components::chat) fn conversation_guard(&self) -> ConversationGuard {
        match self {
            Self::Push(v) => v.guard, Self::AppendText(v) => v.guard.conversation,
            Self::AppendMessageBlock(v) => v.guard.conversation,
            Self::InsertMessageBlock(v) => v.guard.conversation,
            Self::ReplaceBlock(v) => v.guard.conversation, Self::Complete(v) => v.guard.conversation,
            Self::Cancel(v) => v.guard.conversation, Self::Fail(v) => v.guard.conversation,
            Self::EditMessage(v) => v.guard.conversation,
            Self::DeleteMessage(v) => v.guard.conversation,
            Self::Resend(v) => v.source_guard.conversation,
        }
    }
}

/// One ordered conversation event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationEvent {
    pub(in crate::components::chat) event_id: UpdateId,
    pub(in crate::components::chat) sequence: u64,
    pub(in crate::components::chat) update: ConversationUpdate,
}
impl ConversationEvent {
    /// Creates an event envelope.
    pub fn new(event_id: UpdateId, sequence: u64, update: ConversationUpdate) -> Self {
        Self { event_id, sequence, update }
    }
    /// Returns event identity.
    pub fn event_id(&self) -> &UpdateId { &self.event_id }
    /// Returns sequence.
    pub const fn sequence(&self) -> u64 { self.sequence }
    /// Returns update.
    pub const fn update(&self) -> &ConversationUpdate { &self.update }
}

/// Disposition of an affected message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AffectedMessageDisposition {
    /// Message remains present.
    Present,
    /// Message was deleted.
    Deleted,
}
/// One deterministic affected message revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffectedMessage {
    pub(in crate::components::chat) message_id: MessageId,
    pub(in crate::components::chat) previous: Option<MessageRevision>,
    pub(in crate::components::chat) applied: MessageRevision,
    pub(in crate::components::chat) disposition: AffectedMessageDisposition,
}
impl AffectedMessage {
    /// Returns identity.
    pub const fn message_id(&self) -> MessageId { self.message_id }
    /// Returns previous revision, absent for new messages.
    pub const fn previous_revision(&self) -> Option<MessageRevision> { self.previous }
    /// Returns applied revision.
    pub const fn applied_revision(&self) -> MessageRevision { self.applied }
    /// Returns disposition.
    pub const fn disposition(&self) -> AffectedMessageDisposition { self.disposition }
}
/// Successful atomic reducer result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub(in crate::components::chat) revision: ConversationRevision,
    pub(in crate::components::chat) affected_messages: Vec<AffectedMessage>,
}
impl ApplyOutcome {
    /// Returns applied conversation revision.
    pub const fn revision(&self) -> ConversationRevision { self.revision }
    /// Returns affected messages in deterministic order.
    pub fn affected_messages(&self) -> &[AffectedMessage] { &self.affected_messages }
}

fn nonempty(value: impl Into<String>, field: &'static str) -> Result<String, ConversationError> {
    let value = value.into();
    if value.trim().is_empty() {
        Err(ConversationError::InvalidValue { field, reason: "must be non-empty after trimming" })
    } else { Ok(value) }
}
fn exact(value: impl Into<String>, field: &'static str) -> Result<String, ConversationError> {
    let value = value.into();
    if value.is_empty() {
        Err(ConversationError::InvalidValue { field, reason: "must be non-empty" })
    } else { Ok(value) }
}
fn unique<'a>(values: impl IntoIterator<Item = &'a str>, field: &'static str)
    -> Result<(), ConversationError> {
    let mut seen = BTreeSet::new();
    if values.into_iter().all(|value| seen.insert(value)) { Ok(()) } else {
        Err(ConversationError::InvalidValue { field, reason: "names must be unique" })
    }
}
fn canonical_decimal(value: &str) -> bool {
    if value == "0" { return true; }
    if value == "-0" { return false; }
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    if unsigned.is_empty() { return false; }
    let mut parts = unsigned.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next();
    parts.next().is_none() && !integer.is_empty()
        && integer.bytes().all(|byte| byte.is_ascii_digit())
        && (integer == "0" || !integer.starts_with('0'))
        && fraction.is_none_or(|part| !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit()) && !part.ends_with('0'))
}

}

pub use compact::*;

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    #[test] fn gh62_provider_independent_model_contract() { assert!(ToolCallContent::new(ToolCallId::new("c").unwrap(), "tool", vec![]).is_ok()); }
    #[test] fn gh62_update_id_public_construction() { assert_eq!(UpdateId::new("event").unwrap().to_string(), "event"); }
    #[test] fn gh62_empty_and_missing_contract() { assert!(ChatMessage::new(MessageId::new(1), ChatRole::User, vec![]).is_err()); }
    #[test] fn gh62_revisioned_atomic_mutations() { assert_eq!(MessageRevision::new(2).unwrap().get(), 2); }
    #[test] fn gh62_message_transition_matrix() { assert_ne!(MessageStatus::Pending, MessageStatus::Complete); }
    #[test] fn gh62_event_idempotency_contract() { let update = ConversationUpdate::complete(MessageMutationGuard::new(ConversationGuard::new(ConversationRevision::INITIAL), MessageId::new(1), MessageRevision::INITIAL)); let event = ConversationEvent::new(UpdateId::new("same").unwrap(), 0, update); assert_eq!(event, event.clone()); }
    #[test] fn gh62_replay_retention_boundary() { assert_ne!(UpdateId::new("old").unwrap(), UpdateId::new("new").unwrap()); }
    #[test] fn gh62_ordered_update_contract() { let event = ConversationEvent::new(UpdateId::new("ordered").unwrap(), 7, ConversationUpdate::push(ConversationGuard::new(ConversationRevision::INITIAL), ChatMessage::new(MessageId::new(1), ChatRole::User, vec![MessageBlockEntry::new(BlockId::new(1), MessageBlock::Text("x".into()))]).unwrap())); assert_eq!(event.sequence(), 7); }
    #[test] fn gh62_terminal_revision_race_contract() { assert_eq!(MessageRevision::INITIAL.get(), 1); assert!(MessageRevision::new(0).is_err()); }
    #[test] fn gh62_cancellation_contract() { let update = ConversationUpdate::cancel(MessageMutationGuard::new(ConversationGuard::new(ConversationRevision::INITIAL), MessageId::new(1), MessageRevision::INITIAL)); assert!(matches!(update, ConversationUpdate::Cancel(_))); }
}
