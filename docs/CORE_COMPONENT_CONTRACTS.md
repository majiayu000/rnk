# Core Component Contracts

This document defines the first component set `rnk` should present to new
users. The broader component library can grow independently, but these contracts
should stay clear, documented, and tested.

## Contract Terms

- Controlled usage means caller-owned state is passed through an explicit state
  type and input handler.
- Uncontrolled usage means the component owns state through a hook or internal
  signal.
- Keyboard contract means the keys that change state, submit, cancel, or
  navigate are documented.
- Callback contract means the observable output from input handling. In current
  `rnk`, handler functions return `InteractionOutcome<T>` instead of invoking a
  callback directly for most controlled widgets.
- Disabled mode ignores input. Read-only mode may allow navigation, but must not
  mutate values or submit data.

## Components

| Component | Controlled | Uncontrolled | Keyboard | Callback / Outcome | Disabled / Read-only | Test Anchor |
| --- | --- | --- | --- | --- | --- | --- |
| `Box` | Not stateful. Caller controls style and children through the builder. | Not applicable. | None. Layout-only component. | None. Render output is the observable behavior. | Not applicable. Use `hidden()` for display suppression. | `tests/core_component_contracts.rs` renders `Box` with `Text`. |
| `Text` | Not stateful. Caller controls content and spans through the builder. | Not applicable. | None. Display-only component. | None. Render output and style fields are observable. | Not applicable. | `tests/core_component_contracts.rs` renders styled text. |
| `TextInput` | `TextInputState` plus `handle_text_input(...)`. | `use_text_input(TextInputOptions)` returns a `TextInputHandle`. | Character input edits, arrows/Home/End move cursor, Enter submits, Escape cancels. | `InteractionOutcome<String>` returns changed value, submitted value, cancelled, handled, or ignored. | `disabled()` ignores all input. `read_only()` allows cursor movement but blocks edits and submit. | `tests/core_component_contracts.rs` covers edit, submit, read-only, and disabled behavior. |
| `SelectInput` | `SelectInputState` plus `handle_select_input(...)`. | `SelectInput::new(items)` can own local highlighted state while rendered. | Arrow or vim navigation moves highlight, Enter/Space submits, Escape cancels. | `InteractionOutcome<usize>` returns submitted item index or navigation handling. | Disabled ignores all input. Read-only allows highlight movement but blocks submit. | `tests/core_component_contracts.rs` covers navigation, submit, read-only, and disabled behavior. |
| `TextArea` | `TextAreaState` plus `handle_textarea_input_with_mode(...)`. | Apps can store `TextAreaState` in a signal and render `TextArea::new(&state)`. | Default keymap supports cursor movement, deletion, Enter, Tab, word movement, and line movement. | `InteractionOutcome<String>` returns changed content, cancel, handled, or ignored. | Disabled ignores input. Read-only allows non-editing movement but blocks content changes. | `tests/core_component_contracts.rs` covers edit, read-only, and disabled behavior. |
| `CommandPalette` | `CommandPaletteState` plus `handle_command_palette_input(...)`. | `CommandPalette::new(commands)` can render with owned builder state. | Character input filters, Up/Down moves selection, Enter submits enabled command, Escape closes. | `InteractionOutcome<String>` returns submitted command id, changed query, cancel, handled, or ignored. | Disabled ignores input. Read-only allows selection movement but blocks query edits and submit. Disabled commands cannot submit. | `tests/core_component_contracts.rs` covers query, disabled command, submit, read-only, and disabled behavior. |

`TextInput` is the component contract name. Its current public surface is
`TextInputState`, `TextInputOptions`, `TextInputHandle`, `use_text_input(...)`,
and `handle_text_input(...)`; there is not a standalone `TextInput` builder type
yet.

## Maintenance Rules

- Do not add a component to the recommended beginner set until its state,
  keyboard, outcome, and disabled/read-only behavior are documented.
- Do not claim callback support unless the component exposes either a direct
  callback builder or a deterministic `InteractionOutcome<T>` handler.
- Add an interaction or render test for every contract row that changes.
