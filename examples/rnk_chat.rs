//! rnk-chat: a terminal chat client built with rnk.
//!
//! Two things in here used to be hand-rolled and are now library concerns,
//! because both were wrong in ways that only show up with real input.
//!
//! **Scrolling is by row, not by message.** This example used to page with
//! `.skip(offset).take(12)`, which treats every message as one unit. A wrapped
//! paragraph is four rows and an acknowledgement is one, so paging by count
//! scrolls a different distance every time and never lands where the reader
//! expects. `MessageListState` indexes the transcript by the rows its messages
//! actually occupy, and `visible_range()` reports which rows of which messages
//! fall inside the viewport — including the partly visible ones at each edge.
//!
//! **The draft is a `ChatComposerState`, not a `String`.** Backspace used to be
//! `String::pop`, which removes one `char`; a `char` is not a user-perceived
//! character, so deleting from an emoji built out of a ZWJ sequence took a piece
//! off the end and left something else behind. The composer deletes by grapheme
//! cluster and reports its cursor in terminal cells, which is the only unit that
//! puts a cursor in the right column after a CJK character.
//!
//! Run with: cargo run --example rnk_chat

use rnk::components::InteractionOutcome;
use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListEntry,
    MessageListState, MessageMeasureOutcome, MessageMeasureRequest, MessageRows,
    MessageShellMeasureConfig, MessageVariantKey, RowOffset, ViewportRows,
};
use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, ComposerProjection, MessageId, MessageRevision,
    handle_key,
};
use rnk::core::TextWrap;
use rnk::hooks::use_window_size;
use rnk::layout::text_flow::{
    TextFlow, TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};
use rnk::prelude::*;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Columns a message bubble's content is laid out in.
const BUBBLE_WIDTH: u16 = 60;

/// Rows of chrome above and below the transcript: header, input box, footer.
const CHROME_ROWS: u16 = 8;

/// Rows a page-up or page-down moves.
const PAGE_ROWS: u64 = 5;

const MEASUREMENT_CACHE: usize = 512;

fn main() -> std::io::Result<()> {
    render(app).fullscreen().run()
}

#[derive(Clone)]
struct ChatMessage {
    role: Role,
    content: String,
    timestamp: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
    System,
}

/// The conversation, plus the row index that decides what is on screen.
///
/// Two fields rather than one because they answer different questions.
/// `MessageListState` is a row index — it knows how tall each message is and
/// nothing about what it says. The content lives here, and the two are joined by
/// the message index that `visible_range()` reports.
#[derive(Clone)]
struct Transcript {
    messages: Vec<ChatMessage>,
    rows: MessageListState,
}

impl Transcript {
    fn new(messages: Vec<ChatMessage>, viewport_rows: u64) -> Self {
        let entries = entries_for(&messages);
        let rows = MessageListState::try_new(
            &entries,
            BUBBLE_WIDTH,
            ViewportRows::new(viewport_rows.max(1)),
            MEASUREMENT_CACHE,
            measure,
        )
        .expect("every message measures");
        Self { messages, rows }
    }

    /// Appends a message and follows it if the reader is at the bottom.
    ///
    /// Following is the list's decision, not this example's: content arriving
    /// while the reader has scrolled up must not yank them back down.
    fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
        let entry = entry_for(
            self.messages.len() - 1,
            self.messages.last().expect("just pushed"),
        );
        let revision = self.rows.revision();
        if self
            .rows
            .try_append(revision, std::slice::from_ref(&entry), measure)
            .is_err()
        {
            // Re-measuring from scratch is the honest fallback for an example:
            // a half-updated index would report row positions that exist
            // nowhere on screen.
            *self = Self::new(self.messages.clone(), self.rows.viewport_rows().get());
        }
    }

    fn scroll_by(&mut self, delta: i64) {
        let current = self.rows.scroll_offset().get() as i64;
        let target = current.saturating_add(delta).max(0) as u64;
        let revision = self.rows.revision();
        // Clamping and the follow-state transition both belong to the list:
        // landing exactly on the last row resumes following, and anything above
        // it stays paused.
        let _ = self.rows.try_scroll_to(revision, RowOffset::new(target));
    }

    fn resize_viewport(&mut self, viewport_rows: u64) {
        let rows = ViewportRows::new(viewport_rows.max(1));
        if rows == self.rows.viewport_rows() {
            return;
        }
        let revision = self.rows.revision();
        let _ = self.rows.try_set_viewport_rows(revision, rows);
    }
}

/// Builds the measurement entry for one message.
///
/// The key is derived from the message's own text, so editing content changes
/// the key and the cached height is invalidated exactly where it should be.
fn entry_for(index: usize, message: &ChatMessage) -> MessageListEntry {
    let shell =
        MessageShellMeasureConfig::try_new(BUBBLE_WIDTH, HorizontalInsets::new(1, 1), vec![])
            .expect("a positive content width");
    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain(
            message.content.clone(),
            TextFlowSourceKind::Exact,
            Style::default(),
        ),
        options: TextFlowOptions::new(usize::from(shell.content_width()), TextWrap::Wrap),
    };
    MessageListEntry::new(
        MessageId::new(index as u64 + 1),
        MessageRevision::INITIAL,
        MessageVariantKey::new(role_key(message.role)),
        MessageExpansionKey::new(0),
        MessageCompositeMeasureConfig::try_new(vec![identity], shell).expect("a valid config"),
    )
}

fn entries_for(messages: &[ChatMessage]) -> Vec<MessageListEntry> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| entry_for(index, message))
        .collect()
}

const fn role_key(role: Role) -> u64 {
    match role {
        Role::User => 0,
        Role::Assistant => 1,
        Role::System => 2,
    }
}

/// Rows one bubble occupies: its wrapped content, plus the name line.
///
/// Measured through the same `TextFlow` the renderer wraps with. A separate
/// estimate here would make the index and the paint disagree about how tall a
/// message is, and the transcript would scroll to rows that are not where it
/// thinks they are.
fn measure(request: MessageMeasureRequest<'_>) -> MessageMeasureOutcome<String, ()> {
    let mut rows = 1u64; // the name and timestamp line
    for flow in request.key.config().text_flows() {
        match TextFlow::try_build(&flow.input, &flow.options) {
            Ok(built) => rows += built.row_count() as u64,
            Err(error) => return MessageMeasureOutcome::Failed(error.to_string()),
        }
    }
    match MessageRows::try_new(rows) {
        Ok(rows) => MessageMeasureOutcome::Measured(rows),
        Err(error) => MessageMeasureOutcome::Failed(error.to_string()),
    }
}

fn app() -> Element {
    let (_, terminal_height) = use_window_size();
    let viewport_rows = u64::from(terminal_height.saturating_sub(CHROME_ROWS).max(1));

    let transcript = use_signal(|| Transcript::new(initial_messages(), viewport_rows));
    let composer = use_signal(ChatComposerState::new);
    let is_typing = use_signal(|| false);
    let app = use_app();

    let transcript_input = transcript.clone();
    let composer_input = composer.clone();
    let is_typing_input = is_typing.clone();

    use_input(move |input, key| {
        if key.escape || (key.ctrl && input == "c") {
            app.exit();
            return;
        }

        if key.page_up {
            transcript_input.update(|state| state.scroll_by(-(PAGE_ROWS as i64)));
            return;
        }
        if key.page_down {
            transcript_input.update(|state| state.scroll_by(PAGE_ROWS as i64));
            return;
        }

        // Editing, deletion, movement and submission in one call. The example
        // decides none of it.
        let mut state = composer_input.get();
        let outcome = handle_key(&mut state, &ChatComposerKeyMap::new(), input, key);

        if let InteractionOutcome::Submitted(text) = outcome {
            transcript_input.update(|transcript| {
                transcript.push(ChatMessage {
                    role: Role::User,
                    content: text.clone(),
                    timestamp: current_time(),
                });
            });

            is_typing_input.set(true);
            let transcript_reply = transcript_input.clone();
            let typing_reply = is_typing_input.clone();
            let prompt = text.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(800));
                typing_reply.set(false);
                transcript_reply.update(|transcript| {
                    transcript.push(ChatMessage {
                        role: Role::Assistant,
                        content: generate_response(&prompt),
                        timestamp: current_time(),
                    });
                });
            });

            // The draft survives Enter and is cleared only once the send is
            // known to have succeeded.
            if let Some(token) = state.pending_submission().map(|pending| pending.token()) {
                let _ = state.acknowledge_success(token);
            }
        }

        composer_input.set(state);
    });

    // Checked before updating, and read without cloning. `Signal::update` calls
    // `trigger_render()` unconditionally, so an unconditional update here would
    // schedule a render from inside a render and spin forever.
    let viewport_changed = transcript
        .with(|state| state.rows.viewport_rows() != ViewportRows::new(viewport_rows.max(1)));
    if viewport_changed {
        transcript.update(|state| state.resize_viewport(viewport_rows));
    }

    // Borrowed rather than cloned: `get()` would copy the whole transcript —
    // every message plus the row index — on every frame.
    let transcript_view = transcript.with(|state| message_list(state, is_typing.get()));
    let composer_view = composer.with(input_area);

    Box::new()
        .flex_direction(FlexDirection::Column)
        .children(vec![header(), transcript_view, composer_view, footer()])
        .into_element()
}

fn header() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(JustifyContent::SpaceBetween)
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

fn message_list(transcript: &Transcript, is_typing: bool) -> Element {
    let mut children = Vec::new();

    if transcript.messages.is_empty() {
        children.push(
            Box::new()
                .flex_grow(1.0)
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center)
                .child(
                    Text::new("Start a conversation...")
                        .color(Color::BrightBlack)
                        .into_element(),
                )
                .into_element(),
        );
    } else {
        // Which rows of which messages are on screen — including the messages
        // only partly visible at the top and bottom edges, which paging by
        // message count cannot express at all.
        match transcript.rows.visible_range() {
            Ok(range) => {
                for slice in &range.slices {
                    if let Some(message) = transcript.messages.get(slice.message_index) {
                        children.push(render_message(message));
                    }
                }
            }
            // Reported rather than silently drawing an empty transcript: an
            // index that cannot answer is a bug worth seeing.
            Err(error) => children.push(
                Text::new(format!("transcript unavailable: {error}"))
                    .color(Color::Red)
                    .into_element(),
            ),
        }
    }

    if is_typing {
        children.push(
            Box::new()
                .padding_x(1.0)
                .margin_top(0.5)
                .child(
                    Text::new("Assistant is typing...")
                        .color(Color::BrightBlack)
                        .italic()
                        .into_element(),
                )
                .into_element(),
        );
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .flex_grow(1.0)
        .padding(1)
        .children(children)
        .into_element()
}

fn render_message(msg: &ChatMessage) -> Element {
    let (name, name_color, content_color, align) = match msg.role {
        Role::User => ("You", Color::Blue, Color::White, JustifyContent::FlexEnd),
        Role::Assistant => (
            "Assistant",
            Color::Green,
            Color::Reset,
            JustifyContent::FlexStart,
        ),
        Role::System => (
            "System",
            Color::Yellow,
            Color::BrightBlack,
            JustifyContent::Center,
        ),
    };

    let bubble_bg = match msg.role {
        Role::User => Color::Ansi256(24),
        Role::Assistant => Color::Ansi256(238),
        Role::System => Color::Ansi256(236),
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(align)
        .margin_bottom(0.5)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .max_width(i32::from(BUBBLE_WIDTH))
                .padding_x(1.0)
                .padding_y(0.5)
                .background(bubble_bg)
                .border_style(BorderStyle::Round)
                .border_color(Color::Ansi256(240))
                .children(vec![
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::SpaceBetween)
                        .children(vec![
                            Text::new(name).color(name_color).bold().into_element(),
                            Text::new(&msg.timestamp).dim().into_element(),
                        ])
                        .into_element(),
                    Text::new(&msg.content).color(content_color).into_element(),
                ])
                .into_element(),
        )
        .into_element()
}

fn input_area(composer: &ChatComposerState) -> Element {
    let projection = ComposerProjection::build(composer, BUBBLE_WIDTH);
    let first_row = projection.scroll_offset();
    let mut lines = Box::new().flex_direction(FlexDirection::Column);

    if composer.text().is_empty() {
        lines = lines.child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .children(vec![
                    Text::new("> ").color(Color::Cyan).bold().into_element(),
                    Text::new("Type a message...")
                        .color(Color::BrightBlack)
                        .into_element(),
                    cell(" ", true),
                ])
                .into_element(),
        );
    } else {
        for (offset, row) in projection.visible_slice().iter().enumerate() {
            let absolute_row = first_row + offset;
            let cursor_column =
                (absolute_row == projection.cursor_row()).then(|| projection.cursor_column());
            lines = lines.child(input_line(row, cursor_column, offset == 0));
        }
    }

    Box::new()
        .padding_x(1.0)
        .padding_y(0.5)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .margin_x(1.0)
        .child(lines.into_element())
        .into_element()
}

fn input_line(row: &str, cursor_column: Option<usize>, first: bool) -> Element {
    let mut line = Box::new().flex_direction(FlexDirection::Row).child(
        Text::new(if first { "> " } else { "  " })
            .color(Color::Cyan)
            .bold()
            .into_element(),
    );

    let mut column = 0usize;
    let mut painted_cursor = false;
    for cluster in row.graphemes(true) {
        let at_cursor = cursor_column == Some(column);
        painted_cursor |= at_cursor;
        line = line.child(cell(cluster, at_cursor));
        // Cells, not clusters: a CJK character occupies two columns.
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

fn footer() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .background(Color::Ansi256(236))
        .gap(2.0)
        .children(vec![
            Text::new("Enter")
                .color(Color::Yellow)
                .bold()
                .into_element(),
            Text::new("Send").dim().into_element(),
            Text::new("PgUp/PgDn")
                .color(Color::Yellow)
                .bold()
                .into_element(),
            Text::new("Scroll").dim().into_element(),
            Text::new("Esc").color(Color::Yellow).bold().into_element(),
            Text::new("Exit").dim().into_element(),
        ])
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
        // Truncated by grapheme cluster. Slicing to byte 30 panics the moment
        // someone types a multi-byte character, which is any non-ASCII one.
        let preview: String = input.graphemes(true).take(30).collect();
        format!("I received your message: \"{preview}\". How can I assist you further?")
    }
}

fn initial_messages() -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: Role::System,
            content: "Welcome to rnk-chat! This is a demo of rnk's chat UI capabilities."
                .to_string(),
            timestamp: current_time(),
        },
        ChatMessage {
            role: Role::Assistant,
            content: "Hi! I'm an AI assistant. How can I help you today?".to_string(),
            timestamp: current_time(),
        },
    ]
}
