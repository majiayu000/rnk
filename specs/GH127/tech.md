# Tech Spec：线性 styled boundary 归一化

## Linked Issue

GH-127: https://github.com/majiayu000/rnk/issues/127

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":127,"complete":true,"paths":["src/layout/text_flow.rs","src/layout/text_flow/style_normalization.rs","src/layout/text_flow/tests.rs","src/layout/text_flow/tests/style_normalization.rs","tests/text_flow_style_normalization.rs"],"spec_refs":["specs/GH127/product.md","specs/GH127/tech.md","specs/GH127/tasks.md","specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本 packet 只规划 GH-127。GH-58 是已存在的 TextFlow 行为合同；GH-101 只有 issue-native
workflow closure 要求，没有 `specs/GH101/`，因此不能伪造为 `spec_refs`。#126、#128、
#129、#130 同样没有 spec packet；它们通过 live issue/PR/merge dependency gate 与 exact
regression commands 验证。

## Codebase Context

以下锚点已在包含 #126 的 clean base
`50f6a203c1861814d288d4bdeae0e28d877af34c` 上通过 Read/`rg` 核实。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| styled input | `src/layout/text_flow.rs:40`, `src/layout/text_flow.rs:47`, `src/layout/text_flow.rs:68` | `StyledTextRange` 持有 public `Range<usize>` 与完整 `Style`；`TextFlowInput` 保存 caller 原始 vector | 优化只能建立 private view，不能排序写回或改变 public/cache identity |
| result identity | `src/layout/text_flow.rs:107`, `src/layout/text_flow.rs:113`, `src/layout/text_flow.rs:188` | cache identity clone 完整 input/options；flow 公开 tokens/rows/map/diagnostics | compatibility oracle 必须逐字段比较完整结果，不只比较 rendered text |
| build ordering | `src/layout/text_flow.rs:198`, `src/layout/text_flow.rs:211`, `src/layout/text_flow.rs:222` | immediate interruption 先于 option/range validation；成功 tokenization 后才 layout/map/publish | 新 preprocessing 不得改变 immediate interrupt、typed validation 与 atomic publish 顺序 |
| cache publication | `src/layout/text_flow.rs:279`, `src/layout/text_flow.rs:294`, `src/layout/text_flow.rs:303`, `src/layout/text_flow.rs:309` | cache 先比较 exact identity；completed flow 构建成功后才递增 count/publish Arc | interruption/error 必须保留上一 Arc 与 build count |
| typed errors | `src/layout/text_flow.rs:323`, `src/layout/text_flow.rs:347` | invalid、overlap、coverage、overflow、interrupted 是 closed `TextFlowError` | 不增加 string/boolean fallback，也不改变现有 variant payload |
| range validation | `src/layout/text_flow.rs:376`, `src/layout/text_flow.rs:389`, `src/layout/text_flow.rs:396` | 先按 caller 顺序检查 bounds/char boundary，再把 non-empty ranges 按 start 排序检查 overlap | private normalized plan 可复用这次排序；必须保留 first-invalid 与 overlap pair |
| quadratic normalization | `src/layout/text_flow.rs:407`, `src/layout/text_flow.rs:414`, `src/layout/text_flow.rs:420`, `src/layout/text_flow.rs:428` | 每个 grapheme 用 `.find()` 扫全部 ranges，再用 `flat_map(start,end)` 扫全部 endpoints | `G × R` 根因；GH-127 唯一生产修改点 |
| existing core tests | `src/layout/text_flow/tests.rs` | 已覆盖 cache、styled runs、split combining/ZWJ、部分 invalid 与 interruption；文件已超过 800 行 | 自然拆分 styled-normalization unit 子模块；父文件只保留模块声明和必要的 stable exact-selector wrapper，并回到 800 行以内 |
| engine first-source contract | `src/layout/engine/text_flow_bridge.rs:350`, `src/layout/engine/text_flow_bridge.rs:562`, `src/layout/engine/text_flow_bridge.rs:581` | engine cache 比较完整 identity；split combining/ZWJ 保留 first-source color 并发 diagnostic | 只作为 no-write regression gate |
| source-map property | `tests/property_tests.rs:48`, `tests/property_tests.rs:65`, `tests/property_tests.rs:93` | property 验证 source EGC ranges 与 position map total/round-trip | 4096 cases 必须保持，不修改该文件 |
| merged #128 | `src/layout/text_flow/truncate.rs`, `tests/text_flow_truncate_regressions.rs:34`, `tests/text_flow_truncate_regressions.rs:102` | current main 已有 tab-aware truncation 与独立 linear operation tests | GH-127 不修改 truncate paths，final gate 全量运行现有 fixture |
| merged #129/#130 | `src/layout/engine/text_flow_bridge.rs:601`, `src/layout/engine/context_sync/tests.rs:10`, `src/layout/engine/context_sync/tests.rs:179` | current main 已验证 detached-flow purge、unchanged Arc reuse 与精确 dirty path | GH-127 不修改 engine paths，final gate保留三项 exact contract |
| merged #126 | PR [#136](https://github.com/majiayu000/rnk/pull/136), merge `50f6a203c1861814d288d4bdeae0e28d877af34c` | current main 已包含 prompt interruption polling，并建立 `wrap.rs` 与 `tests/text_flow_wrap_interruption.rs` 的行为合同 | implementation head 必须包含该 merge；只运行其真实 contract，不改其路径或固定旧 callback count |
| review evidence | PR [#109 discussion](https://github.com/majiayu000/rnk/pull/109#discussion_r3651392332) | exact head `67ca427986a5e747e6799cd111cb874c5200cc75` 的 styled-boundary thread 仍 unresolved/non-outdated | 只有 GH-127 implementation exact-head gate 完成后才能由 human resolve |

## 设计方案

### 1. Spec、dependency 与 duplicate gate

当前 target repo 没有 `checks/route_gate.py`。本 spec-only 工作的 route 证据必须从固定
SpecRail revision 的隔离 mirror 生成；它不替代 human spec approval，也不授权
implementation。

implementation owner 开始前必须 fresh 完成：

1. GH-127 三文件 spec PR 已 merged，且有 human approval；issue 无 `parked` 或冲突 readiness。
2. search GitHub open/merged PR、remote/local branches 与 worktrees，确认只有一个 GH-127
   implementation owner。
3. implementation head 必须验证
   `50f6a203c1861814d288d4bdeae0e28d877af34c` 是 ancestor。
4. implementation base 必须包含 current main 的 #128 PR #134、#129 PR #135、#130
   PR #138 merge commits。
5. planned diff 必须是 manifest 五路径的非空子集；任何需要 `wrap.rs`、`truncate.rs`、engine、
   renderer、public exports、Cargo 或 workflow 的发现都停止并修订 specs。

### 2. Validation 产出 private normalized plan

把 `validate_styled_ranges(input) -> Result<(), TextFlowError>` 收敛为等价 private validation
结果，例如 `ValidatedStyledRanges<'a>`。名字是实现指引，不是 public API 承诺。结果至少保存：

- caller `styled_ranges` 的 immutable borrow；
- 每个 range 的 original input ordinal；
- existing validation 所需的 sorted non-empty range view；
- start/end 两个独立 endpoint event（包括 empty range 的相同 start/end）及 endpoint ordinal；
- normalization-only deterministic operation observer。

validation 行为顺序固定：

1. 仍先按 caller 顺序检查 `start > end`、`end > source.len()` 与 UTF-8 char boundaries，
   返回第一个 `InvalidStyleRange`。
2. 仅 non-empty ranges 参与 overlap check；按 start 的确定排序后返回第一对真实 overlap。
3. adjacent 与 empty ranges 成功进入 plan；不修改 input vector，不合并或去重。
4. existing validation/sort phase 与 normalization counter 分开记账。B-001/B-002 只证明 issue
   所指的 post-validation style/boundary normalization 不再 `G × R`；不得把 sort comparisons
   伪装成 normalization operations，也不得用此排除掩盖新的嵌套 scan。

`usize::MAX`、reverse range、source-end 之外和 multibyte scalar interior 均只经过 checked
比较与 `is_char_boundary`，不做 endpoint 加减，因此不产生 overflow/panic。

### 3. 两个 monotonic projections

production normalization 只消费 validation plan，并对 source grapheme 单次前进：

- **Style cursor**：在 sorted non-empty ranges 上单调推进已结束 range；当前唯一 candidate
  满足 `range.start <= grapheme.start < range.end` 时 clone 其 exact Style，否则 clone
  `default_style`。不扫描已消费或未来 ranges。
- **Boundary cursor**：对 sorted endpoint events 单调分配严格位于
  `grapheme.start < boundary < grapheme.end` 的 events。每个 event 携带 original range/endpoint
  ordinal；输出前按 `grapheme ordinal -> original range ordinal -> start/end ordinal`
  投影，保持当前 diagnostic 顺序与重复项。

允许等价的 linear merge/bucket 设计，但必须同时满足：

- post-validation 每个 grapheme、range cursor advance 与 endpoint event 访问次数为常数；
- 没有 per-grapheme `.find()`、全 endpoint `.filter()`、binary search per event 或
  per-grapheme sort；
- memory 上界 `O(G + R)`，所有 allocation failure/checked arithmetic 沿现有 fail-loud
  boundary 退出；
- empty range 两个 endpoint 与 adjacent ranges 的共享 endpoint 都是不同 event；
- diagnostics 只含严格内部 boundary，exact `grapheme_range` 不重建。

### 4. Deterministic operation counter

在 private `src/layout/text_flow/style_normalization.rs` 提供 test-only observer/seam；
`text_flow.rs` 只负责 private module wiring 与现有 build integration。不得 public
re-export、写入 `TextFlow`、改变 cache identity 或在 release production path 保存全局
状态。observer 对以下 production normalization actions 各计一次：

1. 取得一个 source grapheme；
2. validation plan 物化/访问一个 endpoint；
3. cursor 前进；breakdown 必须分别记录 style-range advance 与 boundary-endpoint visit；
4. 把一个已匹配 endpoint 投影成 ordered diagnostic event。

`src/layout/text_flow/tests/style_normalization.rs` 的
`styled_boundary_normalization_operation_count_is_linear` 真实调用同一 production
normalization，对 2k/4k/8k 的每个 fixture family 分别断言：

1. 一 non-empty range/一 ASCII EGC；
2. 许多 combining/ZWJ EGC，每个 EGC 含 strict-interior、adjacent/shared 与 empty
   endpoint events；
3. 一个 combining EGC 承载大量 strict-interior adjacent non-empty/empty events 的
   one-EGC skew。

```text
operations <= 12 * (G + R) + 64
next_operations <= 2 * previous_operations + 128
```

三类都使用同一 absolute/slope bound；后两类还断言 ordered projection count大于零且等于
fixture 的 exact expected event count，使 observer action 4 不能退化成未观测工作。同一 helper
在 debug 普通 test 与 release exact test 执行。另一个 exact negative
`styled_boundary_operation_bound_failure_reports_complete_diagnostics` 用 schema-valid
synthetic observed count 进入 bound validator，断言 failure 文本包含 size、`G`、`R`、
internal/projected event counts、action breakdown、observed、bound、previous density；
它不替代 production counter positive。

wall-clock benchmark 可以人工观察，但不得进入 pass/fail 或 PR completion evidence。

### 5. Public behavior integration fixture

新增 `tests/text_flow_style_normalization.rs`，只经 public
`TextFlow`/`TextFlowCache` API 验证：

- first-source style + split combining/ZWJ diagnostics；
- adjacent、内部 empty、合法未排序 ranges 的 exact diagnostic order/multiplicity；
- invalid/reverse/non-char/`usize::MAX` 与 overlap errors；
- source/token/run/map/diagnostic/cache identity 的 cold-vs-current oracle；
- range vector 顺序/style/endpoint变化触发 miss，完全相同 input/options Arc reuse；
- immediate interruption precedence、large valid ranges polling、failure 保留 previous
  public Arc/cache identity/完整 flow 语义、retry 等于 cold build。

fixture 不复制 production merge、不读取 private counter 或 private `build_count`、不使用
wall clock、不访问网络或真实 terminal。精确 `build_count` 原子性只在 crate unit
`styled_normalization_polling_and_cache_count_are_atomic` 中断言；不得为 integration test
新增 public accessor。内部 operation/counter test 与 public semantic fixture 是两份独立
证据。

### 6. Interruption 与 #126 ordering

`try_build_interruptible` 当前先 immediate poll，再 validation，再 tokenization。GH-127 保持：

- initial `true` 立即 `Interrupted`；
- initial `false` 后 invalid/overlap 仍先返回 typed validation error；
- validation 成功后，private plan construction/normalization 每个 bounded batch 或每个
  cursor event 使用现有 callback；
- cancellation 立即丢弃 candidate tokens/diagnostics，cache publish 仍只发生在完整 flow。

#126 PR #136 已合并；其 wrap collection/width/placement polling 合同由
`50f6a203c1861814d288d4bdeae0e28d877af34c` 固定。GH-127 不修改其两路径，也不把 wrap
callback counts 写进 B-002 counter。若 GH-127 在 pre-wrap 阶段增加合法 polls，只更新
自己 range-normalization fixture，不能修改 #126 assertions。发现两合同无法同时满足时
回到 spec review，而不是抢写 `wrap.rs`。

### 7. Compatibility 与 no-write boundaries

不新增/修改 public declarations。以下均为 regression-only：

- `TextFlowInput`/`StyledTextRange`/`TextFlowDiagnostic`/`TextFlowError`/cache identity；
- `src/layout/text_flow/wrap.rs`、`truncate.rs`；
- `src/layout/engine/**` 与 renderer；
- `tests/property_tests.rs`、`tests/text_flow_truncate_regressions.rs` 和 #126 test；
- Cargo manifests、exports、docs、workflows。

## Product-to-Test Mapping

下表中的 `planned:` 名称由本 implementation 创建；未标 `planned:` 的 test 已在写作 base
或 #126 merge `50f6a203c1861814d288d4bdeae0e28d877af34c` 实际存在。

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 post-validation `O(G+R)` | private `style_normalization.rs` validated plan + cursors | planned unit: `styled_boundary_normalization_operation_count_is_linear`，三类 2k/4k/8k 断言无 nested scan |
| B-002 absolute/slope counter | private production observer + unit gate | planned unit exact，同步断言 `12*(G+R)+64` 与 doubling `+128` |
| B-003 debug/release diagnostics | counter bound validator | planned unit: `styled_boundary_operation_bound_failure_reports_complete_diagnostics`; debug/release exact commands |
| B-004 first-source style | style cursor + public integration | planned integration: `public_styled_flow_preserves_first_source_style_and_diagnostics`; existing engine combining/ZWJ exact tests |
| B-005 strict internal diagnostics | boundary cursor | planned integration: `public_styled_flow_preserves_first_source_style_and_diagnostics`; existing `split_combining_and_zwj_style_boundary_normalizes` |
| B-006 order/multiplicity | original ordinal projection | planned unit: `style_boundary_event_order_and_multiplicity_are_stable`; planned integration: `public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges` |
| B-007 legal unsorted input | validation plan + public integration | planned integration: `public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges` |
| B-008 empty source/range | validation/diagnostic projection | planned integration: `public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges`; existing `text_flow_empty_inputs` |
| B-009 invalid/extreme typed errors | validation | planned unit: `styled_range_extremes_preserve_typed_errors`; planned integration: `public_styled_flow_preserves_typed_failures` |
| B-010 overlap typed errors | validation sorted view | planned integration: `public_styled_flow_preserves_typed_failures`，覆盖 overlap/adjacent/empty |
| B-011 no fallback/partial result | build/cache error boundary | planned integration: `public_styled_flow_failures_and_interruption_are_atomic` |
| B-012 complete flow equality | tokenization + public oracle | planned integration: `public_styled_flow_preserves_complete_flow_identity`; `PROPTEST_CASES=4096 cargo test --test property_tests --locked text_flow_logical_source_round_trip -- --exact` |
| B-013 exact cache identity | unchanged input clone | planned integration: `public_styled_flow_preserves_exact_cache_identity` |
| B-014 Arc reuse/miss | `TextFlowCache` regression | planned integration: `public_styled_flow_preserves_exact_cache_identity`; existing `text_flow_cache_invalidation`/`text_flow_cache_reuse` |
| B-015 precedence | build-entry/validation ordering | planned integration: `public_styled_flow_failure_precedence_is_stable`; existing `text_flow_immediate_interruption_precedes_empty_and_cache_results` |
| B-016 bounded cancellation | plan/cursor polling | planned unit: `styled_normalization_polling_and_cache_count_are_atomic`; planned integration: `public_styled_flow_failures_and_interruption_are_atomic` |
| B-017 atomic retry | cache publish boundary | planned unit: `styled_normalization_polling_and_cache_count_are_atomic` 精确断言 private build count；planned public integration: `public_styled_flow_retry_matches_cold_build` 与 L12 只证明 Arc/cache semantics |
| B-018 GH-58/public compatibility | unchanged public surface + regression ledger | existing styled/cache/engine/property exact commands in Verification Plan |
| B-019 #128/#129/#130 compatibility | no-write paths | `cargo test --test text_flow_truncate_regressions --locked`; exact engine bridge/context_sync tests |
| B-020 #126 ownership/order | merged dependency gate; no wrap diff | `git merge-base --is-ancestor 50f6a203c1861814d288d4bdeae0e28d877af34c HEAD`; no-write diff；`cargo test --test text_flow_wrap_interruption --locked` |
| B-021 exhaustive fixtures/real production path | unit + public integration | every planned exact test in Critical Test Ledger; source scan rejects copied merge in integration file |
| B-022 exact-head quality/evidence | coverage/full CI/review gate | fmt/check/clippy/all-targets/property/coverage + exact head CI/reviewThreads/independent review |

## 数据流

```text
TextFlowInput + TextFlowOptions + interruption callback
  -> immediate interruption/options checks
  -> typed styled-range validation + private sorted/original-ordinal plan
  -> one source-grapheme pass
       -> monotonic style cursor
       -> monotonic endpoint cursor
       -> exact ordered diagnostics
       -> canonical TextFlowToken
  -> existing wrap/truncate layout
  -> existing source coverage + position map
  -> completed immutable TextFlow
  -> exact-identity cache publish
```

输入/输出仅在内存中；没有持久化、网络、权限、provider 或 terminal I/O。counter 只存在于
test build，不进入 public result、cache 或 runtime telemetry。

## 备选方案

- **每 grapheme binary-search ranges/endpoints**：拒绝；是 `O(G log R)`，不满足线性合同，
  且重复 search 难以保持 diagnostic original order。
- **用 `HashMap<byte, Style>` 或逐 byte table**：拒绝；source byte 数不等于 grapheme 数，
  大 combining cluster 会放大内存，hash iteration 也不能承担 deterministic order。
- **先排序 caller vector 并去重 boundaries**：拒绝；改变 cache identity 与
  adjacent/empty diagnostic 重数。
- **只缓存上次 `.find()` 结果，仍全量扫描 diagnostics**：拒绝；只修一半根因。
- **criterion/wall-clock regression**：拒绝作为 gate；环境噪声不能证明算法阶数。
- **把 counter 暴露为 public API**：拒绝；性能测试 seam 不能污染稳定 surface。

## 风险

- **Security**：纯本地 indexes 不解释 source。极端 endpoint 使用 checked comparison；
  禁止 allocation size/operation arithmetic overflow、panic 或执行 source controls。
- **Compatibility**：内部排序最容易改变 diagnostics order/multiplicity 或 cache identity。
  original ordinal、complete-flow equality 与 adjacent/empty fixtures固定当前语义。
- **Performance**：existing validation sort 不属于本 issue 的 quadratic root cause，但
  counter 必须明确 phase，防止实现把 `G×R` 移到未计数 helper。source scan + source review
  同时检查所有 range traversals。
- **Cancellation**：新增 plan building 可能形成 range-only uninterruptible pass；B-016
  要求 bounded polling，同时 B-015 固定 typed validation precedence。
- **Maintenance**：private plan/counter若和 production 分叉会产生假 green。unit counter必须
  调同一 production helper，integration只验证 public结果；父 unit 文件必须通过自然拆分
  回到 800 行以内，禁止压缩断言或继续直接追加。
- **Dependency**：#126 已合并并固定 interruption poll density。exact ancestor + disjoint
  no-write diff避免 GH-127 改写其断言。

## Critical Test Ledger

| Ledger ID | Exact test/command | Ownership | Proves |
| --- | --- | --- | --- |
| GH127-L1 | `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact` | unit submodule | ASCII + high-density combining/ZWJ + one-EGC skew 2k/4k/8k production counter absolute+slope，含 ordered projection |
| GH127-L2 | `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_operation_bound_failure_reports_complete_diagnostics -- --exact` | unit submodule | fail-closed counter diagnostics |
| GH127-L3 | `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::style_boundary_event_order_and_multiplicity_are_stable -- --exact` | unit submodule | original order/duplicate endpoints |
| GH127-L4 | `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_range_extremes_preserve_typed_errors -- --exact` | unit submodule | reverse/non-char/`usize::MAX` typed errors |
| GH127-L5 | `cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_normalization_polling_and_cache_count_are_atomic -- --exact` | unit submodule | large range-only cancellation polling + exact private build count atomicity |
| GH127-L6 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_first_source_style_and_diagnostics -- --exact` | new integration | combining/ZWJ first-source behavior |
| GH127-L7 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges -- --exact` | new integration | adjacent/empty/unsorted exact semantics |
| GH127-L8 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_typed_failures -- --exact` | new integration | invalid/overlap/extreme errors |
| GH127-L9 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_complete_flow_identity -- --exact` | new integration | source/token/run/map/diagnostic equality |
| GH127-L10 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_exact_cache_identity -- --exact` | new integration | identity miss/reuse |
| GH127-L11 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_failure_precedence_is_stable -- --exact` | new integration | immediate interrupt vs validation |
| GH127-L12 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_failures_and_interruption_are_atomic -- --exact` | new integration | public Arc/cache identity/complete-flow atomicity；不读取 private build count |
| GH127-L13 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_retry_matches_cold_build -- --exact` | new integration | retry/idempotency |

Ledger set必须 exact 为 `GH127-L1` 至 `GH127-L13`；新增/删除 test 先更新 product mapping、
tech ledger 与 tasks Covers，不允许只在 final catch-all 中补名字。

## Verification Plan

### Dependency 与 scope

```sh
set -euo pipefail
BASE_SHA="$(git rev-parse "${BASE_SHA:?set BASE_SHA to the implementation PR base}^{commit}")"
HEAD_SHA="$(git rev-parse HEAD^{commit})"
test "$(git merge-base "$BASE_SHA" "$HEAD_SHA")" = "$BASE_SHA"
git merge-base --is-ancestor 50f6a203c1861814d288d4bdeae0e28d877af34c HEAD
ALLOWLIST_PATH="$(mktemp "${TMPDIR:-/tmp}/gh127-allowlist.XXXXXX")"
CHANGED_PATHS="$(mktemp "${TMPDIR:-/tmp}/gh127-changed.XXXXXX")"
UNEXPECTED_PATHS="$(mktemp "${TMPDIR:-/tmp}/gh127-unexpected.XXXXXX")"
NEGATIVE_PATHS="$(mktemp "${TMPDIR:-/tmp}/gh127-negative.XXXXXX")"
trap 'rm -f "$ALLOWLIST_PATH" "$CHANGED_PATHS" "$UNEXPECTED_PATHS" "$NEGATIVE_PATHS"' \
  EXIT HUP INT TERM
printf '%s\n' \
  src/layout/text_flow.rs \
  src/layout/text_flow/style_normalization.rs \
  src/layout/text_flow/tests.rs \
  src/layout/text_flow/tests/style_normalization.rs \
  tests/text_flow_style_normalization.rs |
  LC_ALL=C sort > "$ALLOWLIST_PATH"
git diff --name-only "$BASE_SHA...$HEAD_SHA" | LC_ALL=C sort > "$CHANGED_PATHS"
verify_changed_paths() {
  local candidate="$1"
  test -s "$candidate"
  comm -23 "$candidate" "$ALLOWLIST_PATH" > "$UNEXPECTED_PATHS"
  test ! -s "$UNEXPECTED_PATHS"
}
verify_changed_paths "$CHANGED_PATHS"
test -z "$(git diff --name-only --diff-filter=D "$BASE_SHA...$HEAD_SHA")"
for required in \
  src/layout/text_flow/style_normalization.rs \
  src/layout/text_flow/tests/style_normalization.rs \
  tests/text_flow_style_normalization.rs
do
  test -f "$required"
  grep -Fx "$required" "$CHANGED_PATHS"
done
cp "$CHANGED_PATHS" "$NEGATIVE_PATHS"
printf '%s\n' Cargo.toml >> "$NEGATIVE_PATHS"
LC_ALL=C sort -u -o "$NEGATIVE_PATHS" "$NEGATIVE_PATHS"
if verify_changed_paths "$NEGATIVE_PATHS"; then
  echo "negative allowlist fixture unexpectedly accepted Cargo.toml" >&2
  exit 1
fi
git diff --exit-code "$BASE_SHA...$HEAD_SHA" -- \
  src/layout/text_flow/wrap.rs \
  src/layout/text_flow/truncate.rs \
  src/layout/engine.rs \
  src/layout/engine \
  tests/property_tests.rs \
  tests/text_flow_truncate_regressions.rs
test "$(wc -l < src/layout/text_flow/tests.rs | tr -d ' ')" -le 800
```

`git diff --name-only` 的实现 diff 必须是 manifest 五路径的非空子集，且两个新私有
submodule 与 public integration file 必须存在。`text_flow.rs` 只承担 private module
wiring/build integration；`tests.rs` 只承担 module declaration、必要的 stable wrapper 与
自然拆分，禁止用压缩断言规避 800 行门。

### Exact-test discovery

先列出真实 harness inventory，再执行 ledger；任何 selector 为零或多于一个都 fail closed：

```sh
set -euo pipefail
LIB_INVENTORY="$(mktemp "${TMPDIR:-/tmp}/gh127-lib-inventory.XXXXXX")"
INTEGRATION_INVENTORY="$(mktemp "${TMPDIR:-/tmp}/gh127-integration-inventory.XXXXXX")"
RESULT_PATH="$(mktemp "${TMPDIR:-/tmp}/gh127-test-result.XXXXXX")"
IGNORED_INVENTORY="$(mktemp "${TMPDIR:-/tmp}/gh127-ignored-inventory.XXXXXX")"
IGNORED_RESULT="$(mktemp "${TMPDIR:-/tmp}/gh127-ignored-result.XXXXXX")"
trap 'rm -f "$LIB_INVENTORY" "$INTEGRATION_INVENTORY" "$RESULT_PATH" \
  "$IGNORED_INVENTORY" "$IGNORED_RESULT"' EXIT HUP INT TERM
cargo test --workspace --lib --locked -- --list > "$LIB_INVENTORY"
cargo test --test text_flow_style_normalization --locked -- --list \
  > "$INTEGRATION_INVENTORY"
verify_test_result() {
  local inventory="$1"
  local selector="$2"
  local result="$3"
  local matched passed ignored
  matched="$(awk -v expected="$selector: test" \
    '$0 == expected { n += 1 } END { print n + 0 }' "$inventory")"
  passed="$(sed -nE \
    's/^test result:.* ([0-9]+) passed; [0-9]+ failed; [0-9]+ ignored;.*/\1/p' \
    "$result" | awk '{ n += $1 } END { print n + 0 }')"
  ignored="$(sed -nE \
    's/^test result:.* [0-9]+ passed; [0-9]+ failed; ([0-9]+) ignored;.*/\1/p' \
    "$result" | awk '{ n += $1 } END { print n + 0 }')"
  test "$matched" -eq 1
  test "$passed" -eq 1
  test "$ignored" -eq 0
}
for selector in \
  layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear \
  layout::text_flow::tests::style_normalization::styled_boundary_operation_bound_failure_reports_complete_diagnostics \
  layout::text_flow::tests::style_normalization::style_boundary_event_order_and_multiplicity_are_stable \
  layout::text_flow::tests::style_normalization::styled_range_extremes_preserve_typed_errors \
  layout::text_flow::tests::style_normalization::styled_normalization_polling_and_cache_count_are_atomic
do
  cargo test --workspace --lib --locked "$selector" -- --exact > "$RESULT_PATH" 2>&1
  verify_test_result "$LIB_INVENTORY" "$selector" "$RESULT_PATH"
done
for selector in \
  public_styled_flow_preserves_first_source_style_and_diagnostics \
  public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges \
  public_styled_flow_preserves_typed_failures \
  public_styled_flow_preserves_complete_flow_identity \
  public_styled_flow_preserves_exact_cache_identity \
  public_styled_flow_failure_precedence_is_stable \
  public_styled_flow_failures_and_interruption_are_atomic \
  public_styled_flow_retry_matches_cold_build
do
  cargo test --test text_flow_style_normalization --locked "$selector" -- --exact \
    > "$RESULT_PATH" 2>&1
  verify_test_result "$INTEGRATION_INVENTORY" "$selector" "$RESULT_PATH"
done
printf '%s\n' 'ignored_fixture: test' > "$IGNORED_INVENTORY"
printf '%s\n' \
  'test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out' \
  > "$IGNORED_RESULT"
if verify_test_result "$IGNORED_INVENTORY" ignored_fixture "$IGNORED_RESULT"; then
  echo "negative ignored-test fixture unexpectedly passed" >&2
  exit 1
fi
```

### Focused 与 regression

```sh
set -euo pipefail
cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact
cargo test --release --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_normalization_operation_count_is_linear -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_boundary_operation_bound_failure_reports_complete_diagnostics -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::style_boundary_event_order_and_multiplicity_are_stable -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_range_extremes_preserve_typed_errors -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::style_normalization::styled_normalization_polling_and_cache_count_are_atomic -- --exact
cargo test --test text_flow_style_normalization --locked
cargo test --workspace --lib --locked layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::text_flow_cache_invalidation -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::text_flow_cache_reuse -- --exact
cargo test --workspace --lib --locked layout::engine::text_flow_bridge::tests::split_combining_span_boundary_preserves_first_source_style -- --exact
cargo test --workspace --lib --locked layout::engine::text_flow_bridge::tests::split_zwj_span_boundary_preserves_first_source_style -- --exact
PROPTEST_CASES=4096 cargo test --test property_tests --locked text_flow_logical_source_round_trip -- --exact
cargo test --test text_flow_truncate_regressions --locked
cargo test --workspace --lib --locked layout::engine::text_flow_bridge::tests::replace_and_reorder_preserve_only_live_flows -- --exact
cargo test --workspace --lib --locked layout::engine::context_sync::tests::identical_context_sync_keeps_text_leaf_and_root_clean_and_reuses_flow -- --exact
cargo test --workspace --lib --locked layout::engine::context_sync::tests::source_style_wrap_and_overflow_changes_dirty_only_the_affected_text_path -- --exact
cargo test --test text_flow_wrap_interruption --locked
```

### Full quality 与 coverage

```sh
set -euo pipefail
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
BASE_SHA="$(git rev-parse "${BASE_SHA:?set BASE_SHA to the implementation PR base}^{commit}")"
HEAD_SHA="$(git rev-parse HEAD^{commit})"
test "$(git rev-parse HEAD)" = "$HEAD_SHA"
test "$(git merge-base "$BASE_SHA" "$HEAD_SHA")" = "$BASE_SHA"
test -z "$(git status --porcelain --untracked-files=all)"
LCOV_PATH="$(mktemp "${TMPDIR:-/tmp}/gh127-${HEAD_SHA}.lcov.XXXXXX")"
PROVENANCE_PATH="$(mktemp "${TMPDIR:-/tmp}/gh127-${HEAD_SHA}.coverage.XXXXXX")"
EARLY_FAILURE_SENTINEL="$(mktemp "${TMPDIR:-/tmp}/gh127-early-failure.XXXXXX")"
rm -f "$EARLY_FAILURE_SENTINEL"
trap 'rm -f "$EARLY_FAILURE_SENTINEL"' EXIT HUP INT TERM
cargo llvm-cov clean --workspace
cargo llvm-cov --branch --workspace --lib --all-features --lcov \
  --output-path "$LCOV_PATH" --locked
test -s "$LCOV_PATH"
LCOV_SHA256="$(shasum -a 256 "$LCOV_PATH" | awk '{print $1}')"
python3 - "$BASE_SHA" "$HEAD_SHA" "$LCOV_PATH" "$LCOV_SHA256" \
  "$PROVENANCE_PATH" <<'PY'
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

base, head, lcov_path, expected_lcov_sha256, provenance_path = sys.argv[1:]
production = (
    "src/layout/text_flow.rs",
    "src/layout/text_flow/style_normalization.rs",
)
root = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
).resolve(strict=True)


def resolve(*args):
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def checksum(payload, expected):
    actual = hashlib.sha256(payload).hexdigest()
    if actual != expected:
        raise ValueError("raw LCOV checksum mismatch")
    return actual


def parse_lcov(text, expected_paths):
    records = {}
    record = None
    for line in text.splitlines():
        if line.startswith("SF:"):
            if record is not None:
                raise ValueError("nested SF record")
            raw = line[3:]
            source = Path(raw)
            if not source.is_absolute():
                raise ValueError(f"non-absolute/suffix SF path: {raw}")
            try:
                canonical = source.resolve(strict=True)
                relative = canonical.relative_to(root).as_posix()
            except (FileNotFoundError, ValueError) as error:
                raise ValueError(f"outside or nonexistent SF path: {raw}") from error
            if raw != str(canonical):
                raise ValueError(f"non-canonical SF path: {raw}")
            if canonical not in expected_paths:
                raise ValueError(f"unexpected SF record: {relative}")
            if relative in records:
                raise ValueError(f"duplicate SF record: {relative}")
            record = {
                "path": relative,
                "lines": {},
                "branches": {},
                "lf": None,
                "lh": None,
                "brf": None,
                "brh": None,
            }
        elif line == "end_of_record":
            if record is None:
                raise ValueError("end_of_record without SF")
            lines = record["lines"]
            branches = record["branches"]
            if not lines:
                raise ValueError(f"empty/deleted DA data: {record['path']}")
            if record["lf"] != len(lines):
                raise ValueError(f"LF/DA mismatch: {record['path']}")
            if record["lh"] != sum(value > 0 for value in lines.values()):
                raise ValueError(f"LH/DA mismatch: {record['path']}")
            if record["brf"] != len(branches):
                raise ValueError(f"BRF/BRDA mismatch: {record['path']}")
            branch_hits = sum(value not in (None, 0) for value in branches.values())
            if record["brh"] != branch_hits:
                raise ValueError(f"BRH/BRDA mismatch: {record['path']}")
            records[record["path"]] = record
            record = None
        elif record is not None and line.startswith("DA:"):
            number, hits, *_ = line[3:].split(",")
            number = int(number)
            if number in record["lines"]:
                raise ValueError(f"duplicate DA line: {record['path']}:{number}")
            record["lines"][number] = int(hits)
        elif record is not None and line.startswith("BRDA:"):
            number, block, branch, taken = line[5:].split(",")
            key = (int(number), block, branch)
            if key in record["branches"]:
                raise ValueError(f"duplicate BRDA entry: {record['path']}:{key}")
            record["branches"][key] = None if taken == "-" else int(taken)
        elif record is not None and line.startswith(("LF:", "LH:", "BRF:", "BRH:")):
            key, value = line.split(":", 1)
            field = key.lower()
            if record[field] is not None:
                raise ValueError(f"duplicate {key} summary: {record['path']}")
            record[field] = int(value)
    if record is not None:
        raise ValueError("unterminated SF record")
    return records


def expect_failure(name, action):
    try:
        action()
    except (ValueError, OSError):
        return
    raise SystemExit(f"negative verifier fixture unexpectedly passed: {name}")


tracked = subprocess.run(
    ["git", "ls-files", "-z", "--", "*.rs"],
    check=True,
    capture_output=True,
).stdout.split(b"\0")
expected_paths = {
    (root / raw.decode()).resolve(strict=True)
    for raw in tracked
    if raw
}
fixture_source = (root / production[0]).resolve(strict=True)
fixture = (
    f"SF:{fixture_source}\n"
    "DA:1,1\nLF:1\nLH:1\n"
    "BRDA:1,0,0,1\nBRF:1\nBRH:1\nend_of_record\n"
)
parse_lcov(fixture, expected_paths)
expect_failure(
    "empty DA",
    lambda: parse_lcov(
        fixture.replace("DA:1,1\nLF:1\nLH:1\n", "LF:0\nLH:0\n"),
        expected_paths,
    ),
)
expect_failure(
    "deleted DA",
    lambda: parse_lcov(fixture.replace("DA:1,1\n", ""), expected_paths),
)
expect_failure(
    "inconsistent LF/LH summary",
    lambda: parse_lcov(fixture.replace("LF:1", "LF:2"), expected_paths),
)
expect_failure(
    "inconsistent BRF/BRH summary",
    lambda: parse_lcov(fixture.replace("BRH:1", "BRH:0"), expected_paths),
)
expect_failure(
    "suffix SF",
    lambda: parse_lcov(fixture.replace(str(fixture_source), production[0]), expected_paths),
)
expect_failure(
    "outside SF",
    lambda: parse_lcov(fixture.replace(str(fixture_source), str(Path(lcov_path).resolve())), expected_paths),
)
expect_failure(
    "unexpected SF",
    lambda: parse_lcov(fixture.replace(str(fixture_source), str((root / "Cargo.toml").resolve(strict=True))), expected_paths),
)
expect_failure(
    "duplicate SF",
    lambda: parse_lcov(fixture + fixture, expected_paths),
)
expect_failure(
    "bad raw hash",
    lambda: checksum(b"negative hash fixture", "0" * 64),
)
if resolve("rev-parse", "HEAD") != head:
    raise SystemExit("stale coverage: current HEAD changed")
if resolve("merge-base", base, head) != base:
    raise SystemExit("coverage base is not the exact merge-base ancestor")
lcov_bytes = Path(lcov_path).read_bytes()
if not lcov_bytes:
    raise SystemExit("empty raw LCOV artifact")
actual_lcov_sha256 = checksum(lcov_bytes, expected_lcov_sha256)
diff = subprocess.run(
    ["git", "diff", "--unified=0", f"{base}...{head}", "--", *production],
    check=True,
    capture_output=True,
    text=True,
).stdout
changed = {path: set() for path in production}
current = None
for line in diff.splitlines():
    if line.startswith("+++ b/"):
        current = line[6:]
    elif current in changed and line.startswith("@@"):
        match = re.search(r"\+(\d+)(?:,(\d+))?", line)
        if not match:
            raise SystemExit("missing diff hunk coordinates")
        start, count = int(match.group(1)), int(match.group(2) or 1)
        changed[current].update(range(start, start + count))

records = parse_lcov(lcov_bytes.decode(), expected_paths)

missing_records = sorted(set(production) - set(records))
if missing_records:
    raise SystemExit(f"missing planned production LCOV records: {missing_records}")
critical = "src/layout/text_flow/style_normalization.rs"
critical_lines = records[critical]["lines"]
critical_branches = records[critical]["branches"].values()
if any(hits == 0 for hits in critical_lines.values()):
    raise SystemExit("critical normalization executable line coverage is not 100%")
if not records[critical]["branches"] or any(
    taken in (None, 0) for taken in critical_branches
):
    raise SystemExit("critical normalization executable branch coverage is not 100%")

changed_executable = []
for path in production:
    intersection = changed[path] & set(records[path]["lines"])
    if not intersection:
        raise SystemExit(f"zero changed executable lines in planned record: {path}")
    changed_executable.extend(records[path]["lines"][line] for line in intersection)
covered = sum(hits > 0 for hits in changed_executable)
if covered * 100 < len(changed_executable) * 80:
    raise SystemExit(
        f"changed production line coverage below 80%: "
        f"{covered}/{len(changed_executable)}"
    )
provenance = {
    "schema_version": 1,
    "base_sha": base,
    "head_sha": head,
    "merge_base_sha": base,
    "lcov_path": str(Path(lcov_path).resolve()),
    "lcov_sha256": actual_lcov_sha256,
    "production_records": list(production),
    "changed_executable_lines": len(changed_executable),
    "covered_changed_executable_lines": covered,
    "critical_executable_lines": len(critical_lines),
    "covered_critical_executable_lines": len(critical_lines),
    "critical_executable_branches": len(records[critical]["branches"]),
    "covered_critical_executable_branches": len(records[critical]["branches"]),
}
Path(provenance_path).write_text(
    json.dumps(provenance, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
print(json.dumps(provenance, sort_keys=True))
PY
test "$(git rev-parse HEAD)" = "$HEAD_SHA"
test "$(shasum -a 256 "$LCOV_PATH" | awk '{print $1}')" = "$LCOV_SHA256"
test -s "$PROVENANCE_PATH"
test -z "$(git status --porcelain --untracked-files=all)"
if bash -c 'set -euo pipefail; false; touch "$1"' _ "$EARLY_FAILURE_SENTINEL"; then
  echo "negative early-failure fixture unexpectedly returned zero" >&2
  exit 1
fi
test ! -e "$EARLY_FAILURE_SENTINEL"
```

coverage evidence 必须绑定 implementation PR base/head merge-base：所有 GH-127 changed
production lines 合计 >=80%，private normalization module 的全部可执行 line/branch 各为
100%。上述 verifier 只接受 repo root 下 canonical、tracked Rust source 的 exact `SF:`
路径，拒绝 suffix/outside/unexpected/duplicate record，并交叉校验 `LF/LH` 与 `DA`、
`BRF/BRH` 与 `BRDA`。两个 planned production file 任一缺失 record 或 changed-executable
交集为空、DA 空/删除、summary 不一致、critical module 零 branch、stale head、dirty
tracked/untracked worktree、错误 merge-base/raw checksum，或 shell early failure 被吞掉均
失败。review evidence 必须同时保存 `LCOV_PATH` raw artifact 与
`PROVENANCE_PATH` JSON。

### SpecRail 与 review

- target repo 明确不存在 `checks/route_gate.py`；不得伪装成本地 route gate。以下命令从
  target clean checkout 执行，只从 immutable
  `23caa70e76904eaa82323208d645d5781a365649` archive 建 mirror，验证两个 checker
  SHA-256 与 GH127/GH58 byte-identical inputs，并 fail closed：

```sh
set -euo pipefail
: "${SPEC_RAIL_ROOT:?set SPEC_RAIL_ROOT to the external SpecRail checkout}"
SPEC_RAIL_REV=23caa70e76904eaa82323208d645d5781a365649
CHECK_WORKFLOW_SHA256=8c791545f78d93649385ef0f9780454a7d4552f8da06da1fdee0de9cb8030a7e
ROUTE_GATE_SHA256=56954390bc5f9733601d94b5d18f78a7d5179c07fc47cd6dd8e8135685c8ac4a
git -C "$SPEC_RAIL_ROOT" rev-parse --is-inside-work-tree
git -C "$SPEC_RAIL_ROOT" cat-file -e "$SPEC_RAIL_REV^{commit}"
test "$(git -C "$SPEC_RAIL_ROOT" rev-parse "$SPEC_RAIL_REV^{commit}")" = "$SPEC_RAIL_REV"
test -z "$(git status --porcelain --untracked-files=all -- specs/GH127 specs/GH58)"
test ! -e "$PWD/checks/route_gate.py"
SPEC_RAIL_MIRROR="$(mktemp -d "${TMPDIR:-/tmp}/gh127-specrail.XXXXXX")"
trap 'rm -rf "$SPEC_RAIL_MIRROR"' EXIT HUP INT TERM
git -C "$SPEC_RAIL_ROOT" archive "$SPEC_RAIL_REV" | tar -x -C "$SPEC_RAIL_MIRROR"
mkdir -p "$SPEC_RAIL_MIRROR/specs"
cp -R specs/GH127 "$SPEC_RAIL_MIRROR/specs/GH127"
cp -R specs/GH58 "$SPEC_RAIL_MIRROR/specs/GH58"
test -f "$SPEC_RAIL_MIRROR/checks/check_workflow.py"
test -f "$SPEC_RAIL_MIRROR/checks/route_gate.py"
test "$(shasum -a 256 "$SPEC_RAIL_MIRROR/checks/check_workflow.py" | awk '{print $1}')" \
  = "$CHECK_WORKFLOW_SHA256"
test "$(shasum -a 256 "$SPEC_RAIL_MIRROR/checks/route_gate.py" | awk '{print $1}')" \
  = "$ROUTE_GATE_SHA256"
diff -qr specs/GH127 "$SPEC_RAIL_MIRROR/specs/GH127"
diff -qr specs/GH58 "$SPEC_RAIL_MIRROR/specs/GH58"
python3 "$SPEC_RAIL_MIRROR/checks/check_workflow.py" \
  --repo "$SPEC_RAIL_MIRROR" --spec-dir "$SPEC_RAIL_MIRROR/specs/GH127"
python3 "$SPEC_RAIL_MIRROR/checks/route_gate.py" \
  --repo "$SPEC_RAIL_MIRROR" --route implement --issue 127 \
  --state ready_to_implement --mode required --json
```

- GitHub checks 必须绑定 PR exact `headRefOid`，不是 merge-ref/stale run。
- independent reviewer 与 implementer 分离；human final review/merge gate保留。
- fresh GraphQL `reviewThreads` 覆盖 GH-127 PR，且 PR #109
  `discussion_r3651392332` 仅在实现证据通过后由 human 处理。

## 回滚方案

- merge 前：普通 commit 回退整个五路径 implementation diff；保留失败 counter、test 与
  review evidence，不改 test threshold。
- merge 后：用普通 revert 撤销 GH-127 implementation PR，不 force push，不回滚
  #126/#128/#129/#130。
- 如果 cursor 优化正确但 diagnostics/cache identity compatibility 失败，整体回退并修订
  product/tech；禁止只关掉 diagnostics、去重 empty boundaries 或 fallback default style。
- rollback 后 GH-127 保持 open/blocked，PR #109 thread保持 unresolved，直到新的 exact-head
  implementation 重新通过全部 gate。
