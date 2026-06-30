# Product Spec: pre-1.0 公共 API 边界加固

Linked issue: https://github.com/majiayu000/rnk/issues/50

## 1. 背景

`rnk` 已经推荐 `rnk::prelude::*`、`prelude::lite::*`、`prelude::widgets::*`
作为应用入口，但 crate root 仍公开 `core`、`components`、`hooks`、`layout`、
`renderer`、`runtime`、`testing` 等模块。pre-1.0 可以接受这个状态，但用户需要在
docs.rs 和 README 中清楚知道哪些 API 是 stable app surface，哪些是 advanced 或
experimental。

## 2. 目标

1. 新用户从 README 和 crate docs 都能看到推荐入口是 `rnk::prelude::*`。
2. docs.rs 上的 root modules 带有 stable / advanced / experimental 级别说明。
3. `docs/API_STABILITY.md` 与 `src/lib.rs` 的 root re-exports 保持一致。
4. 不删除已有 public modules，不破坏现有 pre-1.0 用户。

## 3. 非目标

1. 不把 `renderer`、`runtime`、`testing` 改成 private。
2. 不移除 root compatibility re-exports。
3. 不改变 `prelude` 实际导出集合。
4. 不承诺 1.0 semver；本 issue 只加固 pre-1.0 guidance。

## 4. 验收标准

1. `src/lib.rs` 的 public module rustdoc 明确 API 级别。
2. `src/prelude.rs` 说明 full/lite/widgets/testing 的推荐使用方式。
3. `docs/API_STABILITY.md` 包含 root compatibility surface 的实际 re-export 项。
4. `README.md` 的新用户路径提醒从 prelude 开始，lower-level modules 只用于 extension/integration/testing。
5. `cargo doc --workspace --no-deps --all-features --locked` 通过。
