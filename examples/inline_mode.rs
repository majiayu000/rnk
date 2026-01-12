//! Inline mode example demonstrating runtime mode switching and println
//!
//! This example shows:
//! - Default inline mode rendering (output persists in terminal history)
//! - Runtime mode switching between inline and fullscreen
//! - Println for persistent messages above the UI
//!
//! Controls:
//! - Space: Toggle between inline and fullscreen mode
//! - Enter: Print a message (only visible in inline mode)
//! - q: Quit

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    // Default is inline mode - output persists in terminal history
    render(app).run()

    // Alternative: start in fullscreen mode
    // render(app).fullscreen().run()
}

fn app() -> Element {
    let app = use_app();
    let message_count = use_signal(|| 0u32);

    // Handle keyboard input
    let count = message_count.clone();
    let app_clone = app.clone();
    use_input(move |input, key| {
        match input {
            "q" => app_clone.exit(),
            " " => {
                // Toggle between inline and fullscreen mode
                if app_clone.is_alt_screen() {
                    app_clone.exit_alt_screen();
                    app_clone.println("Switched to inline mode");
                } else {
                    app_clone.println("Switching to fullscreen mode...");
                    app_clone.enter_alt_screen();
                }
            }
            _ if key.return_key => {
                // Print a persistent message (only works in inline mode)
                count.update(|c| *c += 1);
                app_clone.println(format!("Message #{}: Hello from rnk!", count.get()));
            }
            _ => {}
        }
    });

    let is_fullscreen = app.is_alt_screen();
    let mode_text = if is_fullscreen {
        "Fullscreen"
    } else {
        "Inline"
    };
    let mode_color = if is_fullscreen {
        Color::Magenta
    } else {
        Color::Cyan
    };

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .border_style(BorderStyle::Round)
        .border_color(mode_color)
        .child(
            Text::new("Inline Mode Demo")
                .color(mode_color)
                .bold()
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(
                    Text::new(format!("Current mode: {}", mode_text))
                        .color(mode_color)
                        .into_element(),
                )
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(
                    Text::new(format!("Messages printed: {}", message_count.get())).into_element(),
                )
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Controls:").dim().into_element())
                .child(
                    Text::new("  Space: Toggle inline/fullscreen mode")
                        .dim()
                        .into_element(),
                )
                .child(
                    Text::new("  Enter: Print message (inline mode only)")
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
                    Text::new(if is_fullscreen {
                        "Note: Println is ignored in fullscreen mode"
                    } else {
                        "Note: Messages persist above the UI in inline mode"
                    })
                    .italic()
                    .color(Color::Yellow)
                    .into_element(),
                )
                .into_element(),
        )
        .into_element()
}
