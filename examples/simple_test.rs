//! Simple test - just print the UI without alternate screen
//!
//! This example uses rnk's built-in render API for simplicity.
//! Run with: cargo run --example simple_test

use rnk::core::Dimension;
use rnk::prelude::*;

fn main() {
    // Get terminal size
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    println!("Terminal width: {}\n", width);

    let element = create_ui();

    // Use rnk's built-in render API
    let output = rnk::render_to_string_auto(&element);
    println!("{}", output);
}

fn create_ui() -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .padding_y(1.0)
                .child(
                    Text::new("Tink Interactive Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Box::new()
                        .width(30)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Yellow)
                        .padding(1)
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("Counter Demo")
                                .color(Color::Yellow)
                                .bold()
                                .underline()
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(
                            Box::new()
                                .flex_direction(FlexDirection::Row)
                                .child(Text::new("Value: ").color(Color::White).into_element())
                                .child(Text::new("42").color(Color::Green).bold().into_element())
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(
                            Text::new("[+] Increment")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .child(
                            Text::new("[-] Decrement")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .child(
                            Text::new("[0] Reset")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Box::new().width(2).into_element())
                .child(
                    Box::new()
                        .flex_grow(1.0)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Blue)
                        .padding(1)
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("List Navigation")
                                .color(Color::Blue)
                                .bold()
                                .underline()
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(create_list_items())
                        .child(Newline::new().into_element())
                        .child(
                            Text::new("[j/k] Navigate")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Magenta)
                .padding(1)
                .child(
                    Text::new("Welcome! Press 'h' for help.")
                        .color(Color::Magenta)
                        .italic()
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}

fn create_list_items() -> Element {
    let items = vec!["First item", "Second item", "Third item", "Fourth item"];
    let selected = 0;

    let mut container = Box::new().flex_direction(FlexDirection::Column);

    for (i, item) in items.iter().enumerate() {
        let is_selected = i == selected;
        let mut row = Box::new().flex_direction(FlexDirection::Row).padding_x(1.0);

        if is_selected {
            row = row.background(Color::Ansi256(236));
        }

        row = row
            .child(
                Text::new(if is_selected { "> " } else { "  " })
                    .color(Color::Cyan)
                    .bold()
                    .into_element(),
            )
            .child(
                Text::new(*item)
                    .color(if is_selected {
                        Color::White
                    } else {
                        Color::Ansi256(250)
                    })
                    .into_element(),
            );

        container = container.child(row.into_element());
    }

    container.into_element()
}
