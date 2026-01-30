//! Renderer benchmarks

use rnk::Style;
use rnk::core::{Color, Dimension, Element, FlexDirection};
use rnk::renderer::{ClipRegion, Output, render_to_string};

fn main() {
    divan::main();
}

#[divan::bench(args = [(80, 24), (120, 40), (200, 50), (300, 100)])]
fn output_buffer_creation(size: (u16, u16)) {
    let _output = Output::new(size.0, size.1);
}

#[divan::bench]
fn output_write_ascii() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    for y in 0..24 {
        output.write(0, y, "Hello, World! This is a test line.", &style);
    }
}

#[divan::bench]
fn output_write_styled() {
    let mut output = Output::new(80, 24);
    let mut style = Style::default();
    style.color = Some(Color::Green);
    style.bold = true;

    for y in 0..24 {
        output.write(0, y, "Styled text with colors and bold", &style);
    }
}

#[divan::bench]
fn output_write_cjk() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    for y in 0..24 {
        output.write(0, y, "你好世界！这是一段中文测试文本。", &style);
    }
}

#[divan::bench]
fn output_write_mixed() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    for y in 0..24 {
        output.write(0, y, "Hello 你好 World 世界 Mixed 混合", &style);
    }
}

#[divan::bench]
fn output_fill_rect() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    output.fill_rect(10, 5, 60, 14, '█', &style);
}

#[divan::bench(args = [(80, 24), (120, 40), (200, 50)])]
fn output_render_to_ansi(size: (u16, u16)) {
    let mut output = Output::new(size.0, size.1);
    let style = Style::default();

    for y in 0..size.1 {
        output.write(0, y, "Test content for rendering benchmark", &style);
    }

    divan::black_box(output.render());
}

#[divan::bench]
fn output_render_styled_ansi() {
    let mut output = Output::new(80, 24);

    let colors = [
        Color::Red,
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Cyan,
        Color::Magenta,
    ];

    for y in 0..24 {
        let mut style = Style::default();
        style.color = Some(colors[y as usize % colors.len()]);
        style.bold = y % 2 == 0;
        style.italic = y % 3 == 0;

        output.write(0, y, "Colorful styled text for benchmark", &style);
    }

    divan::black_box(output.render());
}

#[divan::bench]
fn render_simple_element() {
    let element = Element::text("Hello, World!");
    divan::black_box(render_to_string(&element, 80));
}

#[divan::bench]
fn render_nested_boxes() {
    let mut root = Element::root();

    let mut outer = Element::box_element();
    outer.style.padding = rnk::core::Edges::new(1.0, 2.0, 0.0, 2.0);

    let mut inner = Element::box_element();
    inner.add_child(Element::text("Nested content"));

    outer.add_child(inner);
    root.add_child(outer);

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench(args = [10, 50, 100])]
fn render_many_text_elements(count: usize) {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    for i in 0..count {
        root.add_child(Element::text(&format!("Line number {}", i)));
    }

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench]
fn render_styled_text() {
    let mut root = Element::root();

    let mut text = Element::text("Bold and colorful text");
    text.style.bold = true;
    text.style.color = Some(Color::Cyan);

    root.add_child(text);

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench]
fn output_clip_region() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    output.clip(ClipRegion {
        x1: 10,
        y1: 5,
        x2: 70,
        y2: 19,
    });

    for y in 0..24 {
        output.write(0, y, "This text should be clipped to the region", &style);
    }

    output.unclip();

    divan::black_box(output.render());
}

#[divan::bench]
fn output_overwrite_wide_chars() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    // Write wide characters
    output.write(0, 0, "你好世界你好世界你好世界", &style);

    // Overwrite with ASCII
    output.write(2, 0, "AAAA", &style);

    divan::black_box(output.render());
}

#[divan::bench]
fn render_cjk_content() {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    for _ in 0..10 {
        root.add_child(Element::text("这是一段中文测试文本，用于测试渲染性能。"));
    }

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench]
fn render_with_colors() {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    let colors = [Color::Red, Color::Green, Color::Blue, Color::Yellow];

    for (i, color) in colors.iter().enumerate() {
        let mut text = Element::text(&format!("Colored line {}", i));
        text.style.color = Some(*color);
        root.add_child(text);
    }

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench(args = [(40, 12), (80, 24), (120, 40)])]
fn render_full_screen(size: (u16, u16)) {
    let mut root = Element::root();
    root.style.width = Dimension::Points(size.0 as f32);
    root.style.height = Dimension::Points(size.1 as f32);
    root.style.flex_direction = FlexDirection::Column;

    for i in 0..size.1 {
        root.add_child(Element::text(&format!(
            "Line {:03}: {}",
            i,
            "x".repeat(size.0 as usize - 10)
        )));
    }

    divan::black_box(render_to_string(&root, size.0));
}
