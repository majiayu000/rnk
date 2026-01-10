# Tink Layout Validator Skill

## Purpose
Validate layout calculations to prevent visual bugs like misaligned elements, incorrect dimensions, and Unicode width issues.

## When to Use
- After modifying any code in `src/layout/`
- After modifying `src/renderer/output.rs`
- When adding new components with custom sizing
- When handling Unicode text

## Validation Checklist

### 1. Unicode Width Consistency
```bash
# Run Unicode width tests
cargo test testing::generators::tests::test_unicode_width_cases
```

Key invariants:
- ASCII characters = 1 cell width
- CJK characters = 2 cells width
- Box-drawing characters = 1 cell width
- Layout engine and renderer must use same width calculation

### 2. Layout Bounds Validation
```bash
# Run layout validation tests
cargo test testing::renderer::tests::test_layout_validation
```

Key invariants:
- All coordinates must be non-negative
- All dimensions must be non-negative
- Elements must not exceed terminal bounds (unless overflow enabled)
- Children must be within parent bounds (accounting for padding/border)

### 3. Flex Layout Tests
```rust
// Test row layout
let row = Box::new()
    .flex_direction(FlexDirection::Row)
    .child(Box::new().width(10))
    .child(Box::new().width(10));
// Children should have x = [0, 10]

// Test column layout
let col = Box::new()
    .flex_direction(FlexDirection::Column)
    .child(Box::new().height(5))
    .child(Box::new().height(5));
// Children should have y = [0, 5]
```

### 4. Border and Padding Tests
```rust
// Border adds 1 to each side
let bordered = Box::new()
    .width(20)
    .border_style(BorderStyle::Single);
// Content area = 18 (20 - 2)

// Padding adds to content offset
let padded = Box::new()
    .padding(2);
// Content starts at (2, 2) from box origin
```

## Common Bug Patterns

### Bug: Unicode width mismatch
**Symptom:** Text appears shifted, borders don't align
**Cause:** Layout uses unicode-width but render uses byte length
**Check:** `src/renderer/output.rs:write()` must use `char.width()`

### Bug: Child position accumulation
**Symptom:** Nested boxes overlap instead of stack
**Cause:** Child offset not properly accumulated
**Check:** `render_element()` passes correct `offset_x/y` to children

### Bug: Border not accounted in content position
**Symptom:** Text overlaps border
**Cause:** Text rendering doesn't offset by border width
**Check:** `text_x/y` calculation includes border check

## Automated Validation

```rust
use tink::testing::{TestRenderer, assert_layout_valid};

#[test]
fn test_my_component_layout() {
    let element = MyComponent::new().into_element();
    let renderer = TestRenderer::new(80, 24);

    // Validate all layout constraints
    renderer.validate_layout(&element).expect("Layout should be valid");

    // Check specific dimensions
    let layout = renderer.get_layout(&element).unwrap();
    assert!(layout.width > 0.0);
    assert!(layout.height > 0.0);
}
```

## Integration with CI

```yaml
# .github/workflows/layout-validation.yml
- name: Layout Tests
  run: |
    cargo test layout::
    cargo test testing::renderer::
    cargo test testing::generators::tests::test_unicode_width_cases
```
