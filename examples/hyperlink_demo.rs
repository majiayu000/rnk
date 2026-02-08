//! Hyperlink demo
//!
//! Run: cargo run --example hyperlink_demo

use rnk::components::{Hyperlink, HyperlinkBuilder, set_hyperlinks_supported};
use rnk::core::Color;

fn main() {
    println!("=== Terminal Hyperlink Demo ===\n");
    println!("(Links are clickable in supported terminals: iTerm2, Kitty, WezTerm, etc.)\n");

    // Force enable hyperlinks for demo
    set_hyperlinks_supported(true);

    // Simple hyperlink
    println!("Simple link:");
    println!(
        "{}\n",
        Hyperlink::new("https://github.com", "Click here to visit GitHub").render()
    );

    // URL as text
    println!("URL as display text:");
    println!("{}\n", Hyperlink::url("https://docs.rs/rnk").render());

    // Using Hyperlink struct
    println!("Hyperlink with ID (for multi-line links):");
    let link_with_id =
        Hyperlink::new("https://rust-lang.org", "Rust Programming Language").with_id("rust-link");
    println!("{}\n", link_with_id.render());

    // Styled hyperlink with builder
    println!("Styled hyperlink (blue, underlined, bold):");
    let styled = HyperlinkBuilder::new("https://crates.io", "Crates.io")
        .color(Color::Cyan)
        .underline(true)
        .bold(true);
    println!("{}\n", styled.render());

    // Different colors
    println!("Different colored links:");
    let red_link = HyperlinkBuilder::new("https://example.com/red", "Red Link")
        .color(Color::Red)
        .underline(true);
    let green_link = HyperlinkBuilder::new("https://example.com/green", "Green Link")
        .color(Color::Green)
        .underline(true);
    let yellow_link = HyperlinkBuilder::new("https://example.com/yellow", "Yellow Link")
        .color(Color::Yellow)
        .underline(true);
    println!(
        "{}  {}  {}\n",
        red_link.render(),
        green_link.render(),
        yellow_link.render()
    );

    // Fallback mode
    println!("Fallback mode (for unsupported terminals):");
    set_hyperlinks_supported(false);
    println!(
        "{}\n",
        Hyperlink::new("https://github.com/majiayu000/rnk", "rnk repo").render()
    );

    // Custom fallback
    set_hyperlinks_supported(false);
    let link = Hyperlink::new("https://example.com", "Example");
    let custom_fallback = link.render_with_fallback(|text, url| {
        format!("[{}]({})", text, url) // Markdown style
    });
    println!("Custom fallback (Markdown style): {}\n", custom_fallback);
}
