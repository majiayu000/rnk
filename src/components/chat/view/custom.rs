//! Closed borrowed inputs for custom chat block rendering.

use super::{ChatMessageViewStyle, MessageViewVariant};
use crate::components::chat::{
    BlockId, ChatRole, CodeContent, DiffContent, ErrorContent, LinkContent, MessageBlock,
    MessageId, MessageRevision, MessageStatus, QuoteContent, TerminalAttachmentSummary,
    ThinkingContent, ToolCallContent, ToolResultContent,
};
use crate::core::Element;

/// A closed borrowed projection of every typed chat block.
#[derive(Debug, Clone, Copy)]
pub enum ChatBlockRef<'a> {
    /// Plain text content.
    Text(&'a str),
    /// Markdown source.
    Markdown(&'a str),
    /// Typed code content.
    Code(&'a CodeContent),
    /// Typed thinking content and lifecycle.
    Thinking(&'a ThinkingContent),
    /// Typed tool-call content and lifecycle.
    ToolCall(&'a ToolCallContent),
    /// Typed tool-result content and lifecycle.
    ToolResult(&'a ToolResultContent),
    /// Typed error content.
    Error(&'a ErrorContent),
    /// Typed diff content.
    Diff(&'a DiffContent),
    /// Typed quoted content.
    Quote(&'a QuoteContent),
    /// Typed inert link content.
    Link(&'a LinkContent),
    /// Typed inert terminal attachment summary.
    TerminalAttachmentSummary(&'a TerminalAttachmentSummary),
}

impl<'a> From<&'a MessageBlock> for ChatBlockRef<'a> {
    fn from(block: &'a MessageBlock) -> Self {
        match block {
            MessageBlock::Text(content) => Self::Text(content),
            MessageBlock::Markdown(content) => Self::Markdown(content),
            MessageBlock::Code(content) => Self::Code(content),
            MessageBlock::Thinking(content) => Self::Thinking(content),
            MessageBlock::ToolCall(content) => Self::ToolCall(content),
            MessageBlock::ToolResult(content) => Self::ToolResult(content),
            MessageBlock::Error(content) => Self::Error(content),
            MessageBlock::Diff(content) => Self::Diff(content),
            MessageBlock::Quote(content) => Self::Quote(content),
            MessageBlock::Link(content) => Self::Link(content),
            MessageBlock::TerminalAttachmentSummary(content) => {
                Self::TerminalAttachmentSummary(content)
            }
        }
    }
}

/// Read-only typed context supplied for one custom block-renderer call.
#[derive(Debug, Clone, Copy)]
pub struct ChatRenderContext<'a> {
    /// Identity of the containing message.
    pub message_id: MessageId,
    /// Current revision of the containing message.
    pub message_revision: MessageRevision,
    /// Role of the containing message.
    pub role: ChatRole,
    /// Current lifecycle of the containing message.
    pub status: &'a MessageStatus,
    /// Conversation-lifetime identity of the current block.
    pub block_id: BlockId,
    /// Current observational source position.
    pub position: usize,
    /// Presentation container selected for the message.
    pub variant: MessageViewVariant,
    /// Stable reconciliation key owned by the view layer.
    pub stable_key: &'a str,
    /// Resolved local style owned by the view layer.
    pub style: &'a ChatMessageViewStyle,
}

/// Explicit result of a custom block-renderer call.
#[derive(Debug)]
pub enum ChatRenderOverride {
    /// Continue through the library's default typed renderer.
    UseDefault,
    /// Use the supplied element as this block's custom body.
    Element(Box<Element>),
}

impl ChatRenderOverride {
    /// Wraps an owned element as an explicit custom block body.
    pub fn element(element: Element) -> Self {
        Self::Element(Box::new(element))
    }
}

/// Typed customization boundary for one chat block body.
pub trait ChatBlockRenderer {
    /// Renders one borrowed typed block or explicitly selects the default path.
    fn render(&self, block: ChatBlockRef<'_>, context: ChatRenderContext<'_>)
    -> ChatRenderOverride;
}

impl<F> ChatBlockRenderer for F
where
    F: for<'block, 'context> Fn(
        ChatBlockRef<'block>,
        ChatRenderContext<'context>,
    ) -> ChatRenderOverride,
{
    fn render(
        &self,
        block: ChatBlockRef<'_>,
        context: ChatRenderContext<'_>,
    ) -> ChatRenderOverride {
        self(block, context)
    }
}
