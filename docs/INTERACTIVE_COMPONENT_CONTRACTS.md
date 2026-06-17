# Interactive Component Contracts

This document defines the shared state and event contract for interactive
components. Applications should prefer controlled state plus pure handlers when
they need testable behavior, and may keep using existing uncontrolled builders
for simple rendering.

## Shared Types

`InteractionMode` is the shared input gate:

| Mode | Contract |
| --- | --- |
| `Enabled` | Navigation, mutation, submit, and cancel are allowed. |
| `ReadOnly` | Focus and navigation are allowed where documented. Value mutation and submit are blocked. Cancel may still close or mark the interaction cancelled. |
| `Disabled` | Input is ignored and state is left unchanged. |

`InteractionOutcome<T>` is the shared handler result:

| Outcome | Contract |
| --- | --- |
| `Ignored` | The key was irrelevant or blocked by mode. |
| `Handled` | The key was consumed without a public value payload. |
| `Changed(T)` | The component value changed. |
| `Submitted(T)` | The component submitted a value. |
| `Cancelled` | The interaction was cancelled. |

## Component Matrix

| Component | Controlled state | Handler | Changed payload | Submitted payload | Read-only behavior |
| --- | --- | --- | --- | --- | --- |
| `TextInput` | `TextInputState` | `handle_text_input` | Current text | Current text | Cursor movement allowed; editing and submit blocked. |
| `SelectInput` | `SelectInputState` | `handle_select_input` | Navigation is `Handled` | Highlighted index | Highlight navigation allowed; submit blocked. |
| `MultiSelect` | `MultiSelectState` | `handle_multi_select_input` | Selected indices | Selected indices | Highlight navigation allowed; selection changes and submit blocked. |
| `Confirm` | `ConfirmState` | `handle_confirm_input_with_mode` | Not used | Boolean answer | Focus toggle allowed; answer submit blocked. |
| `FilePicker` | `FilePickerState` | `handle_file_picker_input` | Selected paths | Selected paths | Cursor and directory navigation allowed; search, selection, and submit blocked. |
| `ColorPicker` | `ColorPickerState` | `handle_color_picker_input` | Selected color | Selected color | Selection changes and submit blocked. |
| `CommandPalette` | `CommandPaletteState` | `handle_command_palette_input` | Query text | Command id | Command navigation allowed; query edits and submit blocked. |
| `TextArea` | `TextAreaState` | `handle_textarea_input_with_mode` | Full content | Not submitted by the base handler | Cursor movement and non-mutating actions allowed; text edits blocked. |
| `Viewport` | `ViewportState` | `handle_viewport_input_with_mode` | `(x_offset, y_offset)` | Not submitted | Scroll navigation allowed; disabled mode blocks scrolling. |

The original compatibility handlers remain available where they existed. New
code should use the `*_with_mode` or shared-outcome handler when it needs
disabled/read-only behavior or direct assertions in tests.

## Testing Contract

Each audited component has unit coverage for at least one meaningful value path
and one mode path:

- value edit or selection change
- submit or cancel where the component supports it
- disabled or read-only handling
- keyboard navigation through the pure handler

Full runtime key/mouse simulation belongs to the test harness workstream tracked
by issue #27.
