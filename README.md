# rnk

A React-like declarative terminal UI framework for Rust, inspired by [Ink](https://github.com/vadimdemedes/ink) and [Bubbletea](https://github.com/charmbracelet/bubbletea).

[![Crates.io](https://img.shields.io/crates/v/rnk.svg)](https://crates.io/crates/rnk)
[![Documentation](https://docs.rs/rnk/badge.svg)](https://docs.rs/rnk)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- **React-like API**: Familiar component model with hooks (`use_signal`, `use_effect`, `use_input`, `use_cmd`)
- **Command System**: Elm-inspired side effect management for async tasks, timers, file I/O
- **Type-safe Commands**: `TypedCmd<M>` for compile-time message type checking
- **Declarative Macros**: `row!`, `col!`, `text!` for concise UI building
- **Declarative UI**: Build TUIs with composable components
- **Flexbox Layout**: Powered by [Taffy](https://github.com/DioxusLabs/taffy) for flexible layouts
- **Inline Mode** (default): Output persists in terminal history (like Ink/Bubbletea)
- **Fullscreen Mode**: Uses alternate screen buffer (like vim)
- **Line-level Diff Rendering**: Only changed lines are redrawn for efficiency
- **Persistent Output**: `println()` API for messages that persist above the UI
- **Cross-thread Rendering**: `request_render()` for async/multi-threaded apps
- **Rich Components**: 45+ components including Box, Text, List, Table, Tree, Modal, LineChart, Calendar, CodeEditor, and more
- **Animation System**: Keyframe animations with 28 easing functions
- **Chainable Style API**: CSS-like fluent styling with `.fg()`, `.bg()`, `.bold()`, `.p()`, `.m()`
- **Mouse Support**: Full mouse event handling
- **Bracketed Paste**: Distinguish between typed and pasted text
- **Theme System**: Centralized theming with semantic colors
- **Cross-platform**: Works on Linux, macOS, and Windows

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rnk = "0.14"
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

### Using Declarative Macros

```rust
use rnk::prelude::*;
use rnk::{col, row, text, spacer};

fn app() -> Element {
    col! {
        text!("Header").bold(),
        row! {
            text!("Left"),
            spacer!(),
            text!("Right"),
        },
        text!("Footer").dim(),
    }
}
```

### Counter with Keyboard Input

```rust
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let count = use_signal(|| 0i32);
    let app = use_app();

    use_input(move |input, key| {
        if input == "q" {
            app.exit();
        } else if key.up_arrow {
            count.update(|c| *c += 1);
        } else if key.down_arrow {
            count.update(|c| *c -= 1);
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

### Type-safe Commands with TypedCmd

```rust
use rnk::prelude::*;
use rnk::cmd::TypedCmd;
use std::time::Duration;

#[derive(Clone, Debug)]
enum Msg {
    DataLoaded(Vec<String>),
    Error(String),
    Tick(u64),
}

fn app() -> Element {
    let data = use_signal(|| Vec::new());
    let tick = use_signal(|| 0u64);

    // Type-safe command - compiler ensures callback returns Msg
    use_cmd_once(move || {
        TypedCmd::batch(vec![
            // Async data loading
            TypedCmd::perform(
                || async { vec!["Item 1".into(), "Item 2".into()] },
                Msg::DataLoaded,
            ),
            // Periodic tick
            TypedCmd::tick(Duration::from_secs(1), |_| Msg::Tick(1)),
        ])
        .on_msg(move |msg| {
            match msg {
                Msg::DataLoaded(items) => data.set(items),
                Msg::Tick(n) => tick.update(|t| *t += n),
                Msg::Error(_) => {}
            }
        })
    });

    col! {
        text!("Data: {:?}", data.get()),
        text!("Ticks: {}", tick.get()),
    }
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

use_input(move |input, _key| {
    if input == " " {
        if rnk::is_alt_screen().unwrap_or(false) {
            rnk::exit_alt_screen();  // Switch to inline
        } else {
            rnk::enter_alt_screen(); // Switch to fullscreen
        }
    }
});
```

## Components

### Layout Components

| Component | Description |
|-----------|-------------|
| `Box` | Flexbox container with full layout support |
| `Spacer` | Flexible space filler |
| `Newline` | Vertical space |
| `Transform` | Transform child text content |

### Text & Display

| Component | Description |
|-----------|-------------|
| `Text` | Styled text with colors and formatting |
| `Gradient` | Text with gradient colors |
| `Hyperlink` | Clickable terminal hyperlinks |
| `Spinner` | Animated loading indicator |
| `Cursor` | Blinking cursor component |

### Data Display

| Component | Description |
|-----------|-------------|
| `List` | Selectable list with keyboard navigation |
| `Table` | Data table with headers and styling |
| `Tree` | Hierarchical tree view |
| `Progress` / `Gauge` | Progress bars |
| `Sparkline` | Inline data visualization |
| `BarChart` | Horizontal/vertical bar charts |
| `Scrollbar` | Scrollbar indicator |

### Input Components

| Component | Description |
|-----------|-------------|
| `TextInput` | Single-line text input |
| `TextArea` | Multi-line text editor with vim keybindings |
| `SelectInput` | Dropdown-style selection |
| `MultiSelect` | Multiple item selection |
| `Confirm` | Yes/No confirmation dialog |
| `FilePicker` | File system browser |

### Navigation

| Component | Description |
|-----------|-------------|
| `Tabs` | Tab navigation |
| `Paginator` | Page navigation (dots, numbers, arrows) |
| `Viewport` | Scrollable content viewport |
| `Help` | Keyboard shortcut help display |

### Feedback & Overlay

| Component | Description |
|-----------|-------------|
| `Modal` | Modal overlay |
| `Dialog` | Dialog box with buttons |
| `Notification` / `Toast` | Notification messages |
| `Message` | Styled message boxes (info, success, warning, error) |
| `Static` | Permanent output above dynamic UI |

### Theming

| Component | Description |
|-----------|-------------|
| `Theme` | Centralized theme configuration |
| `ThemeBuilder` | Fluent theme construction |

### Example: Box

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

### Example: Tree

```rust
let root = TreeNode::new("root", "Root")
    .child(TreeNode::new("child1", "Child 1")
        .child(TreeNode::new("grandchild", "Grandchild")))
    .child(TreeNode::new("child2", "Child 2"));

Tree::new(root)
    .expanded(&["root", "child1"])
    .selected("grandchild")
    .into_element()
```

### Example: Modal

```rust
Modal::new()
    .visible(show_modal.get())
    .align(ModalAlign::Center)
    .child(
        Dialog::new()
            .title("Confirm")
            .message("Are you sure?")
            .buttons(vec!["Yes", "No"])
            .on_select(|idx| { /* handle */ })
            .into_element()
    )
    .into_element()
```

### Example: Notification

```rust
Notification::new()
    .items(notifications.get())
    .position(NotificationPosition::TopRight)
    .max_visible(3)
    .into_element()
```

## Hooks

### State Management

| Hook | Description |
|------|-------------|
| `use_signal` | Reactive state management |
| `use_memo` | Memoized computation |
| `use_callback` | Memoized callback |

### Effects & Commands

| Hook | Description |
|------|-------------|
| `use_effect` | Side effects with dependencies |
| `use_effect_once` | One-time side effect |
| `use_cmd` | Command execution with dependencies |
| `use_cmd_once` | One-time command execution |

### Input

| Hook | Description |
|------|-------------|
| `use_input` | Keyboard input handling |
| `use_mouse` | Mouse event handling |
| `use_paste` | Bracketed paste handling |
| `use_text_input` | Text input state management |

### Focus & Navigation

| Hook | Description |
|------|-------------|
| `use_focus` | Focus state for a component |
| `use_focus_manager` | Global focus management |
| `use_scroll` | Scroll state management |

### Application

| Hook | Description |
|------|-------------|
| `use_app` | Application control (exit, etc.) |
| `use_window_title` | Set terminal window title |
| `use_frame_rate` | Frame rate monitoring |
| `use_measure` | Measure element dimensions |
| `use_stdin` / `use_stdout` / `use_stderr` | Stdio access |

### Example: use_paste

```rust
use_paste(move |event| {
    match event {
        PasteEvent::Start => { /* paste started */ }
        PasteEvent::Content(text) => {
            // Handle pasted text
            input_buffer.update(|b| b.push_str(&text));
        }
        PasteEvent::End => { /* paste ended */ }
    }
});
```

### Example: use_memo

```rust
let items = use_signal(|| vec![1, 2, 3, 4, 5]);

// Only recomputes when items change
let sum = use_memo(
    move || items.get().iter().sum::<i32>(),
    vec![items.get().len()],
);
```

## Command System

### Basic Commands

```rust
use rnk::cmd::Cmd;

// No-op command
Cmd::none()

// Execute multiple commands concurrently
Cmd::batch(vec![cmd1, cmd2, cmd3])

// Execute commands sequentially
Cmd::sequence(vec![cmd1, cmd2, cmd3])

// Delay execution
Cmd::sleep(Duration::from_secs(1))

// Timer tick (single)
Cmd::tick(Duration::from_secs(1), |timestamp| { /* handle */ })

// System clock aligned tick
Cmd::every(Duration::from_secs(1), |timestamp| { /* handle */ })

// Async task
Cmd::perform(|| async { /* work */ })

// Chain commands
cmd.and_then(another_cmd)
```

### Terminal Control Commands

```rust
// Clear screen
Cmd::clear_screen()

// Cursor control
Cmd::hide_cursor()
Cmd::show_cursor()

// Window title
Cmd::set_window_title("My App")

// Request window size (triggers resize event)
Cmd::window_size()

// Screen mode
Cmd::enter_alt_screen()
Cmd::exit_alt_screen()

// Mouse
Cmd::enable_mouse()
Cmd::disable_mouse()

// Bracketed paste
Cmd::enable_bracketed_paste()
Cmd::disable_bracketed_paste()
```

### External Process Execution

```rust
// Execute external process (suspends TUI)
Cmd::exec_cmd("vim", &["file.txt"], |result| {
    match result {
        ExecResult::Success(code) => { /* process exited */ }
        ExecResult::Error(err) => { /* error */ }
    }
})
```

### Type-safe Commands (TypedCmd)

```rust
use rnk::cmd::TypedCmd;

#[derive(Clone)]
enum Msg {
    Loaded(String),
    Error(String),
}

// Compiler ensures all callbacks return Msg
let cmd: TypedCmd<Msg> = TypedCmd::perform(
    || async { "data".to_string() },
    Msg::Loaded,
);

// Handle messages
cmd.on_msg(|msg| {
    match msg {
        Msg::Loaded(data) => { /* handle */ }
        Msg::Error(err) => { /* handle */ }
    }
})
```

## Declarative Macros

```rust
use rnk::{col, row, text, styled_text, spacer, box_element, when, list};

// Vertical layout
col! {
    text!("Line 1"),
    text!("Line 2"),
}

// Horizontal layout
row! {
    text!("Left"),
    spacer!(),
    text!("Right"),
}

// Formatted text
text!("Count: {}", count)

// Styled text
styled_text!("Error!", color: Color::Red)

// Conditional rendering
when!(show_error => text!("Error occurred!"))

// List from iterator
list!(items.iter(), |item| text!("{}", item))

// List with index
list_indexed!(items.iter(), |idx, item| text!("[{}] {}", idx, item))
```

## Theme System

```rust
use rnk::components::{Theme, ThemeBuilder, set_theme, get_theme, with_theme};

// Create custom theme
let theme = ThemeBuilder::new()
    .primary(Color::Cyan)
    .secondary(Color::Magenta)
    .success(Color::Green)
    .warning(Color::Yellow)
    .error(Color::Red)
    .build();

// Set global theme
set_theme(theme);

// Use theme colors
let color = get_theme().semantic_color(SemanticColor::Primary);

// Scoped theme
with_theme(dark_theme, || {
    // Components here use dark_theme
});
```

## Cross-thread Rendering

```rust
use std::thread;

fn main() -> std::io::Result<()> {
    thread::spawn(|| {
        loop {
            // Update state...

            // Notify rnk to re-render
            rnk::request_render();

            // Print persistent message
            rnk::println("Background task completed");

            thread::sleep(Duration::from_secs(1));
        }
    });

    render(app).run()
}
```

## Testing

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
# Basic examples
cargo run --example hello
cargo run --example counter
cargo run --example todo_app

# Showcase applications
cargo run --example rnk_top      # htop-like system monitor
cargo run --example rnk_git      # lazygit-style Git UI
cargo run --example rnk_chat     # Terminal chat client

# Component demos
cargo run --example tree_demo
cargo run --example multi_select_demo
cargo run --example notification_demo
cargo run --example textarea_demo
cargo run --example file_picker_demo
cargo run --example theme_demo

# Advanced examples
cargo run --example typed_cmd_demo
cargo run --example macros_demo
cargo run --example streaming_demo
cargo run --example glm_chat

# Note: use `--example` (not `cargo run example <name>`)
```

## Architecture

```
src/
├── animation/      # Keyframe animations, easing functions, spring physics
├── cmd/            # Command system (Cmd, TypedCmd, executor)
├── components/     # 45+ UI components
├── core/           # Element, Style, Color primitives
├── hooks/          # React-like hooks (use_signal, use_animation, use_transition)
├── layout/         # Taffy-based flexbox layout engine
├── macros.rs       # Declarative UI macros
├── renderer/       # Terminal rendering, App runner
├── runtime/        # Signal handling, environment detection
└── testing/        # TestRenderer, TestHarness, assertions
```

## Comparison with Ink/Bubbletea

| Feature | rnk | Ink | Bubbletea |
|---------|-----|-----|-----------|
| Language | Rust | JavaScript | Go |
| Rendering | Line-level diff | Line-level diff | Line-level diff |
| Layout | Flexbox (Taffy) | Flexbox (Yoga) | Manual |
| State | Hooks + Signals | React hooks | Model-Update |
| Type-safe Cmds | TypedCmd<M> | N/A | N/A |
| Declarative Macros | row!/col!/text! | JSX | N/A |
| Components | 40+ | ~10 | Bubbles lib |
| Inline mode | ✓ | ✓ | ✓ |
| Fullscreen | ✓ | ✓ | ✓ |
| Mouse support | ✓ | ✗ | ✓ |
| Bracketed paste | ✓ | ✗ | ✓ |
| Theme system | ✓ | ✗ | Lipgloss |
| Println | ✓ | Static | tea.Println |
| Cross-thread | request_render() | - | tea.Program.Send |
| Suspend/Resume | ✓ | ✗ | ✓ |

## License

MIT
