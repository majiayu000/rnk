//! GH-63: typed message and AI content-block views.
//!
//! Every block kind and lifecycle state has to reach the screen as itself. The
//! cases that matter are the ones where a defect is invisible: a failure that
//! renders like success, a truncated preview with nothing saying so, and absent
//! optional metadata replaced by a plausible placeholder.

use rnk::components::chat::{
    BlockId, ChatMessage, ChatMessageMetadata, ChatMessageView, ChatMessageViewOptions, ChatRole,
    CodeContent, ErrorContent, ErrorSource, FailureCause, LinkContent, MessageAuthor, MessageBlock,
    MessageBlockEntry, MessageId, MessageStatus, MessageTimestamp, MessageViewVariant,
    QuoteContent, StreamingIndicatorFrame, TerminalAttachmentSummary, ThinkingContent,
    ThinkingDisclosure, ThinkingId, ThinkingStatus, ToolArgument, ToolCallContent, ToolCallId,
    ToolCallStatus, ToolResultContent, ToolResultStatus, TypedField, TypedValue,
};
use rnk::prelude::*;

const WIDTH: u16 = 60;

/// Render a message built from one block.
fn render_block(block: MessageBlock) -> String {
    render_message(message_with(block, ChatRole::Assistant))
}

fn message_with(block: MessageBlock, role: ChatRole) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(1),
        role,
        vec![MessageBlockEntry::new(BlockId::new(1), block)],
    )
    .expect("a message with one block is valid")
}

fn render_message(message: ChatMessage) -> String {
    render_view(ChatMessageView::new(&message))
}

fn render_view(view: ChatMessageView<'_>) -> String {
    render_to_string_no_trim(&view.into_element(), WIDTH)
}

fn options(
    disclosure: ThinkingDisclosure,
    thinking: usize,
    tool_result: usize,
) -> ChatMessageViewOptions {
    ChatMessageViewOptions::new(
        MessageViewVariant::Compact,
        disclosure,
        std::num::NonZeroUsize::new(thinking).expect("non-zero"),
        std::num::NonZeroUsize::new(tool_result).expect("non-zero"),
        StreamingIndicatorFrame::default(),
        Default::default(),
    )
}

fn cause(text: &str) -> FailureCause {
    FailureCause::new(text).expect("non-empty cause")
}

#[test]
fn every_role_is_labelled() {
    for (role, label) in [
        (ChatRole::User, "user"),
        (ChatRole::Assistant, "assistant"),
        (ChatRole::System, "system"),
        (ChatRole::Tool, "tool"),
    ] {
        let message = message_with(MessageBlock::Text("hi".into()), role);
        let output = render_message(message);
        assert!(output.contains(label), "{role:?} rendered as {output:?}");
    }
}

#[test]
fn a_failed_message_never_looks_like_a_complete_one() {
    let mut failed = message_with(MessageBlock::Text("partial".into()), ChatRole::Assistant);
    failed = ChatMessage::try_restore(
        failed.id(),
        failed.role(),
        MessageStatus::Failed(cause("rate limited")),
        failed.revision(),
        failed.blocks().to_vec(),
        failed.metadata().clone(),
    )
    .expect("restoring a failed message is valid");

    let output = render_message(failed);
    assert!(
        output.contains("rate limited"),
        "the typed cause must survive to the screen: {output:?}"
    );
}

#[test]
fn each_message_lifecycle_state_is_distinguishable() {
    let base = message_with(MessageBlock::Text("body".into()), ChatRole::Assistant);
    let states = [
        MessageStatus::Pending,
        MessageStatus::Streaming,
        MessageStatus::Complete,
        MessageStatus::Cancelled,
        MessageStatus::Failed(cause("upstream refused")),
    ];

    let rendered: Vec<String> = states
        .iter()
        .map(|status| {
            let message = ChatMessage::try_restore(
                base.id(),
                base.role(),
                status.clone(),
                base.revision(),
                base.blocks().to_vec(),
                base.metadata().clone(),
            )
            .expect("valid");
            render_message(message)
        })
        .collect();

    // Complete is the unmarked case; the rest must each differ from it.
    let complete = &rendered[2];
    for (index, output) in rendered.iter().enumerate() {
        if index == 2 {
            continue;
        }
        assert_ne!(
            output, complete,
            "state {:?} renders identically to Complete",
            states[index]
        );
    }
}

#[test]
fn absent_optional_metadata_leaves_nothing_behind() {
    let without = message_with(MessageBlock::Text("body".into()), ChatRole::User);
    let bare = render_message(without.clone());

    let with = without.clone().with_metadata(ChatMessageMetadata::new(
        Some(MessageAuthor::new("Ada").expect("non-empty")),
        Some(MessageTimestamp::new("09:31").expect("non-empty")),
    ));
    let annotated = render_message(with);

    assert!(annotated.contains("Ada") && annotated.contains("09:31"));
    assert!(
        !bare.contains("Ada") && !bare.contains("09:31"),
        "metadata appeared without being supplied: {bare:?}"
    );
    // Nothing stands in for the absent values.
    for placeholder in ["unknown", "N/A", "--", "anonymous"] {
        assert!(
            !bare.to_lowercase().contains(placeholder),
            "placeholder {placeholder:?} invented in {bare:?}"
        );
    }
}

#[test]
fn an_error_block_never_degrades_to_ordinary_text() {
    let error = ErrorContent::new("disk full")
        .expect("non-empty")
        .with_source(ErrorSource::new("io::ErrorKind::StorageFull").expect("non-empty"));
    let as_error = render_block(MessageBlock::Error(error));
    let as_text = render_block(MessageBlock::Text("disk full".into()));

    assert!(as_error.contains("disk full"));
    assert!(
        as_error.contains("io::ErrorKind::StorageFull"),
        "the optional source must be projected: {as_error:?}"
    );
    assert_ne!(
        as_error, as_text,
        "an error must not render the same as plain text carrying the same words"
    );
}

#[test]
fn thinking_disclosure_is_controlled_by_the_caller() {
    let content = ThinkingContent::new(
        ThinkingId::new("t1").expect("non-empty"),
        "one\ntwo\nthree\nfour\nfive\nsix",
    );

    let collapsed = render_view(
        ChatMessageView::new(&message_with(
            MessageBlock::Thinking(content.clone()),
            ChatRole::Assistant,
        ))
        .options(options(ThinkingDisclosure::Collapsed, 2, 12)),
    );
    let expanded = render_view(
        ChatMessageView::new(&message_with(
            MessageBlock::Thinking(content),
            ChatRole::Assistant,
        ))
        .options(options(ThinkingDisclosure::Expanded, 2, 12)),
    );

    assert!(collapsed.contains("one") && collapsed.contains("two"));
    assert!(
        !collapsed.contains("six"),
        "collapsed preview leaked past its limit: {collapsed:?}"
    );
    assert!(
        collapsed.contains("4 more lines"),
        "truncation must be stated: {collapsed:?}"
    );
    assert!(expanded.contains("six"));
    assert!(
        !expanded.contains("more lines"),
        "nothing was hidden, so nothing should claim it was: {expanded:?}"
    );
}

#[test]
fn a_preview_at_its_exact_limit_claims_no_hidden_content() {
    let content = ThinkingContent::new(ThinkingId::new("t1").expect("non-empty"), "a\nb\nc");
    let output = render_view(
        ChatMessageView::new(&message_with(
            MessageBlock::Thinking(content),
            ChatRole::Assistant,
        ))
        .options(options(ThinkingDisclosure::Collapsed, 3, 12)),
    );

    assert!(output.contains('c'));
    assert!(
        !output.contains("more line"),
        "an off-by-one would claim hidden content that does not exist: {output:?}"
    );
}

#[test]
fn previews_count_rows_the_way_the_renderer_does() {
    // A lone CR is a hard break to the shared flow but ordinary text to
    // `str::lines()`. A preview built on the latter would under-count rows and
    // silently show more than its limit.
    let content = ThinkingContent::new(
        ThinkingId::new("t1").expect("non-empty"),
        "alpha\rbeta\rgamma\rdelta",
    );
    let output = render_view(
        ChatMessageView::new(&message_with(
            MessageBlock::Thinking(content),
            ChatRole::Assistant,
        ))
        .options(options(ThinkingDisclosure::Collapsed, 2, 12)),
    );

    assert!(
        output.contains("2 more lines"),
        "CR-separated rows were not counted as rows: {output:?}"
    );
    assert!(
        !output.contains("delta"),
        "content past the limit leaked: {output:?}"
    );
}

#[test]
fn tool_call_covers_every_lifecycle_state() {
    let call_id = ToolCallId::new("call-1").expect("non-empty");
    let statuses = [
        (ToolCallStatus::Pending, "pending"),
        (ToolCallStatus::Running, "running"),
        (ToolCallStatus::Succeeded, "succeeded"),
        (ToolCallStatus::Cancelled, "cancelled"),
        (ToolCallStatus::Failed(cause("timed out")), "timed out"),
    ];

    for (status, expected) in statuses {
        let content = ToolCallContent::new(call_id.clone(), "search", Vec::new())
            .expect("valid call")
            .with_status(status.clone());
        let output = render_block(MessageBlock::ToolCall(content));
        assert!(
            output.contains(expected),
            "{status:?} did not render {expected:?}: {output:?}"
        );
    }
}

#[test]
fn typed_arguments_render_without_json_and_keep_their_order() {
    let arguments = vec![
        ToolArgument::new("query", TypedValue::String("rust".into())).expect("valid"),
        ToolArgument::new("limit", TypedValue::Integer(10)).expect("valid"),
        ToolArgument::new("exact", TypedValue::Bool(true)).expect("valid"),
        ToolArgument::new("missing", TypedValue::Null).expect("valid"),
        ToolArgument::new(
            "nested",
            TypedValue::object(vec![
                TypedField::new("inner", TypedValue::List(vec![TypedValue::Integer(1)]))
                    .expect("valid"),
            ])
            .expect("valid"),
        )
        .expect("valid"),
    ];

    let content = ToolCallContent::new(
        ToolCallId::new("call-1").expect("non-empty"),
        "search",
        arguments,
    )
    .expect("valid call");
    let output = render_block(MessageBlock::ToolCall(content));

    for name in ["query", "limit", "exact", "missing", "nested"] {
        assert!(output.contains(name), "{name} missing from {output:?}");
    }
    assert!(output.contains("true") && output.contains("null") && output.contains("10"));
    assert!(
        output.contains("inner"),
        "nested fields must recurse: {output:?}"
    );

    // Source order, not sorted.
    let query_at = output.find("query").expect("query rendered");
    let limit_at = output.find("limit").expect("limit rendered");
    assert!(query_at < limit_at, "argument order changed: {output:?}");
}

#[test]
fn an_empty_string_argument_is_visible() {
    let content = ToolCallContent::new(
        ToolCallId::new("call-1").expect("non-empty"),
        "echo",
        vec![ToolArgument::new("text", TypedValue::String(String::new())).expect("valid")],
    )
    .expect("valid call");

    let output = render_block(MessageBlock::ToolCall(content));
    assert!(
        output.contains("\"\""),
        "an empty string must not read as a missing value: {output:?}"
    );
}

#[test]
fn tool_result_states_its_status_and_any_truncation() {
    let call_id = ToolCallId::new("call-1").expect("non-empty");

    for (status, expected) in [
        (ToolResultStatus::Pending, "pending"),
        (ToolResultStatus::Streaming, "streaming"),
        (ToolResultStatus::Complete, "complete"),
        (ToolResultStatus::Cancelled, "cancelled"),
        (
            ToolResultStatus::Failed(cause("tool crashed")),
            "tool crashed",
        ),
    ] {
        let content = ToolResultContent::new(call_id.clone(), "output").with_status(status.clone());
        let output = render_block(MessageBlock::ToolResult(content));
        assert!(
            output.contains(expected),
            "{status:?} did not render {expected:?}: {output:?}"
        );
    }

    let long = (1..=20)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let content = ToolResultContent::new(call_id, long).with_status(ToolResultStatus::Complete);
    let output = render_view(
        ChatMessageView::new(&message_with(
            MessageBlock::ToolResult(content),
            ChatRole::Tool,
        ))
        .options(options(ThinkingDisclosure::Collapsed, 5, 3)),
    );

    assert!(output.contains("17 more lines"), "{output:?}");
}

#[test]
fn code_shows_a_language_only_when_one_exists() {
    let with_language = CodeContent::new("fn main() {}")
        .expect("valid")
        .with_language("rust")
        .expect("non-empty");
    let without = CodeContent::new("fn main() {}").expect("valid");

    assert!(render_block(MessageBlock::Code(with_language)).contains("rust"));
    assert!(!render_block(MessageBlock::Code(without)).contains("rust"));
}

#[test]
fn empty_and_multiline_content_survive() {
    let empty = render_block(MessageBlock::Text(String::new()));
    assert!(
        !empty.is_empty(),
        "an empty block must still occupy the transcript"
    );

    let multiline = render_block(MessageBlock::Text("first\nsecond\nthird".into()));
    for part in ["first", "second", "third"] {
        assert!(multiline.contains(part), "{part} lost from {multiline:?}");
    }
}

#[test]
fn unicode_content_is_not_mangled() {
    let output = render_block(MessageBlock::Text("你好 world 👨‍👩‍👧‍👦".into()));
    assert!(output.contains("你好"));
    assert!(output.contains("world"));
}

#[test]
fn narrow_widths_do_not_drop_content() {
    let message = message_with(
        MessageBlock::Text("alpha beta gamma delta".into()),
        ChatRole::Assistant,
    );
    let element = ChatMessageView::new(&message).into_element();

    for width in [8u16, 12, 20] {
        let output = render_to_string_no_trim(&element, width);
        let joined: String = output.split_whitespace().collect();
        for word in ["alpha", "beta", "gamma", "delta"] {
            assert!(
                joined.contains(word),
                "width {width} dropped {word}: {output:?}"
            );
        }
    }
}

#[test]
fn extended_block_variants_render_their_typed_payloads() {
    let quote = QuoteContent::new("to be").expect("valid");
    assert!(render_block(MessageBlock::Quote(quote)).contains("to be"));

    let link = LinkContent::new("docs", "https://example.invalid/docs").expect("valid");
    let rendered = render_block(MessageBlock::Link(link));
    assert!(rendered.contains("docs"));
    assert!(rendered.contains("example.invalid"));

    let attachment = TerminalAttachmentSummary::new("log.txt", "42 lines")
        .expect("valid")
        .with_media_type("text/plain")
        .expect("non-empty");
    let rendered = render_block(MessageBlock::TerminalAttachmentSummary(attachment));
    assert!(rendered.contains("log.txt"));
    assert!(rendered.contains("text/plain"));
    assert!(rendered.contains("42 lines"));
}

#[test]
fn the_streaming_indicator_is_driven_only_by_the_supplied_frame() {
    let base = message_with(MessageBlock::Text("partial".into()), ChatRole::Assistant);
    let streaming = ChatMessage::try_restore(
        base.id(),
        base.role(),
        MessageStatus::Streaming,
        base.revision(),
        base.blocks().to_vec(),
        base.metadata().clone(),
    )
    .expect("valid");

    let frame_of = |frame: usize| {
        let options = ChatMessageViewOptions::new(
            MessageViewVariant::Compact,
            ThinkingDisclosure::Collapsed,
            std::num::NonZeroUsize::new(5).expect("non-zero"),
            std::num::NonZeroUsize::new(12).expect("non-zero"),
            StreamingIndicatorFrame::new(frame),
            Default::default(),
        );
        render_view(ChatMessageView::new(&streaming).options(options))
    };

    // Deterministic: the same frame always renders the same thing.
    assert_eq!(frame_of(1), frame_of(1));
    assert_ne!(frame_of(1), frame_of(3));
    // And it wraps rather than panicking on a large frame.
    assert_eq!(frame_of(1), frame_of(5));
}

#[test]
fn a_custom_renderer_can_override_a_block_or_decline_it() {
    use rnk::components::chat::{ChatBlockRef, ChatRenderContext, ChatRenderOverride};

    let message = ChatMessage::new(
        MessageId::new(1),
        ChatRole::Assistant,
        vec![
            MessageBlockEntry::new(BlockId::new(1), MessageBlock::Text("replace me".into())),
            MessageBlockEntry::new(BlockId::new(2), MessageBlock::Markdown("keep me".into())),
        ],
    )
    .expect("valid");

    let renderer = |block: ChatBlockRef<'_>, _context: ChatRenderContext<'_>| match block {
        ChatBlockRef::Text(_) => {
            ChatRenderOverride::element(Text::new("custom body").into_element())
        }
        _ => ChatRenderOverride::UseDefault,
    };

    let output = render_view(ChatMessageView::new(&message).renderer(&renderer));

    assert!(output.contains("custom body"));
    assert!(
        !output.contains("replace me"),
        "the override did not replace the default body: {output:?}"
    );
    assert!(
        output.contains("keep me"),
        "a declined block must still take the default path: {output:?}"
    );
}

#[test]
fn every_variant_renders_the_same_content() {
    let message = message_with(MessageBlock::Text("body text".into()), ChatRole::Assistant);

    for variant in [
        MessageViewVariant::Compact,
        MessageViewVariant::Bordered,
        MessageViewVariant::Bubble,
    ] {
        let options = ChatMessageViewOptions::new(
            variant,
            ThinkingDisclosure::Collapsed,
            std::num::NonZeroUsize::new(5).expect("non-zero"),
            std::num::NonZeroUsize::new(12).expect("non-zero"),
            StreamingIndicatorFrame::default(),
            Default::default(),
        );
        let output = render_view(ChatMessageView::new(&message).options(options));
        assert!(
            output.contains("body text"),
            "{variant:?} lost the body: {output:?}"
        );
    }
}

#[test]
fn thinking_covers_every_lifecycle_state() {
    for (status, expected) in [
        (ThinkingStatus::Pending, "pending"),
        (ThinkingStatus::Streaming, "streaming"),
        (ThinkingStatus::Complete, "complete"),
        (ThinkingStatus::Cancelled, "cancelled"),
        (
            ThinkingStatus::Failed(cause("context lost")),
            "context lost",
        ),
    ] {
        let content = ThinkingContent::new(ThinkingId::new("t1").expect("non-empty"), "reasoning")
            .with_status(status.clone());
        let output = render_block(MessageBlock::Thinking(content));
        assert!(
            output.contains(expected),
            "{status:?} did not render {expected:?}: {output:?}"
        );
    }
}
