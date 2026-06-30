# rnk Framework Comparison

This document is a current evidence snapshot for `rnk` compared with Ink
(JavaScript), Bubbletea (Go), Ratatui (Rust), and Warp/WarpUI. It is not a
parity scorecard. Terminal UI behavior depends on terminal emulator support,
platform behavior, and the exact application mode.

## Summary

| Area | rnk | Ratatui | Ink | Bubbletea |
|------|-----|---------|-----|-----------|
| Language | Rust | Rust | JavaScript / TypeScript | Go |
| Architecture | React-like components and hooks | immediate-mode widgets and buffers | React components | Elm-style update loop |
| Layout | Taffy Flexbox | constraint-based terminal layout | Yoga Flexbox | Manual layout, often with Lip Gloss |
| State | hooks and signals | application-owned state | React hooks | model/update messages |
| Rendering | line-level diff output | buffer/frame rendering through backend | line-level output | line-level output |
| Inline mode | supported | backend/application-specific | supported | supported |
| Fullscreen mode | supported | common through crossterm backend | supported | supported |
| Mouse input | supported through hooks | supported through backend events | limited / application-specific | supported |
| Bracketed paste | supported | backend/application-specific | limited / application-specific | supported |
| Theme system | semantic terminal theme API | application/widget styling | application-specific | Lip Gloss ecosystem |
| Rust type safety | native | native | not applicable | not applicable |

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

The maturity spec records the original readiness plan. Current open areas should
be tracked through active GitHub issues and linked specs. Important areas are:

- Keeping the public API boundary tight enough that new users do not depend on
  advanced or experimental modules by accident.
- Expanding controlled/uncontrolled contracts from the recommended core
  component set to the broader interactive input surface.
- Turning broad examples into a smaller set of product-like templates for agent
  UIs, chat-style tools, dashboards, and forms.
- Keeping release automation and publishing checks trustworthy.

See [RNK_MATURITY_SPEC.md](RNK_MATURITY_SPEC.md) for issue-level acceptance
criteria. See [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md) for the
current terminal matrix and Unicode/ANSI behavior contract.

## Choosing A Framework

Choose `rnk` when you want a Rust-native, declarative terminal UI with Flexbox
layout, hooks, typed commands, and built-in components.

Choose Ratatui when you want direct control over terminal buffers, frame
rendering, and a mature Rust widget ecosystem, especially if your application is
already structured around an immediate-mode draw loop.

Choose Ink when your application is already in the JavaScript/TypeScript and
React ecosystem.

Choose Bubbletea when your application is Go-first or benefits from the broader
Charm ecosystem and Elm-style update model.

## Warp And WarpUI

Warp is a full agentic development environment and terminal product. Its open
source repository includes `warpui` and `warpui_core`, which are product-internal
UI framework crates for Warp's app shell, views, native windowing, rendering,
fonts, clipboard, and platform integration. Those crates are useful design
references, but they are not a direct `rnk` competitor because they are not
distributed as a third-party crates.io UI library.

Choose `rnk` when you want a small Rust dependency for terminal-first
applications. Look at WarpUI when you are studying how a production terminal
product organizes UI primitives, component contracts, issue/spec workflow, and
release discipline.

`rnk` should learn these practices from WarpUI and Warp:

- Treat public positioning as part of the product surface.
- Keep issue readiness explicit with labels such as `ready-to-spec` and
  `ready-to-implement`.
- Prefer component contracts that document state, events, disabled/read-only
  behavior, and test anchors.
- Keep release automation observable and fail-fast.

`rnk` should not copy WarpUI's scope. Native windows, GPU rendering, desktop app
distribution, and product-internal service integration are outside `rnk`'s
terminal-first library boundary.

## Documentation Policy

This file should stay evidence-based:

- Do not claim complete parity with another framework unless the matching tests
  and runtime behavior are linked.
- Prefer current source references and maturity-spec issues over historical
  roadmap assertions.
- Keep old planning notes under `docs/todo/` clearly marked as historical or
  refreshed into current status.
