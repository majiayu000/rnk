//! Timer and Stopwatch demo
//!
//! Run: cargo run --example timer_demo

use rnk::components::{StopwatchState, TimerState, format_duration_mmss, format_duration_precise};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Timer & Stopwatch Demo ===\n");

    // Timer demo
    println!("--- Countdown Timer ---");
    let mut timer = TimerState::new(Duration::from_secs(5));

    println!(
        "Initial: {} (progress: {:.0}%)",
        timer.format_mmss(),
        timer.progress() * 100.0
    );
    println!("Starting timer...");
    timer.start();

    for i in 1..=5 {
        thread::sleep(Duration::from_secs(1));
        timer.tick();
        println!(
            "After {}s: {} (progress: {:.0}%, running: {}, finished: {})",
            i,
            timer.format_mmss(),
            timer.progress() * 100.0,
            timer.running,
            timer.is_finished()
        );
    }

    println!();

    // Stopwatch demo
    println!("--- Stopwatch ---");
    let mut stopwatch = StopwatchState::new();

    println!("Starting stopwatch...");
    stopwatch.start();

    for i in 1..=3 {
        thread::sleep(Duration::from_millis(500));
        stopwatch.tick();
        println!("After {}ms: {}", i * 500, stopwatch.format_precise());

        // Record a lap
        stopwatch.lap();
    }

    stopwatch.pause();
    println!("\nPaused at: {}", stopwatch.format_precise());

    println!("\nLap times:");
    for (i, lap) in stopwatch.laps().iter().enumerate() {
        println!("  Lap {}: {}", i + 1, format_duration_precise(*lap));
    }

    println!("\nSplit times (time between laps):");
    for (i, split) in stopwatch.splits().iter().enumerate() {
        println!("  Split {}: {}", i + 1, format_duration_precise(*split));
    }

    // Format helpers demo
    println!("\n--- Format Helpers ---");
    let d = Duration::from_secs(3661); // 1 hour, 1 minute, 1 second
    println!("Duration: {:?}", d);
    println!("  format_duration_mmss: {}", format_duration_mmss(d));

    let d2 = Duration::from_millis(125456);
    println!("Duration: {:?}", d2);
    println!("  format_duration_precise: {}", format_duration_precise(d2));
}
