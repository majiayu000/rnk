//! Example demonstrating new Message and Spinner components
//!
//! This example shows how to use the new chat-style components:
//! - Message component for different roles
//! - ToolCall and ThinkingBlock components
//! - Spinner component for loading animations
//!
//! Press Enter to see different message types

use rnk::prelude::*;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Print banner
    println!();
    rnk::println(
        Box::new()
            .border_style(BorderStyle::Round)
            .border_color(Color::Cyan)
            .padding(1)
            .child(
                Text::new("Message Components Demo")
                    .color(Color::Cyan)
                    .bold()
                    .into_element(),
            )
            .child(
                Text::new("Press Enter to see examples, 'q' to quit")
                    .dim()
                    .into_element(),
            )
            .into_element(),
    );
    println!();

    // Run interactive demo
    render(app).run()
}

fn app() -> Element {
    let step = use_signal(|| 0u32);
    let app = use_app();

    let current_step = step.clone();
    let app_clone = app.clone();
    use_input(move |input, key| match input {
        "q" => app_clone.exit(),
        _ if key.return_key => {
            let s = current_step.get();
            show_step(s);
            current_step.update(|v| *v += 1);
        }
        _ => {}
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .border_style(BorderStyle::Round)
        .border_color(Color::Yellow)
        .child(
            Text::new(format!(
                "Step: {} - Press Enter for next example",
                step.get()
            ))
            .color(Color::Yellow)
            .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(Text::new("Examples:").dim().into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .flex_direction(FlexDirection::Column)
                .child(Text::new("  0: User message").dim().into_element())
                .child(Text::new("  1: Assistant message").dim().into_element())
                .child(Text::new("  2: System message").dim().into_element())
                .child(Text::new("  3: Tool call").dim().into_element())
                .child(Text::new("  4: Tool result").dim().into_element())
                .child(Text::new("  5: Thinking block").dim().into_element())
                .child(Text::new("  6: Error message").dim().into_element())
                .child(Text::new("  7: Spinner demo").dim().into_element())
                .into_element(),
        )
        .into_element()
}

fn show_step(step: u32) {
    match step {
        0 => {
            // User message
            println!();
            rnk::println(Message::user("Hello, how can you help me today?").into_element());
        }
        1 => {
            // Assistant message
            println!();
            rnk::println(
                Message::assistant("I can help you with various tasks! What would you like to do?")
                    .into_element(),
            );
        }
        2 => {
            // System message
            println!();
            rnk::println(Message::system("System initialized successfully").into_element());
        }
        3 => {
            // Tool call
            println!();
            rnk::println(ToolCall::new("read_file", "path=/tmp/test.txt").into_element());
        }
        4 => {
            // Tool result
            println!();
            rnk::println(Message::tool_result("File read successfully: 42 lines").into_element());
        }
        5 => {
            // Thinking block
            println!();
            rnk::println(
                ThinkingBlock::new(
                    "Let me analyze this problem...\n\
                     First, I need to understand the requirements\n\
                     Then, I'll consider different approaches\n\
                     Finally, I'll choose the best solution\n\
                     This might take a moment...",
                )
                .max_lines(3)
                .into_element(),
            );
        }
        6 => {
            // Error message
            println!();
            rnk::println(Message::error("Failed to connect to server").into_element());
        }
        7 => {
            // Spinner demo
            println!();
            println!("Starting spinner (will auto-stop after 2 seconds)...");
            let spinner = Spinner::builder()
                .message("Processing...")
                .cancellable(false)
                .build();
            std::thread::sleep(Duration::from_secs(2));
            spinner.stop();
            println!("Done!");
        }
        _ => {
            println!();
            println!("Demo complete! Press 'q' to quit.");
        }
    }
}
