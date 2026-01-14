//! Fullscreen test - mimics sage CLI exactly
use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;
use crossterm::{
    terminal::{self, ClearType},
    cursor, execute,
    event::{self, Event, KeyCode},
};
use std::io::{self, Write};
use std::time::Duration;

fn main() -> io::Result<()> {
    // Enter alternate screen and raw mode (EXACTLY like sage)
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    // Get terminal size
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    // Clear screen and move to top
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    // Build UI exactly like sage
    let welcome = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Sage Agent").color(Color::Cyan).bold().into_element())
        .child(Text::new("Rust-based LLM Agent").dim().into_element())
        .child(Newline::new().into_element())
        .child(Text::new("Type a message to get started, or use /help for commands").dim().into_element())
        .into_element();

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
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("▸▸").into_element())
                .child(Text::new(" permissions required").dim().into_element())
                .into_element(),
        )
        .into_element();

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

    // Render and print (EXACTLY like sage)
    let output = rnk::render_to_string(&root, term_width);
    print!("{}", output);
    stdout.flush()?;

    // Wait for any key press
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }
    }

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;

    Ok(())
}
