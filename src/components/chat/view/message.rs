//! Composition of one typed chat message into an element tree.
//!
//! The view borrows an immutable [`ChatMessage`] and walks its blocks in source
//! order. It owns the reconciliation keys, so a block that moves within a
//! message keeps its identity across frames rather than being rebuilt.
//!
//! Optional metadata stays optional: an absent author or timestamp renders
//! nothing. Substituting a placeholder would put text on screen the application
//! never supplied and a reader cannot tell apart from real data.

use super::block::{render_block, streaming_indicator};
use super::custom::{ChatBlockRef, ChatBlockRenderer, ChatRenderContext, ChatRenderOverride};
use super::{ChatMessageViewOptions, MessageViewVariant};
use crate::components::chat::{BlockId, ChatMessage, ChatRole, MessageBlock, MessageStatus};
use crate::components::{Box as BoxView, Text};
use crate::core::{BorderStyle, Color, Dimension, Element, FlexDirection};

/// A typed view over one chat message.
///
/// Borrowing rather than owning keeps rendering a pure projection of
/// conversation state: the view cannot drift from the message it describes.
pub struct ChatMessageView<'a> {
    message: &'a ChatMessage,
    options: ChatMessageViewOptions,
    renderer: Option<&'a dyn ChatBlockRenderer>,
}

impl<'a> ChatMessageView<'a> {
    /// Creates a view with default presentation options.
    pub fn new(message: &'a ChatMessage) -> Self {
        Self {
            message,
            options: ChatMessageViewOptions::default(),
            renderer: None,
        }
    }

    /// Replaces the presentation options.
    pub fn options(mut self, options: ChatMessageViewOptions) -> Self {
        self.options = options;
        self
    }

    /// Installs a typed custom block renderer.
    ///
    /// The renderer may decline any individual block, in which case that block
    /// takes the library's default path.
    pub fn renderer(mut self, renderer: &'a dyn ChatBlockRenderer) -> Self {
        self.renderer = Some(renderer);
        self
    }

    /// Builds the element tree for this message.
    pub fn into_element(self) -> Element {
        // A message fills the width it is given. Left to size itself, the
        // container takes the max-content width of its widest row — usually the
        // header — and the body then wraps to that width instead of the real
        // one, so the tail of every long line is clipped away at the edge.
        let mut view = BoxView::new()
            .flex_direction(FlexDirection::Column)
            .width(Dimension::Percent(100.0));

        view = match self.options.variant() {
            MessageViewVariant::Compact => view,
            MessageViewVariant::Bordered => view.border_style(BorderStyle::Single),
            MessageViewVariant::Bubble => view
                .border_style(BorderStyle::Round)
                .border_color(role_color(self.message.role())),
        };

        view = view.child(self.header());

        for (position, entry) in self.message.blocks().iter().enumerate() {
            let stable_key = block_key(self.message, entry.id());
            let body = self.render_one(entry.block(), entry.id(), position, &stable_key);
            // The key wrapper is a column: a row-direction box sizes its child
            // to max-content and clips at the terminal edge instead of
            // wrapping, which silently drops the tail of a long block.
            view = view.child(
                BoxView::new()
                    .key(stable_key)
                    .flex_direction(FlexDirection::Column)
                    .child(body),
            );
        }

        // The indicator belongs to the message, not to any block: it says the
        // message is still arriving, which no single block can know.
        if matches!(self.message.status(), MessageStatus::Streaming) {
            view = view.child(streaming_indicator(self.options.indicator_frame()));
        }

        view.into_element()
    }

    /// Render one block, giving a custom renderer first refusal.
    fn render_one(
        &self,
        block: &MessageBlock,
        block_id: BlockId,
        position: usize,
        stable_key: &str,
    ) -> Element {
        if let Some(renderer) = self.renderer {
            let context = ChatRenderContext {
                message_id: self.message.id(),
                message_revision: self.message.revision(),
                role: self.message.role(),
                status: self.message.status(),
                block_id,
                position,
                variant: self.options.variant(),
                stable_key,
                style: self.options.style(),
            };
            if let ChatRenderOverride::Element(element) =
                renderer.render(ChatBlockRef::from(block), context)
            {
                return *element;
            }
        }

        render_block(block, &self.options)
    }

    /// The role line, plus whatever optional metadata was actually supplied.
    fn header(&self) -> BoxView {
        let mut header = BoxView::new().child(
            Text::new(role_label(self.message.role()).to_owned())
                .color(role_color(self.message.role()))
                .bold(),
        );

        let metadata = self.message.metadata();
        if let Some(author) = metadata.author() {
            header = header.child(Text::new(format!(" {}", author.as_str())).dim());
        }
        if let Some(timestamp) = metadata.timestamp() {
            header = header.child(Text::new(format!(" {}", timestamp.as_str())).dim());
        }

        // A terminal status other than plain completion is worth stating; a
        // failure that renders identically to success is a silent failure.
        if let Some(status) = status_label(self.message.status()) {
            header = header.child(Text::new(format!(" · {status}")).dim());
        }

        header
    }
}

/// Reconciliation key for one block within one message.
///
/// Keyed on identity, never on position, so reordering blocks moves them rather
/// than rebuilding them.
fn block_key(message: &ChatMessage, block_id: BlockId) -> String {
    format!("chat-msg-{}-block-{}", message.id().get(), block_id.get())
}

fn role_label(role: ChatRole) -> &'static str {
    match role {
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::System => "system",
        ChatRole::Tool => "tool",
    }
}

fn role_color(role: ChatRole) -> Color {
    match role {
        ChatRole::User => Color::Green,
        ChatRole::Assistant => Color::Blue,
        ChatRole::System => Color::Yellow,
        ChatRole::Tool => Color::Magenta,
    }
}

/// A status worth showing beside the role, or `None` when it adds nothing.
fn status_label(status: &MessageStatus) -> Option<String> {
    match status {
        // Streaming has its own indicator, and a complete message is the
        // unremarkable case.
        MessageStatus::Streaming | MessageStatus::Complete => None,
        MessageStatus::Pending => Some("pending".to_owned()),
        MessageStatus::Cancelled => Some("cancelled".to_owned()),
        MessageStatus::Failed(cause) => Some(format!("failed: {}", cause.as_str())),
    }
}
