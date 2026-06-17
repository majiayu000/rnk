# Terminal Compatibility

This document defines the current terminal behavior contract for `rnk`.
It separates behavior guaranteed by the library from behavior that depends on
the user's terminal emulator, shell, multiplexer, operating system, or CI
environment.

## Status Terms

| Term | Meaning |
|------|---------|
| Implemented | `rnk` emits or handles the feature and has source-level tests where practical. |
| Best effort | `rnk` emits standard terminal sequences, but the terminal decides the final behavior. |
| Terminal-dependent | Behavior is outside `rnk` control and varies by emulator or configuration. |
| Unsupported | `rnk` does not currently provide a safe contract for this behavior. |
| Not automatically tested | The behavior needs a real terminal or emulator and is not proven by CI. |

## Environment Matrix

The matrix records the contract that `rnk` can make today. It is not a claim
that every listed terminal has been manually certified on every platform.

| Environment | Inline / fullscreen | Mouse and bracketed paste | Hyperlinks and color | Resize behavior | Non-TTY and CI |
|-------------|---------------------|---------------------------|----------------------|-----------------|----------------|
| macOS Terminal | Implemented by inline diff rendering and alternate-screen sequences; final alternate-screen behavior is terminal-dependent. | Best effort through crossterm enable/disable commands. | Color sequences are emitted; OSC 8 hyperlinks are terminal-dependent and may fall back to plain text. | Implemented repaint on size change; terminal wrapping policy remains terminal-dependent. | Use normal TTY path when attached to a terminal. |
| iTerm2 | Implemented by inline diff rendering and alternate-screen sequences. | Best effort; iTerm2 commonly supports both, but `rnk` treats this as terminal-dependent. | Best effort for colors and OSC 8 hyperlinks. | Implemented repaint on size change. | Use normal TTY path when attached to a terminal. |
| Windows Terminal | Implemented through crossterm and ANSI-compatible terminal behavior. | Best effort through crossterm; input mode support depends on Windows terminal state. | Best effort for ANSI colors and OSC 8 hyperlinks. | Implemented repaint on size change. | Use normal TTY path when attached to a terminal. |
| tmux | Implemented when the pane passes through the required ANSI behavior. | Terminal-dependent; tmux mode, mouse settings, and paste forwarding can change behavior. | Terminal-dependent; OSC 8 and truecolor may require tmux configuration. | Implemented repaint in the application; tmux pane resize behavior is external. | Use normal TTY path inside tmux. |
| SSH / remote TTY | Implemented when stdin/stdout are TTYs and the remote `$TERM` supports the emitted sequences. | Terminal-dependent across SSH client, server, `$TERM`, and multiplexer settings. | Terminal-dependent. | Implemented repaint in `rnk`; remote terminal resize delivery is external. | Non-TTY pipes and redirects should use non-interactive rendering paths. |
| Non-TTY / CI | Interactive terminal mode is unsupported as an end-user experience. | Unsupported. | Structured rendering and snapshots are supported; emulator-specific behavior is not automatically tested. | Source-level resize decisions are tested; real emulator resize is not automatically tested. | `Environment` detects CI and TTY state so apps can disable interactive behavior. |

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
