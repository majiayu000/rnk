//! Terminal resize example - responding to terminal size changes
//!
//! Equivalent to ink's examples/terminal-resize
//!
//! Run with: cargo run --example terminal_resize

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

fn app() -> Element {
    let app = use_app();
    let term_size = use_signal(|| crossterm::terminal::size().unwrap_or((80, 24)));

    let term_size_clone = term_size.clone();
    use_input(move |ch, _key| {
        if ch == "q" {
            app.exit();
        }
        // Update terminal size on any key press (simple approach)
        if let Ok(size) = crossterm::terminal::size() {
            term_size_clone.set(size);
        }
    });

    let (width, height) = term_size.get();
    let width = width as u32;
    let height = height as u32;

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Cyan)
                .padding(1)
                .child(
                    Text::new("Terminal Resize Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Current dimensions
        .child(
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Yellow)
                .padding(1)
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("Current Terminal Size:")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(Newline::new().into_element())
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("  Width:  ").into_element())
                        .child(
                            Text::new(format!("{} columns", width))
                                .color(Color::Green)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("  Height: ").into_element())
                        .child(
                            Text::new(format!("{} rows", height))
                                .color(Color::Green)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Size indicator
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new(format!(
                        "Size category: {}",
                        if width < 40 {
                            "Small"
                        } else if width < 80 {
                            "Medium"
                        } else if width < 120 {
                            "Large"
                        } else {
                            "Extra Large"
                        }
                    ))
                    .color(Color::Magenta)
                    .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Visual size bar
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Width visualization:").dim().into_element())
                .child(
                    Text::new("█".repeat((width as usize).min(60)))
                        .color(Color::Blue)
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Instructions
        .child(
            Text::new("Try resizing your terminal window!")
                .color(Color::Ansi256(245))
                .italic()
                .into_element(),
        )
        .child(Text::new("Press 'q' to quit").dim().into_element())
        .into_element()
}
