//! Streaming Output Demo
//!
//! Demonstrates rnk's support for streaming/incremental output:
//!
//! 1. **Line-level diff rendering**: Only changed lines are redrawn (like Ink/Bubbletea)
//! 2. **Static component**: Permanent output that persists above dynamic UI
//! 3. **rnk::println()**: Print messages that persist (like Bubbletea's tea.Println())
//!
//! Run with: cargo run --example streaming_demo

use std::time::Duration;

use rnk::hooks::use_signal;
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    println!("Streaming Output Demo\n");
    println!("This demo shows:");
    println!("  1. Static component - logs that persist above the UI");
    println!("  2. Line-level diff rendering - only changed lines update");
    println!("  3. Simulated streaming - progressive text updates\n");
    println!("Press Ctrl+C to exit.\n");

    // Start background thread before entering the render loop
    std::thread::spawn(|| {
        let mut tick = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(100));
            tick += 1;
            rnk::request_render();

            // Print permanent log every 20 ticks
            if tick % 20 == 0 {
                rnk::println(format!("[LOG] Tick {} completed", tick));
            }
        }
    });

    render(app).run()
}

fn app() -> Element {
    // Counter for the dynamic UI
    let counter = use_signal(|| 0);

    // Streaming text simulation
    let stream_text = use_signal(|| String::new());
    let full_text = "Hello! This is simulated streaming text that appears progressively...";

    // Update state on each render
    let current = counter.get();
    counter.set(current + 1);

    // Simulate streaming text
    let char_index = (current as usize) % (full_text.len() + 10);
    if char_index < full_text.len() {
        let current_text: String = full_text.chars().take(char_index + 1).collect();
        stream_text.set(current_text);
    } else if char_index == full_text.len() {
        stream_text.set(full_text.to_string());
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        // Separator
        .child(
            Text::new("─".repeat(50))
                .color(Color::Ansi256(240))
                .into_element(),
        )
        // Dynamic UI - this part updates with line-level diff
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .padding(1)
                .child(
                    Text::new("Dynamic UI (line-level diff rendering)")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .child(Newline::new().into_element())
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("Counter: ").color(Color::White).into_element())
                        .child(
                            Text::new(format!("{}", counter.get()))
                                .color(Color::Green)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Newline::new().into_element())
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("Streaming text:")
                                .color(Color::Yellow)
                                .into_element(),
                        )
                        .child(
                            Text::new(format!("{}_", stream_text.get()))
                                .color(Color::White)
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Newline::new().into_element())
                .child(
                    Text::new("Note: Only changed lines are redrawn! Logs above persist.")
                        .dim()
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}
