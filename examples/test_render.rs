use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;

fn main() {
    // Get actual terminal size
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
    println!("Terminal size: {}x{}", term_width, term_height);
    
    // Simple test
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(Text::new("Line 1 - This should be at column 0").into_element())
        .child(Text::new("Line 2 - Also at column 0").into_element())
        .into_element();
    
    let output = rnk::render_to_string(&element, term_width);
    
    // Print each line with position info
    for (i, line) in output.lines().enumerate() {
        let first_char_pos = line.chars().position(|c| c != ' ').unwrap_or(0);
        if !line.is_empty() {
            println!("Line {}: starts at col {}, content: |{}|", i, first_char_pos, line);
        }
    }
}
