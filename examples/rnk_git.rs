//! rnk-git: A lazygit-style Git UI built with rnk
//!
//! Demonstrates rnk's capabilities for building multi-panel TUI applications.
//!
//! Run with: cargo run --example rnk_git

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).fullscreen().run()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Status,
    Staged,
    Commits,
    Diff,
}

#[derive(Clone)]
struct FileStatus {
    name: String,
    status: FileState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum FileState {
    Modified,
    Added,
    Deleted,
    Untracked,
    Renamed,
}

#[derive(Clone)]
struct Commit {
    hash: String,
    message: String,
    author: String,
    time: String,
}

fn app() -> Element {
    let active_panel = use_signal(|| Panel::Status);
    let status_selected = use_signal(|| 0usize);
    let staged_selected = use_signal(|| 0usize);
    let commits_selected = use_signal(|| 0usize);

    let unstaged_files = use_signal(|| mock_unstaged_files());
    let staged_files = use_signal(|| mock_staged_files());
    let commits = use_signal(|| mock_commits());

    let app = use_app();

    // Clone for input handler
    let active_panel_input = active_panel.clone();
    let status_selected_input = status_selected.clone();
    let staged_selected_input = staged_selected.clone();
    let commits_selected_input = commits_selected.clone();
    let unstaged_files_input = unstaged_files.clone();
    let staged_files_input = staged_files.clone();
    let commits_input = commits.clone();

    use_input(move |input, key| {
        let panel = active_panel_input.get();

        // Global keys
        if input == "q" {
            app.exit();
            return;
        }

        // Panel switching
        if input == "1" {
            active_panel_input.set(Panel::Status);
            return;
        }
        if input == "2" {
            active_panel_input.set(Panel::Staged);
            return;
        }
        if input == "3" {
            active_panel_input.set(Panel::Commits);
            return;
        }

        // Tab to cycle panels
        if key.tab {
            let next = match panel {
                Panel::Status => Panel::Staged,
                Panel::Staged => Panel::Commits,
                Panel::Commits => Panel::Status,
                Panel::Diff => Panel::Status,
            };
            active_panel_input.set(next);
            return;
        }

        // Navigation within panel
        match panel {
            Panel::Status => {
                let count = unstaged_files_input.get().len();
                if key.down_arrow || input == "j" {
                    status_selected_input.update(|s| *s = (*s + 1).min(count.saturating_sub(1)));
                } else if key.up_arrow || input == "k" {
                    status_selected_input.update(|s| *s = s.saturating_sub(1));
                } else if input == " " || key.return_key {
                    // Stage file
                    let idx = status_selected_input.get();
                    if idx < unstaged_files_input.get().len() {
                        let file = unstaged_files_input.get()[idx].clone();
                        staged_files_input.update(|f| f.push(file));
                        unstaged_files_input.update(|f| {
                            f.remove(idx);
                        });
                        if idx > 0 && idx >= unstaged_files_input.get().len() {
                            status_selected_input.update(|s| *s = s.saturating_sub(1));
                        }
                    }
                }
            }
            Panel::Staged => {
                let count = staged_files_input.get().len();
                if key.down_arrow || input == "j" {
                    staged_selected_input.update(|s| *s = (*s + 1).min(count.saturating_sub(1)));
                } else if key.up_arrow || input == "k" {
                    staged_selected_input.update(|s| *s = s.saturating_sub(1));
                } else if input == " " || key.return_key {
                    // Unstage file
                    let idx = staged_selected_input.get();
                    if idx < staged_files_input.get().len() {
                        let file = staged_files_input.get()[idx].clone();
                        unstaged_files_input.update(|f| f.push(file));
                        staged_files_input.update(|f| {
                            f.remove(idx);
                        });
                        if idx > 0 && idx >= staged_files_input.get().len() {
                            staged_selected_input.update(|s| *s = s.saturating_sub(1));
                        }
                    }
                }
            }
            Panel::Commits => {
                let count = commits_input.get().len();
                if key.down_arrow || input == "j" {
                    commits_selected_input.update(|s| *s = (*s + 1).min(count.saturating_sub(1)));
                } else if key.up_arrow || input == "k" {
                    commits_selected_input.update(|s| *s = s.saturating_sub(1));
                }
            }
            Panel::Diff => {}
        }
    });

    // Get current diff content
    let diff_content = get_diff_content(
        active_panel.get(),
        &unstaged_files.get(),
        status_selected.get(),
        &staged_files.get(),
        staged_selected.get(),
        &commits.get(),
        commits_selected.get(),
    );

    Box::new()
        .flex_direction(FlexDirection::Column)
        .children(vec![
            header(),
            Box::new()
                .flex_direction(FlexDirection::Row)
                .flex_grow(1.0)
                .children(vec![
                    // Left panels
                    Box::new()
                        .flex_direction(FlexDirection::Column)
                        .width(40)
                        .children(vec![
                            status_panel(
                                &unstaged_files.get(),
                                status_selected.get(),
                                active_panel.get() == Panel::Status,
                            ),
                            staged_panel(
                                &staged_files.get(),
                                staged_selected.get(),
                                active_panel.get() == Panel::Staged,
                            ),
                            commits_panel(
                                &commits.get(),
                                commits_selected.get(),
                                active_panel.get() == Panel::Commits,
                            ),
                        ])
                        .into_element(),
                    // Right panel - diff view
                    diff_panel(&diff_content, active_panel.get() == Panel::Diff),
                ])
                .into_element(),
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
            Text::new("rnk-git")
                .color(Color::Magenta)
                .bold()
                .into_element(),
        )
        .child(
            Text::new("main")
                .color(Color::Cyan)
                .into_element(),
        )
        .into_element()
}

fn status_panel(files: &[FileStatus], selected: usize, active: bool) -> Element {
    let border_color = if active { Color::Green } else { Color::BrightBlack };

    let mut children = vec![
        Text::new(format!(" Unstaged Changes ({}) ", files.len()))
            .color(if active { Color::Green } else { Color::White })
            .bold()
            .into_element(),
    ];

    for (i, file) in files.iter().take(5).enumerate() {
        let is_selected = i == selected;
        let (icon, color) = match file.status {
            FileState::Modified => ("M", Color::Yellow),
            FileState::Added => ("A", Color::Green),
            FileState::Deleted => ("D", Color::Red),
            FileState::Untracked => ("?", Color::BrightBlack),
            FileState::Renamed => ("R", Color::Cyan),
        };

        children.push(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .background(if is_selected && active {
                    Color::Ansi256(238)
                } else {
                    Color::Reset
                })
                .children(vec![
                    Text::new(format!(" {} ", icon)).color(color).into_element(),
                    Text::new(&file.name)
                        .color(if is_selected && active {
                            Color::White
                        } else {
                            Color::Reset
                        })
                        .into_element(),
                ])
                .into_element(),
        );
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .border_style(BorderStyle::Round)
        .border_color(border_color)
        .flex_grow(1.0)
        .children(children)
        .into_element()
}

fn staged_panel(files: &[FileStatus], selected: usize, active: bool) -> Element {
    let border_color = if active { Color::Green } else { Color::BrightBlack };

    let mut children = vec![
        Text::new(format!(" Staged Changes ({}) ", files.len()))
            .color(if active { Color::Green } else { Color::White })
            .bold()
            .into_element(),
    ];

    for (i, file) in files.iter().take(5).enumerate() {
        let is_selected = i == selected;
        let (icon, color) = match file.status {
            FileState::Modified => ("M", Color::Yellow),
            FileState::Added => ("A", Color::Green),
            FileState::Deleted => ("D", Color::Red),
            FileState::Untracked => ("?", Color::BrightBlack),
            FileState::Renamed => ("R", Color::Cyan),
        };

        children.push(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .background(if is_selected && active {
                    Color::Ansi256(238)
                } else {
                    Color::Reset
                })
                .children(vec![
                    Text::new(format!(" {} ", icon)).color(color).into_element(),
                    Text::new(&file.name)
                        .color(if is_selected && active {
                            Color::White
                        } else {
                            Color::Reset
                        })
                        .into_element(),
                ])
                .into_element(),
        );
    }

    if files.is_empty() {
        children.push(Text::new(" (no staged files)").dim().into_element());
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .border_style(BorderStyle::Round)
        .border_color(border_color)
        .flex_grow(1.0)
        .children(children)
        .into_element()
}

fn commits_panel(commits: &[Commit], selected: usize, active: bool) -> Element {
    let border_color = if active { Color::Green } else { Color::BrightBlack };

    let mut children = vec![
        Text::new(" Commits ")
            .color(if active { Color::Green } else { Color::White })
            .bold()
            .into_element(),
    ];

    for (i, commit) in commits.iter().take(6).enumerate() {
        let is_selected = i == selected;

        children.push(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .background(if is_selected && active {
                    Color::Ansi256(238)
                } else {
                    Color::Reset
                })
                .children(vec![
                    Text::new(format!(" {} ", &commit.hash[..7]))
                        .color(Color::Yellow)
                        .into_element(),
                    Text::new(&commit.message)
                        .color(if is_selected && active {
                            Color::White
                        } else {
                            Color::Reset
                        })
                        .into_element(),
                ])
                .into_element(),
        );
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .border_style(BorderStyle::Round)
        .border_color(border_color)
        .flex_grow(1.0)
        .children(children)
        .into_element()
}

fn diff_panel(content: &str, active: bool) -> Element {
    let border_color = if active { Color::Green } else { Color::BrightBlack };

    let mut children = vec![
        Text::new(" Diff ")
            .color(Color::White)
            .bold()
            .into_element(),
    ];

    for line in content.lines().take(20) {
        let color = if line.starts_with('+') && !line.starts_with("+++") {
            Color::Green
        } else if line.starts_with('-') && !line.starts_with("---") {
            Color::Red
        } else if line.starts_with("@@") {
            Color::Cyan
        } else if line.starts_with("diff") || line.starts_with("index") {
            Color::Yellow
        } else {
            Color::Reset
        };

        children.push(
            Text::new(format!(" {}", line))
                .color(color)
                .into_element(),
        );
    }

    Box::new()
        .flex_direction(FlexDirection::Column)
        .border_style(BorderStyle::Round)
        .border_color(border_color)
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
            Text::new("Tab").color(Color::Yellow).bold().into_element(),
            Text::new("Switch").dim().into_element(),
            Text::new("Space").color(Color::Yellow).bold().into_element(),
            Text::new("Stage/Unstage").dim().into_element(),
            Text::new("↑↓").color(Color::Yellow).bold().into_element(),
            Text::new("Navigate").dim().into_element(),
            Text::new("1-3").color(Color::Yellow).bold().into_element(),
            Text::new("Panels").dim().into_element(),
        ])
        .into_element()
}

fn get_diff_content(
    panel: Panel,
    unstaged: &[FileStatus],
    unstaged_idx: usize,
    staged: &[FileStatus],
    staged_idx: usize,
    commits: &[Commit],
    commits_idx: usize,
) -> String {
    match panel {
        Panel::Status => {
            if let Some(file) = unstaged.get(unstaged_idx) {
                mock_diff(&file.name)
            } else {
                "No file selected".to_string()
            }
        }
        Panel::Staged => {
            if let Some(file) = staged.get(staged_idx) {
                mock_diff(&file.name)
            } else {
                "No file selected".to_string()
            }
        }
        Panel::Commits => {
            if let Some(commit) = commits.get(commits_idx) {
                mock_commit_diff(&commit.hash)
            } else {
                "No commit selected".to_string()
            }
        }
        Panel::Diff => "".to_string(),
    }
}

fn mock_unstaged_files() -> Vec<FileStatus> {
    vec![
        FileStatus {
            name: "src/main.rs".to_string(),
            status: FileState::Modified,
        },
        FileStatus {
            name: "src/lib.rs".to_string(),
            status: FileState::Modified,
        },
        FileStatus {
            name: "Cargo.toml".to_string(),
            status: FileState::Modified,
        },
        FileStatus {
            name: "README.md".to_string(),
            status: FileState::Modified,
        },
        FileStatus {
            name: "new_file.rs".to_string(),
            status: FileState::Untracked,
        },
    ]
}

fn mock_staged_files() -> Vec<FileStatus> {
    vec![
        FileStatus {
            name: "src/utils.rs".to_string(),
            status: FileState::Added,
        },
    ]
}

fn mock_commits() -> Vec<Commit> {
    vec![
        Commit {
            hash: "a1b2c3d4e5f6".to_string(),
            message: "feat: add new feature".to_string(),
            author: "dev".to_string(),
            time: "2h ago".to_string(),
        },
        Commit {
            hash: "b2c3d4e5f6a1".to_string(),
            message: "fix: resolve bug".to_string(),
            author: "dev".to_string(),
            time: "5h ago".to_string(),
        },
        Commit {
            hash: "c3d4e5f6a1b2".to_string(),
            message: "docs: update README".to_string(),
            author: "dev".to_string(),
            time: "1d ago".to_string(),
        },
        Commit {
            hash: "d4e5f6a1b2c3".to_string(),
            message: "refactor: clean up code".to_string(),
            author: "dev".to_string(),
            time: "2d ago".to_string(),
        },
        Commit {
            hash: "e5f6a1b2c3d4".to_string(),
            message: "test: add unit tests".to_string(),
            author: "dev".to_string(),
            time: "3d ago".to_string(),
        },
        Commit {
            hash: "f6a1b2c3d4e5".to_string(),
            message: "chore: bump version".to_string(),
            author: "dev".to_string(),
            time: "4d ago".to_string(),
        },
    ]
}

fn mock_diff(filename: &str) -> String {
    format!(
        r#"diff --git a/{0} b/{0}
index 1234567..abcdefg 100644
--- a/{0}
+++ b/{0}
@@ -10,6 +10,8 @@ fn main() {{
     let app = App::new();
+    // New feature added
+    app.enable_feature();
     app.run();
-    // Old comment removed
 }}
"#,
        filename
    )
}

fn mock_commit_diff(hash: &str) -> String {
    format!(
        r#"commit {}
Author: dev <dev@example.com>
Date:   Mon Jan 1 12:00:00 2024

    feat: add new feature

diff --git a/src/main.rs b/src/main.rs
@@ -1,5 +1,7 @@
 fn main() {{
+    println!("Hello, world!");
     run();
 }}
"#,
        hash
    )
}
