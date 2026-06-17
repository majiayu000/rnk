# rnk Examples

This index separates examples by intent. CI builds the example set through the
workspace gates, so files listed here should stay deterministic enough to compile
without local terminal assumptions.

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

## Showcase

These are app-shaped examples that demonstrate larger workflows:

- `rnk_top.rs`: system monitor-style dashboard.
- `rnk_git.rs`: Git status interface.
- `rnk_chat.rs`: chat-style terminal application.
- `chat.rs`: compact chat interface.
- `glm_chat.rs` and `glm_chat/`: chat prompt surface.
- `claude_input_box.rs`: Claude-style input box.
- `claude_inline_input_box.rs`: inline Claude-style input box.
- `interactive_demo.rs`: mixed interaction demo.
- `textarea_demo.rs`: text editing surface.
- `viewport_demo.rs`: scrollable viewport surface.

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
