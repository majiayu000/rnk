# rnk

A React-like declarative terminal UI framework for Rust, inspired by [Ink](https://github.com/vadimdemedes/ink).

## Features

- **React-like API**: Familiar component model with hooks (`use_signal`, `use_effect`, `use_input`)
- **Declarative UI**: Build TUIs with composable components
- **Rich Components**: Box, Text, List, Table, Tabs, Progress, Sparkline, BarChart, and more
- **Flexbox Layout**: Powered by Taffy for flexible layouts
- **Mouse Support**: Full mouse event handling
- **Keyboard Input**: Easy keyboard event handling with focus management
- **Cross-platform**: Works on Linux, macOS, and Windows

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rnk = "0.2"
```

## Example

```rust
use rnk::prelude::*;

fn main() {
    rnk::run(app);
}

fn app() -> Element {
    let count = use_signal(|| 0);

    use_input(move |key| {
        if key.code == Key::Return {
            count.set(count.get() + 1);
        }
    });

    Box::new()
        .child(Text::new(format!("Count: {}", count.get())).into_element())
        .into_element()
}
```

## Components

### Basic Components
- `Box` - Flexbox container
- `Text` - Text with styling (colors, bold, italic, etc.)
- `Span` / `Line` - Rich text composition
- `Spacer` - Flexible space

### Data Display
- `List` - Selectable list with highlight
- `Table` - Data table with headers
- `Tabs` - Tab navigation

### Visualization
- `Progress` / `Gauge` - Progress bars
- `Sparkline` - Inline data visualization
- `BarChart` - Horizontal/vertical bar charts
- `Scrollbar` - Scroll indicator

## Hooks

- `use_signal` - Reactive state management
- `use_effect` - Side effects
- `use_input` - Keyboard input handling
- `use_mouse` - Mouse event handling
- `use_focus` - Focus management
- `use_scroll` - Scroll state
- `use_window_title` - Set terminal window title

## License

MIT
