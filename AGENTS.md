# AGENTS

This file contains repository-specific guidance for `rnk`. Use the skills and
tooling exposed by the active agent runtime; do not hard-code user-specific
skill installation paths in this repository.

## Scope

- Keep `rnk` terminal-first. Do not turn the crate into a native desktop GUI,
  GPU renderer, or product-specific application shell.
- Treat the public Rust crate API as the product boundary. The `rnk` binary is
  a repository demo, not a supported standalone CLI.
- Preserve the documented Rust 1.88 minimum unless a task explicitly changes
  the MSRV contract.
- Read nested `AGENTS.md` files before editing files in their subtrees.

## Verification

Use the smallest focused check while iterating. Before handing off a general
code change, run the relevant CI-equivalent commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
```

For documentation-only changes, verify links and examples affected by the edit;
do not claim the Rust test gate ran when it was intentionally skipped.
