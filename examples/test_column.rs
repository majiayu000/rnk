//! Test column layout rendering
use rnk::prelude::*;

fn main() {
    let text = "Available slash commands:\n\n## Built-in Commands\n- /status - Show agent status\n- /init - Initialize Sage in project";
    
    println!("=== Test: Column Layout with Multiple Text Children ===\n");
    
    let mut container = Box::new().flex_direction(FlexDirection::Column);
    for line in text.lines() {
        container = container.child(Text::new(line).color(Color::White).into_element());
    }
    let element = container.into_element();
    
    let rendered = rnk::render_to_string(&element, 80);
    println!("Rendered output ({} lines):", rendered.lines().count());
    println!("---");
    println!("{}", rendered);
    println!("---");
    
    // Debug: print each line with line number
    println!("\nLine-by-line debug:");
    for (i, line) in rendered.lines().enumerate() {
        println!("Line {}: len={} '{}'", i, line.len(), line);
    }
}
