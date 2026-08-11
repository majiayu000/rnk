//! Fullscreen chat composed from the public chat contracts.
//!
//! `FullscreenChatShell` owns region arithmetic and composer routing,
//! `MessageListState` owns row scrolling, and `ChatMessageView` owns semantic
//! message projection. The example only supplies fixture content and colors.
//!
//! Run with: `cargo run --example rnk_chat`

use rnk::components::ScrollableBox;
use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageList,
    MessageListEntry, MessageListMeasureError, MessageListState, MessageMeasureOutcome,
    MessageMeasureRequest, MessageResizeConfigOutcome, MessageRows, MessageShellMeasureConfig,
    MessageVariantKey, RowOffset, ViewportRows,
};
use rnk::components::chat::{
    BlockId, ChatComposerKeyMap, ChatComposerState, ChatMessage, ChatMessageMetadata,
    ChatMessageView, ChatMessageViewOptions, ChatRole, FullscreenChatShell, FullscreenFocus,
    FullscreenFocusOutcome, FullscreenKeyOutcome, MessageAuthor, MessageBlock, MessageBlockEntry,
    MessageId, MessageRevision, MessageStatus, MessageTimestamp,
};
use rnk::core::{Overflow, TextWrap};
use rnk::hooks::{BracketedPasteGuard, Key, use_paste, use_window_size};
use rnk::layout::LayoutEngine;
use rnk::layout::text_flow::{
    TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};
use rnk::prelude::*;
use std::sync::{Arc, Mutex};

const STATUS_ROWS: u16 = 1;
const PAGE_ROWS: u64 = 5;
const MEASUREMENT_CACHE: usize = 512;

fn main() -> std::io::Result<()> {
    let _paste = BracketedPasteGuard::new()?;
    render(app).fullscreen().run()
}

#[derive(Clone)]
pub(crate) struct ChatSurface {
    messages: Vec<ChatMessage>,
    transcript: MessageListState,
    composer: ChatComposerState,
    focus: FullscreenFocus,
    overlay_open: bool,
    width: u16,
    height: u16,
    revision: u64,
    typing: bool,
    status: String,
}

impl ChatSurface {
    pub(crate) fn try_new(width: u16, height: u16) -> Result<Self, String> {
        let messages = initial_messages()?;
        let transcript = build_transcript(&messages, width, 1)?;
        let mut candidate = Self {
            messages,
            transcript,
            composer: ChatComposerState::new(),
            focus: FullscreenFocus::Composer,
            overlay_open: false,
            width,
            height,
            revision: 0,
            typing: false,
            status: "ready".to_owned(),
        };
        let shell = candidate.try_shell()?;
        candidate.absorb_shell(&shell);
        Ok(candidate)
    }

    pub(crate) fn try_shell(&self) -> Result<FullscreenChatShell, String> {
        let mut shell = FullscreenChatShell::try_new(
            self.transcript.clone(),
            self.composer.clone(),
            self.width,
            self.height,
            STATUS_ROWS,
        )
        .map_err(|error| error.to_string())?;
        shell.set_overlay_open(self.overlay_open);
        shell.set_focus(self.focus);
        Ok(shell)
    }

    fn absorb_shell(&mut self, shell: &FullscreenChatShell) {
        self.transcript = shell.transcript().clone();
        self.composer = shell.composer().clone();
        self.focus = shell.focus();
        self.overlay_open = shell.overlay_open();
    }

    pub(crate) fn try_resize(&self, width: u16, height: u16) -> Result<Self, String> {
        let provisional = FullscreenChatShell::try_new(
            self.transcript.clone(),
            self.composer.clone(),
            width,
            height,
            STATUS_ROWS,
        )
        .map_err(|error| error.to_string())?;
        let mut candidate = self.clone();
        let viewport_rows = ViewportRows::new(u64::from(provisional.layout().transcript().rows()));
        let messages = &candidate.messages;
        candidate
            .transcript
            .try_resize(
                candidate.transcript.revision(),
                width,
                viewport_rows,
                |request| {
                    let Some(message) = messages
                        .iter()
                        .find(|message| message.id() == request.old_entry.message_id())
                    else {
                        return MessageResizeConfigOutcome::Failed(
                            "resize identity is absent".to_owned(),
                        );
                    };
                    match entry_for(message, request.new_width) {
                        Ok(entry) => {
                            MessageResizeConfigOutcome::Rebuilt(entry.measure_config().clone())
                        }
                        Err(error) => MessageResizeConfigOutcome::Failed(error),
                    }
                },
                |request| measure_message_request(messages, request),
            )
            .map_err(measure_error)?;
        candidate.width = width;
        candidate.height = height;
        candidate.status = format!("{}x{}", width, height);
        let shell = candidate.try_shell()?;
        candidate.absorb_shell(&shell);
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "surface revision exhausted".to_owned())?;
        Ok(candidate)
    }

    pub(crate) fn try_scroll(&self, delta: i64) -> Result<Self, String> {
        let mut candidate = self.clone();
        let current = i64::try_from(candidate.transcript.scroll_offset().get())
            .map_err(|_| "scroll offset exceeds signed range".to_owned())?;
        let target = current.saturating_add(delta).max(0) as u64;
        candidate
            .transcript
            .try_scroll_to(candidate.transcript.revision(), RowOffset::new(target))
            .map_err(|error| error.to_string())?;
        candidate.try_shell()?;
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "surface revision exhausted".to_owned())?;
        Ok(candidate)
    }

    pub(crate) fn try_key(&self, input: &str, key: &Key) -> Result<(Self, Option<String>), String> {
        let mut shell = self.try_shell()?;
        let outcome = shell
            .handle_key(&ChatComposerKeyMap::new(), input, key)
            .map_err(|error| error.to_string())?;
        let mut candidate = self.clone();
        candidate.absorb_shell(&shell);

        let submitted = match outcome {
            FullscreenKeyOutcome::Submitted(text) => {
                let token = candidate
                    .composer
                    .pending_submission()
                    .map(|pending| pending.token())
                    .ok_or_else(|| "submitted composer has no pending token".to_owned())?;
                candidate.try_push(message(
                    next_message_id(&candidate.messages)?,
                    ChatRole::User,
                    "You",
                    text.clone(),
                )?)?;
                candidate
                    .composer
                    .acknowledge_success(token)
                    .map_err(|error| format!("composer acknowledgement failed: {error:?}"))?;
                candidate.typing = true;
                candidate.status = "assistant is typing".to_owned();
                Some(text)
            }
            FullscreenKeyOutcome::Cancelled => {
                candidate.status = "input cancelled".to_owned();
                None
            }
            FullscreenKeyOutcome::Changed(_) => {
                candidate.status = "editing".to_owned();
                None
            }
            FullscreenKeyOutcome::Consumed(_) | FullscreenKeyOutcome::Unconsumed(_) => None,
            FullscreenKeyOutcome::Overlay => None,
        };
        candidate.try_shell()?;
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "surface revision exhausted".to_owned())?;
        Ok((candidate, submitted))
    }

    pub(crate) fn try_paste(&self, content: &str) -> Result<Self, String> {
        let (candidate, submitted) = self.try_key(content, &Key::default())?;
        if submitted.is_some() {
            return Err("paste unexpectedly submitted the draft".to_owned());
        }
        Ok(candidate)
    }

    fn try_push(&mut self, message: ChatMessage) -> Result<(), String> {
        let mut messages = self.messages.clone();
        messages.push(message);
        let entry = entry_for(
            messages
                .last()
                .expect("candidate contains appended message"),
            self.width,
        )?;
        let mut transcript = self.transcript.clone();
        transcript
            .try_append(
                transcript.revision(),
                std::slice::from_ref(&entry),
                |request| measure_message_request(&messages, request),
            )
            .map_err(measure_error)?;
        self.messages = messages;
        self.transcript = transcript;
        Ok(())
    }

    pub(crate) fn try_set_focus(&self, focus: FullscreenFocus) -> Result<Self, String> {
        let mut shell = self.try_shell()?;
        match shell.set_focus(focus) {
            FullscreenFocusOutcome::HeldByOverlay(_) => {
                return Err("overlay owns focus".to_owned());
            }
            FullscreenFocusOutcome::Moved(_) | FullscreenFocusOutcome::Unchanged(_) => {}
        }
        let mut candidate = self.clone();
        candidate.absorb_shell(&shell);
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "surface revision exhausted".to_owned())?;
        Ok(candidate)
    }

    pub(crate) fn try_set_overlay(&self, open: bool) -> Result<Self, String> {
        let mut shell = self.try_shell()?;
        shell.set_overlay_open(open);
        let mut candidate = self.clone();
        candidate.absorb_shell(&shell);
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "surface revision exhausted".to_owned())?;
        Ok(candidate)
    }

    pub(crate) fn try_reply(&self, expected_revision: u64, prompt: &str) -> Result<Self, String> {
        if self.revision != expected_revision {
            return Err(format!(
                "reply revision is stale: expected {expected_revision}, current {}",
                self.revision
            ));
        }
        let mut candidate = self.clone();
        let id = next_message_id(&candidate.messages)?;
        candidate.try_push(message(
            id,
            ChatRole::Assistant,
            "Assistant",
            generate_response(prompt),
        )?)?;
        candidate.typing = false;
        candidate.status = "ready".to_owned();
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "surface revision exhausted".to_owned())?;
        candidate.try_shell()?;
        Ok(candidate)
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    #[cfg(test)]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    #[cfg(test)]
    pub(crate) fn message_ids(&self) -> Vec<u64> {
        self.messages
            .iter()
            .map(|message| message.id().get())
            .collect()
    }
}

pub(crate) fn app() -> Element {
    let (terminal_width, terminal_height) = use_window_size();
    let surface = use_signal(|| {
        Arc::new(Mutex::new(
            ChatSurface::try_new(terminal_width, terminal_height)
                .expect("fullscreen chat requires a non-zero usable terminal"),
        ))
    });
    let app = use_app();
    let input_surface = surface.clone();

    use_input(move |input, key| {
        if key.escape || (key.ctrl && input == "c") {
            app.exit();
            return;
        }
        let handle = input_surface.get();
        let mut locked = match handle.lock() {
            Ok(surface) => surface,
            Err(_) => {
                eprintln!("rnk_chat: chat state lock is poisoned; exiting");
                app.exit();
                return;
            }
        };
        let current = locked.clone();
        let candidate = if key.f1 {
            current
                .try_set_overlay(!current.overlay_open)
                .map(|state| (state, None))
        } else if key.tab {
            let focus = if current.focus == FullscreenFocus::Composer {
                FullscreenFocus::Transcript
            } else {
                FullscreenFocus::Composer
            };
            current.try_set_focus(focus).map(|state| (state, None))
        } else if key.page_up {
            current
                .try_scroll(-(PAGE_ROWS as i64))
                .map(|state| (state, None))
        } else if key.page_down {
            current
                .try_scroll(PAGE_ROWS as i64)
                .map(|state| (state, None))
        } else {
            current.try_key(input, key)
        };
        match candidate {
            Ok((candidate, submitted)) => {
                let reply_revision = candidate.revision();
                *locked = candidate;
                drop(locked);
                input_surface.set(handle.clone());
                if let Some(prompt) = submitted {
                    let reply_surface = input_surface.clone();
                    let reply_handle = handle;
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        let Ok(mut locked) = reply_handle.lock() else {
                            eprintln!("rnk_chat: asynchronous reply state lock is poisoned");
                            reply_surface.set(reply_handle);
                            return;
                        };
                        let current = locked.clone();
                        *locked = match current.try_reply(reply_revision, &prompt) {
                            Ok(candidate) => candidate,
                            Err(error) => {
                                let mut failure = current;
                                failure.status = format!("reply failed: {error}");
                                match failure.revision.checked_add(1) {
                                    Some(revision) => failure.revision = revision,
                                    None => {
                                        failure.status =
                                            "reply failed: surface revision exhausted".to_owned()
                                    }
                                }
                                failure
                            }
                        };
                        drop(locked);
                        reply_surface.set(reply_handle);
                    });
                }
            }
            Err(error) => {
                let mut candidate = current;
                candidate.status = format!("update refused: {error}");
                if let Some(revision) = candidate.revision.checked_add(1) {
                    candidate.revision = revision;
                } else {
                    candidate.status = "update refused: surface revision exhausted".to_owned();
                }
                *locked = candidate;
                drop(locked);
                input_surface.set(handle);
            }
        }
    });

    let paste_surface = surface.clone();
    use_paste(move |event| {
        let handle = paste_surface.get();
        let mut locked = match handle.lock() {
            Ok(surface) => surface,
            Err(_) => {
                eprintln!("rnk_chat: chat state lock is poisoned during paste");
                return;
            }
        };
        let current = locked.clone();
        match current.try_paste(event.content()) {
            Ok(candidate) => *locked = candidate,
            Err(error) => {
                let mut candidate = current;
                candidate.status = format!("paste refused: {error}");
                if let Some(revision) = candidate.revision.checked_add(1) {
                    candidate.revision = revision;
                }
                *locked = candidate;
            }
        }
        drop(locked);
        paste_surface.set(handle);
    });

    let handle = surface.get();
    let resize_result = match handle.lock() {
        Ok(mut current) => {
            if current.width == terminal_width && current.height == terminal_height {
                None
            } else {
                let result = current.try_resize(terminal_width, terminal_height);
                match result {
                    Ok(candidate) => *current = candidate,
                    Err(error) => {
                        current.status = format!("resize refused: {error}");
                        if let Some(revision) = current.revision.checked_add(1) {
                            current.revision = revision;
                        }
                    }
                }
                Some(())
            }
        }
        Err(_) => {
            return Text::new("chat state lock is poisoned")
                .color(Color::Red)
                .into_element();
        }
    };
    if resize_result.is_some() {
        surface.set(handle.clone());
    }

    match handle.lock() {
        Ok(surface) => render_surface(&surface),
        Err(_) => Text::new("chat state lock is poisoned")
            .color(Color::Red)
            .into_element(),
    }
}

pub(crate) fn render_surface(surface: &ChatSurface) -> Element {
    let shell = match surface.try_shell() {
        Ok(shell) => shell,
        Err(error) => {
            return Text::new(format!("chat unavailable: {error}"))
                .color(Color::Red)
                .into_element();
        }
    };
    let layout = shell.layout();
    let transcript = render_transcript(surface, shell.transcript()).unwrap_or_else(|error| {
        Text::new(format!("transcript unavailable: {error}"))
            .color(Color::Red)
            .into_element()
    });
    Box::new()
        .flex_direction(FlexDirection::Column)
        .child(
            ScrollableBox::new()
                .width(layout.width())
                .height(i32::from(layout.transcript().rows()))
                .child(transcript)
                .into_element(),
        )
        .child(render_composer(
            shell.composer(),
            layout.width(),
            layout.composer().rows(),
        ))
        .child(status_bar(surface))
        .into_element()
}

fn render_transcript(
    surface: &ChatSurface,
    transcript: &MessageListState,
) -> Result<Element, String> {
    MessageList::new(transcript)
        .try_into_element(|entry, _key, slice| {
            let message = surface
                .messages
                .iter()
                .find(|message| message.id() == entry.message_id())
                .ok_or_else(|| "visible message identity is absent".to_owned())?;
            let visible_rows = slice
                .viewport_rows
                .end
                .checked_sub(slice.viewport_rows.start)
                .ok_or_else(|| "visible viewport rows are reversed".to_owned())?;
            let message_rows = slice
                .message_rows
                .end
                .checked_sub(slice.message_rows.start)
                .ok_or_else(|| "visible message rows are reversed".to_owned())?;
            if visible_rows != message_rows {
                return Err("message and viewport slice heights disagree".to_owned());
            }
            let height = i32::try_from(visible_rows)
                .map_err(|_| "visible slice height exceeds layout range".to_owned())?;
            let offset = u16::try_from(slice.message_rows.start)
                .map_err(|_| "message row offset exceeds renderer range".to_owned())?;
            Ok(ScrollableBox::new()
                .height(height)
                .scroll_offset_y(offset)
                .child(
                    ChatMessageView::new(message)
                        .options(ChatMessageViewOptions::default())
                        .into_element(),
                )
                .into_element())
        })
        .map_err(|error| error.to_string())
}

fn render_composer(composer: &ChatComposerState, width: u16, rows: u16) -> Element {
    let projection = rnk::components::chat::ComposerProjection::build(composer, width);
    Box::new()
        .height(i32::from(rows))
        .overflow_y(Overflow::Hidden)
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Cyan).bold().into_element())
        .child(Text::new(projection.visible_slice().join("\n")).into_element())
        .child(Text::new("▏").color(Color::Cyan).into_element())
        .into_element()
}

fn status_bar(surface: &ChatSurface) -> Element {
    let typing = if surface.typing { " · typing" } else { "" };
    Box::new()
        .height(i32::from(STATUS_ROWS))
        .background(Color::Ansi256(236))
        .child(
            Text::new(format!("rnk-chat · {}{typing}", surface.status))
                .dim()
                .into_element(),
        )
        .into_element()
}

fn build_transcript(
    messages: &[ChatMessage],
    width: u16,
    viewport_rows: u64,
) -> Result<MessageListState, String> {
    let entries = messages
        .iter()
        .map(|message| entry_for(message, width))
        .collect::<Result<Vec<_>, _>>()?;
    MessageListState::try_new(
        &entries,
        width,
        ViewportRows::new(viewport_rows),
        MEASUREMENT_CACHE,
        |request| measure_message_request(messages, request),
    )
    .map_err(measure_error)
}

fn measure_error(error: MessageListMeasureError<String, ()>) -> String {
    match error {
        MessageListMeasureError::State(source) => source.to_string(),
        MessageListMeasureError::ConfigRebuildFailed {
            message_index,
            message_id,
            ..
        } => format!(
            "message {} at index {message_index} config rebuild failed",
            message_id.get()
        ),
        MessageListMeasureError::ConfigRebuildCancelled {
            message_index,
            message_id,
            ..
        } => format!(
            "message {} at index {message_index} config rebuild was cancelled",
            message_id.get()
        ),
        MessageListMeasureError::MeasurementFailed { key, .. } => {
            format!("message {} measurement failed", key.message_id().get())
        }
        MessageListMeasureError::Cancelled { key, .. } => {
            format!(
                "message {} measurement was cancelled",
                key.message_id().get()
            )
        }
        _ => "unknown message measurement failure".to_owned(),
    }
}

fn entry_for(message: &ChatMessage, width: u16) -> Result<MessageListEntry, String> {
    let shell = MessageShellMeasureConfig::try_new(width, HorizontalInsets::new(0, 0), vec![])
        .map_err(|error| error.to_string())?;
    let text = message
        .blocks()
        .iter()
        .filter_map(|entry| match entry.block() {
            MessageBlock::Text(value) | MessageBlock::Markdown(value) => Some(value.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain(text, TextFlowSourceKind::Exact, Style::default()),
        options: TextFlowOptions::new(usize::from(shell.content_width()), TextWrap::Wrap),
    };
    let variant = match message.role() {
        ChatRole::User => 0,
        ChatRole::Assistant => 1,
        ChatRole::System => 2,
        ChatRole::Tool => 3,
    };
    Ok(MessageListEntry::new(
        message.id(),
        message.revision(),
        MessageVariantKey::new(variant),
        MessageExpansionKey::new(0),
        MessageCompositeMeasureConfig::try_new(vec![identity], shell)
            .map_err(|error| error.to_string())?,
    ))
}

fn measure_message_request(
    messages: &[ChatMessage],
    request: MessageMeasureRequest<'_>,
) -> MessageMeasureOutcome<String, ()> {
    let Some(message) = messages
        .iter()
        .find(|message| message.id() == request.entry.message_id())
    else {
        return MessageMeasureOutcome::Missing;
    };
    let element = ChatMessageView::new(message).into_element();
    let engine = LayoutEngine::new();
    let frame = match engine.prepare_element_incremental(
        &element,
        None,
        request.key.config().shell().outer_width(),
        u16::MAX,
    ) {
        Ok(frame) => frame,
        Err(error) => return MessageMeasureOutcome::Failed(error.to_string()),
    };
    let height = frame.snapshot().root().border_bounds().height();
    let rows = match u64::try_from(height) {
        Ok(0) | Err(_) => {
            return MessageMeasureOutcome::Failed("root snapshot height is invalid".to_owned());
        }
        Ok(rows) => rows,
    };
    match MessageRows::try_new(rows) {
        Ok(rows) => MessageMeasureOutcome::Measured(rows),
        Err(error) => MessageMeasureOutcome::Failed(error.to_string()),
    }
}

fn next_message_id(messages: &[ChatMessage]) -> Result<MessageId, String> {
    messages.last().map_or(Ok(MessageId::new(1)), |message| {
        message
            .id()
            .get()
            .checked_add(1)
            .map(MessageId::new)
            .ok_or_else(|| "message identity exhausted".to_owned())
    })
}

fn message(
    id: MessageId,
    role: ChatRole,
    author: &str,
    content: String,
) -> Result<ChatMessage, String> {
    let metadata = ChatMessageMetadata::new(
        Some(MessageAuthor::new(author).map_err(|error| error.to_string())?),
        Some(MessageTimestamp::new(current_time()).map_err(|error| error.to_string())?),
    );
    ChatMessage::try_restore(
        id,
        role,
        MessageStatus::Complete,
        MessageRevision::INITIAL,
        vec![MessageBlockEntry::new(
            BlockId::new(id.get()),
            MessageBlock::Text(content),
        )],
        metadata,
    )
    .map_err(|error| error.to_string())
}

fn initial_messages() -> Result<Vec<ChatMessage>, String> {
    Ok(vec![
        message(
            MessageId::new(1),
            ChatRole::System,
            "System",
            "Welcome to rnk-chat. Row measurement comes from the published layout snapshot."
                .to_owned(),
        )?,
        message(
            MessageId::new(2),
            ChatRole::Assistant,
            "Assistant",
            "Hi! Resize the terminal or use Page Up and Page Down to inspect row clipping."
                .to_owned(),
        )?,
    ])
}

fn current_time() -> String {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(elapsed) => {
            let seconds = elapsed.as_secs();
            format!("{:02}:{:02}", (seconds / 3600) % 24, (seconds / 60) % 60)
        }
        Err(_) => "clock-error".to_owned(),
    }
}

fn generate_response(input: &str) -> String {
    let normalized = input.to_lowercase();
    if normalized.contains("hello") || normalized.contains("hi") {
        "Hello! How can I help you today?".to_owned()
    } else if normalized.contains("rnk") {
        "rnk composes typed terminal UI state, layout, and rendering.".to_owned()
    } else {
        format!("I received your message: {input}")
    }
}
