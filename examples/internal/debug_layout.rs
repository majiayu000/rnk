use rnk::prelude::Box as RnkBox;
use rnk::prelude::*;

fn main() {
    let term_width = 120u16; // Approximate terminal width
    let term_height = 30u16;

    // Render welcome message exactly as in rnk_app.rs
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

    // Full layout - mimicking render_ui
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
        .child(bottom)
        .into_element();

    let output = rnk::render_to_string(&root, term_width);

    println!("=== Raw Output (with line numbers) ===");
    for (i, line) in output.lines().enumerate() {
        // Show the raw line with a marker for position
        println!("{:3}:|{}|", i, line);
    }
    println!("=== End ===");

    // Also print character positions for first few lines
    println!("\n=== First 5 lines character analysis ===");
    for (i, line) in output.lines().take(5).enumerate() {
        let stripped = strip_ansi(line);
        let first_non_space = stripped
            .chars()
            .position(|c| c != ' ')
            .unwrap_or(stripped.len());
        println!(
            "Line {}: first non-space at column {}, content: '{}'",
            i,
            first_non_space,
            stripped.trim()
        );
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
