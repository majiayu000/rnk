//! Test rendering directly to terminal
use crossterm::{cursor, execute, terminal};
use rnk::prelude::Box as RnkBox;
use rnk::prelude::*;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
    println!("Detected terminal size: {}x{}", term_width, term_height);

    // Welcome message
    let welcome = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("Sage Agent")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(Text::new("Rust-based LLM Agent").dim().into_element())
        .child(Newline::new().into_element())
        .child(
            Text::new("Type a message to get started")
                .dim()
                .into_element(),
        )
        .into_element();

    // Bottom
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
        .into_element();

    // Root
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

    // Print output WITH visible markers to see exact positions
    println!("\n=== Rendering with column markers ===");

    // Print column ruler
    print!("     ");
    for i in 0..term_width.min(100) {
        print!("{}", i % 10);
    }
    println!();

    for (i, line) in output.lines().take(10).enumerate() {
        println!("{:4}|{}", i, line);
    }

    // Print the last few lines (bottom area)
    println!("...");
    let lines: Vec<_> = output.lines().collect();
    for (i, line) in lines.iter().rev().take(4).rev().enumerate() {
        let line_num = lines.len() - 4 + i;
        println!("{:4}|{}", line_num, line);
    }

    Ok(())
}
