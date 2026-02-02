//! Select Input demo - Shows SelectInput component API
//!
//! Run: cargo run --example select_demo

use rnk::components::{SelectInput, SelectInputStyle, SelectItem};
use rnk::core::Color;

fn main() {
    println!("=== Select Input Demo ===\n");

    // Create items
    let items = vec![
        SelectItem::new("Rust", "rust"),
        SelectItem::new("Python", "python"),
        SelectItem::new("JavaScript", "javascript"),
        SelectItem::new("Go", "go"),
        SelectItem::new("TypeScript", "typescript"),
    ];

    println!("--- SelectItem API ---");
    for item in &items {
        println!(
            "  SelectItem {{ label: {:?}, value: {:?} }}",
            item.label, item.value
        );
    }
    println!();

    // SelectInput configuration
    println!("--- SelectInput Configuration ---");
    let select = SelectInput::new(items.clone())
        .highlighted(2)
        .limit(3)
        .focused(true)
        .vim_navigation(true)
        .number_shortcuts(true);

    println!("  items: {} items", select.len());
    println!("  is_empty: {}", select.is_empty());
    println!();

    // Style configuration
    println!("--- SelectInputStyle ---");
    let style = SelectInputStyle::new()
        .highlight_color(Color::Cyan)
        .highlight_bg(Color::BrightBlack)
        .highlight_bold(true)
        .indicator("❯ ")
        .item_color(Color::White);

    println!("  highlight_color: {:?}", style.highlight_color);
    println!("  highlight_bg: {:?}", style.highlight_bg);
    println!("  highlight_bold: {}", style.highlight_bold);
    println!("  indicator: {:?}", style.indicator);
    println!("  indicator_padding: {:?}", style.indicator_padding);
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    let highlighted_idx = 2;
    for (i, item) in items.iter().enumerate() {
        let is_highlighted = i == highlighted_idx;
        let prefix = if is_highlighted { "❯ " } else { "  " };
        let color_start = if is_highlighted { "\x1b[1;36m" } else { "" };
        let color_end = if is_highlighted { "\x1b[0m" } else { "" };
        println!("  {}{}{}{}", color_start, prefix, item.label, color_end);
    }
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{SelectInput, SelectItem}};");
    println!("use rnk::run_app;");
    println!();
    println!("fn app() -> Element {{");
    println!("    let items = vec![");
    println!("        SelectItem::new(\"Option 1\", 1),");
    println!("        SelectItem::new(\"Option 2\", 2),");
    println!("    ];");
    println!();
    println!("    SelectInput::new(items)");
    println!("        .highlighted(0)");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}
