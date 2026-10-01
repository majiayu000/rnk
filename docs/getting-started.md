# Build Your First Rust Terminal UI with rnk

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

Example targets belong to the cloned repository, rather than the application
you created above. Clone it and start with these examples:

```bash
git clone https://github.com/majiayu000/rnk.git
cd rnk
cargo run --example hello      # minimal render path
cargo run --example counter    # state and keyboard input
cargo run --example todo_app   # app-shaped workflow
```

Then browse the [example catalog](../examples/README.md) for component demos
and larger showcase apps.

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

## 6. Choose Inline Or Fullscreen Rendering

Inline rendering keeps the UI in the normal terminal screen. It suits a CLI
that shows progress alongside existing shell output. Fullscreen rendering uses
the alternate screen and suits an application that owns the terminal viewport.
Choose the mode in `main`; the same `app` function works in either mode:

```rust
// Inline UI in the regular terminal screen:
render(app).inline().run()?;

// Or a fullscreen UI in the alternate screen:
render(app).fullscreen().run()?;
```

These are alternatives, not two sequential calls. Keep the `use_app().exit()`
handler from the counter example so the user can leave the application.
For a larger conversation UI, continue with the [chat quick start](CHAT_QUICKSTART.md).

## First-Run Questions

- **Do I install an `rnk` command?** Add the crate with `cargo add rnk` and run
  your own application. The repository's binary is a demo, not a supported
  standalone CLI.
- **Why is `cargo run --example counter` missing?** That target lives in the
  repository. Use the clone commands above; in your own project use `cargo run`.
- **Why does input behave differently in CI or when redirected?** Interactive
  evaluation needs a terminal. Inspect the [runtime API](https://docs.rs/rnk/latest/rnk/runtime/)
  for non-TTY behavior and use the [testing helpers](https://docs.rs/rnk/latest/rnk/testing/)
  to test components without an interactive session.
- **How does this compare with other terminal libraries?** See the
  [framework comparison](COMPARISON.md) for architecture and current gaps.
  [Ink](https://github.com/vadimdemedes/ink) documents React terminal apps in
  JavaScript; [Ratatui](https://ratatui.rs/) documents its Rust widget and buffer
  model. Choose based on your application and language, rather than an
  unsupported speed or feature-parity claim.

[Back to the project README](../README.md) · [License](../LICENSE)
