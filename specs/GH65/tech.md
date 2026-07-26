# Tech Spec：variable-height MessageList 与滚动锚定

## Linked Issue

GH-65: https://github.com/majiayu000/rnk/issues/65

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":65,"complete":true,"paths":["specs/GH65/product.md","specs/GH65/tech.md","specs/GH65/tasks.md","src/components/chat/mod.rs","src/components/chat/message_list.rs","src/components/chat/message_list/types.rs","src/components/chat/message_list/error.rs","src/components/chat/message_list/height_index.rs","src/components/chat/message_list/state.rs","src/components/chat/message_list/tests.rs","src/components/mod.rs","src/prelude.rs","tests/message_list_public_api.rs","tests/message_list_properties.rs","tests/message_list_render.rs","tests/virtual_scroll_compat.rs","benches/message_list.rs","Cargo.toml"],"spec_refs":["specs/GH65/product.md","specs/GH65/tech.md","specs/GH65/tasks.md","specs/GH57/product.md","specs/GH58/product.md","specs/GH60/product.md","specs/GH62/product.md","specs/GH63/product.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-65 的 row-index、measurement cache、anchor/follow state、visible slices、
render closure 和验证。GH-58 继续唯一拥有 TextFlow/Unicode width/wrapping；GH-60 继续拥有
transactional layout/render failure；GH-62 继续拥有 `MessageId`/`MessageRevision`；
GH-63 可在 closure 内渲染，但它的 view/block 类型不进入高度索引。

## Codebase Context

以下锚点在 `origin/main` `cc7ab1004f315ab8ac69aa10fd0ef7892be76862` 上核实。
当前 main 尚无 `src/components/chat`，且 GH-58/GH-60/GH-62 的生产实现尚未合并；开始
implementation 时必须从它们的真实 merged head 重新定位路径、类型和签名，不能把本文伪 API
当作已经存在。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Fixed virtual scroll | `src/components/layout/scrollable.rs:178` | `start=scroll_offset.min(items.len())`，`end=scroll_offset+viewport_height`，offset/height 实际为 item count | GH-65 新增 row-based API，不改这条兼容路径 |
| Generic scroll state | `src/hooks/use_scroll.rs:17`, `src/hooks/use_scroll.rs:140` | 保存 `offset_y/content_height/viewport_height`，无稳定 message identity、height index 或 anchor | 不足以表达 variable-height message list；不扩展为聊天专用 state |
| Legacy chat example | `examples/rnk_chat.rs:138` | `.skip(scroll_offset).take(12)` 按消息条数分页 | 证明问题存在；example 迁移不属于本 issue |
| Component exports | `src/components/mod.rs:12`, `src/components/mod.rs:67` | 无 `components::chat`；公开 fixed-height virtual scroll | implementation 需在 GH-62 merged module 上增加 MessageList exports 并保持旧导出 |
| Prelude exports | `src/prelude.rs:75` | 公开现有 component API，无 MessageList | 新 public surface 需要显式导出与 crate 外 compile fixture |
| TextFlow config/build | `src/layout/text_flow.rs:79`, `src/layout/text_flow.rs:200` | checked width/wrap/tab/overflow policy 与 typed build failure 已存在 | GH-58 merged API 是唯一 row measurement authority |
| Property/bench tooling | `Cargo.toml:62`, `Cargo.toml:66` | dev dependencies 已有 `proptest`、`divan`；bench 通过 explicit `[[bench]]` 注册 | property test 可直接复用；新增 message-list bench 需登记 Cargo.toml |
| Existing benchmark style | `benches/layout.rs:1` | 使用 divan benchmark entry/Bencher | 新 10k benchmark 遵循仓库约定 |
| GH-57 umbrella | `specs/GH57/product.md` | 要求 chat list 按 visual rows、保持 anchor/new-content 与性能预算 | GH-65 是其列表层实现合同 |
| GH-58 dependency | `specs/GH58/product.md` | 定义唯一 TextFlow row count/source mapping/resize/error | MessageList 只消费其 checked row count，不复制算法 |
| GH-60 dependency | `specs/GH60/product.md` | 定义 candidate/commit、required layout 与 typed failure | MessageList mutation/render 采用同一 fail-atomic 原则 |
| GH-62 dependency | `specs/GH62/product.md` | 定义 stable `MessageId`、nonzero `MessageRevision` 与 typed reducer | order entries 和 measure keys 必须复用这些真实类型 |
| GH-63 integration | `specs/GH63/product.md` | 定义纯 `ChatMessageView` 与 typed render trait/closure | 只在 MessageList render closure 内消费，不耦合 index |

## 设计方案

### 1. Implementation gate 与模块边界

Spec-only PR 可以在 dependencies 实现前 review/merge。生产实现开始前 coordinator 必须：

1. 从 fresh `origin/main` 建 implementation branch；
2. 用 GitHub 记录 GH-58、GH-60、GH-62 implementation PR 的 merged commit；
3. 验证三者都是 implementation base 的 ancestor；
4. 重新读取 merged `TextFlow`、layout error、`MessageId`/`MessageRevision` 和 chat module；
5. 对本 manifest 做 source-drift audit；路径/API 不一致时先更新并重新 review spec。

任一 required dependency 未 merge 即停止 implementation。GH-63 不阻塞 core index；若已合并，
integration 仍只能通过 render closure。实现不能从 open dependency branch、spec branch或
推测 API 开工。

模块计划：

```text
components::chat::message_list
├── types.rs         public validated value/config/output types
├── error.rs         closed typed state/measure/render error families
├── height_index.rs  private Fenwick prefix-row index + exact cache
├── state.rs         caller-owned order, atomic mutations, anchor/follow transitions
├── tests.rs         deterministic unit/operation-count fixtures
└── message_list.rs  public facade and typed render-closure adapter
```

每个 production 文件目标 200–400 行，任何文件不得超过 800 行。`height_index` 不 import
GH-63 view/block 类型；facade 只把 entry/slice 交给调用方。

### 2. Public types 与 closed errors

最终名称可为适配 merged dependencies 做机械调整，但语义和类型边界不得弱化：

```text
MessageRows(NonZeroU64)
RowOffset(u64)
ViewportRows(u64)
MessageVariantKey(u64)
MessageExpansionKey(u64)

MessageMeasureKey {
  message_id: MessageId,
  width: u16,
  content_revision: MessageRevision,
  variant: MessageVariantKey,
  expansion: MessageExpansionKey,
}

MessageListEntry {
  message_id: MessageId,
  content_revision: MessageRevision,
  variant: MessageVariantKey,
  expansion: MessageExpansionKey,
}

MessageAnchor { message_id: MessageId, intra_message_row: RowOffset }
BottomFollowState = Following | Paused { new_content_below: bool }

VisibleMessageSlice {
  message_id: MessageId,
  message_index: usize,
  message_rows: Range<RowOffset>,
  viewport_rows: Range<RowOffset>,
}

VisibleMessageRange {
  total_rows: u64,
  scroll_offset: RowOffset,
  slices: Vec<VisibleMessageSlice>,
}
```

`MessageRows::try_new(0)` typed 失败。所有 `u64` prefix arithmetic 使用 checked operations；
到 renderer `u16/usize` 的转换使用 `TryFrom` 并保留 coordinate overflow category，不截断。
公开 config/private state 字段不能依赖 caller struct literal 构造；构造器校验 cache capacity、
width 和初始 ID 唯一性。

Closed errors（不得 `Any`、catch-all/string-only variant）：

```text
MessageListStateError =
  DuplicateMessageId { message_id } |
  UnknownMessageId { message_id } |
  ZeroMessageRows { key } |
  MissingMeasurement { key } |
  StaleMeasurement { expected_revision, actual_revision, key } |
  RowArithmeticOverflow |
  CoordinateOverflow { value, target } |
  InvalidAnchor { message_id, intra_message_row } |
  InvalidCacheCapacity

MessageListMeasureError<E> =
  State(MessageListStateError) |
  MeasurementFailed { key, source: E } |
  Cancelled { key, source: E }

MessageListRenderError<E> =
  State(MessageListStateError) |
  RenderFailed { message_id, message_rows, source: E }
```

所有 error 实现 `Display`、`Error` 和适用的 `source()`；crate 外 fixture 对 closed state error
无 wildcard 穷举。若 merged cancellation 类型与 failure source 不同，使用两个 generic
参数或 repository 既有 typed cause，不能压成字符串。未知 ID、missing measurement 与 stale
result 保持不同 category。

### 3. Exact cache 与 prefix row index

State 内部持有：

```text
entries: Vec<MessageListEntry>
positions: HashMap<MessageId, usize>
rows: Vec<MessageRows>
prefix_rows: FenwickRows
measurements: BoundedMeasurementCache<MessageMeasureKey, MessageRows>
viewport: { width, rows, scroll_offset }
anchor: Option<MessageAnchor>
follow: BottomFollowState
revision: MessageListRevision
```

`MessageListRevision` 是本 state 的 nonzero checked generation，与 GH-62 content revision
分开。Cache 使用完整 `MessageMeasureKey: Eq + Hash`；hash 只找 bucket，命中后仍由 Eq 比较
全部五字段。Bounded eviction 使用确定性 caller-owned LRU（或仓库已有等价 deterministic
policy），capacity 由 validated config 提供；不能 process-global、随机淘汰或 background
静默修改。

Fenwick tree 保存每条消息 rows：

- `prefix_sum(i)` 得到 `[0, i)` 总行数；
- `lower_bound(row)` 找到包含 global row 的最小消息 index；
- `point_update(i, delta)` 更新单条高度；
- `total_rows()` checked 返回总高度。

非空 viewport 的有效 offset 为
`min(requested, total_rows.saturating_sub(viewport_rows))`。禁止把 offset 与 message index
混用。Lookup 和 point update 的 deterministic operation counter 必须不超过
`ceil(log2(max(n,1))) + 2`（若实现细节需要更小的固定常数，spec update 后再锁定）；每帧
不遍历 `[0, first_visible)`。

结构性 prepend/insert/delete/reorder 可从 `rows` 重建 Fenwick 与 positions，复杂度 `O(n)`；
exact cache 保留 unchanged keys。Resize 为每条 entry 请求新 width key，一次成功后重建；
同 width 重试命中 cache。Cache eviction 只影响未来 measurement，不改变 active `rows`。

### 4. Measurement contract 与原子 mutation

Public mutation facade：

```text
try_replace_all(entries, viewport, measure)
try_append(entries, measure)
try_prepend(entries, measure)
try_insert(index, entry, measure)
try_update(entry, measure)
try_remove(message_id)
try_resize(width, viewport_rows, measure)
try_set_viewport_rows(viewport_rows)
try_scroll_to(offset)
jump_to_bottom()
try_apply_measurement(expected_state_revision, key, rows)
visible_range()
```

`measure` 是 typed `FnMut(&MessageMeasureKey) -> Result<MessageRows, E>`（若 merged cancellation
有专用 result，则在不丢 source 的前提下适配）。MessageList 不传 message body `Any`；调用方
根据 stable ID/revision 取得 GH-62 message，并用 GH-58 构建 exact TextFlow row count。

所有 mutation 遵循：

```text
validate input/IDs/revision
  -> clone/stage affected order, positions and viewport inputs
  -> collect exact cache hits and required keys
  -> run all missing measurements into candidate cache
  -> validate nonzero rows and checked totals
  -> build/update candidate prefix index
  -> restore candidate anchor/follow state
  -> checked state revision +1
  -> commit all candidate fields once
```

任一步 Err/cancellation drop candidate，原 entries/positions/rows/index/cache/viewport/anchor/
follow/revision 逐字段不变。`try_apply_measurement` 同时核对 expected state revision、active
entry 的 exact key 和 result row；旧 revision 或旧 key typed 拒绝。成功但无 observable
变化的 scroll/resize/update 是 no-op 且 revision 不增加。

Content streaming、variant change、expand/collapse 都通过同 ID 的 `try_update`：任何 key
字段变化要求新 measurement，point update `O(log n)`；key 完全相等则禁止重新测量。

### 5. Visible slices

`visible_range()` 不测量、不修改 state：

1. 空列表或 viewport rows=0 返回空 slices；
2. 用 `lower_bound(scroll_offset)` 在 `O(log n)` 找首项；
3. 计算首项 `intra_start = scroll_offset - prefix_sum(index)`；
4. 依次生成与 `[scroll_offset, scroll_offset + viewport_rows)` 相交的 message-local 和
   viewport-local 半开范围；
5. 到 viewport end 或列表 end 停止。

示例：消息高度 `[3, 5, 2]`、offset `2`、viewport `6` 的 slices 为：

```text
id0 message_rows 2..3 -> viewport_rows 0..1
id1 message_rows 0..5 -> viewport_rows 1..6
```

不存在 active height/key 时返回 `MissingMeasurement`，不能跳过或按一行处理。每个 slice
验证 `start < end <= measured_rows`，总工作量 `O(log n + k)`。

### 6. Anchor、删除与 follow state transition

State 在每次成功 mutation 前从当前 visible range 捕获 top anchor：
`{top.message_id, top.message_rows.start}`。恢复规则按优先级：

1. `Following`：忽略旧 top anchor，commit 后 offset=新的 max offset，anchor 从新 viewport
   重新计算；
2. `Paused` 且 anchor ID 存在：恢复同 intra row；若消息缩短，clamp 到 `rows-1` 并在
   `MessageListUpdate.anchor_clamped=true`；
3. anchor 被删除：按 mutation 前 order 选择下一 surviving ID 的 row 0；无下一项则选上一
   surviving ID 的 `rows-1`；空列表为 None/offset 0；
4. global offset 为 `prefix_sum(anchor_index)+intra_row`，再做 checked/max-offset clamp；
   若 viewport 比 remaining content 高，viewport top 的物理 clamp 可使 anchor 位于 viewport
   内而非首行，update result 必须显式报告 `viewport_clamped=true`，不能假称 exact top。

Follow transition table：

| Input | Before | After |
| --- | --- | --- |
| explicit user scroll 到 `< max_offset` | 任意 | `Paused { existing flag }`，捕获 top anchor |
| explicit user scroll 到 `max_offset` | 任意 | `Following`，清 flag |
| jump-to-bottom | 任意 | `Following`，offset=max，清 flag |
| append/stream growth | Following | Following，offset 跟随新 max |
| append/stream growth below prior viewport | Paused | 保持 anchor，`new_content_below=true` |
| prepend/insert/update/resize/expand/collapse/delete | Paused | 保持/替代 anchor；不得自动 Following |
| mutation removes all content | Paused | Paused + None anchor；flag 保留到显式 jump/scroll-bottom |

“below prior viewport”以 mutation 前 viewport end 与变更 row interval 比较，不能用“是否最后一条
消息”猜测。`new_content_below` 一旦 true，在 Paused 内保持 true，直到显式回到底部。

### 7. Render closure 与 GH-63

Facade 只消费 immutable state/visible range：

```text
MessageList::new(&state)
  .try_into_element(|message_id, visible_slice| -> Result<Element, E> { ... })
  -> Result<Element, MessageListRenderError<E>>
```

closure 按 slices 顺序恰好调用一次，并收到 exact message-local partial range。调用方可在
closure 中读取 GH-62 message、建立 GH-63 `ChatMessageView` 并裁剪到该 partial range；
MessageList height index 不 import renderer 类型、不解释 block、不重算 TextFlow。

任何 closure failure 立即返回带 message ID/range/source 的 typed error，不返回已构造的
partial/default Element，也不修改 state。GH-60 merged API 若要求 frame transaction token，
facade 在候选 frame 中构造全部 children，全部成功后才交给 layout commit。

### 8. Compatibility 与 exports

- `src/components/layout/scrollable.rs` 和 `src/hooks/use_scroll.rs` 不在 planned changes；
  `virtual_scroll_view` 原签名、路径和 item-count 语义原样保留。
- `components::chat` 应由 GH-62 创建；GH-65 只增加 `message_list` module 与 concrete exports。
- `prelude` 只 re-export 明确 public types/functions，不使用 type alias 掩盖旧/新语义。
- crate 外 compile fixture 同时构造 fixed-height `virtual_scroll_view` 和新 MessageList，
  防止 API 被错误替换。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `types.rs`, `state.rs`, GH-62 types | `message_list_public_surface_is_typed` |
| B-002 | exact key/cache | `measurement_key_uses_all_identity_fields_and_exact_equality` |
| B-003 | caller-owned state/revision | `identical_inputs_produce_identical_state` |
| B-004 | constructors/visible range | `empty_zero_viewport_and_zero_width_contract` |
| B-005 | lower-bound/slice builder | `partial_first_and_last_message_ranges_are_row_exact` |
| B-006 | prepend restore | `prepend_preserves_top_message_and_intra_row` |
| B-007 | update/resize restore | `height_changes_preserve_or_report_anchor_clamp` |
| B-008 | remove transition | `deleted_anchor_selects_next_then_previous_survivor` |
| B-009 | follow transition | `follow_pause_and_explicit_resume_state_machine` |
| B-010 | append/stream transition | `append_and_stream_growth_follow_or_mark_new_content` |
| B-011 | invalidation/cache | `resize_variant_expansion_and_structure_cache_contract` |
| B-012 | staged commit | `measurement_failure_and_cancellation_are_atomic` |
| B-013 | `error.rs` | `closed_error_categories_are_exhaustive_and_keep_sources` |
| B-014 | state revision/delayed result | `stale_measurement_is_rejected_without_mutation` |
| B-015 | measure adapter | `message_height_comes_from_single_text_flow_result` |
| B-016 | module dependency/render facade | `message_list_render_closure_receives_exact_visible_slices` |
| B-017 | render facade | `render_failure_has_source_and_never_returns_partial_frame` |
| B-018 | Fenwick counter | `lookup_and_point_update_have_logarithmic_operation_bound` |
| B-019 | rebuild/cache counter | `structural_and_resize_costs_are_explicit_and_reuse_cache` |
| B-020 | proptest oracle | `variable_height_index_matches_naive_oracle` |
| B-021 | bench + counter | `cargo bench --bench message_list -- message_list_10k`; B-018 counter test |
| B-022 | unchanged fixed API | `fixed_height_virtual_scroll_api_is_unchanged` |
| B-023 | implementation preflight | dependency merged-ancestry commands in task gate |
| B-024 | closure audit | exact/full tests, coverage, CI/review/PR-gate evidence at current head |

每个 bare test name 都对应 tasks 中的完整 `cargo test ... -- --exact` 命令。Property test 使用
固定 32-byte ChaCha seed、至少 256 cases，并输出 seed/最小操作序列。Operation-count test 是
复杂度硬门禁；divan benchmark 不用 wall-clock threshold 造成 flaky CI。

## 数据流

```text
GH-62 ordered messages + viewport(width, rows)
  -> exact MessageMeasureKey per message
  -> caller measure closure
       -> GH-58 TextFlow::try_build
       -> checked row_count
  -> candidate exact cache + rows + Fenwick prefix index
  -> anchor/follow transition + checked state revision
  -> atomic MessageListState commit
  -> visible_range(offset, viewport rows)
  -> Vec<VisibleMessageSlice>
  -> typed render closure (可使用 GH-63)
  -> GH-60 candidate frame commit
```

无持久化、网络或 global mutable state。Provider/offline data 由调用方先写入 GH-62；MessageList
只处理 stable identity、measurement 与可见投影。

## 备选方案

- **每帧线性扫描高度：** 实现简单但 10k/high-frequency streaming 为 `O(n)`，拒绝。
- **固定估算高度后再修正：** 会产生 jump 且掩盖 missing measurement，违反原子/fail-loud，
  拒绝。
- **全局 cache：** 生命周期、并发与 eviction 不可预测，测试不确定，拒绝。
- **直接扩展 `virtual_scroll_view`：** 会改变既有 item-count 语义并强制迁移，拒绝。
- **把 GH-63 view 存进 index：** 将测量和 renderer 生命周期耦合，阻止其他 renderer，拒绝。
- **Segment tree：** 可满足复杂度，但这里只需 prefix sum/lower-bound/point update；Fenwick
  更小且足够。若 implementation evidence 显示 range update 必需，先更新 spec 再替换。

## 风险

- Security: 无 unsafe、网络、shell 或 secret；row/count 使用 checked arithmetic，error
  display 不输出消息正文。任何新增 unsafe 必须单独人工安全 review。
- Compatibility: 最大风险是误改 `virtual_scroll_view` 或现有 exports；独立 compatibility
  fixture 与 planned-path audit 阻止此事。
- Performance: 结构重建/resize 可 `O(n)`，但 lookup/point update 必须对数；固定 operation
  counter 是硬门禁，10k benchmark 记录趋势。
- Maintenance: Cache/anchor/follow 状态互相影响；candidate commit、closed transition table、
  naive property oracle 和分模块文件所有权降低漂移。
- Dependency drift: 当前 base 缺少三项 required implementation；必须执行 source-drift/
  ancestry gate，不能以本文伪签名强行适配。

## 测试计划

- [ ] Unit tests：exact key、partial slices、anchor/delete/follow transition、cache reuse、
  stale/overflow/missing/zero/failure/cancellation atomicity、operation count。
- [ ] Property tests：固定 seed 随机 operation 序列逐步对比独立 naive row-vector oracle。
- [ ] Integration tests：GH-58 measurement、GH-63-compatible render closure、typed render
  failure、crate 外 public errors/exports、fixed-height compatibility。
- [ ] Benchmark：divan 10k mixed heights 的 lookup/slice/point update/prepend，保存 current
  head 命令和输出。
- [ ] Full gates：`cargo fmt --all -- --check`、`cargo check --workspace --all-targets
  --all-features --locked`、`cargo test --workspace --all-targets --all-features --locked`、
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`。
- [ ] Coverage：新代码 line coverage 至少 80%；cache exactness、anchor/follow transition、
  candidate rollback、stale/error 关键路径 100%。
- [ ] Manual verification：无必需终端 UI 手测；example/adoption 不在 scope。审阅 benchmark
  trend、public API docs 与 PR current-head evidence。

## 回滚方案

通过普通 revert 移除 MessageList exports、modules、tests、bench 与 Cargo bench entry。不得
force push；不得回退 GH-58/GH-60/GH-62。因为旧 `virtual_scroll_view` 未修改，回滚不会要求
旧调用方迁移。若性能或 correctness gate 失败，在 merge 前保持 PR 未合并并修正根因，不以
默认高度、关闭 property case 或弱化断言作为回滚。
