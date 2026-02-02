//! Macros Demo - Declarative UI with macros
//!
//! This example demonstrates the declarative macros for building UI:
//! - `row!` and `col!` for layouts
//! - `text!` for text elements
//! - `spacer!` for flexible spacing
//! - `when!` for conditional rendering
//! - `list!` for rendering collections

use rnk::hooks::{use_app, use_input, use_signal};
use rnk::prelude::*;
use rnk::{col, list, row, spacer, text, when};

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let selected = use_signal(|| 0usize);
    let show_footer = use_signal(|| true);

    let items = vec!["Apple", "Banana", "Cherry", "Date", "Elderberry"];

    let selected_for_input = selected.clone();
    let show_footer_for_input = show_footer.clone();
    use_input(move |input, key| {
        if key.up_arrow {
            selected_for_input.update(|s| {
                if *s > 0 {
                    *s -= 1
                }
            });
        } else if key.down_arrow {
            selected_for_input.update(|s| {
                if *s < 4 {
                    *s += 1
                }
            });
        } else if input == "f" {
            show_footer_for_input.update(|v| *v = !*v);
        } else if input == "q" || key.escape {
            app.exit();
        }
    });

    let current = selected.get();
    let footer_visible = show_footer.get();

    // Using macros for declarative UI
    col! {
        // Header
        text!("=== Macros Demo ==="),
        text!(""),

        // Instructions row
        row! {
            text!("↑/↓: Navigate"),
            spacer!(),
            text!("f: Toggle footer"),
            spacer!(),
            text!("q: Quit"),
        },
        text!(""),

        // List of items using list! macro
        list!(items.iter().enumerate(), |item| {
            let (idx, name) = item;
            let prefix = if idx == current { "→ " } else { "  " };
            text!("{}{}", prefix, name)
        }),

        text!(""),

        // Conditional footer using when! macro
        when!(footer_visible =>
            row! {
                text!("Selected: "),
                text!("{}", items[current]),
                spacer!(),
                text!("(footer visible)"),
            };
            else text!("(footer hidden - press 'f' to show)")
        ),
    }
}
