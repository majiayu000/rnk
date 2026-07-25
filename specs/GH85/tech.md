# Tech Spec：聊天布局 benchmark artifact、可信 baseline 与 promotion gate

## Linked Issue

GH-85: https://github.com/majiayu000/rnk/issues/85

## Product Spec

见 [`product.md`](product.md)。

GH-85 消费 GH-61 的 immutable snapshot producer、parity 与 per-frame deterministic work
counters，不改变其 layout/recovery 语义。当前 #61 仍带 `parked`，且本写作基线尚无其生产
实现；GH-85 implementation 必须等 #61 实现合入后，在 exact merged SHA 上重新定位测量 seam。

## Codebase Context

以下锚点已在 `26499553b33a133071139d6baa6fce8b190ae0b3` 核实：

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Existing layout benches | `benches/layout.rs:71`, `benches/layout.rs:90`, `benches/layout.rs:137`, `benches/layout.rs:160` | Divan 只覆盖 engine creation、通用 full tree/grid/text/getters | 没有 chat mutation、strategy matrix、allocation/work artifact |
| Bench registration/deps | `Cargo.toml:76`, `Cargo.toml:83`, `Cargo.toml:85` | dev dependencies 已含 `serde_json`/`divan`，只注册现有四个 bench | 新 machine-readable bench 需要显式注册；依赖与 lockfile 必须同步 |
| Existing CI | `.github/workflows/ci.yml:50`, `.github/workflows/ci.yml:98`, `.github/workflows/ci.yml:225` | CI 编译 benches，并由 `ci-gate` 汇总 required jobs；没有 benchmark contract job | GH-85 required job 应接入现有 `ci.yml`，不新建平行 required workflow |
| CI concurrency | `.github/workflows/ci.yml:8` | 同一 ref 的旧 PR run 会被取消 | benchmark artifact 必须绑定当前 exact base/head，取消后的部分结果不可复用 |
| Coverage | `.github/workflows/ci.yml:183` | coverage 目前 `continue-on-error`，不负责 performance decision | benchmark gate 不得借 coverage 的 advisory 状态宣称 green |
| GH-61 measurement dependency | `specs/GH61/product.md:33`, `specs/GH61/tech.md:389`, `specs/GH61/tech.md:391`, `specs/GH61/tasks.md:54` | GH-61 规划 per-frame deterministic work counters，benchmark 已拆到 #85 | GH-85 只读消费 merged seam；不得先发明未合入 public API |
| Split provenance | `specs/GH61/product.md:46`, `specs/GH61/tasks.md:20` | 当前 packet 明确把 workload/baseline/promotion/regression gate 排除到 #85 | GH-85 范围对应拆分前 B-024 至 B-028 |

## 计划变更清单

```specrail-planned-changes
{
  "issue": 85,
  "complete": true,
  "paths": [
    ".github/benchmarks/gh61-baseline.json",
    ".github/scripts/check_gh61_benchmark.py",
    ".github/workflows/ci.yml",
    "Cargo.lock",
    "Cargo.toml",
    "benches/chat_layout.rs",
    "benches/support/chat_layout.rs",
    "specs/GH61/product.md",
    "specs/GH61/tech.md",
    "specs/GH85/product.md",
    "specs/GH85/tasks.md",
    "specs/GH85/tech.md",
    "tests/fixtures/gh61_benchmark_schema.json",
    "tests/fixtures/gh85_gh61_dependency.json",
    "tests/layout_snapshot_benchmark_contract.rs"
  ],
  "spec_refs": [
    "specs/GH85/product.md#B-001",
    "specs/GH85/product.md#B-002",
    "specs/GH85/product.md#B-003",
    "specs/GH85/product.md#B-004",
    "specs/GH85/product.md#B-005",
    "specs/GH85/product.md#B-006",
    "specs/GH85/product.md#B-007",
    "specs/GH85/product.md#B-008",
    "specs/GH85/product.md#B-009"
  ]
}
```

`.github/benchmarks/gh61-baseline.json` 只属于 B-009 的后续独立 promotion PR；首次
implementation PR 的 diff/manifest 必须排除该路径。`target/gh61-baseline-candidate.json`
是 CI artifact，不入库，因此不在 planned repository paths 中。

## 设计方案

### 1. 依赖与 route gate

implementation 开始前必须同时满足：

- #85 具有 canonical `ready_to_implement` 且 product/tech/tasks 获得人工接受；
- #61 的 implementation 已合入，`GH61_MERGED_SHA` 已由 GitHub merged evidence 解析；
- duplicate search 与 `implement` route gate 在 current checkout fresh 通过。

GH-61 合入后先执行：

```sh
git merge-base --is-ancestor "$GH61_MERGED_SHA" HEAD
test "$(git show -s --format=%H "$GH61_MERGED_SHA")" = "$GH61_MERGED_SHA"
rg -n 'SnapshotBuildReport|SnapshotWorkCounters|visited_nodes|mutated_nodes|text_flow_recomputes|snapshot_nodes|rebuild_count' src tests
python3 -m json.tool tests/fixtures/gh85_gh61_dependency.json >/dev/null
cargo test --test layout_snapshot_benchmark_contract --locked \
  dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact
```

`tests/fixtures/gh85_gh61_dependency.json` 是 closed dependency manifest，exact keys 为
`schema_version`、`issue`、`gh61_merged_sha`、`resolved_at_head`、`snapshot_report`、
`work_counter`、`strategy_entrypoints`、`counter_fields`。两个 symbol object 只允许
`path`、`symbol`；三个 strategy entry 只允许 `strategy`、`path`、`symbol`，且 strategy
集合严格等于 `{full, incremental, recovered}`；`counter_fields` 严格等于
`{visited_nodes, mutated_nodes, text_flow_recomputes, snapshot_nodes, rebuild_count}`。
每个 path/symbol 必须在 `GH61_MERGED_SHA` tree 与 current HEAD 中唯一解析，merged SHA
必须是 HEAD 祖先，wiring test 必须实际调用三个 strategy 并读取所有 counters。缺字段、
unknown key、placeholder、零匹配、多匹配、非祖先或任一 strategy 未接线都 blocked。

任何前置条件缺失均保持 blocked，不以本 spec 中的拟议类型或未来 GH-61 test 名替代真实
merged API。

### 2. Workload runner 与固定矩阵

`benches/support/chat_layout.rs` 构建确定 corpus/scenario，`benches/chat_layout.rs` 只负责
执行、分配采集与结构化输出。setup/tree construction 在计时区间外；每个 strategy 从等价
committed 起点运行同一 target。

| scenario | fixed input / minimum operations | required strategies |
| --- | --- | --- |
| `unchanged_frame` | 1000-message transcript；64 个相同 frame operations | full、incremental；recovered 禁止 |
| `streaming_delta` | 1000-message transcript；32 个 grapheme-safe ASCII/CJK/emoji/combining deltas | full、incremental、recovered |
| `append_message` | 1000-message committed 起点；64 次 single-message append | full、incremental、recovered |
| `middle_insert` | 1000-message committed 起点；32 次在 index 500 附近 insert | full、incremental、recovered |
| `variable_height_transcript` | 1000 messages；64 次 update 循环 1..12 logical rows，并含 CJK/emoji | full、incremental、recovered |
| `resize_invalidation` | 1000 messages；30 个完整 `120x40 -> 80x24 -> 120x40` cycles | full、incremental、recovered |

固定 benchmark 常量为：

```text
seed = 0x9e3779b97f4a7c15
default_target_size = 1000 messages
default_viewport_sequence = [(120, 40)]
resize_viewport_sequence = [(120, 40), (80, 24), (120, 40)]
message_corpus_revision = "gh85-chat-v1"
warmup_iterations = 3
leg_sample_count = 5
sample_count = 10 per scenario/strategy/batch row
batch_count = 3
paired_order = ABBA per batch
```

这些常量由 support module 唯一声明，schema fixture/checker/test 引用同一合同，禁止维护第二份
名称或阈值表。`sample_count < 10`、batch 数不等于 3 或 paired order 不等于 ABBA 时 artifact
无效。

### 3. Closed artifact schemas 与 hash

candidate、canonical 与 current-run compare artifacts 共用一个 closed envelope。top-level
exact keys 为：

```text
schema_version, checker_revision, artifact_role, source_sha, pr_base_oid, merge_base_sha,
head_sha, content_sha256, config_sha256, cargo_lock_sha256,
message_corpus_revision, message_corpus_sha256, rustc, target, profile,
runner, workload, paired_order, comparison_id, execution_trace, rows
```

- `artifact_role` 闭集为 `candidate`、`canonical`、`compare_base_current_run`、
  `compare_head_current_run`。
- `runner` exact keys 为 `os`、`arch`、`cpu`、`fingerprint`；`workload` exact keys 为
  `seed`、`target_size`、`viewport_sequence`、`warmup_iterations`、`leg_sample_count`、
  `sample_count`、`batch_count`、`scenario_matrix`、`paired_order_contract`；后者固定为
  `{"bootstrap":"not_applicable","compare":"ABBA"}`。
- `paired_order` 只允许 `not_applicable` 或 `ABBA`；candidate/canonical 必须是
  `not_applicable`、`comparison_id=null`、`execution_trace=[]`，current-run compare 必须是
  `ABBA` 且具有同一 nonempty `comparison_id`。
- `execution_trace` entry exact keys 为 `pair_id`、`batch_index`、`sequence_index`、`role`、
  `source_sha`、`binary_sha256`；每个 pair 的 sequence 必须精确为
  `[(0,base),(1,head),(2,head),(3,base)]`。
- row exact keys 为 `scenario`、`strategy`、`operation_count`、`sample_count`、
  `batch_index`、`pair_id`、`median_ns`、`allocation_count`、`allocated_bytes`、
  `visited_nodes`、`mutated_nodes`、`text_flow_recomputes`、`snapshot_nodes`、
  `rebuild_count`。每个 row 以 scenario/strategy/batch 聚合；current-run 两个 base legs 或
  两个 head legs 各采 5 samples，聚合后 `sample_count=10`。

所有 object 层级都设置 `additionalProperties=false`；未知 key、duplicate JSON key、未知
enum、缺 required key、非法 null、负 counter 或越界 integer 一律 blocked。`median_ns`
必须大于 0；allocation 与 work counters 可以为 0。GH-61 的 per-frame `rebuild_count`
只能为 0/1；聚合后 recovered row 必须等于 operation count，其他 strategy 为 0。

hash 统一使用 SHA-256 小写 hex：

- `message_corpus_sha256`：exact corpus UTF-8 bytes；
- `config_sha256`：对仅含 `schema_version`、`checker_revision`、完整 `workload`、
  `message_corpus_revision`、`message_corpus_sha256` 的 RFC 8785 canonical JSON bytes 求 hash；
- `content_sha256`：对完整 artifact 移除且只移除 top-level `content_sha256` 后的 RFC 8785
  canonical JSON bytes 求 hash；不得排除 `source_sha`、role、runner、trace 或 rows；
- `binary_sha256`：checker 实际执行的 exact bench executable bytes。

candidate 的 `source_sha=head_sha=exact implementation head`；canonical 的
`source_sha=head_sha=exact merged implementation SHA`；current-run base/head 的
`source_sha` 分别等于 `pr_base_oid`/current head。candidate 不能改 role 后成为 canonical，
canonical 必须由独立 rerun 生成。bench-only allocation instrumentation 不进入 production
report，也不新增 public `Any`、arbitrary closure 或运行时 allocator replacement API。

role-specific refs 同样闭合：candidate 记录 implementation PR exact `pr_base_oid`、
`merge_base_sha`、`head_sha`；canonical 固定
`pr_base_oid=merge_base_sha=head_sha=source_sha`；两类 current-run artifact 共享 exact
PR `pr_base_oid`/`merge_base_sha`/`head_sha`。除 candidate/canonical 的
`comparison_id=null` 外不允许 null。candidate/canonical row 的 `pair_id` 是对
artifact role、source SHA、scenario、strategy、batch index 的 length-prefixed UTF-8
bytes 求 SHA-256；current-run row 使用 compare protocol 的 `pair_id`。

negative fixtures 至少包含：top-level/nested unknown key、duplicate JSON key、missing row
key、unknown role/order、role/source mismatch、content/config/corpus/binary hash mismatch、
zero timing、negative allocation、candidate-as-canonical、canonical-as-current-run、
missing/duplicate/cross-run/wrong-order pair、source-not-ancestor、source-equals-head、
caller-supplied baseline 与 incompatible fingerprint。每个 fixture 必须 schema-targeted，
不能用另一处更早的 parse failure 冒充目标 predicate 覆盖。

### 4. Checker CLI 与 deterministic pre-gates

`.github/scripts/check_gh61_benchmark.py` 使用参数数组调用外部命令，并提供闭合 CLI：

- `--list-scenarios`：输出 machine-readable exact scenario/strategy/minimum matrix；
- `--validate-dependency-manifest PATH --repo PATH --gh61-merged-sha SHA`：执行 dependency
  ancestry/anchor/strategy/counter fail-closed gate；
- `--validate-artifact PATH --expected-role ROLE`：验证 closed schema、unknown/duplicate keys、
  role、hash、SHA/fingerprint、paired order、nonzero operation/sample 与所有 counter；
- `--mode bootstrap --candidate-out target/gh61-baseline-candidate.json`：只写
  non-authoritative candidate；
- `--mode promote --source-sha SHA --canonical-out .github/benchmarks/gh61-baseline.json`：
  只允许在独立 promotion checkout 中 fresh rerun，并直接生成 canonical role；
- `--mode compare --repo PATH --base-worktree PATH --pr-base-oid SHA --head-sha SHA
  --run-id ID --target-root PATH --artifact-dir PATH`：从 exact base tree 解析 canonical
  baseline，并在一个 checker process 内 build/run current base/head；不接受调用方任意
  `--base` artifact 或跨 run raw measurement。

CI required job 先运行 dependency wiring、GH-61 parity/work-counter 与 allocation contract
exact tests，再执行 artifact validation。任何前置失败都停止 performance decision，但上传
诊断 artifact；禁止捕获异常后返回 success。

### 5. Trusted baseline 与 compare

compare 通过 `git show <pr_base_oid>:.github/benchmarks/gh61-baseline.json` 读取 repo-owned
baseline，验证：

- `artifact_role=canonical`，`source_sha` 是 `pr_base_oid` 的祖先，且不等于 current
  `head_sha`；
- closed schema、`content_sha256`、`config_sha256`、corpus/Cargo hashes 全部重算一致；
- `pr_base_oid`、merge base 与 head SHA 匹配当前 GitHub PR exact refs。

trust/staleness predicate 是闭合的：

- baseline missing、不是从 exact base tree 读取、role 非 canonical、content hash 无效、
  `source_sha` 不在 `pr_base_oid` ancestry、等于 current head 或任一 exact ref 不匹配：
  `blocked`；
- baseline 自身按其 schema 有效且 ancestry 可信，但 schema/checker/config/corpus/Cargo/
  toolchain/runner fingerprint 与 current compare 不兼容：`needs_rebaseline`；
- 只有所有 predicate 通过才是 `trusted`。checker 输出第一个及完整 `rejection_items[]`，
  不把 stale/untrusted 转换为零值 row。

canonical baseline 只授权 workload/config/fingerprint 可比较性与历史 promotion 来源；实际
threshold denominator 必须来自本次 run 的 `compare_base_current_run`，不能直接用 canonical
row，也不能复用 candidate 或旧 CI 的 raw base artifact。

CI 在同一 runner 上执行以下协议：

```sh
test "$(git rev-parse HEAD)" = "$HEAD_SHA"
git worktree add --detach "$RUNNER_TEMP/gh85-base" "$PR_BASE_OID"
test "$(git -C "$RUNNER_TEMP/gh85-base" rev-parse HEAD)" = "$PR_BASE_OID"
python3 .github/scripts/check_gh61_benchmark.py \
  --mode compare \
  --repo "$GITHUB_WORKSPACE" \
  --base-worktree "$RUNNER_TEMP/gh85-base" \
  --pr-base-oid "$PR_BASE_OID" \
  --head-sha "$HEAD_SHA" \
  --run-id "$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" \
  --target-root "$RUNNER_TEMP/gh85-targets" \
  --artifact-dir "$RUNNER_TEMP/gh85-artifacts"
```

checker 以参数数组分别执行
`["cargo","build","--manifest-path",CHECKOUT/Cargo.toml,"--bench","chat_layout",
"--locked","--release","--target-dir",TARGET_DIR,"--message-format=json"]`：base 的
`CHECKOUT/TARGET_DIR` 是 exact base worktree/`$RUNNER_TEMP/gh85-targets/base`，head 是
`$GITHUB_WORKSPACE`/`$RUNNER_TEMP/gh85-targets/head`。checker 从 Cargo JSON 解析
executable，验证其 checkout source SHA，记录 executable bytes 的 `binary_sha256`；
build/setup 不进入 timing。

每个 leg 用参数数组运行
`[EXECUTABLE,"--scenario",SCENARIO,"--strategy",STRATEGY,"--batch-index",N,
"--leg-index",L,"--seed","0x9e3779b97f4a7c15","--warmup-iterations","3",
"--sample-count","5","--artifact-out",LEG_PATH]`。checker 验证 leg artifact 后才写
`$RUNNER_TEMP/gh85-artifacts/base-current-run.json` 与
`head-current-run.json`；leg files 不是可复用 comparison input。

`comparison_id` 是对 `run-id`、`pr_base_oid`、`head_sha`、runner fingerprint、
`config_sha256` 的 length-prefixed UTF-8 bytes 求 SHA-256；`pair_id` 是对
`comparison_id`、scenario、strategy、batch index 的同样编码求 SHA-256。每个
scenario/strategy/batch 依次运行 A(base)、B(head)、B(head)、A(base)，每 leg 5 samples；
两个 A legs 聚合为一个 base row，两个 B legs 聚合为一个 head row。两份 current-run
artifacts 必须有相同 comparison/trace、互补 role/source、相同 pair set，且每个 trace 精确
为 sequence 0..3 的 ABBA。缺 pair、重复 pair、跨 run ID、错序、binary/source mismatch、
runner/config 不同均 blocked。

每个 scenario/strategy 分别计算 3 个 batch：

- timing：`head/base > 1.20` 且 `head-base > 50_000ns`，3 batches 至少 2 个满足才失败；
- allocation count：相对增加 `> 10%` 且绝对增加 `> 8` 才失败；
- allocated bytes：相对增加 `> 10%` 且绝对增加 `> 4096` 才失败。

timing base/head `median_ns` 任一为 0 都是 invalid denominator，整个 comparison blocked。
allocation metric 的 base/head 都为 0 时该 metric 无回归；base 为 0、head 大于 0 时相对条件
视为满足，但仍只有 head 严格大于对应绝对 floor（8 或 4096）才失败。该规则逐 metric、
逐 batch 应用，禁止除零、NaN/Infinity 或 warning + fallback。

### 6. Bootstrap 与独立 promotion

首次 implementation PR 不存在 trusted baseline，只允许 bootstrap：

```text
decision=bootstrap_valid
comparison_status=not_available
promotion_required=true
```

candidate 必须绑定 exact implementation head 并验证 B-001 至 B-003；implementation diff
不得包含 canonical baseline。job 被取消、失败或只产生部分 artifact 时，candidate 不得进入
promotion。candidate 必须具有 `artifact_role=candidate`、`paired_order=not_applicable`、
empty trace 与有效 hash；unknown key 或 role/hash/source 不一致时 blocked。

implementation 合入后，baseline-promotion lane 从 default branch 创建独立 PR，在 exact
merged implementation SHA 的隔离 checkout 运行 `--mode promote`，直接生成
`artifact_role=canonical`、`source_sha=head_sha=exact merged implementation SHA`、
`paired_order=not_applicable`、empty trace 与 fresh hashes。promotion 只能写
`.github/benchmarks/gh61-baseline.json`，不得复制 candidate、转换 candidate role 或只改
SHA。PR 必须通过独立 review、current exact-head CI、SpecRail gate 与单独 merge
authorization；checker 只生成/验证文件，没有批准或 merge 权限。promotion head 自身仍不
受信，baseline 只有合入并出现在未来 PR base tree 后才可用于 compare。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | fixed workload matrix | `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact` |
| B-002 | artifact aggregation/closed schema | `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked closed_schema_rejects_unknown_duplicate_and_partial_rows -- --exact` |
| B-003 | roles/source/hash/environment | `cargo test --test layout_snapshot_benchmark_contract --locked artifact_hashes_cover_roles_sources_config_corpus_trace_and_rows -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked candidate_canonical_and_current_run_roles_are_not_interchangeable -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact` |
| B-004 | dependency/prerequisite gates | `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact` |
| B-005 | exact-checkout ABBA/timing | `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact` |
| B-006 | allocation comparator | `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact` |
| B-007 | base-tree trust/fingerprint gate | `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked fingerprint_mismatch_needs_rebaseline -- --exact` |
| B-008 | implementation bootstrap | `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact` |
| B-009 | exclusive promotion lifecycle | `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rerun_emits_fresh_canonical_role_and_hashes -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_promotion_never_self_authorize -- --exact`; manual diff check: promotion PR changes only `.github/benchmarks/gh61-baseline.json` and records independent review/current CI/SpecRail/merge authorization |

## 数据流

```text
fixed corpus + target/viewport sequence
  -> merged GH-61 full/incremental/recovered snapshot producers
  -> deterministic SnapshotWorkCounters
  -> bench-only timing + allocation collector
  -> exact-head versioned candidate artifact
  -> schema + prerequisite gates
  -> bootstrap(non-authoritative), or
     base-tree trusted baseline + same-runner ABBA compare
  -> blocked / needs_rebaseline / regression / comparison_passed

post-implementation exact merged SHA
  -> isolated rerun
  -> independent baseline-promotion PR
  -> human review + current CI + SpecRail gate + merge authorization
  -> canonical baseline in future PR base tree
```

artifact 不进入 runtime 持久化或 public API。candidate 只作为 current-run CI artifact；canonical
baseline 是唯一 checked-in performance evidence。

## 备选方案

- **复用 `benches/layout.rs` 的通用 microbenchmark**：拒绝；没有 chat mutation、producer
  strategy 与 versioned artifact。
- **解析 Divan pretty output**：拒绝；未版本化文本不是稳定 machine contract。
- **单次 wall-clock 超阈值即失败**：拒绝；hosted runner 噪声会制造不稳定门。
- **从 feature head 读取 baseline**：拒绝；允许当前改动自行降低标准。
- **bootstrap 直接写 canonical baseline**：拒绝；把测量者、writer 与授权者合为同一 PR。

## 风险

- **Security**：corpus/诊断可能含 terminal controls；artifact 使用 JSON 结构化字段，不把
  payload 拼入 shell。所有外部命令使用参数数组，禁止 command injection。
- **Compatibility**：GH-61 尚未实现，真实 work-counter seam 可能改变；实现必须在 merged
  SHA 重新定位并更新 spec，禁止用 guessed adapter 静默兼容。
- **Performance/CI noise**：runner 调度、thermal、toolchain 漂移影响 timing；same-runner
  ABBA、3 batches、双阈值和 fingerprint 把不可比较状态显式化。
- **Evidence**：bootstrap 没有旧 scenario baseline；其状态必须与 performance green 分离。
- **Maintenance**：schema/support/checker/test 若复制常量会漂移；固定矩阵由一个 source
  生成并以 closed negative fixtures 验证。

## 测试计划

- [ ] Dependency：merged GH-61 ancestry、closed anchor manifest、full/incremental/recovered
      wiring 与全部 counter fields。
- [ ] Contract：固定 matrix、minimum operations、row aggregation、closed/unknown-key policy、
      artifact roles、source/config/corpus/content/binary hashes、唯一 timing 字段与负例。
- [ ] Comparator：exact-base/current-head isolated builds、same-runner ABBA trace/pair identity、
      timing 2-of-3、timing zero denominator、allocation 相对+绝对/zero denominator、
      self/stale/untrusted/missing baseline 与 fingerprint mismatch。
- [ ] Lifecycle：bootstrap candidate exact-head binding、implementation diff 禁止 canonical
      baseline、partial candidate、独立 promotion 重新测量与 self-trust rejection。
- [ ] Full gates：
      `cargo fmt --all -- --check`；
      `cargo check --workspace --all-targets --all-features --locked`；
      `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
      `cargo test --workspace --all-targets --all-features --locked`；
      `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`。
- [ ] GitHub：current exact-head required CI、independent review、resolved review threads、
      SpecRail `pr_gate` 与 explicit merge authorization。

## 回滚方案

若 checker、harness 或 CI integration 产生错误阻断，整体回滚 GH-85 implementation PR，
恢复原有 CI；不得保留一个把 invalid evidence 判 green 的宽松 fallback。candidate 为
untracked artifact，可直接丢弃。

若仅 timing 因 runner noise 不可比较，可把 timing decision 降为 `needs_rebaseline`，但保留
required parity/work/allocation/schema/exact-head gates；不得删除 artifact 字段或伪造 base。
若 canonical baseline 有误，通过新的独立 promotion PR 重新测量和替换，不直接编辑 SHA 或
在 feature PR 内放宽阈值。回滚后 #85 保持打开，保存 exact failed head/CI/artifact/review
证据；#61 与其他 layout correctness 合同不回滚。
