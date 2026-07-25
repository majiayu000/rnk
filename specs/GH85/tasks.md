# Task Plan：聊天布局 benchmark baseline 与回归门

## Linked Issue

GH-85: https://github.com/majiayu000/rnk/issues/85

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Dependency: [`../GH61/product.md`](../GH61/product.md)、
  [`../GH61/tech.md`](../GH61/tech.md)、[`../GH61/tasks.md`](../GH61/tasks.md)

## 实现前置门

#85 必须具有 canonical `ready_to_implement` 且本 packet 已人工接受；#61 implementation
必须已合入并记录 exact merged SHA。当前 #61 仍为 `parked`，本基线也没有其生产测量 seam，
因此以下 implementation tasks 尚不可启动。不得把 spec 中的拟议 API 当成已存在实现。

## 实现任务

- [ ] `SP85-T1` 建立 benchmark contract 的可重复 red fixtures。Owner:
      `benchmark-root-cause-lane` | Dependencies: accepted GH85 packet、canonical
      `ready_to_implement`、#61 exact merged implementation SHA、fresh duplicate evidence 与
      implement route gate | Done when: fixtures 在实现前证明现有 generic layout benches
      缺少固定 chat matrix/versioned artifact/trusted-baseline lifecycle，并列出所有
      positive/negative case 名称；不得修改 test assertion 基础设施 | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked -- --list`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`。
  - File ownership: 独占 `tests/layout_snapshot_benchmark_contract.rs`；不写 production、
    bench、Cargo、workflow、checker 或 baseline。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 提交 red checkpoint 与完整 fixture manifest 后停止写该文件；T2 串行接管。

- [ ] `SP85-T2` 实现固定 workload runner、versioned artifact、allocation/timing collector 与
      fail-closed checker。Owner: `benchmark-contract-lane` | Dependencies: SP85-T1 handoff；
      T1 writer 已停止；#61 merged counter seam 已按 exact SHA 重新定位 | Done when: matrix
      严格等于 tech 表；row 聚合与 counters 完整；`median_ns` 是唯一 timing 字段；checker
      CLI 只从 exact base tree 取 canonical baseline，正确返回 blocked/
      `needs_rebaseline`/regression/bootstrap 状态；implementation bootstrap 只写 candidate，
      所有负例均 fail closed | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked invalid_or_partial_rows_fail_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked artifact_binds_environment_and_exact_shas -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fingerprint_mismatch_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_promotion_never_self_authorize -- --exact`。
  - File ownership: 接管 `tests/layout_snapshot_benchmark_contract.rs`；独占
    `.github/scripts/check_gh61_benchmark.py`、`benches/chat_layout.rs`、
    `benches/support/chat_layout.rs`、`tests/fixtures/gh61_benchmark_schema.json`、
    `Cargo.toml`、`Cargo.lock`。只读 GH-61 production seam；不写 workflow 或 canonical
    baseline。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 交付 exact CLI、schema version、candidate path、fixed constants 与全部 fixture
    结果；T2 停止写所有文件后 T3/T4 才开始。

- [ ] `SP85-T3` 把 benchmark contract 接入现有 required CI。Owner:
      `benchmark-ci-lane` | Dependencies: SP85-T2 complete handoff；T2 writer 已停止 |
      Done when: `.github/workflows/ci.yml` 的独立 benchmark job 先运行 deterministic
      prerequisite tests，再运行 bootstrap 或 trusted compare；artifact 即使 non-green 也上传
      诊断，required summary 不把 blocked/`needs_rebaseline`/bootstrap 解释为 performance
      pass；job 使用 GitHub exact base/head 与 existing concurrency cancellation contract |
      Verify: `python3 .github/scripts/check_gh61_benchmark.py --list-scenarios`；
      `python3 .github/scripts/check_gh61_benchmark.py --validate-artifact tests/fixtures/gh61_benchmark_schema.json`；
      manual workflow inspection：benchmark job 是 `ci-gate` required dependency，且
      implementation diff 不含 `.github/benchmarks/gh61-baseline.json`。
  - File ownership: 独占 `.github/workflows/ci.yml`；其他文件只读。
  - Covers: B-001, B-002, B-003, B-004, B-007, B-008。
  - Handoff: 输出 current exact-head CI artifact/status mapping；不修改 branch protection、
    GitHub labels 或 baseline。

- [ ] `SP85-T4` 完成 implementation exact-head verification 与人工 handoff。Owner:
      `benchmark-verification-lane` | Dependencies: SP85-T2、SP85-T3 完成且所有 writers 停止 |
      Done when: 每个 exact contract test 证明 matched=1/passed=1/ignored=0；full Rust gates、
      candidate schema、implementation diff guard、current CI、independent review、
      reviewThreads 与 SpecRail `pr_gate` 绑定同一 head；只申请 implementation merge
      authorization，不申请 baseline promotion authorization | Verify: 重跑 SP85-T2/T3
      全部命令及本文件“验证”章节的 fresh full commands。
  - File ownership: 全仓只读；发现 production/test/workflow 缺陷时退回对应 owner 新 checkpoint，
    不跨 ownership 偷改。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: implementation 合入后记录 exact merged SHA；未合入不得启动 T5。

- [ ] `SP85-T5` 在独立 PR 中重新测量并 promotion canonical baseline。Owner:
      `baseline-promotion-lane` | Dependencies: SP85-T4 implementation 已由人工授权并合入；
      exact merged implementation SHA 已记录；独立 branch/PR 与新的 SpecRail/current CI/
      review/merge authorization | Done when: 在 exact merged implementation SHA 的隔离
      checkout fresh rerun bootstrap，canonical artifact content/metadata 来自该 rerun；
      promotion PR 只写 canonical baseline，未复制旧 candidate、未只改 SHA，且 promotion
      head 不自信任该 baseline | Verify:
      `python3 .github/scripts/check_gh61_benchmark.py --validate-artifact .github/benchmarks/gh61-baseline.json`；
      `git diff --name-only <promotion-base>...HEAD` 精确等于
      `.github/benchmarks/gh61-baseline.json`；记录 independent review、current exact-head CI、
      SpecRail gate 与 separate merge authorization。
  - File ownership: 独占 `.github/benchmarks/gh61-baseline.json`；其余全仓只读。
  - Covers: B-003, B-007, B-008, B-009。
  - Handoff: 只有 promotion 合入 default branch 后，未来 PR 才可把该 baseline 当 trusted
    base-tree evidence；不得在本 task 自动 merge。

- [ ] `SP85-T6` 在未来 PR 验证首次 trusted compare。Owner:
      `benchmark-post-promotion-verification-lane` | Dependencies: SP85-T5 promotion 已人工授权
      并合入，且该 commit 位于当前 PR base tree | Done when: fresh fixture 验证 base-tree
      ancestry/content hash、same-runner ABBA、timing 2-of-3、allocation 双阈值和
      fingerprint mismatch；证据绑定当前 exact base/head | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fingerprint_mismatch_needs_rebaseline -- --exact`。
  - File ownership: 全仓只读；只产出 current-run evidence。
  - Covers: B-005, B-006, B-007, B-009。
  - Handoff: compare 仍需普通 PR 的 current CI/review/merge authorization；本验证不授予 merge。

## 并行拆分

- Writable dependency graph：`T1 -> T2 -> {T3 || T4-readiness} -> T4 -> implementation merge
  -> T5 -> promotion merge -> T6`；T4 的执行必须等 T3 完成，只有只读 readiness 收集可提前。
- T1/T2 串行接管同一 contract test；T2 独占 bench/checker/Cargo/schema，T3 独占
  `.github/workflows/ci.yml`，没有共享 writable file。
- T4/T6 全仓只读；缺陷必须退回唯一 owner。T5 是后续独立 PR，唯一可写 canonical baseline。
- implementation PR、promotion PR 的 merge 都是分离的人工 gate；任何 lane 不得自行 push、
  approve、resolve review threads 或 merge。

## 验证

- Product invariant 集合与 task `Covers:` union 均精确为 B-001 至 B-009。
- planned implementation paths 限于 tech manifest；首次 implementation diff 必须排除
  `.github/benchmarks/gh61-baseline.json`，promotion diff 必须只包含该文件。
- 所有 filtered Rust tests 先 `--list --exact`，再执行并证明 matched=1、passed=1、ignored=0。
- artifact 的 scenario/strategy/minimum operation matrix、字段、closed enum、SHA/fingerprint、
  nonnegative counter 与 recovered aggregation 必须完整；partial/invalid evidence non-green。
- bootstrap 只允许 `bootstrap_valid`/`comparison_status=not_available`/
  `promotion_required=true`，不得输出 performance pass。
- fresh full commands：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
```

- exact-head GitHub CI、independent review、reviewThreads、SpecRail `pr_gate` 与 explicit human
  merge authorization 必须绑定同一 implementation 或 promotion head，不能跨 PR 复用。

## Handoff Notes

- 当前 branch 只交付 `specs/GH85/*`，不得实现、改 label、push、开 PR、merge、关闭 issue
  或解除 #61 的 `parked`。
- #85 当前没有 canonical readiness label；#61 仍带 `parked`，且 #85 依赖其 snapshot/work
  counter implementation。两项都由 human/orchestrator 决策，不能由本 packet 静默绕过。
- checker 名称与 canonical path 沿用拆分来源：
  `.github/scripts/check_gh61_benchmark.py`、
  `.github/benchmarks/gh61-baseline.json`；如要改为 GH85 命名，必须先更新 issue/spec。
- timing 使用 same-runner ABBA、3 batches、20%+50µs、two-of-three；allocation 使用
  10%+8 allocations / 4096 bytes。fingerprint 不兼容是 `needs_rebaseline`，不是 green。
- implementation bootstrap 与 canonical promotion 必须是两个 PR 生命周期；promotion 仍需
  独立 review/current CI/SpecRail/merge authorization。
