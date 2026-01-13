//! Fixed Bottom Layout Demo
//!
//! Demonstrates rnk's support for fixed-bottom layout with scrollable content:
//! - Scrollable content area that fills remaining space
//! - Fixed bottom input + status bar
//! - Virtual scrolling for large lists
//!
//! Run with: cargo run --example fixed_bottom_demo

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).fullscreen().run()
}

fn app() -> Element {
    let scroll = use_scroll();
    let input_text = use_signal(|| String::new());
    let messages = use_signal(|| {
        (0..50)
            .map(|i| format!("Message {}: This is a sample message content", i))
            .collect::<Vec<_>>()
    });
    let permission_mode = use_signal(|| 0); // 0: normal, 1: bypass, 2: plan

    let app = use_app();

    // Set up scroll dimensions
    let message_count = messages.get().len();
    scroll.set_content_size(80, message_count);
    scroll.set_viewport_size(80, 15); // Approximate viewport height

    // Clone for closure
    let scroll_clone = scroll.clone();
    let input_text_clone = input_text.clone();
    let messages_clone = messages.clone();
    let permission_mode_clone = permission_mode.clone();

    // Handle input
    use_input(move |input, key| {
        if input == "q" {
            app.exit();
        } else if key.up_arrow {
            scroll_clone.scroll_up(1);
        } else if key.down_arrow {
            scroll_clone.scroll_down(1);
        } else if key.page_up {
            scroll_clone.page_up();
        } else if key.page_down {
            scroll_clone.page_down();
        } else if key.tab && key.shift {
            // Shift+Tab to cycle permission mode
            permission_mode_clone.update(|m| *m = (*m + 1) % 3);
        } else if key.backspace {
            input_text_clone.update(|t| {
                t.pop();
            });
        } else if key.return_key {
            // Add new message
            let text = input_text_clone.get();
            if !text.is_empty() {
                messages_clone.update(|msgs| {
                    msgs.push(format!("You: {}", text));
                });
                input_text_clone.set(String::new());
                // Auto-scroll to bottom
                scroll_clone.scroll_to_bottom();
            }
        } else if !input.is_empty() && input.chars().all(|c| !c.is_control()) {
            input_text_clone.update(|t| t.push_str(input));
        }
    });

    // Get terminal size for layout
    let (width, height) = rnk::renderer::Terminal::size().unwrap_or((80, 24));

    // Calculate available height for content (minus input line and status bar)
    let bottom_height = 3; // separator + input + status
    let content_height = height.saturating_sub(bottom_height);

    // Build the layout
    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(height as i32)
        .width(width as i32)
        // Scrollable content area
        .child(
            ScrollableBox::new()
                .height(content_height as i32)
                .scroll_offset_y(scroll.offset_y() as u16)
                .flex_direction(FlexDirection::Column)
                .children(messages.get().iter().map(|msg| {
                    let is_user = msg.starts_with("You:");
                    let color = if is_user { Color::Cyan } else { Color::White };
                    let prefix = if is_user { "❯ " } else { "● " };

                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(
                            Text::new(prefix)
                                .color(if is_user { Color::Cyan } else { Color::Green })
                                .into_element(),
                        )
                        .child(Text::new(msg.clone()).color(color).into_element())
                        .into_element()
                }))
                .into_element(),
        )
        // Separator line
        .child(
            Text::new("─".repeat(width as usize))
                .color(Color::Ansi256(240))
                .into_element(),
        )
        // Input line
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("❯ ").color(Color::Cyan).bold().into_element())
                .child(
                    Text::new(format!("{}█", input_text.get()))
                        .color(Color::White)
                        .into_element(),
                )
                .into_element(),
        )
        // Status bar
        .child(render_status_bar(
            permission_mode.get(),
            scroll.offset_y(),
            message_count,
        ))
        .into_element()
}

fn render_status_bar(mode: i32, scroll_offset: usize, total_messages: usize) -> Element {
    let mode_text = match mode {
        0 => "permissions required",
        1 => "bypass permissions on",
        2 => "plan mode",
        _ => "unknown",
    };

    let mode_color = match mode {
        0 => Color::Yellow,
        1 => Color::Red,
        2 => Color::Magenta,
        _ => Color::White,
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("▸▸ ").color(mode_color).into_element())
        .child(Text::new(mode_text).color(mode_color).into_element())
        .child(Text::new(" (shift+tab to cycle)").dim().into_element())
        .child(Spacer::new().into_element())
        .child(
            Text::new(format!("[{}/{}]", scroll_offset + 1, total_messages))
                .dim()
                .into_element(),
        )
        .into_element()
}
