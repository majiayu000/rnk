//! Chat example - simple chat application with input
//!
//! Equivalent to ink's examples/chat
//!
//! Run with: cargo run --example chat

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let input = use_signal(|| String::new());
    let messages = use_signal(|| Vec::<String>::new());

    let input_clone = input.clone();
    let messages_clone = messages.clone();

    use_input(move |ch, key| {
        if ch == "q" && key.ctrl {
            app.exit();
        } else if key.return_key {
            let current_input = input_clone.get();
            if !current_input.is_empty() {
                messages_clone.update(|msgs| {
                    msgs.push(format!("You: {}", current_input));
                });
                input_clone.set(String::new());
            }
        } else if key.backspace || key.delete {
            input_clone.update(|s| {
                s.pop();
            });
        } else if !ch.is_empty() && !key.ctrl && !key.alt {
            input_clone.update(|s| s.push_str(ch));
        }
    });

    let current_messages = messages.get();
    let current_input = input.get();

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .child(
                    Text::new("Tink Chat")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Messages area
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .min_height(10.0)
                .children(
                    current_messages
                        .iter()
                        .map(|msg| Text::new(msg).color(Color::White).into_element()),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Input area
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new("Enter message: ")
                        .color(Color::Green)
                        .into_element(),
                )
                .child(
                    Text::new(format!("{}_", current_input))
                        .color(Color::White)
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Help
        .child(
            Text::new("Press Enter to send, Ctrl+Q to quit")
                .dim()
                .into_element(),
        )
        .into_element()
}
