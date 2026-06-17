# Design Tokens And Component Variants

`rnk` now exposes a small design-token layer on top of `Theme` without changing
the public field layout of `Theme`.

## Compatibility Boundary

`Theme` remains a color-bearing public struct. Non-color tokens are resolved with
`Theme::design_tokens()` instead of being stored as new public fields. This keeps
existing struct-literal construction working while giving components a shared
contract for spacing, density, borders, focus, states, symbols, and variants.

The token and action APIs are exported from `rnk::components`. They are not in
the prelude yet; application examples should opt in with narrow imports until the
contract has more usage.

## Token Groups

`DesignTokens` contains:

- `SpacingTokens`: terminal-cell spacing scale.
- `DensityTokens`: density mode plus default gap and padding.
- `BorderTokens`: panel, dialog, control, and focus border styles.
- `FocusTokens`: focus marker and bold behavior.
- `StateTokens`: selected, disabled, and active emphasis behavior.
- `SymbolTokens`: shared selected, status, and disclosure symbols.

The default token set keeps current terminal output compatible, including the
existing Unicode status symbols used by notifications and toasts.

## Variants And States

Theme-aware components can resolve:

- `ComponentVariant`: default, primary, secondary, success, warning, error, info.
- `ComponentState`: rest, focused, selected, disabled, active.
- `Theme::variant_style(variant, state)`: resolved foreground, background,
  border color, and emphasis.

Capsule-like components resolve through these shared variant/state paths instead
of owning separate palettes. Status styles resolve through the theme and shared
symbol tokens; `Info` intentionally keeps the historical primary-accent
foreground for compatibility with existing notification/toast output.

## Action Primitive

`ActionButton` is the shared primitive for button-like labels inside higher-level
components. It combines:

- `ActionRole`: primary, secondary, destructive.
- `ActionState`: rest, focused, disabled.
- `ActionShape`: brackets, angles, parens, plain, padded.
- `Theme::action_style(role, state)`: resolved action colors and emphasis.

`Confirm` and `Dialog` both render their button labels through this primitive.
Existing per-component color setters still override resolved theme colors.

## Current Limits

This pass intentionally avoids changing the public `Theme` fields. Custom
non-color token injection can be added later with a compatibility plan, for
example through a separate theme extension object or a future minor-version
migration.

Not every component has been retokenized. The current guarantee is that shared
actions, status styles, and capsule variants have one resolver path, and new
components should prefer the token APIs instead of adding hard-coded palettes or
symbols.
