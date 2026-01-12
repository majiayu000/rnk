//! Mouse demo - demonstrating mouse input handling
//!
//! Run with: cargo run --example mouse_demo

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let mouse_x = use_signal(|| 0u16);
    let mouse_y = use_signal(|| 0u16);
    let last_action = use_signal(|| String::from("None"));
    let click_count = use_signal(|| 0u32);

    use_input(move |ch, _key| {
        if ch == "q" {
            app.exit();
        }
    });

    // Clone signals for use in mouse handler closure
    let mouse_x_clone = mouse_x.clone();
    let mouse_y_clone = mouse_y.clone();
    let last_action_clone = last_action.clone();
    let click_count_clone = click_count.clone();

    use_mouse(move |mouse| {
        mouse_x_clone.set(mouse.x);
        mouse_y_clone.set(mouse.y);

        let action = match mouse.action {
            MouseAction::Press(btn) => {
                click_count_clone.set(click_count_clone.get() + 1);
                format!("Press {:?}", btn)
            }
            MouseAction::Release(btn) => format!("Release {:?}", btn),
            MouseAction::Drag(btn) => format!("Drag {:?}", btn),
            MouseAction::Move => "Move".to_string(),
            MouseAction::ScrollUp => "Scroll Up".to_string(),
            MouseAction::ScrollDown => "Scroll Down".to_string(),
            MouseAction::ScrollLeft => "Scroll Left".to_string(),
            MouseAction::ScrollRight => "Scroll Right".to_string(),
        };

        let modifiers = format!(
            "{}{}{}",
            if mouse.ctrl { "Ctrl+" } else { "" },
            if mouse.shift { "Shift+" } else { "" },
            if mouse.alt { "Alt+" } else { "" }
        );

        last_action_clone.set(format!("{}{}", modifiers, action));
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Text::new("Mouse Demo")
                .color(Color::Cyan)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Instructions
        .child(
            Text::new("Move your mouse, click, scroll, or drag to see events.")
                .color(Color::Yellow)
                .into_element(),
        )
        .child(
            Text::new("Hold Ctrl/Shift/Alt to see modifier detection.")
                .color(Color::Yellow)
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Mouse position
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Ansi256(240))
                .padding_x(1.0)
                .margin_bottom(1.0)
                .child(
                    Text::spans(vec![
                        Span::new("Position: ").color(Color::White),
                        Span::new("(").color(Color::Ansi256(240)),
                        Span::new(format!("{}", mouse_x.get()))
                            .color(Color::Green)
                            .bold(),
                        Span::new(", ").color(Color::Ansi256(240)),
                        Span::new(format!("{}", mouse_y.get()))
                            .color(Color::Green)
                            .bold(),
                        Span::new(")").color(Color::Ansi256(240)),
                    ])
                    .into_element(),
                )
                .into_element(),
        )
        // Last action
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Ansi256(240))
                .padding_x(1.0)
                .margin_bottom(1.0)
                .child(
                    Text::spans(vec![
                        Span::new("Last Action: ").color(Color::White),
                        Span::new(last_action.get()).color(Color::Magenta).bold(),
                    ])
                    .into_element(),
                )
                .into_element(),
        )
        // Click counter
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Ansi256(240))
                .padding_x(1.0)
                .child(
                    Text::spans(vec![
                        Span::new("Click Count: ").color(Color::White),
                        Span::new(format!("{}", click_count.get()))
                            .color(Color::Cyan)
                            .bold(),
                    ])
                    .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(Text::new("Press 'q' to exit").dim().into_element())
        .into_element()
}
