//! Test CRLF vs LF line endings
use rnk::prelude::Box as RnkBox;
use rnk::prelude::*;

fn main() {
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Line 1").into_element())
        .child(Text::new("Line 2").into_element())
        .child(Text::new("Line 3").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, 80);

    println!("=== Output bytes ===");
    for (i, b) in output.bytes().enumerate() {
        if b == b'\r' {
            print!("[CR]");
        } else if b == b'\n' {
            print!("[LF]\n");
        } else {
            print!("{}", b as char);
        }
    }
    println!("\n=== End ===");

    // Check if contains \r\n
    if output.contains("\r\n") {
        println!("Output uses CRLF line endings");
    } else if output.contains("\n") {
        println!("Output uses LF line endings");
    } else {
        println!("No line breaks found");
    }
}
