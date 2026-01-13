# rnk

A React-like declarative terminal UI framework for Rust, inspired by [Ink](https://github.com/vadimdemedes/ink) and [Bubbletea](https://github.com/charmbracelet/bubbletea).

[![Crates.io](https://img.shields.io/crates/v/rnk.svg)](https://crates.io/crates/rnk)
[![Documentation](https://docs.rs/rnk/badge.svg)](https://docs.rs/rnk)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- **React-like API**: Familiar component model with hooks (`use_signal`, `use_effect`, `use_input`)
- **Declarative UI**: Build TUIs with composable components
- **Flexbox Layout**: Powered by [Taffy](https://github.com/DioxusLabs/taffy) for flexible layouts
- **Inline Mode** (default): Output persists in terminal history (like Ink/Bubbletea)
- **Fullscreen Mode**: Uses alternate screen buffer (like vim)
- **Line-level Diff Rendering**: Only changed lines are redrawn for efficiency
- **Persistent Output**: `println()` API for messages that persist above the UI
- **Cross-thread Rendering**: `request_render()` for async/multi-threaded apps
- **Rich Components**: Box, Text, List, Table, Tabs, Progress, Sparkline, BarChart, and more
- **Mouse Support**: Full mouse event handling
- **Cross-platform**: Works on Linux, macOS, and Windows

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rnk = "0.6"
```

## Examples

### Hello World

```rust
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    Box::new()
        .padding(1)
        .border_style(BorderStyle::Round)
        .child(Text::new("Hello, rnk!").color(Color::Green).bold().into_element())
        .into_element()
}
```

### Counter with Keyboard Input

```rust
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let count = use_signal(|| 0);
    let app = use_app();

    use_input(move |event| {
        match event.code {
            KeyCode::Up => count.set(count.get() + 1),
            KeyCode::Down => count.set(count.get() - 1),
            KeyCode::Char('q') => app.exit(),
            _ => {}
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .child(Text::new(format!("Count: {}", count.get())).bold().into_element())
        .child(Text::new("↑/↓ to change, q to quit").dim().into_element())
        .into_element()
}
```

### Streaming Output Demo

```rust
use rnk::prelude::*;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Background thread for periodic updates
    std::thread::spawn(|| {
        let mut tick = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(100));
            tick += 1;
            rnk::request_render();

            // Print persistent log every 20 ticks
            if tick % 20 == 0 {
                rnk::println(format!("[LOG] Tick {} completed", tick));
            }
        }
    });

    render(app).run()
}

fn app() -> Element {
    let counter = use_signal(|| 0);
    counter.set(counter.get() + 1);

    Box::new()
        .child(Text::new(format!("Frame: {}", counter.get())).into_element())
        .into_element()
}
```

## Render Modes

### Inline Mode (Default)

Output appears at current cursor position and persists in terminal history.

```rust
render(app).run()?;           // Inline mode (default)
render(app).inline().run()?;  // Explicit inline mode
```

### Fullscreen Mode

Uses alternate screen buffer. Content is cleared on exit.

```rust
render(app).fullscreen().run()?;
```

### Configuration Options

```rust
render(app)
    .fullscreen()           // Use alternate screen
    .fps(30)                // Target 30 FPS (default: 60)
    .exit_on_ctrl_c(false)  // Handle Ctrl+C manually
    .run()?;
```

### Runtime Mode Switching

Switch between modes at runtime:

```rust
let app = use_app();

use_input(move |event| {
    if event.code == KeyCode::Char(' ') {
        if rnk::is_alt_screen().unwrap_or(false) {
            rnk::exit_alt_screen();  // Switch to inline
        } else {
            rnk::enter_alt_screen(); // Switch to fullscreen
        }
    }
});
```

## Render APIs

### Interactive Applications

```rust
// Run interactive TUI application
render(app).run()?;
```

### Static Rendering (Non-interactive)

Render elements to string without running the event loop:

```rust
use rnk::prelude::*;

let element = Box::new()
    .border_style(BorderStyle::Round)
    .child(Text::new("Hello!").into_element())
    .into_element();

// Render with specific width
let output = rnk::render_to_string(&element, 80);
println!("{}", output);

// Render with auto-detected terminal width
let output = rnk::render_to_string_auto(&element);
println!("{}", output);
```

## Components

### Box

Flexbox container with full layout support.

```rust
Box::new()
    .flex_direction(FlexDirection::Column)
    .justify_content(JustifyContent::Center)
    .align_items(AlignItems::Center)
    .padding(1)
    .margin(1.0)
    .width(50)
    .height(10)
    .border_style(BorderStyle::Round)
    .border_color(Color::Cyan)
    .background(Color::Ansi256(236))
    .child(/* ... */)
    .into_element()
```

**Border Styles**: `None`, `Single`, `Double`, `Round`, `Bold`, `Custom(chars)`

**Per-side Border Colors**:
```rust
Box::new()
    .border_style(BorderStyle::Single)
    .border_top_color(Color::Red)
    .border_bottom_color(Color::Blue)
    .border_left_color(Color::Green)
    .border_right_color(Color::Yellow)
```

### Text

Styled text with colors and formatting.

```rust
Text::new("Hello, World!")
    .color(Color::Green)
    .background_color(Color::Black)
    .bold()
    .italic()
    .underline()
    .strikethrough()
    .dim()
    .into_element()
```

**Rich Text with Spans**:
```rust
Text::builder()
    .span("Normal ")
    .span_styled("bold", |s| s.bold())
    .span(" and ")
    .span_styled("colored", |s| s.color(Color::Cyan))
    .build()
    .into_element()
```

### List

Selectable list with keyboard navigation.

```rust
List::new()
    .items(vec!["Item 1", "Item 2", "Item 3"])
    .selected(current_index)
    .highlight_style(|s| s.color(Color::Cyan).bold())
    .on_select(|idx| { /* handle selection */ })
    .into_element()
```

### Table

Data table with headers and styling.

```rust
Table::new()
    .headers(vec!["Name", "Age", "City"])
    .rows(vec![
        vec!["Alice", "30", "NYC"],
        vec!["Bob", "25", "LA"],
    ])
    .column_widths(vec![20, 10, 15])
    .header_style(|s| s.bold().color(Color::Yellow))
    .into_element()
```

### Tabs

Tab navigation component.

```rust
Tabs::new()
    .tabs(vec!["Home", "Settings", "About"])
    .selected(current_tab)
    .on_change(|idx| { /* handle tab change */ })
    .into_element()
```

### Progress / Gauge

Progress bars and gauges.

```rust
// Simple progress bar
Progress::new()
    .progress(0.75)  // 75%
    .width(30)
    .filled_char('█')
    .empty_char('░')
    .into_element()

// Gauge with label
Gauge::new()
    .ratio(0.5)
    .label("50%")
    .into_element()
```

### Sparkline

Inline data visualization.

```rust
Sparkline::new()
    .data(&[1, 3, 7, 2, 5, 8, 4])
    .width(20)
    .into_element()
```

### BarChart

Horizontal and vertical bar charts.

```rust
BarChart::new()
    .data(&[("A", 10), ("B", 20), ("C", 15)])
    .bar_width(3)
    .bar_gap(1)
    .into_element()
```

### Static

Permanent output that persists above dynamic UI.

```rust
Static::new(
    items.to_vec(),
    |item, index| {
        Text::new(format!("[{}] {}", index + 1, item))
            .color(Color::Gray)
            .into_element()
    }
).into_element()
```

### Transform

Transform child text content.

```rust
Transform::new(|s| s.to_uppercase())
    .child(Text::new("will be uppercase").into_element())
    .into_element()
```

### Spacer / Newline

Layout helpers.

```rust
Box::new()
    .flex_direction(FlexDirection::Row)
    .child(Text::new("Left").into_element())
    .child(Spacer::new().into_element())  // Flexible space
    .child(Text::new("Right").into_element())
    .into_element()

// Add vertical space
Newline::new().into_element()
```

### Spinner

Animated loading indicator.

```rust
Spinner::new()
    .style(SpinnerStyle::Dots)
    .label("Loading...")
    .into_element()
```

### Message

Styled message boxes for info, success, warning, error.

```rust
Message::info("Information message")
Message::success("Operation completed!")
Message::warning("Please be careful")
Message::error("Something went wrong")
```

## Hooks

### use_signal

Reactive state management.

```rust
let count = use_signal(|| 0);

// Read value
let value = count.get();

// Update value
count.set(value + 1);

// Update with function
count.update(|v| v + 1);
```

### use_effect

Side effects with dependencies.

```rust
let data = use_signal(|| Vec::new());

use_effect(
    move || {
        // Effect runs when dependencies change
        println!("Data loaded: {:?}", data.get());

        // Optional cleanup
        Some(Box::new(|| {
            println!("Cleanup");
        }))
    },
    vec![data.get().len()],  // Dependencies
);
```

### use_input

Keyboard input handling.

```rust
use_input(move |event| {
    match event.code {
        KeyCode::Char('q') => { /* quit */ }
        KeyCode::Enter => { /* submit */ }
        KeyCode::Up => { /* move up */ }
        KeyCode::Down => { /* move down */ }
        _ => {}
    }
});
```

### use_mouse

Mouse event handling.

```rust
use_mouse(move |event| {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            println!("Clicked at ({}, {})", event.column, event.row);
        }
        MouseEventKind::Moved => { /* handle hover */ }
        _ => {}
    }
});
```

### use_focus

Focus management for form inputs.

```rust
let (focused, set_focused) = use_focus(false);

use_input(move |event| {
    if event.code == KeyCode::Tab {
        set_focused(!focused);
    }
});
```

### use_scroll

Scroll state management.

```rust
let scroll = use_scroll(items.len(), visible_height);

use_input(move |event| {
    match event.code {
        KeyCode::Up => scroll.scroll_up(),
        KeyCode::Down => scroll.scroll_down(),
        KeyCode::PageUp => scroll.page_up(),
        KeyCode::PageDown => scroll.page_down(),
        _ => {}
    }
});
```

### use_app

Application control.

```rust
let app = use_app();

use_input(move |event| {
    if event.code == KeyCode::Char('q') {
        app.exit();  // Exit the application
    }
});
```

### use_window_title

Set terminal window title.

```rust
use_window_title("My TUI App");
```

## Cross-thread Rendering

When updating state from background threads:

```rust
use std::thread;
use std::sync::{Arc, RwLock};

fn main() -> std::io::Result<()> {
    let shared_data = Arc::new(RwLock::new(String::new()));
    let data_clone = Arc::clone(&shared_data);

    thread::spawn(move || {
        loop {
            // Update shared state
            *data_clone.write().unwrap() = fetch_data();

            // Notify rnk to re-render
            rnk::request_render();

            thread::sleep(Duration::from_secs(1));
        }
    });

    render(move || app(&shared_data)).run()
}
```

## Println API

Print persistent messages above the UI (inline mode only):

```rust
// Simple text
rnk::println("Task completed!");

// Formatted text
rnk::println(format!("Downloaded {} files", count));

// Rich elements
let banner = Box::new()
    .border_style(BorderStyle::Round)
    .child(Text::new("Success!").color(Color::Green).into_element())
    .into_element();
rnk::println(banner);
```

## Colors

```rust
// Basic colors
Color::Black, Color::Red, Color::Green, Color::Yellow,
Color::Blue, Color::Magenta, Color::Cyan, Color::White,
Color::Gray

// 256 colors
Color::Ansi256(240)  // 0-255

// RGB colors
Color::Rgb { r: 255, g: 128, b: 0 }
```

## Testing

rnk provides testing utilities for verifying UI components:

```rust
use rnk::testing::{TestRenderer, assert_layout_valid};

#[test]
fn test_component() {
    let element = my_component();

    // Validate layout
    let renderer = TestRenderer::new(80, 24);
    renderer.validate_layout(&element).expect("valid layout");

    // Check rendered output
    let output = rnk::render_to_string(&element, 80);
    assert!(output.contains("expected text"));
}
```

## Running Examples

```bash
# Interactive demo with keyboard input
cargo run --example interactive_demo

# Streaming output demo
cargo run --example streaming_demo

# Counter example
cargo run --example counter

# Hello world
cargo run --example hello

# Static rendering demo
cargo run --example static_demo

# Todo app (comprehensive demo)
cargo run --example todo_app
```

## Architecture

```
rnk/
├── components/     # UI components (Box, Text, List, etc.)
├── core/           # Element, Style, Color primitives
├── hooks/          # React-like hooks (use_signal, use_effect, etc.)
├── layout/         # Taffy-based flexbox layout engine
├── renderer/       # Terminal rendering, App runner
└── testing/        # Test utilities
```

## Comparison with Ink/Bubbletea

| Feature | rnk | Ink | Bubbletea |
|---------|-----|-----|-----------|
| Language | Rust | JavaScript | Go |
| Rendering | Line-level diff | Line-level diff | Line-level diff |
| Layout | Flexbox (Taffy) | Flexbox (Yoga) | Manual |
| State | Hooks | React hooks | Model-Update |
| Inline mode | ✅ | ✅ | ✅ |
| Fullscreen | ✅ | ✅ | ✅ |
| Println | ✅ | Static component | tea.Println |
| Cross-thread | request_render() | - | tea.Program.Send |

## License

MIT
