//! Table example - displaying tabular data with flexbox
//!
//! Equivalent to ink's examples/table
//!
//! Run with: cargo run --example table

use tink::prelude::*;

fn main() -> std::io::Result<()> {
    render(app)
}

/// Sample user data
struct User {
    id: u32,
    name: &'static str,
    email: &'static str,
}

fn app() -> Element {
    let users = vec![
        User { id: 0, name: "alice_dev", email: "alice@example.com" },
        User { id: 1, name: "bob_smith", email: "bob.smith@mail.com" },
        User { id: 2, name: "charlie123", email: "charlie@test.org" },
        User { id: 3, name: "diana_code", email: "diana@company.io" },
        User { id: 4, name: "eve_hacker", email: "eve@secure.net" },
        User { id: 5, name: "frank_ops", email: "frank@devops.com" },
        User { id: 6, name: "grace_ml", email: "grace@ai.research" },
        User { id: 7, name: "henry_web", email: "henry@frontend.dev" },
        User { id: 8, name: "iris_data", email: "iris@analytics.co" },
        User { id: 9, name: "jack_api", email: "jack@backend.io" },
    ];

    Box::new()
        .flex_direction(FlexDirection::Column)
        .width(80)
        .padding(1)
        // Title
        .child(
            Text::new("User Table")
                .color(Color::Cyan)
                .bold()
                .underline()
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Header row
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .border_style(BorderStyle::Single)
                .border_color(Color::Ansi256(240))
                .child(
                    Box::new()
                        .width(8)
                        .child(Text::new("ID").bold().color(Color::Yellow).into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .width(20)
                        .child(Text::new("Name").bold().color(Color::Yellow).into_element())
                        .into_element(),
                )
                .child(
                    Box::new()
                        .flex_grow(1.0)
                        .child(Text::new("Email").bold().color(Color::Yellow).into_element())
                        .into_element(),
                )
                .into_element(),
        )
        // Data rows
        .children(users.iter().enumerate().map(|(idx, user)| {
            let bg = if idx % 2 == 0 {
                Some(Color::Ansi256(236))
            } else {
                None
            };

            let mut row = Box::new()
                .flex_direction(FlexDirection::Row)
                .padding_x(1.0);

            if let Some(bg_color) = bg {
                row = row.background(bg_color);
            }

            row.child(
                Box::new()
                    .width(8)
                    .child(
                        Text::new(format!("{}", user.id))
                            .color(Color::Ansi256(245))
                            .into_element(),
                    )
                    .into_element(),
            )
            .child(
                Box::new()
                    .width(20)
                    .child(Text::new(user.name).color(Color::Green).into_element())
                    .into_element(),
            )
            .child(
                Box::new()
                    .flex_grow(1.0)
                    .child(Text::new(user.email).color(Color::Blue).into_element())
                    .into_element(),
            )
            .into_element()
        }))
        // Footer
        .child(Newline::new().into_element())
        .child(
            Text::new(format!("Total: {} users", users.len()))
                .dim()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(Text::new("Press 'q' to exit").dim().into_element())
        .into_element()
}
