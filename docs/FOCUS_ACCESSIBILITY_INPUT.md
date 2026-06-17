# Focus, Accessibility, And Input Semantics

This document defines the focus traversal, keyboard, and accessibility metadata
contract for `rnk` interactive components.

## Focus Traversal

`UseFocusOptions` keeps the original global focus registration shape.
`ScopedFocusOptions` adds traversal metadata for scoped focus registration:

| Option | Contract |
| --- | --- |
| `id` | Stable application ID for direct `focus(id)` calls. |
| `scope` | Required traversal group for `ScopedFocusOptions`. Scoped navigation ignores focusable elements outside the group. |
| `focus_order` | Optional order within the scope. Lower values receive focus first; registration order breaks ties. |
| `auto_focus` | Focuses the element on mount when no active focus exists. |
| `is_active` | Removes disabled or hidden controls from traversal. |

Default traversal is opt-in. Apps can install `use_focus_traversal()` for global
Tab and Shift-Tab movement, or `use_focus_traversal_in_scope(scope)` when a
dialog, form, palette, or panel should trap focus locally.

Use `use_focus(UseFocusOptions::new())` for global registration and
`use_scoped_focus(ScopedFocusOptions::new(scope))` for local traversal. The
manager keeps the previous unscoped APIs (`register`, `update`, `focus_next`,
`focus_previous`, `focus`) for compatibility. New scoped operations are
additive: `register_with_options`, `update_with_options`,
`focus_next_in_scope`, and `focus_previous_in_scope`.

## Keyboard Conventions

Common components use these key meanings:

| Key | Contract |
| --- | --- |
| `Tab` | Move focus forward when traversal is installed; components should not treat BackTab as plain Tab. |
| `Shift+Tab` / terminal `BackTab` | Move focus backward when traversal is installed. |
| `Escape` | Cancel or close the current interaction when the component supports cancellation. |
| `Enter` | Submit the focused value or action when enabled. |
| Arrow keys | Move cursor, highlight, selection, or viewport position. |
| `Space` | Toggle or submit where the component documents that behavior. |

`Key` exposes `tab` and `back_tab` separately. `KeyBinding` also has
`KeyType::BackTab`; legacy tests that construct `Key { tab: true, shift: true,
.. }` still match BackTab bindings for compatibility.

## Accessibility Metadata

`Element` can carry `AccessibilityProps`:

| Field | Contract |
| --- | --- |
| `role` | Semantic role such as `TextInput`, `Select`, `MultiSelect`, `Dialog`, `Menu`, `ColorPicker`, `FilePicker`, `TextArea`, or `Viewport`. |
| `label` | Short readable name. |
| `description` | Longer context such as option counts or action hints. |
| `disabled` | Mirrors disabled interaction mode where the rendered component stores that mode. |
| `read_only` | Mirrors read-only interaction mode where available. |
| `focusable` | Whether the root should participate in focus semantics. |
| `selected` | Optional selection state for option-like elements. |
| `value` | Current value or highlighted/selected summary. |

Interactive roots now attach metadata for `TextInput`, `SelectInput`,
`MultiSelect`, `Confirm`, `ColorPicker`, `CommandPalette`, `FilePicker`,
`TextArea`, and `Viewport`. Components that expose mode only through pure
handler functions document disabled/read-only behavior in
`INTERACTIVE_COMPONENT_CONTRACTS.md`; their render-only roots may expose only
the focusable/value metadata available at render time.

## Accessible Fallback Text

`Element::accessible_text()` returns a readable fallback assembled from
accessibility label, value, description, text content, and descendants. It is
intended for tests, screen-reader oriented fallbacks, and non-interactive
inspection. It does not replace terminal-native accessibility APIs, which vary
by emulator and operating system.

## Test Coverage

The current unit coverage locks:

- scoped focus traversal and focus order
- separate Tab and BackTab key semantics
- `Shortcut::tab()` versus `Shortcut::shift_tab()`
- `Element::accessible_text()` fallback behavior
- component roots compiling with accessibility metadata

Full runtime focus, paste, mouse, resize, and app-shaped interaction simulation
is tracked separately by issue #27.
