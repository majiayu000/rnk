# Tech Spec：聊天布局 benchmark artifact、可信 baseline 与 promotion gate

## Linked Issue

GH-85: https://github.com/majiayu000/rnk/issues/85

## Product Spec

见 [`product.md`](product.md)。

GH-85 消费 GH-61 的 immutable snapshot producer、parity 与 per-frame deterministic work
counters，不改变其 layout/recovery 语义。#61 的实时label不是授权来源；GH-85 implementation
必须等 #61 实现合入后，在 exact merged SHA 上重新定位测量 seam。

## Codebase Context

以下锚点已在 `26499553b33a133071139d6baa6fce8b190ae0b3` 核实：

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Existing layout benches | `benches/layout.rs:71`, `benches/layout.rs:90`, `benches/layout.rs:137`, `benches/layout.rs:160` | Divan 只覆盖 engine creation、通用 full tree/grid/text/getters | 没有 chat mutation、strategy matrix、allocation/work artifact |
| Bench registration/deps | `Cargo.toml:76`, `Cargo.toml:83`, `Cargo.toml:85` | dev dependencies 已含 `serde_json`/`divan`，只注册现有四个 bench | 新 machine-readable bench 需要显式注册；依赖与 lockfile 必须同步 |
| Existing CI | `.github/workflows/ci.yml:50`, `.github/workflows/ci.yml:98`, `.github/workflows/ci.yml:225` | CI 编译 benches，并由 `ci-gate` 汇总八个既有 required jobs；没有base-owned benchmark trust root | 保持`ci.yml`与八job `ci-gate`不变；GH-85另写exact-head commit status，不把workflow job check当required |
| CI concurrency | `.github/workflows/ci.yml:8` | 同一 ref 的旧 PR run 会被取消 | benchmark artifact 必须绑定当前 exact base/head，取消后的部分结果不可复用 |
| Coverage | `.github/workflows/ci.yml:183` | coverage 目前 `continue-on-error`，不负责 performance decision | benchmark gate 不得借 coverage 的 advisory 状态宣称 green |
| GH-61 measurement dependency | `specs/GH61/product.md:33`, `specs/GH61/tech.md:389`, `specs/GH61/tech.md:391`, `specs/GH61/tasks.md:54` | GH-61 规划 per-frame deterministic work counters，benchmark 已拆到 #85 | GH-85 只读消费 merged seam；不得先发明未合入 public API |
| Split provenance | `specs/GH61/product.md:46`, `specs/GH61/tasks.md:20` | 当前 packet 明确把 workload/baseline/promotion/regression gate 排除到 #85 | GH-85 范围对应拆分前 B-024 至 B-028 |

## 计划变更清单

```json
{
  "issue": 85,
  "complete": true,
  "paths": [
    ".github/benchmarks/gh61-baseline.json",
    ".github/scripts/check_gh61_benchmark.py",
    ".github/workflows/layout-benchmark-authority.yml",
    "Cargo.lock",
    "Cargo.toml",
    "benches/chat_layout.rs",
    "benches/support/chat_layout.rs",
    "specs/GH61/product.md",
    "specs/GH61/tasks.md",
    "specs/GH61/tech.md",
    "specs/GH85/product.md",
    "specs/GH85/tasks.md",
    "specs/GH85/tech.md",
    "tests/fixtures/gh61_benchmark_schema.json",
    "tests/fixtures/gh85_gh61_dependency.json",
    "tests/layout_snapshot_benchmark_contract.rs"
  ],
  "owners": {
    ".github/benchmarks/gh61-baseline.json": "SP85-T5B",
    ".github/scripts/check_gh61_benchmark.py": "SP85-T2A",
    ".github/workflows/layout-benchmark-authority.yml": "SP85-T2A",
    "Cargo.lock": "SP85-T2A",
    "Cargo.toml": "SP85-T2A",
    "benches/chat_layout.rs": "SP85-T2A->SP85-T2B",
    "benches/support/chat_layout.rs": "SP85-T2A",
    "specs/GH61/product.md": "current-spec-pr",
    "specs/GH61/tasks.md": "current-spec-pr",
    "specs/GH61/tech.md": "current-spec-pr",
    "specs/GH85/product.md": "current-spec-pr",
    "specs/GH85/tasks.md": "current-spec-pr",
    "specs/GH85/tech.md": "current-spec-pr",
    "tests/fixtures/gh61_benchmark_schema.json": "SP85-T2A",
    "tests/fixtures/gh85_gh61_dependency.json": "SP85-T7B",
    "tests/layout_snapshot_benchmark_contract.rs": "SP85-T1->SP85-T2A"
  },
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
`gh85-benchmark-status-reporter` service/App/key/audit configuration全部在repo外由T3 external owner
管理，不对应也不得虚构planned repository source path。

## 设计方案

### 1. 依赖与 trusted phase-zero route classification

implementation 开始前必须同时满足：

- maintainer 对当前 exact packet/head 明确确认可以实施；readiness label 仅描述队列状态；
- #61 的 implementation 已合入，`GH61_MERGED_SHA` 已由 GitHub merged evidence 解析；
- duplicate search fresh通过，且maintainer明确授权两PR lifecycle中的implementation head；
  后续promotion head必须另行取得明确授权，前一PR授权不可复用。

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
`work_counter`、`strategy_entrypoints`、`counter_fields`、`prerequisite_commands`。两个
symbol object 只允许 `path`、`symbol`；三个 strategy entry 只允许 `strategy`、`path`、
`symbol`，且 strategy 集合严格等于 `{full, incremental, recovered}`；`counter_fields` 严格等于
`{visited_nodes, mutated_nodes, text_flow_recomputes, snapshot_nodes, rebuild_count}`。
`prerequisite_commands` 必须恰有三项；每项 exact keys 为 `id`、`category`、`spec_ref`、
`argv`、`working_directory`、`expected_exit_code`、`expected_matched`、`expected_passed`、
`expected_ignored`。`category` 是闭合 enum，必须按
`[parity, work_counter, allocation_correctness]` 顺序各出现一次；对应 `spec_ref` 必须严格为
`specs/GH61/product.md#B-012`、`specs/GH61/product.md#B-011`、
`specs/GH85/product.md#B-004`，不接受其他 category/ref pairing。`id` 与完整 `argv`
均唯一；`working_directory`必须是字面量`checkout_root`。`argv`必须匹配checker-owned闭合
allowlist：`["cargo","test",可选"--workspace",可选"--lib"或["--test",TEST_TARGET],
"--locked",EXACT_TEST,"--","--exact"]`；可选项顺序固定，`TEST_TARGET`/`EXACT_TEST`只能是
不含`/`、`\\`、`..`的单个UTF-8 token。其他Cargo子命令/flag、absolute path、manifest-path、
target-dir、shell token或环境赋值一律拒绝。parity 与
work-counter argv 必须由 GH-61 merged tree 中实际存在的 exact Rust tests 解析，不在本
spec 预写未来 GH-61 test 名。T7A只读search merged GH-61 tests/tasks verification：若其中
存在明确断言allocation counter归属、计数与reset语义的exact Rust test，discovery evidence
记录该真实argv；若不存在，只记录`missing(require GH85 fallback)`，不得提前写manifest。
T1随后创建并提交GH-85已规划
`tests/layout_snapshot_benchmark_contract.rs` 的 fallback，argv 严格等于
`["cargo","test","--test","layout_snapshot_benchmark_contract","--locked",
"allocation_counter_contract_is_correct_before_benchmark","--","--exact"]`，且
`working_directory="checkout_root"`。两条分支的 category/spec_ref 均保持
`allocation_correctness`/`specs/GH85/product.md#B-004`。expected 值固定为 exit 0、
matched 1、passed 1、ignored 0。
每个 path/symbol 必须在 `GH61_MERGED_SHA` tree 与 current HEAD 中唯一解析，merged SHA
必须是 HEAD 祖先。T7B只能在T1停止后定稿manifest：前两项取T7A real evidence，第三项取T7A
发现的real test或已经存在于T1 checkpoint的fallback；不存在的future test、placeholder或由
T2A稍后创建的路径都不得进入manifest。T2A只读消费final manifest并实现fallback behavior。
wiring test 必须实际调用三个 strategy 并读取所有
counters；缺字段、category 集不完整/重复/未知/错序、spec_ref 不匹配、
empty/missing/duplicate/unknown command、placeholder、absolute/traversal/symlink escape、
零匹配、多匹配、非祖先或任一
strategy 未接线都 blocked。

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
rust_toolchain = "1.88.0"
target = "x86_64-unknown-linux-gnu"
profile = { name="release", opt_level=3, lto="off", codegen_units=16,
            debug=0, debug_assertions=false, overflow_checks=false,
            panic="unwind", incremental=false, strip="none" }
runner_image = "ubuntu-24.04"
sandbox_image = "docker.io/library/rust:1.88.0-bookworm@sha256:4727898c104ecd2e22d780925832502faee9fe4e70581b8572af081370b315a0"
warmup_iterations = 3
leg_sample_count = 5
sample_count = 10 per scenario/strategy/batch row
batch_count = 3
paired_order = ABBA per batch
```

这些常量由support module唯一声明，schema fixture/checker/test引用同一合同，禁止维护第二份
名称或阈值表。workflow用exact `1.88.0` toolchain与`x86_64-unknown-linux-gnu` target构建，不能
解析`stable`、minimum range或runner当前预装版本。`ubuntu-24.04`是稳定compatibility label；
hosted image build/version只记diagnostic。toolchain/target/profile/runner-image/sandbox-image合同变化必须走
`contract_update_bootstrap`，合入后重新authority measurement与promotion；不得在normal compare
自动接受。`sample_count < 10`、batch数不等于3或paired order不等于ABBA时artifact无效。

### 3. Closed artifact schemas 与 hash

candidate、canonical 与 current-run compare artifacts 共用一个 closed envelope。top-level
exact keys 为：

```text
schema_version, checker_revision, artifact_role, source_sha, refs,
content_sha256, config_sha256, cargo_lock_sha256,
message_corpus_revision, message_corpus_sha256, rustc, target, profile,
runner_compatibility, runner_observation, workload, build, prerequisite_results,
paired_order, comparison_id, execution_trace, rows
```

- `artifact_role` 闭集为 `candidate`、`canonical`、`compare_base_current_run`、
  `compare_head_current_run`。
- `refs`是按role区分的closed object。canonical exact keys为`historical_source_sha`、
  `authority_repository_id`、`authority_workflow_ref`、`authority_default_ref_sha`、
  `authority_run_id`；future compare用这些字段和canonical自身digest重新取得并验证
  platform attestation。其source只需在
  未来PR base ancestry中可验证，不等于当前invocation refs。candidate/current-run exact keys为
  `current_pr_base_oid`、`current_merge_base_sha`、`current_head_sha`、`current_run_id`，且
  `merge-base(base,head)==base`。两种refs shape不得混用。
- `runner_compatibility` exact keys为`os`、`arch`、`target`、`rustc`、`runner_image`、
  `sandbox_image_digest`、`sandbox_policy_sha256`，仅这些
  稳定字段参与canonical compatibility。`runner_observation` exact keys为`cpu_model`、
  `logical_cpu_count`、`kernel_release`、`hosted_image_build`；它只作诊断，不进入canonical staleness判断，但同一
  ABBA comparison的base/head observation必须逐字段相同。
- `workload` exact keys 为
  `seed`、`target_size`、`viewport_sequence`、`warmup_iterations`、`leg_sample_count`、
  `sample_count`、`batch_count`、`scenario_matrix`、`paired_order_contract`；后者固定为
  `{"bootstrap":"not_applicable","compare":"ABBA"}`。
- `build` exact keys 为 `source_sha`、`manifest_sha256`、`cargo_lock_sha256`、
  `executable_sha256`、`target`、`profile`、`sandbox_image_digest`、`sandbox_policy_sha256`。
  candidate/canonical build source 必须等于
  artifact `source_sha`；compare base/head build source 必须分别等于current refs的
  `current_pr_base_oid`/`current_head_sha`。即使 `execution_trace=[]`，candidate/canonical 也必须有 nonempty
  `executable_sha256`。build `cargo_lock_sha256`/`target`/`profile` 必须分别等于 top-level
  同名值；authority measurement也必须使用相同pinned container policy，不得在trusted host直接执行
  source-controlled Cargo build/binary。
- `prerequisite_results` 必须恰有三项，顺序/数量/id/category/spec_ref 必须与 dependency
  manifest commands 精确一致；entry exact keys 为 `id`、`category`、`spec_ref`、
  `argv_sha256`、`source_sha`、`exit_code`、`matched`、`passed`、`ignored`。category 闭集、
  顺序与 category/ref pairing 仍适用；每项必须 exit 0、matched 1、passed 1、ignored 0；
  candidate/current compare 的 result source 为current exact checkout，canonical 为历史
  authority source SHA。
- `paired_order` 只允许 `not_applicable` 或 `ABBA`；candidate/canonical 必须是
  `not_applicable`、`comparison_id=null`、`execution_trace=[]`，current-run compare 必须是
  `ABBA` 且具有同一 nonempty `comparison_id`。
- `execution_trace` entry exact keys 为 `pair_id`、`batch_index`、`sequence_index`、`role`、
  `source_sha`、`binary_sha256`；每个 pair 的 sequence 必须精确为
  `[(0,base),(1,head),(2,head),(3,base)]`，且每个 `binary_sha256` 必须等于对应 role
  artifact 的 `build.executable_sha256`。
- row exact keys 为 `scenario`、`strategy`、`operation_count`、`sample_count`、
  `batch_index`、`pair_id`、`median_ns`、`allocation_count`、`allocated_bytes`、
  `visited_nodes`、`mutated_nodes`、`text_flow_recomputes`、`snapshot_nodes`、
  `rebuild_count`、`sample_observations`。`sample_observations`必须恰有10项，entry exact keys为
  `leg_index`、`sequence_index`、`leg_sample_index`、`role`、`source_sha`、`binary_sha256`、
  `start_state_sha256`、`reset_sequence`、`timing_ns`、`allocation_count`、`allocated_bytes`、
  `visited_nodes`、`mutated_nodes`、`text_flow_recomputes`、`snapshot_nodes`、`rebuild_count`、
  `operation_counters`。`operation_counters`必须恰有`operation_count`项，entry exact keys为
  `operation_index`、`visited_nodes`、`mutated_nodes`、`text_flow_recomputes`、`snapshot_nodes`、
  `rebuild_count`；operation index严格为0..operation_count-1，每个counter nonnegative，
  `rebuild_count`只允许0/1。sample的五个work totals分别由对应per-operation字段以u128 checked
  sum后checked转回schema integer，任一overflow blocked。每个row由该数组直接聚合；current-run
  两个base legs或两个head legs各采5 samples，聚合后`sample_count=10`。每个same-role leg必须
  贡献其原始`leg_sample_index=0..4`各一次；leg/sequence/role/source/binary必须匹配ABBA trace，
  禁止跨leg reuse、drop、duplicate或renumber。start state hash全部相同；reset sequence在每个leg
  内严格0..4。

所有 object 层级都设置 `additionalProperties=false`；未知 key、duplicate JSON key、未知
enum、缺 required key、非法 null、负 counter 或越界 integer 一律 blocked。`median_ns`
必须大于0。10个timing值排序后，以0-based index 4和5的u64值转u128、checked相加并向下整除2；
溢出或输入数不等于10时blocked。每个sample在warmup后必须从同一serialized target state重建并
将allocation/work instrumentation归零，再执行exact operation_count；10个sample的allocation/
work字段必须逐字段相同，row直接记录该共同值，不求和或平均。GH-61 per-operation
`rebuild_count`只能为0/1；每个recovered sample的aggregate等于operation_count，其他strategy
为0。reset evidence缺失或deterministic字段不一致时blocked。

hash 统一使用 SHA-256 小写 hex：

- `message_corpus_sha256`：exact corpus UTF-8 bytes；
- `build.manifest_sha256`：build source worktree 的 exact `Cargo.toml` bytes；
- `build.cargo_lock_sha256` 与 top-level `cargo_lock_sha256`：build source worktree 的 exact
  `Cargo.lock` bytes；
- `config_sha256`：对仅含 `schema_version`、`checker_revision`、完整 `workload`、
  `runner_compatibility`、`message_corpus_revision`、`message_corpus_sha256` 的 RFC 8785
  canonical JSON bytes 求 hash；volatile runner observation明确排除；
- `content_sha256`：对完整 artifact 移除且只移除 top-level `content_sha256` 后的 RFC 8785
  canonical JSON bytes 求 hash；不得排除 `source_sha`、role、runner、trace 或 rows；
- `build.executable_sha256` 与 trace `binary_sha256`：checker 实际执行的 exact bench
  executable bytes；
- `prerequisite_results[].argv_sha256`：dependency manifest 对应 argv 的 length-prefixed
  UTF-8 string array bytes。

candidate 的 `source_sha`等于current refs的exact implementation head；canonical 的
`source_sha`等于authority历史exact merged implementation/contract-update source SHA；current-run base/head 的
`source_sha`分别等于current refs的base/head。candidate不能改role后成为canonical，
canonical 必须由独立 rerun 生成。bench-only allocation instrumentation 不进入 production
report，也不新增 public `Any`、arbitrary closure 或运行时 allocator replacement API。

role-specific refs采用上述两个互斥shape；canonical历史refs不与未来PR current refs比较
相等，只验证`historical_source_sha`是future base祖先且attestation绑定bytes/digest。candidate
与两类current-run artifact共享current exact refs。除candidate/canonical的
`comparison_id=null` 外不允许 null。candidate/canonical row 的 `pair_id` 是对
artifact role、source SHA、scenario、strategy、batch index 的 length-prefixed UTF-8
bytes 求 SHA-256；current-run row 使用 compare protocol 的 `pair_id`。

negative fixtures 至少包含：top-level/nested unknown key、duplicate JSON key、missing row
key、unknown role/order、role/source/build mismatch、missing/unknown build key、
content/config/corpus/binary hash mismatch、empty/missing/duplicate/unknown/failed
prerequisite command/result、prerequisite category missing/duplicate/unknown/wrong-order、
category/spec_ref mismatch、allocation fallback missing 或 benchmark-before-prerequisite、
zero timing、negative allocation、candidate-as-canonical、canonical-as-current-run、
missing/duplicate/cross-run/wrong-order pair、historical-source-not-ancestor、canonical/current
refs shape混用、base-not-ancestor、merge-base-mismatch、volatile-observation-pair-mismatch、
absolute/traversal/symlink-escape prerequisite、argv allowlist escape、caller-supplied baseline、
authority mismatch与incompatible stable class。每个 fixture 必须 schema-targeted，
不能用另一处更早的 parse failure 冒充目标 predicate 覆盖。

### 4. Checker CLI 与 deterministic pre-gates

`.github/scripts/check_gh61_benchmark.py` 使用参数数组调用外部命令，并提供闭合 CLI：

- `--list-scenarios`：输出 machine-readable exact scenario/strategy/minimum matrix；
- `--validate-dependency-manifest PATH --repo PATH --gh61-merged-sha SHA`：执行 dependency
  ancestry/anchor/strategy/counter fail-closed gate；
- `--validate-artifact PATH --expected-role ROLE`：验证 closed schema、unknown/duplicate keys、
  role、hash、historical/current refs、stable/volatile runner字段、paired order、nonzero
  operation/sample与所有counter；
- `--validate-raw-transport PATH --route-artifact PATH --max-compressed-bytes N
  --max-uncompressed-bytes N --max-entries N --max-depth N --max-items N`：PATH必须是由trusted host
  通过Actions artifact REST API下载但尚未解包的immutable ZIP bytes；先做bounded archive preflight，
  再stream-extract到fresh contained dir并closed-parse，不执行其中路径或binary；
- `--mode bootstrap --repo PATH --pr-base-oid SHA --head-sha SHA --run-id ID
  --run-attempt N --route-artifact PATH --raw-artifact PATH --artifact-dir PATH --candidate-out PATH`：
  trusted_validate显式验证base/head objects、ancestry、exact merge-base、implementation diff、raw
  binding与output containment后，只从normalized hostile input写non-authoritative candidate；不得
  build/run head或从cwd/script path/environment隐式补参数；
- `--mode generate-authority-subject --repo PATH --repository-id ID --workflow-ref REF
  --default-ref-sha SHA --source-sha SHA --run-id ID --run-attempt N --target-root PATH
  --artifact-dir PATH --subject-out PATH --unsigned-metadata-out PATH`：仅default-ref-owned受信workflow
  可用，在repo外输出canonical subject与unsigned metadata；不得生成、伪造或内嵌attestation；
- `--mode finalize-authority --subject PATH --unsigned-metadata PATH --attestation-bundle PATH
  --attestation-id ID --repository-id ID --workflow-ref REF --default-ref-sha SHA --source-sha SHA
  --run-id ID --run-attempt N --authority-out PATH`：只能在full-SHA pinned attest v4 action成功后读取其exact
  `bundle-path`/`attestation-id` outputs，重验subject digest与workflow/run/source chain后输出final
  `authority.json`；missing/wrong output或bundle禁止finalize；
- `--mode validate-promotion --repo PATH --pr-base-oid SHA --head-sha SHA --run-id ID
  --run-attempt N --authority-envelope PATH --attestation-bundle PATH --authority-artifact-id ID
  --authority-artifact-digest DIGEST --authority-run-id ID --committed-canonical PATH`：read-only读取
  promotion head已提交blob与不可变authority handoff/GitHub artifact metadata，验证bytes/digest/
  attestation/diff；不得创建、修改或覆盖canonical path；
- `--mode compare --repo PATH --base-worktree PATH --pr-base-oid SHA --head-sha SHA
  --run-id ID --run-attempt N --route-artifact PATH --raw-artifact PATH --artifact-dir PATH`：从exact
  base tree解析canonical，只对same-run normalized raw base/head执行closed validation/comparison；
  不build/run PR binary，不接受调用方任意`--base` artifact或跨run raw measurement。

#### 4.1 Base-owned route、sandbox 与 exact-SHA status

`.github/workflows/layout-benchmark-authority.yml`是唯一PR benchmark workflow。它必须先由独立
trust-root PR合入protected default ref；workflow job check不作为required。maintainer必须把exact
active commit status context `gh85/layout-benchmark/vN`配置为ruleset/branch-protection required status；
`N`是service与base-owned workflow共同绑定的单调递增`reporter_epoch`，且context绑定
外部专用GitHub App `gh85-benchmark-status-reporter`的exact integration_id；required status必须在GitHub
当前评估的latest head/test-merge SHA上success。完成same-repo与fork双SHA smoke前不得measurement；
若Statuses API或ruleset拒绝任一SHA，实施blocked，不得改用workflow check、单SHA、repository App/token
或不校验source的context。所有repo workflow对`statuses`/`checks`均为none；Checks API只允许external
App service在provision/reinstall/id rotation时执行下述non-required registration lifecycle。

专用App只安装base repo，permissions精确为metadata read、pull requests read、commit statuses write、
checks write。
App private key/installation token只存在于external service的server-side secret manager/HSM，不得进入
repository、organization、environment Actions secrets，也不得在Actions mint。external endpoint、固定
OIDC audience `gh85-benchmark-status-reporter`、App id/installation id、allowed workflow path/ref/SHA set、
active reporter epoch/context、key version与service config digest是
T3维护的protected external configuration；packet不发明service repo source path或hardcode key。
pending/final request jobs只申请GitHub OIDC `id-token:write`并向exact endpoint发送closed bundle。
service验证token `iss=https://token.actions.githubusercontent.com`、exact audience、repository_id、
workflow_ref/path、workflow_sha/ref属于reviewed protected default-ref version、`event_name=pull_request_target`、
run_id/attempt及PR/base/head/binding；PR-head workflow/OIDC、wrong repo/audience/ref/SHA或replay均拒绝。

external service先在merge-locked maintenance window闭合判定lifecycle：

- `genesis`仅允许next `reporter_epoch=1`。service必须从GitHub App/installation inventory、server-side
  key/config inventory、ruleset/branch-protection与known status/check context audit完整证明不存在prior
  reporter App、installation、key、`gh85/layout-benchmark/v*`、`gh85/reporter-registration/v*`或任何prior
  rule/context/integration evidence，并记录含repository id、query/inventory digests、timestamp与auditor的
  closed `genesis_absence_receipt`；只有receipt验证通过后才可provision/验证initial
  App/installation/key/service config。genesis没有old credential，因此禁止要求或伪造revoke/canary。
- `rotation`要求current active epoch/context/integration evidence存在，next epoch严格递增且`>1`、不可复用/
  回退。service先provision并验证new credential/service config；此时old credential可暂存。registration
  验证通过后、final priming前必须按下述顺序撤销并产生closed `revocation_receipt`。

两条route随后都fresh读取current protected default-branch SHA，并以new server-side installation token调用
Checks API
`POST /repos/{base_owner}/{base_repo}/check-runs`创建唯一closed registration check run：name精确为
`gh85/reporter-registration/v{reporter_epoch}`，head SHA精确为该default-ref
SHA，`status=completed`、`conclusion=success`，`external_id`绑定repository/App/installation/config/key
version、reporter epoch与selected lifecycle receipt digest；output明确声明non-required、不得代表benchmark。
service随后调用
`GET /repos/{base_owner}/{base_repo}/check-runs/{check_run_id}` fresh读取该check run，验证id/name/head SHA/
status/conclusion/external_id及response `app.id`/`app.slug`均与service-held installation对应App匹配，
把check-run id、response digest与verified integration_id记入audit。maintainer只有取得该evidence后才能在
ruleset为required commit status选择verified integration_id。registration context本身永不required，其
success也不能映射或复制为benchmark status。rotation registration验证通过后、任何final new-epoch
priming前，service必须撤销被替换的old key/installation并记录revocation timestamp、credential/install id、
provider receipt及failed-canary digest为closed `revocation_receipt`；随后用old credential执行closed
read/write canary/API request，只有明确authentication failure且audit digest匹配receipt才可继续。
timeout、ambiguous response、仍成功或无receipt均blocked。genesis跳过整段revocation/canary且只能消费
已验证的`genesis_absence_receipt`。

registration manifest/audit与随后priming manifest schema都含nullable `genesis_absence_receipt`、
`revocation_receipt`，并要求精确XOR：恰一项为matching closed receipt、另一项不存在；neither、both、
receipt type/mode/epoch不匹配均blocked。genesis receipt只允许epoch 1且prior rule/context/integration均无；
revocation receipt只允许rotation epoch >1且prior active evidence闭合。

revocation证明完成后，service才通过GitHub REST fresh完整分页枚举base repo每个open PR；对每项fresh GET
current base/head/mergeability/test-merge，验证test-merge ordered parents后，只用new credential向head与
test-merge POST或覆盖
`gh85/layout-benchmark/v{reporter_epoch}=pending`。closed priming manifest记录sorted PR/pair set、page/query
digests、epoch、service config digest、new credential/installation identity、selected lifecycle receipt与两个status
ids/timestamps；missing/duplicate/invalid pair均blocked。每个SHA都必须fresh list/combined-query并证明latest
same-context status id等于post-revocation POST response、timestamp晚于revocation、state为pending、target
binding包含new config digest。old credential在revocation前写入的任何vN success必须被该latest pending覆盖；
revocation后canary保证其不能再竞态。原子ruleset切换前必须重新分页枚举并逐pair re-query，证明open-PR set
与pair digest仍相同；
变化则丢弃manifest并重新priming。只有全部pair已pending，maintainer才可在同一maintenance transaction把
service active epoch/config与ruleset required tuple原子切到
`(context=gh85/layout-benchmark/v{reporter_epoch},integration_id=current App)`；same-repo/fork smoke及ruleset
GET均通过后才解除merge lock。old context不再required，哪怕同一App写过success也因context不匹配不能
满足new epoch；routine key/install/config/trust rotation保持同一App integration id，不得把old status描述为
其他source，也不得删除/改写historical status；ruleset绑定的是App integration id而非installation/key，
revoke-before-final-priming不改变App integration id。

只有confirmed App credential compromise才在merge lock内额外provision新的App ID/integration与new epoch；
它是rotation且epoch >1，new App registration通过后同样必须先撤销old App installation/keys并取得closed
`revocation_receipt`，再只用new
App完成all-open-PR priming及原子ruleset/service switch。注册、revocation、priming、rotation或ruleset更新
任一步失败都保持blocked，无repo
workflow/check、unversioned context、宽松source或双required-context fallback。

workflow只监听`pull_request_target`的opened/synchronize/reopened/ready_for_review；payload先规范化
`pr_number`、base owner/repo/SHA与head owner/repo/SHA，再fresh查询current PR证明event head仍current。
它不监听`pull_request_review`，也不查询、解释或缓存review/marker/permission。workflow file始终来自
protected default ref。`phase_zero`不得checkout/execute/import PR head文件；它从exact `PR_BASE_OID`
（该tree缺approved checker时从exact protected default-ref SHA）用`git show`复制trusted checker到
runner temp，记录`trusted_policy_sha`与checker digest；两者都没有checker时blocked。

phase zero在任何prerequisite/build/benchmark前验证objects、base ancestry与exact merge-base，再以
NUL分隔name/status/raw-mode读取raw diff、关闭rename自动接受并验证tree entry。symlink(`120000`)、
submodule(`160000`)、file type/mode change、rename/copy、case/path normalization collision、unknown
class或空/ambiguous diff均blocked。regular-file topology classes闭合且按优先级匹配：

- `canonical`：仅`.github/benchmarks/gh61-baseline.json`；
- `benchmark_runtime`：`src/**/*.rs`、`crates/*/src/**/*.rs`与exact `benches/chat_layout.rs`；
- `benchmark_contract`：root/crate `Cargo.toml`/`Cargo.lock`、GH85 checker/authority workflow、
  `benches/support/chat_layout.rs`、contract test与两个GH85 fixture；
- `non_benchmark`：`examples/**`；排除GH85 contract/fixtures的ordinary `tests/**`/`tests/golden/**`；
  排除chat/support的其他`benches/**`；`docs/**`、`specs/**`、`video/**`、`.claude/**`；排除
  canonical/checker/authority workflow的其他`.github/**`；`crates/*/README.md`；以及exact root
  metadata `README.md`、`CONTRIBUTING.md`、`CHANGELOG.md`、`LICENSE`、`SECURITY.md`、
  `CODE_OF_CONDUCT.md`、`AGENTS.md`、`CLAUDE.md`、`DESIGN_ISSUES.md`、`.gitignore`、
  `rustfmt.toml`、`.rustfmt.toml`。

contract test必须以`git ls-files -z`枚举当前tree全部tracked paths，并证明每个regular file精确命中
一个class；unknown/new topology、zero/multiple matches均blocked，只能经authorized contract update
扩展。逐path分类后route必须唯一命中：

| route | exact raw-diff predicate | `route_status` | `authorization_status` |
| --- | --- | --- | --- |
| `initial_implementation_bootstrap` | base无canonical；至少一个exact initial chat bench，可另含non-benchmark | `bootstrap_valid` | `external_required` |
| `contract_update_bootstrap` | base有canonical；至少一个contract，可另含non-benchmark，无runtime/canonical | `contract_update_valid` | `external_required` |
| `canonical_only_promotion` | canonical缺失或不同；raw diff精确只有canonical | `promotion_valid` | `external_required` |
| `normal_trusted_compare` | base有trusted canonical；至少一个runtime，可另含non-benchmark，无contract/canonical | `comparison_valid` | `not_required` |
| `non_benchmark_change` | raw diff非空且全部non-benchmark | `not_applicable_valid` | `not_required` |

classifier不读取review或给出performance/merge authorization。branch protection单独要求approving
review并dismiss stale approvals；CONTRIBUTING maintainer merge authorization必须绑定current exact head。
benchmark status只能证明route/artifact/performance合同，不能授予或替代人工merge authorization。
`authorization_status`闭合为`external_required|not_required`；`route_status`与`performance_status`
沿用闭合enum。只有`comparison_valid + not_required + passed`映射`comparison_passed`。

workflow设置`gh85-pr-${pr_number}`、`cancel-in-progress:true`与每job timeout。所有PR jobs显式guard
`github.event_name == 'pull_request_target'`；`always` jobs使用`always() &&`同一guard。authority job只允许
`workflow_dispatch`，任何mixed dispatch执行blocked。所有job使用fresh `ubuntu-24.04` VM：

1. `status_pending_request`权限精确为`id-token:write`，无contents/pull/status/check write。它以exact
   audience取得OIDC并把event PR/base/head/run/attempt/workflow identity与`state=pending` request binding
   交给external service。service fresh `GET /repos/{base}/pulls/{pr}`解析current head、base、mergeability与
   current test-merge SHA，再调用`GET /repos/{base}/git/commits/{test_merge_sha}`证明ordered parents精确
   base、head；缺失/invalid pair就不写success。
   service对head与test-merge各POST相同context pending，并返回opaque receipt id/digest与bound pair；
   Actions只把receipt/pair作为controller outputs，不得到App credential。
2. `phase_zero`权限精确`contents:read`、`pull-requests:read`，不读取reviews。它上传唯一
   `gh85-phase-zero-${run_id}-${run_attempt}-${pr_number}`，只含base checker/route artifact；binding
   覆盖run/attempt/PR/base/head/merge-base/raw-diff/policy/checker/status-context，输出artifact identity、
   binding digest与`collect_expected`。
3. `sandbox_collect`权限`{}`，不接收route/checker/auth artifact。host用无凭证git fetch validated
   public base/head exact SHA，并先从trusted base Cargo manifests/lock执行`cargo fetch --locked`准备只读
   dependency cache；initial/normal route保证这些contract files未变。PR-controlled Cargo build scripts/
   binaries仅在不同的ephemeral Docker container运行，image精确为
   `docker.io/library/rust:1.88.0-bookworm@sha256:4727898c104ecd2e22d780925832502faee9fe4e70581b8572af081370b315a0`。
   每次build及每个ABBA leg使用fresh amd64 container，参数至少含`--platform linux/amd64
   --network none --read-only --cap-drop ALL
   --security-opt no-new-privileges --pids-limit 256 --memory 4g --cpus 2 --user 65532:65532`，无Docker
   socket、host workspace、host process/env/token、Actions runtime或credential挂载；只挂载source/
   dependency cache为RO、该base/head专属target/output为RW及bounded noexec/nosuid/nodev tmpfs。base与
   head、每leg output目录互不共享。host wait/remove并检查exit，hash固定文件后生成closed raw binding，
   才在container全部消失后由host-side pinned `raw_upload` action上传untrusted raw。job controller outputs
   精确为artifact name/id/digest、run/attempt/PR/head与binding digest，且只从host binding step/action
   outputs取得；container无`GITHUB_OUTPUT`/Actions command files，不能伪造。image升级走contract route。
4. `trusted_validate`fresh VM权限精确`actions:read`、`contents:read`、`pull-requests:read`。可信
   phase-zero artifact可用pinned download action；raw禁止`actions/download-artifact` auto-extraction。
   validator先从`needs.sandbox_collect.outputs`取得closed controller handoff，再调用
   `GET /repos/{base_owner}/{base_repo}/actions/artifacts/{artifact_id}`查询exact raw metadata，以
   name/id/digest/run/attempt/PR/head/binding/expiry逐字段匹配，再调用
   `GET /repos/{base_owner}/{base_repo}/actions/artifacts/{artifact_id}/zip`把immutable ZIP作为bytes下载到
   `RUNNER_TEMP`，不把response pipe给extractor。trusted bounded archive preflight在任何
   extraction前验证compressed/uncompressed size、entry count/depth、CRC、central/local headers、无
   trailing data，且entries精确为regular `manifest.json`、`base-current-run.raw.json`、
   `head-current-run.raw.json`、`trace.json`、`prerequisite-results.json`；absolute/traversal、duplicate
   normalized name、symlink/hardlink、unknown file均blocked。随后stream-extract with quotas到fresh
   contained dir并closed-parse；只执行phase artifact的base checker，不执行raw/PR binary。promotion
   historical artifact lookup也只在该job使用actions read。它独占validated tuple/artifact identity。
5. final `status_reporter_request`权限也精确`id-token:write`，只信trusted_validate outputs/identity与
   pending receipt；它取得fresh OIDC并发送closed final bundle。service重新fresh GET PR/test merge并重验
   object parents/mergeability，要求current `(head,test_merge)`与pending pair完全相同、run未supersede、
   validation binding有效。只有这样才向pair两端POST success；其他情况只允许对原bound pair写failure
   或保持pending，绝不向新pair success。new synchronize run只拥有新pair；service receipt、App/
   installation/config/key version、request/status ids与pair/run/binding进入不可变audit。

只有validation通过且tuple精确为`(bootstrap_valid,external_required,not_available)`、
`(contract_update_valid,external_required,not_available)`、
`(promotion_valid,external_required,not_available)`、
`(not_applicable_valid,not_required,not_available)`或
`(comparison_valid,not_required,passed)`时status可success。只有最后一项表示performance passed；
regression、needs_rebaseline、blocked、cancelled、unexpected skipped或其他tuple均failure。

#### 4.2 Deterministic prerequisites

base-owned workflow中的closed container entrypoint（不是PR head脚本，也不接收phase-zero checker
artifact）读取head/base checkout内只作为data的dependency manifest，先由固化在trusted workflow
中的manifest schema/digest与argv allowlist验证，再严格按
`parity -> work_counter -> allocation_correctness`顺序、以`shell=false`和声明的
`working_directory=checkout_root`执行三个prerequisite `argv`。collector先对checkout root取
realpath并验证其exact HEAD；解析所有manifest paths时拒绝absolute、`..`、NUL与symlink escape，
再把cwd固定为该root。argv必须匹配第1节closed Cargo exact-test allowlist；manifest不能指定
任意executor、cwd或environment。collector把每个exact category/spec_ref/result写入始终untrusted的
`prerequisite_results`；trusted_validate中的base-owned checker随后从actual raw bytes重验manifest
digest、command identity、顺序、exit/matched/passed/ignored及其发生在benchmark前的trace，且不重跑
PR binary。command array长度不为3、category缺失/重复/未知/错序、spec_ref pairing错误、id/argv
duplicate、unknown key、test zero-match、failed/ignored或result缺失/多出时，collector不得开始
benchmark且trusted decision为blocked。allocation fallback必须先由GH-85 contract test证明
allocation counter对operation的归属、计数和reset语义，不能用一次非零观察值替代correctness。
sandbox container先运行dependency wiring与全部prerequisite commands，trusted_validate再执行
artifact validation；任何前置失败都停止performance decision，但上传诊断artifact，禁止捕获异常
后返回success。

### 5. Trusted baseline 与 compare

compare 通过 `git show <pr_base_oid>:.github/benchmarks/gh61-baseline.json` 读取 repo-owned
baseline，验证：

- `artifact_role=canonical`，历史`source_sha`与`refs.historical_source_sha`一致且是current
  PR base的祖先；它无需等于current base/head，且通常早于二者；
- closed schema、`content_sha256`、`config_sha256`、corpus/Cargo hashes 全部重算一致；
- authority attestation绑定canonical exact bytes/digest、historical source、default-ref SHA与
  immutable authority run；当前PR base/merge-base/head/run只从invocation参数验证，不写入或
  要求等于canonical历史refs。

trust/staleness predicate 是闭合的：

- baseline missing、不是从 exact base tree 读取、role 非 canonical、content hash/attestation
  无效、historical source不在current base ancestry或current invocation refs无效：
  `blocked`；
- baseline 自身按其 schema 有效且 ancestry 可信，但 schema/checker/config/corpus/Cargo/
  toolchain/runner stable compatibility class 与 current compare 不兼容：`needs_rebaseline`；
- 只有所有 predicate 通过才是 `trusted`。checker 输出第一个及完整 `rejection_items[]`，
  不把 stale/untrusted 转换为零值 row。

canonical baseline 只授权 workload/config/fingerprint 可比较性与历史 promotion 来源；实际
threshold denominator 必须来自本次 run 的 `compare_base_current_run`，不能直接用 canonical
row，也不能复用 candidate 或旧 CI 的 raw base artifact。

所有bootstrap/compare/promotion validation在执行前必须证明base/head objects存在、
`git merge-base --is-ancestor PR_BASE_OID HEAD_SHA`成功，且
`git merge-base PR_BASE_OID HEAD_SHA == PR_BASE_OID`。trusted validation checkout HEAD必须等于
PR_BASE_OID；head object只允许read-only inspect，不能checkout/execute。promotion validation以
`git show HEAD_SHA:.github/benchmarks/gh61-baseline.json`读取
committed blob并与worktree path bytes一致，随后保持repo clean。失败时不得运行route-specific
executor。

PR path的结构必须等价于：

```yaml
name: layout-benchmark-authority
on:
  pull_request_target:
    types: [opened, synchronize, reopened, ready_for_review]
  workflow_dispatch:
    inputs:
      source_sha:
        required: true
        type: string
concurrency:
  group: gh85-pr-${{ github.event.pull_request.number }}
  cancel-in-progress: true
permissions: {}
jobs:
  status_pending_request:
    if: ${{ github.event_name == 'pull_request_target' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    permissions:
      id-token: write
    outputs:
      head_sha: ${{ steps.pending.outputs.head_sha }}
      test_merge_sha: ${{ steps.pending.outputs.test_merge_sha }}
      receipt_id: ${{ steps.pending.outputs.receipt_id }}
      receipt_digest: ${{ steps.pending.outputs.receipt_digest }}
    steps:
      - id: pending
        run: request exact-audience OIDC; send closed pending bundle to protected external reporter endpoint
  phase_zero:
    if: ${{ github.event_name == 'pull_request_target' }}
    needs: status_pending_request
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    permissions:
      contents: read
      pull-requests: read
    outputs:
      artifact_name: ${{ steps.route.outputs.artifact_name }}
      artifact_id: ${{ steps.route_upload.outputs.artifact-id }}
      artifact_digest: ${{ steps.route_upload.outputs.artifact-digest }}
      route_status: ${{ steps.route.outputs.route_status }}
      authorization_status: ${{ steps.route.outputs.authorization_status }} # external_required|not_required
      binding_digest: ${{ steps.route.outputs.binding_digest }}
      collect_expected: ${{ steps.route.outputs.collect_expected }}
    steps:
      # Never checkout/execute head; copy exact base/default-ref checker to RUNNER_TEMP.
      - id: route
        run: trusted-checker phase-zero with exact event refs; classify only, never query reviews
      - id: route_upload
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: ${{ steps.route.outputs.artifact_name }}
          path: ${{ runner.temp }}/gh85-phase-zero
          overwrite: false
  sandbox_collect:
    needs: phase_zero
    if: ${{ github.event_name == 'pull_request_target' && needs.phase_zero.outputs.collect_expected == 'true' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 45
    permissions: {}
    outputs:
      artifact_name: ${{ steps.raw_binding.outputs.artifact_name }}
      artifact_id: ${{ steps.raw_upload.outputs.artifact-id }}
      artifact_digest: ${{ steps.raw_upload.outputs.artifact-digest }}
      run_id: ${{ steps.raw_binding.outputs.run_id }}
      run_attempt: ${{ steps.raw_binding.outputs.run_attempt }}
      pr_number: ${{ steps.raw_binding.outputs.pr_number }}
      head_sha: ${{ steps.raw_binding.outputs.head_sha }}
      binding_digest: ${{ steps.raw_binding.outputs.binding_digest }}
    steps:
      - name: Collect hostile raw measurements in pinned ephemeral containers
        run: unauthenticated host fetch; trusted-base cargo fetch; fresh networkless hardened container per build/leg
      - id: raw_binding
        run: after all containers exit, host hashes fixed files and emits closed controller binding
      - id: raw_upload
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: ${{ steps.raw_binding.outputs.artifact_name }}
          path: ${{ runner.temp }}/gh85-raw
          overwrite: false
  trusted_validate:
    needs: [phase_zero, sandbox_collect]
    if: ${{ always() && github.event_name == 'pull_request_target' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    permissions:
      actions: read
      contents: read
      pull-requests: read
    outputs:
      route_status: ${{ steps.validate.outputs.route_status }}
      authorization_status: ${{ steps.validate.outputs.authorization_status }}
      performance_status: ${{ steps.validate.outputs.performance_status }}
      binding_digest: ${{ steps.validate.outputs.binding_digest }}
      artifact_id: ${{ steps.validated_upload.outputs.artifact-id }}
      artifact_digest: ${{ steps.validated_upload.outputs.artifact-digest }}
    steps:
      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4
        with:
          name: ${{ needs.phase_zero.outputs.artifact_name }}
          run-id: ${{ github.run_id }}
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
        with:
          ref: ${{ github.event.pull_request.base.sha }}
          fetch-depth: 0
          persist-credentials: false
      - name: Download raw ZIP bytes without auto-extraction
        run: require sandbox controller outputs; query matching REST metadata; download bytes; preflight before extraction
      - id: validate
        run: execute only phase-zero checker; hostile-parse raw; never execute PR binary
      - id: validated_upload
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: gh85-validated-${{ github.run_id }}-${{ github.run_attempt }}-${{ github.event.pull_request.number }}
          path: ${{ runner.temp }}/gh85-validated
          overwrite: false
  status_reporter_request:
    if: ${{ always() && github.event_name == 'pull_request_target' }}
    needs: [status_pending_request, phase_zero, sandbox_collect, trusted_validate]
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    permissions:
      id-token: write
    steps:
      - run: request fresh exact-audience OIDC; send receipt + validated final bundle to external reporter service
```

Actions reporter request bundle是closed JSON：公共keys为`schema_version`、`phase`、`state`、`context`、
`reporter_epoch`、`service_config_digest`、
`repository_id`、`repository`、`workflow_ref`、`workflow_sha`、`event_name`、`run_id`、`run_attempt`、
`pr_number`、`event_base_sha`、`event_head_sha`、`binding_digest`与`target_url`；pending禁止final fields，
final另要求pending `receipt_id/receipt_digest`及validated route/auth/performance/artifact identity。job用
exact audience请求GitHub OIDC，把token仅作为TLS request Authorization发送到T3记录的exact protected
endpoint；不把token写artifact/output/log。endpoint拒绝unknown/duplicate key、wrong content type、
redirect、expired token、clock skew超限、receipt reuse、phase/state mismatch，或request reporter epoch/
context/service config digest不等于protected active config。

service完成OIDC与fresh PR/test-merge验证后，才用server-side App installation token分别调用
`POST /repos/{base_owner}/{base_repo}/statuses/{bound_sha}`。`bound_sha`从closed request URL绑定且必须依次
为fresh head/test-merge pair；status body exact keys仅为`state`、
`context=gh85/layout-benchmark/v{reporter_epoch}`、
bounded `description`、`target_url`；`target_url`中的closed status-binding digest必须commit exact
reporter epoch与完整service config digest，并解析到同一immutable audit record。POST response只验证API
实际返回且本合同消费的`id`、`context`、
`state`、`creator.id`、`creator.login`、`creator.type`；不要求nonexistent response `sha`，也不把
creator user冒充App slug/id。随后service对pair各调用
`GET /repos/{base_owner}/{base_repo}/commits/{bound_sha}/status`，验证combined response `sha`等于URL
bound SHA，`statuses[]`包含同一status id/context/state/target_url。专用App identity另由service持有的
App/installation identity、registration check audit与ruleset integration_id证明，不从status creator推断。

service返回的closed receipt记录pair、两个status ids、request/status/combined-response digests、
App id/installation id/reporter epoch/key version/config digest、run/attempt/PR/binding与expiry。wrong pair/context/
source、caller-supplied status、stale run或redirect均blocked；repository workflow永远看不到App token。

job不得从`GITHUB_SHA`、`github.sha`、synthetic merge ref或本地branch推导refs。workflow job
check不配置为required；required identity只有head+test-merge上的active
`gh85/layout-benchmark/v{reporter_epoch}` status及
dedicated App integration id。两个reporter-request jobs只有`id-token:write`，repo workflow没有
`statuses:write`/`checks:write`；只有trusted_validate有`actions:read`，sandbox_collect为`permissions:{}`且不用`actions/checkout`或
trusted artifact。现有`.github/workflows/ci.yml`及八job `ci-gate`独立不变；两类required gates
各自通过，不能互相映射success。

所有trust-root action使用闭合full-SHA allowlist，YAML中保留对应human-readable major comment：

| action | reviewed exact SHA | comment |
| --- | --- | --- |
| `actions/checkout` | `11d5960a326750d5838078e36cf38b85af677262` | `# v4` |
| `actions/upload-artifact` | `ea165f8d65b6e75b540449e92b4886f43607fa02` | `# v4` |
| `actions/download-artifact` | `d3f86a106a0bac45b974a628896c90dbdf5c8093` | `# v4` |
| `actions/attest` | `1e69f48acb82d1966a394da916b4c1698aa569d6` | `# v4` |

mutable tag、short SHA、unknown action或同一action的其他SHA均blocked。任何action升级都是
benchmark-contract变更，必须先走authorized `contract_update_bootstrap`，合入default ref后重新
authority measurement和canonical-only promotion；普通implementation/compare不得顺手升级。

`sandbox_collect`的base-owned host先验证public refs并fetch；Cargo build script与benchmark binary
绝不直接在host执行：

```sh
[[ "$HEAD_REPOSITORY" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]
[[ "$BASE_REPOSITORY" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]
[[ "$HEAD_SHA" =~ ^[0-9a-f]{40}$ ]]
[[ "$PR_BASE_OID" =~ ^[0-9a-f]{40}$ ]]
git init "$RUNNER_TEMP/gh85-source"
git -C "$RUNNER_TEMP/gh85-source" -c credential.helper= -c http.extraHeader= fetch --no-tags \
  "https://github.com/$HEAD_REPOSITORY.git" "$HEAD_SHA"
git -C "$RUNNER_TEMP/gh85-source" -c credential.helper= -c http.extraHeader= fetch --no-tags \
  "https://github.com/$BASE_REPOSITORY.git" "$PR_BASE_OID"
git -C "$RUNNER_TEMP/gh85-source" cat-file -e "${HEAD_SHA}^{commit}"
git -C "$RUNNER_TEMP/gh85-source" cat-file -e "${PR_BASE_OID}^{commit}"
git -C "$RUNNER_TEMP/gh85-source" merge-base --is-ancestor "$PR_BASE_OID" "$HEAD_SHA"
test "$(git -C "$RUNNER_TEMP/gh85-source" merge-base "$PR_BASE_OID" "$HEAD_SHA")" = "$PR_BASE_OID"
git -C "$RUNNER_TEMP/gh85-source" worktree add --detach "$HEAD_CHECKOUT" "$HEAD_SHA"
git -C "$RUNNER_TEMP/gh85-source" worktree add --detach "$RUNNER_TEMP/gh85-base" "$PR_BASE_OID"
test "$(git -C "$HEAD_CHECKOUT" rev-parse HEAD)" = "$HEAD_SHA"
test "$(git -C "$RUNNER_TEMP/gh85-base" rev-parse HEAD)" = "$PR_BASE_OID"
cargo fetch --manifest-path "$RUNNER_TEMP/gh85-base/Cargo.toml" --locked
docker run --rm --platform linux/amd64 --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --pids-limit 256 --memory 4g --cpus 2 \
  --user 65532:65532 --tmpfs /tmp:rw,noexec,nosuid,nodev,size=256m \
  --mount type=bind,src="$HEAD_CHECKOUT",dst=/src,readonly \
  --mount type=bind,src="$DEPS_CACHE",dst=/usr/local/cargo/registry,readonly \
  --mount type=bind,src="$HEAD_TARGET",dst=/target \
  --mount type=bind,src="$HEAD_OUTPUT",dst=/out \
  docker.io/library/rust:1.88.0-bookworm@sha256:4727898c104ecd2e22d780925832502faee9fe4e70581b8572af081370b315a0 \
  cargo build --manifest-path /src/Cargo.toml --bench chat_layout --locked --release \
  --target-dir /target --message-format=json
```

host的`cargo fetch`只消费trusted base manifests/lock且不得执行build；route保证initial/normal head未改
这些contract files。上例省略的每次base build、head build及每个ABBA leg都必须使用新container；
leg把已hash binary/source以RO mount带入，仅挂该leg的empty output RW。禁止`--privileged`、host/pid/
ipc network、Docker socket、device、credential、Actions command file或任意host workspace mount。
host在每个container exit后wait、inspect exit、强制remove并确认无残留process/container，再从固定
output filenames读取/hash；head container从未得到base output path。全部hostile container结束后，
host-side pinned upload action才获得Actions runtime并上传raw ZIP。sandbox image label/digest、limits或
mount contract任一变化均走`contract_update_bootstrap`。

base-owned container entrypoint以参数数组分别执行
`["cargo","build","--manifest-path",CHECKOUT/Cargo.toml,"--bench","chat_layout",
"--locked","--release","--target-dir",TARGET_DIR,"--message-format=json"]`：base 的
`CHECKOUT/TARGET_DIR` 是container内exact base/head source及各自isolated target dir。host从Cargo JSON
解析executable并记录bytes的`binary_sha256`；所有这些字段仍是不可信raw evidence；
build/setup 不进入 timing。

每个 leg 用参数数组运行
`[EXECUTABLE,"--scenario",SCENARIO,"--strategy",STRATEGY,"--batch-index",N,
"--leg-index",L,"--seed","0x9e3779b97f4a7c15","--warmup-iterations","3",
"--sample-count","5","--artifact-out",LEG_PATH]`。checker 验证 leg artifact 后才写
fixed raw output中的`base-current-run.raw.json`与`head-current-run.raw.json`；此job不验证或赋予
信任，leg/raw files不是可复用comparison input；
trusted_validate随后从不同VM用base-owned checker重新parse/aggregate/validate。

raw transport不是`artifact_role`，也不是candidate/canonical/current-run validated artifact。
trusted_validate用Actions REST metadata定位exact id/name/digest/run/attempt且`expired=false`，通过
authenticated byte download保存ZIP，禁止raw使用download-artifact action。preflight在extract前
验证EOCD/central/local headers、CRC、compressed/uncompressed quota、entry count/depth、fixed regular
filenames、无absolute/traversal/duplicate-normalized/link/hardlink/unknown/trailing data；随后逐entry
stream-extract with quotas到fresh realpath-contained dir。任何raw字段只有在base checker从route binding
与actual bytes重算并写入validated artifact后才具有决策意义。

`comparison_id` 是对 `run-id`、current `pr_base_oid`、current `head_sha`、stable compatibility
class、volatile observation、
`config_sha256` 的 length-prefixed UTF-8 bytes 求 SHA-256；`pair_id` 是对
`comparison_id`、scenario、strategy、batch index 的同样编码求 SHA-256。每个
scenario/strategy/batch 依次运行 A(base)、B(head)、B(head)、A(base)，每 leg 5 samples；
两个 A legs 聚合为一个 base row，两个 B legs 聚合为一个 head row。两份 current-run
artifacts 必须有相同 comparison/trace、互补 role/source、相同 pair set，且每个 trace 精确
为 sequence 0..3 的 ABBA。缺 pair、重复 pair、跨 run ID、错序、binary/source mismatch、
stable compatibility/config不同或同一ABBA内base/head volatile observation不同均blocked；
volatile observation与历史canonical observation不同只进入诊断。

每个leg sample在warmup后先从同一serialized target state重建、重置allocation/work counters，
再执行exact operation_count；reset、operation与raw observations都写入leg诊断。每个聚合row必须
精确消费10个timing observations；排序后取第5/第6个值，以u128 checked sum向下整除2得到
`median_ns`。10个sample的allocation/work/rebuild aggregates必须逐字段相同，row取共同值；
任何不一致或recovered `rebuild_count != operation_count`均blocked。

每个 scenario/strategy 分别计算 3 个 paired batch：

- timing：`head/base > 1.20` 且 `head-base > 50_000ns`，3 batches 至少 2 个满足才失败；
- allocation count：任一batch相对增加 `> 10%` 且绝对增加 `> 8` 即失败；
- allocated bytes：任一batch相对增加 `> 10%` 且绝对增加 `> 4096` 即失败。

timing base/head `median_ns` 任一为 0 都是 invalid denominator，整个 comparison blocked。
allocation metric 的 base/head 都为 0 时该 metric 无回归；base 为 0、head 大于 0 时相对条件
视为满足，但仍只有 head 严格大于对应绝对 floor（8 或 4096）才失败。该规则逐 metric、
逐 batch 应用，allocation不使用2-of-3；禁止除零、NaN/Infinity或warning + fallback。

### 6. Bootstrap 与独立 promotion

首次implementation PR不存在trusted baseline，只允许`initial_implementation_bootstrap`：

```text
route_status=bootstrap_valid
authorization_status=external_required
performance_status=not_available
promotion_required=true
```

candidate 必须绑定 exact implementation head 并验证 B-001 至 B-003；implementation diff
不得包含 canonical baseline。job 被取消、失败或只产生部分 artifact 时，candidate 不得进入
promotion。candidate必须具有`artifact_role=candidate`、current refs、
`paired_order=not_applicable`、empty trace与有效hash；unknown key或role/hash/source不一致时
blocked。bootstrap CLI的repo/base/head/run/target/artifact/output参数全部required；output必须
位于显式artifact dir，realpath containment验证通过，且不得是repo canonical path。candidate
不进入authority generation输入，也不能被未来run复用。

同一planned `.github/workflows/layout-benchmark-authority.yml`也承载post-merge authority job，
但PR jobs与authority job按event/permissions隔离。workflow必须存在于protected default ref，
top-level permissions为空；authority job exact permissions为以下四项，所有未列权限为`none`：

```yaml
on:
  pull_request_target:
    types: [opened, synchronize, reopened, ready_for_review]
  workflow_dispatch:
    inputs:
      source_sha:
        required: true
        type: string
concurrency:
  group: ${{ github.event_name == 'workflow_dispatch' && format('gh85-authority-{0}', inputs.source_sha) || format('gh85-pr-{0}', github.event.pull_request.number) }}
  cancel-in-progress: true
permissions: {}
jobs:
  authority:
    if: >-
      github.event_name == 'workflow_dispatch' &&
      github.ref == format('refs/heads/{0}', github.event.repository.default_branch)
    runs-on: ubuntu-24.04
    timeout-minutes: 60
    permissions:
      contents: read
      id-token: write
      attestations: write
      artifact-metadata: write
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
        with:
          ref: ${{ github.sha }}
          fetch-depth: 0
          persist-credentials: false
      - name: Generate unsigned subject and metadata
        run: trusted-checker --mode generate-authority-subject with all exact inputs
      - name: Attest canonical subject
        id: attest
        uses: actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6 # v4
        with:
          subject-path: ${{ runner.temp }}/gh85-authority/canonical.json
      - name: Finalize authority envelope after attestation
        run: >-
          trusted-checker --mode finalize-authority
          --attestation-bundle "${{ steps.attest.outputs.bundle-path }}"
          --attestation-id "${{ steps.attest.outputs.attestation-id }}" with all exact inputs
      - name: Upload immutable authority handoff
        id: authority_upload
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: gh85-authority-${{ github.run_id }}-${{ github.run_attempt }}-${{ inputs.source_sha }}
          path: |
            ${{ runner.temp }}/gh85-authority/canonical.json
            ${{ runner.temp }}/gh85-authority/unsigned-metadata.json
            ${{ steps.attest.outputs.bundle-path }}
            ${{ runner.temp }}/gh85-authority/authority.json
          overwrite: false
```

authority job只能在implementation或authorized contract update已合入、source SHA是exact default
ref祖先且对应merge authorization evidence存在时由maintainer触发。PR head workflow、artifact、
checker或token不得调用`workflow_dispatch`、影响authority inputs或取得attestation；phase-zero
artifact也不授予authority。authority job不checkout PR head，不运行PR artifact，只运行其自身
trusted default-ref checker。`generate-authority-subject`先fresh测量并只写canonical subject与unsigned
metadata；full-SHA pinned attest v4 action step id必须是`attest`；`finalize-authority`只能在该step成功后消费exact
`${{ steps.attest.outputs.bundle-path }}`与`${{ steps.attest.outputs.attestation-id }}`。不得声称finalize
发生在attest之前，也不得让generator构造bundle或final envelope。

implementation或authorized contract update合入后，default-ref-owned受信workflow运行
`generate-authority-subject`。workflow只接受repository default branch的exact SHA与已记录
`AUTHORITY_SOURCE_SHA`（即对应已合入implementation或contract-update exact source SHA），在repo外两个隔离
目录分别作为source/target；验证default-ref object、source object、
`source_sha`是`default_ref_sha`祖先、source checkout exact HEAD、dependency manifest、closed
prerequisites、build/config/corpus hashes后fresh测量。unsigned metadata只记录subject digest、
repository/workflow/default-ref/source/run/attempt与closed artifact name；final `authority.json`在
attest之后闭合为：

```text
canonical_subject_path
canonical_sha256
historical_source_sha
repository_id
authority_workflow_ref
authority_default_ref_sha
authority_run_id
authority_run_attempt
attestation_id
attestation_bundle_sha256
authority_artifact_name
```

finalizer重算subject/bundle digest并验证bundle subject、repository/workflow/run/source和action
attestation id；canonical bytes不嵌入bundle digest，避免自引用。upload step完成后，T5A从
`authority_upload.outputs.artifact-id`与`artifact-digest`记录实际artifact id/digest/run/attempt/name；
这些post-upload字段不反写已上传内容。closed name在run/attempt/source维度唯一，`overwrite=false`；
同名重复、missing action output、bundle path越界或final envelope早于attest均blocked。

promotion CI按exact repository id、artifact id、run id/attempt、name与artifact digest从GitHub API
查询并下载immutable artifact，要求非expired且恰有canonical subject、unsigned metadata、action
bundle、final envelope各一份；再以artifact的workflow_run.id调用
`GET /repos/{owner}/{repo}/actions/runs/{run_id}`，要求entire run `event=workflow_dispatch`、
`conclusion=success`、head SHA/ref/workflow path/attempt均与authority envelope相同，不能只接受authority
job success或artifact存在。missing/expired/wrong run/id/digest/name/bundle/conclusion均blocked。随后以平台
root验证签名、issuer、subject与workflow identity；仅重算JSON hash不算authentication。authority
workflow与checker都不得写任何checkout中的
`.github/benchmarks/gh61-baseline.json`。implementation candidate、promotion branch contents或
caller提供的canonical bytes都不能作为authority输入。

promotion与future compare必须先核对local canonical SHA-256等于expected subject digest，再执行：

```sh
gh attestation verify "$CANONICAL_FILE" \
  -R "$EXPECTED_REPOSITORY" \
  --signer-workflow "$EXPECTED_REPOSITORY/.github/workflows/layout-benchmark-authority.yml" \
  --signer-digest "$AUTHORITY_DEFAULT_REF_SHA" \
  --source-ref "refs/heads/$DEFAULT_BRANCH" \
  --source-digest "$AUTHORITY_DEFAULT_REF_SHA" \
  --deny-self-hosted-runners \
  --format json >"$RUNNER_TEMP/gh85-attestation-verification.json"
```

trusted checker随后只从verified result的certificate/subject读取不可伪造identity，要求subject
digest精确等于canonical SHA-256、repository与workflow path/ref精确匹配、source digest等于
authority default-ref SHA、source ref为protected default ref、certificate event为
`workflow_dispatch`；不能用可由workflow控制的predicate字段替代certificate约束。

baseline-promotion PR的raw diff必须精确只有`.github/benchmarks/gh61-baseline.json`，其bytes必须
来自上述immutable authority handoff；即使docs/specs或其他non-benchmark path也不得混入。
base-owned workflow选择`canonical_only_promotion`，从exact promotion head用
`git show HEAD_SHA:.github/benchmarks/gh61-baseline.json`读取committed blob，并用
`validate-promotion` read-only验证：

1. current base/head objects存在，base是head祖先且exact merge-base等于base；
2. 未剥离任何path的raw diff精确等于canonical path，base blob缺失或与head blob不同；
3. GitHub API artifact metadata证明id/digest/name/run/attempt完全匹配、未expired且四个handoff文件
   各恰一份；final envelope的repository/workflow/run/default-ref/source链闭合，action bundle平台
   attestation签名与subject digest验证通过，historical source是promotion base祖先；
4. committed blob bytes与`canonical_bytes`逐byte相同，SHA-256与`canonical_sha256`一致，closed
   schema/content/config/corpus/build hashes重新计算一致；
5. validation前后repo status与canonical inode/bytes/digest保持不变，checker没有write handle。

任一失败返回blocked；CI不得调用authority generator、不得把`canonical-out`指向repo、不得先
覆盖committed file再验证。成功decision只能是`promotion_valid`，它不表示性能无回归。
promotion仍需current exact-head CI、independent review、resolved review threads与maintainer
对同一head的单独merge authorization；只有合入future base tree后才能用于normal compare。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | fixed workload matrix | `cargo test --test layout_snapshot_benchmark_contract --locked fixed_six_scenario_matrix_has_minimum_nonzero_operations -- --exact` |
| B-002 | artifact aggregation/closed schema | `cargo test --test layout_snapshot_benchmark_contract --locked median_ns_is_the_only_timing_field -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked ten_sample_even_median_and_deterministic_counters_are_exact -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked per_operation_counters_sum_checked_and_abba_samples_keep_leg_identity -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked recovered_rows_aggregate_one_rebuild_per_operation -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked closed_schema_rejects_unknown_duplicate_and_partial_rows -- --exact` |
| B-003 | roles/source/build/runner/ref separation | `cargo test --test layout_snapshot_benchmark_contract --locked artifact_hashes_cover_roles_sources_config_corpus_trace_and_rows -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked all_roles_require_closed_build_provenance -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked candidate_canonical_and_current_run_roles_are_not_interchangeable -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked canonical_refs_are_historical_and_current_refs_are_invocation_scoped -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked runner_compatibility_excludes_volatile_cpu_identity -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked abba_requires_identical_current_runner_observation -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked pinned_toolchain_target_profile_and_runner_class_are_closed -- --exact` |
| B-004 | base-owned sandbox/OIDC reporter/status/dependency gates | `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_uses_base_owned_checker_and_rejects_untrusted_head_policy -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_rejects_mixed_spec_symlink_mode_and_ambiguous_diffs -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked phase_zero_same_run_handoff_is_exact_and_replay_safe -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked workflow_jobs_have_exact_permissions_and_isolated_fresh_vms -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked sandbox_collect_isolates_pr_code_in_pinned_networkless_containers -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked sandbox_rejects_mutable_image_network_privilege_host_mount_and_output_reuse -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked trusted_validate_preflights_raw_zip_bytes_before_extraction -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked raw_zip_preflight_rejects_bombs_links_traversal_duplicates_crc_and_trailing_data -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked raw_upload_controller_outputs_bind_artifact_to_head_and_run -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked dedicated_reporter_oidc_rejects_repo_token_pr_head_workflow_and_spoofed_context -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked reporter_service_verifies_oidc_workflow_repository_and_binding_claims -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked reporter_registration_check_binds_required_status_to_verified_app_integration -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked registration_check_is_non_required_and_cannot_satisfy_benchmark_context -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_rotation_keeps_same_app_integration_and_versions_context -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_rotation_primes_all_open_pr_head_and_test_merge_pairs_before_switch -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked reporter_compromise_provisions_new_app_epoch_and_revokes_old_credentials -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked required_statuses_bind_current_head_and_test_merge_for_same_repo_and_fork -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked commit_status_response_and_combined_status_schema_are_exact -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked newest_head_concurrency_replay_and_timeout_are_fail_closed -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_matches_merged_gh61_and_all_strategies -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_requires_complete_prerequisite_category_set -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_missing_duplicate_unknown_categories_and_spec_refs -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked dependency_manifest_rejects_invalid_prerequisite_command_arrays -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked allocation_correctness_fallback_runs_before_benchmark -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_paths_and_argv_are_contained -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked prerequisite_commands_execute_and_record_before_benchmark -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked failed_prerequisite_never_reports_performance_green -- --exact` |
| B-005 | exact ancestry/ABBA/timing | `cargo test --test layout_snapshot_benchmark_contract --locked workflow_binds_event_head_and_base_without_merge_ref -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_compare_require_base_ancestor_and_exact_merge_base -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked same_runner_abba_builds_exact_base_and_head_and_rejects_pair_mismatch -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked timing_requires_two_of_three_paired_regressions -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked zero_timing_denominator_is_blocked -- --exact` |
| B-006 | allocation comparator | `cargo test --test layout_snapshot_benchmark_contract --locked allocation_requires_relative_and_absolute_thresholds -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked allocation_regression_fails_on_any_paired_batch -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked zero_allocation_denominator_uses_absolute_floor -- --exact` |
| B-007 | base-tree trust/stable compatibility gate | `cargo test --test layout_snapshot_benchmark_contract --locked trusted_baseline_rejects_self_stale_and_untrusted_sources -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked trust_predicates_distinguish_blocked_from_needs_rebaseline -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked canonical_refs_are_historical_and_current_refs_are_invocation_scoped -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked runner_compatibility_excludes_volatile_cpu_identity -- --exact` |
| B-008 | five routes/external authorization/performance/path separation | `cargo test --test layout_snapshot_benchmark_contract --locked route_selection_is_mutually_exclusive_and_only_comparison_passed_is_green -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked safe_docs_route_is_not_applicable_and_mixed_runtime_routes_are_closed -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked repository_path_classes_cover_legitimate_topology_and_block_unknowns -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked all_current_tracked_paths_match_exactly_one_class -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked canonical_promotion_diff_is_exactly_one_path -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked restricted_routes_require_external_authorization_without_review_interpretation -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked benchmark_status_never_grants_merge_authorization -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked review_rules_and_contributing_authorization_are_external_prerequisites -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked external_authorization_and_performance_status_are_independent -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked authorized_contract_update_is_non_green_and_requires_rebaseline_promotion -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_requires_explicit_repo_refs_and_exact_merge_base -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked implementation_writes_candidate_but_never_canonical_baseline -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked partial_candidate_never_authorizes_promotion -- --exact` |
| B-009 | three-stage authority/immutable read-only promotion | `cargo test --test layout_snapshot_benchmark_contract --locked trust_root_actions_are_pinned_to_reviewed_full_shas -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked authority_workflow_permissions_and_attestation_identity_are_exact -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked authority_pipeline_requires_action_bundle_outputs_and_finalizes_after_attest -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked authority_artifact_handoff_rejects_missing_expired_wrong_run_id_digest_or_bundle -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked combined_workflow_event_guards_and_authority_whole_run_success_are_closed -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked promotion_validation_is_read_only_and_authority_bound -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked promotion_rejects_committed_blob_not_matching_authority -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked bootstrap_and_promotion_never_self_authorize -- --exact`; manual diff check: promotion PR canonical bytes match exact authority handoff and records current exact-head repository CI、independent review、resolved threads与maintainer merge authorization |

| B-004 | reporter epoch/config binding supplement | `cargo test --test layout_snapshot_benchmark_contract --locked reporter_epoch_and_config_digest_bind_oidc_status_and_receipt -- --exact` |
| B-004 | revoke-before-final-priming ordering | `cargo test --test layout_snapshot_benchmark_contract --locked rotation_revocation_requires_failed_old_credential_canary_and_audit_receipt -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked reporter_rotation_revokes_old_credential_before_final_epoch_priming -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked post_revocation_pending_is_latest_new_credential_status_and_overwrites_old_success -- --exact` |
| B-004 | genesis/rotation receipt XOR | `cargo test --test layout_snapshot_benchmark_contract --locked genesis_epoch_one_requires_audited_absence_without_revocation_canary -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked lifecycle_manifests_require_exactly_one_genesis_or_revocation_receipt -- --exact`; `cargo test --test layout_snapshot_benchmark_contract --locked genesis_rejects_prior_rule_context_integration_and_rotation_rejects_epoch_one -- --exact` |

## 数据流

```text
base-owned pull_request_target run
  -> status_pending_request: exact-audience OIDC + closed pending bundle
  -> external dedicated App service: verify workflow identity + current head/test-merge -> pending pair
  -> phase_zero: exact refs/diff -> five-route classification only
  -> same-run route/checker artifact (trusted fresh VM)
  -> sandbox_collect: host fetch + pinned networkless ephemeral containers
  -> raw ZIP uploaded host-side after hostile containers exit
  -> trusted_validate: REST byte download + archive preflight + base checker
  -> route_status + authorization_status + performance_status + validated artifact
  -> status_reporter_request: fresh OIDC + receipt + validated final bundle
  -> external service re-query: same pair only -> dual success|failure + combined-status audit

post-implementation exact merged SHA
  -> generate-authority-subject (subject + unsigned metadata)
  -> full-SHA pinned attest v4 action
  -> finalize-authority (action bundle path/id required)
  -> upload-artifact immutable four-file handoff
  -> independent baseline-promotion PR commits exact subject bytes
  -> read-only promotion validation
  -> independent review + current exact-head repository CI + resolved threads + maintainer merge authorization
  -> canonical baseline in future PR base tree
```

artifact 不进入 runtime 持久化或 public API。candidate 只作为 current-run CI artifact；canonical
baseline 是唯一 checked-in performance evidence。

```mermaid
sequenceDiagram
    participant P as phase_zero (trusted base)
    participant U as sandbox_collect (zero permission host)
    participant V as trusted_validate (trusted fresh VM)
    participant Q as OIDC reporter request jobs
    participant S as external dedicated App service
    participant A as authority (default ref)
    participant R as promotion PR
    S->>S: lock; genesis(absence) XOR rotation(revoke+canary); final-prime pairs; switch tuple
    Q->>S: OIDC pending bundle
    S->>S: verify workflow; fresh head/test-merge parents; POST pending pair
    P->>P: classify diff without reading reviews
    P-->>U: controller sends only validated public refs, never trusted artifact
    U->>U: host fetch, then fresh hardened container per build/leg
    P-->>V: bound checker and route artifact
    U-->>V: immutable hostile raw ZIP identity only
    V->>V: REST bytes, archive preflight, base checker; no PR execution
    V-->>Q: validated tuple and artifact identity
    Q->>S: fresh OIDC final bundle + pending receipt
    S->>S: re-query same pair; POST final pair; GET combined statuses
    A->>A: generate subject then attest then finalize
    A->>R: immutable subject/bundle/envelope artifact identity
    R->>R: read-only verify committed subject bytes and attestation
```

## 备选方案

- **复用 `benches/layout.rs` 的通用 microbenchmark**：拒绝；没有 chat mutation、producer
  strategy 与 versioned artifact。
- **解析 Divan pretty output**：拒绝；未版本化文本不是稳定 machine contract。
- **单次 wall-clock 超阈值即失败**：拒绝；hosted runner 噪声会制造不稳定门。
- **从 feature head 读取 baseline**：拒绝；允许当前改动自行降低标准。
- **bootstrap 直接写 canonical baseline**：拒绝；把测量者、writer 与授权者合为同一 PR。
- **phase zero与benchmark拆成两个workflow/run**：拒绝；跨run artifact/status transport会引入
  cancel、replay与required-status竞态，单一base-owned run可以闭合identity与ordering。
- **repository `GITHUB_TOKEN`或repo内App token直接写required status**：拒绝；PR workflow不可持有或
  mint reporter credential，专用external App service必须先验证OIDC workflow identity与current pair。

## 风险

- **Security**：PR Cargo/build/binary只在fixed-digest、networkless、read-only、no-capability ephemeral
  containers执行；没有host/Actions/Docker socket挂载，base/head输出隔离。trusted validator先对raw ZIP
  bytes做bounded archive preflight再解析；外部命令使用参数数组。
- **Compatibility**：GH-61 尚未实现，真实 work-counter seam 可能改变；实现必须在 merged
  SHA 重新定位并更新 spec，禁止用 guessed adapter 静默兼容。
- **Performance/CI noise**：runner 调度、thermal、toolchain 漂移影响 timing；same-runner
  ABBA、3 batches与双阈值控制噪声。stable compatibility class决定跨run可比性；volatile CPU
  observation不强绑canonical，但本次base/head必须一致。
- **Evidence**：bootstrap 没有旧 scenario baseline；其状态必须与 performance green 分离。
- **Authorization**：automated classifier/status不读取reviews；required review/stale dismissal与
  maintainer final authorization由branch protection/CONTRIBUTING独立绑定exact head。status不能授权merge。
- **Reporter availability/key lifecycle**：service unavailable、credential revocation或audit gap时只允许
  pending/failure并blocked；T3负责genesis absence/rotation revocation XOR、monotonic epoch、versioned
  registration/context、pre-priming revocation canary/receipt、post-revocation all-open-PR pending overwrite、
  atomic ruleset switch及compromise-only App replacement，禁止repo token、unversioned context或双required
  context fallback。
- **Maintenance**：schema/support/checker/test 若复制常量会漂移；固定矩阵由一个 source
  生成并以 closed negative fixtures 验证。

## 测试计划

- [ ] Dependency：merged GH-61 ancestry、closed anchor manifest、full/incremental/recovered
      wiring 与全部 counter fields。
- [ ] Contract：固定matrix、minimum operations、exact 10-sample reset/checked even median、
      deterministic allocation/work equality、closed/unknown-key policy、historical/current refs、
      stable/volatile runner fields、source/config/corpus/content/binary hashes与负例。
- [ ] Comparator：base ancestor/exact merge-base、isolated builds、same-runner ABBA trace/pair
      identity、timing 2-of-3、allocation any-batch双阈值、zero denominator、self/stale/untrusted/
      missing baseline与stable compatibility mismatch。
- [ ] Lifecycle：combined guarded events、five mutually-exclusive routes、external human authorization、
      route/auth/performance status separation、sandbox/validator/OIDC requester/service isolation、head+test-merge
      dedicated-App status identity、versioned registration、genesis absence/rotation revocation receipt XOR、
      revoke-before-final-priming canary/receipt、post-revocation latest-pending overwrite、same-App routine
      rotation与compromise new-App replacement、
      same-repo/fork smoke、
      real response/combined schema、raw controller handoff、
      review-rule provisioning、concurrency/timeout/newest-pair invalidation、full-SHA action allowlist、
      explicit bootstrap inputs、candidate
      non-reuse、三阶段default-ref authority、immutable upload/download handoff与read-only promotion。
- [ ] Full gates：
      `cargo fmt --all -- --check`；
      `cargo check --workspace --all-targets --all-features --locked`；
      `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
      `cargo test --workspace --all-targets --all-features --locked`；
      `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`。
- [ ] GitHub：current exact-head required repository CI、independent review、resolved review
      threads与maintainer对同一head的explicit merge authorization；labels仅描述状态。

## 回滚方案

若 checker、harness 或 CI integration 产生错误阻断，整体回滚 GH-85 implementation PR，
恢复原有 CI；不得保留一个把 invalid evidence 判 green 的宽松 fallback。candidate 为
untracked artifact，可直接丢弃。

若仅 timing 因 runner noise 不可比较，可把 timing decision 降为 `needs_rebaseline`，但保留
required parity/work/allocation/schema/exact-head gates；不得删除 artifact 字段或伪造 base。
若 canonical baseline 有误，通过新的独立 promotion PR 重新测量和替换，不直接编辑 SHA 或
在 feature PR 内放宽阈值。回滚后 #85 保持打开，保存 exact failed head/CI/artifact/review
证据；#61 与其他 layout correctness 合同不回滚。
