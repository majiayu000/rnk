//! Typed presentation contracts for provider-independent chat messages.
//!
//! This first slice defines the values shared by custom block renderers. Message
//! composition, default block rendering, and caller-owned caching remain private
//! implementation stages.

use std::num::NonZeroUsize;

mod block;
mod cache;
mod custom;
mod message;

pub use custom::{ChatBlockRef, ChatBlockRenderer, ChatRenderContext, ChatRenderOverride};
pub use message::ChatMessageView;

const DEFAULT_THINKING_PREVIEW_LINES: NonZeroUsize =
    NonZeroUsize::new(5).expect("five is non-zero");
const DEFAULT_TOOL_RESULT_PREVIEW_LINES: NonZeroUsize =
    NonZeroUsize::new(12).expect("twelve is non-zero");

/// Presentation container selected for one typed chat message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageViewVariant {
    /// A minimal container with compact spacing.
    #[default]
    Compact,
    /// A container with a visible border.
    Bordered,
    /// A role-oriented message bubble.
    Bubble,
}

/// Caller-controlled visibility for one thinking block.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThinkingDisclosure {
    /// Show only the configured preview.
    #[default]
    Collapsed,
    /// Show the complete thinking content.
    Expanded,
}

/// Explicit animation frame supplied by the caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamingIndicatorFrame(usize);

impl StreamingIndicatorFrame {
    /// Creates an explicit frame value.
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the frame value.
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Local style input for one typed chat message view.
///
/// The concrete resolved fields are intentionally private until message
/// rendering owns their theme resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatMessageViewStyle {
    private: (),
}

impl ChatMessageViewStyle {
    /// Creates an empty local style input.
    pub const fn new() -> Self {
        Self { private: () }
    }
}

/// Typed presentation inputs shared by chat message rendering stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessageViewOptions {
    variant: MessageViewVariant,
    thinking_disclosure: ThinkingDisclosure,
    thinking_preview_lines: NonZeroUsize,
    tool_result_preview_lines: NonZeroUsize,
    indicator_frame: StreamingIndicatorFrame,
    style: ChatMessageViewStyle,
}

impl ChatMessageViewOptions {
    /// Creates a complete set of typed presentation inputs.
    pub fn new(
        variant: MessageViewVariant,
        thinking_disclosure: ThinkingDisclosure,
        thinking_preview_lines: NonZeroUsize,
        tool_result_preview_lines: NonZeroUsize,
        indicator_frame: StreamingIndicatorFrame,
        style: ChatMessageViewStyle,
    ) -> Self {
        Self {
            variant,
            thinking_disclosure,
            thinking_preview_lines,
            tool_result_preview_lines,
            indicator_frame,
            style,
        }
    }

    /// Returns the selected message container.
    pub const fn variant(&self) -> MessageViewVariant {
        self.variant
    }

    /// Returns the default controlled thinking disclosure.
    pub const fn thinking_disclosure(&self) -> ThinkingDisclosure {
        self.thinking_disclosure
    }

    /// Returns the non-zero thinking preview limit.
    pub const fn thinking_preview_lines(&self) -> NonZeroUsize {
        self.thinking_preview_lines
    }

    /// Returns the non-zero tool-result preview limit.
    pub const fn tool_result_preview_lines(&self) -> NonZeroUsize {
        self.tool_result_preview_lines
    }

    /// Returns the caller-supplied indicator frame.
    pub const fn indicator_frame(&self) -> StreamingIndicatorFrame {
        self.indicator_frame
    }

    /// Returns the local style input.
    pub const fn style(&self) -> &ChatMessageViewStyle {
        &self.style
    }
}

impl Default for ChatMessageViewOptions {
    fn default() -> Self {
        Self::new(
            MessageViewVariant::Compact,
            ThinkingDisclosure::Collapsed,
            DEFAULT_THINKING_PREVIEW_LINES,
            DEFAULT_TOOL_RESULT_PREVIEW_LINES,
            StreamingIndicatorFrame::default(),
            ChatMessageViewStyle::default(),
        )
    }
}
