//! TextArea demo - Multi-line text editor
//!
//! Run: cargo run --example textarea_demo

use rnk::components::textarea::{Position, TextAreaState, TextAreaStyle};
use rnk::core::{BorderStyle, Color};

fn main() {
    println!("=== TextArea Component Demo ===\n");

    // Create textarea state
    let mut state = TextAreaState::new();

    println!("--- TextAreaState API ---");
    println!("  is_empty: {}", state.is_empty());
    println!("  line_count: {}", state.line_count());
    println!("  cursor: {:?}", state.cursor());
    println!();

    // Insert content
    println!("--- Text Editing ---");
    state.insert_string("Hello, World!\nThis is a multi-line\ntext editor component.");
    println!("After insert_string():");
    println!("  content: {:?}", state.content());
    println!("  line_count: {}", state.line_count());
    println!("  char_count: {}", state.char_count());
    println!("  cursor: {:?}", state.cursor());
    println!();

    // Cursor movement
    println!("--- Cursor Movement ---");
    state.move_to_start();
    println!("move_to_start(): cursor = {:?}", state.cursor());

    state.move_to_line_end();
    println!("move_to_line_end(): cursor = {:?}", state.cursor());

    state.move_down();
    println!("move_down(): cursor = {:?}", state.cursor());

    state.move_word_right();
    println!("move_word_right(): cursor = {:?}", state.cursor());

    state.move_to_end();
    println!("move_to_end(): cursor = {:?}", state.cursor());
    println!();

    // Selection
    println!("--- Selection ---");
    state.move_to_start();
    state.select_to(Position::new(0, 5));
    println!("select_to(0, 5): has_selection = {}", state.has_selection());
    println!("  selected_text: {:?}", state.selected_text());

    state.select_all();
    println!(
        "select_all(): selected_text length = {}",
        state.selected_text().map_or(0, |s| s.len())
    );

    state.clear_selection();
    println!(
        "clear_selection(): has_selection = {}",
        state.has_selection()
    );
    println!();

    // Deletion
    println!("--- Deletion ---");
    state.set_content("Hello World Test");
    state.set_cursor(Position::new(0, 11));
    println!("Initial: {:?}, cursor at col 11", state.content());

    state.delete_before_cursor();
    println!("delete_before_cursor(): {:?}", state.content());

    state.delete_word_before();
    println!("delete_word_before(): {:?}", state.content());
    println!();

    // Configuration
    println!("--- Configuration ---");
    state.set_max_lines(Some(5));
    println!("set_max_lines(5)");

    state.set_char_limit(Some(100));
    println!("set_char_limit(100)");

    state.set_placeholder("Enter text here...");
    println!("set_placeholder(): {:?}", state.placeholder());

    state.set_tab_width(4);
    state.set_soft_tabs(true);
    println!("set_tab_width(4), set_soft_tabs(true)");
    println!();

    // KeyMap configurations
    println!("--- KeyMap Configurations ---");
    println!("TextAreaKeyMap::default():");
    println!("  ←/→/↑/↓        - Cursor movement");
    println!("  Ctrl+←/→       - Word navigation");
    println!("  Home/End       - Line start/end");
    println!("  Backspace/Del  - Delete char");
    println!("  Ctrl+Backspace - Delete word");
    println!("  Ctrl+K         - Delete line");
    println!("  Enter          - New line");
    println!("  Tab            - Insert tab/spaces");
    println!();

    println!("TextAreaKeyMap::minimal():");
    println!("  Basic arrow keys and delete only");
    println!();

    // Style options
    println!("--- TextAreaStyle Options ---");
    let _style = TextAreaStyle::new()
        .focused_border(BorderStyle::Round)
        .focused_border_color(Color::Cyan)
        .blurred_border_color(Color::BrightBlack)
        .text_color(Color::White)
        .placeholder_color(Color::BrightBlack)
        .cursor_color(Color::Cyan)
        .line_numbers(true)
        .cursor_char('█')
        .prompt("> ");

    println!("  focused_border: Round");
    println!("  focused_border_color: Cyan");
    println!("  line_numbers: true");
    println!("  cursor_char: '█'");
    println!("  prompt: \"> \"");
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    state.clear();
    state.set_content("fn main() {\n    println!(\"Hello, World!\");\n}");
    state.set_cursor(Position::new(1, 4));

    print_textarea_visual(&state);

    // Usage example
    println!("\n--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::textarea::{{TextArea, TextAreaState, handle_textarea_input}};");
    println!("use rnk::hooks::{{use_signal, use_input}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let state = use_signal(|| {{");
    println!("        let mut s = TextAreaState::new();");
    println!("        s.set_placeholder(\"Enter code here...\");");
    println!("        s");
    println!("    }});");
    println!();
    println!("    let keymap = TextAreaKeyMap::default();");
    println!();
    println!("    use_input(move |input, key| {{");
    println!("        let mut s = state.get();");
    println!("        handle_textarea_input(&mut s, input, key, &keymap);");
    println!("        state.set(s);");
    println!("    }});");
    println!();
    println!("    TextArea::new(&state.get())");
    println!("        .focused(true)");
    println!("        .line_numbers(true)");
    println!("        .height(10)");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}

fn print_textarea_visual(state: &TextAreaState) {
    let width = 50;
    let border_h = "─".repeat(width - 2);

    println!("  \x1b[36m╭{}╮\x1b[0m", border_h);

    let cursor = state.cursor();
    for (row, line) in state.lines().iter().enumerate() {
        let line_num = format!("\x1b[90m{:2}\x1b[0m", row + 1);
        let is_cursor_line = row == cursor.row;

        let content = if is_cursor_line {
            // Show cursor
            let chars: Vec<char> = line.chars().collect();
            let before: String = chars.iter().take(cursor.col).collect();
            let cursor_char = chars.get(cursor.col).copied().unwrap_or(' ');
            let after: String = chars.iter().skip(cursor.col + 1).collect();
            format!("{}\x1b[46m\x1b[30m{}\x1b[0m{}", before, cursor_char, after)
        } else {
            line.clone()
        };

        let content_width = width - 10;
        let display = if line.len() > content_width {
            format!("{}…", &line[..content_width - 1])
        } else {
            format!("{:width$}", content, width = content_width)
        };

        println!(
            "  \x1b[36m│\x1b[0m {} \x1b[90m│\x1b[0m {} \x1b[36m│\x1b[0m",
            line_num, display
        );
    }

    // Empty lines to fill viewport
    for i in state.line_count()..5 {
        let line_num = format!("\x1b[90m{:2}\x1b[0m", i + 1);
        let empty = " ".repeat(width - 10);
        println!(
            "  \x1b[36m│\x1b[0m {} \x1b[90m│\x1b[0m {} \x1b[36m│\x1b[0m",
            line_num, empty
        );
    }

    println!("  \x1b[36m╰{}╯\x1b[0m", border_h);
    println!(
        "    \x1b[90mLine {}, Col {} | {} lines\x1b[0m",
        cursor.row + 1,
        cursor.col + 1,
        state.line_count()
    );
}
