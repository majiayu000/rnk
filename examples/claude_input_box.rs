//! Claude/Sage style inline chat input with native terminal scrollback.
//!
//! This is the model used by Sage:
//! - the live rnk component is only the bottom input box
//! - submitted transcript is printed with `app.println()`
//! - scrolling above the prompt is handled by the terminal's native scrollback
//!
//! Run with: cargo run --example claude_input_box

use rnk::hooks::use_interval_when;
use rnk::prelude::*;
use std::time::Duration;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const PROMPT: &str = "❯ ";
const MAX_VISIBLE_INPUT_LINES: usize = 4;
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

#[derive(Clone, Debug, Default)]
struct InlineInputState {
    chars: Vec<char>,
    cursor_pos: usize,
}

impl InlineInputState {
    fn insert_str(&mut self, input: &str) {
        for ch in input.chars().filter(|ch| !ch.is_control()) {
            self.chars.insert(self.cursor_pos, ch);
            self.cursor_pos += 1;
        }
    }

    fn move_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    fn move_right(&mut self) {
        if self.cursor_pos < self.chars.len() {
            self.cursor_pos += 1;
        }
    }

    fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    fn move_end(&mut self) {
        self.cursor_pos = self.chars.len();
    }

    fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.chars.remove(self.cursor_pos);
        }
    }

    fn delete(&mut self) {
        if self.cursor_pos < self.chars.len() {
            self.chars.remove(self.cursor_pos);
        }
    }

    fn clear(&mut self) {
        self.chars.clear();
        self.cursor_pos = 0;
    }

    fn submitted_text(&self) -> String {
        self.chars.iter().collect()
    }
}

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let input_state = use_signal(InlineInputState::default);
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

    use_input(move |input, key| {
        if key.escape || (key.ctrl && input.eq_ignore_ascii_case("c")) {
            app_for_handler.exit();
            return;
        }

        if !input_ready {
            return;
        }

        if key.ctrl && input.eq_ignore_ascii_case("u") {
            input_for_handler.update(|state| state.clear());
            return;
        }

        if key.return_key {
            let submitted = input_for_handler.get().submitted_text();
            if submitted.trim().is_empty() {
                return;
            }

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
            input_for_handler.update(|state| state.clear());
            return;
        }

        input_for_handler.update(|state| {
            if key.left_arrow {
                state.move_left();
            } else if key.right_arrow {
                state.move_right();
            } else if key.home {
                state.move_home();
            } else if key.end {
                state.move_end();
            } else if key.backspace {
                state.backspace();
            } else if key.delete {
                state.delete();
            } else if !key.ctrl && !key.alt && !input.is_empty() {
                state.insert_str(input);
            }
        });
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

fn render_input_box(state: &InlineInputState, width: usize) -> Element {
    let width = safe_terminal_width(width);
    let border = "─".repeat(width);
    let input_width = input_viewport_width(width);
    let lines = visible_input_lines(state, input_width, MAX_VISIBLE_INPUT_LINES);

    let mut container = Box::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .child(render_border_line(&border, width));

    for (index, line) in lines.iter().enumerate() {
        container = container.child(render_input_line(line, index == 0, width));
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

fn render_input_line(line: &InputLine, show_prompt: bool, width: usize) -> Element {
    let mut row = Box::new()
        .flex_direction(FlexDirection::Row)
        .width(width as i32)
        .height(1)
        .overflow(Overflow::Hidden)
        .flex_shrink(0.0);

    if show_prompt {
        row = row.child(
            Text::new(PROMPT)
                .color(Color::BrightCyan)
                .bold()
                .into_element(),
        );
    } else {
        row = row.child(Text::new(" ".repeat(UnicodeWidthStr::width(PROMPT))).into_element());
    }

    for cell in &line.cells {
        row = row.child(match cell {
            InputCell::Text(ch) => Text::new(ch.to_string()).into_element(),
            InputCell::Cursor(ch) => Text::new(ch.to_string())
                .color(Color::Black)
                .background(Color::BrightCyan)
                .into_element(),
        });
    }

    row.into_element()
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum InputCell {
    Text(char),
    Cursor(char),
}

impl InputCell {
    fn width(&self) -> usize {
        match self {
            Self::Text(ch) | Self::Cursor(ch) => ch.width().unwrap_or(1).max(1),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct InputLine {
    cells: Vec<InputCell>,
    has_cursor: bool,
}

fn visible_input_lines(
    state: &InlineInputState,
    viewport_width: usize,
    max_visible_lines: usize,
) -> Vec<InputLine> {
    let wrapped = wrap_input_cells(state, viewport_width);
    let line_count = max_visible_lines.max(1).min(wrapped.len());
    let cursor_line = wrapped
        .iter()
        .position(|line| line.has_cursor)
        .unwrap_or_else(|| wrapped.len().saturating_sub(1));
    let start = cursor_line.saturating_add(1).saturating_sub(line_count);

    wrapped[start..(start + line_count).min(wrapped.len())].to_vec()
}

fn wrap_input_cells(state: &InlineInputState, viewport_width: usize) -> Vec<InputLine> {
    let viewport_width = viewport_width.max(1);
    let mut lines = vec![InputLine::default()];
    let mut col = 0usize;
    let mut index = 0usize;
    let mut cursor_inserted = false;

    while index < state.chars.len() || !cursor_inserted {
        let cell = if !cursor_inserted && index == state.cursor_pos {
            let cursor_char = state.chars.get(index).copied().unwrap_or(' ');
            cursor_inserted = true;
            let cell = InputCell::Cursor(cursor_char);
            if index < state.chars.len() {
                index += 1;
            }
            cell
        } else if let Some(ch) = state.chars.get(index).copied() {
            index += 1;
            InputCell::Text(ch)
        } else {
            break;
        };

        let cell_width = cell.width();
        if col > 0 && col + cell_width > viewport_width {
            lines.push(InputLine::default());
            col = 0;
        }

        if let Some(line) = lines.last_mut() {
            line.has_cursor |= matches!(cell, InputCell::Cursor(_));
            line.cells.push(cell);
        }

        col += cell_width;
    }

    if lines.is_empty() {
        lines.push(InputLine {
            cells: vec![InputCell::Cursor(' ')],
            has_cursor: true,
        });
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_replaces_current_character() {
        let mut state = InlineInputState::default();

        state.insert_str("abcd");
        state.move_left();
        state.move_left();

        let lines = wrap_input_cells(&state, 20);

        assert_eq!(line_for_test(&lines[0]), "ab█d");
    }

    #[test]
    fn wraps_wide_characters_inside_width() {
        let mut state = InlineInputState::default();

        state.insert_str("a你b");
        let lines = wrap_input_cells(&state, 3);

        assert_eq!(line_for_test(&lines[0]), "a你");
        assert_eq!(line_for_test(&lines[1]), "b█");
    }

    #[test]
    fn visible_lines_are_capped_around_cursor() {
        let mut state = InlineInputState::default();

        state.insert_str("abcdefghi");
        let lines = visible_input_lines(&state, 3, 2);

        assert_eq!(lines.len(), 2);
        assert_eq!(line_for_test(&lines[0]), "ghi");
        assert_eq!(line_for_test(&lines[1]), "█");
    }

    #[test]
    fn wraps_printed_messages_to_terminal_width() {
        assert_eq!(wrap_text("abcdef", 3), vec!["abc", "def"]);
    }

    #[test]
    fn opening_reveal_preserves_line_width() {
        let line = ASCII_LOGO[0];

        assert_eq!(reveal_ascii_line(line, 0).len(), line.len());
        assert_eq!(reveal_ascii_line(line, usize::MAX), line);
    }

    #[test]
    fn opening_visible_columns_are_capped() {
        let max_width = ASCII_LOGO
            .iter()
            .map(|line| UnicodeWidthStr::width(*line))
            .max()
            .unwrap();

        assert!(opening_visible_cols(0) > 0);
        assert_eq!(opening_visible_cols(usize::MAX), max_width);
    }

    fn line_for_test(line: &InputLine) -> String {
        line.cells
            .iter()
            .map(|cell| match cell {
                InputCell::Text(ch) => *ch,
                InputCell::Cursor(_) => '█',
            })
            .collect()
    }
}
