# Tink Full Validation Skill

## Purpose
Run comprehensive validation across all modules before commits and releases.

## When to Use
- Before committing code
- Before creating pull requests
- Before releases
- After major refactoring

## Quick Validation (< 1 minute)

```bash
# Run all unit tests
cargo test --lib

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

## Full Validation (< 5 minutes)

```bash
# 1. Static Analysis
cargo fmt --check
cargo clippy -- -D warnings -W clippy::pedantic -A clippy::must_use_candidate

# 2. All Library Tests
cargo test --lib

# 3. Doc Tests
cargo test --doc

# 4. Integration Tests
cargo test --test '*'

# 5. Examples Build
cargo build --examples

# 6. Property Tests (if available)
cargo test -- --ignored

# 7. Coverage Check (requires cargo-tarpaulin)
cargo tarpaulin --out Html --skip-clean
```

## Pre-Commit Checklist

### Code Quality
- [ ] `cargo fmt` - Code is formatted
- [ ] `cargo clippy` - No warnings
- [ ] No `todo!()` or `unimplemented!()` in production code
- [ ] No `unwrap()` without justification comment
- [ ] No hardcoded values without constants

### Testing
- [ ] All tests pass: `cargo test --lib`
- [ ] New code has tests
- [ ] Edge cases covered (empty strings, zero dimensions, Unicode)

### Documentation
- [ ] Public APIs have doc comments
- [ ] Complex logic has inline comments
- [ ] CHANGELOG updated (if applicable)

## Module-Specific Validation

### Layout Module
```bash
cargo test layout::
cargo test testing::generators::tests::test_unicode_width_cases
```

### Renderer Module
```bash
cargo test renderer::
cargo test testing::renderer::
```

### Hooks Module
```bash
cargo test hooks::
```

### Components Module
```bash
cargo test components::
```

## Validation Script

Create `scripts/validate.sh`:

```bash
#!/bin/bash
set -e

echo "=== Tink Full Validation ==="

echo "1/7 Formatting..."
cargo fmt --check

echo "2/7 Clippy..."
cargo clippy -- -D warnings

echo "3/7 Library tests..."
cargo test --lib

echo "4/7 Doc tests..."
cargo test --doc

echo "5/7 Integration tests..."
cargo test --test '*'

echo "6/7 Examples build..."
cargo build --examples

echo "7/7 Example run test..."
timeout 2 cargo run --example hello || true

echo "=== All validations passed ==="
```

## CI Workflow

```yaml
# .github/workflows/validate.yml
name: Full Validation

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Format Check
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Tests
        run: cargo test --all-targets

      - name: Doc Tests
        run: cargo test --doc

      - name: Build Examples
        run: cargo build --examples

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Coverage
        run: cargo tarpaulin --out Xml

      - name: Upload Coverage
        uses: codecov/codecov-action@v3
```

## Coverage Requirements

| Module | Minimum Coverage |
|--------|-----------------|
| core | 90% |
| components | 85% |
| hooks | 80% |
| layout | 90% |
| renderer | 85% |
| testing | 70% |

## Validation Failure Response

### Format Failures
```bash
cargo fmt
```

### Clippy Warnings
- Fix the warning
- If false positive, add `#[allow(clippy::...)]` with comment explaining why

### Test Failures
1. Read the failure message
2. Check if it's a real bug or test issue
3. Fix the bug or update the test
4. Re-run validation

### Coverage Drops
1. Identify uncovered code
2. Add tests for uncovered paths
3. If code is unreachable, consider removing it
