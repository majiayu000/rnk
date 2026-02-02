//! Layout Utils demo - pad_to_width function
//!
//! Run: cargo run --example layout_demo

use rnk::layout::{Position, pad_to_width};

fn main() {
    println!("=== Layout Utils Demo ===\n");

    // pad_to_width with different alignments
    println!("pad_to_width (width=40):");
    println!("|{}|", pad_to_width("Left aligned", 40, Position::Start));
    println!("|{}|", pad_to_width("Center aligned", 40, Position::Center));
    println!("|{}|", pad_to_width("Right aligned", 40, Position::End));
    println!();

    // Custom position
    println!("Custom position (25% from left):");
    println!("|{}|", pad_to_width("At 25%", 40, Position::At(0.25)));
    println!();

    // Building a simple card layout manually
    println!("Combined example - Card layout:");
    let width = 50;
    let inner_width = width - 4; // Account for borders and padding
    let border = "═".repeat(width - 2);

    println!("╔{}╗", border);
    println!(
        "║ {} ║",
        pad_to_width("USER PROFILE", inner_width, Position::Center)
    );
    println!("╠{}╣", border);
    println!("║ {} ║", format_row("Name:", "Alice Smith", inner_width));
    println!(
        "║ {} ║",
        format_row("Email:", "alice@example.com", inner_width)
    );
    println!("║ {} ║", format_row("Role:", "Administrator", inner_width));
    println!("║ {} ║", format_row("Status:", "Active", inner_width));
    println!("╚{}╝", border);
}

/// Format a key-value row with space between
fn format_row(key: &str, value: &str, width: usize) -> String {
    let key_width = key.len();
    let value_width = value.len();
    let spaces = width.saturating_sub(key_width + value_width);
    format!("{}{}{}", key, " ".repeat(spaces), value)
}
