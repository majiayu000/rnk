//! Multi-Select demo - Shows MultiSelect component API
//!
//! Run: cargo run --example multi_select_demo

use rnk::components::{MultiSelect, MultiSelectItem, MultiSelectStyle};
use rnk::core::Color;

fn main() {
    println!("=== Multi-Select Demo ===\n");

    // Create items with some pre-selected
    let items = vec![
        MultiSelectItem::new("Apple", "apple"),
        MultiSelectItem::selected("Banana", "banana"), // Pre-selected
        MultiSelectItem::new("Cherry", "cherry"),
        MultiSelectItem::selected("Date", "date"), // Pre-selected
        MultiSelectItem::new("Elderberry", "elderberry"),
    ];

    println!("--- MultiSelectItem API ---");
    for item in &items {
        println!(
            "  {{ label: {:?}, value: {:?}, selected: {} }}",
            item.label, item.value, item.selected
        );
    }
    println!();

    // MultiSelect configuration
    println!("--- MultiSelect Configuration ---");
    let select = MultiSelect::new(items.clone())
        .highlighted(1)
        .limit(4)
        .focused(true)
        .vim_navigation(true);

    println!("  items: {} items", select.len());
    println!("  is_empty: {}", select.is_empty());
    println!("  selected_values: {:?}", select.selected_values());
    println!();

    // Style configuration
    println!("--- MultiSelectStyle ---");
    let style = MultiSelectStyle::new()
        .highlight_color(Color::Cyan)
        .highlight_bold(true)
        .indicator("❯ ")
        .checkboxes("◉ ", "◯ ")
        .selected_color(Color::Green);

    println!("  highlight_color: {:?}", style.highlight_color);
    println!("  indicator: {:?}", style.indicator);
    println!("  checkbox_selected: {:?}", style.checkbox_selected);
    println!("  checkbox_unselected: {:?}", style.checkbox_unselected);
    println!("  selected_color: {:?}", style.selected_color);
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    let highlighted_idx = 1;
    for (i, item) in items.iter().enumerate() {
        let is_highlighted = i == highlighted_idx;
        let prefix = if is_highlighted { "❯ " } else { "  " };
        let checkbox = if item.selected { "◉ " } else { "◯ " };

        let (color_start, color_end) = if is_highlighted {
            ("\x1b[1;36m", "\x1b[0m") // Bold cyan
        } else if item.selected {
            ("\x1b[32m", "\x1b[0m") // Green
        } else {
            ("", "")
        };

        println!(
            "  {}{}{}{}{}",
            color_start, prefix, checkbox, item.label, color_end
        );
    }
    println!();

    // Keyboard controls
    println!("--- Keyboard Controls ---");
    println!("  ↑/↓ or j/k  - Navigate");
    println!("  Space       - Toggle selection");
    println!("  Ctrl+A      - Select all");
    println!("  Ctrl+D      - Deselect all");
    println!("  Home/End    - Jump to first/last");
    println!("  PgUp/PgDn   - Page navigation");
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{MultiSelect, MultiSelectItem}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let items = vec![");
    println!("        MultiSelectItem::new(\"Option 1\", 1),");
    println!("        MultiSelectItem::selected(\"Option 2\", 2),");
    println!("    ];");
    println!();
    println!("    MultiSelect::new(items)");
    println!("        .highlighted(0)");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}
