# Product Spec: 扩展交互输入组件合同

Linked issue: https://github.com/majiayu000/rnk/issues/49

## 1. 背景

`rnk` 已为 `Box`、`Text`、`TextInput`、`SelectInput`、`TextArea` 和
`CommandPalette` 建立核心组件合同。其它交互输入组件已有不少 handler 和单测，
但没有进入跨组件合同文档与 `tests/core_component_contracts.rs` 的用户可依赖测试锚点。

## 2. 目标

1. 将 `MultiSelect`、`Confirm`、`FilePicker` 和 `ColorPicker` 提升为已审计输入组件。
2. 对每个组件记录 controlled state、uncontrolled/render usage、keyboard contract、
   `InteractionOutcome<T>` payload、disabled/read-only 行为和测试锚点。
3. 为每个组件增加 focused contract test，覆盖至少一个 value path 和一个 mode path。
4. 明确 `Paginator`、`ContextMenu`、`CodeEditor` 等 deferred 组件不在本 PR 范围。

## 3. 非目标

1. 不修改 production handler 行为。
2. 不把所有 45+ 组件都提升为推荐 beginner set。
3. 不为 `Paginator` 改返回类型；该组件需要单独 issue 讨论 `InteractionOutcome` 迁移。
4. 不新增 callback builder API；当前合同以 pure handler outcome 为准。

## 4. 验收标准

1. `docs/CORE_COMPONENT_CONTRACTS.md` 记录四个新增 audited input components。
2. `docs/INTERACTIVE_COMPONENT_CONTRACTS.md` 标注这些组件已有 core contract test anchor。
3. `tests/core_component_contracts.rs` 新增四个测试并通过。
4. `cargo test --test core_component_contracts --locked` 通过。
5. `cargo test --workspace --lib --locked` 通过。
