//! Claude/Sage style inline chat input with native terminal scrollback.
//!
//! This is the model used by Sage:
//! - the live rnk component is only the bottom input box
//! - submitted transcript is printed with `app.println()`
//! - scrolling above the prompt is handled by the terminal's native scrollback
//!
//! Run with: cargo run --example claude_input_box

use rnk::components::InteractionOutcome;
use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, ComposerProjection, handle_key,
};
use rnk::hooks::use_interval_when;
use rnk::prelude::*;
use std::num::NonZeroUsize;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let input_state =
        use_signal(|| ChatComposerState::new().with_max_visible_lines(MAX_VISIBLE_INPUT_LINES));
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

    let input_for_handler = input_state.clone();
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
        let mut state = input_for_handler.get();
        let outcome = handle_key(&mut state, &keymap, input, key);

        if let InteractionOutcome::Submitted(submitted) = outcome {
            let message_number = count_for_handler.get() + 1;
            count_for_handler.set(message_number);
            let width = terminal_content_width();

            app_for_handler.println(user_message(&submitted, width));
            app_for_handler.println(assistant_message(
                &format!(
                    "Received message #{message_number}. It is now part of the terminal scrollback; use your terminal's normal scroll gesture to move upward."
                ),
                width,
            ));
            app_for_handler.println("");

            // The composer keeps the draft until the send is confirmed, so it
            // is cleared here rather than by Enter itself. A failed send would
            // instead call `acknowledge_failure` and leave the text in place.
            if let Some(token) = state.pending_submission().map(|pending| pending.token()) {
                let _ = state.acknowledge_success(token);
            }
        }

        input_for_handler.set(state);
    });

    if input_ready {
        render_input_box(&input_state.get(), terminal_content_width())
    } else {
        render_opening_animation(opening_frame.get(), terminal_content_width())
    }
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

fn user_message(text: &str, width: usize) -> Element {
    message_element("You: ", Color::BrightCyan, text, width)
}

fn assistant_message(text: &str, width: usize) -> Element {
    message_element("● ", Color::BrightGreen, text, width)
}

fn message_element(prefix: &str, color: Color, text: &str, width: usize) -> Element {
    let prefix_width = UnicodeWidthStr::width(prefix);
    let continuation = " ".repeat(prefix_width);
    let available_width = safe_terminal_width(width)
        .saturating_sub(prefix_width)
        .max(1);
    let mut container = Box::new()
        .flex_direction(FlexDirection::Column)
        .width(safe_terminal_width(width) as i32);

    for (index, line) in wrap_text(text, available_width).into_iter().enumerate() {
        let line_prefix = if index == 0 {
            prefix.to_string()
        } else {
            continuation.clone()
        };

        container = container.child(
            Text::spans(vec![
                Span::new(line_prefix).color(color).bold(),
                Span::new(line),
            ])
            .into_element(),
        );
    }

    container.into_element()
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

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();

    for source_line in text.lines() {
        let mut current = String::new();
        let mut col = 0usize;

        for ch in source_line.chars() {
            let ch_width = ch.width().unwrap_or(0).max(1);

            if col > 0 && col + ch_width > width {
                lines.push(current);
                current = String::new();
                col = 0;
            }

            current.push(ch);
            col += ch_width;
        }

        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
