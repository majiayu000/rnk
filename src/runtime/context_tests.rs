use super::{RuntimeContext, current_runtime, with_runtime};
use crate::core::{ElementId, NodeKey};
use crate::layout::Layout;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[test]
fn test_runtime_context_creation() {
    let ctx = RuntimeContext::new();
    assert!(!ctx.should_exit());
    assert!(!ctx.is_mouse_enabled());
    assert!(!ctx.is_screen_reader_enabled());
}

#[test]
fn test_runtime_context_exit() {
    let ctx = RuntimeContext::new();
    assert!(!ctx.should_exit());
    ctx.exit();
    assert!(ctx.should_exit());
}

#[test]
fn test_runtime_context_input_handlers() {
    let mut ctx = RuntimeContext::new();
    assert_eq!(ctx.input_handler_count(), 0);

    ctx.register_input_handler(|_, _| {});
    assert_eq!(ctx.input_handler_count(), 1);

    ctx.register_input_handler(|_, _| {});
    assert_eq!(ctx.input_handler_count(), 2);
}

#[test]
fn test_runtime_context_mouse_enabled() {
    let mut ctx = RuntimeContext::new();
    assert!(!ctx.is_mouse_enabled());

    ctx.register_mouse_handler(|_| {});
    assert!(ctx.is_mouse_enabled());
}

#[test]
fn test_runtime_context_measurements() {
    let mut ctx = RuntimeContext::new();
    let id = ElementId::new();
    assert!(ctx.get_measurement(id).is_none());

    ctx.set_measurement(id, 80, 24);
    assert_eq!(ctx.get_measurement(id), Some((80, 24)));
}

#[test]
fn test_runtime_context_measurements_by_key() {
    let mut ctx = RuntimeContext::new();
    let id = ElementId::new();
    let mut by_id = HashMap::new();
    by_id.insert(
        id,
        Layout {
            x: 0.0,
            y: 0.0,
            width: 42.0,
            height: 9.0,
        },
    );

    let mut by_key = HashMap::new();
    by_key.insert(
        "main-panel".to_string(),
        Layout {
            x: 0.0,
            y: 0.0,
            width: 42.0,
            height: 9.0,
        },
    );

    ctx.set_measure_layouts_with_keys(by_id, by_key);
    assert_eq!(ctx.get_measurement(id), Some((42, 9)));
    assert_eq!(
        ctx.get_measurement_by_key_dims("main-panel"),
        Some((42.0, 9.0))
    );
}

#[test]
fn test_runtime_context_measurements_by_node_key_and_alias() {
    let mut ctx = RuntimeContext::new();
    let id = ElementId::new();
    let node_key = NodeKey::with_key("main-panel", TypeId::of::<i32>(), 0);

    let mut by_id = HashMap::new();
    by_id.insert(
        id,
        Layout {
            x: 0.0,
            y: 0.0,
            width: 42.0,
            height: 9.0,
        },
    );

    let mut by_node_key = HashMap::new();
    by_node_key.insert(
        node_key,
        Layout {
            x: 0.0,
            y: 0.0,
            width: 42.0,
            height: 9.0,
        },
    );

    let mut aliases = HashMap::new();
    aliases.insert("main-panel".to_string(), node_key);

    ctx.set_measure_layouts_with_node_keys(by_id, by_node_key, aliases);
    assert_eq!(
        ctx.get_measurement_by_node_key_dims(node_key),
        Some((42.0, 9.0))
    );
    assert_eq!(
        ctx.resolve_measurement_key_alias("main-panel"),
        Some(node_key)
    );
    assert_eq!(
        ctx.get_measurement_by_key_dims("main-panel"),
        Some((42.0, 9.0))
    );
}

#[test]
fn test_runtime_context_context_values_nested() {
    let mut ctx = RuntimeContext::new();
    ctx.push_context_value(7, "outer".to_string());
    assert_eq!(ctx.context_value::<String>(7).as_deref(), Some("outer"));

    ctx.push_context_value(7, "inner".to_string());
    assert_eq!(ctx.context_value::<String>(7).as_deref(), Some("inner"));

    ctx.pop_context_value(7);
    assert_eq!(ctx.context_value::<String>(7).as_deref(), Some("outer"));

    ctx.pop_context_value(7);
    assert_eq!(ctx.context_value::<String>(7), None);
}

#[test]
fn test_with_runtime() {
    let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

    let result = with_runtime(ctx.clone(), || {
        let runtime = current_runtime().unwrap();
        runtime.borrow_mut().register_input_handler(|_, _| {});
        runtime.borrow().input_handler_count()
    });

    assert_eq!(result, 1);
    assert!(current_runtime().is_none());
}

#[test]
fn test_hook_state_persistence() {
    let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

    with_runtime(ctx.clone(), || {
        let runtime = current_runtime().unwrap();
        let hook = runtime.borrow_mut().use_hook(|| 42i32);
        assert_eq!(hook.get::<i32>(), Some(42));
        hook.set(100i32);
    });

    with_runtime(ctx.clone(), || {
        let runtime = current_runtime().unwrap();
        let hook = runtime.borrow_mut().use_hook(|| 0i32);
        assert_eq!(hook.get::<i32>(), Some(100));
    });
}

#[test]
fn test_handlers_cleared_on_render() {
    let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

    with_runtime(ctx.clone(), || {
        let runtime = current_runtime().unwrap();
        runtime.borrow_mut().register_input_handler(|_, _| {});
        runtime.borrow_mut().register_input_handler(|_, _| {});
        assert_eq!(runtime.borrow().input_handler_count(), 2);
    });

    with_runtime(ctx.clone(), || {
        let runtime = current_runtime().unwrap();
        assert_eq!(runtime.borrow().input_handler_count(), 0);
        runtime.borrow_mut().register_input_handler(|_, _| {});
        assert_eq!(runtime.borrow().input_handler_count(), 1);
    });
}
