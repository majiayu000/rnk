# Terminal UI Historical Notes

This file replaces an older bug checklist that mixed fixed items with active
work. Current acceptance criteria live in
[../RNK_MATURITY_SPEC.md](../RNK_MATURITY_SPEC.md).

## Current Status

| Area | Current status | Follow-up |
|------|----------------|-----------|
| Panic restoration | Runtime support exists. | Keep covered by release and runtime tests. |
| Signal handling | Runtime signal helpers exist. | Keep terminal behavior documented in #26. |
| CI / non-TTY detection | Environment detection exists. | Document supported fallback behavior in #26. |
| Mouse input | Mouse hooks exist. | Exercise behavior in component and harness tests. |
| Bracketed paste | Paste support exists. | Exercise behavior in #23/#27 input tests. |
| Grapheme-aware measurement | Layout measurement uses grapheme segmentation. | Specify renderer clipping behavior in #26. |
| ANSI-aware clipping | Still needs explicit support or an unsupported-contract decision. | Track in #26. |
| Resize cleanup | Needs compatibility tests for width shrink and stale cells. | Track in #26/#27. |

## Active Issues

- #26 defines the terminal compatibility, Unicode, and ANSI behavior contract.
- #27 expands harness support for resize, input, mouse, paste, focus, and golden
  app flows.

## Historical Context

The original comparison against Ink and Bubbletea was useful for discovery, but
some entries are now stale. New terminal work should start from source behavior,
tests, and the maturity spec instead of this historical checklist.
