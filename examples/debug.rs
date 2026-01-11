//! Debug example - prints layout info without terminal features

use rnk::prelude::*;
use rnk::layout::LayoutEngine;
use rnk::renderer::Output;

fn main() {
    let root = Box::new()
        .padding(1.0)
        .border_style(BorderStyle::Round)
        .child(
            Text::new("Hello, Tink!")
                .color(Color::Green)
                .bold()
                .into_element()
        )
        .into_element();

    println!("Element tree built:");
    print_element(&root, 0);

    let mut engine = LayoutEngine::new();
    engine.compute(&root, 80, 24);

    println!("\nLayout computed (80x24):");
    print_layout(&root, &engine, 0);

    // Render to a small buffer
    let mut output = Output::new(20, 10);
    render_element(&root, &engine, &mut output, 0.0, 0.0);

    println!("\nRendered output (20x10):");
    println!("{}", output.render());
}

fn render_element(element: &Element, engine: &LayoutEngine, output: &mut Output, offset_x: f32, offset_y: f32) {
    use rnk::layout::Layout;

    let layout = engine.get_layout(element.id).unwrap_or(Layout::default());

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let width = layout.width as u16;
    let height = layout.height as u16;

    // Render border if set
    if element.style.has_border() {
        let (tl, tr, bl, br, h, v) = element.style.border_style.chars();

        let mut border_style = element.style.clone();
        if let Some(border_color) = element.style.border_color {
            border_style.color = Some(border_color);
        }

        // Top border
        if height > 0 {
            output.write_char(x, y, tl.chars().next().unwrap(), &border_style);
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, y, h.chars().next().unwrap(), &border_style);
            }
            if width > 1 {
                output.write_char(x + width - 1, y, tr.chars().next().unwrap(), &border_style);
            }
        }

        // Bottom border
        if height > 1 {
            let bottom_y = y + height - 1;
            output.write_char(x, bottom_y, bl.chars().next().unwrap(), &border_style);
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, bottom_y, h.chars().next().unwrap(), &border_style);
            }
            if width > 1 {
                output.write_char(x + width - 1, bottom_y, br.chars().next().unwrap(), &border_style);
            }
        }

        // Side borders
        for row in (y + 1)..(y + height - 1) {
            output.write_char(x, row, v.chars().next().unwrap(), &border_style);
            if width > 1 {
                output.write_char(x + width - 1, row, v.chars().next().unwrap(), &border_style);
            }
        }
    }

    // Render text content
    if let Some(text) = &element.text_content {
        let text_x = x + if element.style.has_border() { 1 } else { 0 }
            + element.style.padding.left as u16;
        let text_y = y + if element.style.has_border() { 1 } else { 0 }
            + element.style.padding.top as u16;
        output.write(text_x, text_y, text, &element.style);
    }

    // Render children - Taffy already includes border/padding in child positions
    let child_offset_x = offset_x + layout.x;
    let child_offset_y = offset_y + layout.y;

    for child in &element.children {
        render_element(child, engine, output, child_offset_x, child_offset_y);
    }
}

fn print_element(element: &Element, indent: usize) {
    let spaces = " ".repeat(indent * 2);
    println!(
        "{}Element id={:?} type={:?} text={:?}",
        spaces,
        element.id,
        element.element_type,
        element.text_content
    );
    println!(
        "{}  border_style={:?} has_border={}",
        spaces,
        element.style.border_style,
        element.style.has_border()
    );
    for child in &element.children {
        print_element(child, indent + 1);
    }
}

fn print_layout(element: &Element, engine: &LayoutEngine, indent: usize) {
    let spaces = " ".repeat(indent * 2);
    if let Some(layout) = engine.get_layout(element.id) {
        println!(
            "{}id={:?}: x={:.1} y={:.1} w={:.1} h={:.1}",
            spaces, element.id, layout.x, layout.y, layout.width, layout.height
        );
    } else {
        println!("{}id={:?}: NO LAYOUT", spaces, element.id);
    }
    for child in &element.children {
        print_layout(child, engine, indent + 1);
    }
}
