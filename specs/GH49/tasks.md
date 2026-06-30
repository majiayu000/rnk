# Tasks: GH49

Linked issue: https://github.com/majiayu000/rnk/issues/49

## SP49-T1 写组件合同文档

Owner: Codex

Done when:
- `CORE_COMPONENT_CONTRACTS.md` 记录 `MultiSelect`、`Confirm`、`FilePicker`、`ColorPicker`。
- `INTERACTIVE_COMPONENT_CONTRACTS.md` 说明这些组件有 core contract anchor。

Verify:
- `rg -n "MultiSelect|Confirm|FilePicker|ColorPicker|Paginator" docs/CORE_COMPONENT_CONTRACTS.md docs/INTERACTIVE_COMPONENT_CONTRACTS.md`

## SP49-T2 增加跨组件合同测试

Owner: Codex

Done when:
- `tests/core_component_contracts.rs` 覆盖四个新增组件。
- 每个测试覆盖 value/submit/cancel 或 disabled/read-only mode。

Verify:
- `cargo test --test core_component_contracts --locked`

## SP49-T3 验证

Owner: Codex

Done when:
- Focused test、workspace lib test、workspace check 通过。

Verify:
- `cargo test --workspace --lib --locked`
- `cargo check --workspace --locked`
