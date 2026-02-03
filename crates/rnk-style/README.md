# rnk-style

Standalone terminal styling library for Rust. Style your terminal output without the full TUI framework.

Inspired by [Lip Gloss](https://github.com/charmbracelet/lipgloss).

## Features

- **Chainable API**: Fluent style building with `.fg()`, `.bg()`, `.bold()`, etc.
- **Color Support**: ANSI 16, ANSI 256, and RGB/Hex colors
- **Text Formatting**: Bold, italic, underline, strikethrough, dim, inverse
- **Layout**: Padding, margin, borders with multiple styles
- **Alignment**: Left, center, right text alignment
- **Zero Dependencies**: Only `unicode-width` for proper character width handling

## Quick Start

```rust
use rnk_style::{Style, Color};

fn main() {
    let style = Style::new()
        .fg(Color::Cyan)
        .bg(Color::Black)
        .bold()
        .padding(1, 2)
        .border(BorderStyle::Rounded);

    println!("{}", style.render("Hello, World!"));
}
```

## Examples

### Basic Styling

```rust
use rnk_style::{Style, Color};

// Simple colored text
let error = Style::new().fg(Color::Red).bold();
println!("{}", error.render("Error: Something went wrong"));

// With background
let highlight = Style::new()
    .fg(Color::Black)
    .bg(Color::Yellow);
println!("{}", highlight.render("Important!"));
```

### Padding and Borders

```rust
use rnk_style::{Style, Color, BorderStyle};

let box_style = Style::new()
    .padding(1, 2)
    .border(BorderStyle::Rounded)
    .border_fg(Color::Cyan);

println!("{}", box_style.render("Boxed content"));
```

### Preset Styles

```rust
use rnk_style::Style;

println!("{}", Style::error().render("Error message"));
println!("{}", Style::success().render("Success!"));
println!("{}", Style::warning().render("Warning..."));
println!("{}", Style::info().render("Info"));
```

### Hex Colors

```rust
use rnk_style::{Style, Color};

let style = Style::new()
    .fg(Color::hex("#ff6b6b"))
    .bg(Color::hex("#2d3436"));

println!("{}", style.render("Custom colors"));
```

## License

MIT
