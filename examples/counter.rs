//! Interactive counter example demonstrating hooks

use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(counter_app)
}

fn counter_app() -> Element {
    // App context for programmatic exit
    let app = use_app();

    // Reactive state
    let count = use_signal(|| 0i32);

    // Handle keyboard input
    let count_for_input = count.clone();
    use_input(move |input, key| {
        if input == "q" {
            app.exit();
        } else if key.up_arrow || input == "k" {
            count_for_input.update(|c| *c += 1);
        } else if key.down_arrow || input == "j" {
            count_for_input.update(|c| *c -= 1);
        } else if input == "r" {
            count_for_input.set(0);
        }
    });

    // Build UI
    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .child(
            Text::new("Counter Demo")
                .color(Color::Cyan)
                .bold()
                .into_element()
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .child(
                    Text::new(format!("Count: {}", count.get()))
                        .color(if count.get() >= 0 { Color::Green } else { Color::Red })
                        .bold()
                        .into_element()
                )
                .into_element()
        )
        .child(
            Box::new()
                .margin_top(1.0)
                .flex_direction(FlexDirection::Column)
                .child(
                    Text::new("Controls:")
                        .dim()
                        .into_element()
                )
                .child(
                    Text::new("  j/Down: Decrement")
                        .dim()
                        .into_element()
                )
                .child(
                    Text::new("  k/Up:   Increment")
                        .dim()
                        .into_element()
                )
                .child(
                    Text::new("  r:      Reset")
                        .dim()
                        .into_element()
                )
                .child(
                    Text::new("  q:      Exit")
                        .dim()
                        .into_element()
                )
                .into_element()
        )
        .into_element()
}
