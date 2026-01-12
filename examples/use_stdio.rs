//! Use stdout/stderr example - demonstrating standard I/O hooks
//!
//! Equivalent to ink's examples/use-stdout and use-stderr
//!
//! Run with: cargo run --example use_stdio

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let stdout = use_stdout();
    let stderr = use_stderr();
    let stdin = use_stdin();

    let stdout_count = use_signal(|| 0usize);
    let stderr_count = use_signal(|| 0usize);

    let stdout_clone = stdout;
    let stderr_clone = stderr;
    let stdout_count_clone = stdout_count.clone();
    let stderr_count_clone = stderr_count.clone();

    use_input(move |ch, _key| match ch {
        "q" => app.exit(),
        "o" => {
            let count = stdout_count_clone.get();
            stdout_count_clone.set(count + 1);
            let _ = stdout_clone.writeln(&format!("[stdout] Message #{}", count + 1));
        }
        "e" => {
            let count = stderr_count_clone.get();
            stderr_count_clone.set(count + 1);
            let _ = stderr_clone.writeln(&format!("[stderr] Error #{}", count + 1));
        }
        _ => {}
    });

    let is_tty = stdin.is_tty();
    let stdout_is_tty = stdin.stdout_is_tty();
    let stderr_is_tty = stdin.stderr_is_tty();

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
                    Text::new("Standard I/O Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // TTY Status
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Yellow)
                .padding(1)
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("TTY Status:")
                        .color(Color::Yellow)
                        .bold()
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("  stdin:  ").into_element())
                        .child(tty_indicator(is_tty))
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("  stdout: ").into_element())
                        .child(tty_indicator(stdout_is_tty))
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("  stderr: ").into_element())
                        .child(tty_indicator(stderr_is_tty))
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Counters
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Green)
                        .padding(1)
                        .margin_right(2.0)
                        .child(
                            Text::new(format!("stdout: {}", stdout_count.get()))
                                .color(Color::Green)
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Red)
                        .padding(1)
                        .child(
                            Text::new(format!("stderr: {}", stderr_count.get()))
                                .color(Color::Red)
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
                .child(Text::new("  o - Write to stdout").dim().into_element())
                .child(Text::new("  e - Write to stderr").dim().into_element())
                .child(Text::new("  q - Quit").dim().into_element())
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("Note: stdout/stderr writes appear outside the UI")
                .color(Color::Ansi256(245))
                .italic()
                .into_element(),
        )
        .into_element()
}

fn tty_indicator(is_tty: bool) -> Element {
    if is_tty {
        Text::new("TTY").color(Color::Green).bold().into_element()
    } else {
        Text::new("Not TTY").color(Color::Red).into_element()
    }
}
