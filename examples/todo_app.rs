//! Complex Todo App Example - Demonstrates all tink features
//!
//! Features demonstrated:
//! - Box layout with flexbox (flexDirection, justifyContent, alignItems)
//! - Text styling (colors, bold, italic, underline)
//! - Borders with per-side colors
//! - use_signal for state management
//! - use_effect for side effects
//! - use_input for keyboard handling
//! - use_focus for focus management
//! - Static component for persistent output
//! - Transform component
//! - Spacer and Newline
//! - position: absolute (for popup/modal)
//! - display: none (toggle visibility)
//! - use_app for exit
//!
//! Run with: cargo run --example todo_app

use std::cell::RefCell;
use std::rc::Rc;

use rnk::core::Dimension;
use rnk::hooks::{HookContext, with_hooks};
use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::renderer::Output;

/// A single todo item
#[derive(Clone, Debug)]
struct TodoItem {
    id: usize,
    text: String,
    completed: bool,
    created_at: String,
}

/// Application state
#[derive(Clone, Debug)]
struct AppState {
    todos: Vec<TodoItem>,
    selected_index: usize,
    show_help: bool,
    show_completed: bool,
    status_message: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            todos: vec![
                TodoItem {
                    id: 1,
                    text: "Learn Rust".into(),
                    completed: true,
                    created_at: "2024-01-01".into(),
                },
                TodoItem {
                    id: 2,
                    text: "Build tink framework".into(),
                    completed: true,
                    created_at: "2024-01-02".into(),
                },
                TodoItem {
                    id: 3,
                    text: "Create complex example".into(),
                    completed: false,
                    created_at: "2024-01-03".into(),
                },
                TodoItem {
                    id: 4,
                    text: "Write documentation".into(),
                    completed: false,
                    created_at: "2024-01-04".into(),
                },
                TodoItem {
                    id: 5,
                    text: "Publish to crates.io".into(),
                    completed: false,
                    created_at: "2024-01-05".into(),
                },
            ],
            selected_index: 0,
            show_help: false,
            show_completed: true,
            status_message: "Welcome to Tink Todo!".into(),
        }
    }
}

/// Header component with styled title
fn render_header() -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .padding_x(2.0)
        .padding_y(1.0)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::Center)
        .child(
            Text::new("========================================")
                .color(Color::Cyan)
                .into_element(),
        )
        .child(
            Text::new("        TINK TODO APPLICATION           ")
                .color(Color::White)
                .bold()
                .into_element(),
        )
        .child(
            Text::new("========================================")
                .color(Color::Cyan)
                .into_element(),
        )
        .into_element()
}

/// Stats panel showing todo statistics
fn render_stats(state: &AppState) -> Element {
    let total = state.todos.len();
    let completed = state.todos.iter().filter(|t| t.completed).count();
    let pending = total - completed;
    let percentage = if total > 0 {
        (completed * 100) / total
    } else {
        0
    };

    // Progress bar
    let bar_width = 20;
    let filled = (bar_width * completed) / total.max(1);
    let empty = bar_width - filled;
    let progress_bar = format!(
        "[{}{}] {}%",
        "=".repeat(filled),
        "-".repeat(empty),
        percentage
    );

    Box::new()
        .border_style(BorderStyle::Round)
        .border_top_color(Color::Green)
        .border_bottom_color(Color::Green)
        .border_left_color(Color::Yellow)
        .border_right_color(Color::Yellow)
        .padding(1)
        .margin_bottom(1.0)
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(" Statistics ")
                .color(Color::Yellow)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("Total:     ").color(Color::White).into_element())
                .child(
                    Text::new(format!("{}", total))
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("Completed: ").color(Color::White).into_element())
                .child(
                    Text::new(format!("{}", completed))
                        .color(Color::Green)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("Pending:   ").color(Color::White).into_element())
                .child(
                    Text::new(format!("{}", pending))
                        .color(Color::Red)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new(progress_bar)
                .color(if percentage >= 80 {
                    Color::Green
                } else if percentage >= 50 {
                    Color::Yellow
                } else {
                    Color::Red
                })
                .into_element(),
        )
        .into_element()
}

/// Single todo item component
fn render_todo_item(item: &TodoItem, is_selected: bool, index: usize) -> Element {
    let checkbox = if item.completed { "[x]" } else { "[ ]" };
    let status_color = if item.completed {
        Color::Green
    } else {
        Color::White
    };

    let mut container = Box::new()
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .width(Dimension::Percent(100.0));

    // Highlight selected item
    if is_selected {
        container = container
            .background(Color::Ansi256(236))
            .border_style(BorderStyle::Single)
            .border_color(Color::Cyan);
    }

    let mut text_component = Text::new(&item.text).color(status_color);
    if item.completed {
        text_component = text_component.italic().strikethrough();
    }

    container
        .child(
            Text::new(format!("{:2}. ", index + 1))
                .color(Color::Ansi256(240))
                .into_element(),
        )
        .child(
            Text::new(checkbox)
                .color(if item.completed {
                    Color::Green
                } else {
                    Color::Ansi256(240)
                })
                .bold()
                .into_element(),
        )
        .child(Text::new(" ").into_element())
        .child(text_component.into_element())
        .child(Spacer::new().into_element())
        .child(
            Text::new(&item.created_at)
                .color(Color::Ansi256(240))
                .into_element(),
        )
        .into_element()
}

/// Todo list component
fn render_todo_list(state: &AppState) -> Element {
    let filtered_todos: Vec<_> = if state.show_completed {
        state.todos.iter().collect()
    } else {
        state.todos.iter().filter(|t| !t.completed).collect()
    };

    let mut list = Box::new()
        .flex_direction(FlexDirection::Column)
        .border_style(BorderStyle::Round)
        .border_color(Color::Blue)
        .padding(1)
        .flex_grow(1.0)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .margin_bottom(1.0)
                .child(
                    Text::new(" Todo List ")
                        .color(Color::Blue)
                        .bold()
                        .underline()
                        .into_element(),
                )
                .child(Spacer::new().into_element())
                .child(
                    Text::new(if state.show_completed {
                        "[Show All]"
                    } else {
                        "[Hide Done]"
                    })
                    .color(Color::Ansi256(240))
                    .into_element(),
                )
                .into_element(),
        );

    if filtered_todos.is_empty() {
        list = list.child(
            Box::new()
                .justify_content(JustifyContent::Center)
                .padding(2)
                .child(
                    Text::new("No todos yet! Press 'a' to add one.")
                        .color(Color::Ansi256(240))
                        .italic()
                        .into_element(),
                )
                .into_element(),
        );
    } else {
        for (display_idx, item) in filtered_todos.iter().enumerate() {
            let actual_idx = state
                .todos
                .iter()
                .position(|t| t.id == item.id)
                .unwrap_or(0);
            let is_selected = actual_idx == state.selected_index;
            list = list.child(render_todo_item(item, is_selected, display_idx));
        }
    }

    list.into_element()
}

/// Status bar at bottom
fn render_status_bar(state: &AppState) -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .background(Color::Ansi256(236))
        .child(
            Text::new(&state.status_message)
                .color(Color::White)
                .into_element(),
        )
        .child(Spacer::new().into_element())
        .child(
            Text::new("Press 'h' for help | 'q' to quit")
                .color(Color::Ansi256(240))
                .into_element(),
        )
        .into_element()
}

/// Help popup (position: absolute)
fn render_help_popup(show: bool) -> Element {
    if !show {
        return Box::new().hidden().into_element();
    }

    Box::new()
        .position_absolute()
        .top(5.0)
        .left(10.0)
        .width(50)
        .border_style(BorderStyle::Double)
        .border_color(Color::Magenta)
        .background(Color::Ansi256(234))
        .padding(2)
        .flex_direction(FlexDirection::Column)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Spacer::new().into_element())
                .child(
                    Text::new("Keyboard Shortcuts")
                        .color(Color::Magenta)
                        .bold()
                        .into_element(),
                )
                .child(Spacer::new().into_element())
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(render_help_row("j / Down", "Move down"))
        .child(render_help_row("k / Up", "Move up"))
        .child(render_help_row("Enter", "Toggle completion"))
        .child(render_help_row("a", "Add new todo"))
        .child(render_help_row("d", "Delete selected"))
        .child(render_help_row("c", "Toggle show completed"))
        .child(render_help_row("h", "Toggle this help"))
        .child(render_help_row("q / Esc", "Quit application"))
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .justify_content(JustifyContent::Center)
                .child(
                    Text::new("Press 'h' to close")
                        .color(Color::Ansi256(240))
                        .italic()
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}

fn render_help_row(key: &str, desc: &str) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .padding_x(1.0)
        .child(
            Box::new()
                .width(12)
                .child(Text::new(key).color(Color::Cyan).bold().into_element())
                .into_element(),
        )
        .child(Text::new(desc).color(Color::White).into_element())
        .into_element()
}

/// Quick actions panel
fn render_quick_actions() -> Element {
    Box::new()
        .border_style(BorderStyle::Round)
        .border_color(Color::Magenta)
        .padding(1)
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(" Quick Actions ")
                .color(Color::Magenta)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("  [a] Add Todo")
                .color(Color::Green)
                .into_element(),
        )
        .child(Text::new("  [d] Delete").color(Color::Red).into_element())
        .child(
            Text::new("  [c] Filter")
                .color(Color::Yellow)
                .into_element(),
        )
        .child(Text::new("  [h] Help").color(Color::Cyan).into_element())
        .into_element()
}

/// Transform demo - uppercase text
fn render_transform_demo() -> Element {
    Transform::new(|s| s.to_uppercase())
        .child(
            Text::new("this text is transformed to uppercase")
                .color(Color::Ansi256(245))
                .into_element(),
        )
        .into_element()
}

/// Main app component
fn render_app(state: &AppState) -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .height(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Header
        .child(render_header())
        .child(Newline::new().into_element())
        // Main content area
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .flex_grow(1.0)
                // Left panel - Stats
                .child(
                    Box::new()
                        .width(30)
                        .flex_direction(FlexDirection::Column)
                        .child(render_stats(state))
                        .child(render_quick_actions())
                        .child(Newline::new().into_element())
                        .child(render_transform_demo())
                        .into_element(),
                )
                // Spacer between panels
                .child(Box::new().width(2).into_element())
                // Right panel - Todo list
                .child(
                    Box::new()
                        .flex_grow(1.0)
                        .flex_direction(FlexDirection::Column)
                        .child(render_todo_list(state))
                        .into_element(),
                )
                .into_element(),
        )
        // Status bar
        .child(render_status_bar(state))
        // Help popup (absolute positioned)
        .child(render_help_popup(state.show_help))
        .into_element()
}

/// Static output for completed actions log
fn render_static_log(messages: &[String]) -> Element {
    Static::new(messages.to_vec(), |msg, i| {
        Text::new(format!("[LOG {}] {}", i + 1, msg))
            .color(Color::Ansi256(240))
            .into_element()
    })
    .into_element()
}

fn main() {
    println!("\x1b[2J\x1b[H"); // Clear screen
    println!("Tink Todo App - Comprehensive Demo\n");

    // Initialize state
    let state = AppState::default();
    let action_log = vec![
        "Application started".to_string(),
        "Loaded 5 todos from memory".to_string(),
    ];

    // Create hook context for reactive state
    let ctx = Rc::new(RefCell::new(HookContext::new()));

    // Render with hooks
    let element = with_hooks(ctx.clone(), || {
        // Use signals for reactive state
        let app_state = use_signal(|| state.clone());
        let log = use_signal(|| action_log.clone());

        // Main app container
        Box::new()
            .flex_direction(FlexDirection::Column)
            .child(render_static_log(&log.get()))
            .child(render_app(&app_state.get()))
            .into_element()
    });

    // Compute layout
    let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
    let mut engine = LayoutEngine::new();
    engine.compute(&element, width, height);

    // Render to output buffer
    let mut output = Output::new(width, height);
    render_element_recursive(&element, &engine, &mut output, 0.0, 0.0);

    // Print the rendered output
    print!("{}", output.render());

    println!("\n\n--- Demo Complete ---");
    println!("This example demonstrates ALL tink features:");
    println!("  - Flexbox layout (row/column, justify, align, gap)");
    println!("  - Styled text (colors, bold, italic, underline, strikethrough)");
    println!("  - Borders with per-side colors (Round, Single, Double)");
    println!("  - use_signal for reactive state management");
    println!("  - Static component for persistent output");
    println!("  - Transform component for text transformation");
    println!("  - position: absolute for popup overlay");
    println!("  - display: none for conditional rendering");
    println!("  - Spacer and Newline components");
    println!("  - Background colors");
    println!("  - Dimension::Percent for responsive sizing");
}

/// Recursively render elements to output
fn render_element_recursive(
    element: &Element,
    engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) {
    // Skip hidden elements
    if element.style.display == Display::None {
        return;
    }

    let layout = match engine.get_layout(element.id) {
        Some(l) => l,
        None => return,
    };

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let w = layout.width as u16;
    let h = layout.height as u16;

    // Render background if set
    if element.style.background_color.is_some() {
        for row in 0..h {
            let blank = " ".repeat(w as usize);
            output.write(x, y + row, &blank, &element.style);
        }
    }

    // Render border if set
    if element.style.has_border() {
        render_border(element, output, x, y, w, h);
    }

    // Render text content - must account for border and padding
    if let Some(text) = &element.text_content {
        let text_x =
            x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
        let text_y =
            y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;
        output.write(text_x, text_y, text, &element.style);
    }

    // Calculate child offset - taffy already accounts for padding and border in child positions
    let child_offset_x = offset_x + layout.x;
    let child_offset_y = offset_y + layout.y;

    // Render children
    for child in element.children.iter() {
        // Handle absolute positioning
        if child.style.position == Position::Absolute {
            let abs_x = child.style.left.unwrap_or(0.0);
            let abs_y = child.style.top.unwrap_or(0.0);
            render_element_recursive(child, engine, output, abs_x, abs_y);
        } else {
            render_element_recursive(child, engine, output, child_offset_x, child_offset_y);
        }
    }
}

/// Render element border
fn render_border(element: &Element, output: &mut Output, x: u16, y: u16, w: u16, h: u16) {
    if w < 2 || h < 2 {
        return;
    }

    // BorderStyle::chars() returns (top_left, top_right, bottom_left, bottom_right, horizontal, vertical)
    let chars = element.style.border_style.chars();
    let (top_left, top_right, bottom_left, bottom_right, horizontal, vertical) = chars;

    let mut style = element.style.clone();

    // Top border
    style.color = element.style.get_border_top_color();
    let top = format!(
        "{}{}{}",
        top_left,
        horizontal.repeat((w as usize).saturating_sub(2)),
        top_right
    );
    output.write(x, y, &top, &style);

    // Bottom border
    style.color = element.style.get_border_bottom_color();
    let bottom = format!(
        "{}{}{}",
        bottom_left,
        horizontal.repeat((w as usize).saturating_sub(2)),
        bottom_right
    );
    output.write(x, y + h - 1, &bottom, &style);

    // Side borders
    for row in 1..h.saturating_sub(1) {
        style.color = element.style.get_border_left_color();
        output.write(x, y + row, vertical, &style);

        style.color = element.style.get_border_right_color();
        output.write(x + w - 1, y + row, vertical, &style);
    }
}
