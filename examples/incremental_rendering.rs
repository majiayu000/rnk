//! Incremental rendering example - showing conditional and incremental updates
//!
//! Equivalent to ink's examples/incremental-rendering
//!
//! Run with: cargo run --example incremental_rendering

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

#[derive(Clone)]
struct Step {
    name: String,
    status: StepStatus,
}

#[derive(Clone, PartialEq)]
enum StepStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

fn app() -> Element {
    let app = use_app();
    let steps = use_signal(|| vec![
        Step { name: "Initializing".to_string(), status: StepStatus::Pending },
        Step { name: "Fetching dependencies".to_string(), status: StepStatus::Pending },
        Step { name: "Building project".to_string(), status: StepStatus::Pending },
        Step { name: "Running tests".to_string(), status: StepStatus::Pending },
        Step { name: "Deploying".to_string(), status: StepStatus::Pending },
    ]);

    let steps_clone = steps.clone();

    use_input(move |ch, _key| {
        match ch {
            "q" => app.exit(),
            "n" => {
                // Advance to next step
                steps_clone.update(|s| {
                    // Find first pending and mark previous as complete
                    let mut found_running = false;
                    for step in s.iter_mut() {
                        if step.status == StepStatus::Running {
                            step.status = StepStatus::Complete;
                            found_running = true;
                        } else if step.status == StepStatus::Pending && found_running {
                            step.status = StepStatus::Running;
                            break;
                        } else if step.status == StepStatus::Pending && !found_running {
                            step.status = StepStatus::Running;
                            break;
                        }
                    }
                });
            }
            "f" => {
                // Fail current step
                steps_clone.update(|s| {
                    for step in s.iter_mut() {
                        if step.status == StepStatus::Running {
                            step.status = StepStatus::Failed;
                            break;
                        }
                    }
                });
            }
            "r" => {
                // Reset all steps
                steps_clone.update(|s| {
                    for step in s.iter_mut() {
                        step.status = StepStatus::Pending;
                    }
                });
            }
            _ => {}
        }
    });

    let current_steps = steps.get();
    let all_complete = current_steps.iter().all(|s| s.status == StepStatus::Complete);
    let has_failed = current_steps.iter().any(|s| s.status == StepStatus::Failed);

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
                    Text::new("Incremental Rendering Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Steps
        .children(current_steps.iter().map(|step| render_step(step)))
        .child(Newline::new().into_element())
        // Status
        .child(
            if all_complete {
                Box::new()
                    .border_style(BorderStyle::Double)
                    .border_color(Color::Green)
                    .padding(1)
                    .child(
                        Text::new("All steps completed successfully!")
                            .color(Color::Green)
                            .bold()
                            .into_element(),
                    )
                    .into_element()
            } else if has_failed {
                Box::new()
                    .border_style(BorderStyle::Double)
                    .border_color(Color::Red)
                    .padding(1)
                    .child(
                        Text::new("Pipeline failed!")
                            .color(Color::Red)
                            .bold()
                            .into_element(),
                    )
                    .into_element()
            } else {
                Box::new().into_element()
            },
        )
        .child(Newline::new().into_element())
        // Help
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Controls:").dim().into_element())
                .child(Text::new("  n - Next step").dim().into_element())
                .child(Text::new("  f - Fail current step").dim().into_element())
                .child(Text::new("  r - Reset all").dim().into_element())
                .child(Text::new("  q - Quit").dim().into_element())
                .into_element(),
        )
        .into_element()
}

fn render_step(step: &Step) -> Element {
    let (icon, color) = match step.status {
        StepStatus::Pending => ("○", Color::Ansi256(240)),
        StepStatus::Running => ("◐", Color::Yellow),
        StepStatus::Complete => ("●", Color::Green),
        StepStatus::Failed => ("✗", Color::Red),
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .margin_bottom(0.5)
        .child(
            Text::new(format!("{} ", icon))
                .color(color)
                .into_element(),
        )
        .child(
            Text::new(&step.name)
                .color(match step.status {
                    StepStatus::Pending => Color::Ansi256(245),
                    StepStatus::Running => Color::Yellow,
                    StepStatus::Complete => Color::Green,
                    StepStatus::Failed => Color::Red,
                })
                .bold()
                .into_element(),
        )
        .child(
            if step.status == StepStatus::Running {
                Text::new(" (in progress...)")
                    .color(Color::Yellow)
                    .italic()
                    .into_element()
            } else {
                Box::new().into_element()
            },
        )
        .into_element()
}
