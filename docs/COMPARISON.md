# rnk Framework Comparison

This document is a current evidence snapshot for `rnk` compared with Ink
(JavaScript) and Bubbletea (Go). It is not a parity scorecard. Terminal UI
behavior depends on terminal emulator support, platform behavior, and the exact
application mode.

## Summary

| Area | rnk | Ink | Bubbletea |
|------|-----|-----|-----------|
| Language | Rust | JavaScript / TypeScript | Go |
| Architecture | React-like components and hooks | React components | Elm-style update loop |
| Layout | Taffy Flexbox | Yoga Flexbox | Manual layout, often with Lip Gloss |
| State | hooks and signals | React hooks | model/update messages |
| Rendering | line-level diff output | line-level output | line-level output |
| Inline mode | supported | supported | supported |
| Fullscreen mode | supported | supported | supported |
| Mouse input | supported through hooks | limited / application-specific | supported |
| Bracketed paste | supported | limited / application-specific | supported |
| Theme system | semantic terminal theme API | application-specific | Lip Gloss ecosystem |
| Rust type safety | native | not applicable | not applicable |

## Current Strengths

- Declarative Rust API with components, hooks, signals, and command helpers.
- Flexbox layout through Taffy, including nested component composition.
- Broad built-in component surface for forms, display, feedback, layout, and
  showcase apps.
- Terminal runtime support for inline/fullscreen rendering, mouse input,
  bracketed paste, panic restoration, signal handling, CI/non-TTY detection, and
  frame pacing.
- Workspace helper crates for standalone styling and icon usage.

## Known Gaps

The maturity spec tracks the remaining work. Important open areas are:

- Public API boundary and pre-1.0 stability policy.
- Consistent controlled/uncontrolled contracts for interactive components.
- Non-color design tokens, component variants, and shared action primitives.
- Focus scopes, keyboard conventions, accessibility metadata, and fallback text.
- Terminal compatibility matrix for Unicode, ANSI, resize, clipping, and emulator
  behavior.
- Stronger test harness support for input, mouse, resize, focus, and golden app
  flows.

See [RNK_MATURITY_SPEC.md](RNK_MATURITY_SPEC.md) for issue-level acceptance
criteria.

## Choosing A Framework

Choose `rnk` when you want a Rust-native, declarative terminal UI with Flexbox
layout, hooks, typed commands, and built-in components.

Choose Ink when your application is already in the JavaScript/TypeScript and
React ecosystem.

Choose Bubbletea when your application is Go-first or benefits from the broader
Charm ecosystem and Elm-style update model.

## Documentation Policy

This file should stay evidence-based:

- Do not claim complete parity with another framework unless the matching tests
  and runtime behavior are linked.
- Prefer current source references and maturity-spec issues over historical
  roadmap assertions.
- Keep old planning notes under `docs/todo/` clearly marked as historical or
  refreshed into current status.
