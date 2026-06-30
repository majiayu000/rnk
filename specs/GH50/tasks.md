# Tasks: GH50

Linked issue: https://github.com/majiayu000/rnk/issues/50

## SP50-T1 加固 crate root rustdoc

Owner: Codex

Done when:
- `src/lib.rs` public modules 标注 API level。
- crate-level API levels 与 policy 一致。

Verify:
- `rg -n "Advanced extension surface|Experimental|Stable app surface" src/lib.rs`

## SP50-T2 加固 prelude 和 README 入口说明

Owner: Codex

Done when:
- `src/prelude.rs` 说明 full/lite/widgets/testing。
- README 告诉新用户从 prelude 开始。

Verify:
- `rg -n "prelude::lite|prelude::widgets|prelude::testing|lower-level modules" src/prelude.rs README.md`

## SP50-T3 同步 API stability policy

Owner: Codex

Done when:
- `docs/API_STABILITY.md` 包含 root re-export 细节和 public-module guidance。

Verify:
- `rg -n "render_handle|AccessibilityProps|AccessibilityRole|root compatibility" docs/API_STABILITY.md`

## SP50-T4 验证

Owner: Codex

Done when:
- docs/check/fmt 通过。

Verify:
- `cargo fmt --all -- --check`
- `cargo doc --workspace --no-deps --all-features --locked`
- `cargo check --workspace --locked`
