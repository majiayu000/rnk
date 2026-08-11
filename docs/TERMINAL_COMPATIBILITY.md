# Terminal Compatibility

This document defines the current terminal behavior contract for `rnk`.
It separates behavior guaranteed by the library from behavior that depends on
the user's terminal emulator, shell, multiplexer, operating system, or CI
environment.

## Status Terms

| Code | Meaning |
|------|---------|
| `verified` | The named evidence ran on the recorded source head and environment. |
| `best_effort` | `rnk` emits standard sequences, but the terminal decides the result. |
| `terminal_dependent` | Behavior varies by emulator, transport, or configuration. |
| `unsupported` | `rnk` has no safe contract for this behavior. |
| `unverified` | No current-head evidence establishes the behavior. |

## Environment Matrix

The matrix uses only the closed status vocabulary above. Each required
dimension appears exactly once; a row does not imply broader certification.

| Dimension | Status | Current boundary |
|---|---|---|
| OS | `verified` | The artifact records the CI runner OS and architecture for the exact head. |
| Terminal emulator | `unverified` | No named emulator was exercised by current-head automation. |
| Inline | `verified` | Inline commit and PTY lifecycle tests ran on the recorded runner. |
| Fullscreen | `verified` | Fullscreen layout and PTY lifecycle tests ran on the recorded runner. |
| Paste | `verified` | Runtime dispatch and grapheme-safe composer paste tests ran; emulator forwarding is not certified. |
| Resize | `verified` | Overflow, paused-anchor, draft, and message-order transitions ran. |
| Raw restoration | `verified` | Complete captured termios state plus cursor/mouse/screen sequences were checked. |
| tmux | `unverified` | No current-head tmux session evidence exists. |
| SSH | `unverified` | No current-head remote TTY evidence exists. |

## Chat Evidence Matrix

The chat checks bind structured output to the exact source revision under test.
They do not promote source or golden evidence into a claim about an emulator
that CI did not run. CI records its current `GITHUB_SHA`, runner environment and
the named evidence below in the job artifact.
These compatibility checks use no network or secret.

<!-- gh68-terminal-matrix-v1
{"schema":"gh68-terminal-matrix-v1","cells":[{"evidence":"runner.os+runner.arch","id":"os","status":"verified"},{"evidence":"none","id":"terminal_emulator","status":"unverified"},{"evidence":"gh68_inline_example_contract","id":"inline","status":"verified"},{"evidence":"gh68_fullscreen_example_contract","id":"fullscreen","status":"verified"},{"evidence":"gh68_chat_tutorial_contract+test_event_loop_paste_dispatch_requests_render","id":"paste","status":"verified"},{"evidence":"gh68_fullscreen_example_contract","id":"resize","status":"verified"},{"evidence":"gh68_fullscreen_example_contract+gh68_inline_example_contract","id":"raw_restoration","status":"verified"},{"evidence":"none","id":"tmux","status":"unverified"},{"evidence":"none","id":"ssh","status":"unverified"}]}
-->

| Dimension | Evidence kind | Environment and head binding | Status |
|---|---|---|---|
| OS | Recorded runner OS and architecture | CI runner; exact checked-out `GITHUB_SHA` | `verified` |
| Terminal emulator | No emulator-specific run | None | `unverified` |
| Inline | Commit outcomes and PTY lifecycle | CI runner; exact checked-out `GITHUB_SHA` | `verified` |
| Fullscreen | Public shell layout and PTY lifecycle | CI runner; exact checked-out `GITHUB_SHA` | `verified` |
| Paste | Runtime event dispatch, selection replacement, and grapheme deletion | CI runner; exact checked-out `GITHUB_SHA` | `verified` |
| Resize | Overflowing paused viewport with interleaved draft input | CI runner; exact checked-out `GITHUB_SHA` | `verified` |
| Raw restoration | Complete termios snapshot and output-sequence checks | CI runner; exact checked-out `GITHUB_SHA` | `verified` |
| tmux | No tmux session run | None | `unverified` |
| SSH | No remote TTY run | None | `unverified` |

No cell in this matrix is a macOS Terminal, iTerm2, Windows Terminal, tmux or
SSH certification. A real-terminal evidence record may say `verified` only when
it also names the evidence kind, environment and exact source head; the current
automated matrix intentionally makes no such claim.

## Terminal Features

Inline rendering is implemented by updating the current terminal position and
clearing changed or removed lines. Shorter replacement lines are erased before
the replacement text is written, so stale suffix cells are not part of the
managed output line-diff contract.

Fullscreen rendering uses the alternate screen buffer. Entering and leaving
alternate screen is implemented by crossterm or ANSI sequences, but the exact
scrollback and restoration behavior belongs to the terminal emulator.

Mouse input and bracketed paste are best-effort terminal modes. `rnk` can request
those modes and dispatch events when the terminal sends them. It cannot force an
emulator, SSH client, or tmux pane to support or forward those events.
The interactive chat examples hold a `BracketedPasteGuard` for their complete
terminal session, so both normal return and unwind disable the input mode.

Hyperlinks use OSC 8 when hyperlink support is detected or explicitly enabled.
When support is disabled, the hyperlink component renders fallback text. OSC 8
clickability is terminal-dependent.

Colors are emitted as terminal style sequences through structured `Text` and
`Span` styling. The terminal controls palette, contrast, theme remapping, and
truecolor fidelity.

## Unicode Text Contract

Measurement, wrapping, and truncation in `layout::measure` are grapheme-aware.
They use Unicode grapheme clusters and display width, so combining marks and
emoji sequences are handled as clusters rather than independent scalar values.

Renderer output is terminal-cell based. `renderer::Output` writes `char` values
using `UnicodeWidthChar`; wide characters occupy two cells and the second cell is
stored internally as a placeholder. A wide character that cannot fit at the right
edge is not split across cells.

Viewport clipping is cell-offset based. If a horizontal offset lands inside a
wide character, the whole wide character is skipped. If a wide character would
overflow the right edge of the viewport, it is omitted instead of being split.

Ambiguous-width Unicode characters are resolved by the Unicode width crate and
may not match every terminal's locale-specific rendering choice. Applications
that need exact CJK ambiguous-width behavior should test against their target
terminal configuration.

## ANSI And Raw Escape Sequences

Structured styling through `Text`, `Span`, and renderer `Style` is supported.
Those styles are not treated as visible text for layout by application code.

Raw ANSI, SGR, or OSC escape sequences embedded inside plain text are unsupported
for layout measurement, wrapping, truncation, and viewport clipping. `rnk` does
not currently parse escape sequences before width calculation, so escape
sequence characters and payload can be measured or clipped like normal text.

Components that intentionally produce raw terminal sequences, such as Markdown
styling or OSC 8 hyperlinks, are direct terminal-output helpers. They should not
be used as width-safe input to layout-sensitive text measurement unless the app
accepts terminal-dependent behavior.

## Resize Contract

When the observed terminal width or height changes, `TerminalController` marks
the terminal for repaint. In fullscreen mode it also clears the alternate screen
before repainting, which removes stale cells from prior wider frames. In inline
mode it repaints without clearing the whole scrollback.

Line-level rendering clears changed lines before writing new text and clears
rows that disappeared from the previous frame. This covers shorter replacement
lines and stale rows in the application's managed output region.

CI tests cover the source-level resize decision and buffer clipping behavior.
They do not prove real emulator resize delivery, tmux forwarding, or terminal
scrollback restoration.
