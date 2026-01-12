//! Integration tests for tink

use std::cell::RefCell;
use std::rc::Rc;

use rnk::hooks::{HookContext, with_hooks};
use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::renderer::Output;

#[test]
fn test_simple_box_render() {
    let element = Box::new()
        .padding(1)
        .child(Text::new("Hello").into_element())
        .into_element();

    let mut engine = LayoutEngine::new();
    engine.compute(&element, 80, 24);

    // Verify layout was computed
    let layout = engine.get_layout(element.id);
    assert!(layout.is_some());
}

#[test]
fn test_nested_boxes() {
    let element = Box::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Box::new()
                .child(Text::new("Row 1").into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .child(Text::new("Row 2").into_element())
                .into_element(),
        )
        .into_element();

    let mut engine = LayoutEngine::new();
    engine.compute(&element, 80, 24);

    let layout = engine.get_layout(element.id);
    assert!(layout.is_some());
}

#[test]
fn test_styled_text() {
    let element = Text::new("Styled")
        .color(Color::Green)
        .bold()
        .underline()
        .into_element();

    assert_eq!(element.style.color, Some(Color::Green));
    assert!(element.style.bold);
    assert!(element.style.underline);
}

#[test]
fn test_output_buffer() {
    let mut output = Output::new(40, 10);
    let style = rnk::core::Style::default();

    output.write(0, 0, "Hello, World!", &style);
    let rendered = output.render();

    assert!(rendered.contains("Hello, World!"));
}

#[test]
fn test_colored_output() {
    let mut output = Output::new(40, 10);
    let mut style = rnk::core::Style::default();
    style.color = Some(Color::Red);
    style.bold = true;

    output.write(0, 0, "Error", &style);
    let rendered = output.render();

    // Should contain ANSI escape codes
    assert!(rendered.contains("\x1b["));
    assert!(rendered.contains("Error"));
}

#[test]
fn test_border_rendering() {
    let element = Box::new()
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .width(10)
        .height(3)
        .into_element();

    assert_eq!(element.style.border_style, BorderStyle::Round);
    assert!(element.style.has_border());
}

#[test]
fn test_flex_properties() {
    let element = Box::new()
        .flex_direction(FlexDirection::Column)
        .justify_content(JustifyContent::Center)
        .align_items(AlignItems::Center)
        .gap(2.0)
        .into_element();

    assert_eq!(element.style.flex_direction, FlexDirection::Column);
    assert_eq!(element.style.justify_content, JustifyContent::Center);
    assert_eq!(element.style.align_items, AlignItems::Center);
    assert_eq!(element.style.gap, 2.0);
}

#[test]
fn test_padding_and_margin() {
    let element = Box::new().padding(2).margin(1).into_element();

    assert_eq!(element.style.padding.top, 2.0);
    assert_eq!(element.style.padding.right, 2.0);
    assert_eq!(element.style.margin.top, 1.0);
    assert_eq!(element.style.margin.left, 1.0);
}

// === Hooks Integration Tests ===

#[test]
fn test_use_signal_in_component() {
    let ctx = Rc::new(RefCell::new(HookContext::new()));

    // Simulate a component render
    let element = with_hooks(ctx.clone(), || {
        let count = use_signal(|| 42i32);

        Box::new()
            .child(Text::new(format!("Count: {}", count.get())).into_element())
            .into_element()
    });

    assert!(element.children.len() == 1);
}

#[test]
fn test_signal_state_persistence() {
    let ctx = Rc::new(RefCell::new(HookContext::new()));

    // First render
    with_hooks(ctx.clone(), || {
        let count = use_signal(|| 0i32);
        count.set(100);
    });

    // Second render - state should persist
    let value = with_hooks(ctx.clone(), || {
        let count = use_signal(|| 0i32);
        count.get()
    });

    assert_eq!(value, 100);
}

#[test]
fn test_multiple_signals_in_component() {
    let ctx = Rc::new(RefCell::new(HookContext::new()));

    with_hooks(ctx.clone(), || {
        let name = use_signal(|| "Alice".to_string());
        let age = use_signal(|| 30i32);

        assert_eq!(name.get(), "Alice");
        assert_eq!(age.get(), 30);

        name.set("Bob".to_string());
        age.set(25);
    });

    // Verify persistence
    with_hooks(ctx.clone(), || {
        let name = use_signal(|| "ignored".to_string());
        let age = use_signal(|| 999i32);

        assert_eq!(name.get(), "Bob");
        assert_eq!(age.get(), 25);
    });
}

#[test]
fn test_signal_update_closure() {
    let ctx = Rc::new(RefCell::new(HookContext::new()));

    with_hooks(ctx.clone(), || {
        let items = use_signal(|| vec![1, 2, 3]);

        items.update(|v| v.push(4));
        assert_eq!(items.get(), vec![1, 2, 3, 4]);

        items.update(|v| v.retain(|&x| x % 2 == 0));
        assert_eq!(items.get(), vec![2, 4]);
    });
}

#[test]
fn test_reactive_ui_pattern() {
    let ctx = Rc::new(RefCell::new(HookContext::new()));

    // Simulate initial render
    let element1 = with_hooks(ctx.clone(), || {
        let selected = use_signal(|| 0usize);

        Box::new()
            .child(Text::new(format!("Selected: {}", selected.get())).into_element())
            .into_element()
    });

    // Update state
    with_hooks(ctx.clone(), || {
        let selected = use_signal(|| 0usize);
        selected.set(5);
    });

    // Re-render should reflect new state
    let element2 = with_hooks(ctx.clone(), || {
        let selected = use_signal(|| 0usize);

        Box::new()
            .child(Text::new(format!("Selected: {}", selected.get())).into_element())
            .into_element()
    });

    // First element had "Selected: 0", second has "Selected: 5"
    let text1 = element1
        .children
        .get(0)
        .and_then(|e| e.text_content.as_ref());
    let text2 = element2
        .children
        .get(0)
        .and_then(|e| e.text_content.as_ref());

    assert_eq!(text1, Some(&"Selected: 0".to_string()));
    assert_eq!(text2, Some(&"Selected: 5".to_string()));
}

#[test]
fn test_position_absolute() {
    let element = Box::new()
        .width(40)
        .height(20)
        .child(
            Box::new()
                .position_absolute()
                .top(5.0)
                .left(10.0)
                .child(Text::new("Absolute").into_element())
                .into_element(),
        )
        .into_element();

    // Verify position is set correctly
    let absolute_child = element.children.iter().next().unwrap();
    assert_eq!(absolute_child.style.position, Position::Absolute);
    assert_eq!(absolute_child.style.top, Some(5.0));
    assert_eq!(absolute_child.style.left, Some(10.0));

    // Test layout computation
    let mut engine = LayoutEngine::new();
    engine.compute(&element, 80, 24);

    let child_layout = engine.get_layout(absolute_child.id);
    assert!(child_layout.is_some());

    // Absolute positioned element should be at the specified position
    let layout = child_layout.unwrap();
    assert_eq!(layout.x, 10.0);
    assert_eq!(layout.y, 5.0);
}

#[test]
fn test_display_none() {
    // Test that display: none elements have zero size in layout
    let element = Box::new()
        .width(80)
        .height(24)
        .child(
            Box::new()
                .hidden() // display: none
                .width(20)
                .height(10)
                .child(Text::new("Hidden").into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .child(Text::new("Visible").into_element())
                .into_element(),
        )
        .into_element();

    // Verify the hidden element has display: none
    let hidden_child = element.children.iter().next().unwrap();
    assert_eq!(hidden_child.style.display, Display::None);

    // Test layout computation - hidden element should have zero size
    let mut engine = LayoutEngine::new();
    engine.compute(&element, 80, 24);

    let hidden_layout = engine.get_layout(hidden_child.id);
    assert!(hidden_layout.is_some());

    let layout = hidden_layout.unwrap();
    // Elements with display: none should have zero width/height
    assert_eq!(layout.width, 0.0);
    assert_eq!(layout.height, 0.0);
}
