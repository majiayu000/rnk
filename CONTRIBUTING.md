# Contributing to rnk

Thank you for your interest in contributing to rnk! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.85+ (edition 2024)
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/majiayu000/rnk.git
cd rnk

# Build the project
cargo build

# Run tests
cargo test --lib

# Run an example
cargo run --example counter
```

## Development Workflow

### Before Making Changes

1. Check existing issues to avoid duplicate work
2. For significant changes, open an issue first to discuss the approach
3. Fork the repository and create a feature branch

### Code Quality

Run these commands before submitting:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test --lib

# Run all tests including integration
cargo test --all-targets
```

### Commit Guidelines

We follow conventional commits:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test additions/changes
- `refactor:` Code refactoring
- `chore:` Maintenance tasks

Example:
```
feat: add Calendar component with date selection

- Support month navigation
- Highlight today and selected dates
- Configurable first day of week
```

**Important**: All commits must include `Signed-off-by` line:
```bash
git commit -s -m "feat: your message"
```

## Project Structure

```
rnk/
├── src/
│   ├── animation/      # Animation system
│   ├── cmd/            # Command system
│   ├── components/     # UI components
│   ├── core/           # Core types (Element, Style, Color)
│   ├── hooks/          # React-like hooks
│   ├── layout/         # Flexbox layout engine
│   ├── renderer/       # Terminal rendering
│   ├── runtime/        # Runtime utilities
│   └── testing/        # Test infrastructure
├── crates/
│   └── rnk-style/      # Standalone styling library
├── examples/           # Example applications
└── tests/              # Integration tests
```

## Adding New Components

1. Create file in `src/components/`
2. Follow existing component patterns (see `src/components/sparkline.rs`)
3. Add to `src/components/mod.rs`
4. Add tests
5. Update README if significant

### Component Template

```rust
//! Component description
//!
//! Usage example in doc comment

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element};

/// Component struct
#[derive(Debug, Clone)]
pub struct MyComponent {
    // fields
}

impl MyComponent {
    pub fn new() -> Self {
        Self { /* defaults */ }
    }

    // Builder methods
    pub fn some_option(mut self, value: Type) -> Self {
        self.field = value;
        self
    }

    pub fn into_element(self) -> Element {
        // Build and return Element
    }
}

impl Default for MyComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_creation() {
        let comp = MyComponent::new();
        let _ = comp.into_element();
    }
}
```

## Adding New Hooks

1. Create file in `src/hooks/`
2. Follow existing hook patterns (see `src/hooks/use_signal.rs`)
3. Add to `src/hooks/mod.rs`
4. Add tests

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

### Using TestHarness

```rust
use rnk::testing::TestHarness;

#[test]
fn test_component_output() {
    let harness = TestHarness::new(my_component);
    harness.assert_text_contains("expected");
}
```

### Using TestRenderer

```rust
use rnk::testing::TestRenderer;

#[test]
fn test_layout() {
    let renderer = TestRenderer::new(80, 24);
    let element = MyComponent::new().into_element();
    renderer.validate_layout(&element).expect("valid layout");
}
```

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Add entry to CHANGELOG.md (if exists)
4. Request review from maintainers

### PR Title Format

```
feat: add new feature
fix: resolve issue with X
docs: update README
```

## Code Style

- Use `rustfmt` defaults
- Prefer explicit types in public APIs
- Document public items with `///`
- Keep functions focused and small
- Avoid over-engineering

## Questions?

- Open an issue for questions
- Check existing issues and discussions

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
