# Product Spec: WarpUI 学习边界和开源工作流定位

Linked issue: https://github.com/majiayu000/rnk/issues/47

## 1. 背景

WarpUI 是 Warp 产品仓库里的 UI/渲染基础设施，不是面向第三方 `cargo add`
的 TUI crate。`rnk` 应学习它的产品化表达、issue 到 spec 到 PR 的队列、
组件合同和发布信任，但不能把 terminal-first 范围扩大成 GPU/windowing
框架。

## 2. 目标

1. 文档明确 `rnk` 与 WarpUI 的关系：不是直接竞品，而是 third-party
   terminal UI crate 与 product-internal UI engine 的区别。
2. README 的定位继续强调 Rust terminal apps、agent UIs、dashboards、
   forms 和 chat-style tools。
3. 贡献流程提供轻量 `ready-to-spec` / `ready-to-implement` 队列说明，支持
   一 issue 一 PR 的 SpecRail 工作方式。
4. 新 issue 模板能引导维护者和贡献者写出目标、非目标、验收标准和验证命令。

## 3. 非目标

1. 不引入 GPU renderer、native windowing、WASM app shell 或 desktop GUI 范围。
2. 不把 WarpUI 描述成可直接替换或直接依赖的 crates.io 库。
3. 不承诺自动审批、自动 merge、自动发布或绕过 maintainer gate。

## 4. 验收标准

1. `docs/COMPARISON.md` 包含 Warp/WarpUI 小节，并解释何时看 WarpUI、何时选
   `rnk`。
2. `README.md` 的 `When To Use rnk` 说明 terminal-first 范围和非目标。
3. `CONTRIBUTING.md` 描述 issue-first、spec-first、one issue one PR 的贡献路径。
4. `.github/ISSUE_TEMPLATE` 增加 spec/implementation request 模板。
5. 文档变更通过 Markdown 链接和格式的基本静态检查。
