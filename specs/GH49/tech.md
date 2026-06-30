# Tech Spec: 扩展交互输入组件合同

Linked issue: #49 (https://github.com/majiayu000/rnk/issues/49)

## 1. 设计

本 PR 只补文档和 tests。生产代码已有以下 public state/handler：

1. `MultiSelectState` + `handle_multi_select_input(...)`
2. `ConfirmState` + `handle_confirm_input_with_mode(...)`
3. `FilePickerState` + `handle_file_picker_input(...)`
4. `ColorPickerState` + `handle_color_picker_input(...)`

这些 handler 已经返回 `InteractionOutcome<T>`，支持 `InteractionMode`，适合进入
contract test。

## 2. 影响文件

1. `docs/CORE_COMPONENT_CONTRACTS.md`
   - 增加 audited input extensions 表。
   - 每行写 controlled/uncontrolled、keyboard、outcome、mode 行为和 test anchor。
2. `docs/INTERACTIVE_COMPONENT_CONTRACTS.md`
   - 在 testing contract 中列出 core contract tests 覆盖的扩展组件。
3. `tests/core_component_contracts.rs`
   - 增加 `MultiSelect`、`Confirm`、`FilePicker`、`ColorPicker` focused tests。
4. `specs/GH49/*`
   - 保存 issue-specific product/tech/tasks。

## 3. Deferred 组件

1. `Paginator` 有 state/handler，但 handler 当前返回 `bool`，应单独 issue 决定是否新增
   `InteractionOutcome` handler。
2. `ContextMenu` 有 state/test，但缺 pure handler。
3. `CodeEditor` 是 builder/render component，没有独立 state/handler contract。

## 4. 验证

1. `cargo test --test core_component_contracts --locked`
2. `cargo test --workspace --lib --locked`
3. `cargo check --workspace --locked`

## 5. 回滚

回滚本 PR 会移除新增合同文档和测试，不影响 production API。
