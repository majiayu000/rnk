//! Use input example - demonstrating keyboard input handling
//!
//! Equivalent to ink's examples/use-input
//!
//! Run with: cargo run --example use_input

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

fn app() -> Element {
    let app = use_app();
    let last_key = use_signal(|| String::from("(none)"));
    let key_history = use_signal(|| Vec::<String>::new());
    let raw_mode = use_signal(|| true);

    let last_key_clone = last_key.clone();
    let key_history_clone = key_history.clone();

    use_input(move |ch, key| {
        if ch == "q" && key.ctrl {
            app.exit();
            return;
        }

        let key_str = if key.up_arrow {
            "↑ Up Arrow".to_string()
        } else if key.down_arrow {
            "↓ Down Arrow".to_string()
        } else if key.left_arrow {
            "← Left Arrow".to_string()
        } else if key.right_arrow {
            "→ Right Arrow".to_string()
        } else if key.return_key {
            "↵ Enter".to_string()
        } else if key.escape {
            "Esc".to_string()
        } else if key.backspace {
            "⌫ Backspace".to_string()
        } else if key.delete {
            "Del".to_string()
        } else if key.tab {
            if key.shift {
                "⇧ Shift+Tab".to_string()
            } else {
                "⇥ Tab".to_string()
            }
        } else if key.ctrl {
            format!("Ctrl+{}", ch.to_uppercase())
        } else if key.alt {
            format!("Alt+{}", ch)
        } else if key.shift && ch.len() == 1 {
            format!("Shift+{}", ch)
        } else if !ch.is_empty() {
            format!("'{}'", ch)
        } else {
            "(unknown)".to_string()
        };

        last_key_clone.set(key_str.clone());
        key_history_clone.update(|history| {
            history.push(key_str);
            if history.len() > 10 {
                history.remove(0);
            }
        });
    });

    let current_last = last_key.get();
    let history = key_history.get();

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .child(
                    Text::new("Keyboard Input Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Current key display
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Yellow)
                .padding(2)
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Column)
                        .align_items(AlignItems::Center)
                        .child(Text::new("Last Key Pressed:").dim().into_element())
                        .child(
                            Text::new(&current_last)
                                .color(Color::Green)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Key history
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Ansi256(240))
                .padding(1)
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("Recent Keys:")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .children(history.iter().rev().enumerate().map(|(i, k)| {
                    let opacity = 255 - (i as u8 * 20).min(200);
                    Text::new(format!("  {}", k))
                        .color(Color::Ansi256(232 + (opacity / 10).min(23)))
                        .into_element()
                }))
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Help
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Try pressing different keys!").dim().into_element())
                .child(Text::new("• Arrow keys, Enter, Tab, Escape").dim().into_element())
                .child(Text::new("• Letters, numbers, symbols").dim().into_element())
                .child(Text::new("• Ctrl/Alt/Shift + key").dim().into_element())
                .child(Newline::new().into_element())
                .child(Text::new("Press Ctrl+Q to quit").dim().into_element())
                .into_element(),
        )
        .into_element()
}
