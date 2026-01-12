//! Simple test - just print the UI without alternate screen

use rnk::core::Dimension;
use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::renderer::Output;

fn main() {
    // Get terminal size
    let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
    println!("Terminal: {}x{}\n", width, height);

    let element = create_ui();

    let mut engine = LayoutEngine::new();
    engine.compute(&element, width, height);

    let mut output = Output::new(width, height);
    render_element(&element, &engine, &mut output, 0.0, 0.0);

    // Just print, no alternate screen
    println!("{}", output.render());
}

fn create_ui() -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .padding_y(1.0)
                .child(
                    Text::new("Tink Interactive Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Box::new()
                        .width(30)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Yellow)
                        .padding(1)
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("Counter Demo")
                                .color(Color::Yellow)
                                .bold()
                                .underline()
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(
                            Box::new()
                                .flex_direction(FlexDirection::Row)
                                .child(Text::new("Value: ").color(Color::White).into_element())
                                .child(Text::new("42").color(Color::Green).bold().into_element())
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(
                            Text::new("[+] Increment")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .child(
                            Text::new("[-] Decrement")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .child(
                            Text::new("[0] Reset")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Box::new().width(2).into_element())
                .child(
                    Box::new()
                        .flex_grow(1.0)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Blue)
                        .padding(1)
                        .flex_direction(FlexDirection::Column)
                        .child(
                            Text::new("List Navigation")
                                .color(Color::Blue)
                                .bold()
                                .underline()
                                .into_element(),
                        )
                        .child(Newline::new().into_element())
                        .child(create_list_items())
                        .child(Newline::new().into_element())
                        .child(
                            Text::new("[j/k] Navigate")
                                .color(Color::Ansi256(240))
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .border_style(BorderStyle::Single)
                .border_color(Color::Magenta)
                .padding(1)
                .child(
                    Text::new("Welcome! Press 'h' for help.")
                        .color(Color::Magenta)
                        .italic()
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}

fn create_list_items() -> Element {
    let items = vec!["First item", "Second item", "Third item", "Fourth item"];
    let selected = 0;

    let mut container = Box::new().flex_direction(FlexDirection::Column);

    for (i, item) in items.iter().enumerate() {
        let is_selected = i == selected;
        let mut row = Box::new().flex_direction(FlexDirection::Row).padding_x(1.0);

        if is_selected {
            row = row.background(Color::Ansi256(236));
        }

        row = row
            .child(
                Text::new(if is_selected { "> " } else { "  " })
                    .color(Color::Cyan)
                    .bold()
                    .into_element(),
            )
            .child(
                Text::new(*item)
                    .color(if is_selected {
                        Color::White
                    } else {
                        Color::Ansi256(250)
                    })
                    .into_element(),
            );

        container = container.child(row.into_element());
    }

    container.into_element()
}

fn render_element(
    element: &Element,
    engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) {
    use rnk::core::{Display, Position};

    if element.style.display == Display::None {
        return;
    }

    let layout = match engine.get_layout(element.id) {
        Some(l) => l,
        None => return,
    };

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let w = layout.width as u16;
    let h = layout.height as u16;

    // Background
    if element.style.background_color.is_some() {
        for row in 0..h {
            output.write(x, y + row, &" ".repeat(w as usize), &element.style);
        }
    }

    // Border
    if element.style.has_border() {
        let (tl, tr, bl, br, hz, vt) = element.style.border_style.chars();
        let mut style = element.style.clone();

        style.color = element.style.get_border_top_color();
        output.write(
            x,
            y,
            &format!("{}{}{}", tl, hz.repeat((w as usize).saturating_sub(2)), tr),
            &style,
        );

        style.color = element.style.get_border_bottom_color();
        output.write(
            x,
            y + h.saturating_sub(1),
            &format!("{}{}{}", bl, hz.repeat((w as usize).saturating_sub(2)), br),
            &style,
        );

        for row in 1..h.saturating_sub(1) {
            style.color = element.style.get_border_left_color();
            output.write(x, y + row, vt, &style);
            style.color = element.style.get_border_right_color();
            output.write(x + w.saturating_sub(1), y + row, vt, &style);
        }
    }

    // Text
    if let Some(text) = &element.text_content {
        let text_x =
            x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
        let text_y =
            y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;
        output.write(text_x, text_y, text, &element.style);
    }

    // Children
    let cx = offset_x + layout.x;
    let cy = offset_y + layout.y;

    for child in element.children.iter() {
        if child.style.position == Position::Absolute {
            render_element(
                child,
                engine,
                output,
                child.style.left.unwrap_or(0.0),
                child.style.top.unwrap_or(0.0),
            );
        } else {
            render_element(child, engine, output, cx, cy);
        }
    }
}
