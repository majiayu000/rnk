//! Spring Animation demo
//!
//! Run: cargo run --example spring_demo

use rnk::animation::{Spring, SpringColor, SpringValue, SpringValue2D};

fn main() {
    println!("=== Spring Animation Demo ===\n");

    // Basic spring physics
    println!("--- Spring Physics ---");
    let spring = Spring::smooth(60.0);
    println!(
        "Smooth spring: angular_freq={}, damping_ratio={}",
        spring.angular_frequency(),
        spring.damping_ratio()
    );

    let bouncy = Spring::bouncy(60.0);
    println!(
        "Bouncy spring: angular_freq={}, damping_ratio={}",
        bouncy.angular_frequency(),
        bouncy.damping_ratio()
    );

    // 1D Spring Value animation
    println!("\n--- 1D Spring Animation ---");
    let mut value = SpringValue::smooth(0.0);
    value.set_target(100.0);

    println!("Animating from 0 to 100:");
    for frame in 0..=10 {
        println!(
            "  Frame {:2}: position={:6.2}, settled={}",
            frame,
            value.get(),
            value.is_settled()
        );

        // Simulate 10 frames at once for faster demo
        for _ in 0..10 {
            value.tick();
        }
    }

    // 2D Spring Value animation
    println!("\n--- 2D Spring Animation ---");
    let mut pos = SpringValue2D::smooth(0.0, 0.0);
    pos.set_target(100.0, 50.0);

    println!("Animating from (0,0) to (100,50):");
    for frame in 0..=5 {
        let (x, y) = pos.get();
        println!("  Frame {:2}: position=({:6.2}, {:6.2})", frame, x, y);

        for _ in 0..20 {
            pos.tick();
        }
    }

    // Color Spring animation
    println!("\n--- Color Spring Animation ---");
    let mut color = SpringColor::smooth(255, 0, 0); // Start red
    color.set_target(0, 0, 255); // Animate to blue

    println!("Animating from Red to Blue:");
    for frame in 0..=5 {
        let (r, g, b) = color.get();
        // Print colored block
        print!("  Frame {:2}: RGB({:3}, {:3}, {:3}) ", frame, r, g, b);
        println!("\x1b[38;2;{};{};{}m████████\x1b[0m", r, g, b);

        for _ in 0..20 {
            color.tick();
        }
    }

    // Snap demonstration
    println!("\n--- Snap (Instant Jump) ---");
    let mut value = SpringValue::smooth(0.0);
    value.set_target(100.0);

    // Tick a few times
    for _ in 0..50 {
        value.tick();
    }
    println!("After animating: position={:.2}", value.get());

    // Snap to new position
    value.snap_to(200.0);
    println!(
        "After snap_to(200): position={:.2}, velocity={:.2}",
        value.get(),
        value.velocity()
    );

    // Different spring presets comparison
    println!("\n--- Spring Presets Comparison ---");
    println!("Simulating 100 frames for each preset (0 -> 100):\n");

    let presets: Vec<(&str, Spring)> = vec![
        ("Smooth", Spring::smooth(60.0)),
        ("Bouncy", Spring::bouncy(60.0)),
        ("Stiff", Spring::stiff(60.0)),
        ("Snappy", Spring::snappy(60.0)),
        ("Gentle", Spring::gentle(60.0)),
    ];

    for (name, spring) in presets {
        let mut value = SpringValue::new(0.0, spring);
        value.set_target(100.0);

        // Collect positions at key frames
        let mut positions = vec![value.get()];
        for _ in 0..100 {
            value.tick();
            if positions.len() < 6 {
                positions.push(value.get());
            }
        }
        positions.push(value.get());

        println!(
            "  {:8}: start={:5.1} -> frame5={:5.1} -> final={:5.1} (settled={})",
            name,
            positions[0],
            positions[5],
            positions.last().unwrap(),
            value.is_settled()
        );
    }
}
