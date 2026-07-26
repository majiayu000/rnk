# Product Spec：variable-height MessageList 与滚动锚定

## Linked Issue

GH-65: https://github.com/majiayu000/rnk/issues/65

## 用户问题

当前 `virtual_scroll_view` 把 `scroll_offset`、`viewport_height` 和每个 item 都当作相同高度的
计数单位。聊天消息在终端中会因宽度、换行、内容流式增长、展示 variant，以及 thinking /
tool result 的展开状态占用不同的终端行，因此现有模型会跳过错误消息、截断可见内容，并在
prepend、resize 或 streaming 时产生可见跳动。

调用方需要一个以终端行为坐标、以稳定消息身份为锚点的 `MessageList`。它必须在用户仍跟随
底部时继续跟随，在用户向上阅读历史时保持原位置并提示下方有新内容。

## 目标

- 提供 caller-owned、基于稳定 `MessageId` 的 variable-height message-list state。
- 以终端行而不是消息条数计算可见范围，并暴露首尾消息的 message-local partial row range。
- 对 prepend、append、streaming、resize、variant、展开/收起、删除和测量失败定义确定行为。
- 让高度缓存、滚动锚点、底部跟随和新内容提示都可观察、可测试且无静默降级。
- 为 10,000 条混合高度消息提供对数级定位/点更新合同和可重复 benchmark。
- 保持既有 fixed-height `virtual_scroll_view` API 与行为兼容。

## 非目标

- 不重写通用 `ScrollableBox`、`ScrollState` 或 fixed-height `virtual_scroll_view`。
- 不强制现有调用方迁移，也不在本 issue 迁移 example 或完整 chat application。
- 不实现 provider、conversation reducer、Composer、消息 block renderer、inline scrollback、
  selection、copy 或 search UI。
- 不复制 GH-58 的 TextFlow/Unicode width/wrapping 算法，不替代 GH-60 的事务式 layout，
  不重新定义 GH-62 的消息身份/revision。
- 不把高度索引耦合到 GH-63 的具体 renderer；GH-63 只能通过 render closure 被消费。

## Behavior Invariants

1. **B-001 — Stable identity and row units.** Message-list state 接受 GH-62 的稳定
   `MessageId` 和 `MessageRevision`；所有 content height、viewport height、scroll offset、
   anchor offset 与 visible range 均以终端行为单位，不以消息条数或 byte/char 数为单位。
2. **B-002 — Exact measurement identity.** 一条已测量高度只可由完全相等的消息身份、
   width、content revision、variant、expansion 和完整测量配置命中。完整配置必须区分每个
   textual child 的 source/style、wrap、tab stop、ellipsis、横纵 overflow、Unicode width
   policy，以及 role/code header、status、block spacing、padding、border 等全部结构性高度
   输入；任一输入改变都不能复用旧高度，hash collision 不能被视为相等。
3. **B-003 — Deterministic independent instances.** 相同初始状态、输入序列和测量结果必须产生
   相同顺序、高度、anchor、visible slices、follow state 与 revision；两个调用方分别创建的
   message-list state 互不影响，一方的测量、淘汰、滚动或 mutation 不改变另一方的可观察结果。
4. **B-004 — Empty and zero-sized viewport.** 初始构造是公开、可失败且原子的 transaction：
   先验证全部初始输入，再按输入顺序测量，成功后一次发布 revision=1 的完整 state；构造失败
   不发布 partial state。空列表产生零 content rows、零 offset、无 anchor、Following 和空
   visible range；非空列表从 bottom-follow 的确定 observation 开始。非空列表的 viewport
   rows 变为零时不渲染消息，但保留已有的 surviving
   message/intra-row anchor。Paused 恢复为非零 rows 后从该 anchor 恢复并报告必要的
   anchor/viewport clamp；Following 在零 rows 期间保持 logical bottom，恢复时必须位于包含
   期间 append/stream growth 的最新 bottom。width 为零若无法测量则返回 typed error，不能猜测
   默认高度。
5. **B-005 — Row-accurate partial visibility.** 可见结果按消息顺序返回；每项同时包含稳定
   `message_id`、message-local row range 和 viewport-local row range。第一条和最后一条消息
   可为 partial range，所有 range 均为半开区间且不越过已测量高度。
6. **B-006 — Prepend anchor with explicit viewport clamp.** 非空 viewport 前 prepend 任意数量
   消息后，若原 top message 仍存在且 max-offset 允许，则其 `message_id` 与原 top
   intra-message row 保持在 viewport 顶部。若全部内容短于 viewport，或 max-offset 使该
   anchor 不可能位于首行，则保留同一 surviving anchor、把 offset clamp 到合法范围，并在
   update result 中报告 `viewport_clamped=true`，不能假称 exact-top 成功。
7. **B-007 — Typed anchor navigation and mutation preservation.** 调用方可以按稳定
   `message_id + intra-message row` 导航，不需要把旧 global offset 当作 message identity；
   合法显式导航优先于 Following/restoration，进入 Paused 并以请求 anchor 替换 stored anchor；
   即使 max-offset clamp 使请求 anchor 只能位于 viewport 内部，下一次 mutation 仍使用该显式
   请求 anchor，不能先以物理 viewport top 覆盖；只有后续显式 user scroll/bottom command
   或另一条 typed navigation 才改变 anchor authority。
   unknown ID 或请求超出该消息已测量范围必须 typed 失败且零 mutation。Insert、append、
   content update、streaming delta、resize、variant change、expand/collapse 和其他高度变化后，
   原 anchor message 若仍存在就保持同一 intra-message row；若新高度更短则只允许 clamp 到
   该消息最后一个有效 row，并在 update result 中报告 `anchor_clamped=true`。
8. **B-008 — Deleted-anchor rule.** 删除 anchor message 时，优先锚定删除前顺序中的下一条
   surviving message 的首行；若没有下一条则锚定上一条 surviving message 的末行；列表变空
   时 anchor 变为 `None`、offset 归零。未知 ID 删除必须 typed 失败且不改变状态。
9. **B-009 — Explicit bottom-follow state.** Follow state 只有可区分的 `Following` 与
   `Paused { new_content_below }`。用户从底部向上滚动后进入 Paused；只有显式滚动到新底部或
   jump-to-bottom 才恢复 Following 并清除提示，resize/delete/collapse 不得隐式恢复。调用方
   必须可通过 immutable public observation 读取 revision、follow state、stored anchor 与
   `new_content_below`，但不能借此修改内部状态。
10. **B-010 — Follow-bottom append and growth.** Following 时 append 或最后消息 streaming
    growth 后，viewport 继续贴住新的最大 offset；viewport rows=0 时 offset 使用 logical
    bottom end、保留 anchor 且不产生 new-content indicator，恢复非零 rows 后贴住包含期间变化
    的最新 bottom。Paused 时这些变化保留 anchor；当变化在原 viewport 下方新增可见内容时
    `new_content_below=true`，并持续到用户返回底部。
11. **B-011 — Update and isolated invalidation semantics.** 同一 stable ID 的 content revision、
    variant、expansion，或 B-002 任一 textual/structural 配置输入改变时，只使受影响的 exact
    measurement 失效；width 改变为所有消息建立新宽度的测量视图。未改变 exact key 的
    prepend、append、insert、delete 或重排可复用缓存，不得因结构变化重新测量全部消息。
    Resize 必须按 entry 顺序以 old exact entry/key 与新 width 构建并深度验证 candidate config；
    config callback 的 failure/cancellation 与 measurement 一样 typed 且整次原子回滚，同
    width 的 no-op 不调用 config 或 measurement callback。Bounded reuse cache 的 capacity
    可以小于 active message 数；淘汰只影响未来复用，不能移除 active entry 当前成功测量的
    exact key handle 或使 `visible_range` 失败。
12. **B-012 — Atomic measurement failure.** 初始构造或一次 mutation 所需的任一测量返回 missing、
    failure 或 cancellation 时，整个 mutation 失败；消息顺序、高度索引、cache、anchor、
    follow/new-content state 和 state revision 与调用前逐字段相同；构造失败不返回 state，
    callback 顺序停在首个失败项且没有 partial observation 可见。
13. **B-013 — Typed fail-loud errors.** Duplicate/unknown message ID、invalid anchor row、
    invalid insert index、missing measurement、zero row height、stale state revision、state
    revision overflow、row arithmetic overflow、coordinate conversion overflow、config/
    measurement failure 和 cancellation 都保留独立 typed category 及可用 source。
    `MessageRows::try_new(0)` 返回不依赖 measurement key 的 public value error；因为
    `Measured` 只能携带已验证 `MessageRows`，state 不声明不可达的 keyed zero-row variant。
    Composite adapter 的零总行 error 作为 measurement failure source 进入 state，并由实际
    request key 提供上下文。Insert
    `index == len` 合法，`index > len` 在 callback、clone 或 indexing 前 typed 失败且逐字段
    零 mutation；不得返回 `Any`、仅字符串错误、warning+fallback 或默认 `1 row`。
14. **B-014 — Revisioned atomic mutations.** State 使用单一 checked revision；每次成功的
    可观察 mutation 恰好递增一次，无变化或失败不递增。携带旧 expected state revision 的
    mutation 必须 typed 拒绝且不能覆盖当前高度；revision 已到最大值时，在调用测量 callback
    或修改任一字段前 fail closed。首版不暴露没有可达成功路径的 delayed-result apply 操作。
15. **B-015 — Renderer-equivalent composite height.** 在相同消息 revision、完整测量配置和
    viewport width 下，缓存的消息高度必须等于最终完整消息 shell 的终端行数：每个 textual
    child 的换行行数和所有 header、status、block spacing、padding、border 等结构行都被计算
    恰好一次。输出不得因只测量其中一个文本块而重叠、截断或错位。
16. **B-016 — Exact measured/rendered revision.** 每个 visible render 调用必须同时收到产生该
    slice 的 exact message entry、shared immutable exact composite measurement-key handle 和
    message-local slice；content revision、variant、expansion 或配置不同的内容不能使用旧
    geometry 渲染。每个 active entry 的 handle 由非淘汰 active slot 持有；bounded reuse
    cache 不能成为 active handle 的唯一 owner。调用方可选择任意 renderer，且更换 renderer
    不改变索引、顺序或 slice 结果。
17. **B-017 — Render failure propagation.** Render closure 对任一 visible slice 失败时，
    MessageList 返回保留原 source 的 typed render error；不跳过该消息、不输出 partial/default
    frame，也不修改 caller-owned state。
18. **B-018 — Logarithmic lookup and point update.** 对已测量的 `n` 条消息，从全局 row offset
    定位首条可见消息为 `O(log n)`；单条高度/content streaming delta 的索引更新为
    `O(log n)`。构造 `k` 条 visible slices 为 `O(log n + k)`，其中每个 key handle clone 为
    `O(1)` 且不复制 source/style/config vectors；每帧不得全表扫描、全表重测或对每条 slice
    深拷贝 measurement identity。
19. **B-019 — Structural and resize cost is explicit.** Prepend、middle insert/delete/reorder
    允许 `O(n)` 重建 row index，但复用 unchanged exact cache entries；width resize 允许为
    `n` 条消息各测量一次并重建，后续同宽度查询不得重复全表测量。
20. **B-020 — Property oracle.** 固定 seed 的 property test 对随机消息高度和
    append/prepend/insert/update/delete/resize/scroll 序列，将 prefix-index 的 total rows、
    offset lookup、partial slices、anchor 与 follow state逐步和独立 naive row-vector oracle
    比对；失败 seed 与操作序列必须可重放。
21. **B-021 — 10k performance evidence.** 10,000 条 mixed-height 消息 benchmark 覆盖
    首可见定位、visible slice 构造、高频单消息 streaming update 与 prepend；确定性 operation
    counter test 对 lookup/point update 提供对数上界硬门禁，benchmark 提供当前 head 的趋势
    证据而不使用易波动的 wall-clock pass/fail 阈值。
22. **B-022 — Fixed-height compatibility.** 既有 `virtual_scroll_view` 的公开路径、函数签名、
    fixed-height item-count offset/viewport 语义和边界行为保持不变；MessageList 是新增 API，
    不静默改变旧调用方。
23. **B-023 — Availability only after completed dependencies.** 当 GH-58、GH-60 或 GH-62
    尚未完整交付时，GH-65 的 production API 不能被报告为可用或完成；三项完成后发布的
    MessageList 必须消费它们已经交付的 typed text/layout/message contracts，不能以 guessed
    fallback 代替。GH-63 是否存在不改变 core index；存在时只影响调用方选择的 renderer。
24. **B-024 — Reproducible completion result.** 对被声明为完成的一个 exact implementation
    version，任何审阅者按该版本声明的完整验证集执行，都必须得到相同的 exact-test、
    property、10k workload、coverage 和 compatibility 通过结果。Coverage 必须由 committed
    `gh57-critical-paths-v1` ledger 确定性生成 `gh57-child-coverage-v1`，绑定 exact head/base/
    raw provenance，changed executable 至少 80%、ledger critical paths 逐项 100%；coverage
    contract 的每次调用都显式提供受支持的 `GH65_COVERAGE_MODE`，missing/unknown mode fail
    closed，ledger 中的 fixture 命令也不能省略该模式。其他版本、零匹配、ignored test、
    部分列表或无法绑定该版本的结果不能证明完成。

## 验收标准

- [ ] 公开 state/API 使用 GH-62 stable ID/revision，并以 row units 表达 viewport、offset、
  anchor 与 partial visible ranges。
- [ ] Cache key 深度包含 message identity、全部 TextFlow 输入和完整 composite shell 配置，
  每一项的 isolated invalidation、reuse 和 collision equality 均有 exact tests。
- [ ] Prepend 的 exact-top 与 short-content viewport clamp、typed anchor navigation、
  typed insert-index boundary、Following/Paused 各自的 zero-row retention/restoration，以及
  append/stream/resize/expand/collapse/delete 规则均有确定 fixture。
- [ ] Following/Paused/new-content 状态转换完整覆盖用户 scroll、jump-to-bottom、append、
  typed navigation、zero viewport、streaming、resize、collapse 与 delete；public immutable
  observation 可读取 indicator/revision/anchor。
- [ ] Initial constructor 的 validation、ordered callbacks、revision=1 observation 与失败不发布
  partial state 均有 exact tests；reuse-cache capacity 小于 active count 时 active handles
  仍全部可见。
- [ ] Missing/unknown/duplicate/invalid-anchor/zero-height/state-revision-overflow/stale/
  row-overflow/failure/cancellation 都 typed、fail-loud、原子且保留 source；callback outcome
  闭合区分 measured/missing/failed/cancelled，无默认高度或 partial commit。
- [ ] Visible slices 只 clone `O(1)` shared immutable key handle；固定 seed property oracle、
  确定性复杂度门禁和 10k benchmark 在当前 head 实际运行。
- [ ] GH-63 只经 render closure 消费；render failure typed 传播且不输出 partial/default frame。
- [ ] 既有 `virtual_scroll_view` 兼容 fixture 与 crate 外 public API fixture 通过。
- [ ] Implementation 开始前验证 GH-58/GH-60/GH-62 merged ancestry；最终 exact-head evidence
  含可复算的 GH57 child coverage artifact，所有 coverage contract invocation 显式给 mode，
  并满足 B-024。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-004、B-008、B-012；空列表、Following/Paused 零 viewport 与 missing measurement 均有闭合结果 |
| 错误与失败路径 | covered: B-007、B-008、B-011、B-012、B-013、B-014、B-017 |
| 授权/权限 | N/A：MessageList 不读取权限、不执行工具、不访问文件/网络；renderer 只由调用方提供 |
| 并发/竞态 | covered: B-003、B-012、B-014；独立实例互不影响，旧 revision fail closed |
| 重试/幂等 | covered: B-003、B-011、B-012、B-014；no-op/retry 不重复推进 revision 或重测 unchanged key |
| 非法状态转换 | covered: B-007、B-008、B-009、B-013、B-014；非法导航/insert/follow/revision 转换均 typed 拒绝 |
| 兼容/迁移 | covered: B-022、B-023；fixed-height API 不变，production availability 等待完整依赖 |
| 降级/回退 | covered: B-004、B-012、B-013、B-015、B-017；禁止默认高度、单块高度或 partial frame |
| 证据与审计完整性 | covered: B-020、B-021、B-023、B-024 |
| 取消/中断 | covered: B-012、B-013、B-017；测量取消和 render failure 均零提交 |

## 发布说明

这是新增且 opt-in 的 `MessageList` API。首次发布应说明：offset/height 单位为终端行；
调用方必须提供 stable message identity、exact content revision、variant/expansion identity
和 checked TextFlow measurement；用户离开底部后应依据 `new_content_below` 呈现提示。
现有 `virtual_scroll_view` 无迁移要求。回滚通过普通 revert 移除新增 API，不回退 GH-58、
GH-60、GH-62 的 correctness 合同，也不 force push。
