//! Help component demo - Display keyboard shortcuts
//!
//! Run: cargo run --example help_demo

use rnk::components::{
    Help, HelpMode, HelpStyle, KeyBinding, editor_help, navigation_help, vim_navigation_help,
};
use rnk::core::Color;

fn main() {
    println!("=== Help Component Demo ===\n");

    // KeyBinding API
    println!("--- KeyBinding API ---");
    let binding = KeyBinding::new("Ctrl+C", "Copy to clipboard");
    println!(
        "  KeyBinding {{ key: {:?}, description: {:?} }}",
        binding.key, binding.description
    );
    println!();

    // Help component creation
    println!("--- Help Component ---");
    let help = Help::new(vec![
        KeyBinding::new("↑/↓", "Navigate"),
        KeyBinding::new("Enter", "Select"),
        KeyBinding::new("q", "Quit"),
    ]);
    println!("  bindings: {}", help.len());
    println!("  is_empty: {}", help.is_empty());
    println!("  is_visible: {}", help.is_visible());
    println!();

    // From tuples
    println!("--- From Tuples ---");
    let help = Help::from_tuples([("↑/↓", "Navigate"), ("Enter", "Select"), ("Esc", "Cancel")]);
    println!("  Help::from_tuples([...]) -> {} bindings", help.len());
    println!();

    // Builder pattern
    println!("--- Builder Pattern ---");
    let help = Help::new(vec![])
        .binding("j/k", "Navigate")
        .binding("Enter", "Select")
        .binding("q", "Quit")
        .single_line();
    println!("  Help::new(vec![]).binding(...).binding(...).single_line()");
    println!("  bindings: {}", help.len());
    println!();

    // Display modes
    println!("--- Display Modes ---");
    println!("  HelpMode::SingleLine - All bindings on one line");
    println!("  HelpMode::MultiLine  - Each binding on its own line");
    println!("  HelpMode::TwoColumn  - Aligned key-description columns");
    println!();

    // Style configuration
    println!("--- HelpStyle ---");
    let style = HelpStyle::new()
        .key_color(Color::Cyan)
        .description_color(Color::BrightBlack)
        .separator(": ")
        .binding_separator("  |  ")
        .key_bold(true)
        .description_dim(true);
    println!("  key_color: {:?}", style.key_color);
    println!("  description_color: {:?}", style.description_color);
    println!("  separator: {:?}", style.separator);
    println!("  binding_separator: {:?}", style.binding_separator);
    println!("  key_bold: {}", style.key_bold);
    println!("  description_dim: {}", style.description_dim);
    println!();

    // Preset bindings
    println!("--- Preset Bindings ---");
    println!("navigation_help():");
    for b in navigation_help() {
        println!("  {} - {}", b.key, b.description);
    }
    println!();

    println!("vim_navigation_help():");
    for b in vim_navigation_help() {
        println!("  {} - {}", b.key, b.description);
    }
    println!();

    println!("editor_help():");
    for b in editor_help() {
        println!("  {} - {}", b.key, b.description);
    }
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    println!("  SingleLine mode:");
    println!(
        "  \x1b[1;36m↑/↓\x1b[0m \x1b[90mNavigate\x1b[0m  •  \x1b[1;36mEnter\x1b[0m \x1b[90mSelect\x1b[0m  •  \x1b[1;36mq\x1b[0m \x1b[90mQuit\x1b[0m"
    );
    println!();

    println!("  MultiLine mode:");
    println!("  \x1b[1;36m↑/↓\x1b[0m   \x1b[90mNavigate\x1b[0m");
    println!("  \x1b[1;36mEnter\x1b[0m \x1b[90mSelect\x1b[0m");
    println!("  \x1b[1;36mq\x1b[0m     \x1b[90mQuit\x1b[0m");
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Help, KeyBinding}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    Help::new(vec![");
    println!("        KeyBinding::new(\"↑/↓\", \"Navigate\"),");
    println!("        KeyBinding::new(\"Enter\", \"Select\"),");
    println!("        KeyBinding::new(\"q\", \"Quit\"),");
    println!("    ])");
    println!("    .single_line()");
    println!("    .into_element()");
    println!("}}");
    println!("```");
}
