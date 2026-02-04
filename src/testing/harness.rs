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
use crate::hooks::context::{HookContext, with_hooks};
use crate::testing::TestRenderer;
use std::sync::{Arc, RwLock};

/// Test harness for interactive component testing
pub struct TestHarness<F>
where
    F: Fn() -> Element,
{
    /// Component function
    component: F,
    /// Hook context for state persistence
    context: Arc<RwLock<HookContext>>,
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
        let context = Arc::new(RwLock::new(HookContext::new()));
        let renderer = TestRenderer::new(width, height);

        let mut harness = Self {
            component,
            context,
            renderer,
            last_output: String::new(),
        };

        // Initial render
        harness.render();
        harness
    }

    /// Render the component and update last_output
    pub fn render(&mut self) -> &str {
        let element = with_hooks(self.context.clone(), || (self.component)());
        self.last_output = self.renderer.render_to_plain(&element);
        &self.last_output
    }

    /// Get the last rendered output (plain text)
    pub fn output(&self) -> &str {
        &self.last_output
    }

    /// Get the last rendered output with ANSI codes
    pub fn output_ansi(&mut self) -> String {
        let element = with_hooks(self.context.clone(), || (self.component)());
        self.renderer.render_to_ansi(&element)
    }

    /// Re-render and return the new output
    pub fn update(&mut self) -> &str {
        self.render()
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

/// Golden file testing for snapshot comparisons
pub struct GoldenTest {
    name: String,
    directory: String,
}

impl GoldenTest {
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
