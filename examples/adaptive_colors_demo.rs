//! Adaptive Colors demo - Colors that adapt to terminal background
//!
//! Run: cargo run --example adaptive_colors_demo

use rnk::core::{AdaptiveColor, Color, adaptive_colors, is_dark_background, set_dark_background};

fn main() {
    println!("=== Adaptive Colors Demo ===\n");

    // Create adaptive colors
    let text_color = AdaptiveColor::new(Color::Black, Color::White);
    let accent_color = AdaptiveColor::new(
        Color::Rgb(0, 100, 180),   // Darker blue for light backgrounds
        Color::Rgb(100, 200, 255), // Bright blue for dark backgrounds
    );

    // Demo with dark background
    println!("=== Dark Background Mode ===");
    set_dark_background(true);
    println!("is_dark_background(): {}\n", is_dark_background());

    let color = text_color.resolve();
    print_colored("Text color", color);

    let color = accent_color.resolve();
    print_colored("Accent color", color);

    // Demo with light background
    println!("\n=== Light Background Mode ===");
    set_dark_background(false);
    println!("is_dark_background(): {}\n", is_dark_background());

    let color = text_color.resolve();
    print_colored("Text color", color);

    let color = accent_color.resolve();
    print_colored("Accent color", color);

    // Using preset adaptive colors
    println!("\n=== Preset Adaptive Colors ===");

    set_dark_background(true);
    println!("\nDark mode:");
    print_adaptive("text()", adaptive_colors::text());
    print_adaptive("muted()", adaptive_colors::muted());
    print_adaptive("success()", adaptive_colors::success());
    print_adaptive("error()", adaptive_colors::error());
    print_adaptive("warning()", adaptive_colors::warning());
    print_adaptive("info()", adaptive_colors::info());
    print_adaptive("accent()", adaptive_colors::accent());
    print_adaptive("highlight()", adaptive_colors::highlight());

    set_dark_background(false);
    println!("\nLight mode:");
    print_adaptive("text()", adaptive_colors::text());
    print_adaptive("muted()", adaptive_colors::muted());
    print_adaptive("success()", adaptive_colors::success());
    print_adaptive("error()", adaptive_colors::error());
    print_adaptive("warning()", adaptive_colors::warning());
    print_adaptive("info()", adaptive_colors::info());
    print_adaptive("accent()", adaptive_colors::accent());
    print_adaptive("highlight()", adaptive_colors::highlight());

    // From hex
    println!("\n=== From Hex ===");
    let custom = AdaptiveColor::from_hex("#333333", "#eeeeee");
    set_dark_background(true);
    print_adaptive("from_hex (dark)", custom);
    set_dark_background(false);
    print_adaptive("from_hex (light)", custom);

    // Universal color (same on both backgrounds)
    println!("\n=== Universal Color ===");
    let universal = AdaptiveColor::universal(Color::Cyan);
    set_dark_background(true);
    print_adaptive("universal (dark)", universal);
    set_dark_background(false);
    print_adaptive("universal (light)", universal);

    // Reset to default
    set_dark_background(true);
    println!("\n(Reset to dark mode)");
}

fn print_colored(name: &str, color: Color) {
    match color {
        Color::Rgb(r, g, b) => {
            println!(
                "  {}: \x1b[38;2;{};{};{}m████████\x1b[0m RGB({}, {}, {})",
                name, r, g, b, r, g, b
            );
        }
        _ => {
            println!("  {}: {:?}", name, color);
        }
    }
}

fn print_adaptive(name: &str, adaptive: AdaptiveColor) {
    let color = adaptive.resolve();
    match color {
        Color::Rgb(r, g, b) => {
            println!("  {:12}: \x1b[38;2;{};{};{}m████\x1b[0m", name, r, g, b);
        }
        Color::Black => println!("  {:12}: \x1b[30m████\x1b[0m Black", name),
        Color::White => println!("  {:12}: \x1b[37m████\x1b[0m White", name),
        Color::Red => println!("  {:12}: \x1b[31m████\x1b[0m Red", name),
        Color::Green => println!("  {:12}: \x1b[32m████\x1b[0m Green", name),
        Color::Yellow => println!("  {:12}: \x1b[33m████\x1b[0m Yellow", name),
        Color::Blue => println!("  {:12}: \x1b[34m████\x1b[0m Blue", name),
        Color::Magenta => println!("  {:12}: \x1b[35m████\x1b[0m Magenta", name),
        Color::Cyan => println!("  {:12}: \x1b[36m████\x1b[0m Cyan", name),
        Color::BrightBlack => println!("  {:12}: \x1b[90m████\x1b[0m BrightBlack", name),
        Color::BrightRed => println!("  {:12}: \x1b[91m████\x1b[0m BrightRed", name),
        Color::BrightGreen => println!("  {:12}: \x1b[92m████\x1b[0m BrightGreen", name),
        Color::BrightYellow => println!("  {:12}: \x1b[93m████\x1b[0m BrightYellow", name),
        Color::BrightBlue => println!("  {:12}: \x1b[94m████\x1b[0m BrightBlue", name),
        Color::BrightMagenta => println!("  {:12}: \x1b[95m████\x1b[0m BrightMagenta", name),
        Color::BrightCyan => println!("  {:12}: \x1b[96m████\x1b[0m BrightCyan", name),
        Color::BrightWhite => println!("  {:12}: \x1b[97m████\x1b[0m BrightWhite", name),
        _ => println!("  {:12}: {:?}", name, color),
    }
}
