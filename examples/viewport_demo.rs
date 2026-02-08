//! Viewport demo - Scrollable text viewer
//!
//! Run: cargo run --example viewport_demo

use rnk::components::viewport::{ViewportState, ViewportStyle};
use rnk::core::{BorderStyle, Color};

fn main() {
    println!("=== Viewport Component Demo ===\n");

    // Create sample content
    let content = generate_sample_content();

    // Create viewport state
    let mut state = ViewportState::new(60, 15);
    state.set_content(&content);

    println!("--- ViewportState API ---");
    println!("  total_line_count: {}", state.total_line_count());
    println!("  visible_line_count: {}", state.visible_line_count());
    println!("  fits_in_viewport: {}", state.fits_in_viewport());
    println!("  y_offset: {}", state.y_offset());
    println!("  scroll_percent: {:.1}%", state.scroll_percent() * 100.0);
    println!("  at_top: {}", state.at_top());
    println!("  at_bottom: {}", state.at_bottom());
    println!();

    // Demonstrate scrolling
    println!("--- Scrolling Demo ---");
    println!("Initial position (top):");
    print_visible_lines(&state, 5);

    state.scroll_down(10);
    println!("\nAfter scroll_down(10):");
    println!(
        "  y_offset: {}, scroll_percent: {:.1}%",
        state.y_offset(),
        state.scroll_percent() * 100.0
    );
    print_visible_lines(&state, 5);

    state.page_down();
    println!("\nAfter page_down():");
    println!(
        "  y_offset: {}, scroll_percent: {:.1}%",
        state.y_offset(),
        state.scroll_percent() * 100.0
    );
    print_visible_lines(&state, 5);

    state.goto_bottom();
    println!("\nAfter goto_bottom():");
    println!(
        "  y_offset: {}, at_bottom: {}",
        state.y_offset(),
        state.at_bottom()
    );
    print_visible_lines(&state, 5);

    state.goto_top();
    println!("\nAfter goto_top():");
    println!(
        "  y_offset: {}, at_top: {}",
        state.y_offset(),
        state.at_top()
    );
    println!();

    // KeyMap configurations
    println!("--- KeyMap Configurations ---");
    println!("ViewportKeyMap::default():");
    println!("  ↑/↓ or j/k    - Scroll up/down");
    println!("  PageUp/Down   - Page navigation");
    println!("  Ctrl+U/D      - Half page");
    println!("  Home/End or g/G - Top/bottom");
    println!();

    println!("ViewportKeyMap::vim():");
    println!("  j/k           - Scroll up/down");
    println!("  Ctrl+B/F      - Page up/down");
    println!("  g/G           - Top/bottom");
    println!();

    println!("ViewportKeyMap::arrows_only():");
    println!("  Arrow keys only, no vim bindings");
    println!();

    // Style options
    println!("--- ViewportStyle Options ---");
    let _style = ViewportStyle::new()
        .border(BorderStyle::Round)
        .border_color(Color::Cyan)
        .background(Color::Black)
        .text_color(Color::White)
        .line_numbers(true)
        .line_number_color(Color::BrightBlack)
        .scrollbar(true)
        .scrollbar_color(Color::Cyan);

    println!("  border: Round");
    println!("  border_color: Cyan");
    println!("  line_numbers: true");
    println!("  scrollbar: true");
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    state.goto_top();
    print_viewport_visual(&state);

    // Usage example
    println!("\n--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::viewport::{{Viewport, ViewportState, handle_viewport_input}};");
    println!("use rnk::hooks::{{use_signal, use_input}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let state = use_signal(|| {{");
    println!("        let mut s = ViewportState::new(80, 20);");
    println!("        s.set_content(include_str!(\"file.txt\"));");
    println!("        s");
    println!("    }});");
    println!();
    println!("    let keymap = ViewportKeyMap::default();");
    println!();
    println!("    use_input(move |input, key| {{");
    println!("        let mut s = state.get();");
    println!("        if handle_viewport_input(&mut s, input, key, &keymap) {{");
    println!("            state.set(s);");
    println!("        }}");
    println!("    }});");
    println!();
    println!("    Viewport::new(&state.get())");
    println!("        .line_numbers(true)");
    println!("        .scrollbar(true)");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}

fn generate_sample_content() -> String {
    let mut lines = Vec::new();
    lines.push("# Viewport Demo Content".to_string());
    lines.push("".to_string());
    lines.push("This is a demonstration of the Viewport component.".to_string());
    lines.push("It supports scrolling through large amounts of text.".to_string());
    lines.push("".to_string());

    for i in 1..=50 {
        lines.push(format!(
            "Line {:3}: Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            i
        ));
    }

    lines.push("".to_string());
    lines.push("# End of Content".to_string());
    lines.push("You've reached the bottom!".to_string());

    lines.join("\n")
}

fn print_visible_lines(state: &ViewportState, max_lines: usize) {
    for (i, line) in state.visible_lines().take(max_lines).enumerate() {
        let global_line = state.y_offset() + i + 1;
        let truncated = if line.len() > 50 {
            format!("{}...", &line[..47])
        } else {
            line.to_string()
        };
        println!("  {:3} │ {}", global_line, truncated);
    }
    if state.visible_line_count() > max_lines {
        println!(
            "  ... │ ({} more lines)",
            state.visible_line_count() - max_lines
        );
    }
}

fn print_viewport_visual(state: &ViewportState) {
    let width = 60;
    let border_h = "─".repeat(width - 2);

    println!("  ╭{}╮", border_h);

    for (i, line) in state.visible_lines().take(10).enumerate() {
        let global_line = state.y_offset() + i + 1;
        let line_num = format!("\x1b[90m{:3}\x1b[0m", global_line);

        let content_width = width - 10; // Account for borders, line num, scrollbar
        let truncated = if line.len() > content_width {
            format!("{}…", &line[..content_width - 1])
        } else {
            format!("{:width$}", line, width = content_width)
        };

        // Scrollbar
        let scroll_char = if i < 3 {
            "\x1b[36m█\x1b[0m"
        } else {
            "\x1b[90m░\x1b[0m"
        };

        println!("  │ {} │ {} │{}│", line_num, truncated, scroll_char);
    }

    println!("  ╰{}╯", border_h);
    println!(
        "    \x1b[90m{:.0}% scrolled\x1b[0m",
        state.scroll_percent() * 100.0
    );
}
