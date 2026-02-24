//! Debug render for interactive demo

use rnk::core::Dimension;
use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::renderer::Output;

fn main() {
    let element = create_demo_ui();

    let mut engine = LayoutEngine::new();
    engine.compute(&element, 80, 24);

    let mut output = Output::new(80, 24);
    render_element(&element, &engine, &mut output, 0.0, 0.0, 0);

    println!("{}", output.render());
}

fn create_demo_ui() -> Element {
    Box::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .padding(1)
        // Title
        .child(
            Box::new()
                .border_style(BorderStyle::Double)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .padding_y(1.0)
                .child(
                    Text::new("Interactive Demo")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        // Main content - two columns
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                // Left column
                .child(
                    Box::new()
                        .width(30)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Yellow)
                        .padding(1)
                        .child(
                            Text::new("Counter Demo")
                                .color(Color::Yellow)
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Box::new().width(2).into_element())
                // Right column
                .child(
                    Box::new()
                        .flex_grow(1.0)
                        .border_style(BorderStyle::Round)
                        .border_color(Color::Blue)
                        .padding(1)
                        .child(
                            Text::new("List Navigation")
                                .color(Color::Blue)
                                .into_element(),
                        )
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}

fn render_element(
    element: &Element,
    engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
    depth: usize,
) {
    use rnk::core::{Display, Position};

    if element.style.display == Display::None {
        return;
    }

    let layout = match engine.get_layout(element.id) {
        Some(l) => l,
        None => return,
    };

    let abs_x = offset_x + layout.x;
    let abs_y = offset_y + layout.y;
    let x = abs_x as u16;
    let y = abs_y as u16;
    let w = layout.width as u16;
    let h = layout.height as u16;

    let indent = "  ".repeat(depth);
    eprintln!(
        "{}Render: offset=({:.0},{:.0}) + layout=({:.0},{:.0}) = ({},{}) size={}x{}",
        indent, offset_x, offset_y, layout.x, layout.y, x, y, w, h
    );

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
    for child in element.children.iter() {
        if child.style.position == Position::Absolute {
            render_element(
                child,
                engine,
                output,
                child.style.left.unwrap_or(0.0),
                child.style.top.unwrap_or(0.0),
                depth + 1,
            );
        } else {
            render_element(child, engine, output, abs_x, abs_y, depth + 1);
        }
    }
}
