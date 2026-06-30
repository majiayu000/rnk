# Tech Spec: pre-1.0 公共 API 边界加固

Linked issue: https://github.com/majiayu000/rnk/issues/50

## 1. 设计

采用非破坏性文档加固：

1. 给 `src/lib.rs` 的 public modules 加 rustdoc comments，标注 stable /
   advanced / experimental。
2. 更新 crate-level docs 的 API level 说明，使 docs.rs 第一屏与 policy 一致。
3. 更新 `src/prelude.rs` module docs，解释 full prelude、lite、widgets、testing 的关系。
4. 更新 `docs/API_STABILITY.md`，补齐 `AccessibilityProps`、`AccessibilityRole`、
   `render_handle`、`render_to_string*` 等 root compatibility details。
5. 更新 README，使新用户路径更明确。

## 2. 影响文件

1. `src/lib.rs`
2. `src/prelude.rs`
3. `docs/API_STABILITY.md`
4. `README.md`
5. `specs/GH50/*`

## 3. 风险

1. 文档可能暗示 advanced modules 不可用。
   - 缓解：明确它们 public and useful，但不是 preferred application surface。
2. docs.rs warnings 可能因 intra-doc links 触发。
   - 缓解：使用 plain code identifiers，不新增 fragile relative links。

## 4. 验证

1. `rg -n "Stable app surface|Advanced extension surface|Experimental|prelude::widgets|render_handle" src/lib.rs src/prelude.rs docs/API_STABILITY.md README.md`
2. `cargo fmt --all -- --check`
3. `cargo doc --workspace --no-deps --all-features --locked`
4. `cargo check --workspace --locked`

## 5. 回滚

回滚本 PR 只会移除文档加固，不影响 production behavior 或 public API。
