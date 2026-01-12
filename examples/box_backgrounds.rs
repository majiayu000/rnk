//! Box backgrounds example - showcasing background colors
//!
//! Equivalent to ink's examples/box-backgrounds
//!
//! Run with: cargo run --example box_backgrounds

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(2)
        // Row 1: Basic colors
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Box::new()
                        .background(Color::Red)
                        .padding(1)
                        .margin_right(1.0)
                        .child(Text::new(" Red ").color(Color::White).bold().into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .background(Color::Green)
                        .padding(1)
                        .margin_right(1.0)
                        .child(
                            Text::new(" Green ")
                                .color(Color::White)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .background(Color::Blue)
                        .padding(1)
                        .margin_right(1.0)
                        .child(
                            Text::new(" Blue ")
                                .color(Color::White)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .background(Color::Yellow)
                        .padding(1)
                        .child(
                            Text::new(" Yellow ")
                                .color(Color::Black)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        // Row 2: More colors
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_top(1.0)
                .child(
                    Box::new()
                        .background(Color::Magenta)
                        .padding(1)
                        .margin_right(1.0)
                        .child(
                            Text::new(" Magenta ")
                                .color(Color::White)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .background(Color::Cyan)
                        .padding(1)
                        .margin_right(1.0)
                        .child(
                            Text::new(" Cyan ")
                                .color(Color::Black)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .background(Color::White)
                        .padding(1)
                        .margin_right(1.0)
                        .child(
                            Text::new(" White ")
                                .color(Color::Black)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .background(Color::Black)
                        .border_style(BorderStyle::Single)
                        .border_color(Color::White)
                        .padding(1)
                        .child(
                            Text::new(" Black ")
                                .color(Color::White)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        // Row 3: ANSI 256 colors
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_top(1.0)
                .children((0..8).map(|i| {
                    Box::new()
                        .background(Color::Ansi256(i * 30 + 21))
                        .width(6)
                        .height(2)
                        .margin_right(1.0)
                        .into_element()
                }))
                .into_element(),
        )
        // Row 4: RGB gradient
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_top(1.0)
                .children((0..16).map(|i| {
                    let r = (i * 16) as u8;
                    Box::new()
                        .background(Color::Rgb(r, 100, 200))
                        .width(3)
                        .height(2)
                        .into_element()
                }))
                .into_element(),
        )
        // Row 5: Background with border
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_top(2.0)
                .child(
                    Box::new()
                        .background(Color::Ansi256(236))
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Cyan)
                        .padding(1)
                        .child(
                            Text::new("Background + Border")
                                .color(Color::Cyan)
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        // Instructions
        .child(
            Box::new()
                .margin_top(2.0)
                .child(Text::new("Press 'q' to exit").dim().into_element())
                .into_element(),
        )
        .into_element()
}
