//! Test harness for simulating user interactions
//!
//! Provides a way to test components with simulated keyboard input,
//! mouse events, and assertions on rendered output.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::testing::TestHarness;
//! use rnk::prelude::*;
//!
//! fn simple_text() -> Element {
//!     Text::new("Hello, World!").into_element()
//! }
//!
//! #[test]
//! fn test_simple() {
//!     let harness = TestHarness::new(simple_text);
//!     harness.assert_text_contains("Hello");
//! }
//! ```

use crate::core::Element;
use crate::hooks::use_input::{Key, KeyCodeKind, MediaKeyKind};
use crate::hooks::use_mouse::Mouse;
use crate::runtime::{RuntimeContext, current_runtime, set_current_runtime, with_runtime};
use crate::testing::TestRenderer;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MediaKeyCode};
use std::cell::RefCell;
use std::rc::Rc;

/// Test harness for interactive component testing
pub struct TestHarness<F>
where
    F: Fn() -> Element,
{
    /// Component function
    component: F,
    /// Runtime context for hook state and event handlers.
    runtime: Rc<RefCell<RuntimeContext>>,
    /// Test renderer
    renderer: TestRenderer,
    /// Last rendered output
    last_output: String,
}

impl<F> TestHarness<F>
where
    F: Fn() -> Element,
{
    /// Create a new test harness for a component
    pub fn new(component: F) -> Self {
        Self::with_size(component, 80, 24)
    }

    /// Create a test harness with custom terminal size
    pub fn with_size(component: F, width: u16, height: u16) -> Self {
        let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
        let renderer = TestRenderer::new(width, height);

        let mut harness = Self {
            component,
            runtime,
            renderer,
            last_output: String::new(),
        };

        // Initial render
        harness.render();
        harness
    }

    /// Render the component and update last_output
    pub fn render(&mut self) -> &str {
        let element = with_runtime(self.runtime.clone(), || (self.component)());
        self.last_output = self.renderer.render_to_plain(&element);
        &self.last_output
    }

    /// Get the last rendered output (plain text)
    pub fn output(&self) -> &str {
        &self.last_output
    }

    /// Get the last rendered output with ANSI codes
    pub fn output_ansi(&mut self) -> String {
        let element = with_runtime(self.runtime.clone(), || (self.component)());
        self.renderer.render_to_ansi(&element)
    }

    /// Re-render and return the new output
    pub fn update(&mut self) -> &str {
        self.render()
    }

    /// Get the runtime context used by this harness.
    pub fn runtime_context(&self) -> Rc<RefCell<RuntimeContext>> {
        self.runtime.clone()
    }

    /// Dispatch a typed key through registered `use_input` handlers and render.
    pub fn send_key(&mut self, code: KeyCodeKind) -> &str {
        self.send_key_with_modifiers(code, KeyModifiers::NONE)
    }

    /// Dispatch a typed key with modifiers through registered `use_input`
    /// handlers and render.
    pub fn send_key_with_modifiers(&mut self, code: KeyCodeKind, modifiers: KeyModifiers) -> &str {
        let event = KeyEvent::new(key_code_to_event_code(code), modifiers);
        self.send_key_event(event)
    }

    /// Dispatch a raw crossterm key event through registered `use_input`
    /// handlers and render.
    pub fn send_key_event(&mut self, event: KeyEvent) -> &str {
        let key = Key::from_event(&event);
        let input = Key::char_from_event(&event);
        self.dispatch_key(&input, &key)
    }

    /// Dispatch an already-built key plus input string and render.
    pub fn dispatch_key(&mut self, input: &str, key: &Key) -> &str {
        self.with_current_runtime(|| {
            crate::hooks::use_input::dispatch_input(input, key);
        });
        self.render()
    }

    /// Dispatch text as a sequence of character key events and render once.
    pub fn send_text(&mut self, text: &str) -> &str {
        let handlers = self.runtime.borrow().input_handlers.clone();
        self.with_current_runtime(|| {
            for ch in text.chars() {
                let event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
                let key = Key::from_event(&event);
                let input = Key::char_from_event(&event);
                for handler in &handlers {
                    handler(&input, &key);
                }
            }
        });
        self.render()
    }

    /// Dispatch a mouse event through registered `use_mouse` handlers and render.
    pub fn send_mouse(&mut self, mouse: Mouse) -> &str {
        let handlers = self.runtime.borrow().mouse_handlers.clone();
        self.with_current_runtime(|| {
            for handler in handlers {
                handler(&mouse);
            }
        });
        self.render()
    }

    /// Dispatch paste content through registered `use_paste` handlers and render.
    pub fn send_paste(&mut self, content: &str) -> &str {
        self.with_current_runtime(|| {
            crate::hooks::paste::dispatch_paste(content);
        });
        self.render()
    }

    /// Resize the test renderer and render at the new dimensions.
    pub fn resize(&mut self, width: u16, height: u16) -> &str {
        self.renderer = TestRenderer::new(width, height);
        self.render()
    }

    /// Move focus to the next registered focusable element and render.
    pub fn focus_next(&mut self) -> &str {
        self.render();
        self.runtime.borrow_mut().focus_manager_mut().focus_next();
        self.render()
    }

    /// Move focus to the previous registered focusable element and render.
    pub fn focus_previous(&mut self) -> &str {
        self.render();
        self.runtime
            .borrow_mut()
            .focus_manager_mut()
            .focus_previous();
        self.render()
    }

    /// Focus a registered focusable element by custom ID and render.
    pub fn focus(&mut self, id: &str) -> &str {
        self.render();
        self.runtime.borrow_mut().focus_manager_mut().focus(id);
        self.render()
    }

    fn with_current_runtime<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = current_runtime();
        set_current_runtime(Some(self.runtime.clone()));

        struct RuntimeGuard {
            previous: Option<Rc<RefCell<RuntimeContext>>>,
        }

        impl Drop for RuntimeGuard {
            fn drop(&mut self) {
                set_current_runtime(self.previous.take());
            }
        }

        let guard = RuntimeGuard { previous };
        let result = f();
        drop(guard);
        result
    }

    // ========== Assertions ==========

    /// Assert that the output contains the given text
    pub fn assert_text_contains(&self, expected: &str) {
        assert!(
            self.last_output.contains(expected),
            "Expected output to contain '{}', but got:\n{}",
            expected,
            self.last_output
        );
    }

    /// Assert that the output does not contain the given text
    pub fn assert_text_not_contains(&self, unexpected: &str) {
        assert!(
            !self.last_output.contains(unexpected),
            "Expected output to NOT contain '{}', but got:\n{}",
            unexpected,
            self.last_output
        );
    }

    /// Assert that the output equals the given text (trimmed)
    pub fn assert_text_equals(&self, expected: &str) {
        let actual = self.last_output.trim();
        let expected = expected.trim();
        assert_eq!(
            actual, expected,
            "Expected output to equal '{}', but got '{}'",
            expected, actual
        );
    }

    /// Assert that the output matches a pattern (line by line)
    pub fn assert_lines_contain(&self, patterns: &[&str]) {
        let lines: Vec<&str> = self.last_output.lines().collect();
        for pattern in patterns {
            assert!(
                lines.iter().any(|line| line.contains(pattern)),
                "Expected a line containing '{}', but none found in:\n{}",
                pattern,
                self.last_output
            );
        }
    }

    /// Assert the number of lines in output
    pub fn assert_line_count(&self, expected: usize) {
        let actual = self.last_output.lines().count();
        assert_eq!(
            actual, expected,
            "Expected {} lines, but got {}",
            expected, actual
        );
    }

    /// Get a specific line from the output (0-indexed)
    pub fn line(&self, index: usize) -> Option<&str> {
        self.last_output.lines().nth(index)
    }

    /// Get all lines from the output
    pub fn lines(&self) -> Vec<&str> {
        self.last_output.lines().collect()
    }

    /// Check if output contains text (returns bool instead of asserting)
    pub fn contains(&self, text: &str) -> bool {
        self.last_output.contains(text)
    }

    /// Get the width of the test terminal
    pub fn width(&self) -> u16 {
        self.renderer.width()
    }

    /// Get the height of the test terminal
    pub fn height(&self) -> u16 {
        self.renderer.height()
    }
}

fn key_code_to_event_code(code: KeyCodeKind) -> KeyCode {
    match code {
        KeyCodeKind::Up => KeyCode::Up,
        KeyCodeKind::Down => KeyCode::Down,
        KeyCodeKind::Left => KeyCode::Left,
        KeyCodeKind::Right => KeyCode::Right,
        KeyCodeKind::PageUp => KeyCode::PageUp,
        KeyCodeKind::PageDown => KeyCode::PageDown,
        KeyCodeKind::Home => KeyCode::Home,
        KeyCodeKind::End => KeyCode::End,
        KeyCodeKind::Insert => KeyCode::Insert,
        KeyCodeKind::Enter => KeyCode::Enter,
        KeyCodeKind::Escape => KeyCode::Esc,
        KeyCodeKind::Tab => KeyCode::Tab,
        KeyCodeKind::BackTab => KeyCode::BackTab,
        KeyCodeKind::Backspace => KeyCode::Backspace,
        KeyCodeKind::Delete => KeyCode::Delete,
        KeyCodeKind::Char(ch) => KeyCode::Char(ch),
        KeyCodeKind::Function(n) => KeyCode::F(n),
        KeyCodeKind::Media(media) => KeyCode::Media(match media {
            MediaKeyKind::Play => MediaKeyCode::Play,
            MediaKeyKind::Pause => MediaKeyCode::Pause,
            MediaKeyKind::PlayPause => MediaKeyCode::PlayPause,
            MediaKeyKind::Stop => MediaKeyCode::Stop,
            MediaKeyKind::Next => MediaKeyCode::TrackNext,
            MediaKeyKind::Previous => MediaKeyCode::TrackPrevious,
            MediaKeyKind::VolumeUp => MediaKeyCode::RaiseVolume,
            MediaKeyKind::VolumeDown => MediaKeyCode::LowerVolume,
            MediaKeyKind::VolumeMute => MediaKeyCode::MuteVolume,
        }),
        KeyCodeKind::Unknown => KeyCode::Null,
    }
}

/// Snapshot testing support
pub struct Snapshot {
    name: String,
    content: String,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }

    /// Assert that the content matches the snapshot
    pub fn assert_match(&self, actual: &str) {
        let actual = actual.trim();
        let expected = self.content.trim();
        assert_eq!(
            actual, expected,
            "Snapshot '{}' mismatch.\nExpected:\n{}\n\nActual:\n{}",
            self.name, expected, actual
        );
    }
}

/// Golden file testing for string snapshot comparisons
pub struct StringSnapshot {
    name: String,
    directory: String,
}

impl StringSnapshot {
    /// Create a new golden test with default directory
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            directory: "tests/golden".to_string(),
        }
    }

    /// Set custom directory for golden files
    pub fn directory(mut self, dir: impl Into<String>) -> Self {
        self.directory = dir.into();
        self
    }

    /// Get the path to the golden file
    pub fn path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.directory).join(format!("{}.golden", self.name))
    }

    /// Assert that the actual output matches the golden file
    /// If UPDATE_GOLDEN=1 env var is set, updates the golden file instead
    pub fn assert_match(&self, actual: &str) {
        let path = self.path();
        let actual = actual.trim();

        // Check if we should update golden files
        if std::env::var("UPDATE_GOLDEN").is_ok() {
            // Create directory if needed
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&path, actual).expect("Failed to write golden file");
            return;
        }

        // Read expected content
        let expected = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                panic!(
                    "Golden file not found: {:?}\nRun with UPDATE_GOLDEN=1 to create it.\nActual output:\n{}",
                    path, actual
                );
            }
        };

        let expected = expected.trim();
        assert_eq!(
            actual, expected,
            "Golden test '{}' mismatch.\nExpected (from {:?}):\n{}\n\nActual:\n{}\n\nRun with UPDATE_GOLDEN=1 to update.",
            self.name, path, expected, actual
        );
    }

    /// Assert that the element's rendered output matches the golden file
    pub fn assert_element_match(&self, element: &Element, width: u16) {
        let renderer = TestRenderer::new(width, 100);
        let output = renderer.render_to_plain(element);
        self.assert_match(&output);
    }
}

/// Inline snapshot for embedding expected output in tests
#[macro_export]
macro_rules! inline_snapshot {
    ($actual:expr, $expected:expr) => {{
        let actual = $actual.trim();
        let expected = $expected.trim();
        assert_eq!(
            actual, expected,
            "Inline snapshot mismatch.\nExpected:\n{}\n\nActual:\n{}",
            expected, actual
        );
    }};
}

/// Create a snapshot assertion
#[macro_export]
macro_rules! assert_snapshot {
    ($name:expr, $actual:expr) => {{
        let snapshot = $crate::testing::Snapshot::new($name, $actual);
        snapshot.assert_match($actual);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;

    fn simple_component() -> Element {
        Text::new("Hello, World!").into_element()
    }

    #[test]
    fn test_harness_creation() {
        let harness = TestHarness::new(simple_component);
        assert!(harness.output().contains("Hello, World!"));
    }

    #[test]
    fn test_assert_text_contains() {
        let harness = TestHarness::new(simple_component);
        harness.assert_text_contains("Hello");
        harness.assert_text_contains("World");
    }

    #[test]
    fn test_assert_text_not_contains() {
        let harness = TestHarness::new(simple_component);
        harness.assert_text_not_contains("Goodbye");
    }

    #[test]
    fn test_lines() {
        fn multi_line() -> Element {
            use crate::components::Box as RnkBox;
            use crate::core::FlexDirection;

            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .child(Text::new("Line 1").into_element())
                .child(Text::new("Line 2").into_element())
                .child(Text::new("Line 3").into_element())
                .into_element()
        }

        let harness = TestHarness::new(multi_line);
        harness.assert_lines_contain(&["Line 1", "Line 2", "Line 3"]);
    }

    #[test]
    fn test_custom_size() {
        let harness = TestHarness::with_size(simple_component, 40, 10);
        assert!(harness.output().contains("Hello"));
    }

    #[test]
    fn test_contains() {
        let harness = TestHarness::new(simple_component);
        assert!(harness.contains("Hello"));
        assert!(!harness.contains("Goodbye"));
    }
}
