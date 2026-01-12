//! Use focus example - demonstrating focus management
//!
//! Equivalent to ink's examples/use-focus
//!
//! Run with: cargo run --example use_focus

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
            Text::new("Focus Management Demo")
                .color(Color::Cyan)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("Press Tab to move focus, Shift+Tab to move backwards")
                .dim()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Focusable items
        .child(focusable_item("First", 0))
        .child(focusable_item("Second", 1))
        .child(focusable_item("Third", 2))
        .child(focusable_item("Fourth", 3))
        .child(Newline::new().into_element())
        .child(Text::new("Press 'q' to quit").dim().into_element())
        .into_element()
}

fn focusable_item(label: &str, _index: usize) -> Element {
    let focus = use_focus(UseFocusOptions::default());

    let border_color = if focus.is_focused {
        Color::Green
    } else {
        Color::Ansi256(240)
    };

    let text_color = if focus.is_focused {
        Color::Green
    } else {
        Color::White
    };

    Box::new()
        .border_style(BorderStyle::Round)
        .border_color(border_color)
        .padding_x(2.0)
        .margin_bottom(1.0)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new(if focus.is_focused { "> " } else { "  " })
                        .color(Color::Green)
                        .bold()
                        .into_element(),
                )
                .child(Text::new(label).color(text_color).bold().into_element())
                .child(if focus.is_focused {
                    Text::new(" (focused)")
                        .color(Color::Green)
                        .italic()
                        .into_element()
                } else {
                    Box::new().into_element()
                })
                .into_element(),
        )
        .into_element()
}
