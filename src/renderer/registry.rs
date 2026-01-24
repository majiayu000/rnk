//! Application registry for cross-thread communication
//!
//! This module provides a global registry that allows multiple apps to run
//! and enables cross-thread render requests via the AppSink trait.

use crate::core::Element;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// === App ID ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AppId(u64);

impl AppId {
    pub(crate) fn new() -> Self {
        Self(APP_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    pub(crate) fn from_raw(raw: u64) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub(crate) fn raw(self) -> u64 {
        self.0
    }
}

static APP_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

// === Printable ===

/// Printable content that can be sent to println
#[derive(Clone)]
pub enum Printable {
    /// Plain text message
    Text(String),
    /// Rendered element (boxed to reduce enum size)
    Element(Box<Element>),
}

/// Trait for types that can be printed via println
pub trait IntoPrintable {
    /// Convert into a printable value
    fn into_printable(self) -> Printable;
}

impl IntoPrintable for String {
    fn into_printable(self) -> Printable {
        Printable::Text(self)
    }
}

impl IntoPrintable for &str {
    fn into_printable(self) -> Printable {
        Printable::Text(self.to_string())
    }
}

impl IntoPrintable for Element {
    fn into_printable(self) -> Printable {
        Printable::Element(Box::new(self))
    }
}

// === App Sink ===

pub trait AppSink: Send + Sync {
    fn request_render(&self);
    fn println(&self, message: Printable);
    fn enter_alt_screen(&self);
    fn exit_alt_screen(&self);
    fn is_alt_screen(&self) -> bool;
}

// === Mode Switch ===

/// Mode switch direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeSwitch {
    /// Switch to alternate screen (fullscreen)
    EnterAltScreen,
    /// Switch to inline mode
    ExitAltScreen,
}

// === App Runtime ===

pub(crate) struct AppRuntime {
    id: AppId,
    render_flag: Arc<AtomicBool>,
    println_queue: Mutex<Vec<Printable>>,
    mode_switch_request: Mutex<Option<ModeSwitch>>,
    alt_screen_state: Arc<AtomicBool>,
}

impl AppRuntime {
    pub(crate) fn new(alternate_screen: bool) -> Arc<Self> {
        Arc::new(Self {
            id: AppId::new(),
            render_flag: Arc::new(AtomicBool::new(true)),
            println_queue: Mutex::new(Vec::new()),
            mode_switch_request: Mutex::new(None),
            alt_screen_state: Arc::new(AtomicBool::new(alternate_screen)),
        })
    }

    pub(crate) fn id(&self) -> AppId {
        self.id
    }

    pub(crate) fn set_alt_screen_state(&self, value: bool) {
        self.alt_screen_state.store(value, Ordering::SeqCst);
    }

    pub(crate) fn render_requested(&self) -> bool {
        self.render_flag.load(Ordering::SeqCst)
    }

    pub(crate) fn clear_render_request(&self) {
        self.render_flag.store(false, Ordering::SeqCst);
    }

    pub(crate) fn take_mode_switch_request(&self) -> Option<ModeSwitch> {
        match self.mode_switch_request.lock() {
            Ok(mut request) => request.take(),
            Err(poisoned) => {
                // Recover from poisoned mutex - still try to get the value
                poisoned.into_inner().take()
            }
        }
    }

    pub(crate) fn take_println_messages(&self) -> Vec<Printable> {
        match self.println_queue.lock() {
            Ok(mut queue) => std::mem::take(&mut *queue),
            Err(poisoned) => {
                // Recover from poisoned mutex - still try to get the messages
                std::mem::take(&mut *poisoned.into_inner())
            }
        }
    }
}

impl AppSink for AppRuntime {
    fn request_render(&self) {
        self.render_flag.store(true, Ordering::SeqCst);
    }

    fn println(&self, message: Printable) {
        match self.println_queue.lock() {
            Ok(mut queue) => queue.push(message),
            Err(poisoned) => poisoned.into_inner().push(message),
        }
        self.request_render();
    }

    fn enter_alt_screen(&self) {
        match self.mode_switch_request.lock() {
            Ok(mut request) => *request = Some(ModeSwitch::EnterAltScreen),
            Err(poisoned) => *poisoned.into_inner() = Some(ModeSwitch::EnterAltScreen),
        }
        self.request_render();
    }

    fn exit_alt_screen(&self) {
        match self.mode_switch_request.lock() {
            Ok(mut request) => *request = Some(ModeSwitch::ExitAltScreen),
            Err(poisoned) => *poisoned.into_inner() = Some(ModeSwitch::ExitAltScreen),
        }
        self.request_render();
    }

    fn is_alt_screen(&self) -> bool {
        self.alt_screen_state.load(Ordering::SeqCst)
    }
}

// === Global Registry ===

type AppRegistry = HashMap<AppId, Arc<dyn AppSink>>;

static APP_REGISTRY: OnceLock<Mutex<AppRegistry>> = OnceLock::new();
static CURRENT_APP: AtomicU64 = AtomicU64::new(0);

fn registry() -> &'static Mutex<AppRegistry> {
    APP_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn set_current_app(id: Option<AppId>) {
    let raw = id.map(|value| value.raw()).unwrap_or(0);
    CURRENT_APP.store(raw, Ordering::SeqCst);
}

pub(crate) fn current_app_sink() -> Option<Arc<dyn AppSink>> {
    let id = AppId::from_raw(CURRENT_APP.load(Ordering::SeqCst))?;
    let registry = registry().lock().ok()?;
    registry.get(&id).cloned()
}

pub(crate) struct AppRegistrationGuard {
    id: AppId,
}

impl Drop for AppRegistrationGuard {
    fn drop(&mut self) {
        unregister_app(self.id);
    }
}

pub(crate) fn register_app(runtime: Arc<AppRuntime>) -> AppRegistrationGuard {
    let id = runtime.id();
    if let Ok(mut registry) = registry().lock() {
        let sink: Arc<dyn AppSink> = runtime;
        registry.insert(id, sink);
        set_current_app(Some(id));
    }
    AppRegistrationGuard { id }
}

fn unregister_app(id: AppId) {
    if let Ok(mut registry) = registry().lock() {
        registry.remove(&id);
    }
    if AppId::from_raw(CURRENT_APP.load(Ordering::SeqCst)) == Some(id) {
        set_current_app(None);
    }
}

// === Public APIs ===

/// Request a re-render from any thread.
///
/// This is useful when state is updated from a background thread
/// and you need to notify the UI to refresh.
///
/// # Example
///
/// ```ignore
/// use std::thread;
/// use rnk::request_render;
///
/// // In a background thread
/// thread::spawn(|| {
///     // ... update some shared state ...
///     request_render(); // Notify rnk to re-render
/// });
/// ```
pub fn request_render() {
    if let Some(sink) = current_app_sink() {
        sink.request_render();
    }
}

/// Print a message that persists above the UI (like Bubbletea's Println).
///
/// In inline mode, this clears the current UI, writes the message,
/// and the UI is re-rendered below it. The message stays in terminal history.
///
/// In fullscreen mode, this is a no-op (messages are ignored, like Bubbletea).
///
/// **Fallback behavior**: If no rnk app is running, the message is printed
/// directly to stdout (using `render_to_string_auto` for Elements).
///
/// Supports both plain text and rendered elements:
///
/// # Examples
///
/// ```ignore
/// use rnk::println;
///
/// // Simple text
/// rnk::println("Task completed successfully!");
/// rnk::println(format!("Downloaded {} files", count));
///
/// // Complex components
/// let banner = Box::new()
///     .border_style(BorderStyle::Round)
///     .padding(1)
///     .child(Text::new("Welcome!").bold().into_element())
///     .into_element();
/// rnk::println(banner);
/// ```
pub fn println(message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println(message.into_printable());
        return;
    }

    // No app running, print directly as fallback
    use crate::renderer::render_to_string_auto;
    let printable = message.into_printable();
    let output = match printable {
        Printable::Text(text) => text,
        Printable::Element(element) => render_to_string_auto(&element),
    };
    std::println!("{}", output);
}

/// Print a message with trailing whitespace trimmed (convenience wrapper).
///
/// This is useful for printing single-line messages where you want to
/// avoid extra blank lines in the terminal history.
///
/// # Example
///
/// ```ignore
/// use rnk::println_trimmed;
///
/// println_trimmed("Task completed!");
/// ```
pub fn println_trimmed(message: impl IntoPrintable) {
    let printable = message.into_printable();
    match printable {
        Printable::Text(text) => println(text.trim_end()),
        Printable::Element(element) => println(*element),
    }
}

/// Request to enter alternate screen mode (fullscreen).
///
/// This can be called from any thread. The mode switch happens
/// on the next render cycle.
///
/// # Example
///
/// ```ignore
/// use rnk::enter_alt_screen;
///
/// // Switch to fullscreen mode
/// enter_alt_screen();
/// ```
pub fn enter_alt_screen() {
    if let Some(sink) = current_app_sink() {
        sink.enter_alt_screen();
    }
}

/// Request to exit alternate screen mode (return to inline).
///
/// This can be called from any thread. The mode switch happens
/// on the next render cycle.
///
/// # Example
///
/// ```ignore
/// use rnk::exit_alt_screen;
///
/// // Return to inline mode
/// exit_alt_screen();
/// ```
pub fn exit_alt_screen() {
    if let Some(sink) = current_app_sink() {
        sink.exit_alt_screen();
    }
}

/// Check if currently in alternate screen mode.
///
/// Returns `None` if no app is running.
pub fn is_alt_screen() -> Option<bool> {
    current_app_sink().map(|sink| sink.is_alt_screen())
}

// === Render Handle ===

/// A handle for requesting renders from any thread.
///
/// This is a cloneable, thread-safe handle that can be used to trigger
/// re-renders from background threads or async tasks.
///
/// # Example
///
/// ```ignore
/// use std::thread;
/// use rnk::render_handle;
///
/// let handle = render_handle().expect("App must be running");
///
/// thread::spawn(move || {
///     // ... do some work ...
///     handle.request_render();
/// });
/// ```
#[derive(Clone)]
pub struct RenderHandle {
    sink: Arc<dyn AppSink>,
}

impl RenderHandle {
    pub(crate) fn new(sink: Arc<dyn AppSink>) -> Self {
        Self { sink }
    }

    /// Request a re-render
    pub fn request_render(&self) {
        self.sink.request_render();
    }

    /// Print a message that persists above the UI
    pub fn println(&self, message: impl IntoPrintable) {
        self.sink.println(message.into_printable());
    }

    /// Request to enter fullscreen mode
    pub fn enter_alt_screen(&self) {
        self.sink.enter_alt_screen();
    }

    /// Request to exit fullscreen mode
    pub fn exit_alt_screen(&self) {
        self.sink.exit_alt_screen();
    }

    /// Check if currently in fullscreen mode
    pub fn is_alt_screen(&self) -> bool {
        self.sink.is_alt_screen()
    }
}

/// Get a render handle for cross-thread render requests.
///
/// Returns `None` if no app is currently running.
pub fn render_handle() -> Option<RenderHandle> {
    current_app_sink().map(RenderHandle::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id_counter() {
        let id1 = AppId::new();
        let id2 = AppId::new();
        assert_ne!(id1, id2);
        assert!(id2.raw() > id1.raw());
    }

    #[test]
    fn test_app_id_from_raw() {
        assert_eq!(AppId::from_raw(0), None);
        let id = AppId::from_raw(42).unwrap();
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn test_printable_text() {
        let p = "hello".into_printable();
        match p {
            Printable::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_printable_string() {
        let p = String::from("world").into_printable();
        match p {
            Printable::Text(text) => assert_eq!(text, "world"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_app_runtime_creation() {
        let runtime = AppRuntime::new(false);
        assert!(!runtime.is_alt_screen());
        assert!(runtime.render_requested()); // Initial render requested
    }

    #[test]
    fn test_app_runtime_alt_screen() {
        let runtime = AppRuntime::new(true);
        assert!(runtime.is_alt_screen());

        runtime.set_alt_screen_state(false);
        assert!(!runtime.is_alt_screen());
    }

    #[test]
    fn test_app_runtime_render_flag() {
        let runtime = AppRuntime::new(false);
        assert!(runtime.render_requested());

        runtime.clear_render_request();
        assert!(!runtime.render_requested());

        runtime.request_render();
        assert!(runtime.render_requested());
    }

    #[test]
    fn test_app_runtime_println() {
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
    fn test_println_fallback() {
        // When no app is running, println should not panic
        println("test message");
        println(String::from("another test"));
    }

    #[test]
    fn test_cross_thread_apis() {
        // These should not panic when no app is running
        request_render();
        enter_alt_screen();
        exit_alt_screen();
        assert_eq!(is_alt_screen(), None);
    }

    #[test]
    fn test_render_handle() {
        let runtime = AppRuntime::new(false);
        let _guard = register_app(runtime.clone());

        let handle = render_handle().expect("Should get handle");
        handle.request_render();
        assert!(runtime.render_requested());

        runtime.clear_render_request();
        handle.println("test");
        assert!(runtime.render_requested());
    }
}
