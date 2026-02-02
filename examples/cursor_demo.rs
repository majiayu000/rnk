//! Cursor demo - Blinking text cursor
//!
//! Run: cargo run --example cursor_demo

use rnk::components::{Cursor, CursorShape, CursorState, CursorStyle};
use rnk::core::Color;

fn main() {
    println!("=== Cursor Component Demo ===\n");

    // Cursor shapes
    println!("--- Cursor Shapes ---");
    let shapes = [
        ("Block", CursorShape::Block),
        ("Underline", CursorShape::Underline),
        ("Bar", CursorShape::Bar),
        ("Custom('▌')", CursorShape::Custom('▌')),
    ];

    for (name, shape) in shapes {
        println!("  {}: '{}'", name, shape.char());
    }
    println!();

    // Cursor style presets
    println!("--- Style Presets ---");
    let presets = [
        ("CursorStyle::block()", CursorStyle::block()),
        ("CursorStyle::underline()", CursorStyle::underline()),
        ("CursorStyle::bar()", CursorStyle::bar()),
    ];

    for (name, style) in presets {
        let state = CursorState::with_style(style);
        let cursor = Cursor::new(&state);
        println!("  {}: {}", name, cursor.render());
    }
    println!();

    // Cursor with colors
    println!("--- Colored Cursors ---");
    let colors = [
        ("Cyan", Color::Cyan),
        ("Green", Color::Green),
        ("Yellow", Color::Yellow),
        ("Magenta", Color::Magenta),
    ];

    for (name, color) in colors {
        let style = CursorStyle::block().color(color);
        let state = CursorState::with_style(style);
        let cursor = Cursor::new(&state);
        print!("  {}: {} ", name, cursor.render());
    }
    println!("\n");

    // Cursor state
    println!("--- CursorState API ---");
    let mut state = CursorState::new();
    println!("  CursorState::new():");
    println!("    is_visible: {}", state.is_visible());
    println!("    is_active: {}", state.is_active());
    println!("    char: '{}'", state.char());
    println!();

    // Toggle visibility
    println!("  toggle_visibility():");
    state.toggle_visibility();
    println!("    is_visible: {}", state.is_visible());
    state.toggle_visibility();
    println!(
        "    is_visible (after toggle again): {}",
        state.is_visible()
    );
    println!();

    // Active state
    println!("  set_active(false):");
    state.set_active(false);
    println!("    is_active: {}", state.is_active());
    println!("    is_visible: {}", state.is_visible());
    state.set_active(true);
    println!("  set_active(true):");
    println!("    is_active: {}", state.is_active());
    println!("    is_visible: {}", state.is_visible());
    println!();

    // Blink simulation
    println!("--- Blink Animation Simulation ---");
    let style = CursorStyle::new().blink_interval(100);
    let mut state = CursorState::with_style(style);

    println!("  Simulating blink with 100ms interval:");
    for time in [0, 50, 100, 150, 200, 250, 300] {
        state.update(time);
        let cursor = Cursor::new(&state);
        println!(
            "    t={}ms: visible={}, render='{}'",
            time,
            state.is_visible(),
            if state.is_visible() {
                cursor.render()
            } else {
                " ".to_string()
            }
        );
    }
    println!();

    // Placeholder
    println!("--- Placeholder Character ---");
    let mut state = CursorState::new();
    state.toggle_visibility(); // Hide cursor

    let cursor_no_placeholder = Cursor::new(&state);
    println!(
        "  Hidden cursor (no placeholder): '{}'",
        cursor_no_placeholder.render()
    );

    let cursor_with_placeholder = Cursor::new(&state).placeholder('_');
    println!(
        "  Hidden cursor (placeholder '_'): '{}'",
        cursor_with_placeholder.render()
    );
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a text input)\n");

    // Simulated text input with cursor
    let text = "Hello, World";
    let cursor_pos = 7;

    print!("  Input: ");
    for (i, ch) in text.chars().enumerate() {
        if i == cursor_pos {
            print!("\x1b[7m{}\x1b[0m", ch); // Reverse video for cursor position
        } else {
            print!("{}", ch);
        }
    }
    println!();

    // Block cursor at end
    print!("  Input (cursor at end): {}", text);
    print!("\x1b[36m█\x1b[0m");
    println!();

    // Bar cursor
    print!("  Input (bar cursor): ");
    for (i, ch) in text.chars().enumerate() {
        if i == cursor_pos {
            print!("\x1b[36m│\x1b[0m{}", ch);
        } else {
            print!("{}", ch);
        }
    }
    println!();

    // Underline cursor
    print!("  Input (underline): ");
    for (i, ch) in text.chars().enumerate() {
        if i == cursor_pos {
            print!("\x1b[4m{}\x1b[0m", ch);
        } else {
            print!("{}", ch);
        }
    }
    println!("\n");

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Cursor, CursorState, CursorStyle}};");
    println!("use rnk::hooks::{{use_signal, use_interval}};");
    println!();
    println!("fn text_input_with_cursor() -> Element {{");
    println!("    let cursor = use_signal(|| CursorState::new());");
    println!();
    println!("    // Blink animation");
    println!("    use_interval(530, move || {{");
    println!("        let mut c = cursor.get();");
    println!("        c.toggle_visibility();");
    println!("        cursor.set(c);");
    println!("    }});");
    println!();
    println!("    let c = cursor.get();");
    println!("    row![");
    println!("        Text::new(\"Enter text: \"),");
    println!("        Text::new(\"Hello\"),");
    println!("        Cursor::new(&c),");
    println!("    ].into_element()");
    println!("}}");
    println!("```");
}
