//! Static demo - shows rendered output without terminal interaction
//!
//! Run with: cargo run --example static_demo

use rnk::prelude::*;
use rnk::layout::LayoutEngine;
use rnk::renderer::Output;

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
                .into_element()
        )
        .into_element();

    render_static(&root, 20, 6);
}

fn demo_styled_text() {
    let root = Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1.0)
        .child(Text::new("Bold text").bold().into_element())
        .child(Text::new("Red error").color(Color::Red).into_element())
        .child(Text::new("Green success").color(Color::Green).into_element())
        .child(Text::new("Dimmed text").dim().into_element())
        .into_element();

    render_static(&root, 20, 8);
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

    render_static(&root, 30, 8);
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
                .into_element()
        )
        .child(Newline::new().into_element())
        .child(
            Text::new(&format!("Count: {}", count))
                .color(Color::Yellow)
                .into_element()
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("Press q to quit")
                .dim()
                .into_element()
        )
        .into_element();

    render_static(&root, 25, 10);
}

fn render_static(root: &Element, width: u16, height: u16) {
    let mut engine = LayoutEngine::new();
    engine.compute(root, width, height);

    let mut output = Output::new(width, height);
    render_element(root, &engine, &mut output, 0.0, 0.0);

    println!("{}", output.render());
}

fn render_element(element: &Element, engine: &LayoutEngine, output: &mut Output, offset_x: f32, offset_y: f32) {
    use rnk::layout::Layout;

    let layout = engine.get_layout(element.id).unwrap_or(Layout::default());

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let width = layout.width as u16;
    let height = layout.height as u16;

    // Render background
    if element.style.background_color.is_some() {
        output.fill_rect(x, y, width, height, ' ', &element.style);
    }

    // Render border
    if element.style.has_border() {
        let (tl, tr, bl, br, h, v) = element.style.border_style.chars();

        let mut border_style = element.style.clone();
        if let Some(border_color) = element.style.border_color {
            border_style.color = Some(border_color);
        }

        if height > 0 {
            output.write_char(x, y, tl.chars().next().unwrap(), &border_style);
            for col in (x + 1)..(x + width.saturating_sub(1)) {
                output.write_char(col, y, h.chars().next().unwrap(), &border_style);
            }
            if width > 1 {
                output.write_char(x + width - 1, y, tr.chars().next().unwrap(), &border_style);
            }
        }

        if height > 1 {
            let bottom_y = y + height - 1;
            output.write_char(x, bottom_y, bl.chars().next().unwrap(), &border_style);
            for col in (x + 1)..(x + width.saturating_sub(1)) {
                output.write_char(col, bottom_y, h.chars().next().unwrap(), &border_style);
            }
            if width > 1 {
                output.write_char(x + width - 1, bottom_y, br.chars().next().unwrap(), &border_style);
            }
        }

        for row in (y + 1)..(y + height.saturating_sub(1)) {
            output.write_char(x, row, v.chars().next().unwrap(), &border_style);
            if width > 1 {
                output.write_char(x + width - 1, row, v.chars().next().unwrap(), &border_style);
            }
        }
    }

    // Render text
    if let Some(text) = &element.text_content {
        let text_x = x + if element.style.has_border() { 1 } else { 0 }
            + element.style.padding.left as u16;
        let text_y = y + if element.style.has_border() { 1 } else { 0 }
            + element.style.padding.top as u16;
        output.write(text_x, text_y, text, &element.style);
    }

    // Render children
    let child_offset_x = offset_x + layout.x;
    let child_offset_y = offset_y + layout.y;

    for child in &element.children {
        render_element(child, engine, output, child_offset_x, child_offset_y);
    }
}
