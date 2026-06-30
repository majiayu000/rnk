# Tasks: GH47

Linked issue: https://github.com/majiayu000/rnk/issues/47

## SP47-T1 更新比较文档

Owner: Codex

Done when:
- `docs/COMPARISON.md` 增加 Warp/WarpUI 小节。
- 文档说明 `rnk` 与 WarpUI 的边界和可学习点。

Verify:
- `rg -n "WarpUI|product-internal|terminal-first" docs/COMPARISON.md`

## SP47-T2 更新 README 定位

Owner: Codex

Done when:
- `README.md` 的 `When To Use rnk` 说明 third-party terminal app 范围。
- README 不暗示 `rnk` 要覆盖 native GUI/windowing。

Verify:
- `rg -n "terminal-first|WarpUI|native window" README.md`

## SP47-T3 更新贡献流程和 issue 模板

Owner: Codex

Done when:
- `CONTRIBUTING.md` 说明 issue-first/spec-first/one issue one PR。
- `.github/ISSUE_TEMPLATE/spec_request.md` 存在并包含验收标准和验证命令字段。

Verify:
- `rg -n "ready-to-spec|ready-to-implement|one issue" CONTRIBUTING.md .github/ISSUE_TEMPLATE`

## SP47-T4 验证

Owner: Codex

Done when:
- 文档搜索命令通过。
- `cargo check --workspace --locked` 通过。
