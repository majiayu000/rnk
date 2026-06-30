# Tech Spec: WarpUI 学习边界和开源工作流定位

Linked issue: https://github.com/majiayu000/rnk/issues/47

## 1. 设计

本变更只触碰文档和 GitHub issue 模板，不改变 Rust API 或运行时代码。

## 2. 影响文件

1. `docs/COMPARISON.md`
   - 增加 Warp/WarpUI 对比小节。
   - 修正“known gaps”措辞，避免引用已经完成的旧 maturity tranche。
2. `README.md`
   - 在 `When To Use rnk` 中加入 terminal-first 范围和 WarpUI 非竞品边界。
3. `CONTRIBUTING.md`
   - 增加 SpecRail-lite 队列说明。
   - 明确 `ready-to-spec`、`ready-to-implement`、human review 和 one issue one PR。
4. `.github/ISSUE_TEMPLATE/spec_request.md`
   - 新增一个面向 spec/implementation 的 issue 模板。
5. `specs/GH47/*`
   - 保存本 issue 的 product/tech/tasks 规格。

## 3. 风险

1. 文档可能过度比较 WarpUI。缓解：明确它不是 direct competitor。
2. 贡献流程可能看起来过重。缓解：称为 lightweight SpecRail queue，不要求所有小改都写长 spec。

## 4. 验证

1. `rg -n "WarpUI|ready-to-spec|ready-to-implement" README.md docs/COMPARISON.md CONTRIBUTING.md .github/ISSUE_TEMPLATE`
2. `python3 .github/scripts/check_markdown_links.py README.md docs/COMPARISON.md CONTRIBUTING.md .github/ISSUE_TEMPLATE specs/GH47`
3. `cargo check --workspace --locked`

## 5. 回滚

回滚本 PR 即可移除新增文档和模板，不涉及数据迁移或 public API。
