use rnk::Element;
use rnk::components::chat::view::{
    ChatBlockRef, ChatBlockRenderer, ChatMessageViewOptions, ChatMessageViewStyle,
    ChatRenderContext, ChatRenderOverride, MessageViewVariant, StreamingIndicatorFrame,
    ThinkingDisclosure,
};
use rnk::components::chat::{
    BlockId, ChatRole, CodeContent, ConversationError, DiffContent, ErrorContent, LinkContent,
    MessageBlock, MessageId, MessageRevision, MessageStatus, QuoteContent,
    TerminalAttachmentSummary, ThinkingContent, ThinkingId, ToolCallContent, ToolCallId,
    ToolResultContent,
};
use std::num::NonZeroUsize;

fn render_context<'a>(
    status: &'a MessageStatus,
    style: &'a ChatMessageViewStyle,
) -> ChatRenderContext<'a> {
    ChatRenderContext {
        message_id: MessageId::new(7),
        message_revision: MessageRevision::INITIAL,
        role: ChatRole::Assistant,
        status,
        block_id: BlockId::new(11),
        position: 2,
        variant: MessageViewVariant::Bordered,
        stable_key: "chat-block/11",
        style,
    }
}

struct ExplicitDefaultRenderer;

impl ChatBlockRenderer for ExplicitDefaultRenderer {
    fn render(
        &self,
        block: ChatBlockRef<'_>,
        context: ChatRenderContext<'_>,
    ) -> ChatRenderOverride {
        assert!(matches!(block, ChatBlockRef::Text("default")));
        assert_eq!(context.message_id, MessageId::new(7));
        assert_eq!(context.message_revision, MessageRevision::INITIAL);
        assert_eq!(context.role, ChatRole::Assistant);
        assert!(matches!(context.status, MessageStatus::Pending));
        assert_eq!(context.block_id, BlockId::new(11));
        assert_eq!(context.position, 2);
        assert_eq!(context.variant, MessageViewVariant::Bordered);
        assert_eq!(context.stable_key, "chat-block/11");
        ChatRenderOverride::UseDefault
    }
}

#[test]
fn typed_trait_and_closure_override_or_explicitly_default() {
    let status = MessageStatus::Pending;
    let style = ChatMessageViewStyle::new();
    let context = render_context(&status, &style);
    let defaults = ChatMessageViewOptions::default();

    assert_eq!(defaults.variant(), MessageViewVariant::Compact);
    assert_eq!(
        defaults.thinking_disclosure(),
        ThinkingDisclosure::Collapsed
    );
    assert_eq!(defaults.thinking_preview_lines().get(), 5);
    assert_eq!(defaults.tool_result_preview_lines().get(), 12);
    assert_eq!(defaults.indicator_frame().get(), 0);
    assert_eq!(defaults.style(), &ChatMessageViewStyle::new());

    let explicit = ChatMessageViewOptions::new(
        MessageViewVariant::Bubble,
        ThinkingDisclosure::Expanded,
        NonZeroUsize::new(2).expect("two is non-zero"),
        NonZeroUsize::new(3).expect("three is non-zero"),
        StreamingIndicatorFrame::new(4),
        ChatMessageViewStyle::new(),
    );
    assert_eq!(explicit.variant(), MessageViewVariant::Bubble);
    assert_eq!(explicit.thinking_disclosure(), ThinkingDisclosure::Expanded);
    assert_eq!(explicit.thinking_preview_lines().get(), 2);
    assert_eq!(explicit.tool_result_preview_lines().get(), 3);
    assert_eq!(explicit.indicator_frame().get(), 4);

    assert!(matches!(
        ExplicitDefaultRenderer.render(ChatBlockRef::Text("default"), context),
        ChatRenderOverride::UseDefault
    ));

    let closure = |block: ChatBlockRef<'_>, received: ChatRenderContext<'_>| {
        assert!(matches!(block, ChatBlockRef::Markdown("custom")));
        assert_eq!(received.block_id, BlockId::new(11));
        ChatRenderOverride::element(Element::text("override"))
    };
    let renderer: &dyn ChatBlockRenderer = &closure;
    match renderer.render(ChatBlockRef::Markdown("custom"), context) {
        ChatRenderOverride::Element(element) => {
            assert_eq!(element.get_text(), Some("override"));
        }
        ChatRenderOverride::UseDefault => {
            panic!("typed closure must return its explicit element")
        }
    }
}

fn typed_variant_name(block: ChatBlockRef<'_>) -> &'static str {
    match block {
        ChatBlockRef::Text(content) => {
            let _: &str = content;
            "text"
        }
        ChatBlockRef::Markdown(content) => {
            let _: &str = content;
            "markdown"
        }
        ChatBlockRef::Code(content) => {
            let _ = content.content();
            "code"
        }
        ChatBlockRef::Thinking(content) => {
            let _ = content.id();
            "thinking"
        }
        ChatBlockRef::ToolCall(content) => {
            let _ = content.call_id();
            "tool_call"
        }
        ChatBlockRef::ToolResult(content) => {
            let _ = content.call_id();
            "tool_result"
        }
        ChatBlockRef::Error(content) => {
            let _ = content.message();
            "error"
        }
        ChatBlockRef::Diff(content) => {
            let _ = content.content();
            "diff"
        }
        ChatBlockRef::Quote(content) => {
            let _ = content.content();
            "quote"
        }
        ChatBlockRef::Link(content) => {
            let _ = content.target();
            "link"
        }
        ChatBlockRef::TerminalAttachmentSummary(content) => {
            let _ = content.summary();
            "terminal_attachment_summary"
        }
    }
}

fn assert_copy<T: Copy>() {}

#[test]
fn typed_renderer_contract_contains_no_dynamic_erasure() -> Result<(), ConversationError> {
    assert_copy::<ChatBlockRef<'static>>();
    assert_copy::<ChatRenderContext<'static>>();

    let call_id = ToolCallId::new("call")?;
    let blocks = vec![
        MessageBlock::Text("text".into()),
        MessageBlock::Markdown("markdown".into()),
        MessageBlock::Code(CodeContent::new("code")?),
        MessageBlock::Thinking(ThinkingContent::new(
            ThinkingId::new("thinking")?,
            "thought",
        )),
        MessageBlock::ToolCall(ToolCallContent::new(call_id.clone(), "tool", Vec::new())?),
        MessageBlock::ToolResult(ToolResultContent::new(call_id, "result")),
        MessageBlock::Error(ErrorContent::new("error")?),
        MessageBlock::Diff(DiffContent::new("diff")?),
        MessageBlock::Quote(QuoteContent::new("quote")?),
        MessageBlock::Link(LinkContent::new("label", "target")?),
        MessageBlock::TerminalAttachmentSummary(TerminalAttachmentSummary::new(
            "attachment",
            "summary",
        )?),
    ];
    let names = blocks
        .iter()
        .map(|block| typed_variant_name(ChatBlockRef::from(block)))
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        [
            "text",
            "markdown",
            "code",
            "thinking",
            "tool_call",
            "tool_result",
            "error",
            "diff",
            "quote",
            "link",
            "terminal_attachment_summary",
        ]
    );
    Ok(())
}
