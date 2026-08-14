//! rnk-chat: a fullscreen terminal chat client built with public chat pieces.
//!
//! The example owns only application data and deterministic replies. The
//! fullscreen shell owns the transcript/composer/status layout, the message list
//! owns row-based scrolling, and the composer owns editing and submission.
//!
//! Run with: cargo run --example rnk_chat

use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageList,
    MessageListEntry, MessageListState, MessageMeasureOutcome, MessageMeasureRequest,
    MessageResizeConfigOutcome, MessageShellMeasureConfig, MessageStructuralSegment,
    MessageStructureSlotKey, MessageVariantKey, RowOffset, ViewportRows, try_measure_composite,
};
use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, ChatRole, ComposerProjection, FullscreenChatShell,
    FullscreenKeyOutcome, FullscreenLayout, MessageId, MessageRevision,
};
use rnk::core::TextWrap;
use rnk::hooks::{Key, use_window_size};
use rnk::layout::text_flow::{
    TextFlow, TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};
use rnk::prelude::*;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Maximum columns a message bubble occupies on a wide terminal.
const MAX_BUBBLE_WIDTH: u16 = 60;
const HEADER_ROWS: u16 = 1;
const STATUS_ROWS: u16 = 1;
const PAGE_ROWS: u64 = 5;
const MEASUREMENT_CACHE: usize = 512;

fn main() -> std::io::Result<()> {
    render(app).fullscreen().run()
}

#[derive(Clone)]
struct ChatMessage {
    role: ChatRole,
    content: String,
    timestamp: String,
}

#[derive(Clone)]
struct Transcript {
    messages: Vec<ChatMessage>,
    shell: FullscreenChatShell,
}

impl Transcript {
    fn new(messages: Vec<ChatMessage>, terminal_width: u16, terminal_height: u16) -> Self {
        let bubble_width = bubble_width(terminal_width);
        let entries = entries_for(&messages, bubble_width);
        let rows = MessageListState::try_new(
            &entries,
            bubble_width,
            ViewportRows::new(1),
            MEASUREMENT_CACHE,
            measure,
        )
        .expect("every seeded message measures");
        let shell = FullscreenChatShell::try_new(
            rows,
            ChatComposerState::new(),
            terminal_width.max(1),
            shell_height(terminal_height).unwrap_or(3),
            STATUS_ROWS,
        )
        .expect("the internal fallback shell uses the supported minimum");

        Self { messages, shell }
    }

    fn resized(&self, terminal_width: u16, terminal_height: u16) -> Result<Self, String> {
        if terminal_width < 3 {
            return Err("terminal is too narrow; at least three columns are required".to_string());
        }
        let width = terminal_width.max(1);
        let height = shell_height(terminal_height).ok_or_else(|| {
            "terminal is too short for header, transcript, composer, and status".to_string()
        })?;
        if self.shell.layout().width() == width && self.shell.layout().height() == height {
            return Ok(self.clone());
        }

        let mut candidate = self.clone();
        candidate
            .shell
            .try_resize(width, height)
            .map_err(|error| error.to_string())?;

        let message_width = bubble_width(width);
        let viewport_rows =
            ViewportRows::new(u64::from(candidate.shell.layout().transcript().rows()));
        let messages = &candidate.messages;
        let mut transcript = candidate.shell.transcript().clone();
        transcript
            .try_resize(
                transcript.revision(),
                message_width,
                viewport_rows,
                |request| match messages
                    .get(request.message_index)
                    .ok_or_else(|| "resize referenced an unknown message".to_string())
                    .and_then(|message| measure_config(message, request.new_width))
                {
                    Ok(config) => MessageResizeConfigOutcome::Rebuilt(config),
                    Err(error) => MessageResizeConfigOutcome::Failed(error),
                },
                measure,
            )
            .map_err(|error| format!("message resize failed: {error:?}"))?;
        candidate
            .shell
            .try_replace_transcript(transcript)
            .map_err(|error| error.to_string())?;

        Ok(candidate)
    }

    fn layout(&self) -> FullscreenLayout {
        self.shell.layout()
    }

    fn handle_input(&mut self, input: &str, key: &Key) -> Option<String> {
        if key.page_up {
            self.scroll_by(-(PAGE_ROWS as i64));
            return None;
        }
        if key.page_down {
            self.scroll_by(PAGE_ROWS as i64);
            return None;
        }

        match self
            .shell
            .handle_key(&ChatComposerKeyMap::new(), input, key)
        {
            Ok(FullscreenKeyOutcome::Submitted(text)) => {
                self.push(ChatMessage {
                    role: ChatRole::User,
                    content: text.clone(),
                    timestamp: current_time(),
                });
                if let Some(token) = self
                    .shell
                    .composer()
                    .pending_submission()
                    .map(|pending| pending.token())
                {
                    if let Err(error) = self.shell.acknowledge_submission_success(token) {
                        self.push(ChatMessage {
                            role: ChatRole::System,
                            content: format!("submission acknowledgement failed: {error}"),
                            timestamp: current_time(),
                        });
                        return None;
                    }
                }
                Some(text)
            }
            Ok(_) => None,
            Err(error) => {
                self.push(ChatMessage {
                    role: ChatRole::System,
                    content: format!("layout unavailable: {error}"),
                    timestamp: current_time(),
                });
                None
            }
        }
    }

    fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
        let message_width = self.shell.transcript().width();
        let entry = entry_for(
            self.messages.len() - 1,
            self.messages.last().expect("just pushed"),
            message_width,
        );
        let mut transcript = self.shell.transcript().clone();
        if transcript
            .try_append(transcript.revision(), std::slice::from_ref(&entry), measure)
            .is_err()
        {
            let viewport_rows = transcript.viewport_rows();
            transcript = MessageListState::try_new(
                &entries_for(&self.messages, message_width),
                message_width,
                viewport_rows,
                MEASUREMENT_CACHE,
                measure,
            )
            .expect("rebuilt transcript measures");
        }
        self.shell
            .try_replace_transcript(transcript)
            .expect("the replacement viewport matches the shell");
    }

    fn scroll_by(&mut self, delta: i64) {
        let mut transcript = self.shell.transcript().clone();
        let current = transcript.scroll_offset().get() as i64;
        let target = current.saturating_add(delta).max(0) as u64;
        if transcript
            .try_scroll_to(transcript.revision(), RowOffset::new(target))
            .is_ok()
        {
            self.shell
                .try_replace_transcript(transcript)
                .expect("scrolling cannot invalidate the shell viewport");
        }
    }
}

fn shell_height(terminal_height: u16) -> Option<u16> {
    terminal_height
        .checked_sub(HEADER_ROWS)
        .filter(|height| *height >= 3)
}

fn bubble_width(terminal_width: u16) -> u16 {
    terminal_width.saturating_sub(2).clamp(1, MAX_BUBBLE_WIDTH)
}

fn entry_for(index: usize, message: &ChatMessage, width: u16) -> MessageListEntry {
    MessageListEntry::new(
        MessageId::new(index as u64 + 1),
        MessageRevision::INITIAL,
        MessageVariantKey::new(role_key(message.role)),
        MessageExpansionKey::new(0),
        measure_config(message, width).expect("message config matches its renderer"),
    )
}

fn entries_for(messages: &[ChatMessage], width: u16) -> Vec<MessageListEntry> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| entry_for(index, message, width))
        .collect()
}

const fn role_key(role: ChatRole) -> u64 {
    match role {
        ChatRole::User => 0,
        ChatRole::Assistant => 1,
        ChatRole::System => 2,
        ChatRole::Tool => 3,
    }
}

fn measure_config(
    message: &ChatMessage,
    width: u16,
) -> Result<MessageCompositeMeasureConfig, String> {
    let shell = MessageShellMeasureConfig::try_new(
        width,
        HorizontalInsets::new(0, 0),
        vec![MessageStructuralSegment::new(
            MessageStructureSlotKey::new(0),
            RowOffset::new(1),
        )],
    )
    .map_err(|error| error.to_string())?;
    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain(
            message.content.clone(),
            TextFlowSourceKind::Exact,
            Style::default(),
        ),
        options: TextFlowOptions::new(usize::from(width), TextWrap::Wrap),
    };

    MessageCompositeMeasureConfig::try_new(vec![identity], shell).map_err(|error| error.to_string())
}

fn measure(request: MessageMeasureRequest<'_>) -> MessageMeasureOutcome<String, ()> {
    match try_measure_composite(request) {
        Ok(rows) => MessageMeasureOutcome::Measured(rows),
        Err(error) => MessageMeasureOutcome::Failed(error.to_string()),
    }
}

fn app() -> Element {
    let (terminal_width, terminal_height) = use_window_size();
    let transcript =
        use_signal(|| Transcript::new(initial_messages(), terminal_width, terminal_height));
    let is_typing = use_signal(|| false);
    let app = use_app();

    let transcript_input = transcript.clone();
    let is_typing_input = is_typing.clone();

    use_input(move |input, key| {
        if key.escape || (key.ctrl && input == "c") {
            app.exit();
            return;
        }

        let mut submitted = None;
        transcript_input.update(|state| {
            submitted = state.handle_input(input, key);
        });

        if let Some(prompt) = submitted {
            is_typing_input.set(true);
            let transcript_reply = transcript_input.clone();
            let typing_reply = is_typing_input.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(800));
                typing_reply.set(false);
                transcript_reply.update(|transcript| {
                    transcript.push(ChatMessage {
                        role: ChatRole::Assistant,
                        content: generate_response(&prompt),
                        timestamp: current_time(),
                    });
                });
            });
        }
    });

    if terminal_width < 3 {
        return terminal_too_small(terminal_width, terminal_height);
    }
    let Some(desired_height) = shell_height(terminal_height) else {
        return terminal_too_small(terminal_width, terminal_height);
    };
    let viewport_changed = transcript.with(|state| {
        state.layout().width() != terminal_width.max(1) || state.layout().height() != desired_height
    });
    if viewport_changed {
        match transcript.with(|state| state.resized(terminal_width, terminal_height)) {
            Ok(resized) => transcript.set(resized),
            Err(error) => return terminal_error(terminal_width, error),
        }
    }

    let typing = is_typing.get();
    let (transcript_view, composer_view, status_view) = transcript.with(|state| {
        (
            message_list(state),
            input_area(state.shell.composer(), state.layout().width()),
            footer(typing, state.layout().status().rows()),
        )
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .children(vec![header(), transcript_view, composer_view, status_view])
        .into_element()
}

fn header() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(JustifyContent::SpaceBetween)
        .height(i32::from(HEADER_ROWS))
        .padding_x(1.0)
        .background(Color::Ansi256(236))
        .children(vec![
            Text::new("rnk-chat")
                .color(Color::Cyan)
                .bold()
                .into_element(),
            Text::new("AI Assistant").color(Color::Green).into_element(),
        ])
        .into_element()
}

fn terminal_too_small(width: u16, height: u16) -> Element {
    terminal_error(
        width,
        format!(
            "terminal {width}x{height} is too small; at least 3 columns by 4 rows are required"
        ),
    )
}

fn terminal_error(width: u16, message: String) -> Element {
    Box::new()
        .width(i32::from(width.max(1)))
        .height(1)
        .child(Text::new(message).color(Color::Yellow).into_element())
        .into_element()
}

fn message_list(transcript: &Transcript) -> Element {
    let layout = transcript.layout();
    let message_width = transcript.shell.transcript().width();
    if transcript.messages.is_empty() {
        return Box::new()
            .height(i32::from(layout.transcript().rows()))
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center)
            .child(
                Text::new("Start a conversation...")
                    .color(Color::BrightBlack)
                    .into_element(),
            )
            .into_element();
    }

    let body = MessageList::new(transcript.shell.transcript()).try_into_element(
        |_entry, _key, slice| -> Result<Element, String> {
            let message = transcript
                .messages
                .get(slice.message_index)
                .ok_or_else(|| "visible slice pointed outside the transcript".to_string())?;
            Ok(render_message_slice(
                message,
                slice.message_rows.clone(),
                message_width,
            ))
        },
    );

    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(i32::from(layout.transcript().rows()))
        .padding_x(1.0)
        .child(match body {
            Ok(element) => element,
            Err(error) => Text::new(format!("transcript unavailable: {error}"))
                .color(Color::Red)
                .into_element(),
        })
        .into_element()
}

fn render_message_slice(message: &ChatMessage, rows: core::ops::Range<u64>, width: u16) -> Element {
    let rendered = rendered_rows(message, width);
    let start = usize::try_from(rows.start).unwrap_or(usize::MAX);
    let end = usize::try_from(rows.end)
        .unwrap_or(usize::MAX)
        .min(rendered.len());

    let mut container = Box::new().flex_direction(FlexDirection::Column);
    for row in rendered
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
    {
        container = container.child(render_message_row(message.role, row, width));
    }
    container.into_element()
}

#[derive(Clone)]
enum RenderedRow {
    Header(String),
    Body(String),
}

fn rendered_rows(message: &ChatMessage, width: u16) -> Vec<RenderedRow> {
    let mut rows = vec![RenderedRow::Header(format!(
        "{} {}",
        role_label(message.role),
        message.timestamp
    ))];
    let flow = TextFlow::try_build(
        &TextFlowInput::plain(
            message.content.clone(),
            TextFlowSourceKind::Exact,
            Style::default(),
        ),
        &TextFlowOptions::new(usize::from(width), TextWrap::Wrap),
    );
    match flow {
        Ok(flow) => rows.extend(flow.rows().iter().cloned().map(RenderedRow::Body)),
        Err(error) => rows.push(RenderedRow::Body(format!("message unavailable: {error}"))),
    }
    rows
}

fn render_message_row(role: ChatRole, row: RenderedRow, width: u16) -> Element {
    let (align, color) = match role {
        ChatRole::User => (JustifyContent::FlexEnd, Color::White),
        ChatRole::Assistant => (JustifyContent::FlexStart, Color::Reset),
        ChatRole::System => (JustifyContent::Center, Color::BrightBlack),
        ChatRole::Tool => (JustifyContent::FlexStart, Color::Magenta),
    };
    let text = match row {
        RenderedRow::Header(text) => Text::new(text)
            .color(role_color(role))
            .bold()
            .into_element(),
        RenderedRow::Body(text) => Text::new(text).color(color).into_element(),
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(align)
        .width(i32::from(width))
        .child(text)
        .into_element()
}

fn role_label(role: ChatRole) -> &'static str {
    match role {
        ChatRole::User => "You",
        ChatRole::Assistant => "Assistant",
        ChatRole::System => "System",
        ChatRole::Tool => "Tool",
    }
}

fn role_color(role: ChatRole) -> Color {
    match role {
        ChatRole::User => Color::Blue,
        ChatRole::Assistant => Color::Green,
        ChatRole::System => Color::Yellow,
        ChatRole::Tool => Color::Magenta,
    }
}

fn input_area(composer: &ChatComposerState, width: u16) -> Element {
    let projection = ComposerProjection::build(composer, width.max(1));
    let first_row = projection.scroll_offset();
    let mut lines = Box::new().flex_direction(FlexDirection::Column);

    for (offset, row) in projection.visible_slice().iter().enumerate() {
        let absolute_row = first_row + offset;
        let cursor_column =
            (absolute_row == projection.cursor_row()).then(|| projection.cursor_column());
        lines = lines.child(input_line(row, cursor_column));
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(i32::try_from(projection.visible_rows()).unwrap_or(i32::MAX))
        .child(lines.into_element())
        .into_element()
}

fn input_line(row: &str, cursor_column: Option<usize>) -> Element {
    let mut line = Box::new().flex_direction(FlexDirection::Row);

    if row.is_empty() {
        line = line.child(
            Text::new("Type a message...")
                .color(Color::BrightBlack)
                .into_element(),
        );
    }

    let mut column = 0usize;
    let mut painted_cursor = false;
    for cluster in row.graphemes(true) {
        let at_cursor = cursor_column == Some(column);
        painted_cursor |= at_cursor;
        line = line.child(cell(cluster, at_cursor));
        column += UnicodeWidthStr::width(cluster).max(1);
    }
    if let Some(target) = cursor_column
        && !painted_cursor
        && target >= column
    {
        line = line.child(cell(" ", true));
    }

    line.into_element()
}

fn cell(cluster: &str, at_cursor: bool) -> Element {
    let text = Text::new(cluster.to_string());
    if at_cursor {
        text.color(Color::Black)
            .background(Color::Cyan)
            .into_element()
    } else {
        text.color(Color::White).into_element()
    }
}

fn footer(is_typing: bool, rows: u16) -> Element {
    let status = if is_typing {
        "Assistant is typing..."
    } else {
        "Enter Send | PgUp/PgDn Scroll | Esc Exit"
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .height(i32::from(rows))
        .padding_x(1.0)
        .background(Color::Ansi256(236))
        .child(Text::new(status).dim().into_element())
        .into_element()
}

fn current_time() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    format!("{hours:02}:{minutes:02}")
}

fn generate_response(input: &str) -> String {
    let input_lower = input.to_lowercase();

    if input_lower.contains("hello") || input_lower.contains("hi") {
        "Hello! How can I help you today?".to_string()
    } else if input_lower.contains("rnk") {
        "rnk is a React-like terminal UI framework for Rust! It features declarative components, hooks, and flexbox layout.".to_string()
    } else if input_lower.contains("help") {
        "I'm here to help! You can ask me about rnk, Rust, or just chat.".to_string()
    } else if input_lower.contains("feature") {
        "rnk has 45+ components, animation system, chainable styles, and more! Check out the examples.".to_string()
    } else if input_lower.contains("thank") {
        "You're welcome! Let me know if you need anything else.".to_string()
    } else {
        let preview: String = input.graphemes(true).take(30).collect();
        format!("I received your message: \"{preview}\". How can I assist you further?")
    }
}

fn initial_messages() -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: ChatRole::System,
            content: "Welcome to rnk-chat! This is a demo of rnk's chat UI capabilities."
                .to_string(),
            timestamp: current_time(),
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "Hi! I'm an AI assistant. How can I help you today?".to_string(),
            timestamp: current_time(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(content: &str) -> ChatMessage {
        ChatMessage {
            role: ChatRole::Assistant,
            content: content.to_string(),
            timestamp: "12:34".to_string(),
        }
    }

    #[test]
    fn partial_slice_paints_only_the_requested_message_rows() {
        let element = render_message_slice(&message("first\nsecond\nthird"), 2..4, 20);
        let rendered = rnk::render_to_string(&element, 20);

        assert!(!rendered.contains("Assistant 12:34"));
        assert!(!rendered.contains("first"));
        assert!(rendered.contains("second"));
        assert!(rendered.contains("third"));
    }

    #[test]
    fn narrow_resize_remeasures_messages_and_preserves_region_agreement() {
        let transcript = Transcript::new(
            vec![message(
                "a message long enough to wrap after the terminal narrows",
            )],
            80,
            24,
        );
        let rows_before = transcript
            .shell
            .transcript()
            .message_rows(MessageId::new(1))
            .expect("known message")
            .get();

        let resized = transcript.resized(12, 12).expect("supported terminal");
        let rows_after = resized
            .shell
            .transcript()
            .message_rows(MessageId::new(1))
            .expect("known message")
            .get();

        assert_eq!(resized.shell.transcript().width(), bubble_width(12));
        assert!(rows_after > rows_before);
        assert_eq!(
            resized.shell.transcript().viewport_rows(),
            ViewportRows::new(u64::from(resized.layout().transcript().rows()))
        );
        assert!(transcript.resized(12, 3).is_err());
        assert!(transcript.resized(2, 12).is_err());
    }
}
