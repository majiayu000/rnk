# rnk - React-like Terminal UI for Rust

A terminal UI framework inspired by [Ink](https://github.com/vadimdemedes/ink) and [Bubbletea](https://github.com/charmbracelet/bubbletea).

## Development Guidelines

### Code Quality Commands

```bash
# Quick validation (before every commit)
cargo fmt && cargo clippy && cargo test --lib

# Full validation
cargo test --all-targets

# Property tests (finds edge cases)
cargo test --test property_tests

# Run a specific example
cargo run --example counter
```

### Module Testing

```bash
# Layout module
cargo test layout::

# Renderer module
cargo test renderer::

# Hooks module
cargo test hooks::

# Components module
cargo test components::

# Testing utilities
cargo test testing::
```

### Key Invariants

1. **Unicode Width**: Layout and render must use same width calculation
   - ASCII = 1 cell, CJK = 2 cells, Box-drawing = 1 cell
   - Test: `cargo test testing::generators::tests::test_unicode_width_cases`

2. **Layout Bounds**: All coordinates and dimensions non-negative
   - Test: `cargo test testing::renderer::tests::test_layout_validation`

3. **Style Isolation**: Styles must not leak between elements
   - Every styled span ends with reset code

4. **Hook State Persistence**: Signal values persist across renders
   - Test: `cargo test hooks::use_signal::tests`

### Common Bug Patterns

| Bug | Symptom | Check |
|-----|---------|-------|
| Unicode width mismatch | Text misaligned | `output.rs:write()` uses `char.width()` |
| Style leakage | Wrong colors | Reset codes in `render()` |
| Child position error | Overlapping elements | `render_element()` offset calculation |
| Hook state loss | Values reset | `HookContext` hook ordering |

### Skills (Claude Code)

Located in `.claude/skills/`:

- **tink-layout-validator.md** - Validate layout calculations
- **tink-render-validator.md** - Validate rendering output
- **tink-hooks-validator.md** - Validate hooks system
- **tink-component-validator.md** - Validate components
- **tink-full-validation.md** - Complete pre-commit validation

### Architecture

```
src/
├── core/           # Element, Style, Color
├── components/     # Box, Text, Spacer, Transform, Static
├── hooks/          # use_signal, use_effect, use_input, use_focus, ...
├── layout/         # LayoutEngine (Taffy), text measurement
├── renderer/       # App runner, Terminal, Output buffer
└── testing/        # TestRenderer, assertions, golden tests, generators
```

### Testing Infrastructure

```rust
use rnk::testing::{TestRenderer, assert_layout_valid, GoldenTest};

// Layout validation
let renderer = TestRenderer::new(80, 24);
renderer.validate_layout(&element).expect("valid");

// Golden file testing
GoldenTest::new("my_component").assert_match(&element);

// Assertions
element.assert_renders_containing("expected text");
element.assert_layout_valid();
```

### CI

GitHub Actions runs on every push/PR:
1. Format check
2. Clippy
3. Library tests
4. Doc tests
5. Property tests
6. Integration tests
7. Cross-platform (Linux, macOS, Windows)
8. Code coverage
