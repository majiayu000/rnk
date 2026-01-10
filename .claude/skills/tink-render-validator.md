# Tink Render Validator Skill

## Purpose
Validate rendering output to prevent visual corruption, style leakage, and ANSI code errors.

## When to Use
- After modifying `src/renderer/`
- After adding new styling options
- When implementing new border styles
- When changing color handling

## Validation Checklist

### 1. ANSI Code Correctness
```bash
# Run ANSI-related tests
cargo test renderer::output::tests::test_styled_output
```

Key invariants:
- Every style opening must have a reset (`\x1b[0m`)
- ANSI sequences must be well-formed: `\x1b[<codes>m`
- No dangling escape sequences

### 2. Style Isolation
```rust
// Style changes should not leak between elements
let element = Box::new()
    .child(Text::new("red").color(Color::Red).into_element())
    .child(Text::new("normal").into_element());  // Should NOT be red
```

### 3. Wide Character Handling
```rust
// Wide chars must occupy 2 cells
let text = Text::new("你好");  // 4 cells total
let renderer = TestRenderer::new(80, 24);
let output = renderer.render_to_plain(&text.into_element());
// Visual alignment must be correct
```

### 4. Border Rendering
```rust
// All border styles must render correctly
for style in [BorderStyle::Single, BorderStyle::Double, BorderStyle::Round, BorderStyle::Bold] {
    let element = Box::new()
        .width(10)
        .height(5)
        .border_style(style);
    // Verify corner and edge characters
}
```

## Common Bug Patterns

### Bug: Style leakage
**Symptom:** Wrong colors appear after styled elements
**Cause:** Missing reset codes
**Check:** Every line in render output ends with reset if it had styles

### Bug: Wide char corruption
**Symptom:** Characters after CJK text are shifted
**Cause:** Output grid doesn't account for wide chars
**Check:** `write()` uses `char.width()` and marks continuation cells with `\0`

### Bug: Border overlap
**Symptom:** Border characters appear in wrong positions
**Cause:** Border drawn at wrong coordinates
**Check:** Border x/y calculation uses layout position correctly

### Bug: Lines shifted in raw mode
**Symptom:** Each line appears progressively shifted right
**Cause:** Using `\n` instead of `\r\n` in raw mode output
**Check:** `output.render()` uses `\r\n` for line endings
**Test:** `cargo test renderer::output::tests::test_raw_mode_line_endings`

## Automated Validation

```rust
use tink::testing::{TestRenderer, strip_ansi_codes};

#[test]
fn test_render_correctness() {
    let renderer = TestRenderer::new(80, 24);
    let element = create_test_element();

    // Test plain output
    let plain = renderer.render_to_plain(&element);
    assert!(plain.contains("expected text"));

    // Test ANSI output
    let ansi = renderer.render_to_ansi(&element);

    // Verify ANSI codes are balanced
    let opens = ansi.matches("\x1b[").count();
    let resets = ansi.matches("\x1b[0m").count();
    // Should have roughly equal opens and resets
    assert!(resets > 0);
}

#[test]
fn test_no_style_leakage() {
    let element = Box::new()
        .child(Text::new("styled").color(Color::Red).into_element())
        .child(Text::new("unstyled").into_element());

    let renderer = TestRenderer::new(80, 24);
    let output = renderer.render_to_ansi(&element);

    // "unstyled" should not have red color code before it
    // Find position of "unstyled" and check preceding codes
}
```

## Visual Regression Testing

```rust
use tink::testing::GoldenTest;

#[test]
fn test_border_styles() {
    let element = Box::new()
        .width(20)
        .height(5)
        .border_style(BorderStyle::Round)
        .child(Text::new("Content").into_element());

    GoldenTest::new("border_round")
        .with_size(40, 10)
        .assert_match(&element);
}
```

## Integration with CI

```yaml
# .github/workflows/render-validation.yml
- name: Render Tests
  run: |
    cargo test renderer::
    cargo test testing::assertions::
    # Visual regression
    cargo test --test golden_tests
```
