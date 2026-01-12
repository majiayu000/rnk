//! Todo App Example - Demonstrates tink's capabilities
//!
//! Run with: cargo run --example todo

use rnk::prelude::*;

fn main() {
    // Create todo items state
    let todos = use_signal(|| {
        vec![
            ("Learn Rust".to_string(), false),
            ("Build tink framework".to_string(), true),
            ("Create todo app".to_string(), false),
        ]
    });

    // Selected index state
    let selected = use_signal(|| 0usize);

    // Input mode state (normal or adding)
    let adding = use_signal(|| false);

    // New todo text buffer
    let input_buffer = use_signal(|| String::new());

    // Handle keyboard input
    use_input({
        let todos = todos.clone();
        let selected = selected.clone();
        let adding = adding.clone();
        let input_buffer = input_buffer.clone();

        move |input: &str, key: &Key| {
            if adding.get() {
                // Adding mode - capture text input
                if key.return_key {
                    let text = input_buffer.get();
                    if !text.is_empty() {
                        todos.update(|t| t.push((text, false)));
                        input_buffer.set(String::new());
                    }
                    adding.set(false);
                } else if key.escape {
                    input_buffer.set(String::new());
                    adding.set(false);
                } else if key.backspace {
                    input_buffer.update(|s| {
                        s.pop();
                    });
                } else if !input.is_empty() && !key.ctrl && !key.alt {
                    // Regular character input
                    input_buffer.update(|s| s.push_str(input));
                }
            } else {
                // Normal mode - navigate and toggle
                if key.up_arrow || input == "k" {
                    let current = selected.get();
                    if current > 0 {
                        selected.set(current - 1);
                    }
                } else if key.down_arrow || input == "j" {
                    let current = selected.get();
                    let len = todos.get().len();
                    if current < len.saturating_sub(1) {
                        selected.set(current + 1);
                    }
                } else if key.return_key || input == " " {
                    let idx = selected.get();
                    todos.update(|t| {
                        if let Some(item) = t.get_mut(idx) {
                            item.1 = !item.1;
                        }
                    });
                } else if input == "a" {
                    adding.set(true);
                } else if input == "d" {
                    let idx = selected.get();
                    let len = todos.get().len();
                    if len > 0 {
                        todos.update(|t| {
                            if idx < t.len() {
                                t.remove(idx);
                            }
                        });
                        // Adjust selection if needed
                        if idx >= len.saturating_sub(1) && idx > 0 {
                            selected.set(idx - 1);
                        }
                    }
                } else if input == "q" || key.escape {
                    std::process::exit(0);
                }
            }
        }
    });

    // Render with a component function
    let _ = render({
        let todos = todos.clone();
        let selected = selected.clone();
        let adding = adding.clone();
        let input_buffer = input_buffer.clone();
        move || build_ui(&todos, &selected, &adding, &input_buffer)
    });
}

fn build_ui(
    todos: &Signal<Vec<(String, bool)>>,
    selected: &Signal<usize>,
    adding: &Signal<bool>,
    input_buffer: &Signal<String>,
) -> Element {
    let todo_list = todos.get();
    let selected_idx = selected.get();
    let is_adding = adding.get();

    // Header
    let header = Box::new()
        .border_style(BorderStyle::Round)
        .padding(1.0)
        .child(
            Text::new("Todo App")
                .bold()
                .color(Color::Cyan)
                .into_element(),
        )
        .into_element();

    // Todo items
    let mut items_box = Box::new()
        .flex_direction(FlexDirection::Column)
        .padding_x(1.0);

    for (idx, (text, done)) in todo_list.iter().enumerate() {
        let is_selected = idx == selected_idx;

        // Checkbox
        let checkbox = if *done { "[x]" } else { "[ ]" };

        // Build item text
        let item_text = format!("{} {}", checkbox, text);

        let mut text_component = Text::new(&item_text);

        if is_selected {
            text_component = text_component.color(Color::Yellow).bold();
        } else if *done {
            text_component = text_component.color(Color::BrightBlack).dim();
        }

        items_box = items_box.child(text_component.into_element());
    }

    if todo_list.is_empty() {
        items_box = items_box.child(
            Text::new("No todos yet. Press 'a' to add one!")
                .color(Color::BrightBlack)
                .italic()
                .into_element(),
        );
    }

    // Input area (when adding)
    let input_area = if is_adding {
        Box::new()
            .border_style(BorderStyle::Single)
            .padding(1.0)
            .margin_top(1.0)
            .child(
                Text::new(&format!("New todo: {}_", input_buffer.get()))
                    .color(Color::Green)
                    .into_element(),
            )
            .into_element()
    } else {
        Newline::new().into_element()
    };

    // Stats
    let completed = todo_list.iter().filter(|(_, done)| *done).count();
    let total = todo_list.len();
    let stats_text = format!("Completed: {}/{}", completed, total);

    let stats = Box::new()
        .margin_top(1.0)
        .padding_x(1.0)
        .child(
            Text::new(&stats_text)
                .color(Color::BrightBlack)
                .into_element(),
        )
        .into_element();

    // Help text
    let help_text = if is_adding {
        "Enter: Save | Esc: Cancel"
    } else {
        "Up/Down: Navigate | Enter/Space: Toggle | a: Add | d: Delete | q: Quit"
    };

    let help = Box::new()
        .margin_top(1.0)
        .padding_x(1.0)
        .child(
            Text::new(help_text)
                .color(Color::BrightBlack)
                .dim()
                .into_element(),
        )
        .into_element();

    // Main container
    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1.0)
        .child(header)
        .child(items_box.into_element())
        .child(input_area)
        .child(stats)
        .child(help)
        .into_element()
}
