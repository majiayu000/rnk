//! Golden file (snapshot) testing support
//!
//! Provides utilities for comparing rendered output against
//! saved "golden" reference files.

use std::fs;
use std::path::PathBuf;

use crate::core::Element;
use super::renderer::TestRenderer;

/// Directory for golden files
const GOLDEN_DIR: &str = "tests/golden";

/// Result of a golden file comparison
#[derive(Debug)]
pub enum GoldenResult {
    /// Output matches the golden file
    Match,
    /// Golden file doesn't exist (first run)
    Created,
    /// Output differs from golden file
    Mismatch {
        expected: String,
        actual: String,
        diff: String,
    },
}

/// Golden file test context
pub struct GoldenTest {
    name: String,
    width: u16,
    height: u16,
}

impl GoldenTest {
    /// Create a new golden test with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            width: 80,
            height: 24,
        }
    }

    /// Set terminal dimensions for this test
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Get the path to the golden file
    fn golden_path(&self) -> PathBuf {
        PathBuf::from(GOLDEN_DIR).join(format!("{}.txt", self.name))
    }

    /// Compare element output against golden file
    pub fn assert_match(&self, element: &Element) {
        let result = self.compare(element);
        match result {
            GoldenResult::Match => {}
            GoldenResult::Created => {
                println!("Golden file created: {}", self.golden_path().display());
            }
            GoldenResult::Mismatch { expected, actual, diff } => {
                panic!(
                    "\n\nGolden file mismatch for '{}':\n\n{}\n\nExpected:\n{}\n\nActual:\n{}\n",
                    self.name, diff, expected, actual
                );
            }
        }
    }

    /// Compare element output against golden file (without panic)
    pub fn compare(&self, element: &Element) -> GoldenResult {
        let renderer = TestRenderer::new(self.width, self.height);
        let actual = renderer.render_to_plain(element);

        let golden_path = self.golden_path();

        if !golden_path.exists() {
            // Create directory if needed
            if let Some(parent) = golden_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            // Write the golden file
            let _ = fs::write(&golden_path, &actual);
            return GoldenResult::Created;
        }

        let expected = fs::read_to_string(&golden_path)
            .unwrap_or_else(|_| String::new());

        if actual == expected {
            GoldenResult::Match
        } else {
            GoldenResult::Mismatch {
                expected: expected.clone(),
                actual: actual.clone(),
                diff: simple_diff(&expected, &actual),
            }
        }
    }

    /// Update the golden file with new output
    pub fn update(&self, element: &Element) -> std::io::Result<()> {
        let renderer = TestRenderer::new(self.width, self.height);
        let output = renderer.render_to_plain(element);

        let golden_path = self.golden_path();
        if let Some(parent) = golden_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(golden_path, output)
    }
}

/// Generate a simple diff between two strings
fn simple_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let mut diff = String::new();
    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let exp_line = expected_lines.get(i).copied().unwrap_or("");
        let act_line = actual_lines.get(i).copied().unwrap_or("");

        if exp_line != act_line {
            diff.push_str(&format!("Line {}:\n", i + 1));
            diff.push_str(&format!("  - {}\n", exp_line));
            diff.push_str(&format!("  + {}\n", act_line));
        }
    }

    if diff.is_empty() {
        if expected_lines.len() != actual_lines.len() {
            diff = format!(
                "Line count differs: expected {}, got {}",
                expected_lines.len(),
                actual_lines.len()
            );
        }
    }

    diff
}

/// Macro for creating golden tests
#[macro_export]
macro_rules! golden_test {
    ($name:ident, $element:expr) => {
        #[test]
        fn $name() {
            let golden = $crate::testing::GoldenTest::new(stringify!($name));
            golden.assert_match(&$element);
        }
    };

    ($name:ident, $width:expr, $height:expr, $element:expr) => {
        #[test]
        fn $name() {
            let golden = $crate::testing::GoldenTest::new(stringify!($name))
                .with_size($width, $height);
            golden.assert_match(&$element);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_diff() {
        let diff = simple_diff("hello\nworld", "hello\nearth");
        assert!(diff.contains("Line 2"));
        assert!(diff.contains("- world"));
        assert!(diff.contains("+ earth"));
    }

    #[test]
    fn test_golden_path() {
        let golden = GoldenTest::new("my_test");
        assert!(golden.golden_path().ends_with("my_test.txt"));
    }
}
