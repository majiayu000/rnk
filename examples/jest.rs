//! Jest-like test runner example - simulating concurrent test execution
//!
//! Equivalent to ink's examples/jest
//!
//! Run with: cargo run --example jest

use std::time::{Duration, Instant};
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

#[derive(Clone)]
struct TestResult {
    path: String,
    status: TestStatus,
}

#[derive(Clone, PartialEq)]
enum TestStatus {
    Running,
    Pass,
    Fail,
}

fn app() -> Element {
    let app = use_app();
    let start_time = use_signal(|| Instant::now());
    let completed_tests = use_signal(|| Vec::<TestResult>::new());
    let running_tests = use_signal(|| Vec::<TestResult>::new());
    let next_test_idx = use_signal(|| 0usize);
    let tick = use_signal(|| 0u32);

    let test_paths: Vec<&'static str> = vec![
        "tests/login.rs",
        "tests/signup.rs",
        "tests/forgot_password.rs",
        "tests/reset_password.rs",
        "tests/view_profile.rs",
        "tests/edit_profile.rs",
        "tests/delete_profile.rs",
        "tests/posts.rs",
        "tests/post.rs",
        "tests/comments.rs",
    ];

    let running_clone = running_tests.clone();
    let completed_clone = completed_tests.clone();
    let next_idx_clone = next_test_idx.clone();
    let tick_clone = tick.clone();
    let paths_len = test_paths.len();

    use_input(move |ch, _key| {
        if ch == "q" {
            app.exit();
        } else if ch == "n" || ch == " " {
            // Advance simulation on keypress
            let idx = next_idx_clone.get();

            // Start new tests (up to 4 concurrent)
            if idx < paths_len && running_clone.get().len() < 4 {
                let path = test_paths[idx].to_string();
                running_clone.update(|tests| {
                    tests.push(TestResult {
                        path,
                        status: TestStatus::Running,
                    });
                });
                next_idx_clone.set(idx + 1);
            }

            // Randomly complete a running test
            let running = running_clone.get();
            if !running.is_empty() {
                let t = tick_clone.get();
                tick_clone.set(t + 1);
                let completed_idx = (t as usize) % running.len();

                if let Some(test) = running.get(completed_idx) {
                    let path = test.path.clone();
                    let status = if rand_bool() { TestStatus::Pass } else { TestStatus::Fail };

                    running_clone.update(|tests| {
                        tests.retain(|t| t.path != path);
                    });

                    completed_clone.update(|tests| {
                        tests.push(TestResult { path, status });
                    });
                }
            }
        }
    });

    let elapsed = start_time.get().elapsed();
    let completed = completed_tests.get();
    let running = running_tests.get();
    let passed = completed.iter().filter(|t| t.status == TestStatus::Pass).count();
    let failed = completed.iter().filter(|t| t.status == TestStatus::Fail).count();
    let is_finished = completed.len() == 10 && running.is_empty();

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Completed tests (using Static-like rendering)
        .children(completed.iter().map(|test| render_test(test)))
        // Running tests
        .child(
            if !running.is_empty() {
                Box::new()
                    .flex_direction(FlexDirection::Column)
                    .margin_top(1.0)
                    .children(running.iter().map(|test| render_test(test)))
                    .into_element()
            } else {
                Box::new().into_element()
            },
        )
        // Summary
        .child(Newline::new().into_element())
        .child(render_summary(is_finished, passed, failed, elapsed))
        .child(Newline::new().into_element())
        .child(Text::new("Press 'n' or Space to advance, 'q' to exit").dim().into_element())
        .into_element()
}

fn render_test(test: &TestResult) -> Element {
    let (badge, color) = match test.status {
        TestStatus::Running => (" RUNS ", Color::Yellow),
        TestStatus::Pass => (" PASS ", Color::Green),
        TestStatus::Fail => (" FAIL ", Color::Red),
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Box::new()
                .background(color)
                .child(
                    Text::new(badge)
                        .color(Color::Black)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(
            Text::new(format!(" {}", test.path))
                .color(Color::White)
                .into_element(),
        )
        .into_element()
}

fn render_summary(is_finished: bool, passed: usize, failed: usize, elapsed: Duration) -> Element {
    let time_str = format!("{:.1}s", elapsed.as_secs_f64());

    Box::new()
        .flex_direction(FlexDirection::Column)
        .border_style(BorderStyle::Single)
        .border_color(if is_finished && failed == 0 {
            Color::Green
        } else if failed > 0 {
            Color::Red
        } else {
            Color::Yellow
        })
        .padding(1)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("Tests: ").bold().into_element())
                .child(
                    Text::new(format!("{} passed", passed))
                        .color(Color::Green)
                        .bold()
                        .into_element(),
                )
                .child(Text::new(", ").into_element())
                .child(
                    Text::new(format!("{} failed", failed))
                        .color(if failed > 0 { Color::Red } else { Color::Ansi256(240) })
                        .bold()
                        .into_element(),
                )
                .child(Text::new(format!(", {} total", passed + failed)).into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("Time:  ").bold().into_element())
                .child(Text::new(time_str).color(Color::Yellow).into_element())
                .into_element(),
        )
        .child(
            if is_finished {
                Text::new(if failed == 0 { "All tests passed!" } else { "Some tests failed" })
                    .color(if failed == 0 { Color::Green } else { Color::Red })
                    .bold()
                    .into_element()
            } else {
                Text::new("Running...").color(Color::Yellow).into_element()
            },
        )
        .into_element()
}

fn rand_bool() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() % 2 == 0
}
