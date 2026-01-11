//! Debug layout for interactive demo

use tink::prelude::*;
use tink::core::Dimension;
use tink::layout::LayoutEngine;

fn main() {
    let element = create_demo_ui();

    let mut engine = LayoutEngine::new();
    engine.compute(&element, 80, 24);

    println!("=== Layout Debug ===\n");
    print_layout(&element, &engine, 0);
}

fn create_demo_ui() -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .padding_y(1.0)
                .child(
                    Text::new("Interactive Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element()
                )
                .into_element()
        )
        .child(Newline::new().into_element())
        // Main content - two columns
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                // Left column - counter
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
                                .into_element()
                        )
                        .into_element()
                )
                .child(Box::new().width(2).into_element())
                // Right column - list
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
                                .into_element()
                        )
                        .into_element()
                )
                .into_element()
        )
        .into_element()
}

fn print_layout(element: &Element, engine: &LayoutEngine, depth: usize) {
    let indent = "  ".repeat(depth);

    if let Some(layout) = engine.get_layout(element.id) {
        let name = if let Some(ref text) = element.text_content {
            format!("Text(\"{}\")", text.chars().take(20).collect::<String>())
        } else if element.style.has_border() {
            format!("Box[border]")
        } else {
            format!("Box")
        };

        println!(
            "{}[{}] x={:.1}, y={:.1}, w={:.1}, h={:.1}",
            indent, name, layout.x, layout.y, layout.width, layout.height
        );
    }

    for child in &element.children {
        print_layout(child, engine, depth + 1);
    }
}
