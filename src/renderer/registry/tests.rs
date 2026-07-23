use super::*;
use crate::cmd::ExecConfig;
use crate::runtime::{RuntimeContext, set_current_runtime};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn runtime_context_with_handle(runtime: Arc<AppRuntime>) -> Rc<RefCell<RuntimeContext>> {
    Rc::new(RefCell::new(RuntimeContext::with_app_control(
        Arc::new(AtomicBool::new(false)),
        RenderHandle::new(runtime),
    )))
}

#[test]
fn test_app_id_counter() {
    let _registry_guard = lock_test_registry();
    let id1 = AppId::new();
    let id2 = AppId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_app_id_from_raw() {
    let _registry_guard = lock_test_registry();
    assert_eq!(AppId::from_raw(0), None);
    let id = AppId::from_raw(42).unwrap();
    assert_eq!(id.raw(), 42);
}

#[test]
fn test_printable_text() {
    let _registry_guard = lock_test_registry();
    let p = "hello".into_printable();
    match p {
        Printable::Text(text) => assert_eq!(text, "hello"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_printable_string() {
    let _registry_guard = lock_test_registry();
    let p = String::from("world").into_printable();
    match p {
        Printable::Text(text) => assert_eq!(text, "world"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_app_runtime_creation() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    assert!(!runtime.is_alt_screen());
    assert!(runtime.render_requested()); // Initial render requested
}

#[test]
fn test_app_runtime_alt_screen() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(true);
    assert!(runtime.is_alt_screen());

    runtime.set_alt_screen_state(false);
    assert!(!runtime.is_alt_screen());
}

#[test]
fn test_app_runtime_render_flag() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    assert!(runtime.render_requested());

    runtime.clear_render_request();
    assert!(!runtime.render_requested());

    runtime.request_render();
    assert!(runtime.render_requested());
}

#[test]
fn test_app_runtime_println() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    runtime.println(Printable::Text("test".to_string()));

    let messages = runtime.take_println_messages();
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        Printable::Text(text) => assert_eq!(text, "test"),
        _ => panic!("Expected Text"),
    }

    // Second take should be empty
    let messages2 = runtime.take_println_messages();
    assert_eq!(messages2.len(), 0);
}

#[test]
fn test_app_runtime_mode_switch() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    runtime.enter_alt_screen();

    let switch = runtime.take_mode_switch_request();
    assert_eq!(switch, Some(ModeSwitch::EnterAltScreen));

    // Second take should be None
    let switch2 = runtime.take_mode_switch_request();
    assert_eq!(switch2, None);
}

#[test]
fn test_registry_operations() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    let guard = register_app(runtime.clone());

    // Should be able to get current app
    let sink = current_app_sink();
    assert!(sink.is_some());

    // Trigger render
    request_render();
    assert!(runtime.render_requested());

    // Clean up
    drop(guard);

    // Should no longer be able to get current app
    let sink2 = current_app_sink();
    assert!(sink2.is_none());
}

#[test]
fn test_unregister_current_app_falls_back_to_previous() {
    let _registry_guard = lock_test_registry();
    let runtime1 = AppRuntime::new(false);
    runtime1.clear_render_request();
    let guard1 = register_app(runtime1.clone());

    let runtime2 = AppRuntime::new(false);
    runtime2.clear_render_request();
    let guard2 = register_app(runtime2.clone());

    request_render();
    assert!(runtime2.render_requested());
    assert!(!runtime1.render_requested());

    runtime1.clear_render_request();
    runtime2.clear_render_request();
    drop(guard2);

    request_render();
    assert!(runtime1.render_requested());

    drop(guard1);
}

#[test]
fn test_app_id_recycled_after_unregister() {
    let _registry_guard = lock_test_registry();
    let runtime1 = AppRuntime::new(false);
    let id1 = runtime1.id();
    let guard1 = register_app(runtime1);
    drop(guard1);

    let runtime2 = AppRuntime::new(false);
    let id2 = runtime2.id();
    let guard2 = register_app(runtime2);
    drop(guard2);

    assert_eq!(id1, id2);
}

#[test]
fn test_println_fallback() {
    let _registry_guard = lock_test_registry();
    // When no app is running, println should not panic
    println("test message");
    println(String::from("another test"));
}

#[test]
fn test_cross_thread_apis() {
    let _registry_guard = lock_test_registry();
    // These should not panic when no app is running
    request_render();
    enter_alt_screen();
    exit_alt_screen();
    assert_eq!(is_alt_screen(), None);
}

#[test]
fn test_render_handle() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    let _guard = register_app(runtime.clone());

    let handle = render_handle().expect("Should get handle");
    handle.request_render();
    assert!(runtime.render_requested());

    runtime.clear_render_request();
    handle.println("test");
    assert!(runtime.render_requested());
}

#[test]
fn test_request_render_uses_runtime_handle_without_registry() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    runtime.clear_render_request();
    set_current_runtime(Some(runtime_context_with_handle(runtime.clone())));

    request_render();
    assert!(runtime.render_requested());

    set_current_runtime(None);
}

#[test]
fn test_println_uses_runtime_handle_without_registry() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    set_current_runtime(Some(runtime_context_with_handle(runtime.clone())));

    println("runtime scoped");
    let messages = runtime.take_println_messages();
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        Printable::Text(text) => assert_eq!(text, "runtime scoped"),
        _ => panic!("Expected Text"),
    }

    set_current_runtime(None);
}

#[test]
fn test_is_alt_screen_uses_runtime_handle_without_registry() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(true);
    set_current_runtime(Some(runtime_context_with_handle(runtime)));

    assert_eq!(is_alt_screen(), Some(true));

    set_current_runtime(None);
}

#[test]
fn test_queue_exec_request_uses_runtime_handle_without_registry() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    runtime.clear_render_request();
    set_current_runtime(Some(runtime_context_with_handle(runtime.clone())));

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();
    queue_exec_request(ExecRequest {
        config: ExecConfig::new("echo").arg("hello"),
        callback: Box::new(move |_| {
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
    });

    assert!(runtime.render_requested());
    assert_eq!(runtime.take_exec_requests().len(), 1);
    assert!(!callback_called.load(Ordering::SeqCst));

    set_current_runtime(None);
}

#[test]
fn test_queue_terminal_cmd_uses_runtime_handle_without_registry() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);
    runtime.clear_render_request();
    set_current_runtime(Some(runtime_context_with_handle(runtime.clone())));

    queue_terminal_cmd(TerminalCmd::HideCursor);

    assert!(runtime.render_requested());
    assert_eq!(runtime.take_terminal_cmds(), vec![TerminalCmd::HideCursor]);

    set_current_runtime(None);
}
