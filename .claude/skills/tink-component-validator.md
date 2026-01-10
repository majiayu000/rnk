# Tink Component Validator Skill

## Purpose
Validate UI components for correct behavior, ensuring builder patterns work correctly and elements render as expected.

## When to Use
- After modifying `src/components/`
- When adding new component properties
- When changing component defaults
- After modifying component-to-element conversion

## Validation Checklist

### 1. Builder Pattern Correctness
```bash
cargo test components::
```

Key invariants:
- All builder methods return `Self` for chaining
- Default values are sensible
- `into_element()` produces valid Element

### 2. Box Component
```rust
// Verify all Box properties
let box_element = Box::new()
    .width(20)
    .height(10)
    .padding(2)
    .margin(1)
    .flex_direction(FlexDirection::Row)
    .border_style(BorderStyle::Single)
    .border_color(Color::Blue)
    .background(Color::Black)
    .child(Text::new("child").into_element())
    .into_element();

// All properties should be reflected in element.style
```

### 3. Text Component
```rust
// Verify all Text styles
let text = Text::new("Hello")
    .color(Color::Red)
    .background(Color::Blue)
    .bold()
    .italic()
    .underline()
    .strikethrough()
    .dim()
    .inverse()
    .into_element();

// All style flags should be set
assert!(text.style.bold);
assert!(text.style.italic);
// etc.
```

### 4. Spacer Component
```rust
// Spacer should have flex_grow = 1
let spacer = Spacer::new().into_element();
assert_eq!(spacer.style.flex_grow, 1.0);
```

### 5. Transform Component
```rust
// Verify text transformations
let upper = Transform::new("hello", TransformFn::Uppercase);
assert_eq!(upper.transform(), "HELLO");

let lower = Transform::new("HELLO", TransformFn::Lowercase);
assert_eq!(lower.transform(), "hello");

let cap = Transform::new("hello world", TransformFn::Capitalize);
assert_eq!(cap.transform(), "Hello World");
```

## Common Bug Patterns

### Bug: Builder method doesn't update state
**Symptom:** Setting property has no effect
**Cause:** Method returns new instance instead of mutating
**Check:** Builder methods use `mut self` and update fields

### Bug: Default override
**Symptom:** Explicit values are overwritten
**Cause:** `into_element()` applies defaults after custom values
**Check:** Defaults only apply to unset fields

### Bug: Child order incorrect
**Symptom:** Children render in wrong order
**Cause:** `child()` prepends instead of appends
**Check:** `children` Vec uses `push()` not `insert(0, ...)`

## Automated Validation

```rust
use tink::testing::{TestRenderer, assert_renders_containing};
use tink::components::*;

#[test]
fn test_box_with_all_properties() {
    let element = Box::new()
        .width(30)
        .height(10)
        .padding(1)
        .border_style(BorderStyle::Round)
        .border_color(Color::Cyan)
        .child(Text::new("Content").into_element())
        .into_element();

    let renderer = TestRenderer::new(80, 24);

    // Verify layout
    let layout = renderer.get_layout(&element).unwrap();
    assert!((layout.width - 30.0).abs() < 0.5);
    assert!((layout.height - 10.0).abs() < 0.5);

    // Verify content renders
    let output = renderer.render_to_plain(&element);
    assert!(output.contains("Content"));

    // Verify border renders (round corners)
    assert!(output.contains("╭")); // top-left
    assert!(output.contains("╯")); // bottom-right
}

#[test]
fn test_text_styling() {
    let element = Text::new("Styled")
        .bold()
        .color(Color::Green)
        .into_element();

    assert!(element.style.bold);
    assert_eq!(element.style.color, Some(Color::Green));
}

#[test]
fn test_nested_children() {
    let element = Box::new()
        .child(Text::new("First").into_element())
        .child(Text::new("Second").into_element())
        .child(Text::new("Third").into_element())
        .into_element();

    assert_eq!(element.children.len(), 3);

    // Verify order
    assert_eq!(element.children[0].text_content, Some("First".into()));
    assert_eq!(element.children[1].text_content, Some("Second".into()));
    assert_eq!(element.children[2].text_content, Some("Third".into()));
}
```

## Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn box_dimensions_non_negative(width in 0u16..1000, height in 0u16..1000) {
        let element = Box::new()
            .width(width)
            .height(height)
            .into_element();

        let renderer = TestRenderer::new(1000, 1000);
        let layout = renderer.get_layout(&element).unwrap();

        prop_assert!(layout.width >= 0.0);
        prop_assert!(layout.height >= 0.0);
    }

    #[test]
    fn text_renders_content(s in "[a-zA-Z0-9 ]{1,50}") {
        let element = Text::new(&s).into_element();
        let renderer = TestRenderer::new(80, 24);
        let output = renderer.render_to_plain(&element);

        prop_assert!(output.contains(&s));
    }
}
```

## Integration with CI

```yaml
# .github/workflows/component-validation.yml
- name: Component Tests
  run: |
    cargo test components::
    cargo test --test property_tests -- components
```
