use rnk::components::{Box as RnkBox, Text};
use rnk::core::{Element, FlexDirection};
use rnk::hooks::{
    KeyCodeKind, Mouse, MouseAction, MouseButton, UseFocusOptions, use_focus, use_focus_traversal,
    use_input, use_mouse, use_paste, use_signal,
};
use rnk::testing::TestHarness;

fn keyboard_app() -> Element {
    let value = use_signal(String::new);

    use_input({
        let value = value.clone();
        move |input, key| {
            if key.return_key {
                value.set("submitted".to_string());
            } else if !input.is_empty() {
                value.update(|current| current.push_str(input));
            }
        }
    });

    Text::new(format!("keyboard: {}", value.get())).into_element()
}

fn mouse_app() -> Element {
    let last_mouse = use_signal(|| "none".to_string());

    use_mouse({
        let last_mouse = last_mouse.clone();
        move |mouse| {
            if mouse.is_left_click() {
                last_mouse.set(format!("left:{}:{}", mouse.x, mouse.y));
            }
        }
    });

    Text::new(format!("mouse: {}", last_mouse.get())).into_element()
}

fn paste_app() -> Element {
    let paste = use_signal(|| "empty".to_string());

    use_paste({
        let paste = paste.clone();
        move |event| {
            paste.set(format!(
                "lines:{} text:{}",
                event.line_count(),
                event.content()
            ));
        }
    });

    Text::new(format!("paste: {}", paste.get())).into_element()
}

fn focus_app() -> Element {
    use_focus_traversal();

    let first = use_focus(UseFocusOptions::new().id("first").auto_focus());
    let second = use_focus(UseFocusOptions::new().id("second"));
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(format!(
                "first {}",
                if first.is_focused { "focused" } else { "idle" }
            ))
            .into_element(),
        )
        .child(
            Text::new(format!(
                "second {}",
                if second.is_focused { "focused" } else { "idle" }
            ))
            .into_element(),
        )
        .into_element()
}

fn resize_app() -> Element {
    Text::new("resize probe").into_element()
}

#[test]
fn harness_sends_keyboard_input_to_use_input_handlers() {
    let mut harness = TestHarness::new(keyboard_app);
    assert_eq!(harness.runtime_context().borrow().input_handler_count(), 1);

    harness.send_text("ab");
    harness.assert_text_contains("keyboard: ab");

    harness.send_key(KeyCodeKind::Enter);
    harness.assert_text_contains("keyboard: submitted");
}

#[test]
fn harness_sends_mouse_events_to_use_mouse_handlers() {
    let mut harness = TestHarness::new(mouse_app);
    assert!(harness.runtime_context().borrow().is_mouse_enabled());

    harness.send_mouse(Mouse {
        x: 3,
        y: 4,
        action: MouseAction::Press(MouseButton::Left),
        ctrl: false,
        shift: false,
        alt: false,
    });

    harness.assert_text_contains("mouse: left:3:4");
}

#[test]
fn harness_sends_paste_events_to_use_paste_handlers() {
    let mut harness = TestHarness::new(paste_app);
    assert_eq!(harness.runtime_context().borrow().paste_handler_count(), 1);

    harness.send_paste("alpha\nbeta");
    harness.assert_text_contains("paste: lines:2 text:alpha");
    harness.assert_text_contains("beta");
}

#[test]
fn harness_focus_helpers_and_tab_traversal_update_focus_state() {
    let mut harness = TestHarness::new(focus_app);
    harness.assert_text_contains("first focused");

    harness.focus_next();
    harness.assert_text_contains("second focused");

    harness.focus("first");
    harness.assert_text_contains("first focused");

    harness.send_key(KeyCodeKind::Tab);
    harness.assert_text_contains("second focused");
}

#[test]
fn harness_resize_updates_renderer_dimensions_and_rerenders() {
    let mut harness = TestHarness::with_size(resize_app, 40, 10);
    assert_eq!(harness.width(), 40);
    assert_eq!(harness.height(), 10);
    harness.assert_text_contains("resize probe");

    harness.resize(12, 4);
    assert_eq!(harness.width(), 12);
    assert_eq!(harness.height(), 4);
    harness.assert_text_contains("resize probe");
}
