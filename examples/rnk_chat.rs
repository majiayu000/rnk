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
    MessageShellMeasureConfig, MessageStructuralSegment, MessageStructureSlotKey,
    MessageVariantKey, RowOffset, ViewportRows, try_measure_composite,
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

/// Columns a message bubble's content is laid out in.
const BUBBLE_WIDTH: u16 = 60;
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
        let entries = entries_for(&messages);
        let rows = MessageListState::try_new(
            &entries,
            BUBBLE_WIDTH,
            ViewportRows::new(1),
            MEASUREMENT_CACHE,
            measure,
        )
        .expect("every seeded message measures");
        let shell = FullscreenChatShell::try_new(
            rows,
            ChatComposerState::new(),
            terminal_width.max(1),
            shell_height(terminal_height),
            STATUS_ROWS,
        )
        .expect("the shell height is clamped to its supported minimum");

        Self { messages, shell }
    }

    fn resize(&mut self, terminal_width: u16, terminal_height: u16) {
        let width = terminal_width.max(1);
        let height = shell_height(terminal_height);
        if self.shell.layout().width() == width && self.shell.layout().height() == height {
            return;
        }
        let _ = self.shell.try_resize(width, height);
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
                    let _ = self.shell.composer_mut().acknowledge_success(token);
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
        let entry = entry_for(
            self.messages.len() - 1,
            self.messages.last().expect("just pushed"),
        );
        let transcript = self.shell.transcript_mut();
        if transcript
            .try_append(transcript.revision(), std::slice::from_ref(&entry), measure)
            .is_err()
        {
            let viewport_rows = transcript.viewport_rows();
            *transcript = MessageListState::try_new(
                &entries_for(&self.messages),
                BUBBLE_WIDTH,
                viewport_rows,
                MEASUREMENT_CACHE,
                measure,
            )
            .expect("rebuilt transcript measures");
        }
    }

    fn scroll_by(&mut self, delta: i64) {
        let transcript = self.shell.transcript_mut();
        let current = transcript.scroll_offset().get() as i64;
        let target = current.saturating_add(delta).max(0) as u64;
        let _ = transcript.try_scroll_to(transcript.revision(), RowOffset::new(target));
    }
}

fn shell_height(terminal_height: u16) -> u16 {
    terminal_height.saturating_sub(HEADER_ROWS).max(3)
}

fn entry_for(index: usize, message: &ChatMessage) -> MessageListEntry {
    MessageListEntry::new(
        MessageId::new(index as u64 + 1),
        MessageRevision::INITIAL,
        MessageVariantKey::new(role_key(message.role)),
        MessageExpansionKey::new(0),
        measure_config(message).expect("message config matches its renderer"),
    )
}

fn entries_for(messages: &[ChatMessage]) -> Vec<MessageListEntry> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| entry_for(index, message))
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

fn measure_config(message: &ChatMessage) -> Result<MessageCompositeMeasureConfig, String> {
    let shell = MessageShellMeasureConfig::try_new(
        BUBBLE_WIDTH,
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
        options: TextFlowOptions::new(usize::from(BUBBLE_WIDTH), TextWrap::Wrap),
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

    let viewport_changed = transcript.with(|state| {
        state.layout().width() != terminal_width.max(1)
            || state.layout().height() != shell_height(terminal_height)
    });
    if viewport_changed {
        transcript.update(|state| state.resize(terminal_width, terminal_height));
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

fn message_list(transcript: &Transcript) -> Element {
    let layout = transcript.layout();
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
            Ok(render_message_slice(message, slice.message_rows.clone()))
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

fn render_message_slice(message: &ChatMessage, rows: core::ops::Range<u64>) -> Element {
    let rendered = rendered_rows(message);
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
        container = container.child(render_message_row(message.role, row));
    }
    container.into_element()
}

#[derive(Clone)]
enum RenderedRow {
    Header(String),
    Body(String),
}

fn rendered_rows(message: &ChatMessage) -> Vec<RenderedRow> {
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
        &TextFlowOptions::new(usize::from(BUBBLE_WIDTH), TextWrap::Wrap),
    );
    match flow {
        Ok(flow) => rows.extend(flow.rows().iter().cloned().map(RenderedRow::Body)),
        Err(error) => rows.push(RenderedRow::Body(format!("message unavailable: {error}"))),
    }
    rows
}

fn render_message_row(role: ChatRole, row: RenderedRow) -> Element {
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
        .width(i32::from(BUBBLE_WIDTH))
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
