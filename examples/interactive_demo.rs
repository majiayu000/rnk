//! Interactive Demo - Full working demo with keyboard input
//!
//! This demonstrates the complete tink feature set with real interactivity.
//! Uses rnk's built-in render API for simplicity.
//!
//! Run with: cargo run --example interactive_demo

use std::io::{Write, stdout};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{self, ClearType},
};

use rnk::core::Dimension;
use rnk::prelude::*;

/// Demo state
struct DemoState {
    counter: i32,
    selected_item: usize,
    items: Vec<String>,
    show_popup: bool,
    messages: Vec<String>,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            counter: 0,
            selected_item: 0,
            items: vec![
                "First item".into(),
                "Second item".into(),
                "Third item".into(),
                "Fourth item".into(),
            ],
            show_popup: false,
            messages: vec!["Welcome! Press 'h' for help.".into()],
        }
    }
}

fn render_ui(state: &DemoState) -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .padding_y(1.0)
                .child(
                    Text::new("Tink Interactive Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Main content - two columns
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                // Left column - counter
                .child(
                    Box::new()
                        .width(30)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Yellow)
                        .padding(1)
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("Counter Demo")
                                .color(Color::Yellow)
                                .bold()
                                .underline()
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(
                            Box::new()
                                .flex_direction(FlexDirection::Row)
                                .child(Text::new("Value: ").color(Color::White).into_element())
                                .child(
                                    Text::new(format!("{}", state.counter))
                                        .color(if state.counter >= 0 {
                                            Color::Green
                                        } else {
                                            Color::Red
                                        })
                                        .bold()
                                        .into_element(),
                                )
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(
                            Text::new("[+] Increment")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .child(
                            Text::new("[-] Decrement")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .child(
                            Text::new("[0] Reset")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Box::new().width(2).into_element())
                // Right column - list
                .child(
                    Box::new()
                        .flex_grow(1.0)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Blue)
                        .padding(1)
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("List Navigation")
                                .color(Color::Blue)
                                .bold()
                                .underline()
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .children(state.items.iter().enumerate().map(|(i, item)| {
                            let is_selected = i == state.selected_item;
                            let mut row =
                                Box::new().flex_direction(FlexDirection::Row).padding_x(1.0);

                            if is_selected {
                                row = row.background(Color::Ansi256(236));
                            }

                            row.child(
                                Text::new(if is_selected { "> " } else { "  " })
                                    .color(Color::Cyan)
                                    .bold()
                                    .into_element(),
                            )
                            .child(
                                Text::new(item)
                                    .color(if is_selected {
                                        Color::White
                                    } else {
                                        Color::Ansi256(250)
                                    })
                                    .into_element(),
                            )
                            .into_element()
                        }))
                        .child(Newline::new().into_element())
                        .child(
                            Text::new("[j/k] Navigate")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Status messages
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Magenta)
                .padding(1)
                .child(
                    Text::new(state.messages.last().unwrap_or(&"".to_string()))
                        .color(Color::Magenta)
                        .italic()
                        .into_element(),
                )
                .into_element(),
        )
        // Help popup (absolute positioned, conditionally shown)
        .child(render_help_popup(state.show_popup))
        .into_element()
}

fn render_help_popup(show: bool) -> Element {
    if !show {
        return Box::new().hidden().into_element();
    }

    Box::new()
        .position_absolute()
        .top(3.0)
        .left(15.0)
        .width(40)
        .border_style(BorderStyle::Double)
        .border_color(Color::Green)
        .background(Color::Ansi256(234))
        .padding(2)
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("  HELP  ")
                .color(Color::Green)
                .bold()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("  +/-/0  Counter controls")
                .color(Color::White)
                .into_element(),
        )
        .child(
            Text::new("  j/k    Navigate list")
                .color(Color::White)
                .into_element(),
        )
        .child(
            Text::new("  h      Toggle this help")
                .color(Color::White)
                .into_element(),
        )
        .child(
            Text::new("  q/Esc  Quit")
                .color(Color::White)
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("Press 'h' to close")
                .color(Color::Ansi256(240))
                .italic()
                .into_element(),
        )
        .into_element()
}

fn main() -> std::io::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut state = DemoState::default();

    // Main loop
    loop {
        // Get current terminal size (in case it changed)
        let (width, _) = terminal::size()?;

        // Render using rnk's built-in API
        let element = render_ui(&state);
        let output = rnk::render_to_string(&element, width);

        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;
        write!(stdout, "{}", output)?;
        stdout.flush()?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('h') => {
                        state.show_popup = !state.show_popup;
                        state.messages.push(if state.show_popup {
                            "Help opened".into()
                        } else {
                            "Help closed".into()
                        });
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        state.counter += 1;
                        state.messages.push(format!("Counter: {}", state.counter));
                    }
                    KeyCode::Char('-') => {
                        state.counter -= 1;
                        state.messages.push(format!("Counter: {}", state.counter));
                    }
                    KeyCode::Char('0') => {
                        state.counter = 0;
                        state.messages.push("Counter reset".into());
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if state.selected_item < state.items.len() - 1 {
                            state.selected_item += 1;
                            state
                                .messages
                                .push(format!("Selected: {}", state.items[state.selected_item]));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if state.selected_item > 0 {
                            state.selected_item -= 1;
                            state
                                .messages
                                .push(format!("Selected: {}", state.items[state.selected_item]));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    println!("Thanks for trying Tink!");
    Ok(())
}
