# Product Spec：固定底部区域的 FullscreenChatShell

## Linked Issue

GH-67: https://github.com/majiayu000/rnk/issues/67

complexity: large

## 用户问题

当前 fullscreen chat example 自行维护消息类型、按消息条数计算的滚动、单行输入、footer、
resize 和退出逻辑。通用 `fixed_bottom_layout` 只提供 flex 组合，不能证明可变高度 transcript
的真实 terminal-row offset、MessageList 锚点、Composer auto-grow、overlay 焦点或失败时的
frame 原子性。现有 input、paste 和 mouse hooks 还会广播给全部 handler；若应用同时注册
transcript、composer 和 overlay handler，同一事件可能被多次消费。

用户需要一个后端无关、受控且可测试的 `FullscreenChatShell`：它只组合最终合并的
Conversation、`ChatMessageView`、`ChatComposer` 与 `MessageList` 公共合同，拥有完整可见
frame 与 alternate-screen 生命周期，并为布局、焦点、路由、resize 和退出恢复提供单一真相。

## 目标

- transcript 精确占用扣除 Composer 与可选 status 后的剩余 terminal rows。
- Composer/status 始终固定在底部；支持尺寸内不重叠，零/过小尺寸 typed 失败。
- 直接消费 GH-65 的 row-based MessageList、height invalidation、anchor、follow 与提示状态，
  不再实现第二套高度或 item-count 滚动。
- 提供 transcript/composer/overlay 的 caller-owned focus、嵌套 overlay 栈和唯一事件路由器。
- 对 resize、streaming、prepend、Composer reflow 和 overlay 变化按到达顺序原子发布 frame。
- 正常、取消、typed failure、panic/unwind 和 suspend/resume 后恢复 raw mode、cursor、
  alternate screen、mouse/focus/paste 输入模式。
- 手工迁移 `examples/rnk_chat.rs`，只组合公开 shell 与公开子组件。

## 非目标

- 不写入终端原生 scrollback；这是 Inline shell 的不同生命周期。
- 不拥有 provider 请求、鉴权、密钥、工具执行、权限判断、重试或会话持久化。
- 不复制 Conversation reducer、ChatMessageView block dispatch、Composer TextFlow 或
  MessageList height/anchor 算法。
- 不实现 terminal 未提供的 native IME preedit/candidate UI；只路由 committed text。
- 不把任意 `Element`、调试字符串、`Any`、provider JSON 或未声明 map 当作状态/错误协议。
- 不以可选视觉降级掩盖布局、消息顺序、锚点、重复输入、渲染或终端恢复失败。

## Behavior Invariants

1. **B-001 — Controlled public shell.** 应用必须能从公开
   `FullscreenChatShellConfig`、`FullscreenChatShellState`、`FullscreenChatShell`、
   `FullscreenShellEvent`、`FullscreenShellObservation` 和纯/事务 handler 构造 shell；
   所有可变状态由 caller 串行持有，shell 不创建隐藏 global conversation、composer、
   message-list、focus 或 overlay state。
2. **B-002 — Complete initial inputs.** 构造必须显式接收完整初始 message entries、
   GH-65 measurement config/callback、Composer state/projection、terminal size、初始 focus
   与空或非空 overlay 栈。空 transcript 与 status 缺失是有效显式输入；缺失 measurement、
   config、projection 或 active overlay body 必须 typed 失败，不能假设一行、默认 width、
   空 element 或旧 frame。
3. **B-003 — Exact remaining-height partition.** 每个成功 frame 的 transcript rows 必须
   精确等于 `terminal_rows - composer_rows - status_rows`；三个区域使用同一 checked
   partition，按 transcript→composer→status 顺序连续排列，无 gap、overlap 或越界。
4. **B-004 — Supported minimum and undersized terminals.** config 的 min columns、
   min transcript rows、min/max composer rows 与可选 status rows 均须 validated。
   `columns=0`、`rows=0`、任一 checked 加减溢出或无法同时容纳最小区域时返回 closed typed
   error，且不调用 measurement/render callback、不修改 state、不发布空白/截断成功 frame。
5. **B-005 — Optional status semantics.** status 缺失时占用零 rows、没有 status child，也不
   伪造 model、连接、token 或成功状态；存在时使用调用方提供的非零 rows/structured element，
   始终位于 Composer 下方且其语义可由 public observation/accessibility fallback 读取。
6. **B-006 — Composer auto-grow without overlap.** Composer rows 只来自 exact-current
   GH-64 projection，并 clamp 到 validated min/max 及“至少保留 min transcript/status”的
   当前上限；clamp 结果必须公开可观察。resize/reflow 后 cursor/selection/draft identity
   由同一 Composer projection 保持，shell 不按字符串或 logical line 重算高度。
7. **B-007 — Row-based transcript only.** shell 必须以 terminal row 为 viewport、offset、
   slice 和 anchor 单位，直接使用唯一稳定 GH-65 MessageList facade/render closure；不得调用
   `virtual_scroll_view`、`.skip().take()`、message count、局部 prefix sum 或 duplicate height
   cache。
8. **B-008 — Exact height invalidation.** width、`MessageRevision`、view variant、Thinking
   disclosure、ToolResult preview/expansion 或完整 measurement config 改变时，shell 必须把
   typed change 交给 GH-65 的完整 cache identity/measurement transaction；无关 message 的
   active shared key handles 保持 O(1) identity，不深拷贝正文。
9. **B-009 — Following behavior.** MessageList observation 为 `Following` 时，append 或 active
   stream 增长使 viewport 跟随最新 bottom，`new_content_below=false`；Composer/status/overlay
   变化不改变该状态。零-row 中间 resize 仍按 GH-65 logical-bottom 合同保留 Following。
10. **B-010 — Paused behavior and new-content signal.** 用户显式滚离 bottom 后保持
    `Paused` 与 stored anchor；下方 append/stream 增长只设置可观察的 new-content 状态，
    不 bottom-jump。只有显式 jump/scroll 到 bottom 才恢复 Following；提示不能只靠颜色。
11. **B-011 — Prepend anchor.** prepend 任意数量/高度历史后，原 visible top 的 stable
    message identity 与 intra-message row 保持；短内容/高度缩短产生 GH-65 明确 clamp flag，
    不把同一个全局 row offset 当成锚点。
12. **B-012 — Resize and reflow.** 每次 width/height resize 都在同一 candidate frame 重建
    Composer projection、MessageList measurement config、region partition 与 visible slices；
    成功后 focus、draft、selection、anchor/follow 和 overlay 栈保持合同，旧 width projection
    不得被发布成新 frame。
13. **B-013 — Typed message content.** transcript render closure 必须通过最终 GH-63
    `ChatMessageView` 公开 borrowed path 保留 source order，并覆盖单/多行 Text、Markdown、
    Code、Thinking 与 ToolResult；shell 不 wildcard-ignore block、不 clone whole payload、
    不把 render failure 变成普通 Text。
14. **B-014 — Public-only example.** `examples/rnk_chat.rs` 只能组合 public Conversation、
    `ChatMessageView`、`ChatComposer`、`MessageList` 与 `FullscreenChatShell`；不得保留私有
    role/message、字符 editor、message-height、item scroll、focus/router、resize 或 terminal
    cleanup state machine，也不得直接输出 ANSI。
15. **B-015 — Observable focus state.** focus target 是闭集 Transcript、Composer 或
    `Overlay(OverlayId)`；初始 target 必须存在且可 focus，成功变化恰好推进一次 checked shell
    revision。public observation 必须同时报告 target、scope、区域 rect、follow/new-content、
    Composer clamp 与 top overlay，供测试/辅助技术观察，而非仅改变边框颜色。
16. **B-016 — Deterministic key precedence.** Resize/shutdown 等 session event 先于普通输入；
    top modal overlay 捕获后，Escape/overlay handler 先于 global focus keys；否则 Tab/BackTab
    先改变 shell focus，再只把事件交给 focused Composer 或 Transcript。每个 key 最多调用一个
    component handler一次；`Ignored`、`Handled`、`Changed`、`Submitted`、`Cancelled`
    使用既有 `InteractionOutcome<T>` 语义，不能靠 hook 注册顺序决定。
17. **B-017 — Nested overlay stack and z-order.** overlay 以 validated stable ID、closed
    capture policy、checked rect/body 和进入前 focus 构成 LIFO stack；base frame 先绘制，
    overlay 按栈顺序绘制且 topmost 最后、clip 到 terminal rect。重复 ID、body 缺失、越界
    算术或关闭非 top overlay typed 失败且 full state/frame 不变。
18. **B-018 — Overlay close and focus restoration.** top dismissible overlay 的 Escape 只关闭
    一层并恢复该层保存的仍有效 focus；嵌套关闭逐层恢复。modal overlay 对未处理 key/paste/
    mouse 仍返回 consumed，不允许穿透 Composer/Transcript；最后一层关闭后才恢复 base focus。
19. **B-019 — Committed text and paste exactly once.** multi-scalar committed input、
    CJK、emoji、combining、ZWJ 与 tab 只送给 focused Composer 的 GH-64 key ingress一次；
    `Event::Paste` 只送 paste ingress一次且绝不再作为 key/submit。ESC/C0/C1、CRLF 与失败的
    typed semantics 完全由 Composer 保留；shell 不 trim、删 control 或部分写草稿。
20. **B-020 — Mouse hit testing and conflicts.** mouse 使用已提交的 checked region rect，
    先按 top overlay→lower overlay→status→composer→transcript z-order hit-test；一次事件只到
    一个 target。resize 后旧坐标、边界外坐标、scroll wheel/drag/key 同时到达均按串行事件
    顺序处理，不重复滚动、不越界、不让 passive status 抢占 focus。
21. **B-021 — Rapid event ordering and revision guard.** resize、conversation outcome、
    stream growth、prepend、Composer projection、overlay 与 input 按 shell 收到的顺序串行；
    每项带 expected shell revision，stale event typed 失败。相同初态和相同事件序列产生相同
    observation、visible slices、focus 与 frame；不得用全量重建/重排掩盖 update 顺序。
22. **B-022 — Checked coordinates and fail-atomic state.** row/column、rect end、region sum、
    list offset、revision 与 event sequence 只用 checked conversion/arithmetic；overflow、
    stale revision、unknown ID、invalid anchor、missing measurement 或 Composer/List failure
    返回具体 closed typed error。所有 callbacks、list/composer candidates 与 focus changes
    通过后才一次 commit；失败前后 state/observation 相等。
23. **B-023 — Atomic layout/render publication.** shell candidate 必须经最终 GH-60 checked
    layout、required-layout 和 staged renderer成功后才替换 committed observation/frame；
    layout、TextFlow、missing layout、coordinate、clip、renderer callback 或 injected failure
    均保留旧 frame/state并携带原 typed source。不得 `unwrap_or_default`、panic、warning +
    fallback、部分 output commit 或 catch panic 后显示旧/空内容为成功。
24. **B-024 — Fullscreen terminal restoration.** public fullscreen session 在 normal exit、
    cancel、typed shell/render failure 与 panic/unwind 四条路径都必须恢复进入前 screen、
    raw mode、cursor visibility、alternate screen、mouse capture、focus reporting 与 bracketed
    paste；每条路径由本 issue 自身 PTY/fake-terminal evidence 覆盖。cleanup 某一步失败必须
    聚合为 typed terminal restoration failure，不能被 Drop 或 warning 静默吞掉。
25. **B-025 — Suspend, restart and explicit capability boundary.** suspend 前恢复 terminal，
    resume 后重新进入 fullscreen 并强制从显式 state 生成完整新 frame；fresh session 只从
    constructor inputs 重建，不继承旧 overlay/focus/frame/measurement handle。不可用的可选
    focus/mouse/paste capability可进入文档化显式状态；terminal/state 恢复不确定不得宣称成功。
26. **B-026 — Accessibility and non-color semantics.** transcript 暴露 Viewport、Composer 暴露
    TextArea、status 暴露 Status、modal overlay 暴露 Dialog semantics；label/value/description、
    focus、paused/new-content、submitting/failed/cancelled 必须能通过 `accessible_text()` 或
    public observation区分，ANSI 去色后仍保留相同语义顺序与错误。
27. **B-027 — Bounded work and lightweight handles.** steady frame 只遍历 GH-65 返回的 visible
    slices，MessageList lookup/slice 保持批准复杂度；shell 对 active measurement key handle
    只做 O(1) clone并确保其在可见 frame 完成前不可被 cache eviction 失效。禁止 clone 全量
    conversation、全量 payload/Element 历史或按 transcript 长度扫描来处理一个 key/resize。
28. **B-028 — Provider/tool security boundary.** shell 只显示应用已提交的 typed state；不读取
    env/secret、不调用 process/network、不执行 Tool Call、不从 block/overlay 文本推导授权。
    缺数据保持空白/typed pending，不能伪造模型、连接、权限或工具成功。
29. **B-029 — Dependency and drift gate.** implementation 必须等待 #62/#63/#64/#65 全部
    CLOSED、各自 final implementation PR/evidence merged，且完整 merge set 是 implementation
    base 的祖先；还必须证明 GH-65 的传递依赖完成。spec-only、open、parked、cap-exhausted
    review 或部分修复不满足。若最终 public API/manifest 与本 packet 漂移，先更新并重新 review
    GH-67，禁止 alias、private field hack、sidecar cache 或复制未解决缺陷。
30. **B-030 — Current-head evidence.** 完成声明必须绑定 implementation PR exact head：
    Product-to-Test Mapping 每个 exact test matched=passed=1、ignored=0；plain/ANSI golden
    不在测试中更新；changed executable coverage ≥80%，committed
    `gh57-critical-paths-v1` 的 exact `file+name+command` 集合逐项 100%，producer/validator
    可从 raw coverage、diff 和 ledger 确定性重算。fresh fmt/check/clippy/all-target tests、
    example、PTY、CI、独立 review、reviewThreads 与 SpecRail PR gate 缺一不可。

## 验收标准

- [ ] 支持尺寸内 partition 等式成立；零、窄、矮、Composer min/max、status absent/present、
      nested overlay 均有 typed 正负证据。
- [ ] GH-65 Following/Paused、new-content、prepend、stream growth、expand/collapse 与
      continuous resize在 shell 序列中保持相同 public observation。
- [ ] Text/Markdown/Code/Thinking/ToolResult 单/多行 render、Composer committed text/paste、
      transcript/composer/overlay focus/key/mouse 冲突均 exact-once。
- [ ] layout/render failure 前后 state/frame相等；normal/cancel/error/panic 与 suspend/resume
      都恢复 raw/cursor/alternate-screen/mouse/focus/paste。
- [ ] `rnk_chat` 只使用 public shell；plain/ANSI golden 语义等价且 accessibility fallback
      不依赖颜色。
- [ ] dependency final merged ancestry、fresh full tests、current-head coverage、CI/review/gate
      全部通过；本 spec PR 只包含 `specs/GH67/{product,tech,tasks}.md`。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-002, B-004, B-005, B-013, B-028 |
| 错误与失败路径 | covered: B-004, B-017, B-022, B-023, B-024, B-025 |
| 授权/权限 | covered: B-028；shell 不执行工具或推导授权 |
| 并发/竞态 | covered: B-012, B-020, B-021, B-022 |
| 重试/幂等 | covered: B-018, B-021, B-023, B-025 |
| 非法状态转换 | covered: B-015, B-017, B-018, B-021, B-022 |
| 兼容/迁移 | covered: B-007, B-014, B-029 |
| 降级/回退 | covered: B-023, B-025, B-026, B-028 |
| 证据与审计完整性 | covered: B-029, B-030 |
| 取消/中断 | covered: B-016, B-019, B-024, B-025 |

## 发布说明

这是新增、opt-in 的 Fullscreen Chat API。发布说明必须明确：Fullscreen 拥有 alternate-screen
可见 transcript，不写 native scrollback；terminal offset/height 单位是 visual rows；
Composer/status 为 fixed-bottom；overlay 由 shell 唯一路由；只有 committed IME text；
provider/tool side effects 仍由应用负责。API 稳定级别必须以最终 GH-62～65 public contracts
和 GH-68 hardening evidence为准，不能因 example 可运行就提前标记 stable。
