# Tech Spec：线性 styled boundary 归一化

## Linked Issue

GH-127: https://github.com/majiayu000/rnk/issues/127

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":127,"complete":true,"paths":["src/layout/text_flow.rs","src/layout/text_flow/tests.rs","tests/text_flow_style_normalization.rs"],"spec_refs":["specs/GH127/product.md","specs/GH127/tech.md","specs/GH127/tasks.md","specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本 packet 只规划 GH-127。GH-58 是已存在的 TextFlow 行为合同；GH-101 只有 issue-native
workflow closure 要求，没有 `specs/GH101/`，因此不能伪造为 `spec_refs`。#126、#128、
#129、#130 同样没有 spec packet；它们通过 live issue/PR/merge dependency gate 与 exact
regression commands 验证。

## Codebase Context

以下锚点已在 clean base
`b4f39ed53506a42b7c06d0b0222bf3ac2c3e5ad8` 上通过 Read/`rg` 核实。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| styled input | `src/layout/text_flow.rs:40`, `src/layout/text_flow.rs:47`, `src/layout/text_flow.rs:68` | `StyledTextRange` 持有 public `Range<usize>` 与完整 `Style`；`TextFlowInput` 保存 caller 原始 vector | 优化只能建立 private view，不能排序写回或改变 public/cache identity |
| result identity | `src/layout/text_flow.rs:107`, `src/layout/text_flow.rs:113`, `src/layout/text_flow.rs:188` | cache identity clone 完整 input/options；flow 公开 tokens/rows/map/diagnostics | compatibility oracle 必须逐字段比较完整结果，不只比较 rendered text |
| build ordering | `src/layout/text_flow.rs:198`, `src/layout/text_flow.rs:211`, `src/layout/text_flow.rs:222` | immediate interruption 先于 option/range validation；成功 tokenization 后才 layout/map/publish | 新 preprocessing 不得改变 immediate interrupt、typed validation 与 atomic publish 顺序 |
| cache publication | `src/layout/text_flow.rs:279`, `src/layout/text_flow.rs:294`, `src/layout/text_flow.rs:303`, `src/layout/text_flow.rs:309` | cache 先比较 exact identity；completed flow 构建成功后才递增 count/publish Arc | interruption/error 必须保留上一 Arc 与 build count |
| typed errors | `src/layout/text_flow.rs:323`, `src/layout/text_flow.rs:347` | invalid、overlap、coverage、overflow、interrupted 是 closed `TextFlowError` | 不增加 string/boolean fallback，也不改变现有 variant payload |
| range validation | `src/layout/text_flow.rs:376`, `src/layout/text_flow.rs:389`, `src/layout/text_flow.rs:396` | 先按 caller 顺序检查 bounds/char boundary，再把 non-empty ranges 按 start 排序检查 overlap | private normalized plan 可复用这次排序；必须保留 first-invalid 与 overlap pair |
| quadratic normalization | `src/layout/text_flow.rs:407`, `src/layout/text_flow.rs:414`, `src/layout/text_flow.rs:420`, `src/layout/text_flow.rs:428` | 每个 grapheme 用 `.find()` 扫全部 ranges，再用 `flat_map(start,end)` 扫全部 endpoints | `G × R` 根因；GH-127 唯一生产修改点 |
| existing core tests | `src/layout/text_flow/tests.rs:120`, `src/layout/text_flow/tests.rs:227`, `src/layout/text_flow/tests.rs:272`, `src/layout/text_flow/tests.rs:311`, `src/layout/text_flow/tests.rs:663` | 已覆盖 cache、styled runs、split combining/ZWJ、部分 invalid 与 interruption | 扩展内部 counter/ordering/extreme tests，不削弱现有断言 |
| engine first-source contract | `src/layout/engine/text_flow_bridge.rs:350`, `src/layout/engine/text_flow_bridge.rs:562`, `src/layout/engine/text_flow_bridge.rs:581` | engine cache 比较完整 identity；split combining/ZWJ 保留 first-source color 并发 diagnostic | 只作为 no-write regression gate |
| source-map property | `tests/property_tests.rs:48`, `tests/property_tests.rs:65`, `tests/property_tests.rs:93` | property 验证 source EGC ranges 与 position map total/round-trip | 4096 cases 必须保持，不修改该文件 |
| merged #128 | `src/layout/text_flow/truncate.rs`, `tests/text_flow_truncate_regressions.rs:34`, `tests/text_flow_truncate_regressions.rs:102` | current main 已有 tab-aware truncation 与独立 linear operation tests | GH-127 不修改 truncate paths，final gate 全量运行现有 fixture |
| merged #129/#130 | `src/layout/engine/text_flow_bridge.rs:601`, `src/layout/engine/context_sync/tests.rs:10`, `src/layout/engine/context_sync/tests.rs:179` | current main 已验证 detached-flow purge、unchanged Arc reuse 与精确 dirty path | GH-127 不修改 engine paths，final gate保留三项 exact contract |
| active #126 owner | PR [#136](https://github.com/majiayu000/rnk/pull/136), head `eda58c2feb349d5aa4d8691186a35f25cffa76f8` | draft PR 独占 `src/layout/text_flow/wrap.rs`、`tests/text_flow_wrap_interruption.rs`；当前 check rollup green | implementation 必须等待 terminal；若 merged，retarget 后复用其真实 callback contract |
| review evidence | PR [#109 discussion](https://github.com/majiayu000/rnk/pull/109#discussion_r3651392332) | exact head `67ca427986a5e747e6799cd111cb874c5200cc75` 的 styled-boundary thread 仍 unresolved/non-outdated | 只有 GH-127 implementation exact-head gate 完成后才能由 human resolve |

## 设计方案

### 1. Spec、dependency 与 duplicate gate

当前 live issue 过早带 `ready_to_implement`，但 GH-127 packet 在本 PR 前不存在。orchestrator
已用 `current_state=ready_to_spec` 的 dry-run `write_spec` route gate 授权本 spec-only
工作；该 artifact 不替代 human spec approval，也不授权 implementation。

implementation owner 开始前必须 fresh 完成：

1. GH-127 三文件 spec PR 已 merged，且有 human approval；issue 无 `parked` 或冲突 readiness。
2. search GitHub open/merged PR、remote/local branches 与 worktrees，确认只有一个 GH-127
   implementation owner。
3. #126 PR #136 已 terminal。若 merged，记录 merge SHA 并验证是 implementation head
   ancestor；若 closed without merge，记录 human disposition，不能复制其未合入代码。
4. implementation base 必须包含 current main 的 #128 PR #134、#129 PR #135、#130
   PR #138 merge commits。
5. planned diff 必须是 manifest 三路径的子集；任何需要 `wrap.rs`、`truncate.rs`、engine、
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

在 `src/layout/text_flow.rs` 提供 private、test-only observer/seam；不得 public re-export、
写入 `TextFlow`、改变 cache identity 或在 release production path 保存全局状态。observer
对以下 production normalization actions各计一次：

1. 取得一个 source grapheme；
2. style cursor 前进一个 range；
3. boundary cursor 访问一个 endpoint；
4. 把已匹配 endpoint 投影到 ordered diagnostic bucket。

`src/layout/text_flow/tests.rs` 的
`styled_boundary_normalization_operation_count_is_linear` 真实调用同一 production
normalization，对 2k/4k/8k 一-range/一-ASCII-EGC fixture 断言：

```text
operations <= 12 * (G + R) + 64
next_operations <= 2 * previous_operations + 128
```

同一 helper 在 debug 普通 test 与 release exact test 执行。另一个 exact negative
`styled_boundary_operation_bound_failure_reports_complete_diagnostics` 用 schema-valid
synthetic observed count 进入 bound validator，断言 failure 文本包含 size、`G`、`R`、
observed、bound、previous density；它不替代 production counter positive。

wall-clock benchmark 可以人工观察，但不得进入 pass/fail 或 PR completion evidence。

### 5. Public behavior integration fixture

新增 `tests/text_flow_style_normalization.rs`，只经 public
`TextFlow`/`TextFlowCache` API 验证：

- first-source style + split combining/ZWJ diagnostics；
- adjacent、内部 empty、合法未排序 ranges 的 exact diagnostic order/multiplicity；
- invalid/reverse/non-char/`usize::MAX` 与 overlap errors；
- source/token/run/map/diagnostic/cache identity 的 cold-vs-current oracle；
- range vector 顺序/style/endpoint变化触发 miss，完全相同 input/options Arc reuse；
- immediate interruption precedence、large valid ranges polling、failure 保留 previous Arc/build
  count、retry 等于 cold build。

fixture 不复制 production merge，不读取 private counter，不使用 wall clock，不访问网络或
真实 terminal。内部 operation counter 和 public semantic fixture 是两份独立证据。

### 6. Interruption 与 #126 ordering

`try_build_interruptible` 当前先 immediate poll，再 validation，再 tokenization。GH-127 保持：

- initial `true` 立即 `Interrupted`；
- initial `false` 后 invalid/overlap 仍先返回 typed validation error；
- validation 成功后，private plan construction/normalization 每个 bounded batch 或每个
  cursor event 使用现有 callback；
- cancellation 立即丢弃 candidate tokens/diagnostics，cache publish 仍只发生在完整 flow。

#126 PR #136 owns wrap collection/width/placement polling。GH-127 不修改其两路径，也不把
wrap callback counts 写进 B-002 counter。若 #136 merged，GH-127 retarget 后运行其真实
四项 tests；如 callback count 因 GH-127 在 pre-wrap 阶段增加合法 polls，GH-127 必须只更新
自己 range-normalization fixture，不能修改 #126 assertions。发现两合同无法同时满足时回到
spec review，而不是抢写 `wrap.rs`。

### 7. Compatibility 与 no-write boundaries

不新增/修改 public declarations。以下均为 regression-only：

- `TextFlowInput`/`StyledTextRange`/`TextFlowDiagnostic`/`TextFlowError`/cache identity；
- `src/layout/text_flow/wrap.rs`、`truncate.rs`；
- `src/layout/engine/**` 与 renderer；
- `tests/property_tests.rs`、`tests/text_flow_truncate_regressions.rs` 和 #126 test；
- Cargo manifests、exports、docs、workflows。

## Product-to-Test Mapping

下表中的 `planned:` 名称由本 implementation 创建；未标 `planned:` 的 test 已在写作 base
或 PR #136 exact head 实际存在。

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 post-validation `O(G+R)` | `text_flow.rs` validated plan + cursors | planned unit: `styled_boundary_normalization_operation_count_is_linear`，2k/4k/8k 断言无 nested scan |
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
| B-016 bounded cancellation | plan/cursor polling | planned unit: `styled_normalization_polling_is_bounded`; planned integration: `public_styled_flow_failures_and_interruption_are_atomic` |
| B-017 atomic retry | cache publish boundary | planned integration: `public_styled_flow_retry_matches_cold_build`; existing `text_flow_interruption` |
| B-018 GH-58/public compatibility | unchanged public surface + regression ledger | existing styled/cache/engine/property exact commands in Verification Plan |
| B-019 #128/#129/#130 compatibility | no-write paths | `cargo test --test text_flow_truncate_regressions --locked`; exact engine bridge/context_sync tests |
| B-020 #126 ownership/order | dependency gate; no wrap diff | `git diff --exit-code \"$BASE_SHA...HEAD\" -- src/layout/text_flow/wrap.rs tests/text_flow_wrap_interruption.rs`; after merge `cargo test --test text_flow_wrap_interruption --locked` |
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
  调同一 production helper，integration只验证 public结果。
- **Dependency**：#126 正在修改 interruption poll density。硬 sequencing + disjoint
  no-write diff避免两个 PR 互改断言。

## Critical Test Ledger

| Ledger ID | Exact test/command | Ownership | Proves |
| --- | --- | --- | --- |
| GH127-L1 | `cargo test --workspace --lib --locked layout::text_flow::tests::styled_boundary_normalization_operation_count_is_linear -- --exact` | `src/layout/text_flow/tests.rs` | 2k/4k/8k production counter absolute+slope |
| GH127-L2 | `cargo test --workspace --lib --locked layout::text_flow::tests::styled_boundary_operation_bound_failure_reports_complete_diagnostics -- --exact` | `src/layout/text_flow/tests.rs` | fail-closed counter diagnostics |
| GH127-L3 | `cargo test --workspace --lib --locked layout::text_flow::tests::style_boundary_event_order_and_multiplicity_are_stable -- --exact` | `src/layout/text_flow/tests.rs` | original order/duplicate endpoints |
| GH127-L4 | `cargo test --workspace --lib --locked layout::text_flow::tests::styled_range_extremes_preserve_typed_errors -- --exact` | `src/layout/text_flow/tests.rs` | reverse/non-char/`usize::MAX` typed errors |
| GH127-L5 | `cargo test --workspace --lib --locked layout::text_flow::tests::styled_normalization_polling_is_bounded -- --exact` | `src/layout/text_flow/tests.rs` | large range-only cancellation polling |
| GH127-L6 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_first_source_style_and_diagnostics -- --exact` | new integration | combining/ZWJ first-source behavior |
| GH127-L7 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_adjacent_empty_and_unsorted_ranges -- --exact` | new integration | adjacent/empty/unsorted exact semantics |
| GH127-L8 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_typed_failures -- --exact` | new integration | invalid/overlap/extreme errors |
| GH127-L9 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_complete_flow_identity -- --exact` | new integration | source/token/run/map/diagnostic equality |
| GH127-L10 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_preserves_exact_cache_identity -- --exact` | new integration | identity miss/reuse |
| GH127-L11 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_failure_precedence_is_stable -- --exact` | new integration | immediate interrupt vs validation |
| GH127-L12 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_failures_and_interruption_are_atomic -- --exact` | new integration | no partial cache/result |
| GH127-L13 | `cargo test --test text_flow_style_normalization --locked public_styled_flow_retry_matches_cold_build -- --exact` | new integration | retry/idempotency |

Ledger set必须 exact 为 `GH127-L1` 至 `GH127-L13`；新增/删除 test 先更新 product mapping、
tech ledger 与 tasks Covers，不允许只在 final catch-all 中补名字。

## Verification Plan

### Dependency 与 scope

```sh
test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"
git diff --name-only "$BASE_SHA...HEAD"
git diff --exit-code "$BASE_SHA...HEAD" -- \
  src/layout/text_flow/wrap.rs \
  src/layout/text_flow/truncate.rs \
  src/layout/engine.rs \
  src/layout/engine \
  tests/property_tests.rs \
  tests/text_flow_truncate_regressions.rs
```

`git diff --name-only` 的实现 diff 必须是 manifest 三路径的非空子集，且
`tests/text_flow_style_normalization.rs` 必须存在。

### Focused 与 regression

```sh
cargo test --workspace --lib --locked layout::text_flow::tests::styled_boundary_normalization_operation_count_is_linear -- --exact
cargo test --release --workspace --lib --locked layout::text_flow::tests::styled_boundary_normalization_operation_count_is_linear -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::styled_boundary_operation_bound_failure_reports_complete_diagnostics -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::style_boundary_event_order_and_multiplicity_are_stable -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::styled_range_extremes_preserve_typed_errors -- --exact
cargo test --workspace --lib --locked layout::text_flow::tests::styled_normalization_polling_is_bounded -- --exact
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
```

若 #126 merged，再运行：

```sh
cargo test --test text_flow_wrap_interruption --locked
```

### Full quality 与 coverage

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
cargo llvm-cov --workspace --lib --all-features --lcov \
  --output-path /private/tmp/gh127-rust-lcov.info --locked
```

coverage evidence 必须绑定 implementation PR base/head merge-base：所有 GH-127 changed
production lines合计 >=80%，private validation/normalization/style cursor/boundary cursor/
operation observer 的可执行 line/branch 各为 100%。零 executable、missing file、stale head
或只上传未校验 artifact 均失败。

### SpecRail 与 review

- current implementation head 运行 pinned
  `python3 checks/check_workflow.py --repo . --spec-dir specs/GH127`。
- GitHub checks 必须绑定 PR exact `headRefOid`，不是 merge-ref/stale run。
- independent reviewer 与 implementer 分离；human final review/merge gate保留。
- fresh GraphQL `reviewThreads` 覆盖 GH-127 PR，且 PR #109
  `discussion_r3651392332` 仅在实现证据通过后由 human 处理。

## 回滚方案

- merge 前：普通 commit 回退整个三路径 implementation diff；保留失败 counter、test 与
  review evidence，不改 test threshold。
- merge 后：用普通 revert 撤销 GH-127 implementation PR，不 force push，不回滚
  #126/#128/#129/#130。
- 如果 cursor 优化正确但 diagnostics/cache identity compatibility 失败，整体回退并修订
  product/tech；禁止只关掉 diagnostics、去重 empty boundaries 或 fallback default style。
- rollback 后 GH-127 保持 open/blocked，PR #109 thread保持 unresolved，直到新的 exact-head
  implementation 重新通过全部 gate。
