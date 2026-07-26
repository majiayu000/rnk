# Task Plan：线性 styled boundary 归一化

## Linked Issue

GH-127: https://github.com/majiayu000/rnk/issues/127

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Behavior set: `B-001` 至 `B-022`
- Planned implementation manifest:
  `src/layout/text_flow.rs`、`src/layout/text_flow/style_normalization.rs`、
  `src/layout/text_flow/tests.rs`、`src/layout/text_flow/tests/style_normalization.rs`、
  `tests/text_flow_style_normalization.rs`
- Upstream behavior contract: [`../GH58/product.md`](../GH58/product.md)、
  [`../GH58/tech.md`](../GH58/tech.md)、[`../GH58/tasks.md`](../GH58/tasks.md)
- Merged ordering contract: #126 / PR #136 merge
  `50f6a203c1861814d288d4bdeae0e28d877af34c`

## 当前实现门

live issue 当前带 `ready_to_implement`，但本 packet 在此次 spec PR 前不存在。现有
dry-run route artifact 只允许 `write_spec`，不授权 implementation。开始 `SP127-T1` 前，
coordinator 必须 fresh 证明：

1. 本三文件 spec-only PR 已 merged，并有绑定 exact spec head/scope 的 human approval。
2. issue 处于单一 canonical `ready_to_implement`，无 `parked`/冲突 readiness。
3. #126 merge `50f6a203c1861814d288d4bdeae0e28d877af34c` 是 implementation
   head ancestor。
4. duplicate search 未发现 GH-127 implementation PR、remote/local branch、worktree owner；
   创建恰好一个 implementation branch/PR。
5. base 包含 #128 PR #134、#129 PR #135、#130 PR #138 的 merge commits。
6. manifest 五路径和 GH-58 spec refs 仍存在；current API/error/diagnostic/cache shape 与本
   packet 一致。任一失败保持 blocked，不改 label、不创建 implementation commit。

## 实现任务

- [ ] `SP127-T1` 执行 dependency、duplicate、route、current-API 与 root-cause preflight。 Owner: `gh127-preflight-owner` | Done when: fresh evidence bundle 和 red root-cause reproduction 完整 | Verify: T1 preflight、baseline check、exact style/property tests 全部通过。
  一份 fresh、只读 evidence bundle 绑定
  implementation base/head，证明 spec approval/readiness、#126 exact merge ancestry、
  #128/#129/#130 ancestry、零 duplicate owner、五路径 manifest 与 PR #109 unresolved
  thread；另在隔离
  scratch checkout 记录 current nested range scans 和 2k/4k/8k red operation-count
  reproduction，不向 implementation branch 提交红测。下列 preflight commands
  全部 nonzero-fail-closed，root-cause evidence 明确显示 4k/8k density 违反 B-002 而现有
  semantic regressions仍 green。
  - Dependencies: human implementation gate。
  - File ownership: 无 target writable path；只读 repo/GitHub evidence 与 scratch artifact。
  - Covers: B-001, B-002, B-003, B-018, B-019, B-020, B-022。
  - Verify:
    `git merge-base --is-ancestor 50f6a203c1861814d288d4bdeae0e28d877af34c HEAD`；
    `git merge-base --is-ancestor "$GH128_MERGE_SHA" HEAD`；
    `git merge-base --is-ancestor "$GH129_MERGE_SHA" HEAD`；
    `git merge-base --is-ancestor "$GH130_MERGE_SHA" HEAD`；
    `rg -n 'styled_ranges|StyleBoundaryNormalized|tokenize_source' src/layout/text_flow.rs`；
    `cargo check --workspace --all-targets --all-features --locked`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes -- --exact`；
    `PROPTEST_CASES=4096 cargo test --test property_tests --locked text_flow_logical_source_round_trip -- --exact`。
  - Handoff: 记录 exact base、#126 merge SHA、duplicate evidence、root-cause
    counter/raw output、现有 semantic outputs；T2 接受 handoff 后 T1 不写任何 owned path。

- [ ] `SP127-T2` 实现 typed validation plan、monotonic style/boundary normalization 与 deterministic private counter。 Owner: `gh127-normalization-core-owner` | Done when: linear merge、compatibility、polling与 L1-L5 完整 | Verify: GH127-L1 至 GH127-L5 及 T2 regressions 各 1 passed/0 ignored。
  validation 保留 caller-first invalid 与 sorted
  overlap pair；private plan 保存 original range/endpoint ordinals；style/boundary cursor
  对 post-validation `G+R` 单调前进；adjacent/empty/unsorted diagnostics 顺序与重数完全
  保持；range preprocessing 有 bounded interruption poll；ASCII、high-density
  combining/ZWJ 与 one-EGC skew 三类 2k/4k/8k production counter 在 debug/release 满足
  absolute+slope，内部 fixtures 的 ordered projection 非零并匹配 exact event count，
  negative bound diagnostics 完整；所有 error/cancellation 不产生 partial result。
  private build count 的失败原子性由 L5 精确断言。T2 regression commands恰好各
  1 passed/0 ignored。
  - Dependencies: SP127-T1 完整 handoff；#126 exact merge ancestor 已证明。
  - File ownership: 独占 `src/layout/text_flow.rs`、
    `src/layout/text_flow/style_normalization.rs`、`src/layout/text_flow/tests.rs` 与
    `src/layout/text_flow/tests/style_normalization.rs`；自然移动现有 styled-normalization
    unit bodies 到新子模块，父文件只保留 module declaration/必要 stable selector wrapper，
    最终 `tests.rs <= 800` 行，禁止压缩/削弱测试。不得修改 `wrap.rs`、`truncate.rs`、
    engine、property/integration/CI 文件。T2 完成后冻结四个文件，只允许因 T3 暴露真实
    production defect 时显式 handback。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-010, B-011, B-012, B-015, B-016, B-017, B-018, B-021。
  - Verify:
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact`；
    `cargo test --release --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_operation_bound_failure_reports_complete_diagnostics -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::style_boundary_event_order_and_multiplicity_are_stable -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_range_extremes_preserve_typed_errors -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_normalization_polling_and_cache_count_are_atomic -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::text_flow_styled_runs -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes -- --exact`；
    `cargo test --workspace --lib --locked layout::text_flow::tests::text_flow_interruption -- --exact`。
  - Handoff: 交付 operation definition、2k/4k/8k raw counts/bounds、private plan invariants、
    exact test outputs 与 source freeze SHA；T3 只消费 public surface。

- [ ] `SP127-T3` 建立 public behavior integration oracle 并运行 dependency regressions。 Owner: `gh127-public-contract-owner` | Done when: GH127-L6 至 L13 与 GH-58/#126/#128/#129/#130 contracts 全 green | Verify: T3 public exact/regression commands 各 1 passed/0 ignored或完整 target green。
  `tests/text_flow_style_normalization.rs` 只用 public API 覆盖 combining、ZWJ、adjacent、
  internal empty、合法未排序、default style、reverse/non-char/out-of-bounds/
  `usize::MAX`、overlap、cache vector order/style/endpoint changes、immediate/during-build
  cancellation、previous Arc/cache identity/完整 flow 与 retry-cold parity；integration
  不读取 private `build_count`、不复制 merge 算法/计数器、不访问 clock，也不得推动 public
  accessor；critical ledger GH127-L6 至 GH127-L13、现有 property/engine/truncation及 #126
  tests 全 green。下列 public exact/regression commands全部满足首行 Verify。
  - Dependencies: SP127-T2 source freeze/handoff。
  - File ownership: 独占 `tests/text_flow_style_normalization.rs`；默认不得写 T2 两文件。
    若发现 production defect，停止 T3，显式把 ownership handback 给 T2 修正并重跑
    T2 全部 gates；禁止在 integration test 内绕过或弱化。
  - Covers: B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012,
    B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021。
  - Verify:
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_first_source_style_and_diagnostics -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_typed_failures -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_complete_flow_identity -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_exact_cache_identity -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_failure_precedence_is_stable -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_failures_and_interruption_are_atomic -- --exact`；
    `cargo test --test text_flow_style_normalization --locked public_styled_flow_retry_matches_cold_build -- --exact`；
    `PROPTEST_CASES=4096 cargo test --test property_tests --locked text_flow_logical_source_round_trip -- --exact`；
    `cargo test --test text_flow_truncate_regressions --locked`；
    `cargo test --workspace --lib --locked layout::engine::text_flow_bridge::tests::replace_and_reorder_preserve_only_live_flows -- --exact`；
    `cargo test --workspace --lib --locked layout::engine::context_sync::tests::identical_context_sync_keeps_text_leaf_and_root_clean_and_reuses_flow -- --exact`；
    `cargo test --workspace --lib --locked layout::engine::context_sync::tests::source_style_wrap_and_overflow_changes_dirty_only_the_affected_text_path -- --exact`；
    `cargo test --test text_flow_wrap_interruption --locked`。
  - Handoff: 交付 GH127-L6..L13 raw outputs、complete-flow equality/cache evidence、
    dependency regression outputs与 exact implementation head；冻结唯一 integration file。

- [ ] `SP127-T4` 完成 immutable exact-head closure audit。 Owner: `gh127-verification-review-owner` | Done when: B/ledger/manifest/coverage/full CI/SpecRail/review closure 全部 fresh | Verify: tech Verification Plan 与 fresh GitHub evidence 全部通过。
  product/tech/tasks B-set 均 exact `B-001..B-022`，task Covers union 无遗漏；
  manifest 五路径是唯一完整 allowed set，actual diff是其非空子集；父 unit file <=800 行；
  GH127-L1..L13 各先经 harness inventory 证明 selector 恰好一个，再实际执行且
  1 passed/0 ignored；debug/release counts、property、dependency regressions、full Rust、
  >=80% changed production line coverage、critical normalization line/branch 100%、
  exact base/head/merge-base + raw LCOV SHA-256 provenance、pinned revision/checker
  SHA-256 + byte-identical input SpecRail mirror、exact-head hosted CI、独立 review 与零
  unresolved non-outdated current threads 全部 fresh；PR #109 thread 只由 human 在证据
  满足后处理。
  - Dependencies: SP127-T1、SP127-T2、SP127-T3 完成并停止写所有 owned paths。
  - File ownership: 无 writable path；纯只读 verification/review，不 resolve thread、
    approve、merge、改 label 或修测试。任一失败 handback 给对应 owner并使全部旧
    head-bound evidence失效。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019,
    B-020, B-021, B-022。
  - Verify:
    `cargo fmt --all -- --check`；
    `cargo check --workspace --all-targets --all-features --locked`；
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
    `cargo test --workspace --all-targets --all-features --locked`；
    tech Verification Plan 的 fresh exact-head `cargo llvm-cov` + diff/LCOV fail-closed
    verifier，并保留 raw LCOV 与 checksum/provenance JSON；
    fixed revision `23caa70e76904eaa82323208d645d5781a365649` external mirror 中的
    checker SHA-256、byte-identical GH127/GH58 inputs、`check_workflow.py` 与
    `route_gate.py`（同时记录 target route gate 不存在）；
    fresh GraphQL reviewThreads 与 exact `headRefOid` check rollup。
  - Handoff: 向 human maintainer 提交 exact head、dependency SHAs、2k/4k/8k counts、
    ledger 13/13、B coverage 22/22、manifest/diff、raw LCOV + checksum/provenance、
    SpecRail checker/input hashes、CI/review JSON；不宣称 final approval/merge。

## Execution Graph and Ownership

```text
Human implementation gate
  -> SP127-T1 (read-only/scratch evidence)
  -> SP127-T2 (private normalization module + split unit tests)
  -> SP127-T3 (public integration test)
  -> SP127-T4 (read-only verification/review)
```

- writer tasks 不并行；每个时刻每个 target path 只有一个 owner。
- #126 merge `50f6a203c1861814d288d4bdeae0e28d877af34c` 固定 `wrap.rs` 与其
  integration test 行为；GH-127 不接管。
- T2→T3 如需 production handback，必须先停止 T3 writer、废弃当前 head evidence，再由
  T2 单独修改；修正后重新执行 T2、T3 全部 gates。
- 不预提交红测，不创建 future-owner test 依赖，不用脚本批量改写 semantic fixtures。

## Invariant Coverage Audit

| Task | Covers |
| --- | --- |
| SP127-T1 | B-001, B-002, B-003, B-018, B-019, B-020, B-022 |
| SP127-T2 | B-001..B-012, B-015, B-016, B-017, B-018, B-021 |
| SP127-T3 | B-004..B-021 |
| SP127-T4 | B-001..B-022 |

- Product invariant set：`B-001` 至 `B-022`，共 22。
- Tech Product-to-Test Mapping set：`B-001` 至 `B-022`，共 22。
- Tasks `Covers:` union：`B-001` 至 `B-022`，共 22。
- Critical ledger：`GH127-L1` 至 `GH127-L13`，共 13；T2 owns L1-L5，T3 owns
  L6-L13，T4只审计。

## 验证

- exact base、human spec approval、readiness 与 #126/#128/#129/#130 ancestry fresh。
- actual implementation diff 是 manifest 五路径非空子集；no-write paths diff为空；
  `src/layout/text_flow/tests.rs <= 800` 行。
- GH127-L1..L13 每项 test selector恰好发现并执行一个 nonignored test。
- 2k/4k/8k debug/release counts 同时满足 absolute bound、doubling slope 和完整 failure
      diagnostics；无 wall-clock gate。
- public complete-flow/cache/error/cancellation/retry fixtures与4096 property green。
- #126/#128/#129/#130 regressions green且断言未修改。
- fmt/check/clippy/all-target/all-feature tests、branch-aware coverage、fixed-revision external
      SpecRail mirror、CI、independent review、reviewThreads 全绑定同一 exact head。

## Handoff Notes

- 当前 PR 只交付 specs；不得实现、设置/修改 readiness label、resolve PR #109 thread、
  approve、merge 或关闭 issue。
- spec PR body 使用 `Refs #127`，不得用 `Fixes #127`；implementation完成前 issue保持 open。
- `StyleBoundaryNormalized` 的 ordered duplicates、caller 原始 range vector cache identity 和
  validation-vs-interruption precedence 都是 compatibility contract，不是可自由清理的
  implementation detail。
- operation counter 只测 post-validation normalization；review 还必须 source-scan，确认
  `G×R` 没有移到未计数 helper。
- #126 merge 已是 hard ancestor；任何后续 current API/path/ledger 变化都使旧
  implementation evidence失效，需要 retarget、重跑并必要时更新 specs。
