//! Use focus with ID example - demonstrating focus management with explicit IDs
//!
//! Equivalent to ink's examples/use-focus-with-id
//!
//! Run with: cargo run --example use_focus_with_id

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let focus_manager = use_focus_manager();

    let focus_manager_clone = focus_manager.clone();

    use_input(move |ch, key| {
        match ch {
            "q" => app.exit(),
            "1" => focus_manager_clone.focus("input-1"),
            "2" => focus_manager_clone.focus("input-2"),
            "3" => focus_manager_clone.focus("input-3"),
            _ => {}
        }
        // Handle Tab for navigation
        if key.tab {
            if key.shift {
                focus_manager_clone.focus_previous();
            } else {
                focus_manager_clone.focus_next();
            }
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Text::new("Focus with ID Demo")
                .color(Color::Cyan)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Instructions
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Yellow)
                .padding(1)
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Press number keys to focus specific inputs:").into_element())
                .child(Text::new("  1 - Focus 'Username'").dim().into_element())
                .child(Text::new("  2 - Focus 'Email'").dim().into_element())
                .child(Text::new("  3 - Focus 'Password'").dim().into_element())
                .child(
                    Text::new("  Tab - Next, Shift+Tab - Previous")
                        .dim()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Form inputs with IDs
        .child(input_field("Username", "input-1"))
        .child(input_field("Email", "input-2"))
        .child(input_field("Password", "input-3"))
        .child(Newline::new().into_element())
        // Submit button (also focusable)
        .child(submit_button())
        .child(Newline::new().into_element())
        .child(Text::new("Press 'q' to quit").dim().into_element())
        .into_element()
}

fn input_field(label: &str, id: &str) -> Element {
    let focus = use_focus(UseFocusOptions {
        auto_focus: id == "input-1", // Auto-focus first input
        id: Some(id.to_string()),
        is_active: true,
    });

    let border_color = if focus.is_focused {
        Color::Cyan
    } else {
        Color::Ansi256(240)
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .margin_bottom(1.0)
        .child(
            Box::new()
                .width(12)
                .child(
                    Text::new(format!("{}:", label))
                        .color(Color::White)
                        .into_element(),
                )
                .into_element(),
        )
        .child(
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(border_color)
                .width(30)
                .padding_x(1.0)
                .child(
                    Text::new(if focus.is_focused { "_" } else { "" })
                        .color(Color::Cyan)
                        .into_element(),
                )
                .into_element(),
        )
        .child(if focus.is_focused {
            Text::new(" < focused")
                .color(Color::Green)
                .italic()
                .into_element()
        } else {
            Box::new().into_element()
        })
        .into_element()
}

fn submit_button() -> Element {
    let focus = use_focus(UseFocusOptions {
        auto_focus: false,
        id: Some("submit".to_string()),
        is_active: true,
    });

    let bg_color = if focus.is_focused {
        Color::Green
    } else {
        Color::Ansi256(240)
    };

    Box::new()
        .background(bg_color)
        .padding_x(3.0)
        .padding_y(0.5)
        .child(
            Text::new("Submit")
                .color(if focus.is_focused {
                    Color::Black
                } else {
                    Color::White
                })
                .bold()
                .into_element(),
        )
        .into_element()
}
