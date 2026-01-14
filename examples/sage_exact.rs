//! Exact reproduction of sage UI for debugging
use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;
use rnk::layout::LayoutEngine;

fn main() {
    let term_width = 150u16;  // Wide terminal like in screenshot
    let term_height = 30u16;

    // Welcome message - exactly as in rnk_app.rs render_welcome()
    let welcome = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("Sage Agent")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new("Rust-based LLM Agent for software engineering tasks")
                .dim()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("Type a message to get started, or use /help for commands")
                .dim()
                .into_element(),
        )
        .into_element();

    // Content is welcome when messages is empty
    let content = welcome;

    // Bottom area
    let separator = "─".repeat(term_width as usize);
    let bottom = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(&separator).dim().into_element())
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
                .child(Text::new("Type your message...").into_element())
                .into_element(),
        )
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("▸▸").into_element())
                .child(Text::new(" permissions required").dim().into_element())
                .into_element(),
        )
        .into_element();

    // Full layout - EXACTLY as in render_ui after fix
    let root = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .child(content)
                .into_element(),
        )
        .child(bottom)
        .into_element();

    // Debug layout
    let mut engine = LayoutEngine::new();
    engine.compute(&root, term_width, term_height);

    println!("=== Layout Debug (term {}x{}) ===", term_width, term_height);
    fn print_layout(element: &Element, engine: &LayoutEngine, depth: usize) {
        let indent = "  ".repeat(depth);
        if let Some(layout) = engine.get_layout(element.id) {
            let name = if let Some(text) = &element.text_content {
                let t: String = text.chars().take(30).collect();
                format!("Text(\"{}\")", t)
            } else {
                format!("Box({:?})", element.style.flex_direction)
            };
            // CRITICAL: Check if x != 0
            let marker = if layout.x > 0.1 { " <-- NON-ZERO X!" } else { "" };
            println!("{}{}: x={:.1}, y={:.1}, w={:.1}, h={:.1}{}", 
                indent, name, layout.x, layout.y, layout.width, layout.height, marker);
        }
        for child in &element.children {
            print_layout(child, engine, depth + 1);
        }
    }
    print_layout(&root, &engine, 0);

    // Render
    println!("\n=== Rendered Output ===");
    let output = rnk::render_to_string(&root, term_width);
    for (i, line) in output.lines().enumerate() {
        // Check for leading spaces
        let stripped = strip_ansi(line);
        let leading = stripped.len() - stripped.trim_start().len();
        if leading > 0 {
            println!("{:3}: [OFFSET={}] |{}|", i, leading, line);
        } else if !stripped.is_empty() {
            println!("{:3}: |{}|", i, line);
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}
