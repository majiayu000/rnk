# Tink Hooks Validator Skill

## Purpose
Validate the hooks system for state management correctness, preventing state loss, effect bugs, and lifecycle issues.

## When to Use
- After modifying `src/hooks/`
- When implementing new hooks
- When debugging state-related issues
- After changes to the render loop

## Validation Checklist

### 1. Signal State Persistence
```bash
cargo test hooks::use_signal::tests
```

Key invariants:
- Signal value persists across re-renders
- Multiple signals maintain independent state
- Signal updates trigger re-render

### 2. Effect Execution
```bash
cargo test hooks::use_effect::tests
```

Key invariants:
- `use_effect` runs on first render
- `use_effect` re-runs when dependencies change
- `use_effect_once` runs exactly once
- Cleanup functions are called on dependency change

### 3. Input Handling
```bash
cargo test hooks::use_input::tests
```

Key invariants:
- Key events are dispatched to registered handlers
- Modifiers (Ctrl, Alt, Shift) are correctly detected
- Handler unregistration works

### 4. Focus Management
```bash
cargo test hooks::use_focus::tests
```

Key invariants:
- Focus moves to next/previous focusable element
- Inactive elements are skipped
- Focus wraps around at boundaries

## Common Bug Patterns

### Bug: State reset on re-render
**Symptom:** Counter resets to 0, inputs cleared
**Cause:** Hook index not properly tracked
**Check:** `HookContext` maintains stable hook ordering

### Bug: Effect runs every render
**Symptom:** API calls on every frame, infinite loops
**Cause:** Dependency comparison fails
**Check:** Dependencies use proper equality comparison

### Bug: Stale closure capture
**Symptom:** Effect/handler uses old state values
**Cause:** Closure captures initial state, not current
**Check:** Use `.get()` inside closures, not captured values

### Bug: Handler memory leak
**Symptom:** Memory grows over time
**Cause:** Input handlers not unregistered
**Check:** Cleanup on component unmount

## Automated Validation

```rust
use tink::hooks::*;

#[test]
fn test_signal_persistence() {
    // Simulate multiple renders
    let mut context = HookContext::new();

    // First render
    let value1 = context.with_hooks(|| {
        let signal = use_signal(|| 42);
        signal.get()
    });
    assert_eq!(value1, 42);

    // Update signal
    context.with_hooks(|| {
        let signal = use_signal(|| 0); // Init ignored after first render
        signal.set(100);
    });

    // Third render - value should persist
    let value3 = context.with_hooks(|| {
        let signal = use_signal(|| 0);
        signal.get()
    });
    assert_eq!(value3, 100);
}

#[test]
fn test_effect_dependency_tracking() {
    let call_count = std::cell::RefCell::new(0);

    let mut context = HookContext::new();

    // Render with dep = 1
    context.with_hooks(|| {
        use_effect(|| {
            *call_count.borrow_mut() += 1;
        }, (1,));
    });
    assert_eq!(*call_count.borrow(), 1);

    // Render with same dep - effect should NOT run
    context.with_hooks(|| {
        use_effect(|| {
            *call_count.borrow_mut() += 1;
        }, (1,));
    });
    assert_eq!(*call_count.borrow(), 1); // Still 1

    // Render with different dep - effect SHOULD run
    context.with_hooks(|| {
        use_effect(|| {
            *call_count.borrow_mut() += 1;
        }, (2,));
    });
    assert_eq!(*call_count.borrow(), 2);
}
```

## Integration Testing

```rust
#[test]
fn test_interactive_counter() {
    // Full integration test with hooks
    fn counter() -> Element {
        let count = use_signal(|| 0);

        use_input({
            let count = count.clone();
            move |_, key| {
                if key.code == KeyCode::Char('+') {
                    count.update(|n| n + 1);
                }
            }
        });

        Text::new(format!("Count: {}", count.get())).into_element()
    }

    // Verify initial state
    let initial = render_once(counter);
    assert!(initial.contains("Count: 0"));

    // Simulate key press and verify update
    simulate_key(KeyCode::Char('+'));
    let updated = render_once(counter);
    assert!(initial.contains("Count: 1"));
}
```

## Integration with CI

```yaml
# .github/workflows/hooks-validation.yml
- name: Hooks Tests
  run: |
    cargo test hooks::
    cargo test --test integration_tests -- hooks
```
