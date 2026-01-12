//! Rich text example - demonstrating Span and Line for mixed styles
//!
//! Run with: cargo run --example rich_text

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();

    use_input(move |ch, _key| {
        if ch == "q" {
            app.exit();
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Text::new("Rich Text Demo")
                .color(Color::Cyan)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Simple text (single style)
        .child(
            Text::new("1. Simple text with single style")
                .color(Color::Yellow)
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Rich text with Spans (multiple styles on one line)
        .child(
            Text::spans(vec![
                Span::new("2. ").color(Color::Yellow),
                Span::new("Multiple ").color(Color::Red).bold(),
                Span::new("styles ").color(Color::Green).italic(),
                Span::new("on ").color(Color::Blue).underline(),
                Span::new("one ").color(Color::Magenta),
                Span::new("line!").color(Color::Cyan).bold(),
            ])
            .into_element(),
        )
        .child(Newline::new().into_element())
        // Using Line builder
        .child(
            Text::line(
                Line::new()
                    .span(Span::new("3. Using ").color(Color::Yellow))
                    .span(Span::new("Line").color(Color::Green).bold())
                    .span(Span::new(" builder for ").color(Color::White))
                    .span(Span::new("fluent API").color(Color::Cyan).italic()),
            )
            .into_element(),
        )
        .child(Newline::new().into_element())
        // Status line example
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Ansi256(240))
                .padding_x(1.0)
                .child(
                    Text::spans(vec![
                        Span::new("Status: ").color(Color::White),
                        Span::new("● ").color(Color::Green),
                        Span::new("Online").color(Color::Green).bold(),
                        Span::new(" | ").color(Color::Ansi256(240)),
                        Span::new("Users: ").color(Color::White),
                        Span::new("42").color(Color::Cyan).bold(),
                        Span::new(" | ").color(Color::Ansi256(240)),
                        Span::new("Errors: ").color(Color::White),
                        Span::new("0").color(Color::Green),
                    ])
                    .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Log output example
        .child(
            Text::new("Log Output Example:")
                .color(Color::Yellow)
                .into_element(),
        )
        .child(
            Text::spans(vec![
                Span::new("[").color(Color::Ansi256(240)),
                Span::new("INFO").color(Color::Blue).bold(),
                Span::new("] ").color(Color::Ansi256(240)),
                Span::new("Application started successfully").color(Color::White),
            ])
            .into_element(),
        )
        .child(
            Text::spans(vec![
                Span::new("[").color(Color::Ansi256(240)),
                Span::new("WARN").color(Color::Yellow).bold(),
                Span::new("] ").color(Color::Ansi256(240)),
                Span::new("Cache miss for key 'user_123'").color(Color::White),
            ])
            .into_element(),
        )
        .child(
            Text::spans(vec![
                Span::new("[").color(Color::Ansi256(240)),
                Span::new("ERROR").color(Color::Red).bold(),
                Span::new("] ").color(Color::Ansi256(240)),
                Span::new("Connection timeout after 30s").color(Color::White),
            ])
            .into_element(),
        )
        .child(Newline::new().into_element())
        .child(Text::new("Press 'q' to exit").dim().into_element())
        .into_element()
}
