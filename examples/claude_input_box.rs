//! Claude/Sage style inline chat input with native terminal scrollback.
//!
//! This is the model used by Sage:
//! - the live rnk component is only the bottom input box
//! - submitted transcript crosses `InlineChatShell`'s typed commit boundary
//! - scrolling above the prompt is handled by the terminal's native scrollback
//!
//! Run with: cargo run --example claude_input_box

use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, ComposerProjection, InlineChatShell, InlineCommitReport,
    InlineKeyOutcome, MessageId, MessageRevision, NativeTerminalSink, ProjectionContext,
    ScrollbackNamespace, ScrollbackSink, ThemeIdentity,
};
use rnk::hooks::use_interval_when;
use rnk::prelude::*;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const PROMPT: &str = "❯ ";
const MAX_VISIBLE_INPUT_LINES: NonZeroUsize = NonZeroUsize::new(4).expect("four is non-zero");
const OPENING_FRAME_MS: u64 = 70;
const OPENING_FRAME_COUNT: usize = 18;
const ASCII_LOGO: [&str; 5] = [
    r" ____  _   _ _  __",
    r"|  _ \| \ | | |/ /",
    r"| |_) |  \| | ' / ",
    r"|  _ <| |\  | . \ ",
    r"|_| \_\_| \_|_|\_\",
];
const HEADER_LINES: [&str; 5] = [
    "Claude/Sage inline input demo",
    "rnk native scrollback mode",
    "messages persist above the prompt",
    "~/examples/claude_input_box",
    "",
];

type TerminalInlineShell = InlineChatShell<NativeTerminalSink<std::io::Stdout>>;

#[derive(Clone)]
struct SharedInlineShell(Arc<Mutex<TerminalInlineShell>>);

impl SharedInlineShell {
    fn new() -> Self {
        let namespace = ScrollbackNamespace::new("claude-input-box")
            .expect("the example namespace is a valid constant");
        let mut shell = InlineChatShell::new(namespace, NativeTerminalSink::new(std::io::stdout()));
        *shell.composer_mut() =
            ChatComposerState::new().with_max_visible_lines(MAX_VISIBLE_INPUT_LINES);
        Self(Arc::new(Mutex::new(shell)))
    }

    fn with<R>(&self, operation: impl FnOnce(&TerminalInlineShell) -> R) -> R {
        match self.0.lock() {
            Ok(shell) => operation(&shell),
            Err(poisoned) => operation(&poisoned.into_inner()),
        }
    }

    fn update<R>(&self, operation: impl FnOnce(&mut TerminalInlineShell) -> R) -> R {
        match self.0.lock() {
            Ok(mut shell) => operation(&mut shell),
            Err(poisoned) => operation(&mut poisoned.into_inner()),
        }
    }
}

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let inline_shell = use_signal(SharedInlineShell::new);
    let submitted_count = use_signal(|| 0u32);
    let opening_frame = use_signal(|| 0usize);
    let intro_printed = use_signal(|| false);

    let opening_done = opening_frame.get() + 1 >= OPENING_FRAME_COUNT;
    let opening_for_interval = opening_frame.clone();
    use_interval_when(
        Duration::from_millis(OPENING_FRAME_MS),
        !opening_done,
        move || {
            opening_for_interval.update(|frame| {
                *frame = frame.saturating_add(1).min(OPENING_FRAME_COUNT - 1);
            });
        },
    );

    let app_for_intro = app.clone();
    let intro_for_effect = intro_printed.clone();
    let intro_already_printed = intro_printed.get();
    use_effect(
        move || {
            if opening_done && !intro_for_effect.get() {
                print_intro(&app_for_intro);
                intro_for_effect.set(true);
            }

            None
        },
        (opening_done, intro_already_printed),
    );

    let shell_for_handler = inline_shell.clone();
    let count_for_handler = submitted_count.clone();
    let app_for_handler = app.clone();
    let input_ready = intro_printed.get();

    let keymap = ChatComposerKeyMap::new();

    use_input(move |input, key| {
        if key.escape || (key.ctrl && input.eq_ignore_ascii_case("c")) {
            app_for_handler.exit();
            return;
        }

        if !input_ready {
            return;
        }

        // One call covers editing, movement, newline and submission. Escape is
        // handled above because this example exits on it rather than treating
        // it as a cancel.
        shell_for_handler.update(|shared| {
            shared.update(|shell| {
                let outcome = shell.handle_key(&keymap, input, key);
                let InlineKeyOutcome::Submitted(submitted) = outcome else {
                    return;
                };

                let message_number = count_for_handler.get() + 1;
                let width = terminal_content_width();
                let reply = format!(
                    "Received message #{message_number}. It is now part of the terminal scrollback; use your terminal's normal scroll gesture to move upward."
                );
                let token = shell
                    .composer()
                    .pending_submission()
                    .expect("Submitted always stages a token")
                    .token();

                match commit_inline_transcript(shell, message_number, &submitted, &reply, width) {
                    Ok(InlineCommitReport::Fixed { .. }) => {
                        if shell.composer_mut().acknowledge_success(token).is_ok() {
                            count_for_handler.set(message_number);
                        }
                    }
                    Ok(InlineCommitReport::Retained { cause }) => {
                        let _ = shell.composer_mut().acknowledge_failure(token);
                        app_for_handler.println(
                            Text::new(format!(
                                "scrollback commit retained; the draft is ready to retry: {cause}"
                            ))
                            .color(Color::Yellow)
                            .into_element(),
                        );
                    }
                    Ok(InlineCommitReport::Latched { evidence }) => {
                        app_for_handler.println(
                            Text::new(format!(
                                "scrollback commit is latched for human inspection; the draft remains frozen: {evidence}"
                            ))
                            .color(Color::Yellow)
                            .into_element(),
                        );
                    }
                    Err(error) => {
                        let _ = shell.composer_mut().acknowledge_failure(token);
                        app_for_handler.println(
                            Text::new(format!(
                                "scrollback commit failed; the draft is ready to retry: {error}"
                            ))
                            .color(Color::Red)
                            .into_element(),
                        );
                    }
                }
            });
        });
    });

    if input_ready {
        inline_shell.with(|shared| {
            shared.with(|shell| render_input_box(shell.composer(), terminal_content_width()))
        })
    } else {
        render_opening_animation(opening_frame.get(), terminal_content_width())
    }
}

fn commit_inline_transcript<S: ScrollbackSink>(
    shell: &mut InlineChatShell<S>,
    message_number: u32,
    submitted: &str,
    reply: &str,
    width: usize,
) -> Result<InlineCommitReport, String> {
    let context = ProjectionContext::new(
        u16::try_from(width.max(1)).unwrap_or(u16::MAX),
        ThemeIdentity::new(0),
    )
    .map_err(|error| format!("invalid projection context: {error}"))?;
    let canonical = format!("You: {submitted}\nAssistant: {reply}");
    shell
        .finish(
            MessageId::new(u64::from(message_number)),
            MessageRevision::INITIAL,
            &canonical,
            context,
        )
        .map_err(|error| error.to_string())
}

fn render_opening_animation(frame: usize, width: usize) -> Element {
    let width = safe_terminal_width(width);
    let visible_cols = opening_visible_cols(frame);
    let spinner = opening_spinner(frame);
    let mut container = Box::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .height((ASCII_LOGO.len() + 3) as i32);

    for line in ASCII_LOGO {
        container = container.child(
            Text::new(reveal_ascii_line(line, visible_cols))
                .color(Color::BrightCyan)
                .bold()
                .into_element(),
        );
    }

    container
        .child(Text::new("").into_element())
        .child(
            Text::spans(vec![
                Span::new(format!("[{}] ", spinner)).color(Color::BrightGreen),
                Span::new("opening inline scrollback demo").dim(),
            ])
            .into_element(),
        )
        .child(Text::new(" ".repeat(width)).dim().into_element())
        .into_element()
}

fn opening_visible_cols(frame: usize) -> usize {
    let max_width = ASCII_LOGO
        .iter()
        .map(|line| UnicodeWidthStr::width(*line))
        .max()
        .unwrap_or(0);

    let step = (max_width / 5).max(1);
    frame.saturating_add(1).saturating_mul(step).min(max_width)
}

fn opening_spinner(frame: usize) -> char {
    const SPINNER: [char; 4] = ['|', '/', '-', '\\'];
    SPINNER[frame % SPINNER.len()]
}

fn reveal_ascii_line(line: &str, visible_cols: usize) -> String {
    line.chars()
        .enumerate()
        .map(|(index, ch)| if index < visible_cols { ch } else { ' ' })
        .collect()
}

fn render_input_box(state: &ChatComposerState, width: usize) -> Element {
    let width = safe_terminal_width(width);
    let border = "─".repeat(width);
    let input_width = input_viewport_width(width);

    // Wrapping, the visible window and the cursor position all come from the
    // library projection. The example used to compute each of them itself.
    let projection = ComposerProjection::build(state, input_width as u16);
    let first_row = projection.scroll_offset();

    let mut container = Box::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .child(render_border_line(&border, width));

    for (offset, row) in projection.visible_slice().iter().enumerate() {
        let absolute_row = first_row + offset;
        let cursor_column =
            (absolute_row == projection.cursor_row()).then(|| projection.cursor_column());
        container = container.child(render_input_line(row, cursor_column, offset == 0, width));
    }

    container
        .child(render_border_line(&border, width))
        .into_element()
}

fn render_border_line(border: &str, width: usize) -> Element {
    Box::new()
        .width(width as i32)
        .height(1)
        .overflow(Overflow::Hidden)
        .flex_shrink(0.0)
        .child(
            Text::new(border.to_string())
                .color(Color::Ansi256(240))
                .into_element(),
        )
        .into_element()
}

fn render_input_line(
    row: &str,
    cursor_column: Option<usize>,
    show_prompt: bool,
    width: usize,
) -> Element {
    let mut line = Box::new()
        .flex_direction(FlexDirection::Row)
        .width(width as i32)
        .height(1)
        .overflow(Overflow::Hidden)
        .flex_shrink(0.0);

    if show_prompt {
        line = line.child(
            Text::new(PROMPT)
                .color(Color::BrightCyan)
                .bold()
                .into_element(),
        );
    } else {
        line = line.child(Text::new(" ".repeat(UnicodeWidthStr::width(PROMPT))).into_element());
    }

    let mut column = 0usize;
    let mut painted_cursor = false;
    for cluster in row.graphemes(true) {
        let at_cursor = cursor_column == Some(column);
        painted_cursor |= at_cursor;
        line = line.child(styled_cell(cluster, at_cursor));
        column += UnicodeWidthStr::width(cluster).max(1);
    }

    // A cursor at the end of the row has no character to sit on, so it gets a
    // space of its own rather than disappearing.
    if let Some(target) = cursor_column
        && !painted_cursor
        && target >= column
    {
        line = line.child(styled_cell(" ", true));
    }

    line.into_element()
}

fn styled_cell(cluster: &str, at_cursor: bool) -> Element {
    let text = Text::new(cluster.to_string());
    if at_cursor {
        text.color(Color::Black)
            .background(Color::BrightCyan)
            .into_element()
    } else {
        text.into_element()
    }
}

fn print_intro(app: &AppContext) {
    app.println(Text::new("").into_element());

    let logo_width = ascii_logo_width();
    for (index, logo_line) in ASCII_LOGO.iter().enumerate() {
        let logo = pad_to_width(logo_line, logo_width + 2);
        let detail = HEADER_LINES.get(index).copied().unwrap_or_default();

        app.println(
            Text::spans(vec![
                Span::new(logo).color(Color::BrightCyan).bold(),
                header_detail_span(index, detail),
            ])
            .into_element(),
        );
    }

    app.println("");
}

fn ascii_logo_width() -> usize {
    ASCII_LOGO
        .iter()
        .map(|line| UnicodeWidthStr::width(*line))
        .max()
        .unwrap_or(0)
}

fn pad_to_width(text: &str, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    format!("{}{}", text, " ".repeat(width.saturating_sub(text_width)))
}

fn header_detail_span(index: usize, text: &str) -> Span {
    if index == 0 {
        Span::new(text).color(Color::BrightWhite).bold()
    } else {
        Span::new(text).color(Color::Ansi256(245))
    }
}

fn terminal_content_width() -> usize {
    let (width, _) = rnk::renderer::Terminal::size().unwrap_or((80, 24));
    width as usize
}

fn safe_terminal_width(width: usize) -> usize {
    width.saturating_sub(1).max(1)
}

fn input_viewport_width(width: usize) -> usize {
    let prompt_width = UnicodeWidthStr::width(PROMPT);
    width.saturating_sub(prompt_width).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    #[derive(Debug, Default)]
    struct RejectOnceWriter {
        rejected: bool,
        bytes: Vec<u8>,
    }

    #[derive(Debug, Default)]
    struct PartialThenFailWriter {
        accepted_once: bool,
        failed_once: bool,
    }

    impl Write for PartialThenFailWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.accepted_once {
                self.accepted_once = true;
                return Ok(bytes.len().min(1));
            }
            if !self.failed_once {
                self.failed_once = true;
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "partial write"));
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Write for RejectOnceWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.rejected {
                self.rejected = true;
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "try again"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn retained_commit_keeps_the_same_shell_and_draft_available_for_retry() {
        let namespace = ScrollbackNamespace::new("test-inline").expect("valid namespace");
        let sink = NativeTerminalSink::new(RejectOnceWriter::default());
        let mut shell = InlineChatShell::new(namespace, sink);
        let keymap = ChatComposerKeyMap::new();

        assert!(matches!(
            shell.handle_key(&keymap, "hello", &Key::default()),
            InlineKeyOutcome::Changed(_)
        ));
        let enter = Key {
            return_key: true,
            ..Key::default()
        };
        assert!(matches!(
            shell.handle_key(&keymap, "", &enter),
            InlineKeyOutcome::Submitted(_)
        ));
        let token = shell
            .composer()
            .pending_submission()
            .expect("submission staged")
            .token();

        let first = commit_inline_transcript(&mut shell, 1, "hello", "reply", 80)
            .expect("content is valid");
        assert!(matches!(first, InlineCommitReport::Retained { .. }));
        shell
            .composer_mut()
            .acknowledge_failure(token)
            .expect("retry keeps draft");
        assert_eq!(shell.composer().text(), "hello");

        assert!(matches!(
            shell.handle_key(&keymap, "", &enter),
            InlineKeyOutcome::Submitted(_)
        ));
        let retry_token = shell
            .composer()
            .pending_submission()
            .expect("retry staged")
            .token();
        let retry =
            commit_inline_transcript(&mut shell, 1, "hello", "reply", 80).expect("retry is valid");
        assert!(matches!(retry, InlineCommitReport::Fixed { .. }));
        shell
            .composer_mut()
            .acknowledge_success(retry_token)
            .expect("confirmed commit clears draft");
        assert_eq!(shell.composer().text(), "");
        assert!(shell.live_messages().is_empty());
    }

    #[test]
    fn partial_write_latches_the_persistent_shell_and_blocks_automatic_retry() {
        let namespace = ScrollbackNamespace::new("test-latched").expect("valid namespace");
        let sink = NativeTerminalSink::new(PartialThenFailWriter::default());
        let mut shell = InlineChatShell::new(namespace, sink);
        let keymap = ChatComposerKeyMap::new();
        shell.handle_key(&keymap, "hello", &Key::default());
        let enter = Key {
            return_key: true,
            ..Key::default()
        };
        assert!(matches!(
            shell.handle_key(&keymap, "", &enter),
            InlineKeyOutcome::Submitted(_)
        ));

        let first = commit_inline_transcript(&mut shell, 1, "hello", "reply", 80)
            .expect("content is valid");

        assert!(matches!(first, InlineCommitReport::Latched { .. }));
        assert_eq!(
            shell.live_state(MessageId::new(1)),
            Some(rnk::components::chat::LiveState::AwaitingResolution)
        );
        assert!(shell.composer().is_submitting(), "draft remains frozen");
        assert!(matches!(
            commit_inline_transcript(&mut shell, 1, "hello", "reply", 80),
            Err(error) if error.contains("latched")
        ));
    }
}
