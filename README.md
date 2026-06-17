# rnk

A React-like declarative terminal UI framework for Rust, inspired by [Ink](https://github.com/vadimdemedes/ink) and [Bubbletea](https://github.com/charmbracelet/bubbletea).

[![Crates.io](https://img.shields.io/crates/v/rnk.svg)](https://crates.io/crates/rnk)
[![Documentation](https://docs.rs/rnk/badge.svg)](https://docs.rs/rnk)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Version Status

The Crates.io badge above is the source of truth for published installs. This
checkout currently declares `0.19.2`; when the checkout is ahead of the latest
published release, use the git dependency below for the latest source version.
Release notes live in [CHANGELOG.md](CHANGELOG.md); older published release
artifacts remain available on
[GitHub Releases](https://github.com/majiayu000/rnk/releases).

Current maturity planning and issue acceptance criteria live in
[docs/RNK_MATURITY_SPEC.md](docs/RNK_MATURITY_SPEC.md).
The current public API boundary and pre-1.0 semver policy live in
[docs/API_STABILITY.md](docs/API_STABILITY.md).
Interactive component state and event contracts live in
[docs/INTERACTIVE_COMPONENT_CONTRACTS.md](docs/INTERACTIVE_COMPONENT_CONTRACTS.md).
Focus, accessibility, and input semantics live in
[docs/FOCUS_ACCESSIBILITY_INPUT.md](docs/FOCUS_ACCESSIBILITY_INPUT.md).

## Features

- **React-like API**: Familiar component model with hooks (`use_signal`, `use_state`, `use_ref`, `use_context`, `use_effect`, `use_layout_effect`, `use_input`, `use_cmd`)
- **Command System**: Elm-inspired side effect management for async tasks, timers, file I/O
- **Type-safe Commands**: `Cmd<M>` for compile-time message type checking
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
- **Theme System**: Centralized theming with semantic colors, design tokens, variants, and shared action styles
- **Cross-platform**: Works on Linux, macOS, and Windows

### Recent Improvements

- Added core hooks: `use_state`, `use_ref`, `create_context`/`use_context`, `use_layout_effect`
- Integrated dirty-row rendering path in `Output::render()`
- Improved command executor startup resilience (runtime creation failure now degrades safely)
- Added `docs/vibe/design-guard-and-fixflow.md` for step-by-step design issue tracking and fixes

## Quick Start

rnk is distributed as a Rust crate for Cargo projects. Add the latest published
Crates.io release:

```bash
cargo add rnk
```

If you edit `Cargo.toml` manually, use the current version shown on the Crates.io badge or package page.

For the latest source version before the next release is published:

```toml
[dependencies]
rnk = { git = "https://github.com/majiayu000/rnk" }
```

The `rnk` binary target is a minimal repository demo and does not expose a
standalone CLI surface. Keep installation docs centered on Cargo dependencies
until a dedicated CLI distribution is added.

## Limitations

- The public distribution is the Rust crate API. The `rnk` binary target is a
  minimal repository demo, not a supported end-user CLI.
- Terminal feature support depends on the user's terminal emulator. Mouse input,
  hyperlinks, bracketed paste, alternate-screen behavior, and color rendering may
  vary by platform and terminal. See
  [docs/TERMINAL_COMPATIBILITY.md](docs/TERMINAL_COMPATIBILITY.md) for the
  current compatibility matrix and Unicode/ANSI behavior contract.
- `rnk-style`, `rnk-style-core`, and `rnk-icons` are workspace crates with their
  own package versions. Their release cadence may differ from the top-level
  `rnk` crate.
- The `http` feature is optional. Applications that need HTTP support must enable
  it explicitly.
- The framework targets Rust `1.88` and newer.

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

### Type-safe Commands with Cmd<M>

```rust
use rnk::prelude::*;
use rnk::cmd::Cmd;
use std::time::Duration;

#[derive(Clone, Debug)]
enum Msg {
    DataLoaded(Vec<String>),
    Tick(u64),
}

fn load_data() -> Cmd<Msg> {
    Cmd::perform(|| async { Msg::DataLoaded(vec!["Item 1".into(), "Item 2".into()]) })
}

fn start_tick() -> Cmd<Msg> {
    Cmd::tick(Duration::from_secs(1), |_| Msg::Tick(1))
}

fn init() -> Cmd<Msg> {
    Cmd::batch(vec![load_data(), start_tick()])
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
| `use_state` | Simplified `(value, setter)` state API |
| `use_ref` | Mutable persistent value without re-render |
| `create_context` / `use_context` | Context-based value sharing |
| `use_memo` | Memoized computation |
| `use_callback` | Memoized callback |

### Effects & Commands

| Hook | Description |
|------|-------------|
| `use_effect` | Side effects with dependencies |
| `use_effect_once` | One-time side effect |
| `use_layout_effect` | Layout effect API (currently aligned with `use_effect`) |
| `use_layout_effect_once` | One-time layout effect |
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
| `use_scoped_focus` | Focus state inside a traversal scope |
| `use_focus_manager` | Global focus management |
| `use_focus_traversal` | Default Tab / Shift+Tab focus traversal |
| `use_focus_traversal_in_scope` | Scoped Tab / Shift+Tab focus traversal |
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

### Type-safe Commands (Cmd<M>)

```rust
use rnk::cmd::Cmd;

#[derive(Clone)]
enum Msg {
    Loaded(String),
}

// Compiler ensures all callbacks return Msg
let cmd: Cmd<Msg> = Cmd::perform(|| async { Msg::Loaded("data".to_string()) });
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
use rnk::components::{
    ActionButton, ActionRole, ActionState, ComponentState, ComponentVariant,
    SemanticColor, Theme, ThemeBuilder, get_theme, set_theme, with_theme,
};
use rnk::core::Color;

// Create custom theme
let theme = ThemeBuilder::new("custom")
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

// Use derived design tokens and variants
let tokens = get_theme().design_tokens();
let focused = get_theme().variant_style(ComponentVariant::Primary, ComponentState::Focused);

// Use shared action styling for button-like labels
let action = ActionButton::new("Save")
    .role(ActionRole::Primary)
    .state(ActionState::Focused)
    .into_element();

// Scoped theme
let dark_theme = Theme::dark();
with_theme(dark_theme, |_| {
    // Components here use dark_theme
});
```

See [Design Tokens And Component Variants](docs/DESIGN_TOKENS_AND_VARIANTS.md)
for the current token and variant contract.

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

The list below is a curated representative set, not a generated index. Cargo
auto-discovers the full top-level example set from `examples/*.rs`.

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
├── cmd/            # Command system (Cmd<M>, executor)
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

See [docs/COMPARISON.md](docs/COMPARISON.md) for the current evidence-based
comparison. In short, `rnk` is a Rust-native, hook/signal-driven, Taffy-backed
terminal UI framework, while feature parity with Ink and Bubbletea depends on
the specific terminal behavior and application pattern being compared.

## License

MIT
