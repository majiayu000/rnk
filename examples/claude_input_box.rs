//! Claude Code style input box demo.
//!
//! Run with: cargo run --example claude_input_box

use rnk::prelude::*;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Debug, Default)]
struct ClaudeInputState {
    chars: Vec<char>,
    cursor_pos: usize,
    scroll_offset: usize,
}

impl ClaudeInputState {
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
        self.scroll_offset = 0;
    }

    fn submitted_text(&self) -> String {
        self.chars.iter().collect()
    }

    fn cursor_visual_col(&self) -> usize {
        self.chars
            .iter()
            .take(self.cursor_pos)
            .map(|ch| ch.width().unwrap_or(0))
            .sum()
    }

    fn keep_cursor_visible(&mut self, viewport_width: usize) {
        let cursor_col = self.cursor_visual_col();

        if cursor_col < self.scroll_offset {
            self.scroll_offset = cursor_col;
        } else if viewport_width > 0 && cursor_col >= self.scroll_offset + viewport_width {
            self.scroll_offset = cursor_col.saturating_sub(viewport_width - 1);
        }
    }
}

fn main() -> std::io::Result<()> {
    render(app).fullscreen().run()
}

fn app() -> Element {
    let app = use_app();
    let scroll = use_scroll();
    let input_state = use_signal(ClaudeInputState::default);
    let messages = use_signal(|| {
        vec![
            "Assistant: Type a message in the fixed input box below.".to_string(),
            "Assistant: Try Chinese text, long lines, arrows, Home/End, Backspace, and Delete."
                .to_string(),
            "Assistant: Press Enter to submit, Esc or Ctrl+C to quit.".to_string(),
        ]
    });

    let input_for_handler = input_state.clone();
    let messages_for_handler = messages.clone();
    let scroll_for_handler = scroll.clone();

    use_input(move |input, key| {
        if key.escape || (key.ctrl && input.eq_ignore_ascii_case("c")) {
            app.exit();
            return;
        }

        let (width, _) = rnk::renderer::Terminal::size().unwrap_or((80, 24));
        let text_width = input_viewport_width(width as usize);

        if key.return_key {
            let submitted = input_for_handler.get().submitted_text();
            if submitted.trim().is_empty() {
                return;
            }

            messages_for_handler.update(|items| {
                items.push(format!("You: {}", submitted));
                items.push("Assistant: This demo records submissions locally.".to_string());
            });
            input_for_handler.update(|state| state.clear());
            scroll_for_handler.scroll_to_bottom();
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

            state.keep_cursor_visible(text_width);
        });
    });

    let (width, height) = rnk::renderer::Terminal::size().unwrap_or((80, 24));
    let width = width as usize;
    let height = height as usize;
    let bottom_height = 4;
    let content_height = height.saturating_sub(bottom_height).max(1);
    let current_messages = messages.get();

    scroll.set_content_size(width, current_messages.len());
    scroll.set_viewport_size(width, content_height);

    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(height as i32)
        .width(width as i32)
        .child(
            ScrollableBox::new()
                .height(content_height as i32)
                .scroll_offset_y(scroll.offset_y() as u16)
                .flex_direction(FlexDirection::Column)
                .children(
                    current_messages
                        .iter()
                        .map(|message| render_message(message)),
                )
                .into_element(),
        )
        .child(render_claude_input_box(&input_state.get(), width))
        .child(render_status_bar(
            width,
            scroll.offset_y(),
            current_messages.len(),
        ))
        .into_element()
}

fn render_message(message: &str) -> Element {
    let is_user = message.starts_with("You:");
    let prefix = if is_user { "❯ " } else { "● " };
    let color = if is_user { Color::Cyan } else { Color::White };
    let prefix_color = if is_user {
        Color::Cyan
    } else {
        Color::BrightGreen
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(prefix).color(prefix_color).into_element())
        .child(Text::new(message.to_owned()).color(color).into_element())
        .into_element()
}

fn render_claude_input_box(state: &ClaudeInputState, width: usize) -> Element {
    let border = "─".repeat(width);
    let (before, cursor, after) = visible_input_segments(state, input_viewport_width(width));

    Box::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(border.clone())
                .color(Color::Ansi256(240))
                .into_element(),
        )
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new("❯ ")
                        .color(Color::BrightCyan)
                        .bold()
                        .into_element(),
                )
                .child(Text::new(before).color(Color::White).into_element())
                .child(
                    Text::new(cursor)
                        .color(Color::Black)
                        .background(Color::BrightCyan)
                        .into_element(),
                )
                .child(Text::new(after).color(Color::White).into_element())
                .into_element(),
        )
        .child(Text::new(border).color(Color::Ansi256(240)).into_element())
        .into_element()
}

fn render_status_bar(width: usize, scroll_offset: usize, total_messages: usize) -> Element {
    let status = format!(
        " Enter submit  Esc/Ctrl+C quit  ←/→ move  Home/End jump  [{}/{}]",
        scroll_offset.saturating_add(1),
        total_messages.max(1)
    );

    Text::new(truncate_to_width(&status, width))
        .color(Color::Ansi256(245))
        .dim()
        .into_element()
}

fn input_viewport_width(width: usize) -> usize {
    let prompt_width = UnicodeWidthStr::width("❯ ");
    width.saturating_sub(prompt_width + 1).max(1)
}

fn visible_input_segments(
    state: &ClaudeInputState,
    viewport_width: usize,
) -> (String, String, String) {
    let start = state.scroll_offset;
    let end = start + viewport_width;
    let cursor_col = state.cursor_visual_col();
    let cursor_char = state.chars.get(state.cursor_pos).copied().unwrap_or(' ');
    let cursor_width = cursor_char.width().unwrap_or(1).max(1);

    let mut before = String::new();
    let mut after = String::new();
    let mut col = 0;

    for (index, ch) in state.chars.iter().enumerate() {
        let ch_width = ch.width().unwrap_or(0);
        let fully_visible = col >= start && col + ch_width <= end;

        if fully_visible {
            if index < state.cursor_pos {
                before.push(*ch);
            } else if index > state.cursor_pos {
                after.push(*ch);
            }
        }

        col += ch_width;
    }

    let cursor_visible = cursor_col >= start && cursor_col + cursor_width <= end;
    let cursor = if cursor_visible {
        cursor_char.to_string()
    } else {
        " ".to_string()
    };

    (before, cursor, after)
}

fn truncate_to_width(input: &str, max_width: usize) -> String {
    let mut output = String::new();
    let mut width = 0;

    for ch in input.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        output.push(ch);
        width += ch_width;
    }

    output
}
