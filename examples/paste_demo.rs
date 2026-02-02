//! Paste hook demo - Shows paste event handling
//!
//! Run: cargo run --example paste_demo

use rnk::hooks::{PasteEvent, is_bracketed_paste_enabled};

fn main() {
    println!("=== Bracketed Paste Demo ===\n");

    println!("Bracketed paste mode allows terminals to distinguish between");
    println!("typed input and pasted text.\n");

    println!("--- PasteEvent API ---\n");

    // Create sample paste events
    let single_line = PasteEvent::new("Hello, World!");
    println!("Single line paste:");
    println!("  content: {:?}", single_line.content());
    println!("  len: {}", single_line.len());
    println!("  is_empty: {}", single_line.is_empty());
    println!("  is_multiline: {}", single_line.is_multiline());
    println!("  line_count: {}", single_line.line_count());
    println!();

    let multi_line = PasteEvent::new("Line 1\nLine 2\nLine 3");
    println!("Multi-line paste:");
    println!("  content: {:?}", multi_line.content());
    println!("  len: {}", multi_line.len());
    println!("  is_multiline: {}", multi_line.is_multiline());
    println!("  line_count: {}", multi_line.line_count());
    println!("  lines:");
    for (i, line) in multi_line.lines().enumerate() {
        println!("    {}: {:?}", i + 1, line);
    }
    println!();

    let empty = PasteEvent::new("");
    println!("Empty paste:");
    println!("  is_empty: {}", empty.is_empty());
    println!("  line_count: {}", empty.line_count());
    println!();

    // Bracketed paste status
    println!("--- Bracketed Paste Mode ---\n");
    println!(
        "Current status: {}",
        if is_bracketed_paste_enabled() {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    println!("To enable bracketed paste in your app:");
    println!("  use rnk::hooks::{{enable_bracketed_paste, disable_bracketed_paste}};");
    println!();
    println!("  // At app start");
    println!("  enable_bracketed_paste()?;");
    println!();
    println!("  // At app exit");
    println!("  disable_bracketed_paste()?;");
    println!();

    println!("To handle paste events:");
    println!("  use rnk::hooks::use_paste;");
    println!();
    println!("  use_paste(|event| {{");
    println!("      println!(\"Pasted: {{}}\", event.content());");
    println!("      if event.is_multiline() {{");
    println!("          println!(\"({{}} lines)\", event.line_count());");
    println!("      }}");
    println!("  }});");
}
