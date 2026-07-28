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

以下锚点在 2026-07-28 fresh `origin/main`
`27151646fa9b6713abfdec464d4877e17b3c9d7c`（PR #145 merge commit）核实。Fresh dependency
snapshot 是 GH-58 OPEN、GH-60 OPEN、GH-62 CLOSED、GH-63 OPEN；PR #145 只合并 GH-63
SP63-T1（head `1406b1d31f1f5186851e37f2de2a09e5722291a9`），其 message/block/cache 仍是 skeleton，
SP63-T2–T5 未交付。该 snapshot 只是 source-drift 证据，不替代 implementation gate；开始
implementation 时仍须从真实 final merged heads 重新定位路径、类型和签名。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Fixed virtual scroll | `src/components/layout/scrollable.rs:178` | `start=scroll_offset.min(items.len())`，`end=scroll_offset+viewport_height`，offset/height 实际为 item count | GH-65 新增 row-based API，不改这条兼容路径 |
| Generic scroll state | `src/hooks/use_scroll.rs:10`, `src/hooks/use_scroll.rs:140` | 保存 `offset_y/content_height/viewport_height`，无稳定 message identity、height index 或 anchor | 不足以表达 variable-height message list；不扩展为聊天专用 state |
| Legacy chat example | `examples/rnk_chat.rs:141` | `.skip(scroll_offset).take(12)` 按消息条数分页 | 证明问题存在；example 迁移不属于本 issue |
| Component exports | `src/components/mod.rs:4`, `src/components/chat/mod.rs:49`, `src/prelude.rs:55` | GH-62 chat 已公开；GH-63 T1 只公开 `view` contract/skeleton；无 MessageList | 增加 MessageList exports 并保持既有 chat/fixed-height 导出 |
| Prelude exports | `src/prelude.rs:55` | 公开现有 chat component API，无 MessageList | 新 public surface 需要显式导出与 crate 外 compile fixture |
| TextFlow config/build | `src/layout/text_flow.rs:79`, `src/layout/text_flow.rs:107`, `src/core/style.rs:271` | checked options/build 已存在；cache identity 仅 `PartialEq`，Style 含 `f32`；无 total-float key wrapper | GH-58 是唯一 row authority；GH-65 key 自建 bitwise-total snapshot |
| Property/bench tooling | `Cargo.toml:82-83` | dev dependencies 已有 `proptest`、`divan`；bench 通过 explicit `[[bench]]` 注册 | property test 可直接复用；新增 message-list bench 需登记 Cargo.toml |
| Existing benchmark style | `benches/layout.rs:1` | 使用 divan benchmark entry/Bencher | 新 10k benchmark 遵循仓库约定 |
| GH-57 umbrella | `specs/GH57/product.md` | 要求 chat list 按 visual rows、保持 anchor/new-content 与性能预算 | GH-65 是其列表层实现合同 |
| GH-58 dependency | `specs/GH58/product.md` | 定义唯一 TextFlow row count/source mapping/resize/error | MessageList 只消费其 checked row count，不复制算法 |
| GH-60 dependency | `specs/GH60/product.md` | 定义 candidate/commit、required layout 与 typed failure | MessageList mutation/render 采用同一 fail-atomic 原则 |
| GH-62 dependency | `specs/GH62/product.md` | 定义 stable `MessageId`、nonzero `MessageRevision` 与 typed reducer | order entries 和 measure keys 必须复用这些真实类型 |
| GH-63 integration | issue #63 + PR #145；`src/components/chat/view/` | #63 OPEN；#145 仅 T1 typed customization contract，default view/message/block/cache 未完成 | 当前使用通用 exact render closure；只在 #63 CLOSED 且完整 implementation ancestry 后调用完整 typed view |

## 设计方案

### 1. Implementation gate 与模块边界

Spec-only PR 可以在 dependencies 实现前 review/merge。生产实现开始前 coordinator 必须对
GH-58、GH-60、GH-62 分别生成一个 fail-closed `DependencyCompletionRecord`：

```text
DependencyCompletionRecord {
  issue: 58 | 60 | 62,
  state: CLOSED,
  closed_at,
  final_evidence_source,
  implementation_prs: non-empty ordered Vec<{
    number, exact_head_sha, merge_commit_sha, merged_at
  }>,
  final_pr_gate_head_sha,
  task_completion_evidence,
}
```

记录的唯一可信来源是该 dependency issue 的最终 closure evidence（issue 中明确链接的 final
SpecRail closure artifact、或人工确认的 complete implementation commit set）；普通 spec PR、
单个 partial/root-cause PR 或 coordinator 自选“看起来足够”的 merged PR 不得生成
`complete` record。Gate 固定执行：

1. fresh fetch `origin/main`，并证明 implementation branch 从该 exact SHA 创建；
2. fresh 查询每个 dependency issue，要求 `state=CLOSED` 和非空 `closed_at`；
3. 读取 final closure evidence，要求它枚举完整 implementation PR/commit set、覆盖 approved
   tasks，并使 `final_pr_gate_head_sha` 等于最后一个 completion PR 的 exact head；
4. 对 set 中每个 PR fresh 验证 `state=MERGED`、head/merge SHA/merged_at 与记录逐值一致；
5. 对每个 merge commit 执行
   `git merge-base --is-ancestor <merge_commit_sha> <implementation-base-sha>`；
6. 重新读取 merged `TextFlow`、layout error、`MessageId`/`MessageRevision` 和 chat module，
   再做 manifest source-drift audit。

任一 issue 仍 OPEN、final evidence 缺失/不完整、任一 commit 未包含、API/path 漂移或 GitHub
查询失败都停止 implementation。特别地，GH-58 仍 OPEN 时，已 merged 但只覆盖 root cause 的
PR #84 不能单独满足 gate。GH-63 不阻塞 core index；PR #145 仅是 T1 partial，不能把 skeleton
当作完整 view。只有 issue #63 CLOSED、final closure evidence 枚举的完整 implementation set
全部 MERGED 且 merge commits 都在 implementation base ancestry 中，closure 才可调用完整
typed view；否则一律使用 exact entry/key/slice 的通用 closure。实现不能从 open dependency
branch、spec branch 或推测 API 开工。

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
enum MessageRowsError { Zero }
RowOffset(u64)
ViewportRows(u64)
MessageListRevision(NonZeroU64) // INITIAL == 1
MessageVariantKey(u64)
MessageExpansionKey(u64)
MessageStructureSlotKey(u64)

MessageStructuralSegment {
  slot: MessageStructureSlotKey,
  rows: RowOffset,
}

MessageShellMeasureConfig {
  outer_width: u16,
  horizontal_insets: { left: u16, right: u16 },
  structural_segments: Vec<MessageStructuralSegment>,
}

MessageCompositeMeasureConfig {
  // 顺序与 renderer 中的 textual children 完全相同；每项是 GH-58 的完整
  // TextFlowInput + TextFlowOptions deep identity。
  text_flows: Vec<TextFlowCacheIdentity>,
  shell: MessageShellMeasureConfig,
}

MessageMeasureKey {
  message_id: MessageId,
  content_revision: MessageRevision,
  variant: MessageVariantKey,
  expansion: MessageExpansionKey,
  config: MessageCompositeMeasureConfig,
}

// private Arc field；公开只读借用，不公开 mutable key/config。
MessageMeasureKeyHandle(Arc<MessageMeasureKey>)

MessageListEntry {
  message_id: MessageId,
  content_revision: MessageRevision,
  variant: MessageVariantKey,
  expansion: MessageExpansionKey,
  measure_config: MessageCompositeMeasureConfig,
}

MessageAnchor { message_id: MessageId, intra_message_row: RowOffset }
enum BottomFollowState {
  Following,
  Paused { new_content_below: bool },
}

VisibleMessageSlice {
  message_id: MessageId,
  message_index: usize,
  measure_key: MessageMeasureKeyHandle,
  message_rows: Range<RowOffset>,
  viewport_rows: Range<RowOffset>,
}

VisibleMessageRange {
  total_rows: u64,
  scroll_offset: RowOffset,
  slices: Vec<VisibleMessageSlice>,
}

MessageListUpdate {
  previous_revision: MessageListRevision,
  applied_revision: MessageListRevision,
  anchor_clamped: bool,
  viewport_clamped: bool,
}

enum MessageListMutation {
  Applied(MessageListUpdate),
  NoChange { revision: MessageListRevision },
}

MessageListObservation {
  revision: MessageListRevision,
  follow_state: BottomFollowState,
  stored_anchor: Option<MessageAnchor>,
  new_content_below: bool,
}

MessageMeasureRequest<'a> {
  entry: &'a MessageListEntry,
  key: &'a MessageMeasureKey,
}

enum MessageMeasureOutcome<Failure, Cancellation> {
  Measured(MessageRows),
  Missing,
  Failed(Failure),
  Cancelled(Cancellation),
}

MessageResizeConfigRequest<'a> {
  message_index: usize,
  old_entry: &'a MessageListEntry,
  old_key: &'a MessageMeasureKey,
  new_width: u16,
  new_viewport_rows: ViewportRows,
}

enum MessageResizeConfigOutcome<Failure, Cancellation> {
  Rebuilt(MessageCompositeMeasureConfig),
  Failed(Failure),
  Cancelled(Cancellation),
}
```

`MessageRows::try_new(raw: u64) -> Result<MessageRows, MessageRowsError>` 在 `raw == 0` 时返回
不携带 measurement key 的 `MessageRowsError::Zero`。只有成功构造的 `MessageRows` 才能进入
`MessageMeasureOutcome::Measured`，因此 state callback boundary 不可能收到 raw zero。
所有 `u64` prefix arithmetic 使用 checked operations；
到 renderer `u16/usize` 的转换使用 `TryFrom` 并保留 coordinate overflow category，不截断。
公开 config/private state 字段不能依赖 caller struct literal 构造；构造器校验 cache capacity、
width、结构 segment rows 和初始 ID 唯一性。

`MessageCompositeMeasureConfig` 直接保存 GH-58 完整的 `TextFlowCacheIdentity` 值，不保存
hash-only digest 或 caller 自报的“style revision”。因此每个 textual child 的 exact source
bytes、structured style ranges/default style、content width、`TextWrap`、`overflow_x/y`、
tab stop、ellipsis、Unicode width policy/revision 全部参与 deep equality；shell 再保存
outer width、horizontal insets 与 role/code header、status、inter-block spacing、padding、
border 等有序 structural segments。

因为 current-main `TextFlowCacheIdentity -> TextFlowInput -> Style` 含 `f32` 且仓库没有可复用的
total-float key，`types.rs` 必须在 key constructor 内生成 private
`MessageMeasureKeyTotalSnapshot`。它保存 ID/revision/variant/expansion 并递归镜像完整
config；`StyleTotalSnapshot` 以不带 `..` 的 exhaustive destructuring 覆盖 `Style` 每个字段，把每个直接/`Option`/`Dimension`/`Edges`
中的 `f32` 转成 `TotalF32Bits(value.to_bits())`，其余字段和值逐项保留。新增 Style 字段因
exhaustive destructuring 必须先更新 snapshot 才能编译。相同 bits（含同 NaN payload/sign）
相等，不同 NaN payload/sign 不等，`+0.0`/`-0.0` 不等；因此所有 bit pattern 都自反且构造
确定。`MessageMeasureKey`/handle 的手写 `PartialEq` 与 `Eq` 只比较该 total snapshot，绝不
委托 `TextFlowCacheIdentity`/`Style` 的浮点 `PartialEq`；若用 hash 加速，hash 也从同一
snapshot 生成且命中后仍做 total equality。GH-58 final API 漂移时只能机械更新完整镜像，
不得缩成 opaque hash 或遗漏字段。

`MessageMeasureKeyHandle` 是拥有 private `Arc<MessageMeasureKey>` 的 concrete public value
type，不是 alias。`Clone` 只递增共享引用计数，固定 `O(1)`、不复制 source/style/config
vectors；`as_key(&self) -> &MessageMeasureKey` 仅提供共享只读借用，不能取得 mutable key。
active cache/index 与 slices 共享同一个 immutable allocation。该 concrete key/handle 必须满足
`Send + Sync + Clone + Eq` compile assertions；生命周期不借用临时 visible-range builder，
因此 `VisibleMessageRange` 可由调用方跨 frame 持有，但任一后续 mutation 只建立新 handle，
绝不原地修改旧 handle。Deep equality 只发生在 mutation/cache lookup、resize candidate
validation 和 render identity validation，不发生在逐帧 slice construction；no-op/invalidation/
resize/render 每处都比较派生的 total snapshot，不能退回含 NaN 时非自反的 config `==`。

Closed errors（不得 `Any`、catch-all/string-only variant）：

```text
enum MessageListStateError {
  DuplicateMessageId { message_id },
  UnknownMessageId { message_id },
  InvalidInsertIndex { index, len },
  MissingMeasurement { key },
  MissingActiveMeasurement { message_id },
  StaleStateRevision { expected, actual },
  StateRevisionOverflow { revision },
  MeasurementIdentityMismatch { entry, key },
  RowArithmeticOverflow,
  CoordinateOverflow { value, target },
  InvalidAnchorRow { message_id, requested, measured_rows },
  InvalidResizeConfig { message_index, message_id, new_width },
  InvalidViewportWidth { width },
  InvalidCacheCapacity,
}

enum MessageCompositeMeasureError<TextFlowFailure> {
  TextFlowFailed { child_index, source: TextFlowFailure },
  InvalidCompositeConfig { child_index },
  RowArithmeticOverflow,
  MessageRows(MessageRowsError),
}

enum MessageListMeasureError<Failure, Cancellation> {
  State(MessageListStateError),
  ConfigRebuildFailed { message_index, message_id, source: Failure },
  ConfigRebuildCancelled { message_index, message_id, source: Cancellation },
  MeasurementFailed { key, source: Failure },
  Cancelled { key, source: Cancellation },
}

enum MessageListRenderError<RenderFailure> {
  State(MessageListStateError),
  RenderFailed { entry, key: MessageMeasureKeyHandle, message_rows, source: RenderFailure },
}
```

所有 error 实现 `Display`、`Error` 和适用的 `source()`；crate 外 fixture 对 closed state error
无 wildcard 穷举。failure 与 cancellation 从首版就是两个 generic source，callback 的 closed
outcome 不要求 inspect 任意 error，也不能把二者压成字符串。未知 ID、invalid insert index、
invalid anchor row、invalid rebuilt config、missing measurement、missing active measurement、
stale state revision、state revision overflow 与 row arithmetic overflow 保持不同 category。
`MessageRowsError` 是独立 public value error，不伪造一个不可达的
`ZeroMessageRows { key }` state variant。Reference composite adapter 在总行数为零时返回
`MessageCompositeMeasureError::MessageRows(MessageRowsError::Zero)`；调用方将它作为
`Failed(source)` 返回后，state 的 `MeasurementFailed { key, source }` 用实际 request key
增加上下文并保留 source chain。

### 3. Exact cache 与 prefix row index

State 内部持有：

```text
entries: Vec<MessageListEntry>
positions: HashMap<MessageId, usize>
rows: Vec<MessageRows>
active_keys: Vec<MessageMeasureKeyHandle>
prefix_rows: FenwickRows
measurements: BoundedMeasurementCache<MessageMeasureKeyHandle, MessageRows>
viewport: { width, rows, scroll_offset }
stored_anchor: Option<MessageAnchor>
anchor_authority: Option<StoredAnchorAuthority> // private: ViewportTop | ExplicitNavigation
follow: BottomFollowState
revision: MessageListRevision
```

`MessageListRevision` 是本 state 的 nonzero checked generation，与 GH-62 content revision
分开，`INITIAL == 1`。Cache handle lookup 使用完整 `MessageMeasureKey` 的 deep equality；
hash/fingerprint 只找 bucket，命中后仍比较 message/revision/variant/expansion、每个
`TextFlowCacheIdentity` 和全部 shell segments。Bounded eviction 使用确定性 state-local LRU
（或仓库已有等价 deterministic policy），capacity 由 validated config 提供；两个 state
实例不能共享 eviction/measurement effects，也不能由 background 静默修改。

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
同 width 重试命中 cache。Cache eviction 只影响未来 measurement，不改变 active `rows` 或
`active_keys`。`active_keys.len() == entries.len() == rows.len()` 是每次 constructor/mutation
commit 的 postcondition；每个 slot 持有其 entry 当前 exact handle，且不是 bounded cache
的借用或唯一 owner。Cache capacity 允许小于 active entry 数，candidate 把全部 active
handles 建好后再按 deterministic LRU 写 reuse cache，eviction 不能触及 candidate/committed
active slots。`visible_range()` 只从对应 `active_keys[index]` clone handle，因此 `k` 个 slices 的 key 成本为
`O(k)` 次固定成本引用计数操作、零 source/style/config vector allocation。

### 4. Measurement contract 与原子 mutation

Public initial constructor 是唯一发布初始 state 的入口：

```text
MessageListState::try_new<F, C>(
  entries: &[MessageListEntry],
  width: u16,
  viewport_rows: ViewportRows,
  measurement_cache_capacity: usize,
  measure: impl FnMut(MessageMeasureRequest<'_>)
    -> MessageMeasureOutcome<F, C>,
) -> Result<MessageListState, MessageListMeasureError<F, C>>
```

它先验证 `width > 0`、capacity > 0、全部 ID 唯一和全部 entry/config structural invariants；
这些 preflight 全部成功前 callback count 必须为 0。然后按 `entries[0..len]` 顺序建立 exact
key、调用 cache/measure flow，stage owned entries、positions、rows、全部 `active_keys`、bounded
reuse cache 和 Fenwick；首个 missing/failed/cancelled/overflow 立即返回 Err，不发布
`MessageListState`，caller 的 input slice 不变，callback trace 精确停在失败 index。成功时一次
发布 `revision=MessageListRevision::INITIAL(1)`：空列表为 Following/offset 0/anchor None；
非空且 viewport rows>0 为 Following、offset=max offset、stored anchor=物理 top 且 authority
为 `ViewportTop`；非空且 rows=0 为 Following、offset=total rows、stored anchor=末消息最后
一行且 authority 为 `ViewportTop`。Constructor 不先创建可观察 skeleton，也不通过
`try_replace_all` 对一个半初始化 state 做 mutation。

Public mutation facade：

```text
try_replace_all(expected_revision, entries, viewport, measure)
  -> Result<MessageListMutation, MessageListMeasureError<F, C>>
try_append(expected_revision, entries, measure)
  -> Result<MessageListMutation, MessageListMeasureError<F, C>>
try_prepend(expected_revision, entries, measure)
  -> Result<MessageListMutation, MessageListMeasureError<F, C>>
try_insert(expected_revision, index, entry, measure)
  -> Result<MessageListMutation, MessageListMeasureError<F, C>>
try_update(expected_revision, entry, measure)
  -> Result<MessageListMutation, MessageListMeasureError<F, C>>
try_resize(expected_revision, width, viewport_rows, rebuild_config, measure)
  -> Result<MessageListMutation, MessageListMeasureError<F, C>>

try_remove(expected_revision, message_id)
  -> Result<MessageListMutation, MessageListStateError>
try_set_viewport_rows(expected_revision, viewport_rows)
  -> Result<MessageListMutation, MessageListStateError>
try_scroll_to(expected_revision, offset)
  -> Result<MessageListMutation, MessageListStateError>
try_scroll_to_message(expected_revision, message_id)
  -> Result<MessageListMutation, MessageListStateError>
try_scroll_to_anchor(expected_revision, MessageAnchor)
  -> Result<MessageListMutation, MessageListStateError>
jump_to_bottom(expected_revision)
  -> Result<MessageListMutation, MessageListStateError>
visible_range() -> Result<VisibleMessageRange, MessageListStateError>
observation() -> MessageListObservation
```

Rust public surface 必须写出上述真实 concrete
`Result<..., MessageListMeasureError<F, C>>`，不得发布 type alias 掩盖 error。`measure` 的
唯一首版签名为：

```text
FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<Failure, Cancellation>
```

`try_resize` 的 `rebuild_config` 也必须是公开 concrete callback bound，不得留作未声明泛型：

```text
FnMut(MessageResizeConfigRequest<'_>)
  -> MessageResizeConfigOutcome<Failure, Cancellation>
```

request 按 committed `entries` 的 index `0..len` 递增顺序调用，并借用该 index 的 exact old
entry 与 active handle 内 exact old key，同时传入 validated new width/viewport rows。
`Rebuilt(config)` 必须逐项验证：shell outer width 等于 new width；horizontal insets checked
后与每个 child TextFlow max width 一致；structural slot 唯一且 rows checked；从 unchanged
message ID/revision/variant/expansion 与 candidate config 构造的新 key 与 config 深度相等。
不匹配返回 `InvalidResizeConfig`。`Failed`/`Cancelled` 映射为各自 config-rebuild typed error，
保留 index/message/source，不得调用该 entry 的 measurement。

`Measured`、`Missing`、`Failed(source)`、`Cancelled(source)` 是 closed variants；
MessageList 不 inspect 任意 `Failure`、不靠 downcast/`Any` 猜 cancellation。request 同时借用
candidate 中的 exact entry 和从该 entry 构造的 exact composite key。测量是 mutation 内同步
完成的 staged callback；首版不发布 `try_apply_measurement` 或其他 delayed-result API。未来若
需要 async，必须另行 spec 一个可达的 staged-mutation token/candidate flow，不能把未提交 key
交给只接受 active key 的方法。

所有 mutation 遵循：

```text
validate expected state revision
  -> validate operation-specific structural boundary
       (insert index、target/input IDs、anchor coordinates) against committed state
  -> detect exact no-op from committed values and return NoChange
  -> checked state revision +1
  -> clone/stage affected order, positions and viewport inputs
  -> collect exact cache hits and required keys
  -> synchronously run all missing MessageMeasureRequest callbacks
  -> map Missing/Failed/Cancelled to distinct typed errors
  -> accept only validated MessageRows and checked totals
  -> stage one exact active key handle per candidate entry
  -> build/update candidate prefix index
  -> restore candidate anchor/follow state
  -> commit all candidate fields once
```

任一步 Err/cancellation drop candidate，原 entries/positions/rows/active_keys/index/cache/
viewport/stored_anchor/anchor_authority/follow/revision 逐字段不变。优先级固定为 expected
revision guard → target/ID/anchor
structural validation → exact no-op detection → `MessageListRevision::checked_next` →
measurement callback/result → candidate arithmetic/postcondition。no-op 在 max revision 仍返回
`NoChange`；因而仅对确有 observable change 的 operation，revision 为 `u64::MAX`、target 合法但 callback
原本会返回 zero/malformed rows 时，必须返回 `StateRevisionOverflow`、callback invocation
count=0 且完整 state 相等；unknown target 仍先返回 `UnknownMessageId`。成功但无 observable
变化的 scroll/resize/update 返回 `NoChange { revision }` 且 revision 不增加；`Applied`
始终返回前后 revision 与 `anchor_clamped`/`viewport_clamped` 两个明确 flag。

`try_insert` 的 structural validation 在 expected revision guard 后首先检查
`index <= entries.len()`：`index == len` 是合法尾插，`index > len` 返回
`InvalidInsertIndex { index, len }`。这个结果优先于 duplicate/new-entry validation，且发生在
任何 entry/config clone、vector indexing、cache lookup 或 callback 前；exact fixture 以一个
duplicate ID + invalid index 的输入证明 callback count=0、没有 panic，完整 state/cache/
observation/revision 相等。合法 `index == len` 再执行普通 duplicate-ID 和 measurement 流程。

Resize 的子流程固定为：

```text
validate expected revision/new width/viewport rows
  -> width 与 viewport rows 都相同：NoChange，两个 callback count=0
  -> width 相同但 viewport rows 改变：不调用 rebuild/measure，执行 viewport transition
  -> checked state revision +1
  -> 按 committed index 顺序调用每个 rebuild_config
  -> 每个 Rebuilt config 立即 deep validation 并建立 immutable candidate key handle
  -> 按相同 entry 顺序处理 exact cache hits，再对 miss 调 measure
  -> checked candidate index + anchor/follow restore
  -> 一次 commit
```

任一 config `Failed`/`Cancelled`/invalid，或后续 measurement missing/failure/cancellation，
丢弃全部 candidate configs/handles/cache/index；即使前面 callbacks 已成功，active state 和
observation 仍逐字段不变。expected revision、invalid width/config-independent input、
exact no-op 与 state revision overflow 均在首次 rebuild callback 前判定；overflow 时 rebuild
与 measure counts 都为 0。

Content streaming、variant change、expand/collapse 都通过同 ID 的 `try_update`：任何 key
或 composite config 字段变化只要求受影响 entry 的新 measurement，point update
`O(log n)`；key 完全相等则禁止重新测量。

Repository-provided reference adapter `try_measure_composite`（位于 facade，不进入
`height_index`）必须对 request 中有序的每个 `TextFlowCacheIdentity` 调 GH-58 checked
`TextFlow::try_build(input, options)`，逐项 checked 累加 `row_count`，再 checked 累加全部
`MessageStructuralSegment.rows`，最后调用 `MessageRows::try_new(total)`。该 adapter 返回
`Result<MessageRows, MessageCompositeMeasureError<TextFlowFailure>>`；total=0 保留
`MessageRowsError::Zero` source，不能自行附造 key。结构 segments 分别表达
role/code/block header、status marker、inter-block spacing、outer padding/border 等 renderer
占行，不能把多个 block 拼成单个 TextFlow 或只测 message body。horizontal insets 必须与每个
child identity 的 `options.max_width` 一致；不一致 typed 失败，不能猜宽度。

该 adapter 是 renderer-equivalent composite contract：集成 fixture 用至少
Text + Code(header/body) + Thinking + ToolResult 四个 textual children，再加 role header、
status、三处 block spacing、padding 和 border，断言 composite rows 等于完整 shell 的最终
terminal rows。GH-63 只可在 render closure 内把相同 typed message 变成 Element；measurement
request、config、cache 和 prefix index 都不 import `ChatMessageView` 或 GH-63 block/view
类型。

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

Committed state 保证每个 entry 同时有 active height/key；若内部 slot parity 被破坏则返回
`MissingActiveMeasurement { message_id }`，不能从 bounded reuse cache 临时找回、跳过或按
一行处理。每个 slice
验证 `start < end <= measured_rows`，并从 active slot clone
`MessageMeasureKeyHandle`。handle clone 固定 `O(1)` 且零 key/config/source/style deep copy，
所以总工作量按 message count 为 `O(log n + k)`，并且不隐藏与消息正文长度相关的 per-slice
成本。exact counter/allocation fixture 断言每 slice 一个 shared-handle clone、零 key vector
clone；cache/mutation identity check 仍执行 deep equality。

### 6. Anchor、删除与 follow state transition

Private `anchor_authority` 区分 `ViewportTop` 与 `ExplicitNavigation`。非零 viewport 的
Paused state 只有在 authority=`ViewportTop` 时，mutation preflight 才从当前 visible top
刷新 `{message_id, top.message_rows.start}`；authority=`ExplicitNavigation` 时必须直接使用
stored requested anchor，即使前一次 navigation 因 max-offset clamp 使它位于 viewport 中部。
显式 user scroll 把 authority 改为 `ViewportTop`，typed navigation 把它改为
`ExplicitNavigation`，jump/bottom-follow 重算物理 top 后为 `ViewportTop`。普通 mutation
restore 不得改变 surviving explicit authority。viewport rows=0 时 visible slices 为空但保留
anchor 与 authority。只有空列表或 anchor 删除且无 survivor 时两者都清除。恢复规则按优先级：

1. 成功 `try_scroll_to_anchor/message`：先应用 validated requested anchor、转换为
   `Paused { new_content_below: false }`、authority=`ExplicitNavigation`，再做 max-offset
   clamp；该命令优先于下方 Following restoration，不能被强制回 bottom；
2. `Following` 且 viewport rows>0：忽略旧 top anchor，commit 后 offset=新的 max offset，
   anchor 从新 viewport 重新计算且 authority=`ViewportTop`；
3. `Following` 且 viewport rows=0：保持 Following 与 surviving stored anchor，offset 使用
   `logical_bottom=total_rows` end coordinate；append/stream growth 更新 logical bottom，
   不设置 new-content indicator；
4. `Paused` 且 stored anchor ID 存在：恢复同 intra row；若消息缩短，clamp 到 `rows-1` 并在
   `MessageListUpdate.anchor_clamped=true`；
4a. `Paused` 的非空 candidate 无有效 surviving anchor 时——committed list 为空且 anchor
   `None`，或 replace-all 删除 anchor 且没有任一旧 ID 存活——任何首次引入内容的 append、
   prepend、insert、replace-all 都保持 `Paused { new_content_below: true }`，选择 candidate
   首条消息 row 0、authority=`ViewportTop`、offset=0；零/非零 viewport 行为相同；
5. stored anchor 被删除：按 mutation 前 order 选择下一 surviving ID 的 row 0；无下一项则选上一
   surviving ID 的 `rows-1`，replacement authority=`ViewportTop`；空列表为 None/offset 0；
6. global offset 为 `prefix_sum(anchor_index)+intra_row`，再做 checked/max-offset clamp；
   若 viewport 比 remaining content 高，viewport top 的物理 clamp 可使 anchor 位于 viewport
   内而非首行，update result 必须显式报告 `viewport_clamped=true`，不能假称 exact top。
7. viewport 从 0 恢复为非零时，Paused 从 preserved stored anchor 恢复并报告 clamp；
   Following 则使用包含零 viewport 期间所有 append/stream growth 的最新 total rows 计算
   最新 max offset，保持 Following/indicator=false，再从恢复后的 viewport top 更新 anchor
   与 `ViewportTop` authority。

`try_scroll_to_message` 等价于 typed row-0 anchor navigation；
`try_scroll_to_anchor` 先验证 ID 存在，再要求
`intra_message_row < measured_rows`。unknown ID 返回 `UnknownMessageId`，越界 row 返回
`InvalidAnchorRow { requested, measured_rows }`，两者都不 clamp、不递增 revision。合法导航的
global offset 仍受 max-offset 限制，若 anchor 只能位于 viewport 内部则返回
`Applied(... viewport_clamped=true)`。Mutation 后因原合法 anchor 所在消息缩短才允许
`anchor_clamped=true`；这与 caller 直接请求非法 row 的 typed error 保持可区分。
合法 typed navigation 无论此前为 Following 或 Paused，都以 requested anchor 替换
`stored_anchor`、清除旧 `new_content_below` 并保持 Paused；只有
`try_scroll_to(offset=max_offset)` 或 `jump_to_bottom` 进入 Following。unknown/invalid
navigation 在 follow transition 前失败，因此原 Following/Paused、indicator、anchor、offset、
revision 均不变。

Follow transition table：

| Input | Before | After |
| --- | --- | --- |
| explicit user scroll 到 `< max_offset` | 任意 | `Paused { existing flag }`，捕获 top anchor |
| explicit user scroll 到 `max_offset` | 任意 | `Following`，清 flag |
| jump-to-bottom | 任意 | `Following`，offset=max，清 flag |
| typed anchor/message navigation 成功 | 任意 | requested anchor 优先；`Paused { false }` |
| typed navigation unknown/invalid | 任意 | typed error；完整 observation/state 不变 |
| viewport rows `nonzero -> 0` | Paused | 空 slices，保留 stored anchor 与 flag |
| viewport rows `0 -> nonzero` | Paused | 从 stored anchor 恢复并报告必要 clamp |
| viewport rows `nonzero -> 0` | Following | 空 slices，保留 anchor；logical bottom=total rows |
| append/stream growth at zero rows | Following | 保持 anchor/Following/indicator=false；logical bottom 随 total rows |
| viewport rows `0 -> nonzero` | Following | 恢复到最新 max bottom，重算 anchor，indicator=false |
| append/stream growth at nonzero rows | Following | Following，offset 跟随新 max |
| append/stream growth below prior viewport | Paused | 保持 anchor，`new_content_below=true` |
| first nonempty append/prepend/insert/replace with no surviving anchor | Paused | 首条 row 0 / offset 0 / `ViewportTop`；保持 Paused，indicator=true |
| prepend/insert/update/resize/expand/collapse/delete | Paused | 保持/替代 anchor；不得自动 Following |
| mutation removes all content | Paused | Paused + None anchor；flag 保留到显式 jump/scroll-bottom |

“below prior viewport”以 mutation 前 viewport end 与变更 row interval 比较，不能用“是否最后一条
消息”猜测。`new_content_below` 一旦 true，在 Paused 内保持 true，直到显式回到底部。

`observation()` 返回一个按调用时刻复制 scalar/validated values 的
`MessageListObservation`；`new_content_below` 必须与
`follow_state == Paused { new_content_below: true }` 等价，Following 时恒为 false。
revision、follow state、stored anchor 与 indicator 均有 public read-only getters/fields，
不返回 `&mut`、interior-mutability guard、entries/cache/index 或可写 state handle。crate 外
fixture 从 paused append/stream 读到 indicator=true，并在 navigation/jump 后读到上述精确
transition。

### 7. Render closure 与 GH-63

Facade 只消费 immutable state/visible range：

```text
MessageList::new(&state)
  .try_into_element(
    |entry: &MessageListEntry,
     key_handle: &MessageMeasureKeyHandle,
     visible_slice: &VisibleMessageSlice|
     -> Result<Element, RenderFailure> { ... }
  )
  -> Result<Element, MessageListRenderError<RenderFailure>>
```

closure 按 slices 顺序恰好调用一次，并同时收到 state 中用于 measurement/index 的 exact
borrowed entry、由该 entry 生成且与 slice 内 `measure_key` 指向同一 immutable allocation 的
exact shared key handle，以及
message-local partial range。facade 在调用前逐项验证
`entry.message_id/revision/variant/expansion`，再把 entry config 派生成 total snapshot 与
`key_handle.as_key()` 的 snapshot 比较，并验证
`Arc::ptr_eq(key_handle, slice.measure_key)` 或等价 typed identity；任一漂移返回
`MeasurementIdentityMismatch`，绝不让 caller 只按 ID 查到新版内容却使用旧 geometry。调用方可用该 exact
entry/revision 选择对应 GH-62 immutable snapshot。当前 #63 OPEN/#145 仅 T1 时调用方使用
通用 closure 构造 Element；只有 #63 CLOSED 且完整 implementation ancestry 已验证时才可在
closure 内建立完整 GH-63 typed view 并裁剪到 partial range。MessageList height index
不 import renderer 类型、不解释 block、不重算 TextFlow。

任何 closure failure 立即返回带 exact entry/key handle/range/source 的 typed error，不返回已构造的
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
| B-002 | complete TextFlow + shell config + total-float identity | `measurement_config_covers_textflow_and_shell_inputs`；`measurement_key_uses_all_identity_fields_and_exact_equality`；`measurement_key_total_equality_is_reflexive_for_nan_payloads`；`measurement_key_total_snapshot_distinguishes_nan_payloads_and_signed_zero` |
| B-003 | caller-owned state/revision/observation | `identical_inputs_produce_identical_state`；`public_observation_is_read_only_and_reports_new_content` |
| B-004 | constructors/stored anchor/visible range | `constructor_measures_initial_entries_in_order_and_publishes_complete_state`；`constructor_failure_publishes_no_state`；`empty_zero_viewport_and_zero_width_contract`；`zero_viewport_retains_and_restores_stored_anchor`；`following_zero_viewport_append_and_restore_latest_bottom` |
| B-005 | lower-bound/slice builder | `partial_first_and_last_message_ranges_are_row_exact` |
| B-006 | prepend restore/max-offset clamp | `prepend_preserves_top_or_reports_short_content_viewport_clamp` |
| B-007 | typed anchor navigation + update clamp | `typed_anchor_navigation_rejects_unknown_and_invalid_rows`；`typed_navigation_overrides_following_and_replaces_observed_anchor`；`viewport_clamped_navigation_anchor_survives_next_mutation`；`height_changes_preserve_or_report_anchor_clamp` |
| B-008 | remove transition | `deleted_anchor_selects_next_then_previous_survivor` |
| B-009 | follow transition + public observation | `follow_pause_and_explicit_resume_state_machine`；`typed_navigation_overrides_following_and_replaces_observed_anchor`；`paused_without_surviving_anchor_structural_repopulation_is_deterministic`；`public_observation_is_read_only_and_reports_new_content` |
| B-010 | append/stream transition | `append_and_stream_growth_follow_or_mark_new_content`；`following_zero_viewport_append_and_restore_latest_bottom`；`paused_without_surviving_anchor_structural_repopulation_is_deterministic` |
| B-011 | isolated invalidation/cache/resize config | `each_textflow_and_shell_input_invalidates_only_affected_entry`；`resize_variant_expansion_and_structure_cache_contract`；`resize_rebuild_config_is_closed_ordered_and_atomic`；`active_measurement_handles_survive_reuse_cache_eviction` |
| B-012 | closed callback outcome + staged commit | `constructor_failure_publishes_no_state`；`measured_missing_failed_and_cancelled_outcomes_are_closed`；`measurement_failure_and_cancellation_are_atomic`；`resize_rebuild_config_is_closed_ordered_and_atomic` |
| B-013 | closed errors and structural boundaries | `message_rows_reject_zero_without_measurement_key`；`closed_error_categories_are_exhaustive_and_keep_sources`；`invalid_insert_index_precedes_clone_index_and_callbacks`；`state_revision_overflow_precedes_measurement_and_is_atomic_at_u64_max` |
| B-014 | state revision/no-op/overflow | `stale_state_revision_and_noop_revision_contract`；`state_revision_overflow_precedes_measurement_and_is_atomic_at_u64_max` |
| B-015 | renderer-equivalent composite adapter | `composite_height_matches_renderer_equivalent_rows` |
| B-016 | exact entry/shared-key-handle/slice facade | `active_measurement_handles_survive_reuse_cache_eviction`；`render_closure_receives_exact_entry_key_and_slice`；`visible_slice_key_handle_is_o1_shared_immutable_and_send_sync`；`render_revision_drift_is_rejected_before_callback` |
| B-017 | render facade | `render_failure_has_source_and_never_returns_partial_frame` |
| B-018 | Fenwick + shared-handle counters | `lookup_and_point_update_have_logarithmic_operation_bound`；`visible_slice_key_handle_is_o1_shared_immutable_and_send_sync` |
| B-019 | rebuild/cache counter | `structural_and_resize_costs_are_explicit_and_reuse_cache`；`resize_rebuild_config_is_closed_ordered_and_atomic` |
| B-020 | proptest oracle | `variable_height_index_matches_naive_oracle` |
| B-021 | bench + allocation/counter | `cargo bench --bench message_list -- message_list_10k`; `visible_slice_key_handle_is_o1_shared_immutable_and_send_sync`; B-018 counter test |
| B-022 | unchanged fixed API | `fixed_height_virtual_scroll_api_is_unchanged` |
| B-023 | implementation preflight | `dependency_completion_records_require_closed_issues_and_complete_commit_sets`；fresh issue/PR/ancestry commands in task gate |
| B-024 | GH57 child coverage + closure audit | ordinary no-mode all-target test asserts typed missing/zero-side-effect and passes；ledger/raw coverage 使用 `fixture`，producer/validator 使用 `produce`/`validate`；unknown/wrong-stage negative fixtures；current-head CI/review evidence |

每个 bare test name 都对应 tasks 中的完整 `cargo test ... -- --exact` 命令。Property test 使用
固定 32-byte ChaCha seed、至少 256 cases，并输出 seed/最小操作序列。Operation-count test 是
复杂度硬门禁；divan benchmark 不用 wall-clock threshold 造成 flaky CI。

## 数据流

```text
GH-62 ordered messages + viewport(width, rows)
  -> resize-config callback(old exact entry/key + new width/viewport)
       -> Rebuilt(config) | Failed(source) | Cancelled(source)
       -> ordered candidate config deep validation
  -> exact MessageMeasureKey per message
       -> ordered Vec<GH-58 TextFlowCacheIdentity>
       -> complete shell structural segments
  -> synchronous closed measure callback
       -> Measured | Missing | Failed(source) | Cancelled(source)
       -> reference adapter builds every GH-58 TextFlow child
       -> checked sum(child row_count + all structural rows)
  -> candidate exact cache + rows + Fenwick prefix index
  -> anchor/follow transition + checked state revision
  -> atomic MessageListState commit
  -> visible_range(offset, viewport rows)
  -> Vec<VisibleMessageSlice(shared immutable O(1)-clone key handles)>
  -> typed render closure(exact entry + exact key handle + exact slice；#63 closure-complete 后才调用 full view)
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
- Dependency drift: 当前 base 的 GH-58/GH-60 仍 OPEN，GH-63 仅 T1；必须执行 source-drift/
  ancestry gate，不能以本文伪签名强行适配。

## 测试计划

- [ ] Unit tests：initial constructor ordered publish/failure-no-state、完整 TextFlow/shell
      total identity/NaN/signed-zero、active handles surviving bounded-cache eviction、isolated invalidation、closed measurement/
      resize-config callback outcomes、invalid-insert pre-clone boundary、partial slices、typed
      navigation overriding Following、viewport-clamped explicit anchor surviving next mutation、
      short-content、Paused no-survivor structural repopulation 与 Following/Paused zero-viewport restore、
      delete/follow transition、cache reuse、stale/state-revision-overflow/missing/zero/failure/
      cancellation atomicity、unkeyed zero-row value error、exact GH-57 aggregate symbol与
      operation count。
- [ ] Property tests：固定 seed 的 public operation 序列对比 naive oracle；private revision
      overflow 只由 T2 unit exact 在 `u64::MAX` 注入验证，不在 crate-outside property 伪造。
- [ ] Integration tests：多 TextFlow + 全 structural rows composite measurement、exact
      entry/shared-key-handle/slice 与 revision-drift rejection、shared handle `O(1)` clone/
      `Send + Sync`、generic render closure（仅 closure-complete 时调 GH-63 view）、typed render failure、crate 外 read-only
      observation/public errors/exports、fixed-height compatibility。
- [ ] Benchmark：divan 10k mixed heights 的 lookup/slice/point update/prepend，保存 current
  head 命令和输出。
- [ ] Full gates：`cargo fmt --all -- --check`、`cargo check --workspace --all-targets
  --all-features --locked`、`cargo test --workspace --all-targets --all-features --locked`
  （ordinary no-mode contract path须断言 typed missing且零副作用后通过）、
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`、
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`。
- [ ] Coverage：Tasks 中必须且只能有一个 `gh57-critical-paths-v1` block，version=1、
  issue=65、9 个 unique exact `file + name` 与逐项 nonempty `verification_command`。
  第九条 ledger command 与唯一 raw all-target coverage command 必须显式使用 `fixture`；
  producer/validator 分别只允许 `produce`/`validate`。ordinary no-mode test harness 必须
  成功证明内部 action 的 missing 拒绝且零副作用；unknown top-level mode、wrong-stage action
  与任何 missing artifact action 都必须失败。
  `gh65_current_head_coverage_contract` 从 raw llvm-cov、committed ledger 和
  merge-base..exact-head diff 确定性 produce/validate `gh57-child-coverage-v1`；artifact
  绑定 child/head/base/merge-base、head commit timestamp、raw absolute path/SHA256、非空
  coverage command，changed executable denominator>0 且至少80%，critical set严格相等、
  每项 denominator>0 且100%。缺失、旧SHA、集合漂移、零 denominator、unknown/duplicate
  symbol、hash/threshold 不符均 fail closed；普通 CI summary 不能替代 artifact。
- [ ] Manual verification：无必需终端 UI 手测；example/adoption 不在 scope。审阅 benchmark
  trend、public API docs 与 PR current-head evidence。

## 回滚方案

通过普通 revert 移除 MessageList exports、modules、tests、bench 与 Cargo bench entry。不得
force push；不得回退 GH-58/GH-60/GH-62。因为旧 `virtual_scroll_view` 未修改，回滚不会要求
旧调用方迁移。若性能或 correctness gate 失败，在 merge 前保持 PR 未合并并修正根因，不以
默认高度、关闭 property case 或弱化断言作为回滚。
