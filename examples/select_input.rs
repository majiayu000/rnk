//! Select input example - list selection with keyboard navigation
//!
//! Equivalent to ink's examples/select-input
//!
//! Run with: cargo run --example select_input

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let selected = use_signal(|| 0usize);
    let confirmed = use_signal(|| Option::<String>::None);

    let items = vec![
        ("red", "Red", Color::Red),
        ("green", "Green", Color::Green),
        ("blue", "Blue", Color::Blue),
        ("yellow", "Yellow", Color::Yellow),
        ("magenta", "Magenta", Color::Magenta),
        ("cyan", "Cyan", Color::Cyan),
    ];

    let items_len = items.len();
    let selected_clone = selected.clone();
    let confirmed_clone = confirmed.clone();

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
        } else if key.return_key {
            let idx = selected_clone.get();
            let items = vec!["Red", "Green", "Blue", "Yellow", "Magenta", "Cyan"];
            confirmed_clone.set(Some(items[idx].to_string()));
        }
    });

    let current_selected = selected.get();
    let current_confirmed = confirmed.get();

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Text::new("Select a color:")
                .color(Color::White)
                .bold()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // List items
        .children(items.iter().enumerate().map(|(idx, (_id, label, color))| {
            let is_selected = idx == current_selected;
            let indicator = if is_selected { "> " } else { "  " };

            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new(indicator)
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .child(
                    Text::new(*label)
                        .color(if is_selected {
                            *color
                        } else {
                            Color::Ansi256(245)
                        })
                        .bold()
                        .into_element(),
                )
                .into_element()
        }))
        .child(Newline::new().into_element())
        // Confirmation message
        .child(if let Some(color) = current_confirmed {
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Green)
                .padding(1)
                .child(
                    Text::new(format!("You selected: {}", color))
                        .color(Color::Green)
                        .bold()
                        .into_element(),
                )
                .into_element()
        } else {
            Box::new().into_element()
        })
        .child(Newline::new().into_element())
        // Help
        .child(
            Text::new("Use j/k or arrows to navigate, Enter to select, q to quit")
                .dim()
                .into_element(),
        )
        .into_element()
}
