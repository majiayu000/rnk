---
name: Bug Report
about: Report a bug in rnk
title: '[Bug] '
labels: bug
assignees: ''
---

## Description

A clear description of the bug.

## Steps to Reproduce

1.
2.
3.

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened.

## Environment

- rnk version:
- Rust version:
- OS:
- Terminal:

## Minimal Reproduction

```rust
use rnk::prelude::*;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    // Minimal code that reproduces the issue
    todo!()
}
```

## Additional Context

Any other relevant information.
