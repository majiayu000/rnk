# Tasks: GH49

Linked issue: https://github.com/majiayu000/rnk/issues/49

- [x] `SP49-T1` Owner: Codex. Done when: `CORE_COMPONENT_CONTRACTS.md` 记录 `MultiSelect`、`Confirm`、`FilePicker`、`ColorPicker`，且 `INTERACTIVE_COMPONENT_CONTRACTS.md` 说明这些组件有 core contract anchor。 Verify: `rg -n "MultiSelect|Confirm|FilePicker|ColorPicker|Paginator" docs/CORE_COMPONENT_CONTRACTS.md docs/INTERACTIVE_COMPONENT_CONTRACTS.md`
- [x] `SP49-T2` Owner: Codex. Done when: `tests/core_component_contracts.rs` 覆盖四个新增组件，且每个测试覆盖 value/submit/cancel 或 disabled/read-only mode。 Verify: `cargo test --test core_component_contracts --locked`
- [x] `SP49-T3` Owner: Codex. Done when: Focused test、workspace lib test、workspace check 通过。 Verify: `cargo test --workspace --lib --locked && cargo check --workspace --locked`
