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

- [ ] `SP85-T7` 在 GH-61 合入后解析真实 measurement seam 并建立 fail-closed dependency
      manifest。Owner: `gh61-dependency-resolution-lane` | Dependencies: accepted GH85
      packet、canonical `ready_to_implement`、#61 exact merged implementation SHA |
      Done when: `tests/fixtures/gh85_gh61_dependency.json` 记录 real
      `SnapshotBuildReport`/`SnapshotWorkCounters` 与 full/incremental/recovered entrypoint
      path/symbol；所有 anchor 在 merged SHA 与 current HEAD 唯一解析；closed counter set
      完整；从 merged GH61 的实际 tests/tasks/verification 解析非空 closed
      `prerequisite_commands` argv array，不预写未来 test 名；command id/argv 唯一并绑定
      exact test，缺失/empty/duplicate/unknown/placeholder/零匹配/多匹配均 blocked | Verify:
      `git merge-base --is-ancestor "$GH61_MERGED_SHA" HEAD`；
      `test "$(git show -s --format=%H "$GH61_MERGED_SHA")" = "$GH61_MERGED_SHA"`；
      `rg -n 'SnapshotBuildReport|SnapshotWorkCounters|visited_nodes|mutated_nodes|text_flow_recomputes|snapshot_nodes|rebuild_count' src tests`；
      `python3 -m json.tool tests/fixtures/gh85_gh61_dependency.json >/dev/null`。
  - File ownership: 独占 `tests/fixtures/gh85_gh61_dependency.json`；为 wiring fixture 向 T1
    交付 assertions，但不写 T1 contract file、production、bench、checker、Cargo 或 workflow。
  - Covers: B-004。
  - Handoff: 交付 GH61 merged SHA、resolved anchors、counter/strategy closed sets 与 fresh
    command output后停止写；T1 必须据此建立实际调用三种 strategy 并读取五个 counters 的
    wiring fixture，T2 使其通过；二者不得静默改写 manifest。

- [ ] `SP85-T1` 建立 benchmark contract 的可重复 red fixtures。Owner:
      `benchmark-root-cause-lane` | Dependencies: SP85-T7 complete handoff、fresh duplicate
      evidence 与 implement route gate | Done when: fixtures 在实现前证明现有 generic layout benches
      缺少固定 chat matrix/versioned artifact/trusted-baseline lifecycle，并列出所有
      positive/negative case 名称；dependency wiring、closed/unknown-key schema、roles/hashes、
      closed build provenance、prerequisite command/result、event exact refs、exact-checkout
      ABBA/pair mismatch、zero denominators 与 promotion source-worktree/rerun 都有命名 red
      fixture；不得修改 test assertion 基础设施 | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked -- --list`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`。
  - File ownership: 独占 `tests/layout_snapshot_benchmark_contract.rs`；不写 production、
    bench、Cargo、workflow、checker 或 baseline。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 提交 red checkpoint 与完整 fixture manifest 后停止写该文件；T2 串行接管。

- [ ] `SP85-T2` 实现固定 workload runner、versioned artifact、allocation/timing collector 与
      fail-closed checker。Owner: `benchmark-contract-lane` | Dependencies: SP85-T1 handoff；
      T1 writer 已停止；#61 merged counter seam 已按 exact SHA 重新定位 | Done when: matrix
      严格等于 tech 表；row 聚合与 counters 完整；closed schema 拒绝 unknown/duplicate
      keys；每个 role 都有 closed build provenance；role/source/config/corpus/content/binary
      hash、prerequisite results 与 ABBA trace/pair identity 完整；
      `median_ns` 是唯一 timing 字段；checker CLI 只从 exact base tree 取 canonical baseline，正确返回 blocked/
      `needs_rebaseline`/regression/bootstrap 状态；implementation bootstrap 只写 candidate，
      zero denominators 与所有 trust/promotion 负例均 fail closed | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked closed_schema_rejects_unknown_duplicate_and_partial_rows -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked artifact_hashes_cover_roles_sources_config_corpus_trace_and_rows -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked all_roles_require_closed_build_provenance -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked candidate_canonical_and_current_run_roles_are_not_interchangeable -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_invalid_prerequisite_command_arrays -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_commands_execute_and_record_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_binds_event_head_and_base_without_merge_ref -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fingerprint_mismatch_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_requires_exact_source_worktree_and_revalidates_provenance -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rerun_emits_fresh_canonical_role_and_hashes -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_promotion_never_self_authorize -- --exact`。
  - File ownership: 接管 `tests/layout_snapshot_benchmark_contract.rs`；独占
    `.github/scripts/check_gh61_benchmark.py`、`benches/chat_layout.rs`、
    `benches/support/chat_layout.rs`、`tests/fixtures/gh61_benchmark_schema.json`、
    `Cargo.toml`、`Cargo.lock`。只读 T7 dependency manifest 与 GH-61 production seam；
    不写 workflow 或 canonical baseline。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 交付 exact CLI、schema version、candidate path、fixed constants 与全部 fixture
    结果；T2 停止写所有文件后 T3/T4 才开始。

- [ ] `SP85-T3` 把 benchmark contract 接入现有 required CI。Owner:
      `benchmark-ci-lane` | Dependencies: SP85-T2 complete handoff；T2 writer 已停止 |
      Done when: `.github/workflows/ci.yml` 的独立 benchmark job 先运行 dependency wiring 与
      dependency manifest 的每个 prerequisite argv，以 `shell=false` 逐条执行并记录
      exit/matched/passed/ignored，全部成功后才运行 bootstrap 或 trusted compare；workflow
      env 只绑定 `${{ github.event.pull_request.head.sha }}`/
      `${{ github.event.pull_request.base.sha }}`，checkout `ref` 是 exact head 且
      `fetch-depth: 0`，不使用 `GITHUB_SHA`/merge ref；compare 在同一
      runner 创建 exact PR-base detached worktree，base/head 使用隔离 target dirs build，
      checker 单进程按每 pair/batch ABBA 运行并产出互补 current-run artifacts；artifact 即使
      non-green 也上传诊断，required summary 不把 blocked/`needs_rebaseline`/bootstrap
      解释为 performance pass；job 使用 GitHub exact base/head 与 existing concurrency
      cancellation contract |
      Verify: `python3 .github/scripts/check_gh61_benchmark.py --list-scenarios`；
      `python3 .github/scripts/check_gh61_benchmark.py --validate-dependency-manifest tests/fixtures/gh85_gh61_dependency.json --repo . --gh61-merged-sha "$GH61_MERGED_SHA"`；
      `python3 .github/scripts/check_gh61_benchmark.py --validate-artifact tests/fixtures/gh61_benchmark_schema.json --expected-role candidate`；
      `test -n "$HEAD_SHA"`；
      `test -n "$PR_BASE_OID"`；
      `git cat-file -e "${HEAD_SHA}^{commit}"`；
      `git cat-file -e "${PR_BASE_OID}^{commit}"`；
      `test "$(git rev-parse HEAD)" = "$HEAD_SHA"`；
      `git worktree add --detach "$RUNNER_TEMP/gh85-base" "$PR_BASE_OID"`；
      `test "$(git -C "$RUNNER_TEMP/gh85-base" rev-parse HEAD)" = "$PR_BASE_OID"`；
      `python3 .github/scripts/check_gh61_benchmark.py --mode compare --repo "$GITHUB_WORKSPACE" --base-worktree "$RUNNER_TEMP/gh85-base" --pr-base-oid "$PR_BASE_OID" --head-sha "$HEAD_SHA" --run-id "$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" --target-root "$RUNNER_TEMP/gh85-targets" --artifact-dir "$RUNNER_TEMP/gh85-artifacts"`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_binds_event_head_and_base_without_merge_ref -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_commands_execute_and_record_before_benchmark -- --exact`；
      manual workflow inspection：checkout exact `ref`/`fetch-depth: 0`、benchmark job 是
      `ci-gate` required dependency，且 implementation diff 不含
      `.github/benchmarks/gh61-baseline.json`。
  - File ownership: 独占 `.github/workflows/ci.yml`；其他文件只读。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008。
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
      review/merge authorization | Done when: 在 exact merged implementation SHA 的 detached
      worktree fresh 运行 `--mode promote`，直接生成 canonical role 与 fresh hashes；
      `PROMOTION_BASE` 只绑定 `${{ github.event.pull_request.base.sha }}`，不使用 branch/
      merge ref/`GITHUB_SHA`；
      promotion PR 只写 canonical baseline，未复制/转换旧 candidate、未只改 SHA，且 promotion
      head 不自信任该 baseline | Verify:
      `git worktree add --detach "$RUNNER_TEMP/gh85-promotion-source" "$IMPLEMENTATION_MERGED_SHA"`；
      `test "$(git -C "$RUNNER_TEMP/gh85-promotion-source" rev-parse HEAD)" = "$IMPLEMENTATION_MERGED_SHA"`；
      `git cat-file -e "${PROMOTION_BASE}^{commit}"`；
      `git -C "$RUNNER_TEMP/gh85-promotion-source" merge-base --is-ancestor "$IMPLEMENTATION_MERGED_SHA" "$PROMOTION_BASE"`；
      `python3 "$RUNNER_TEMP/gh85-promotion-source/.github/scripts/check_gh61_benchmark.py" --mode promote --source-worktree "$RUNNER_TEMP/gh85-promotion-source" --source-sha "$IMPLEMENTATION_MERGED_SHA" --promotion-base-oid "$PROMOTION_BASE" --dependency-manifest "$RUNNER_TEMP/gh85-promotion-source/tests/fixtures/gh85_gh61_dependency.json" --canonical-out "$GITHUB_WORKSPACE/.github/benchmarks/gh61-baseline.json"`；
      `python3 .github/scripts/check_gh61_benchmark.py --validate-artifact .github/benchmarks/gh61-baseline.json --expected-role canonical`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_requires_exact_source_worktree_and_revalidates_provenance -- --exact`；
      `git diff --name-only "$PROMOTION_BASE"...HEAD` 精确等于
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
      zero denominators/fingerprint mismatch；证据绑定当前 exact base/head | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fingerprint_mismatch_needs_rebaseline -- --exact`。
  - File ownership: 全仓只读；只产出 current-run evidence。
  - Covers: B-005, B-006, B-007, B-009。
  - Handoff: compare 仍需普通 PR 的 current CI/review/merge authorization；本验证不授予 merge。

## 并行拆分

- Writable dependency graph：`T7 -> T1 -> T2 -> {T3 || T4-readiness} -> T4 -> implementation merge
  -> T5 -> promotion merge -> T6`；T4 的执行必须等 T3 完成，只有只读 readiness 收集可提前。
- T7 独占 dependency manifest；T1/T2 串行接管同一 contract test；T2 独占
  bench/checker/Cargo/schema，T3 独占
  `.github/workflows/ci.yml`，没有共享 writable file。
- T4/T6 全仓只读；缺陷必须退回唯一 owner。T5 是后续独立 PR，唯一可写 canonical baseline。
- implementation PR、promotion PR 的 merge 都是分离的人工 gate；任何 lane 不得自行 push、
  approve、resolve review threads 或 merge。

## 验证

- Product invariant 集合与 task `Covers:` union 均精确为 B-001 至 B-009。
- planned implementation paths 限于 tech manifest；首次 implementation diff 必须排除
  `.github/benchmarks/gh61-baseline.json`，promotion diff 必须只包含该文件。
- 所有 filtered Rust tests 先 `--list --exact`，再执行并证明 matched=1、passed=1、ignored=0。
- dependency manifest 必须绑定 GH61 merged ancestry、真实 unique anchors、三种 strategy 与
  五个 counter fields，以及 merged GH61 解析出的非空、唯一 closed prerequisite argv；
  T3 未执行/记录任一 command 或 wiring test 未实际消费任一 entry 时 blocked。
- artifact 的 scenario/strategy/minimum operation matrix、closed/unknown-key schema、
  role/source/closed build/config/corpus/content/binary hashes、prerequisite results、
  ABBA trace/pair identity、event exact head/base SHA/fingerprint、
  zero-denominator semantics、nonnegative counter 与 recovered aggregation 必须完整；
  partial/invalid evidence non-green。
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

- 当前 branch 只交付 `specs/GH85/*` 与 GH61 split-residue cleanup
  `specs/GH61/{product,tech}.md`；不得实现、改 label、push、开 PR、merge、关闭 issue 或
  解除 #61 的 `parked`。
- #85 当前没有 canonical readiness label；#61 仍带 `parked`，且 #85 依赖其 snapshot/work
  counter implementation。两项都由 human/orchestrator 决策，不能由本 packet 静默绕过。
- checker 名称与 canonical path 沿用拆分来源：
  `.github/scripts/check_gh61_benchmark.py`、
  `.github/benchmarks/gh61-baseline.json`；如要改为 GH85 命名，必须先更新 issue/spec。
- timing 使用 same-runner ABBA、3 batches、20%+50µs、two-of-three；allocation 使用
  10%+8 allocations / 4096 bytes。fingerprint 不兼容是 `needs_rebaseline`，不是 green。
- implementation bootstrap 与 canonical promotion 必须是两个 PR 生命周期；promotion 仍需
  独立 review/current CI/SpecRail/merge authorization。
