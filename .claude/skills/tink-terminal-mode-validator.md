# Tink Terminal Mode Validator Skill

## Purpose
Validate terminal output correctness in different terminal modes (raw mode, alternate screen, cooked mode) to prevent display corruption.

## When to Use
- After modifying `src/renderer/output.rs`
- When adding new rendering modes
- When creating examples with raw mode or alternate screen
- After any changes to line/character output
- When debugging visual corruption issues

## Critical Bug Pattern: Raw Mode Line Endings

### The Bug
In raw mode (`terminal::enable_raw_mode()`), the terminal does NOT translate `\n` to `\r\n`.
- `\n` = Line Feed only (move down, stay at same column)
- `\r` = Carriage Return (move to column 0)
- `\r\n` = Both (move to column 0 of next line)

**Symptom:** Each line appears shifted right relative to the previous line.

**Fix:** Use `\r\n` for line endings in raw mode output:
```rust
// In output.rs render()
lines.join("\r\n")  // NOT "\n"
```

### Validation Test
```rust
#[test]
fn test_raw_mode_line_endings() {
    let output = Output::new(80, 24);
    // Write some content
    output.write(0, 0, "Line 1", &Style::default());
    output.write(0, 1, "Line 2", &Style::default());

    let rendered = output.render();

    // Must use CRLF for raw mode compatibility
    assert!(rendered.contains("\r\n"), "Output must use CRLF line endings for raw mode");
    assert!(!rendered.ends_with("\n") || rendered.ends_with("\r\n"),
        "Line endings must be CRLF, not just LF");
}
```

## Terminal Mode Checklist

### 1. Raw Mode Compatibility
```bash
# Test example in raw mode
cargo run --example interactive_demo
```

Invariants:
- [ ] Line endings use `\r\n` (CRLF)
- [ ] Cursor positioning is explicit (use crossterm's `MoveTo`)
- [ ] Screen is cleared before full redraws
- [ ] Proper cleanup on exit (disable raw mode, leave alternate screen)

### 2. Alternate Screen Compatibility
```rust
// Proper setup
execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
terminal::enable_raw_mode()?;

// ... render loop ...

// Proper cleanup (in reverse order)
execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
terminal::disable_raw_mode()?;
```

### 3. Cooked Mode (Normal) Compatibility
```rust
// In normal mode, \n is fine
println!("{}", output.render());
// Terminal translates \n to \r\n automatically
```

## Common Terminal Bugs

### Bug: Lines shifted right in raw mode
**Symptom:** Each subsequent line appears further to the right
**Cause:** Using `\n` instead of `\r\n` in raw mode
**Fix:** Use CRLF line endings in `output.render()`

### Bug: Garbage on screen after exit
**Symptom:** Terminal shows artifacts after program exits
**Cause:** Not leaving alternate screen or disabling raw mode
**Fix:** Ensure cleanup in both normal exit and panic handler:
```rust
fn cleanup() -> std::io::Result<()> {
    execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()
}

// Consider using Drop trait for automatic cleanup
```

### Bug: Cursor visible during animation
**Symptom:** Cursor flickers during redraws
**Cause:** Not hiding cursor in alternate screen mode
**Fix:** Use `cursor::Hide` when entering alternate screen

### Bug: Terminal size mismatch
**Symptom:** Content gets cut off or wrapped incorrectly
**Cause:** Using hardcoded size instead of actual terminal size
**Fix:** Use `terminal::size()` and handle resize events:
```rust
if let Event::Resize(width, height) = event::read()? {
    // Re-render with new size
}
```

### Bug: ANSI codes visible instead of colors
**Symptom:** See `[32m` instead of green text
**Cause:** Terminal doesn't support ANSI or codes are malformed
**Check:** Verify ANSI sequence format: `\x1b[<codes>m`

## Automated Validation

### Unit Test for Line Endings
```rust
#[cfg(test)]
mod terminal_mode_tests {
    use super::*;

    #[test]
    fn test_output_uses_crlf() {
        let mut output = Output::new(40, 10);
        output.write(0, 0, "Line 1", &Style::default());
        output.write(0, 1, "Line 2", &Style::default());

        let rendered = output.render();

        // Count line endings
        let lf_only = rendered.matches('\n').count()
            - rendered.matches("\r\n").count();
        assert_eq!(lf_only, 0, "Should not have standalone LF characters");
    }

    #[test]
    fn test_no_trailing_crlf_issues() {
        let mut output = Output::new(40, 10);
        output.write(0, 0, "Single line", &Style::default());

        let rendered = output.render();

        // Single line should not have any line endings at the end
        assert!(!rendered.ends_with("\r\n") || rendered.lines().count() > 1);
    }
}
```

### Integration Test for Raw Mode
```rust
#[test]
#[ignore] // Manual test - requires terminal
fn test_raw_mode_rendering() {
    // This test should be run manually
    // Run: cargo test test_raw_mode_rendering -- --ignored --nocapture

    use crossterm::terminal;
    use std::io::{stdout, Write};

    terminal::enable_raw_mode().unwrap();

    let mut output = Output::new(40, 5);
    output.write(0, 0, "Line 1 should start at column 0", &Style::default());
    output.write(0, 1, "Line 2 should start at column 0", &Style::default());
    output.write(0, 2, "Line 3 should start at column 0", &Style::default());

    print!("{}", output.render());
    stdout().flush().unwrap();

    terminal::disable_raw_mode().unwrap();
    println!("\n\nIf lines above are aligned, test passed!");
}
```

## CI Integration

```yaml
# .github/workflows/terminal-tests.yml
name: Terminal Mode Tests

on: [push, pull_request]

jobs:
  terminal-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Line Ending Tests
        run: |
          cargo test output::tests::
          cargo test terminal_mode_tests::

      - name: Build Interactive Examples
        run: cargo build --examples

      - name: Verify CRLF in Output
        run: |
          # Grep for the correct line ending pattern
          grep -q 'lines.join("\\r\\n")' src/renderer/output.rs || \
            (echo "ERROR: output.rs should use CRLF line endings" && exit 1)
```

## Debugging Terminal Issues

### Step 1: Check Line Endings
```bash
# Dump output to file and inspect
cargo run --example debug_80x24 > output.txt
xxd output.txt | head -50  # Look for 0d 0a (CRLF) vs just 0a (LF)
```

### Step 2: Compare Modes
```bash
# Test in cooked mode (should work)
cargo run --example simple_test

# Test in raw mode (if different, likely line ending issue)
cargo run --example interactive_demo
```

### Step 3: Isolate the Issue
Create minimal reproduction:
```rust
fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    print!("Line 1\r\nLine 2\r\nLine 3\r\n");  // Use explicit CRLF
    std::io::stdout().flush()?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    terminal::disable_raw_mode()?;
    Ok(())
}
```

## Terminal Compatibility Matrix

| Feature | Cooked Mode | Raw Mode | Alt Screen |
|---------|-------------|----------|------------|
| `\n` works | Yes | No (use `\r\n`) | No (use `\r\n`) |
| Auto echo | Yes | No | No |
| Line buffering | Yes | No | No |
| Cursor visible | Yes | Explicit | Explicit |
| ANSI codes | Usually | Yes | Yes |

## Preventive Measures

1. **Always use `\r\n`** in `output.render()` for raw mode compatibility
2. **Test in both modes** during development
3. **Add terminal mode tests** to CI
4. **Use crossterm's built-in** cursor/screen management
5. **Handle cleanup** in Drop trait or panic handler
