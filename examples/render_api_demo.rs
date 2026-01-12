//! Example demonstrating the new render API
//!
//! This example shows how to use the new render_to_string APIs
//! to render elements outside the runtime, which is useful for:
//! - CLI tools that need to print formatted output
//! - Testing and debugging
//! - Generating static content
//!
//! The new APIs also fix alignment issues by:
//! - Calculating actual content width instead of using full terminal width
//! - Automatically trimming trailing spaces

use rnk::prelude::*;

fn main() {
    println!("\n=== Render API Demo ===\n");

    // Example 1: Simple text rendering
    println!("1. Simple text:");
    let text = Text::new("Hello, world!").color(Color::Cyan).into_element();
    let output = rnk::render_to_string_auto(&text);
    println!("{}", output);
    println!();

    // Example 2: Box with border
    println!("2. Box with border:");
    let boxed = Box::new()
        .border_style(BorderStyle::Round)
        .border_color(Color::Green)
        .padding(1)
        .child(Text::new("Boxed content").into_element())
        .into_element();
    let output = rnk::render_to_string_auto(&boxed);
    println!("{}", output);
    println!();

    // Example 3: Multiple elements with different widths
    println!("3. Multiple elements (notice no extra spaces):");
    let elements = vec![
        Text::new("Short").color(Color::Yellow).into_element(),
        Text::new("Medium length text")
            .color(Color::Magenta)
            .into_element(),
        Text::new("This is a much longer piece of text")
            .color(Color::Cyan)
            .into_element(),
    ];

    for element in elements {
        let output = rnk::render_to_string_auto(&element);
        println!("{}", output);
    }
    println!();

    // Example 4: Using render_to_string with specific width
    println!("4. Render with specific width (40 chars):");
    let long_text = Text::new(
        "This is a very long text that will be wrapped to fit within the specified width",
    )
    .color(Color::Blue)
    .into_element();
    let output = rnk::render_to_string(&long_text, 40);
    println!("{}", output);
    println!();

    // Example 5: Complex nested structure
    println!("5. Complex nested structure:");
    let complex = Box::new()
        .border_style(BorderStyle::Double)
        .border_color(Color::Magenta)
        .padding(1)
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("Title")
                .color(Color::Yellow)
                .bold()
                .into_element(),
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(Text::new("Content line 1").into_element())
                .into_element(),
        )
        .child(Text::new("Content line 2").into_element())
        .into_element();
    let output = rnk::render_to_string_auto(&complex);
    println!("{}", output);
    println!();

    // Example 6: Using rnk::println with Element
    println!("6. Using rnk::println (inline mode):");
    rnk::println(
        Message::user("This is printed via rnk::println with proper alignment!").into_element(),
    );
    rnk::println(Message::assistant("The alignment issue is now fixed!").into_element());
    println!();

    // Example 7: Comparison with render_to_string_no_trim
    println!("7. Comparison: trimmed vs no-trim:");
    let test_element = Text::new("Test").color(Color::Green).into_element();

    println!("With trim (default):");
    let trimmed = rnk::render_to_string(&test_element, 80);
    println!("|{}|", trimmed);

    println!("Without trim:");
    let no_trim = rnk::render_to_string_no_trim(&test_element, 80);
    println!("|{}|", no_trim);
    println!();

    println!("=== Demo Complete ===");
}
