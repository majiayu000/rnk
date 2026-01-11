//! Borders example - showcasing all border styles
//!
//! Equivalent to ink's examples/borders
//!
//! Run with: cargo run --example borders

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

fn app() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(2)
        // First row of borders
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Box::new()
                        .border_style(BorderStyle::Single)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("single").into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Double)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("double").into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("round").into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Bold)
                        .padding(1)
                        .child(Text::new("bold").into_element())
                        .into_element(),
                )
                .into_element(),
        )
        // Second row of borders
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_top(1.0)
                .child(
                    Box::new()
                        .border_style(BorderStyle::SingleDouble)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("singleDouble").into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::DoubleSingle)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("doubleSingle").into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Classic)
                        .padding(1)
                        .child(Text::new("classic").into_element())
                        .into_element(),
                )
                .into_element(),
        )
        // Third row - colored borders
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_top(1.0)
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Red)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("red").color(Color::Red).into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Green)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("green").color(Color::Green).into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Blue)
                        .margin_right(2.0)
                        .padding(1)
                        .child(Text::new("blue").color(Color::Blue).into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Yellow)
                        .padding(1)
                        .child(Text::new("yellow").color(Color::Yellow).into_element())
                        .into_element(),
                )
                .into_element(),
        )
        // Instructions
        .child(
            Box::new()
                .margin_top(2.0)
                .child(
                    Text::new("Press 'q' to exit")
                        .dim()
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}
