use std::io::{self, Write};

use crossterm::terminal;
use rnk::prelude::Box as RnkBox;
use rnk::prelude::{Color, Element, FlexDirection, Overflow, RenderOptions, Text};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const PROMPT: &str = "❯ ";

fn render_prompt(input: &str, terminal_width: usize) -> Element {
    let width = input_box_width(terminal_width);
    let visible_input = visible_input_suffix(input, input_viewport_width(width));

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .child(render_prompt_border(width))
        .child(render_prompt_line(&visible_input, width))
        .child(render_prompt_border(width))
        .into_element()
}

fn render_prompt_border(width: usize) -> Element {
    RnkBox::new()
        .width(width as i32)
        .height(1)
        .overflow(Overflow::Hidden)
        .child(
            Text::new("─".repeat(width))
                .color(Color::Ansi256(240))
                .into_element(),
        )
        .into_element()
}

fn render_prompt_line(input: &str, width: usize) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .width(width as i32)
        .height(1)
        .overflow(Overflow::Hidden)
        .child(
            Text::new(PROMPT)
                .color(Color::BrightCyan)
                .bold()
                .into_element(),
        )
        .child(Text::new(input).color(Color::BrightWhite).into_element())
        .into_element()
}

pub(crate) fn draw_prompt_box(input: &str) -> io::Result<()> {
    let width = terminal_content_width();
    print!("{}", prompt_box_output(input, width));
    move_cursor_to_prompt_input(input, width);
    io::stdout().flush()
}

pub(crate) fn redraw_prompt_box(input: &str) -> io::Result<()> {
    clear_live_prompt_box();
    draw_prompt_box(input)
}

pub(crate) fn clear_live_prompt_box() {
    print!("\r\x1b[1A\x1b[2K\x1b[1B\r\x1b[2K\x1b[1B\r\x1b[2K\x1b[2A\r");
}

fn prompt_box_output(input: &str, terminal_width: usize) -> String {
    let width = input_box_width(terminal_width);
    rnk::render_to_string_with_options(
        &render_prompt(input, terminal_width),
        width as u16,
        &RenderOptions {
            trim: false,
            normalize_line_endings: false,
        },
    )
}

fn move_cursor_to_prompt_input(input: &str, terminal_width: usize) {
    let width = input_box_width(terminal_width);
    let visible_input = visible_input_suffix(input, input_viewport_width(width));
    let cursor_column =
        UnicodeWidthStr::width(PROMPT) + UnicodeWidthStr::width(visible_input.as_str()) + 1;
    print!("\r\x1b[1A\x1b[{}G", cursor_column);
}

fn terminal_content_width() -> usize {
    let (width, _) = terminal::size().unwrap_or((80, 24));
    width as usize
}

fn input_box_width(terminal_width: usize) -> usize {
    terminal_width.saturating_sub(1).max(1)
}

fn input_viewport_width(box_width: usize) -> usize {
    box_width
        .saturating_sub(UnicodeWidthStr::width(PROMPT))
        .max(1)
}

fn visible_input_suffix(input: &str, max_width: usize) -> String {
    let max_width = max_width.max(1);
    let mut chars = Vec::new();
    let mut width = 0usize;

    for ch in input.chars().rev() {
        let ch_width = ch.width().unwrap_or(0).max(1);
        if width + ch_width > max_width {
            break;
        }

        chars.push(ch);
        width += ch_width;
    }

    chars.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(input: &str) -> String {
        let mut output = String::new();
        let mut chars = input.chars();

        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                output.push(ch);
            }
        }

        output
    }

    #[test]
    fn prompt_box_renders_three_line_claude_style_frame() {
        let output = strip_ansi(&prompt_box_output("hello", 20));
        let lines: Vec<&str> = output.split("\r\n").collect();
        let border = "─".repeat(input_box_width(20));

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], border);
        assert_eq!(lines[1], "❯ hello");
        assert_eq!(lines[2], border);
    }

    #[test]
    fn visible_input_suffix_keeps_last_cells_that_fit() {
        assert_eq!(visible_input_suffix("abcdef", 3), "def");
        assert_eq!(visible_input_suffix("你好吗", 4), "好吗");
    }
}
