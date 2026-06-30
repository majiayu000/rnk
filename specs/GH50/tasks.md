# Tasks: GH50

Linked issue: https://github.com/majiayu000/rnk/issues/50

- [x] `SP50-T1` Owner: Codex. Done when: `src/lib.rs` public modules 标注 API level，且 crate-level API levels 与 policy 一致。 Verify: `rg -n "Advanced extension surface|Experimental|Stable app surface" src/lib.rs`
- [x] `SP50-T2` Owner: Codex. Done when: `src/prelude.rs` 说明 full/lite/widgets/testing，且 README 告诉新用户从 prelude 开始。 Verify: `rg -n "prelude::lite|prelude::widgets|prelude::testing|lower-level modules" src/prelude.rs README.md`
- [x] `SP50-T3` Owner: Codex. Done when: `docs/API_STABILITY.md` 包含 root re-export 细节和 public-module guidance。 Verify: `rg -n "render_handle|AccessibilityProps|AccessibilityRole|root compatibility" docs/API_STABILITY.md`
- [x] `SP50-T4` Owner: Codex. Done when: docs/check/fmt 通过。 Verify: `cargo fmt --all -- --check && cargo doc --workspace --no-deps --all-features --locked && cargo check --workspace --locked`
