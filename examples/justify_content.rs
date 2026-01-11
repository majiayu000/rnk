//! Justify content example - demonstrating flexbox alignment
//!
//! Equivalent to ink's examples/justify-content
//!
//! Run with: cargo run --example justify_content

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

fn app() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // flex-start
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("justify-content: flex-start")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::FlexStart)
                        .width(60)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::Ansi256(240))
                        .child(item("A"))
                        .child(item("B"))
                        .child(item("C"))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // center
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("justify-content: center")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::Center)
                        .width(60)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::Ansi256(240))
                        .child(item("A"))
                        .child(item("B"))
                        .child(item("C"))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // flex-end
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("justify-content: flex-end")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::FlexEnd)
                        .width(60)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::Ansi256(240))
                        .child(item("A"))
                        .child(item("B"))
                        .child(item("C"))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // space-between
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("justify-content: space-between")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::SpaceBetween)
                        .width(60)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::Ansi256(240))
                        .child(item("A"))
                        .child(item("B"))
                        .child(item("C"))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // space-around
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("justify-content: space-around")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::SpaceAround)
                        .width(60)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::Ansi256(240))
                        .child(item("A"))
                        .child(item("B"))
                        .child(item("C"))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // space-evenly
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("justify-content: space-evenly")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .justify_content(JustifyContent::SpaceEvenly)
                        .width(60)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::Ansi256(240))
                        .child(item("A"))
                        .child(item("B"))
                        .child(item("C"))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(Text::new("Press 'q' to exit").dim().into_element())
        .into_element()
}

fn item(label: &str) -> Element {
    Box::new()
        .background(Color::Blue)
        .padding_x(2.0)
        .child(Text::new(label).color(Color::White).bold().into_element())
        .into_element()
}
