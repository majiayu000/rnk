# Getting Started

This guide takes a new `rnk` user from an empty Cargo project to a small
interactive terminal UI.

## 1. Create A Project

```bash
cargo new rnk-hello
cd rnk-hello
cargo add rnk
```

`rnk` is a Rust crate, not an end-user CLI. You add it to your application and
run your own binary with `cargo run`.

## 2. Render The Smallest App

Replace `src/main.rs` with:

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

Run it:

```bash
cargo run
```

## 3. Add Keyboard Interaction

Replace `src/main.rs` with:

```rust
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let count = use_signal(|| 0i32);
    let app = use_app();

    use_input({
        let count = count.clone();
        move |input, key| {
            if input == "q" {
                app.exit();
            } else if key.up_arrow {
                count.update(|value| *value += 1);
            } else if key.down_arrow {
                count.update(|value| *value -= 1);
            }
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .child(Text::new(format!("Count: {}", count.get())).bold().into_element())
        .child(Text::new("Up/Down changes the count, q exits").dim().into_element())
        .into_element()
}
```

Run it:

```bash
cargo run
```

## 4. Run Repository Examples

When learning from the `rnk` repository, start with these examples:

```bash
cargo run --example hello      # minimal render path
cargo run --example counter    # state and keyboard input
cargo run --example todo_app   # app-shaped workflow
```

Then browse `examples/README.md` for component demos and larger showcase apps.

## 5. Pick An Import Surface

Use the full prelude when learning:

```rust
use rnk::prelude::*;
```

Use the low-conflict prelude when you want fewer names in scope:

```rust
use rnk::prelude::lite::*;
```

Use the widget-focused prelude for component examples:

```rust
use rnk::prelude::widgets::*;
```

Advanced modules such as `rnk::renderer`, `rnk::runtime`, and `rnk::testing`
are available for integration work, but normal apps should start with the
prelude.
