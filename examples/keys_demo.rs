//! Extended Keys demo - Shows supported key types
//!
//! Run: cargo run --example keys_demo

use rnk::hooks::Key;

fn main() {
    println!("=== Extended Keys Demo ===\n");

    println!("The Key struct provides boolean fields for key detection.\n");

    // Show the Key struct fields
    println!("--- Key Struct Fields ---\n");

    println!("Arrow Keys:");
    println!("  key.up_arrow      - Up arrow");
    println!("  key.down_arrow    - Down arrow");
    println!("  key.left_arrow    - Left arrow");
    println!("  key.right_arrow   - Right arrow");
    println!();

    println!("Navigation Keys:");
    println!("  key.page_up       - Page Up");
    println!("  key.page_down     - Page Down");
    println!("  key.home          - Home");
    println!("  key.end           - End");
    println!("  key.insert        - Insert");
    println!();

    println!("Action Keys:");
    println!("  key.return_key    - Enter/Return");
    println!("  key.escape        - Escape");
    println!("  key.tab           - Tab");
    println!("  key.backspace     - Backspace");
    println!("  key.delete        - Delete");
    println!("  key.space         - Space");
    println!();

    println!("Function Keys:");
    println!("  key.f1 - key.f12  - F1 through F12");
    println!();

    println!("Modifiers:");
    println!("  key.ctrl          - Control key held");
    println!("  key.shift         - Shift key held");
    println!("  key.alt           - Alt key held");
    println!("  key.meta          - Meta/Cmd key held");
    println!();

    println!("Media Keys:");
    println!("  key.media_play         - Play");
    println!("  key.media_pause        - Pause");
    println!("  key.media_play_pause   - Play/Pause toggle");
    println!("  key.media_stop         - Stop");
    println!("  key.media_next         - Next track");
    println!("  key.media_previous     - Previous track");
    println!("  key.volume_up          - Volume up");
    println!("  key.volume_down        - Volume down");
    println!("  key.volume_mute        - Mute");
    println!();

    // Example Key instance
    println!("--- Example Key Instance ---\n");
    let key = Key::default();
    println!("Default Key (all false):");
    println!("  up_arrow: {}", key.up_arrow);
    println!("  return_key: {}", key.return_key);
    println!("  ctrl: {}", key.ctrl);
    println!("  f1: {}", key.f1);
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::hooks::use_input;");
    println!();
    println!("// In your component:");
    println!("use_input(|input, key| {{");
    println!("    // Check for specific keys");
    println!("    if key.up_arrow {{");
    println!("        // Handle up arrow");
    println!("    }}");
    println!("    if key.return_key {{");
    println!("        // Handle Enter");
    println!("    }}");
    println!("    if key.ctrl && input == \"c\" {{");
    println!("        // Handle Ctrl+C");
    println!("    }}");
    println!();
    println!("    // Check for character input");
    println!("    if input == \"q\" {{");
    println!("        // Handle 'q' key");
    println!("    }}");
    println!("}});");
    println!("```");
    println!();

    println!("Note: The `input` parameter contains the character pressed (if any),");
    println!("while `key` provides boolean flags for special keys and modifiers.");
}
