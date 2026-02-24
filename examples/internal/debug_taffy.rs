//! Debug Taffy layout computation
use rnk::layout::LayoutEngine;
use rnk::prelude::Box as RnkBox;
use rnk::prelude::*;

fn main() {
    let term_width = 120u16;
    let term_height = 30u16;

    // Simplified welcome
    let welcome = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Sage Agent").into_element())
        .child(Text::new("Subtitle line").into_element())
        .into_element();

    // Root with explicit width/height
    let root = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .child(welcome)
                .into_element(),
        )
        .into_element();

    // Compute layout
    let mut engine = LayoutEngine::new();
    engine.compute(&root, term_width, term_height);

    // Print all layouts
    println!("=== Taffy Layout Results ===");
    println!("Container: {}x{}", term_width, term_height);

    fn print_element_layout(element: &Element, engine: &LayoutEngine, depth: usize) {
        let indent = "  ".repeat(depth);
        if let Some(layout) = engine.get_layout(element.id) {
            let name = if let Some(text) = &element.text_content {
                format!("Text(\"{}\")", text.chars().take(20).collect::<String>())
            } else {
                format!("Box({:?})", element.style.flex_direction)
            };
            println!(
                "{}{}: x={:.1}, y={:.1}, w={:.1}, h={:.1}",
                indent, name, layout.x, layout.y, layout.width, layout.height
            );
        }
        for child in &element.children {
            print_element_layout(child, engine, depth + 1);
        }
    }

    print_element_layout(&root, &engine, 0);

    // Also render and show first few lines
    println!("\n=== Rendered Output (first 5 lines) ===");
    let output = rnk::render_to_string(&root, term_width);
    for (i, line) in output.lines().take(5).enumerate() {
        println!("{}: |{}|", i, line);
    }
}
