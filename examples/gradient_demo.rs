//! Gradient text demo
//!
//! Run: cargo run --example gradient_demo

use rnk::components::Gradient;
use rnk::core::Color;

fn main() {
    println!("=== Gradient Text Demo ===\n");

    // Rainbow gradient
    println!("Rainbow:");
    println!(
        "{}\n",
        Gradient::rainbow().render("Hello, World! This is rainbow text!")
    );

    // Two-color gradient
    println!("Red to Blue:");
    println!(
        "{}\n",
        Gradient::from_two(Color::Red, Color::Blue).render("Gradient from red to blue")
    );

    // Preset gradients
    println!("Warm (red to yellow):");
    println!(
        "{}\n",
        Gradient::warm().render("Warm sunset colors flowing through text")
    );

    println!("Cool (cyan to purple):");
    println!(
        "{}\n",
        Gradient::cool().render("Cool ocean vibes in this text")
    );

    println!("Pastel:");
    println!(
        "{}\n",
        Gradient::pastel().render("Soft pastel rainbow colors")
    );

    println!("Sunset:");
    println!(
        "{}\n",
        Gradient::sunset().render("Beautiful sunset gradient")
    );

    println!("Ocean:");
    println!("{}\n", Gradient::ocean().render("Deep ocean blue gradient"));

    println!("Forest:");
    println!(
        "{}\n",
        Gradient::forest().render("Fresh forest green gradient")
    );

    // Custom gradient
    println!("Custom (Magenta -> Cyan -> Yellow):");
    let custom = Gradient::new(vec![Color::Magenta, Color::Cyan, Color::Yellow]);
    println!("{}\n", custom.render("Custom multi-color gradient text!"));

    // Reversed gradient
    println!("Reversed Rainbow:");
    println!(
        "{}\n",
        Gradient::rainbow()
            .reverse()
            .render("Rainbow but reversed!")
    );
}
