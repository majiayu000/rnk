# Task Plan：聊天布局 benchmark baseline 与回归门

## Linked Issue

GH-85: https://github.com/majiayu000/rnk/issues/85

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Dependency: [`../GH61/product.md`](../GH61/product.md)、
  [`../GH61/tech.md`](../GH61/tech.md)、[`../GH61/tasks.md`](../GH61/tasks.md)

## 实现前置门

maintainer 必须对当前 exact packet/head 明确确认可以实施；readiness label 仅描述队列状态。
#61 implementation 必须已合入并记录 exact merged SHA；在生产测量 seam 合入前不得启动以下
implementation tasks，也不得把 spec 中的拟议 API 当成已存在实现。

## 实现任务

- [ ] `SP85-T7A` 在GH-61合入后只读发现真实measurement seam与prerequisite候选。Owner:
      `gh61-dependency-discovery-lane` | Dependencies: maintainer对当前exact packet/head的明确
      implementation授权、#61 exact merged implementation SHA | Done when: read-only evidence记录
      real `SnapshotBuildReport`/`SnapshotWorkCounters`、full/incremental/recovered path/symbol、
      closed counter set，以及parity/work-counter exact argv；search merged GH61是否已有明确证明
      allocation operation归属、计数与reset语义的exact test，只输出`found(real argv)`或
      `missing(require GH85 fallback)`，不得创建或定稿dependency manifest，也不得引用尚不存在的
      fallback test | Verify:
      `git merge-base --is-ancestor "$GH61_MERGED_SHA" HEAD`；
      `test "$(git show -s --format=%H "$GH61_MERGED_SHA")" = "$GH61_MERGED_SHA"`；
      `rg -n 'SnapshotBuildReport|SnapshotWorkCounters|visited_nodes|mutated_nodes|text_flow_recomputes|snapshot_nodes|rebuild_count' src tests`。
  - File ownership: 全仓只读；只交付discovery evidence，不写manifest、T1 contract file、
    production、bench、checker、Cargo或workflow。
  - Covers: B-004。
  - Handoff: 向T1交付GH61 merged SHA、resolved anchors、counter/strategy closed sets、两条
    real prerequisite argv与allocation discovery result后停止；T1据此决定是否创建fallback。

- [ ] `SP85-T1` 建立 benchmark contract 的可重复 red fixtures。Owner:
      `benchmark-root-cause-lane` | Dependencies: SP85-T7A discovery handoff、fresh duplicate
      evidence与maintainer implementation授权 | Done when: fixtures 在实现前证明现有 generic layout benches
      缺少固定 chat matrix/versioned artifact/trusted-baseline lifecycle，并列出所有
      positive/negative case 名称；dependency wiring、closed/unknown-key schema、roles/hashes、
      closed build provenance、prerequisite category/spec_ref/command/result、allocation
      fallback-before-benchmark、PR/push `ci-gate` result matrix、event exact refs、
      exact-checkout ABBA/pair mismatch、zero denominators 与 promotion source-worktree/rerun
      都有命名 red fixture；不得修改 test assertion 基础设施 | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked -- --list`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_requires_complete_prerequisite_category_set -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_missing_duplicate_unknown_categories_and_spec_refs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_correctness_fallback_runs_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`。
  - File ownership: 独占 `tests/layout_snapshot_benchmark_contract.rs`；不写 production、
    bench、Cargo、workflow、checker 或 baseline。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 提交red checkpoint与完整fixture清单后停止写该文件；若T7A报告allocation test
    missing，本checkpoint必须已创建命名fallback test；随后T7B才能定稿manifest。

- [ ] `SP85-T7B` 在T1 checkpoint后定稿fail-closed dependency manifest。Owner:
      `gh61-dependency-finalization-lane` | Dependencies: SP85-T7A evidence、SP85-T1 stopped；若
      allocation test缺失，T1 fallback exact test已实际存在 | Done when:
      `tests/fixtures/gh85_gh61_dependency.json`记录real anchors/strategies/counters，三个
      prerequisite commands严格按parity/work-counter/allocation-correctness各一次；第三项只能取
      T7A发现的real test或T1已存在的fallback，所有path/argv在merged SHA与current HEAD唯一解析，
      禁止placeholder/未来test名 | Verify:
      `python3 -m json.tool tests/fixtures/gh85_gh61_dependency.json >/dev/null`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`。
  - File ownership: 独占`tests/fixtures/gh85_gh61_dependency.json`；T1 contract file与其他路径只读。
  - Covers: B-004。
  - Handoff: manifest commit后停止；T2只读消费manifest，不得静默改写。

- [ ] `SP85-T2` 实现固定 workload runner、versioned artifact、allocation/timing collector 与
      fail-closed checker。Owner: `benchmark-contract-lane` | Dependencies: SP85-T1与SP85-T7B
      handoff；两者writer均停止；#61 merged counter seam已按exact SHA重新定位 | Done when: matrix
      严格等于 tech 表；row 聚合与 counters 完整；closed schema 拒绝 unknown/duplicate
      keys；每个 role 都有 closed build provenance；role/source/config/corpus/content/binary
      hash、三个按 closed category/spec_ref 顺序记录的 prerequisite results 与 ABBA
      trace/pair identity 完整；allocation fallback 证明 counter 的 operation 归属、计数与
      reset 语义且先于任何 benchmark；
      `median_ns`由exact 10 observations的checked even median产生，deterministic counters逐sample
      相同；checker只从exact base tree取canonical，并唯一选择initial bootstrap、contract-update
      bootstrap、canonical-only promotion或normal trusted compare route；
      只有`comparison_passed`表示无回归；bootstrap显式绑定repo/base/head/run/target/artifact，
      promotion validation只读authority evidence与committed blob；zero denominators、containment、
      ancestry/ref/runner/trust/promotion负例均fail closed | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked closed_schema_rejects_unknown_duplicate_and_partial_rows -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked artifact_hashes_cover_roles_sources_config_corpus_trace_and_rows -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked all_roles_require_closed_build_provenance -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked candidate_canonical_and_current_run_roles_are_not_interchangeable -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked ten_sample_even_median_and_deterministic_counters_are_exact -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked per_operation_counters_sum_checked_and_abba_samples_keep_leg_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_invalid_prerequisite_command_arrays -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_requires_complete_prerequisite_category_set -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_missing_duplicate_unknown_categories_and_spec_refs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_correctness_fallback_runs_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_commands_execute_and_record_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_paths_and_argv_are_contained -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_binds_event_head_and_base_without_merge_ref -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_compare_require_base_ancestor_and_exact_merge_base -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_regression_fails_on_any_paired_batch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked canonical_refs_are_historical_and_current_refs_are_invocation_scoped -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked runner_compatibility_excludes_volatile_cpu_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked abba_requires_identical_current_runner_observation -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked pinned_toolchain_target_profile_and_runner_class_are_closed -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_uses_base_owned_checker_and_rejects_untrusted_head_policy -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_rejects_mixed_spec_symlink_mode_and_ambiguous_diffs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked route_selection_is_mutually_exclusive_and_only_comparison_passed_is_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authorized_contract_update_is_non_green_and_requires_rebaseline_promotion -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_requires_explicit_repo_refs_and_exact_merge_base -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_validation_is_read_only_and_authority_bound -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rejects_committed_blob_not_matching_authority -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_workflow_permissions_and_attestation_identity_are_exact -- --exact`；
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
      Done when: `.github/workflows/layout-benchmark-authority.yml`已作为独立trust-root contract
      在protected default ref可用，PR phase-zero只运行base/default-ref checker且不checkout head；
      authority job permissions精确为`contents: read`、`id-token: write`、`attestations: write`，
      使用`actions/attest@v4`，其他权限none；`.github/workflows/ci.yml` 的独立 `layout_benchmark` job 先运行 dependency
      wiring，再按 parity/work-counter/allocation-correctness 顺序以 `shell=false` 执行
      dependency manifest 的三个 prerequisite argv，并记录 category/spec_ref 与
      exit/matched/passed/ignored，全部成功后才运行唯一选中的initial bootstrap、contract-update
      bootstrap、promotion validation或trusted compare；workflow
      env 只绑定 `${{ github.event.pull_request.head.sha }}`/
      `${{ github.event.pull_request.base.sha }}`，checkout `ref` 是 exact head 且
      `fetch-depth: 0`，不使用 `GITHUB_SHA`/merge ref；compare 在同一
      runner先验证base是head祖先且exact merge-base等于base，再创建exact PR-base detached
      worktree；prerequisite cwd固定checkout root且argv/paths通过closed containment；base/head使用隔离target dirs build，
      checker 单进程按每 pair/batch ABBA 运行并产出互补 current-run artifacts；artifact 即使
      non-green 也上传诊断；job按diff/base canonical状态唯一选择initial bootstrap、
      contract-update bootstrap、canonical-only promotion或normal trusted compare，
      promotion只读验证committed blob与authority，required summary不把blocked/
      `needs_rebaseline`/`bootstrap_valid`/`contract_update_valid`/`promotion_valid`解释为performance
      pass；job使用GitHub exact base/head 与 existing concurrency
      cancellation contract；非 PR push 上该 job 精确 skipped，`ci-gate` 使用 `always()`、
      保留八个既有 required job 的 success checks，并且只允许
      `(pull_request, success)` 或 `(event_name != pull_request, skipped)` 的 benchmark
      result pairing；PR 的 failure/cancelled/skipped 均 non-green |
      Verify: `python3 .github/scripts/check_gh61_benchmark.py --list-scenarios`；
      `python3 .github/scripts/check_gh61_benchmark.py --validate-dependency-manifest tests/fixtures/gh85_gh61_dependency.json --repo . --gh61-merged-sha "$GH61_MERGED_SHA"`；
      `python3 .github/scripts/check_gh61_benchmark.py --validate-artifact tests/fixtures/gh61_benchmark_schema.json --expected-role candidate`；
      `test -n "$HEAD_SHA"`；
      `test -n "$PR_BASE_OID"`；
      `git cat-file -e "${HEAD_SHA}^{commit}"`；
      `git cat-file -e "${PR_BASE_OID}^{commit}"`；
      `test "$(git rev-parse HEAD)" = "$HEAD_SHA"`；
      `git merge-base --is-ancestor "$PR_BASE_OID" "$HEAD_SHA"`；
      `test "$(git merge-base "$PR_BASE_OID" "$HEAD_SHA")" = "$PR_BASE_OID"`；
      `git worktree add --detach "$RUNNER_TEMP/gh85-base" "$PR_BASE_OID"`；
      `test "$(git -C "$RUNNER_TEMP/gh85-base" rev-parse HEAD)" = "$PR_BASE_OID"`；
      `python3 .github/scripts/check_gh61_benchmark.py --mode bootstrap --repo "$GITHUB_WORKSPACE" --pr-base-oid "$PR_BASE_OID" --head-sha "$HEAD_SHA" --run-id "$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" --target-root "$RUNNER_TEMP/gh85-targets" --artifact-dir "$RUNNER_TEMP/gh85-artifacts" --candidate-out "$RUNNER_TEMP/gh85-artifacts/candidate.json"`；
      `python3 .github/scripts/check_gh61_benchmark.py --mode compare --repo "$GITHUB_WORKSPACE" --base-worktree "$RUNNER_TEMP/gh85-base" --pr-base-oid "$PR_BASE_OID" --head-sha "$HEAD_SHA" --run-id "$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" --target-root "$RUNNER_TEMP/gh85-targets" --artifact-dir "$RUNNER_TEMP/gh85-artifacts"`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_binds_event_head_and_base_without_merge_ref -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_commands_execute_and_record_before_benchmark -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked workflow_runs_prerequisites_before_benchmark_and_ci_gate -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked ci_gate_accepts_benchmark_skip_only_for_non_pr_push -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked ci_gate_rejects_pr_benchmark_failed_cancelled_or_skipped -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked ci_gate_preserves_all_existing_required_jobs -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked route_selection_is_mutually_exclusive_and_only_comparison_passed_is_green -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_validation_is_read_only_and_authority_bound -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked authority_workflow_permissions_and_attestation_identity_are_exact -- --exact`；
      manual workflow inspection：checkout exact `ref`/`fetch-depth: 0`、benchmark job 是
      `ci-gate` required dependency，push skip/PR success result expression 精确，且
      implementation diff 不含
      `.github/benchmarks/gh61-baseline.json`。
  - File ownership: 独占 `.github/workflows/ci.yml`与
    `.github/workflows/layout-benchmark-authority.yml`；authority workflow必须在implementation
    measurement PR前通过独立maintainer-authorized trust-root contract merge进入default ref，
    PR head版本不构成授权；其他文件只读。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: 输出 current exact-head CI artifact/status mapping；不修改 branch protection、
    GitHub labels 或 baseline。

- [ ] `SP85-T4` 完成 implementation exact-head verification 与人工 handoff。Owner:
      `benchmark-verification-lane` | Dependencies: SP85-T2、SP85-T3 完成且所有 writers 停止 |
      Done when: 每个 exact contract test 证明 matched=1/passed=1/ignored=0；full Rust gates、
      candidate schema、implementation diff guard、current CI、independent review、
      resolved review threads 与 maintainer 对当前 exact head 的 merge authorization 绑定同一
      head；只申请 implementation merge authorization，不申请 baseline promotion authorization |
      Verify: 重跑 SP85-T2/T3
      全部命令及本文件“验证”章节的 fresh full commands。
  - File ownership: 全仓只读；发现 production/test/workflow 缺陷时退回对应 owner 新 checkpoint，
    不跨 ownership 偷改。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009。
  - Handoff: implementation合入后记录exact merged SHA；未合入不得启动T5A。implementation
    authorization只覆盖该PR，不覆盖authority run或promotion PR。

- [ ] `SP85-T5A` 在trusted default ref生成并attest authority。Owner:
      `benchmark-authority-run-lane` | Dependencies: SP85-T4 implementation或authorized
      contract-update PR已由maintainer明确授权并合入；exact merged source SHA、route与authorization
      evidence已记录；default-ref上的
      `.github/workflows/layout-benchmark-authority.yml`是reviewed trusted version | Done when:
      maintainer从protected default ref触发`workflow_dispatch`；workflow在exact merged source的
      detached checkout fresh测量，只向`RUNNER_TEMP`/immutable artifact输出canonical subject，
      permissions精确为contents read/id-token write/attestations write，使用`actions/attest@v4`；
      attestation绑定repo/workflow/ref/run/source/subject digest，不读取PR candidate/checker、不写repo |
      Verify:
      `python3 .github/scripts/check_gh61_benchmark.py --mode generate-authority --repo "$AUTHORITY_REPO" --repository-id "$GITHUB_REPOSITORY_ID" --workflow-ref "$GITHUB_WORKFLOW_REF" --default-ref-sha "$DEFAULT_REF_SHA" --source-sha "$AUTHORITY_SOURCE_SHA" --run-id "$AUTHORITY_RUN_ID" --target-root "$RUNNER_TEMP/gh85-authority-target" --artifact-dir "$RUNNER_TEMP/gh85-authority-artifacts" --authority-out "$RUNNER_TEMP/gh85-authority-artifacts/authority.json"`；
      inspect trusted workflow permissions与`uses: actions/attest@v4`；记录canonical SHA-256、run id、
      workflow/default-ref/source SHA与platform attestation。
  - File ownership: repo全只读；只允许写repo外authority artifact。不得push、开PR或写canonical。
  - Covers: B-003, B-007, B-008, B-009。
  - Handoff: 向T5B交付immutable authority subject/bundle identifiers；T5B不得重新生成或转换。

- [ ] `SP85-T5B` 在独立PR promotion authority-owned canonical bytes。Owner:
      `baseline-promotion-lane` | Dependencies: SP85-T5A authority完成；promotion PR exact head已知并
      另行取得绑定该head的maintainer明确authorization | Done when: promotion PR只提交authority
      canonical bytes；base-owned phase zero判定`canonical_only_promotion`；required CI使用
      `gh attestation verify`与trusted checker只读验证subject/repo/workflow/ref/source/event，未调用
      generator、未创建/覆盖canonical；成功decision精确为`promotion_valid`且不声称无回归 |
      Verify:
      `git cat-file -e "${PROMOTION_BASE}^{commit}"`；
      `git cat-file -e "${PROMOTION_HEAD}^{commit}"`；
      `git merge-base --is-ancestor "$PROMOTION_BASE" "$PROMOTION_HEAD"`；
      `test "$(git merge-base "$PROMOTION_BASE" "$PROMOTION_HEAD")" = "$PROMOTION_BASE"`；
      `gh attestation verify "$CANONICAL_FILE" -R "$EXPECTED_REPOSITORY" --signer-workflow "$EXPECTED_REPOSITORY/.github/workflows/layout-benchmark-authority.yml" --signer-digest "$AUTHORITY_DEFAULT_REF_SHA" --source-ref "refs/heads/$DEFAULT_BRANCH" --source-digest "$AUTHORITY_DEFAULT_REF_SHA" --deny-self-hosted-runners --format json`；
      `python3 .github/scripts/check_gh61_benchmark.py --mode validate-promotion --repo "$GITHUB_WORKSPACE" --pr-base-oid "$PROMOTION_BASE" --head-sha "$PROMOTION_HEAD" --run-id "$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" --authority-bundle "$RUNNER_TEMP/authority.json" --committed-canonical .github/benchmarks/gh61-baseline.json`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_validation_is_read_only_and_authority_bound -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rejects_committed_blob_not_matching_authority -- --exact`；
      `git diff --name-only "$PROMOTION_BASE"..."$PROMOTION_HEAD"`精确等于canonical path；validation
      前后`git status --porcelain`为空且blob digest不变；记录current exact-head CI、independent
      review、resolved threads与maintainer对同一promotion head的separate merge authorization。
  - File ownership: 独占 `.github/benchmarks/gh61-baseline.json`；其余全仓只读。
  - Covers: B-003, B-007, B-008, B-009。
  - Handoff: 只有promotion合入default branch后，未来PR才可把baseline当trusted base-tree
    evidence；本task不自动merge，且promotion authorization不得复用于未来contract update。

- [ ] `SP85-T6` 在未来 PR 验证首次 trusted compare。Owner:
      `benchmark-post-promotion-verification-lane` | Dependencies: SP85-T5B promotion 已人工授权
      并合入，且该 commit 位于当前 PR base tree | Done when: fresh fixture 验证 base-tree
      ancestry/content hash、stable compatibility/volatile observation、same-runner ABBA、
      timing 2-of-3、allocation any-batch双阈值和zero denominators；证据绑定当前exact
      base/head/run | Verify:
      `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked runner_compatibility_excludes_volatile_cpu_identity -- --exact`；
      `cargo test --test layout_snapshot_benchmark_contract --locked abba_requires_identical_current_runner_observation -- --exact`。
  - File ownership: 全仓只读；只产出 current-run evidence。
  - Covers: B-005, B-006, B-007, B-009。
  - Handoff: compare 仍需普通 PR 的 current CI/review/merge authorization；本验证不授予 merge。

## 并行拆分

- Writable dependency graph：`T7A(read-only) -> T1 -> T7B -> T2 -> T3 trust-root merge
  -> implementation measurement/verification T4 -> implementation merge -> T5A(authority)
  -> T5B(promotion PR) -> promotion merge -> T6`；T4必须等T3 trusted workflow/checker已在base。
- 后续authorized contract-update PR走`contract_update_bootstrap -> contract_update_valid`，取得绑定
  exact head的独立maintainer authorization并合入后，同样进入`T5A -> T5B -> promotion merge`；
  contract-update decision本身始终non-green。
- T7A全只读；T1独占contract test；T7B独占dependency manifest；T2串行接管contract test并
  独占bench/checker/Cargo/schema。T2必须先交付checker/schema/config/toolchain contract checkpoint，
  T3将其与两个workflow作为独立maintainer-authorized trust-root contract合入；该merge前不得运行
  PR-head measurement。后续implementation checkpoint不得再改contract paths。
- T4/T5A/T6全仓只读；缺陷必须退回唯一owner。T5B是后续独立PR，唯一可写canonical baseline。
- implementation PR、promotion PR 的 merge 都是分离的人工 gate；任何 lane 不得自行 push、
  approve、resolve review threads 或 merge。

## 验证

- Product invariant 集合与 task `Covers:` union 均精确为 B-001 至 B-009。
- planned implementation paths 限于 tech manifest；首次 implementation diff 必须排除
  `.github/benchmarks/gh61-baseline.json`，promotion diff 必须只包含该文件。
- 所有 filtered Rust tests 先 `--list --exact`，再执行并证明 matched=1、passed=1、ignored=0。
- dependency manifest 必须绑定 GH61 merged ancestry、真实 unique anchors、三种 strategy 与
  五个 counter fields；prerequisite commands 必须按闭合 category/spec_ref 精确覆盖
  parity、work-counter、allocation-correctness；allocation 必须先 search merged GH61 的
  correctness test，缺失时使用已规划 GH85 contract fallback。T3 未按序执行/记录任一
  command、cwd不是exact checkout root、path absolute/traversal/symlink escape、argv不在closed
  Cargo exact-test allowlist、所选allocation correctness command未先于benchmark或wiring test
  未实际消费任一entry时blocked。
- workflow contract 必须证明 `layout_benchmark` 在 PR 仅 success 可通过，在非 PR push 仅
  skipped 可通过；PR failure/cancelled/skipped、push 上意外 success/failure/cancelled 或任一
  既有 required job 非 success 均不得让 `ci-gate` green。
- artifact 的 scenario/strategy/minimum operation matrix、closed/unknown-key schema、
  role/source/closed build/config/corpus/content/binary hashes、prerequisite results、
  historical/current refs separation、stable compatibility/volatile observation separation、
  ABBA trace/pair identity、event exact head/base ancestry与merge-base、10-sample checked even
  median、sample reset、deterministic counter equality、zero-denominator与recovered aggregation
  必须完整；timing按2-of-3，allocation任一paired batch双阈值即失败；
  partial/invalid evidence non-green。
- route必须在initial implementation bootstrap、contract-update bootstrap、canonical-only
  promotion与normal trusted compare中精确四选一；分别只允许`bootstrap_valid`、
  `contract_update_valid`、`promotion_valid`、`comparison_passed|regression|needs_rebaseline`。只有`comparison_passed`表示
  无回归，missing/ambiguous route blocked。
- authority generation只在default-ref-owned workflow运行且不写repo；promotion validation只读
  committed canonical与immutable authority bundle，validation前后blob/status必须不变。
- fresh full commands：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
```

- exact-head repository CI、independent review、resolved review threads 与 maintainer explicit
  merge authorization 必须绑定同一 implementation 或 promotion head，不能跨 PR 复用。

## Handoff Notes

- 本packet只描述未来implementation/authority/promotion lanes；其中“不自行push/开PR/merge/
  resolve threads”约束仅适用于这些未来lanes，不限制当前spec PR按maintainer授权更新原分支。
  实时label事实必须从GitHub重新读取，且不构成实施或merge授权。
- checker 名称与 canonical path 沿用拆分来源：
  `.github/scripts/check_gh61_benchmark.py`、
  `.github/benchmarks/gh61-baseline.json`；如要改为 GH85 命名，必须先更新 issue/spec。
- timing使用same-runner ABBA、3 batches、20%+50µs、two-of-three；allocation使用
  10%+8 allocations / 4096 bytes且任一paired batch双阈值即失败。stable compatibility class
  不兼容是`needs_rebaseline`；volatile CPU identity只诊断，但同次ABBA observation必须相同。
- implementation bootstrap 与 canonical promotion 必须是两个 PR 生命周期；promotion 仍需
  independent review、current exact-head repository CI、resolved threads与maintainer merge authorization。
