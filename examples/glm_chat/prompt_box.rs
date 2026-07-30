use std::io::{self, Write};

use crossterm::terminal;
use rnk::components::chat::ChatComposerState;
use rnk::prelude::Box as RnkBox;
use rnk::prelude::{Color, Element, FlexDirection, Overflow, RenderOptions, Text};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const PROMPT: &str = "❯ ";

fn render_prompt(composer: &ChatComposerState, terminal_width: usize) -> Element {
    let width = input_box_width(terminal_width);
    let visible_input = visible_input_suffix(&composer.text(), input_viewport_width(width));

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

pub(crate) fn draw_prompt_box(composer: &ChatComposerState) -> io::Result<()> {
    let width = terminal_content_width();
    print!("{}", prompt_box_output(composer, width));
    move_cursor_to_prompt_input(composer, width);
    io::stdout().flush()
}

pub(crate) fn redraw_prompt_box(composer: &ChatComposerState) -> io::Result<()> {
    clear_live_prompt_box();
    draw_prompt_box(composer)
}

pub(crate) fn clear_live_prompt_box() {
    print!("\r\x1b[1A\x1b[2K\x1b[1B\r\x1b[2K\x1b[1B\r\x1b[2K\x1b[2A\r");
}

fn prompt_box_output(composer: &ChatComposerState, terminal_width: usize) -> String {
    let width = input_box_width(terminal_width);
    rnk::render_to_string_with_options(
        &render_prompt(composer, terminal_width),
        width as u16,
        &RenderOptions {
            trim: false,
            normalize_line_endings: false,
        },
    )
}

fn move_cursor_to_prompt_input(composer: &ChatComposerState, terminal_width: usize) {
    let width = input_box_width(terminal_width);
    let visible_input = visible_input_suffix(&composer.text(), input_viewport_width(width));
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

/// The trailing part of `input` that fits in `max_width` cells.
///
/// Clipping walks grapheme clusters, not `char`s: taking the last N `char`s can
/// cut an emoji or a combining sequence in half and leave the terminal drawing
/// something the user never typed.
fn visible_input_suffix(input: &str, max_width: usize) -> String {
    let max_width = max_width.max(1);
    let mut clusters = Vec::new();
    let mut width = 0usize;

    for cluster in input.graphemes(true).rev() {
        let cluster_width = UnicodeWidthStr::width(cluster).max(1);
        if width + cluster_width > max_width {
            break;
        }

        clusters.push(cluster);
        width += cluster_width;
    }

    clusters.iter().rev().copied().collect()
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

    fn composer_with(text: &str) -> ChatComposerState {
        use rnk::components::chat::{ChatComposerKeyMap, handle_key};
        use rnk::hooks::Key;

        let mut composer = ChatComposerState::new();
        handle_key(
            &mut composer,
            &ChatComposerKeyMap::new(),
            text,
            &Key::default(),
        );
        composer
    }

    #[test]
    fn prompt_box_renders_three_line_claude_style_frame() {
        let output = strip_ansi(&prompt_box_output(&composer_with("hello"), 20));
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

    #[test]
    fn visible_input_suffix_never_splits_a_cluster() {
        // The family emoji is one cluster of two cells, so at width 3 it fits
        // whole alongside one more character. A char-based clip would keep only
        // part of it, leaving a dangling joiner.
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        assert_eq!(
            visible_input_suffix(&format!("ab{family}"), 3),
            format!("b{family}")
        );
        // Two cells do not fit in one, and half a cluster is never emitted.
        assert_eq!(visible_input_suffix(&format!("ab{family}"), 1), "");
    }
}
