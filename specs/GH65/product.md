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
2. **B-002 — Exact measurement identity.** 一条已测量高度只可由完全相等的
   `(message_id, width, content_revision, variant, expansion)` key 命中；任一字段改变都不能
   复用旧高度，hash collision 不能被视为 key 相等。
3. **B-003 — Caller ownership and deterministic state.** State 与 measurement cache 由调用方
   持有，不使用 process-global cache；相同初始状态、输入序列和测量结果产生相同顺序、高度、
   anchor、visible slices、follow state 与 revision。
4. **B-004 — Empty and zero-sized viewport.** 空列表产生零 content rows、零 offset、无 anchor
   和空 visible range。viewport rows 为零时不渲染消息；width 为零若无法测量则返回 typed
   error，不能猜测默认高度。
5. **B-005 — Row-accurate partial visibility.** 可见结果按消息顺序返回；每项同时包含稳定
   `message_id`、message-local row range 和 viewport-local row range。第一条和最后一条消息
   可为 partial range，所有 range 均为半开区间且不越过已测量高度。
6. **B-006 — Exact prepend anchor.** 非空 viewport 前 prepend 任意数量消息后，若原 top
   message 仍存在，则其 `message_id` 与原 top intra-message row 必须保持在 viewport 顶部；
   新消息不得把正在阅读历史的用户推离原内容。
7. **B-007 — Mutations preserve a surviving anchor.** Insert、append、content update、
   streaming delta、resize、variant change、expand/collapse 和其他高度变化后，原 anchor
   message 若仍存在就保持同一 intra-message row；若新高度更短则只允许 clamp 到该消息最后
   一个有效 row，并在 update result 中报告发生了 clamp。
8. **B-008 — Deleted-anchor rule.** 删除 anchor message 时，优先锚定删除前顺序中的下一条
   surviving message 的首行；若没有下一条则锚定上一条 surviving message 的末行；列表变空
   时 anchor 变为 `None`、offset 归零。未知 ID 删除必须 typed 失败且不改变状态。
9. **B-009 — Explicit bottom-follow state.** Follow state 只有可区分的 `Following` 与
   `Paused { new_content_below }`。用户从底部向上滚动后进入 Paused；只有显式滚动到新底部或
   jump-to-bottom 才恢复 Following 并清除提示，resize/delete/collapse 不得隐式恢复。
10. **B-010 — Follow-bottom append and growth.** Following 时 append 或最后消息 streaming
    growth 后，viewport 继续贴住新的最大 offset。Paused 时这些变化保留 anchor；当变化在
    原 viewport 下方新增可见内容时 `new_content_below=true`，并持续到用户返回底部。
11. **B-011 — Update and invalidation semantics.** 同一 stable ID 的 content revision、
    variant 或 expansion 改变会触发新 exact measurement；width 改变为所有消息建立新宽度
    的测量视图。未改变 exact key 的 prepend、append、insert、delete 或重排可复用缓存，不得
    因结构变化重新测量全部消息。
12. **B-012 — Atomic measurement failure.** 一次 mutation 所需的任一测量返回 missing、
    failure 或 cancellation 时，整个 mutation 失败；消息顺序、高度索引、cache、anchor、
    follow/new-content state 和 state revision 与调用前逐字段相同。
13. **B-013 — Typed fail-loud errors.** Duplicate/unknown message ID、missing measurement、
    zero row height、stale measurement/revision、row arithmetic overflow、coordinate
    conversion overflow、measurement failure 和 cancellation 都保留独立 typed category
    及可用 source；不得返回 `Any`、仅字符串错误、warning+fallback 或默认 `1 row`。
14. **B-014 — Stale-result rejection.** State 使用单一 checked revision；每次成功的可观察
    mutation 恰好递增一次，无变化或失败不递增。针对旧 revision/key 返回的异步或延迟测量
    结果必须 typed 拒绝，不能覆盖当前高度。
15. **B-015 — Single TextFlow measurement authority.** 消息终端行数由 GH-58 的
    TextFlow/row count（或其合并后的等价 checked API）产生；MessageList 不实现第二套 wrap、
    Unicode width、tab 或 source-to-cell 算法。
16. **B-016 — Renderer-independent index.** 高度索引只依赖稳定 ID、revision、width 和显式
    variant/expansion identity；它不依赖 GH-63 的 `ChatMessageView` 或 block 类型。
    MessageList 通过 typed render closure 把 message entry 与 exact visible slice 交给调用方。
17. **B-017 — Render failure propagation.** Render closure 对任一 visible slice 失败时，
    MessageList 返回保留原 source 的 typed render error；不跳过该消息、不输出 partial/default
    frame，也不修改 caller-owned state。
18. **B-018 — Logarithmic lookup and point update.** 对已测量的 `n` 条消息，从全局 row offset
    定位首条可见消息为 `O(log n)`；单条高度/content streaming delta 的索引更新为
    `O(log n)`。构造 `k` 条 visible slices 为 `O(log n + k)`，每帧不得全表扫描或全表重测。
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
23. **B-023 — Dependency and implementation gate.** Spec 可以先合入；生产实现只有在 GH-58、
    GH-60、GH-62 的 implementation PR 已合并并验证 ancestry 后开始。GH-63 不是高度索引依赖，
    若其实现已合并，只通过 render closure 集成。
24. **B-024 — Evidence-bound completion.** 完成结论必须绑定 implementation PR 当前 exact
    head，包含全部 exact tests、property seed、10k benchmark、full Rust gates、changed-line
    coverage（新代码至少 80%，anchor/cache/error 关键路径 100%）、CI/review threads 和人工
    merge gate；旧 SHA 或仅列出测试不能作为通过证据。

## 验收标准

- [ ] 公开 state/API 使用 GH-62 stable ID/revision，并以 row units 表达 viewport、offset、
  anchor 与 partial visible ranges。
- [ ] Cache key 精确包含 `message_id + width + content_revision + variant + expansion`，所有
  invalidation、reuse 和 stale-result 行为均有 exact tests。
- [ ] Prepend 保持 top ID + intra-row；append/stream/resize/expand/collapse/delete 与 clamp
  规则均有确定 fixture。
- [ ] Following/Paused/new-content 状态转换完整覆盖用户 scroll、jump-to-bottom、append、
  streaming、resize、collapse 与 delete。
- [ ] Missing/unknown/duplicate/zero-height/overflow/stale/failure/cancellation 都 typed、
  fail-loud、原子且保留 source，无默认高度或 partial commit。
- [ ] 固定 seed property oracle、确定性复杂度门禁和 10k benchmark 在当前 head 实际运行。
- [ ] GH-63 只经 render closure 消费；render failure typed 传播且不输出 partial/default frame。
- [ ] 既有 `virtual_scroll_view` 兼容 fixture 与 crate 外 public API fixture 通过。
- [ ] Implementation 开始前验证 GH-58/GH-60/GH-62 merged ancestry；最终证据满足 B-024。

## 边界情况

- **Happy path：** mixed-height 消息可按 row offset 精确定位，首尾 partial slice 正确。
- **Empty：** 空列表、空 append/prepend、删除最后一条、零 viewport 均有确定无 panic 结果。
- **Error：** duplicate/unknown ID、missing/zero/stale measurement 与 overflow typed 返回且零
  mutation。
- **Loading：** 尚未取得所需 measurement 视为 typed missing，不以 placeholder/default
  高度伪装成功；调用方可保留原 frame 后重试。
- **Cancellation：** measurement cancellation 原样传播，candidate/index/cache/revision 不提交。
- **Permission：** 组件不访问文件、终端权限或远端服务；无权限分支不适用。
- **Offline/network failure：** 组件不发网络请求；provider 离线失败由调用方处理，若导致缺少
  measurement 则遵循 typed missing + atomic failure。
- **Concurrency/race：** state 单一 mutable owner；延迟结果携带 exact state revision/key，
  stale 结果拒绝，不能 last-write-wins 覆盖新内容。
- **Compatibility：** fixed-height virtual scroll 不变；旧 API 不强制迁移。
- **Accessibility：** 可见输出保持 logical message order 与稳定 ID，供上层 focus/
  announcement 使用；本 issue 不定义 screen-reader UI。

## 发布说明

这是新增且 opt-in 的 `MessageList` API。首次发布应说明：offset/height 单位为终端行；
调用方必须提供 stable message identity、exact content revision、variant/expansion identity
和 checked TextFlow measurement；用户离开底部后应依据 `new_content_below` 呈现提示。
现有 `virtual_scroll_view` 无迁移要求。回滚通过普通 revert 移除新增 API，不回退 GH-58、
GH-60、GH-62 的 correctness 合同，也不 force push。
