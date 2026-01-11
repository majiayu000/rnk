//! Static component example - demonstrating persistent output
//!
//! Equivalent to ink's examples/static
//!
//! Run with: cargo run --example static_example

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

#[derive(Clone)]
struct LogEntry {
    id: usize,
    message: String,
    level: LogLevel,
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Warn,
    Error,
}

fn app() -> Element {
    let app = use_app();
    let logs = use_signal(|| Vec::<LogEntry>::new());
    let next_id = use_signal(|| 0usize);
    let counter = use_signal(|| 0i32);

    let logs_clone = logs.clone();
    let next_id_clone = next_id.clone();

    // Add log entry helper
    let add_log = move |message: String, level: LogLevel| {
        let id = next_id_clone.get();
        next_id_clone.set(id + 1);
        logs_clone.update(|entries| {
            entries.push(LogEntry { id, message, level });
        });
    };

    let add_log_clone = add_log.clone();
    let counter_clone = counter.clone();

    use_input(move |ch, _key| {
        match ch {
            "q" => app.exit(),
            "i" => add_log_clone(format!("Info message #{}", counter_clone.get()), LogLevel::Info),
            "w" => add_log_clone(format!("Warning message #{}", counter_clone.get()), LogLevel::Warn),
            "e" => add_log_clone(format!("Error message #{}", counter_clone.get()), LogLevel::Error),
            "+" => counter_clone.update(|c| *c += 1),
            "-" => counter_clone.update(|c| *c -= 1),
            _ => {}
        }
    });

    let current_logs = logs.get();
    let current_counter = counter.get();

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .child(
                    Text::new("Static Output Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Static log entries - these persist and don't get re-rendered
        .child(
            Static::new(current_logs.clone(), |entry, _idx| {
                render_log_entry(entry)
            }).into_element(),
        )
        // Separator
        .child(
            if !current_logs.is_empty() {
                Box::new()
                    .margin_y(1.0)
                    .child(
                        Text::new("─".repeat(40))
                            .color(Color::Ansi256(240))
                            .into_element(),
                    )
                    .into_element()
            } else {
                Box::new().into_element()
            },
        )
        // Dynamic counter (this part re-renders)
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Yellow)
                .padding(1)
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("Counter: ").into_element())
                        .child(
                            Text::new(format!("{}", current_counter))
                                .color(if current_counter >= 0 { Color::Green } else { Color::Red })
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Help
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Controls:").dim().into_element())
                .child(Text::new("  i - Add info log").dim().into_element())
                .child(Text::new("  w - Add warning log").dim().into_element())
                .child(Text::new("  e - Add error log").dim().into_element())
                .child(Text::new("  +/- - Change counter").dim().into_element())
                .child(Text::new("  q - Quit").dim().into_element())
                .into_element(),
        )
        .into_element()
}

fn render_log_entry(entry: &LogEntry) -> Element {
    let (prefix, color) = match entry.level {
        LogLevel::Info => ("[INFO]", Color::Blue),
        LogLevel::Warn => ("[WARN]", Color::Yellow),
        LogLevel::Error => ("[ERROR]", Color::Red),
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new(prefix)
                .color(color)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(format!(" {}", entry.message))
                .color(Color::White)
                .into_element(),
        )
        .into_element()
}
