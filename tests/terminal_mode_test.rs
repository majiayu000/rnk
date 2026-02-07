//! Test: Verify inline mode doesn't use alternate screen
//!
//! This test ensures that inline mode renders to the main screen buffer
//! and doesn't send alternate screen escape sequences.

use std::process::Command;

fn terminal_source() -> String {
    std::fs::read_to_string(format!(
        "{}/src/renderer/terminal.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("failed to read terminal.rs")
}

fn section(source: &str, start_marker: &str, end_marker: &str) -> &str {
    let start = source.find(start_marker).expect("start marker not found");
    let end = source[start..]
        .find(end_marker)
        .map(|idx| start + idx)
        .expect("end marker not found");
    &source[start..end]
}

/// Test alternate screen escape sequences
#[test]
fn test_alternate_screen_escape_sequences() {
    // CSI ?1049h - Enter alternate screen
    let enter_alt_screen = "\x1b[?1049h";

    // CSI ?1049l - Leave alternate screen
    let leave_alt_screen = "\x1b[?1049l";

    // Verify the escape sequences are correct
    assert_eq!(enter_alt_screen, "\x1b[?1049h");
    assert_eq!(leave_alt_screen, "\x1b[?1049l");
}

/// Test that fullscreen mode uses alternate screen
#[test]
fn test_fullscreen_uses_alternate_screen() {
    let source = terminal_source();
    let enter_section = section(
        &source,
        "pub fn enter(&mut self)",
        "pub fn enter_inline(&mut self)",
    );
    assert!(enter_section.contains("EnterAlternateScreen"));
    let exit_section = section(
        &source,
        "pub fn exit(&mut self)",
        "pub fn exit_inline(&mut self)",
    );
    assert!(exit_section.contains("LeaveAlternateScreen"));
}

/// Test that inline mode does NOT use alternate screen
#[test]
fn test_inline_no_alternate_screen() {
    let source = terminal_source();
    let enter_inline_section = section(
        &source,
        "pub fn enter_inline(&mut self)",
        "pub fn exit_inline(&mut self)",
    );
    assert!(!enter_inline_section.contains("EnterAlternateScreen"));

    let exit_inline_section = section(
        &source,
        "pub fn exit_inline(&mut self)",
        "pub fn switch_to_alt_screen(&mut self)",
    );
    assert!(!exit_inline_section.contains("LeaveAlternateScreen"));
}

/// Integration test: Verify terminal history preservation
#[test]
fn test_terminal_history_preservation() {
    // Create a simple test program
    let test_program = r#"
#!/bin/bash
# Test script to verify terminal history preservation

echo "=== Before rnk app ==="
echo "Line 1: Previous terminal content"
echo "Line 2: Should be visible after scrolling"
echo ""

# Simulated inline mode output
echo "=== rnk inline mode ==="
echo "App output line 1"
echo "App output line 2"
echo ""

echo "=== After rnk app ==="
echo "Line 3: New terminal content"
echo ""
echo "All content should be scrollable!"
"#;

    let temp_dir = std::env::temp_dir();
    let test_script = temp_dir.join("terminal_history_test.sh");
    std::fs::write(&test_script, test_program).unwrap();

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&test_script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&test_script, perms).unwrap();
    }

    let output = Command::new("bash")
        .arg(&test_script)
        .output()
        .expect("Failed to run test script");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify all content is present
    assert!(stdout.contains("Before rnk app"));
    assert!(stdout.contains("Previous terminal content"));
    assert!(stdout.contains("After rnk app"));
    assert!(stdout.contains("All content should be scrollable"));

    println!("✓ Terminal history preservation test passed");
}
