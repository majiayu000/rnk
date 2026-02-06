//! rnk-chat: A terminal chat client built with rnk
//!
//! Demonstrates rnk's capabilities for building chat-style interfaces.
//!
//! Run with: cargo run --example rnk_chat

use rnk::prelude::*;

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

fn app() -> Element {
    let messages = use_signal(initial_messages);
    let input_text = use_signal(String::new);
    let scroll_offset = use_signal(|| 0usize);
    let is_typing = use_signal(|| false);
    let app = use_app();

    // Clone for input handler
    let messages_input = messages.clone();
    let input_text_input = input_text.clone();
    let scroll_offset_input = scroll_offset.clone();
    let is_typing_input = is_typing.clone();

    use_input(move |input, key| {
        // Exit
        if key.escape || (key.ctrl && input == "c") {
            app.exit();
            return;
        }

        // Scroll
        if key.page_up {
            scroll_offset_input.update(|s| *s = s.saturating_sub(5));
            return;
        }
        if key.page_down {
            let msg_count = messages_input.get().len();
            scroll_offset_input.update(|s| *s = (*s + 5).min(msg_count.saturating_sub(1)));
            return;
        }

        // Send message
        if key.return_key {
            let text = input_text_input.get();
            if !text.trim().is_empty() {
                // Add user message
                messages_input.update(|msgs| {
                    msgs.push(ChatMessage {
                        role: Role::User,
                        content: text.clone(),
                        timestamp: current_time(),
                    });
                });
                input_text_input.set(String::new());

                // Simulate typing indicator
                is_typing_input.set(true);

                // Add mock assistant response after a delay
                let messages_clone = messages_input.clone();
                let is_typing_clone = is_typing_input.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    is_typing_clone.set(false);
                    messages_clone.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: Role::Assistant,
                            content: generate_response(&text),
                            timestamp: current_time(),
                        });
                    });
                });

                // Scroll to bottom
                let msg_count = messages_input.get().len();
                scroll_offset_input.set(msg_count.saturating_sub(8));
            }
            return;
        }

        // Backspace
        if key.backspace {
            input_text_input.update(|t| {
                t.pop();
            });
            return;
        }

        // Regular character input
        if !input.is_empty() && !key.ctrl && !key.alt {
            input_text_input.update(|t| t.push_str(input));
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .children(vec![
            header(),
            message_list(&messages.get(), scroll_offset.get(), is_typing.get()),
            input_area(&input_text.get()),
            footer(),
        ])
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

fn message_list(messages: &[ChatMessage], scroll_offset: usize, is_typing: bool) -> Element {
    let mut children = Vec::new();

    let visible_messages: Vec<_> = messages.iter().skip(scroll_offset).take(12).collect();

    for msg in visible_messages {
        children.push(render_message(msg));
    }

    // Typing indicator
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

    // Empty state
    if messages.is_empty() {
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
                .max_width(60)
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

fn input_area(text: &str) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .padding_y(0.5)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .margin_x(1.0)
        .children(vec![
            Text::new("> ").color(Color::Cyan).bold().into_element(),
            Text::new(if text.is_empty() {
                "Type a message..."
            } else {
                text
            })
            .color(if text.is_empty() {
                Color::BrightBlack
            } else {
                Color::White
            })
            .into_element(),
            Text::new("█").color(Color::Cyan).into_element(),
        ])
        .into_element()
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
        .unwrap()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    format!("{:02}:{:02}", hours, minutes)
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
        format!(
            "I received your message: \"{}\". How can I assist you further?",
            if input.len() > 30 {
                &input[..30]
            } else {
                input
            }
        )
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
