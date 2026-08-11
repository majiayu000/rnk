# rnk Examples

This index separates examples by intent. CI builds the example set through the
workspace gates, so files listed here should stay deterministic enough to compile
without local terminal assumptions.

<!-- gh68-public-chat-examples-v1
{"schema":"gh68-public-chat-examples-v1","records":[{"category":"tutorial","example":"chat.rs","purpose":"minimal typed conversation and composer","runtime_mode":"offline_inline","target_reader":"new_chat_user"},{"category":"showcase","example":"rnk_chat.rs","purpose":"row-measured fullscreen transcript and focus routing","runtime_mode":"interactive_fullscreen","target_reader":"fullscreen_app_author"},{"category":"showcase","example":"claude_input_box.rs","purpose":"native scrollback and explicit commit resolution","runtime_mode":"interactive_inline","target_reader":"inline_app_author"},{"category":"showcase","example":"glm_chat.rs","purpose":"provider adapter and default-deny workspace tools","runtime_mode":"provider_inline","target_reader":"provider_integrator"}]}
-->

## Tutorial

Start here when learning the crate API:

- `hello.rs`: minimal render example.
- `counter.rs`: state and input basics.
- `todo.rs`: compact todo interaction.
- `todo_app.rs`: larger todo application structure.
- `inline_mode.rs`: inline rendering mode.
- `use_input.rs`: keyboard input hook.
- `use_focus.rs`: focus traversal.
- `use_focus_with_id.rs`: explicit focus IDs.
- `use_stdio.rs`: stdio hooks.
- `typed_cmd_demo.rs`: typed command workflow.
- `chat.rs`: smallest typed conversation/composer/view tutorial.

## Showcase

These are app-shaped examples that demonstrate larger workflows:

- `rnk_top.rs`: system monitor-style dashboard.
- `rnk_git.rs`: Git status interface.
- `rnk_chat.rs`: `FullscreenChatShell` showcase with snapshot-measured,
  row-clipped messages.
- `glm_chat.rs`: typed GLM provider adapter with default-deny, one-shot
  workspace tools and `InlineChatShell` publication.
- `claude_input_box.rs`: `InlineChatShell` commit-outcome showcase; only a
  confirmed `Fixed` result clears the draft.
- `inline_chat_scrollback.rs`: inline chat committing finished transcript into
  the terminal's own scrollback, exactly once per message.
- `fullscreen_chat_shell.rs`: fullscreen chat regions — scrolling transcript,
  fixed composer, fixed status bar — and what happens when the terminal is too
  short to hold them.
- `interactive_demo.rs`: mixed interaction demo.
- `textarea_demo.rs`: text editing surface.
- `viewport_demo.rs`: scrollable viewport surface.

### Chat example review

Each chat example was reviewed against GH-68's convergence criteria:

| Example | Outcome |
|---|---|
| `claude_input_box.rs` | Uses `InlineChatShell`; `Fixed`, `Retained`, and `Latched` remain distinct and only `Fixed` clears the composer. |
| `claude_inline_input_box.rs` | Removed. It was byte-identical to `claude_input_box.rs` apart from its own name, so it had no independent purpose. |
| `glm_chat.rs` | Uses typed conversation/view/shell state; missing keys create zero requests and tool execution is exact-call approved, one-shot, bounded, and workspace-contained. The retired `glm_chat/prompt_box.rs` has no independent contract. |
| `chat.rs` | Uses public `ConversationState`, typed updates, `ChatMessageView`, and the shared composer projection. No private transcript or cursor arithmetic remains. |
| `rnk_chat.rs` | Uses `FullscreenChatShell`, `MessageList`, and `ChatMessageView`; root snapshot rows and exact visible slices replace item paging. |
| `inline_chat_scrollback.rs` | Added by #66. Drives `InlineChatShell` end to end: streaming, a repeated terminal event, and a commit into native scrollback. |
| `fullscreen_chat_shell.rs` | Added by #67. Drives `FullscreenChatShell`: real wrapped row counts, region assignment, resize, and a refused layout. |

What no chat example implements for itself any more: Unicode wrapping, visual
cursor placement, delta concatenation, message height, bottom-follow, or
scrollback commit. Each is a library concern with its own tests, and each was
wrong in at least one example before it moved.

Two behaviours are worth naming, because both only misbehave on real input:

- Deletion is by grapheme cluster. `String::pop` removes one `char`, which takes
  a piece off the end of a ZWJ emoji or a combining sequence and leaves a
  different character behind.
- Scrolling is by terminal row. `.skip(n).take(12)` pages by message count, so a
  four-row paragraph and a one-row acknowledgement move the viewport by the same
  amount and it never lands where the reader expects.

## Component Demos

These examples focus on individual components or visual primitives:

- `adaptive_colors_demo.rs`
- `aria.rs`
- `borders.rs`
- `box_backgrounds.rs`
- `confirm_demo.rs`
- `cursor_demo.rs`
- `file_picker_demo.rs`
- `fixed_bottom_demo.rs`
- `gradient_demo.rs`
- `help_demo.rs`
- `hyperlink_demo.rs`
- `justify_content.rs`
- `keys_demo.rs`
- `layout_demo.rs`
- `macros_demo.rs`
- `message_demo.rs`
- `mouse_demo.rs`
- `multi_select_demo.rs`
- `notification_demo.rs`
- `paginator_demo.rs`
- `paste_demo.rs`
- `rich_text.rs`
- `select_demo.rs`
- `select_input.rs`
- `spring_demo.rs`
- `static_demo.rs`
- `streaming_demo.rs`
- `table.rs`
- `terminal_resize.rs`
- `theme_demo.rs`
- `timer_demo.rs`
- `tree_demo.rs`

## Debug

These files exist for regression checks, runtime probes, or narrow behavior
inspection:

- `crlf_test.rs`
- `cross_thread.rs`
- `debug.rs`
- `exact_app_test.rs`
- `fullscreen_test.rs`
- `incremental_rendering.rs`
- `jest.rs`
- `println_element.rs`
- `render_api_demo.rs`
- `sage_exact.rs`
- `simple_test.rs`
- `static_example.rs`
- `subprocess_output.rs`
- `terminal_test.rs`

## Internal

Internal-only or experimental examples live under:

- `internal/`
