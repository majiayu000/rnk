//! Example demonstrating println with Element support
//!
//! This example shows how to use rnk::println() to print both text and
//! complex UI elements above the running application.
//!
//! Controls:
//! - Enter: Print a styled banner
//! - Space: Print a simple text message
//! - q: Quit

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    // Run in inline mode (default) to see println output
    render(app).run()
}

fn app() -> Element {
    let counter = use_signal(|| 0u32);
    let app = use_app();

    let count = counter.clone();
    let app_clone = app.clone();
    use_input(move |input, key| {
        match input {
            "q" => app_clone.exit(),
            " " => {
                // Print simple text
                let current = count.get();
                count.update(|c| *c += 1);
                rnk::println(format!("Message #{}: Hello from rnk!", current + 1));
            }
            _ if key.return_key => {
                // Print a styled element
                let banner = create_banner(count.get() + 1);
                rnk::println(banner);
                count.update(|c| *c += 1);
            }
            _ => {}
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .child(
            Text::new("println() with Element Support")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(Text::new(format!("Messages printed: {}", counter.get())).into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Controls:").dim().into_element())
                .child(
                    Text::new("  Space: Print simple text message")
                        .dim()
                        .into_element(),
                )
                .child(
                    Text::new("  Enter: Print styled banner element")
                        .dim()
                        .into_element(),
                )
                .child(Text::new("  q:     Quit").dim().into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(
                    Text::new("Note: Messages persist above this UI in terminal history")
                        .italic()
                        .color(Color::Yellow)
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}

/// Create a styled banner element
fn create_banner(number: u32) -> Element {
    Box::new()
        .border_style(BorderStyle::Double)
        .border_color(Color::Magenta)
        .padding(1)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new(format!("🎉 Banner #{}", number))
                        .color(Color::Magenta)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .margin_top(1.0)
                        .child(
                            Text::new("This is a complex UI element printed via rnk::println()")
                                .color(Color::White)
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .margin_top(1.0)
                        .child(
                            Text::new("✓ Supports borders, colors, padding, and layout")
                                .color(Color::Green)
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}
