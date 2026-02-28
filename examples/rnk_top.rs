//! rnk-top: An htop-like system monitor built with rnk
//!
//! Demonstrates rnk's capabilities for building complex TUI applications.
//!
//! Run with: cargo run --example rnk_top

use rnk::cmd::Cmd;
use rnk::prelude::*;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    render(app).fullscreen().fps(4).run()
}

fn app() -> Element {
    let processes = use_signal(generate_mock_processes);
    let selected = use_signal(|| 0usize);
    let cpu_history = use_signal(|| vec![30.0; 60]);
    let mem_history = use_signal(|| vec![45.0; 60]);
    let sort_by = use_signal(|| SortBy::Cpu);
    let app = use_app();

    // Clone for closures
    let processes_cmd = processes.clone();
    let cpu_history_cmd = cpu_history.clone();
    let mem_history_cmd = mem_history.clone();

    // Update data periodically
    use_cmd_once(move |_| {
        Cmd::every(Duration::from_millis(1000), move |_| {
            // Update CPU history
            cpu_history_cmd.update(|h| {
                h.remove(0);
                h.push(20.0 + (rand_f64() * 60.0));
            });

            // Update memory history
            mem_history_cmd.update(|h| {
                h.remove(0);
                h.push(40.0 + (rand_f64() * 30.0));
            });

            // Update process data
            processes_cmd.update(|procs| {
                for proc in procs.iter_mut() {
                    proc.cpu = (proc.cpu + rand_f64() * 10.0 - 5.0).clamp(0.0, 100.0);
                    proc.mem = (proc.mem + rand_f64() * 5.0 - 2.5).clamp(0.0, 100.0);
                }
            });
        })
    });

    // Clone for input handler
    let processes_input = processes.clone();
    let sort_by_input = sort_by.clone();
    let selected_input = selected.clone();

    // Handle input
    use_input(move |input, key| {
        let proc_count = processes_input.get().len();
        if input == "q" {
            app.exit();
        } else if key.down_arrow || input == "j" {
            selected_input.update(|s| *s = (*s + 1).min(proc_count.saturating_sub(1)));
        } else if key.up_arrow || input == "k" {
            selected_input.update(|s| *s = s.saturating_sub(1));
        } else if input == "c" {
            sort_by_input.set(SortBy::Cpu);
        } else if input == "m" {
            sort_by_input.set(SortBy::Memory);
        } else if input == "p" {
            sort_by_input.set(SortBy::Pid);
        } else if input == "n" {
            sort_by_input.set(SortBy::Name);
        }
    });

    // Sort processes
    let mut sorted_procs = processes.get();
    match sort_by.get() {
        SortBy::Cpu => sorted_procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap()),
        SortBy::Memory => sorted_procs.sort_by(|a, b| b.mem.partial_cmp(&a.mem).unwrap()),
        SortBy::Pid => sorted_procs.sort_by_key(|p| p.pid),
        SortBy::Name => sorted_procs.sort_by(|a, b| a.name.cmp(&b.name)),
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .children(vec![
            header(),
            system_stats(cpu_history.get(), mem_history.get()),
            process_header(sort_by.get()),
            process_list(sorted_procs, selected.get()),
            footer(),
        ])
        .into_element()
}

fn header() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(JustifyContent::SpaceBetween)
        .padding_x(1.0)
        .background(Color::Ansi256(236))
        .child(
            Text::new("rnk-top")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(format!("rnk v{}", env!("CARGO_PKG_VERSION")))
                .dim()
                .into_element(),
        )
        .into_element()
}

fn system_stats(cpu_history: Vec<f64>, mem_history: Vec<f64>) -> Element {
    let cpu_avg = cpu_history.iter().sum::<f64>() / cpu_history.len() as f64;
    let mem_avg = mem_history.iter().sum::<f64>() / mem_history.len() as f64;

    Box::new()
        .flex_direction(FlexDirection::Row)
        .padding(1)
        .gap(2.0)
        .children(vec![
            // CPU section
            Box::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .children(vec![
                    Text::new("CPU").color(Color::Cyan).bold().into_element(),
                    Sparkline::from_data(cpu_history)
                        .color(Color::Green)
                        .into_element(),
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .children(vec![
                            Text::new("Avg: ").dim().into_element(),
                            Text::new(format!("{:.1}%", cpu_avg))
                                .color(cpu_color(cpu_avg))
                                .into_element(),
                        ])
                        .into_element(),
                ])
                .into_element(),
            // Memory section
            Box::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .children(vec![
                    Text::new("Memory").color(Color::Cyan).bold().into_element(),
                    Sparkline::from_data(mem_history)
                        .color(Color::Yellow)
                        .into_element(),
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .children(vec![
                            Text::new("Used: ").dim().into_element(),
                            Text::new(format!("{:.1}%", mem_avg))
                                .color(mem_color(mem_avg))
                                .into_element(),
                            Text::new(" (6.2 / 16.0 GB)").dim().into_element(),
                        ])
                        .into_element(),
                ])
                .into_element(),
            // System info
            Box::new()
                .flex_direction(FlexDirection::Column)
                .width(25)
                .children(vec![
                    Text::new("System").color(Color::Cyan).bold().into_element(),
                    Text::new("Uptime: 3d 14:23:45").dim().into_element(),
                    Text::new("Load: 2.34 1.89 1.56").dim().into_element(),
                    Text::new("Tasks: 234 (2 running)").dim().into_element(),
                ])
                .into_element(),
        ])
        .into_element()
}

fn process_header(sort_by: SortBy) -> Element {
    let header_style = |col: SortBy| {
        if sort_by == col {
            Color::Cyan
        } else {
            Color::White
        }
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .background(Color::Ansi256(238))
        .children(vec![
            Box::new()
                .width(8)
                .child(
                    Text::new("PID")
                        .color(header_style(SortBy::Pid))
                        .bold()
                        .into_element(),
                )
                .into_element(),
            Box::new()
                .width(20)
                .child(
                    Text::new("NAME")
                        .color(header_style(SortBy::Name))
                        .bold()
                        .into_element(),
                )
                .into_element(),
            Box::new()
                .width(10)
                .child(
                    Text::new("CPU%")
                        .color(header_style(SortBy::Cpu))
                        .bold()
                        .into_element(),
                )
                .into_element(),
            Box::new()
                .width(10)
                .child(
                    Text::new("MEM%")
                        .color(header_style(SortBy::Memory))
                        .bold()
                        .into_element(),
                )
                .into_element(),
            Box::new()
                .width(12)
                .child(Text::new("STATUS").bold().into_element())
                .into_element(),
            Box::new()
                .flex_grow(1.0)
                .child(Text::new("COMMAND").bold().into_element())
                .into_element(),
        ])
        .into_element()
}

fn process_list(processes: Vec<Process>, selected: usize) -> Element {
    let mut children = Vec::new();

    for (i, proc) in processes.iter().take(15).enumerate() {
        let is_selected = i == selected;
        let bg = if is_selected {
            Color::Ansi256(240)
        } else {
            Color::Reset
        };

        children.push(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .padding_x(1.0)
                .background(bg)
                .children(vec![
                    Box::new()
                        .width(8)
                        .child(Text::new(format!("{}", proc.pid)).dim().into_element())
                        .into_element(),
                    Box::new()
                        .width(20)
                        .child(
                            Text::new(&proc.name)
                                .color(if is_selected {
                                    Color::White
                                } else {
                                    Color::Reset
                                })
                                .into_element(),
                        )
                        .into_element(),
                    Box::new()
                        .width(10)
                        .child(
                            Text::new(format!("{:.1}", proc.cpu))
                                .color(cpu_color(proc.cpu))
                                .into_element(),
                        )
                        .into_element(),
                    Box::new()
                        .width(10)
                        .child(
                            Text::new(format!("{:.1}", proc.mem))
                                .color(mem_color(proc.mem))
                                .into_element(),
                        )
                        .into_element(),
                    Box::new()
                        .width(12)
                        .child(
                            Text::new(&proc.status)
                                .color(status_color(&proc.status))
                                .into_element(),
                        )
                        .into_element(),
                    Box::new()
                        .flex_grow(1.0)
                        .child(Text::new(&proc.command).dim().into_element())
                        .into_element(),
                ])
                .into_element(),
        );
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .flex_grow(1.0)
        .children(children)
        .into_element()
}

fn footer() -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .background(Color::Ansi256(236))
        .gap(2.0)
        .children(vec![
            Text::new("q").color(Color::Yellow).bold().into_element(),
            Text::new("Quit").dim().into_element(),
            Text::new("c").color(Color::Yellow).bold().into_element(),
            Text::new("Sort CPU").dim().into_element(),
            Text::new("m").color(Color::Yellow).bold().into_element(),
            Text::new("Sort Mem").dim().into_element(),
            Text::new("p").color(Color::Yellow).bold().into_element(),
            Text::new("Sort PID").dim().into_element(),
            Text::new("n").color(Color::Yellow).bold().into_element(),
            Text::new("Sort Name").dim().into_element(),
            Text::new("↑↓").color(Color::Yellow).bold().into_element(),
            Text::new("Navigate").dim().into_element(),
        ])
        .into_element()
}

// Helper functions
fn cpu_color(cpu: f64) -> Color {
    if cpu > 80.0 {
        Color::Red
    } else if cpu > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn mem_color(mem: f64) -> Color {
    if mem > 80.0 {
        Color::Red
    } else if mem > 60.0 {
        Color::Yellow
    } else {
        Color::Cyan
    }
}

fn status_color(status: &str) -> Color {
    match status {
        "Running" => Color::Green,
        "Sleeping" => Color::BrightBlack,
        "Stopped" => Color::Yellow,
        "Zombie" => Color::Red,
        _ => Color::Reset,
    }
}

// Simple pseudo-random number generator
fn rand_f64() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

// Data structures
#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

#[derive(Clone)]
struct Process {
    pid: u32,
    name: String,
    cpu: f64,
    mem: f64,
    status: String,
    command: String,
}

fn generate_mock_processes() -> Vec<Process> {
    vec![
        Process {
            pid: 1,
            name: "systemd".into(),
            cpu: 0.1,
            mem: 0.5,
            status: "Sleeping".into(),
            command: "/sbin/init".into(),
        },
        Process {
            pid: 1234,
            name: "chrome".into(),
            cpu: 45.2,
            mem: 12.3,
            status: "Running".into(),
            command: "/usr/bin/chrome --type=renderer".into(),
        },
        Process {
            pid: 2345,
            name: "code".into(),
            cpu: 23.1,
            mem: 8.7,
            status: "Running".into(),
            command: "/usr/share/code/code".into(),
        },
        Process {
            pid: 3456,
            name: "rust-analyzer".into(),
            cpu: 67.8,
            mem: 15.2,
            status: "Running".into(),
            command: "rust-analyzer".into(),
        },
        Process {
            pid: 4567,
            name: "cargo".into(),
            cpu: 89.3,
            mem: 6.4,
            status: "Running".into(),
            command: "cargo build --release".into(),
        },
        Process {
            pid: 5678,
            name: "node".into(),
            cpu: 12.4,
            mem: 4.2,
            status: "Sleeping".into(),
            command: "node server.js".into(),
        },
        Process {
            pid: 6789,
            name: "postgres".into(),
            cpu: 3.2,
            mem: 2.1,
            status: "Sleeping".into(),
            command: "postgres: writer process".into(),
        },
        Process {
            pid: 7890,
            name: "redis-server".into(),
            cpu: 1.5,
            mem: 0.8,
            status: "Sleeping".into(),
            command: "redis-server *:6379".into(),
        },
        Process {
            pid: 8901,
            name: "nginx".into(),
            cpu: 0.3,
            mem: 0.2,
            status: "Sleeping".into(),
            command: "nginx: worker process".into(),
        },
        Process {
            pid: 9012,
            name: "docker".into(),
            cpu: 5.6,
            mem: 3.4,
            status: "Sleeping".into(),
            command: "dockerd --host=unix://".into(),
        },
        Process {
            pid: 1122,
            name: "spotify".into(),
            cpu: 8.9,
            mem: 5.6,
            status: "Running".into(),
            command: "/usr/share/spotify/spotify".into(),
        },
        Process {
            pid: 2233,
            name: "slack".into(),
            cpu: 4.3,
            mem: 7.8,
            status: "Sleeping".into(),
            command: "/usr/lib/slack/slack".into(),
        },
        Process {
            pid: 3344,
            name: "firefox".into(),
            cpu: 34.5,
            mem: 9.1,
            status: "Running".into(),
            command: "/usr/lib/firefox/firefox".into(),
        },
        Process {
            pid: 4455,
            name: "kitty".into(),
            cpu: 2.1,
            mem: 1.2,
            status: "Sleeping".into(),
            command: "kitty".into(),
        },
        Process {
            pid: 5566,
            name: "zsh".into(),
            cpu: 0.0,
            mem: 0.1,
            status: "Sleeping".into(),
            command: "-zsh".into(),
        },
    ]
}
