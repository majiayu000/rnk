//! ARIA accessibility example - demonstrating screen reader support
//!
//! Equivalent to ink's examples/aria
//!
//! Run with: cargo run --example aria

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let selected = use_signal(|| 0usize);
    let is_screen_reader = use_is_screen_reader_enabled();

    let items = vec![
        ("apple", "Apple"),
        ("banana", "Banana"),
        ("cherry", "Cherry"),
        ("date", "Date"),
        ("elderberry", "Elderberry"),
    ];

    let items_len = items.len();
    let selected_clone = selected.clone();

    use_input(move |ch, key| {
        if ch == "q" {
            app.exit();
        } else if key.up_arrow || ch == "k" {
            selected_clone.update(|s| {
                if *s == 0 {
                    *s = items_len - 1;
                } else {
                    *s -= 1;
                }
            });
        } else if key.down_arrow || ch == "j" {
            selected_clone.update(|s| {
                *s = (*s + 1) % items_len;
            });
        } else if is_screen_reader {
            // Number keys for screen reader users
            if let Some(num) = ch.chars().next().and_then(|c| c.to_digit(10)) {
                let idx = num as usize;
                if idx > 0 && idx <= items_len {
                    selected_clone.set(idx - 1);
                }
            }
        }
    });

    let current_selected = selected.get();

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
                    Text::new("Accessibility Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Screen reader status
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("Screen Reader: ").into_element())
                .child(if is_screen_reader {
                    Text::new("Enabled")
                        .color(Color::Green)
                        .bold()
                        .into_element()
                } else {
                    Text::new("Disabled")
                        .color(Color::Ansi256(245))
                        .into_element()
                })
                .into_element(),
        )
        .child(Newline::new().into_element())
        // List heading
        .child(
            Text::new("Select a fruit:")
                .color(Color::Yellow)
                .bold()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Accessible list
        .children(items.iter().enumerate().map(|(idx, (_id, label))| {
            let is_selected = idx == current_selected;

            // For screen readers, include number prefix
            let display_label = if is_screen_reader {
                format!("{}. {}", idx + 1, label)
            } else {
                format!("{}{}", if is_selected { "> " } else { "  " }, label)
            };

            Box::new()
                .flex_direction(FlexDirection::Row)
                .background(
                    if is_selected {
                        Some(Color::Ansi256(236))
                    } else {
                        None
                    }
                    .unwrap_or(Color::Black),
                )
                .child(
                    Text::new(display_label)
                        .color(if is_selected {
                            Color::Cyan
                        } else {
                            Color::White
                        })
                        .bold()
                        .into_element(),
                )
                .child(if is_selected && is_screen_reader {
                    Text::new(" (selected)").color(Color::Green).into_element()
                } else {
                    Box::new().into_element()
                })
                .into_element()
        }))
        .child(Newline::new().into_element())
        // Help text
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Controls:").dim().into_element())
                .child(Text::new("  j/k or arrows - Navigate").dim().into_element())
                .child(if is_screen_reader {
                    Text::new("  1-5 - Jump to item (screen reader mode)")
                        .dim()
                        .into_element()
                } else {
                    Box::new().into_element()
                })
                .child(Text::new("  q - Quit").dim().into_element())
                .into_element(),
        )
        .into_element()
}
