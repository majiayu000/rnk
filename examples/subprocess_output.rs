//! Subprocess output example - displaying output from child processes
//!
//! Equivalent to ink's examples/subprocess-output
//!
//! Run with: cargo run --example subprocess_output

use std::process::Command;
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

#[derive(Clone)]
struct ProcessOutput {
    command: String,
    output: String,
    success: bool,
}

fn app() -> Element {
    let app = use_app();
    let outputs = use_signal(|| Vec::<ProcessOutput>::new());
    let is_running = use_signal(|| false);
    let current_cmd = use_signal(|| String::new());

    let outputs_clone = outputs.clone();
    let is_running_clone = is_running.clone();
    let current_cmd_clone = current_cmd.clone();

    use_input(move |ch, key| {
        if ch == "q" && !is_running_clone.get() {
            app.exit();
        } else if key.return_key && !is_running_clone.get() {
            let cmd = current_cmd_clone.get();
            if !cmd.is_empty() {
                is_running_clone.set(true);

                // Run command
                let result = if cfg!(target_os = "windows") {
                    Command::new("cmd").args(["/C", &cmd]).output()
                } else {
                    Command::new("sh").args(["-c", &cmd]).output()
                };

                let output = match result {
                    Ok(output) => ProcessOutput {
                        command: cmd.clone(),
                        output: String::from_utf8_lossy(&output.stdout).to_string()
                            + &String::from_utf8_lossy(&output.stderr),
                        success: output.status.success(),
                    },
                    Err(e) => ProcessOutput {
                        command: cmd.clone(),
                        output: format!("Error: {}", e),
                        success: false,
                    },
                };

                outputs_clone.update(|list| list.push(output));
                current_cmd_clone.set(String::new());
                is_running_clone.set(false);
            }
        } else if key.backspace || key.delete {
            if !is_running_clone.get() {
                current_cmd_clone.update(|s| { s.pop(); });
            }
        } else if !ch.is_empty() && !key.ctrl && !key.alt && !is_running_clone.get() {
            current_cmd_clone.update(|s| s.push_str(ch));
        }
    });

    let current_outputs = outputs.get();
    let running = is_running.get();
    let cmd = current_cmd.get();

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
                    Text::new("Subprocess Output Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Previous outputs
        .children(current_outputs.iter().map(|output| {
            Box::new()
                .flex_direction(FlexDirection::Column)
                .margin_bottom(1.0)
                .child(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(
                            Text::new(if output.success { "✓ " } else { "✗ " })
                                .color(if output.success { Color::Green } else { Color::Red })
                                .into_element(),
                        )
                        .child(
                            Text::new(format!("$ {}", output.command))
                                .color(Color::Yellow)
                                .bold()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(
                    Box::new()
                        .padding_left(2.0)
                        .child(
                            Text::new(truncate_output(&output.output, 5))
                                .color(Color::Ansi256(250))
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element()
        }))
        // Input area
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(if running { Color::Yellow } else { Color::Green })
                .padding(1)
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new("$ ")
                        .color(Color::Green)
                        .bold()
                        .into_element(),
                )
                .child({
                    let display_text = if running {
                        "Running...".to_string()
                    } else {
                        format!("{}_", cmd)
                    };
                    Text::new(display_text)
                        .color(if running { Color::Yellow } else { Color::White })
                        .into_element()
                })
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Help
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Type a command and press Enter to run").dim().into_element())
                .child(Text::new("Try: echo hello, ls, date, pwd").dim().into_element())
                .child(Text::new("Press 'q' to quit (when not running)").dim().into_element())
                .into_element(),
        )
        .into_element()
}

fn truncate_output(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() > max_lines {
        let mut result: Vec<&str> = lines.into_iter().take(max_lines).collect();
        result.push("...(truncated)");
        result.join("\n")
    } else {
        output.trim().to_string()
    }
}
