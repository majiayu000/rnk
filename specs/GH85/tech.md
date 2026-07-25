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
| GH-61 measurement dependency | `specs/GH61/product.md:34`, `specs/GH61/tech.md:448`, `specs/GH61/tasks.md:49` | GH-61 规划 per-frame deterministic work counters，benchmark 已拆到 #85 | GH-85 只读消费 merged seam；不得先发明未合入 public API |
| Split provenance | `specs/GH61/product.md:47`, `specs/GH61/tasks.md:20` | 当前 packet 明确把 workload/baseline/promotion/regression gate 排除到 #85 | GH-85 范围对应拆分前 B-024 至 B-028 |

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
    "specs/GH85/product.md",
    "specs/GH85/tasks.md",
    "specs/GH85/tech.md",
    "tests/fixtures/gh61_benchmark_schema.json",
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
- #61 的 implementation 已合入，记录 exact merged SHA，并重新核对 snapshot producer、
  strategy report 与 `SnapshotWorkCounters` 的真实路径/类型；
- duplicate search 与 `implement` route gate 在 current checkout fresh 通过。

任何条件缺失均保持 blocked，不以本 spec 中的拟议类型替代真实 GH-61 API。

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
sample_count = 10 per scenario/strategy/batch row
batch_count = 3
paired_order = ABBA per batch
```

这些常量由 support module 唯一声明，schema fixture/checker/test 引用同一合同，禁止维护第二份
名称或阈值表。`sample_count < 10`、batch 数不等于 3 或 paired order 不等于 ABBA 时 artifact
无效。

### 3. Versioned artifact

每个 row 以 scenario/strategy/batch 聚合，schema 至少包含：

```text
schema_version, mode, scenario, strategy, operation_count, seed, target_size,
viewport_sequence, message_corpus_revision, rustc, target, cargo_lock_sha256, profile,
runner_os, runner_arch, runner_cpu, runner_fingerprint, pr_base_oid, merge_base_sha, head_sha,
warmup_iterations, sample_count, batch_index, median_ns, allocation_count, allocated_bytes,
visited_nodes, mutated_nodes, text_flow_recomputes, snapshot_nodes, rebuild_count
```

`median_ns` 是唯一 timing 字段。所有 enum 闭合；required numeric fields 缺失、负值或越界时
返回结构化 non-green decision，不 warning + fallback。GH-61 的 per-frame
`rebuild_count` 只能为 0/1；聚合后 recovered row 必须等于 operation count，其他 strategy
为 0。bench-only allocation instrumentation 不进入 production report，也不新增 public
`Any`、arbitrary closure 或运行时 allocator replacement API。

### 4. Checker CLI 与 deterministic pre-gates

`.github/scripts/check_gh61_benchmark.py` 使用参数数组调用外部命令，并提供闭合 CLI：

- `--list-scenarios`：输出 machine-readable exact scenario/strategy/minimum matrix；
- `--validate-artifact PATH`：验证 schema、closed enum、SHA/fingerprint、nonzero
  operation/sample 与所有 counter；
- `--mode bootstrap --candidate-out target/gh61-baseline-candidate.json`：只写
  non-authoritative candidate；
- `--mode compare --repo . --pr-base-oid SHA --head-artifact PATH`：从 exact base tree 解析
  canonical baseline，不接受任意调用方 `--base` 文件。

CI required job 先运行 GH-61 parity、work-counter 与 allocation contract exact tests，再执行
artifact validation。任何前置失败都停止 performance decision，但上传诊断 artifact；禁止
捕获异常后返回 success。

### 5. Trusted baseline 与 compare

compare 通过 `git show <pr_base_oid>:.github/benchmarks/gh61-baseline.json` 读取 repo-owned
baseline，验证：

- `source_sha` 是 `pr_base_oid` 的祖先，且不等于 current `head_sha`；
- baseline content hash 与版本化 metadata 一致；
- schema/corpus/scenario/toolchain/runner fingerprint 与 head artifact 可比较；
- `pr_base_oid`、merge base 与 head SHA 匹配当前 GitHub PR exact refs。

missing/invalid/stale/self/untrusted baseline 为 blocked；合法但 fingerprint 不兼容返回
`needs_rebaseline`。二者均为 non-green，不能转为零回归数据。

同一 runner 对 base/head 使用 ABBA 交错次序运行 3 个 paired batches。每个 scenario/strategy
分别计算：

- timing：`head/base > 1.20` 且 `head-base > 50_000ns`，3 batches 至少 2 个满足才失败；
- allocation count：相对增加 `> 10%` 且绝对增加 `> 8` 才失败；
- allocated bytes：相对增加 `> 10%` 且绝对增加 `> 4096` 才失败。

### 6. Bootstrap 与独立 promotion

首次 implementation PR 不存在 trusted baseline，只允许 bootstrap：

```text
decision=bootstrap_valid
comparison_status=not_available
promotion_required=true
```

candidate 必须绑定 exact implementation head 并验证 B-001 至 B-003；implementation diff
不得包含 canonical baseline。job 被取消、失败或只产生部分 artifact 时，candidate 不得进入
promotion。

implementation 合入后，baseline-promotion lane 从 default branch 创建独立 PR，在 exact
merged implementation SHA 的隔离 checkout 重新运行 bootstrap。promotion 只能写
`.github/benchmarks/gh61-baseline.json`，不得仅复制 candidate 或改 SHA。PR 必须通过独立
review、current exact-head CI、SpecRail gate 与单独 merge authorization；checker 无写入、
批准或 merge 权限。promotion head 自身仍不受信，baseline 只有合入并出现在未来 PR base
tree 后才可用于 compare。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | fixed workload matrix | `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact` |
| B-002 | artifact aggregation/schema | `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked invalid_or_partial_rows_fail_closed -- --exact` |
| B-003 | runner metadata/timing field | `cargo test --test layout_snapshot_benchmark_contract --locked artifact_binds_environment_and_exact_shas -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact` |
| B-004 | required deterministic pre-gates | `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact` |
| B-005 | paired timing comparator | `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact` |
| B-006 | allocation comparator | `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact` |
| B-007 | base-tree trust/fingerprint gate | `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked fingerprint_mismatch_needs_rebaseline -- --exact` |
| B-008 | implementation bootstrap | `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact` |
| B-009 | exclusive promotion lifecycle | `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_promotion_never_self_authorize -- --exact`; manual diff check: promotion PR changes only `.github/benchmarks/gh61-baseline.json` and records independent review/current CI/SpecRail/merge authorization |

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

- [ ] Contract：固定 matrix、minimum operations、row aggregation、唯一 timing 字段、所有
      metadata/counter 与 invalid/partial/negative fixtures。
- [ ] Comparator：timing 2-of-3 双阈值、allocation 相对+绝对阈值、single outlier、
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
