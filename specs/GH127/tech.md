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

1. GH-127 三文件 spec PR 已 merged，且有 human approval。只有此条件满足后，maintainer
   才可把唯一 readiness 从 `ready_to_spec` 替换为 `ready_to_implement`；agent 不改 label。
2. 紧邻 route gate fresh 查询 live issue，把 labels 与 pinned `labels.yaml` 的 readiness
   取交集且要求恰好一项，并把该查询值传给 route gate；任何其他/冲突状态都保持 blocked。
3. search GitHub open/merged PR、remote/local branches 与 worktrees，确认只有一个 GH-127
   implementation owner。
4. implementation head 必须验证
   `50f6a203c1861814d288d4bdeae0e28d877af34c` 是 ancestor。
5. implementation base 必须包含 current main 的 #128 PR #134、#129 PR #135、#130
   PR #138 merge commits。
6. planned diff 必须是 manifest 五路径的非空子集；任何需要 `wrap.rs`、`truncate.rs`、engine、
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
| B-022 exact-head quality/evidence | coverage/full CI/review gate | fresh PR base/head before+after、canonical 1..EOF LCOV、fmt/check/clippy/all-targets/property/coverage + exact head CI/reviewThreads/review |

## 数据流

`TextFlowInput/options + interruption` → immediate checks → typed range validation/private
ordinal plan → 单次 grapheme pass（style cursor + endpoint cursor + ordered diagnostics）→
canonical tokens → existing wrap/truncate/map → completed immutable flow → exact cache publish。

输入/输出仅在内存中；没有持久化、网络、权限、provider 或 terminal I/O。counter 只存在于
test build，不进入 public result、cache 或 runtime telemetry。

## 备选方案

- 每 grapheme binary search 是 `O(G log R)`；逐 byte/hash table 放大 combining cluster
  内存且无稳定顺序；两者均不采用。
- 排序写回 caller vector/去重 boundary 会破坏 cache identity 与 diagnostic 重数；只缓存
  style `.find()` 又遗漏 diagnostic 根因，均不采用。
- wall-clock 不能证明阶数，counter 只保留 private test seam，不进入 public API。

## 风险

- **Security/compatibility**：checked endpoint/operation arithmetic；original ordinals、
  complete-flow equality 与 adjacent/empty fixtures固定顺序、重数和 cache identity。
- **Performance/cancellation**：counter phase 与 source review 防止 `G×R` 外移；plan/cursor
  bounded polling 同时保留 validation precedence。
- **Maintenance/dependency**：unit counter调用 production helper，integration只读 public
  结果；自然拆分父 test 文件；#126 exact ancestor + no-write diff 固定其 polling 合同。

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

### Closure trust boundary

完整可执行 closure 位于 `tasks.md` 的 `SP127-T4 exact-head closure reference`。该命令块是
本节的规范性实现；不得用手工抽样或 `--name-only` 替代。它先解析 absolute Python
interpreter、清除全部 Python startup/path 注入并统一使用 `-I -S`，同时设置
`GIT_NO_REPLACE_OBJECTS=1`。

每次 closure 开始必须：

1. 要求 caller `BASE_SHA` 与 local clean `HEAD`，fresh `git fetch --no-tags origin main`，
   读取 implementation PR 的 `baseRefOid/headRefOid`；
2. 证明 `BASE_SHA == FETCH_HEAD == baseRefOid ==
   merge-base(baseRefOid, headRefOid)`，且 `HEAD == headRefOid`；
3. 从 `BASE_SHA:specs/GH127/tech.md` 的 exact `100644 blob`读取唯一 planned manifest，
   使待审 head不能通过改自己 allowlist 来授权额外路径；
4. 用 `git diff --raw -z --no-renames` 逐 record验证 path、status、old/new mode；actual
   必须是五路径 manifest 的非空子集，status只允许 `A/M`，target mode只允许
   `100644/100755`，`M` 的 source mode也必须 regular。rename/copy会在禁用 rename
   detection 后含 `D`，和 delete/typechange/unmerged、duplicate/non-canonical/path escape
   一样 fail closed。三个 planned new files仍必须出现，父 unit file不得超过800行。

### Immutable source、coverage 与 provenance

closure 不在可变 implementation checkout 上收集 coverage。它读取 exact `headRefOid` 的
recursive tree entries，只接受 `100644/100755 blob`，对每个 blob通过 `git cat-file`
取 bytes并重算 Git OID。一个 fresh、worktree外、mode `0700` evidence root承载：

- descriptor-relative 安全物化的 source tree；固定 root dirfd 后，每层 parent 用
  `O_NOFOLLOW|O_DIRECTORY` 打开，目标以 `O_CREAT|O_EXCL|O_NOFOLLOW` 创建；
- 包含 path/mode/OID/line-count 的 exact tree manifest，以及它的 SHA-256；
- source tree 完成后改为只读；`CARGO_TARGET_DIR`、raw LCOV、test inventory/result、
  provenance与所有临时输出均在 source tree 外。

任一 symlink、non-directory parent、existing target、non-regular entry、OID mismatch、
absolute/`..`/non-canonical/duplicate path 均须在任何 Cargo 执行前失败。coverage verifier
只把 `SF:` 精确映射到物化 root + manifest tracked Rust path；EOF来自 exact Git blob
bytes，不重新打开可变 checkout source。它核对raw LCOV SHA-256、record uniqueness、
`DA/BRDA` 1-based范围、hits类型与非负性、`LF/LH/BRF/BRH` summaries、两个 production
record的changed-executable交集、changed production line >=80%和critical private module
可执行line/branch各100%。Cargo.toml raw diff、source/destination symlink、existing target、
suffix/outside/duplicate `SF:`、empty/deleted/line 0/超EOF/negative `DA`、invalid `BRDA`、
summary/hash和early-shell-failure fixtures均须证明 fail closed。

### Tests、SpecRail 与 final rebind

GH127-L1..L13先从真实 harness inventory证明各 selector恰好一个，再运行并解析为
`matched=1/passed=1/ignored=0`；debug/release counter、4096 property、#126/#128/#129/#130
regressions和full fmt/check/clippy/all-target/all-feature tests均从同一只读 exact source
执行。

target repo没有 `checks/route_gate.py`。fixed SpecRail revision
`23caa70e76904eaa82323208d645d5781a365649` 同样只能从exact regular blobs/OID通过上述
descriptor materializer建立只读external mirror；验证
`check_workflow.py=8c791545f78d93649385ef0f9780454a7d4552f8da06da1fdee0de9cb8030a7e`
和
`route_gate.py=56954390bc5f9733601d94b5d18f78a7d5179c07fc47cd6dd8e8135685c8ac4a`。
GH127/GH58 inputs来自 trusted `BASE_SHA` exact blobs，并证明implementation head对应
tree entries相同；不得 `cp` mutable checkout。执行checker时只显式把已验证mirror
`checks/`插入`sys.path`，入口也来自该mirror，ambient CWD、`PYTHONPATH`、user site和
`sitecustomize`均不在信任边界内。

所有测试和evidence完成后再次 fresh fetch remote main、再次读取PR base/head并重算
merge-base、raw diff digest和source manifest hash；它们必须逐项等于开始snapshot，
worktree仍clean，hosted checks/reviewThreads也必须绑定该exact head。任一main/PR/head/
tree/blob/diff漂移使全部证据失效并从头执行。GitHub checks不得使用merge-ref或stale run；
independent reviewer与implementer分离，human final review/merge gate保留；PR #109
`discussion_r3651392332`仅在实现证据通过后由human处理。

## 回滚方案

- merge 前：普通 commit 回退整个五路径 implementation diff；保留失败 counter、test 与
  review evidence，不改 test threshold。
- merge 后：用普通 revert 撤销 GH-127 implementation PR，不 force push，不回滚
  #126/#128/#129/#130。
- 如果 cursor 优化正确但 diagnostics/cache identity compatibility 失败，整体回退并修订
  product/tech；禁止只关掉 diagnostics、去重 empty boundaries 或 fallback default style。
- rollback 后 GH-127 保持 open/blocked，PR #109 thread保持 unresolved，直到新的 exact-head
  implementation 重新通过全部 gate。
