//! Cross-thread render example
//!
//! Demonstrates how to update UI from a background thread using `request_render()`.

use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use rnk::prelude::*;

/// Shared state that can be updated from any thread
struct AppState {
    counter: i32,
    messages: Vec<String>,
}

fn main() -> std::io::Result<()> {
    // Create shared state
    let state = Arc::new(RwLock::new(AppState {
        counter: 0,
        messages: vec!["Starting...".to_string()],
    }));

    // Clone for background thread
    let state_clone = Arc::clone(&state);

    // Spawn background thread that updates state
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));

            // Update state
            {
                let mut s = state_clone.write().unwrap();
                s.counter += 1;
                let count = s.counter;
                s.messages
                    .push(format!("Update #{} from background thread", count));

                // Keep only last 5 messages
                if s.messages.len() > 5 {
                    s.messages.remove(0);
                }
            }

            // Notify rnk to re-render
            rnk::request_render();
        }
    });

    // Run the app with a closure that reads the shared state
    let state_for_render = Arc::clone(&state);
    render(move || app(&state_for_render)).run()
}

fn app(state: &Arc<RwLock<AppState>>) -> Element {
    let s = state.read().unwrap();

    let mut container = Box::new().flex_direction(FlexDirection::Column).padding(1);

    // Title
    container = container.child(
        Text::new("Cross-Thread Render Demo")
            .bold()
            .color(Color::Cyan)
            .into_element(),
    );

    container = container.child(Text::new("─".repeat(30)).dim().into_element());

    // Counter
    container = container.child(
        Box::new()
            .child(
                Text::new(format!("Counter: {}", s.counter))
                    .color(Color::Green)
                    .into_element(),
            )
            .into_element(),
    );

    container = container.child(Text::new("").into_element());

    // Messages
    container = container.child(Text::new("Recent messages:").bold().into_element());

    for msg in &s.messages {
        container = container.child(Text::new(format!("  • {}", msg)).dim().into_element());
    }

    container = container.child(Text::new("").into_element());

    container = container.child(Text::new("Press Ctrl+C to exit").dim().into_element());

    container.into_element()
}
