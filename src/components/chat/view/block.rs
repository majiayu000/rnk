//! Typed default views for every chat block.
//!
//! Each view reads the GH-62 model through its borrowed accessors only. No
//! block renders provider JSON, executes anything, or emits raw ANSI, and a
//! failed lifecycle always shows the typed cause it was given rather than a
//! generic word.
//!
//! Previews (thinking, tool results) are cut on rows produced by the shared
//! [`TextFlow`], not by `str::lines()`. The two disagree: `lines()` treats a
//! lone CR as ordinary text, so a preview built from it would claim a row count
//! the renderer does not paint.

use std::num::NonZeroUsize;

use super::{ChatMessageViewOptions, StreamingIndicatorFrame, ThinkingDisclosure};
use crate::components::chat::{
    CodeContent, DiffContent, ErrorContent, LinkContent, MessageBlock, QuoteContent,
    TerminalAttachmentSummary, ThinkingContent, ThinkingStatus, ToolArgument, ToolCallContent,
    ToolCallStatus, ToolResultContent, ToolResultStatus, TypedValue,
};
use crate::components::{Box as BoxView, Markdown, Text};
use crate::core::{Color, Element, FlexDirection, Style, TextWrap};
use crate::layout::{TextFlow, TextFlowInput, TextFlowOptions, TextFlowSourceKind};

/// Width used to split source into logical rows for a preview.
///
/// A preview counts source lines, not painted columns — the message view has
/// not resolved a width yet, and re-flowing at the real width is the renderer's
/// job. A width this large means only hard breaks split rows.
const PREVIEW_ROW_WIDTH: usize = usize::MAX;

/// Render one block's default body.
pub(super) fn render_block(block: &MessageBlock, options: &ChatMessageViewOptions) -> Element {
    match block {
        MessageBlock::Text(content) => text_view(content),
        MessageBlock::Markdown(content) => markdown_view(content),
        MessageBlock::Code(content) => code_view(content),
        MessageBlock::Thinking(content) => thinking_view(content, options),
        MessageBlock::ToolCall(content) => tool_call_view(content),
        MessageBlock::ToolResult(content) => tool_result_view(content, options),
        MessageBlock::Error(content) => error_view(content),
        MessageBlock::Diff(content) => diff_view(content),
        MessageBlock::Quote(content) => quote_view(content),
        MessageBlock::Link(content) => link_view(content),
        MessageBlock::TerminalAttachmentSummary(content) => attachment_view(content),
    }
}

/// Plain text, exactly as given.
///
/// Empty content still produces a node, so the block keeps its place among its
/// siblings instead of silently collapsing.
fn text_view(content: &str) -> Element {
    Text::new(content.to_owned())
        .wrap(TextWrap::Wrap)
        .into_element()
}

fn markdown_view(content: &str) -> Element {
    Markdown::new(content.to_owned()).into_element()
}

/// Code, with its language shown only when one was supplied.
fn code_view(content: &CodeContent) -> Element {
    let mut view = column();
    if let Some(language) = content.language() {
        view = view.child(dim_label(language));
    }
    view.child(
        Text::new(content.content().to_owned())
            .wrap(TextWrap::Wrap)
            .color(Color::Cyan),
    )
    .into_element()
}

fn diff_view(content: &DiffContent) -> Element {
    let mut view = column();
    if let Some(language) = content.language() {
        view = view.child(dim_label(language));
    }
    view.child(Text::new(content.content().to_owned()).wrap(TextWrap::Wrap))
        .into_element()
}

fn quote_view(content: &QuoteContent) -> Element {
    let mut view = column().child(Text::new(content.content().to_owned()).wrap(TextWrap::Wrap));
    // Attribution is optional; an absent one leaves nothing behind rather than
    // a placeholder that would read as real data.
    if let Some(attribution) = content.attribution() {
        view = view.child(dim_label(attribution));
    }
    view.into_element()
}

/// An inert link: label and target as text, never activated.
fn link_view(content: &LinkContent) -> Element {
    column()
        .child(Text::new(content.label().to_owned()).wrap(TextWrap::Wrap))
        .child(dim_label(content.target()))
        .into_element()
}

/// An inert attachment summary: no file is opened or read.
fn attachment_view(content: &TerminalAttachmentSummary) -> Element {
    let mut header = content.name().to_owned();
    if let Some(media_type) = content.media_type() {
        header.push_str(" (");
        header.push_str(media_type);
        header.push(')');
    }
    column()
        .child(Text::new(header).bold())
        .child(Text::new(content.summary().to_owned()).wrap(TextWrap::Wrap))
        .into_element()
}

/// Thinking, collapsed to a preview or shown whole, per the caller.
///
/// Disclosure is controlled: this view never decides on its own, so the same
/// content and options always produce the same rows.
fn thinking_view(content: &ThinkingContent, options: &ChatMessageViewOptions) -> Element {
    let status = thinking_status_label(content.status());
    let mut view = column().child(dim_label(&format!("thinking · {status}")));

    let preview = match options.thinking_disclosure() {
        ThinkingDisclosure::Expanded => Preview::whole(content.content()),
        ThinkingDisclosure::Collapsed => {
            Preview::limited(content.content(), options.thinking_preview_lines())
        }
    };

    view = view.child(Text::new(preview.text.clone()).wrap(TextWrap::Wrap));
    if let Some(marker) = preview.hidden_marker() {
        view = view.child(dim_label(&marker));
    }
    view.into_element()
}

/// A tool call: name, typed arguments, and lifecycle.
fn tool_call_view(content: &ToolCallContent) -> Element {
    let mut view = column()
        .child(Text::new(content.name().to_owned()).bold())
        .child(dim_label(&format!(
            "{} · {}",
            content.call_id().as_str(),
            tool_call_status_label(content.status())
        )));

    // Source order is the model's order; it is not sorted or deduplicated here.
    for argument in content.arguments() {
        view = view.child(argument_view(argument));
    }

    view.into_element()
}

/// A tool result: output preview, lifecycle, and whether anything was cut.
fn tool_result_view(content: &ToolResultContent, options: &ChatMessageViewOptions) -> Element {
    let preview = Preview::limited(content.output(), options.tool_result_preview_lines());

    let mut view = column()
        .child(dim_label(&format!(
            "{} · {}",
            content.call_id().as_str(),
            tool_result_status_label(content.status())
        )))
        .child(Text::new(preview.text.clone()).wrap(TextWrap::Wrap));

    if let Some(marker) = preview.hidden_marker() {
        view = view.child(dim_label(&marker));
    }

    view.into_element()
}

/// An error, always styled as one.
///
/// This never falls back to the plain-text view: an error that reads like
/// ordinary assistant output is worse than no output at all.
fn error_view(content: &ErrorContent) -> Element {
    let mut view = column().child(
        Text::new(content.message().to_owned())
            .wrap(TextWrap::Wrap)
            .color(Color::Red)
            .bold(),
    );

    if let Some(source) = content.source() {
        view = view.child(
            Text::new(source.as_str().to_owned())
                .wrap(TextWrap::Wrap)
                .color(Color::Red),
        );
    }

    view.into_element()
}

/// A streaming indicator advanced only by the caller's frame.
///
/// Nothing here reads a clock, so a rendered frame is reproducible and tests do
/// not depend on timing.
pub(super) fn streaming_indicator(frame: StreamingIndicatorFrame) -> Element {
    const FRAMES: [&str; 4] = ["   ", ".  ", ".. ", "..."];
    let glyph = FRAMES[frame.get() % FRAMES.len()];
    dim_label(glyph).into_element()
}

/// One typed argument, rendered as `name: value` without any JSON.
fn argument_view(argument: &ToolArgument) -> Element {
    BoxView::new()
        .child(dim_label(&format!("{}: ", argument.name())))
        .child(Text::new(render_typed_value(argument.value())).wrap(TextWrap::Wrap))
        .into_element()
}

/// Render a typed value tree as text.
///
/// The closed `TypedValue` set is walked exhaustively. Serialising to JSON
/// would reintroduce a provider-shaped, untyped representation at the very
/// boundary this model exists to keep typed.
fn render_typed_value(value: &TypedValue) -> String {
    match value {
        TypedValue::Null => "null".to_owned(),
        TypedValue::Bool(flag) => flag.to_string(),
        TypedValue::Integer(number) => number.to_string(),
        TypedValue::Decimal(decimal) => decimal.as_str().to_owned(),
        // Quoted, so an empty string is visible rather than reading as a
        // missing value.
        TypedValue::String(text) => format!("{text:?}"),
        TypedValue::List(values) => {
            let rendered: Vec<String> = values.iter().map(render_typed_value).collect();
            format!("[{}]", rendered.join(", "))
        }
        TypedValue::Object(fields) => {
            let rendered: Vec<String> = fields
                .iter()
                .map(|field| format!("{}: {}", field.name(), render_typed_value(field.value())))
                .collect();
            format!("{{{}}}", rendered.join(", "))
        }
    }
}

/// A source preview, and how much of it was withheld.
struct Preview {
    text: String,
    hidden_rows: usize,
}

impl Preview {
    /// The whole source, with nothing withheld.
    fn whole(source: &str) -> Self {
        Self {
            text: source.to_owned(),
            hidden_rows: 0,
        }
    }

    /// At most `limit` logical rows of `source`.
    ///
    /// Rows come from the shared text flow, so a preview describes the same
    /// rows the renderer would paint for that source.
    fn limited(source: &str, limit: NonZeroUsize) -> Self {
        let Some(rows) = logical_rows(source) else {
            // The flow could not be built. Showing the source unchanged is
            // honest; claiming a row count we could not compute is not.
            return Self::whole(source);
        };

        let limit = limit.get();
        if rows.len() <= limit {
            return Self::whole(source);
        }

        Self {
            text: rows[..limit].join("\n"),
            hidden_rows: rows.len() - limit,
        }
    }

    /// How the withheld rows are declared, if any were.
    ///
    /// Truncation is always stated. Content that vanishes with no marker is
    /// indistinguishable from content that was never there.
    fn hidden_marker(&self) -> Option<String> {
        match self.hidden_rows {
            0 => None,
            1 => Some("… 1 more line".to_owned()),
            count => Some(format!("… {count} more lines")),
        }
    }
}

/// Split `source` into logical rows using the shared flow.
///
/// Returns `None` if the flow rejects the input, so callers decide what to do
/// rather than receiving a silently wrong row set.
fn logical_rows(source: &str) -> Option<Vec<String>> {
    let input = TextFlowInput::plain(
        source.to_owned(),
        TextFlowSourceKind::Exact,
        Style::default(),
    );
    let options = TextFlowOptions::new(PREVIEW_ROW_WIDTH, TextWrap::Wrap);
    let flow = TextFlow::try_build(&input, &options).ok()?;
    Some(flow.rows().to_vec())
}

fn thinking_status_label(status: &ThinkingStatus) -> String {
    match status {
        ThinkingStatus::Pending => "pending".to_owned(),
        ThinkingStatus::Streaming => "streaming".to_owned(),
        ThinkingStatus::Complete => "complete".to_owned(),
        ThinkingStatus::Cancelled => "cancelled".to_owned(),
        // The typed cause is the whole point of a failed state; dropping it
        // would leave no way to tell one failure from another.
        ThinkingStatus::Failed(cause) => format!("failed: {}", cause.as_str()),
    }
}

fn tool_call_status_label(status: &ToolCallStatus) -> String {
    match status {
        ToolCallStatus::Pending => "pending".to_owned(),
        ToolCallStatus::Running => "running".to_owned(),
        ToolCallStatus::Succeeded => "succeeded".to_owned(),
        ToolCallStatus::Cancelled => "cancelled".to_owned(),
        ToolCallStatus::Failed(cause) => format!("failed: {}", cause.as_str()),
    }
}

fn tool_result_status_label(status: &ToolResultStatus) -> String {
    match status {
        ToolResultStatus::Pending => "pending".to_owned(),
        ToolResultStatus::Streaming => "streaming".to_owned(),
        ToolResultStatus::Complete => "complete".to_owned(),
        ToolResultStatus::Cancelled => "cancelled".to_owned(),
        ToolResultStatus::Failed(cause) => format!("failed: {}", cause.as_str()),
    }
}

fn column() -> BoxView {
    BoxView::new().flex_direction(FlexDirection::Column)
}

fn dim_label(text: &str) -> Text {
    Text::new(text.to_owned()).dim()
}
