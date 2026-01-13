//! Static demo - shows rendered output without terminal interaction
//!
//! This example uses rnk's built-in render API for simplicity.
//! Run with: cargo run --example static_demo

use rnk::prelude::*;

fn main() {
    println!("=== Tink Static Demo ===\n");

    // Demo 1: Simple Hello World
    println!("1. Hello World Box:");
    demo_hello();

    println!("\n2. Styled Text:");
    demo_styled_text();

    println!("\n3. Nested Boxes:");
    demo_nested();

    println!("\n4. Counter UI (static snapshot):");
    demo_counter();
}

fn demo_hello() {
    let root = Box::new()
        .padding(1.0)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .child(
            Text::new("Hello, Tink!")
                .color(Color::Green)
                .bold()
                .into_element(),
        )
        .into_element();

    // Use rnk's render API with specific width
    let output = rnk::render_to_string(&root, 20);
    println!("{}", output);
}

fn demo_styled_text() {
    let root = Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1.0)
        .child(Text::new("Bold text").bold().into_element())
        .child(Text::new("Red error").color(Color::Red).into_element())
        .child(
            Text::new("Green success")
                .color(Color::Green)
                .into_element(),
        )
        .child(Text::new("Dimmed text").dim().into_element())
        .into_element();

    let output = rnk::render_to_string(&root, 20);
    println!("{}", output);
}

fn demo_nested() {
    let inner1 = Box::new()
        .border_style(BorderStyle::Single)
        .padding(1.0)
        .child(Text::new("Box 1").color(Color::Yellow).into_element())
        .into_element();

    let inner2 = Box::new()
        .border_style(BorderStyle::Single)
        .padding(1.0)
        .child(Text::new("Box 2").color(Color::Magenta).into_element())
        .into_element();

    let root = Box::new()
        .flex_direction(FlexDirection::Row)
        .gap(1.0)
        .border_style(BorderStyle::Double)
        .padding(1.0)
        .child(inner1)
        .child(inner2)
        .into_element();

    let output = rnk::render_to_string(&root, 30);
    println!("{}", output);
}

fn demo_counter() {
    let count = 42;

    let root = Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1.0)
        .border_style(BorderStyle::Round)
        .child(
            Text::new("Counter App")
                .bold()
                .color(Color::Cyan)
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new(&format!("Count: {}", count))
                .color(Color::Yellow)
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(Text::new("Press q to quit").dim().into_element())
        .into_element();

    let output = rnk::render_to_string(&root, 25);
    println!("{}", output);
}
