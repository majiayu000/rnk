# rnk Maturity Spec

This spec defines what remains before `rnk` can be treated as a mature Rust
terminal UI framework and design-system library. It is intentionally scoped to
terminal UI. `rnk` is not a desktop, mobile, or web GUI toolkit.

## Baseline

- Current crate version: `0.19.2`.
- Current public distribution: the `rnk` Rust crate.
- Current binary target: a minimal repository demo, not a supported user CLI.
- Local verification baseline for this spec tranche:
  - `cargo test --workspace`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`

## Maturity Goal

`rnk` should become a dependable Rust TUI framework with a small stable public
API, a coherent terminal design system, predictable interactive component
contracts, and release gates that prove every workspace crate stays healthy.

The maturity target is reached when:

- Public API boundaries are explicit and versioned.
- Interactive components expose consistent state and event contracts.
- Theme and design tokens cover component variants, density, focus, and states.
- Terminal behavior is documented and tested across realistic environments.
- Unicode, ANSI, resize, non-TTY, panic, and signal behavior are specified.
- CI and release workflows validate the complete workspace and feature matrix.
- Stale documentation is either updated, archived, or clearly marked historical.

## Non-Goals

- Building a pixel-based GUI toolkit.
- Replacing Tauri, Dioxus, Flutter, egui, or web UI frameworks.
- Publishing a supported `rnk` CLI before the crate API is stable.
- Claiming feature parity with Ink, Bubble Tea, or Ratatui without evidence.

## Current Strengths

- React-like component model and hooks.
- Taffy-backed flexbox layout.
- Inline and fullscreen render modes.
- Rich component inventory across display, feedback, input, layout, viewport,
  textarea, theme, animation, and testing modules.
- Grapheme-aware measurement and truncation in layout utilities.
- Panic, signal, CI, TTY, frame-rate, mouse, and bracketed paste primitives.
- A broad example set and a workspace that currently passes local full checks.

## Workstreams

### 1. Workspace CI And Release Gates

Problem:

The repository is a workspace, but GitHub workflows still lean heavily on the
root crate. Release validation also publishes only the root crate and runs a
root-only test gate.

Required outcome:

- CI has required workspace-level jobs for test, clippy, docs, and examples.
- CI covers `--all-features`, `--no-default-features`, and the declared MSRV.
- Release dry-runs every publishable crate before publishing.
- Helper crates have an explicit publish policy and dependency version policy.
- Benchmarks at least compile in CI; performance thresholds can come later.

Acceptance criteria:

- `cargo test --workspace --all-targets --all-features --locked` is part of CI.
- `cargo test --workspace --all-targets --no-default-features --locked` is part
  of CI, or any package exclusions are documented in the workflow.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  is part of CI.
- `cargo doc --workspace --no-deps --all-features --locked` is part of CI.
- The declared MSRV is verified by CI, or the declared MSRV is removed from
  crate metadata and public docs.
- `cargo check --workspace --examples --all-features --locked` is part of CI.
- `cargo check --workspace --benches --all-features --locked` is part of CI.
- Release workflow validates root and helper crate versions from Cargo metadata.
- `rnk-style` does not rely on a path-only dependency if it is intended to be
  published independently.

GitHub issue: [#20](https://github.com/majiayu000/rnk/issues/20)

### 2. Documentation Truth And Roadmap Cleanup

Problem:

Public docs describe `rnk`, but older documents used the historical name, made
unsupported 100% completion claims, and listed already-fixed runtime problems as
active blockers.

Required outcome:

- Active docs use the `rnk` name.
- Historical docs are moved or marked as historical.
- Comparison docs avoid unsupported 100% claims.
- The roadmap separates completed history, active gaps, and non-goals.
- README links to this spec as the current maturity plan.

Acceptance criteria:

- `rg -n "Tink|tink" README.md docs src examples` returns only intentional
  historical references.
- `docs/COMPARISON.md` reflects current capabilities and known gaps.
- `docs/todo/*` is either updated, moved under an archive path, or marked stale.
- README example listings are described as curated or generated.

GitHub issue: [#21](https://github.com/majiayu000/rnk/issues/21)

### 3. Public API Boundary And Stability Policy

Problem:

The crate exposes broad modules and many internal-looking structs, fields, and
helpers. This is convenient today but makes semver stability hard.

Required outcome:

- Stable public API is defined.
- Experimental APIs are explicitly marked.
- Internal modules are hidden or documented as unstable.
- Struct and enum evolution rules are documented.

Acceptance criteria:

- `rnk::prelude` or a new `rnk::ui` surface is the recommended stable import.
- Unstable internals are behind `#[doc(hidden)]`, renamed, or documented.
- Public field exposure is reviewed for `Element`, `Style`, renderer, runtime,
  and testing types.
- A semver policy explains what can change before `1.0`.

GitHub issue: [#22](https://github.com/majiayu000/rnk/issues/22)

### 4. Interactive Component State And Event Contracts

Problem:

Interactive components do not yet share a consistent contract for controlled
state, uncontrolled state, callbacks, submitted values, disabled behavior, and
event propagation.

Required outcome:

- Input components follow one shared contract.
- Components expose state handles or callbacks for meaningful interactions.
- Selection and submitted values can be observed without inspecting internals.
- Keyboard and mouse behavior is documented consistently.

Acceptance criteria:

- `TextInput`, `SelectInput`, `MultiSelect`, `Confirm`, `FilePicker`,
  `ColorPicker`, `CommandPalette`, `TextArea`, and `Viewport` are audited.
- Each component documents controlled and uncontrolled usage.
- `on_change`, `on_submit`, or equivalent callbacks exist where appropriate.
- Disabled/read-only behavior is represented consistently.
- Component tests cover value changes, focus movement, submit, cancel, disabled
  or read-only handling, and keyboard or mouse navigation for audited widgets.

GitHub issue: [#23](https://github.com/majiayu000/rnk/issues/23)

Current contract document:
[docs/INTERACTIVE_COMPONENT_CONTRACTS.md](INTERACTIVE_COMPONENT_CONTRACTS.md)

### 5. Design Tokens And Component Variants

Problem:

The theme system is mostly color-oriented. A design library needs shared tokens
for spacing, density, borders, focus, emphasis, symbols, and variants.

Required outcome:

- Tokens cover color, spacing, border, density, focus, and state.
- Component defaults resolve through the theme.
- Common variants are consistent across components.
- A reusable `Button` or action primitive exists if button-like controls remain
  part of higher-level components.

Acceptance criteria:

- Theme includes non-color tokens or an explicit reason for excluding them.
- Hard-coded component palettes are replaced or justified.
- Components document supported variants and states.
- Confirm/Dialog button styling is aligned with a shared action primitive.

GitHub issue: [#24](https://github.com/majiayu000/rnk/issues/24)

Current contract document:
[docs/DESIGN_TOKENS_AND_VARIANTS.md](DESIGN_TOKENS_AND_VARIANTS.md)

### 6. Focus, Accessibility, And Input Semantics

Problem:

Focus and accessibility primitives exist, but focus traversal, roles, labels,
descriptions, disabled state, and accessible fallbacks are still too manual for a
polished component library.

Required outcome:

- Focus scopes and default traversal behavior are available.
- Components can expose role, label, description, and disabled state.
- Screen-reader and non-interactive fallbacks are documented.
- Tab, Shift-Tab, Escape, Enter, and arrow-key conventions are consistent.

Acceptance criteria:

- Focus manager supports scoped traversal or documents why it does not.
- Common components declare keyboard contracts.
- Accessibility metadata is available on interactive components.
- Tests cover focus traversal and accessible fallback text.

GitHub issue: [#25](https://github.com/majiayu000/rnk/issues/25)

### 7. Terminal Compatibility, Unicode, And ANSI Behavior

Problem:

The README correctly warns that terminal support varies. The next maturity step
is to turn that warning into a compatibility matrix and explicit behavior spec.

Required outcome:

- Terminal feature behavior is documented for common environments.
- Unicode measurement, wrapping, truncation, rendering, and clipping are
  specified separately.
- ANSI parsing/slicing support is either implemented or declared unsupported.
- Resize behavior and wide-character clipping are covered by tests.

Acceptance criteria:

- Compatibility matrix covers macOS Terminal, iTerm2, Windows Terminal, tmux,
  SSH/non-TTY, and CI where practical.
- Spec distinguishes grapheme-aware measurement from renderer cell behavior.
- ANSI-aware truncation/clipping is tested or listed as unsupported.
- Resize tests cover width shrink and stale cell cleanup behavior.

GitHub issue: [#26](https://github.com/majiayu000/rnk/issues/26)

### 8. Testing Harness And Real-App Validation

Problem:

The test utilities are useful for rendering and layout assertions, but they do
not yet fully validate interactive design-library workflows.

Required outcome:

- Test harness can simulate keyboard, mouse, paste, focus, and resize.
- Golden/snapshot support covers ANSI and plain text outputs.
- Showcase apps validate real library components instead of bypassing them.
- Examples are categorized and checked as part of CI.

Acceptance criteria:

- Harness exposes `send_key`, `send_mouse`, paste, resize, and focus helpers.
- Golden tests exist for chat, git, top, forms, and textarea-like flows.
- `cargo build --examples` or an equivalent example gate is required in CI.
- Example index clearly separates tutorial, showcase, debug, and internal files.

GitHub issue: [#27](https://github.com/majiayu000/rnk/issues/27)

## Issue Map

| Workstream | Priority | Issue |
| --- | --- | --- |
| Workspace CI and release gates | P1 | [#20](https://github.com/majiayu000/rnk/issues/20) |
| Documentation truth and roadmap cleanup | P1 | [#21](https://github.com/majiayu000/rnk/issues/21) |
| Public API boundary and stability policy | P0 | [#22](https://github.com/majiayu000/rnk/issues/22) |
| Interactive component state and event contracts | P0 | [#23](https://github.com/majiayu000/rnk/issues/23) |
| Design tokens and component variants | P1 | [#24](https://github.com/majiayu000/rnk/issues/24) |
| Focus, accessibility, and input semantics | P1 | [#25](https://github.com/majiayu000/rnk/issues/25) |
| Terminal compatibility, Unicode, and ANSI behavior | P1 | [#26](https://github.com/majiayu000/rnk/issues/26) |
| Testing harness and real-app validation | P2 | [#27](https://github.com/majiayu000/rnk/issues/27) |

## Verification Plan

Every implementation PR against these workstreams should record:

- The issue it closes or advances.
- The public behavior changed.
- The terminal environments affected.
- Local commands run.
- Whether CI covers the touched workspace crate.
- Whether docs or examples need updates.

Minimum local verification for this spec-only PR:

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```
