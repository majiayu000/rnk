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
- 只有在 MessageList 与 Composer 提供已合并、先 prepare 后无失败 commit/abort 的
  candidate capability 后，才允许开始实现跨组件原子 frame。
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
   `FullscreenChatShellConfig`、`FullscreenChatStateBundle`、`FullscreenChatShell`、
   `FullscreenShellEvent`、`FullscreenSessionCommand`、`FullscreenShellObservation` 和
   纯/事务 handler 构造 shell。bundle由caller串行拥有且精确包含同一组 shell、
   MessageList 与 Composer live states；只提供只读accessor和受控的内部disjoint borrow，
   禁止caller复制/重建component state、取得平行 `&mut` 或创建隐藏 global state。
2. **B-002 — Complete initial inputs.** bundle构造必须显式接收完整初始 message entries、
   GH-65 measurement config/callback、Composer state与projection inputs、terminal size、初始
   focus与空或非空overlay栈，并返回实际创建的同一MessageList/Composer/shell states。空
   transcript与status缺失是有效显式输入；缺失measurement、config、projection inputs或active
   overlay body必须typed失败，不能假设一行、默认width、空element或旧frame。
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
6. **B-006 — Composer auto-grow without overlap.** shell必须先由terminal/config/status用
   checked arithmetic计算 `composer_cap`，再把当前width与该cap作为GH-64 projection input，
   生成包含cursor的exact-current visible window；`composer_rows`只能取该新projection高度并
   clamp到validated min与cap。禁止先用旧/较大cap投影再裁rect。clamp、visible range与cursor
   必须公开可观察；shell不按字符串或logical line重算高度。
7. **B-007 — Row-based transcript only.** shell 必须以 terminal row 为 viewport、offset、
   slice 和 anchor 单位，直接使用唯一稳定 GH-65 MessageList facade/render closure；不得调用
   `virtual_scroll_view`、`.skip().take()`、message count、局部 prefix sum 或 duplicate height
   cache。
8. **B-008 — Exact height invalidation.** width、`MessageRevision`、view variant、Thinking
   disclosure、ToolResult preview/expansion 或完整 measurement config 改变时，shell 必须把
   typed change 交给 GH-65 的完整 cache identity/measurement transaction；无关 message 的
   active shared key handles 保持 O(1) identity，不深拷贝正文。
9. **B-009 — Following behavior in a supported shell viewport.** MessageList observation 为
   `Following` 时，append 或 active stream 增长使 viewport 跟随最新 bottom，
   `new_content_below=false`；Composer/status/overlay 变化不改变该状态。成功 shell frame
   的 transcript rows 始终非零；zero-row/undersized terminal 按 B-004 在调用 MessageList
   前失败。GH-65 自身的 zero-row logical-bottom 行为只由 GH-65 component contract验证，
   不是 GH-67 shell 的可达成功状态。
10. **B-010 — Paused behavior and new-content signal.** 用户显式滚离 bottom 后保持
    `Paused` 与 stored anchor；下方 append/stream 增长只设置可观察的 new-content 状态，
    不 bottom-jump。只有显式 jump/scroll 到 bottom 才恢复 Following；提示不能只靠颜色。
11. **B-011 — Prepend anchor.** prepend 任意数量/高度历史后，原 visible top 的 stable
    message identity 与 intra-message row 保持；短内容/高度缩短产生 GH-65 明确 clamp flag，
    不把同一个全局 row offset 当成锚点。
12. **B-012 — Resize and reflow.** 每次 width/height resize 都在同一 candidate frame先重算
    `composer_cap`，再以该cap重建cursor-containing Composer projection、MessageList
    measurement config、region partition与visible slices；成功后focus、draft、selection、
    anchor/follow和overlay栈保持合同，旧width或旧cap projection不得被发布成新frame。
13. **B-013 — Typed message content.** transcript render closure 必须通过最终 GH-63
    `ChatMessageView` 公开 borrowed path 保留 source order，并覆盖单/多行 Text、Markdown、
    Code、Thinking 与 ToolResult；shell 不 wildcard-ignore block、不 clone whole payload、
    不把 render failure 变成普通 Text。
14. **B-014 — Public-only example.** `examples/rnk_chat.rs` 只能组合 public Conversation、
    `ChatMessageView`、`ChatComposer`、`MessageList` 与 `FullscreenChatShell`；不得保留私有
    role/message、字符 editor、message-height、item scroll、focus/router、resize 或 terminal
    cleanup state machine，也不得直接输出 ANSI。
15. **B-015 — Observable and valid focus state.** focus target 是闭集 Transcript、Composer
    或 `Overlay(OverlayId)`；只有存在且 kind 为 Modal/Pointer 的 focusable overlay 可成为
    Overlay target，Passive overlay 永远不可 focus。初始 target 必须存在且可 focus，成功变化
    恰好推进一次 checked shell revision。public observation 必须同时报告 target、scope、
    区域 rect、follow/new-content、Composer clamp 与完整 top overlay state，供测试/辅助技术
    观察，而非仅改变边框颜色。
16. **B-016 — Total deterministic key/paste precedence.** shell event与session command是
    disjoint closed domains；唯一session dispatch先处理Suspend/Resume/Shutdown，shell handler
    永远收不到它们。top Modal对所有key/paste有唯一捕获结果；无Modal时Tab的focus ring固定为
    Transcript→Composer→focusable Pointer overlays bottom→top，BackTab严格反向；从current后
    一项开始并在两端wrap，Passive/不可focus/closed ID跳过或typed失败。一次输入最多调用一个
    shell/component/overlay handler一次，不依赖hook注册顺序。
17. **B-017 — Nested overlay state and z-order.** overlay 以 validated stable ID、closed
    `OverlayKind`（Modal/Pointer/Passive）、focusability、checked rect/body、handler capability
    和进入前 focus 构成 LIFO stack；base frame 先绘制，overlay 按栈顺序绘制且 topmost 最后、
    clip 到 terminal rect。Passive 永不接收输入，Pointer 只在命中或显式 focus时接收，Modal
    独占输入。重复 ID、非法 kind/focus 组合、body/handler缺失、越界算术或关闭非 top overlay
    typed 失败且 full state/frame 不变。
18. **B-018 — Overlay close and focus restoration.** top dismissible overlay 的 Escape 只关闭
    一层并恢复该层保存的仍有效 focus；嵌套关闭逐层恢复。Modal 对未处理 key/paste/mouse
    仍 consumed；Pointer 一旦命中或持有 focus也不向 lower layer二次派发；Passive 始终
    fall through且不抢 focus。最后一层关闭后才恢复 base focus。
19. **B-019 — Committed text and paste exactly once.** multi-scalar committed input、
    CJK、emoji、combining、ZWJ 与 tab 只送给 focused Composer 的 GH-64 key ingress一次；
    `Event::Paste` 只送 paste ingress一次且绝不再作为 key/submit。ESC/C0/C1、CRLF 与失败的
    typed semantics 完全由 Composer 保留；shell 不 trim、删 control 或部分写草稿。
20. **B-020 — Total mouse hit testing and fallthrough.** mouse 使用已提交的 checked region
    rect，按 top→lower overlay 后再 status→composer→transcript扫描。Modal 无论命中与否
    独占；Pointer miss继续扫描、hit后只调用该 overlay一次并停止；Passive 总是跳过；status
    consumed但不抢 focus；Composer/Transcript 的 press、wheel、release、drag、move 与
    terminal 外坐标均有唯一 handler/outcome。resize 后旧坐标或 mouse/key同时到达按串行事件
    顺序处理，不重复滚动、不越界。
21. **B-021 — Rapid event ordering and revision guard.** resize、conversation outcome、
    stream growth、prepend、Composer projection、overlay 与 input 按 shell 收到的顺序串行；
    每项带 expected shell revision，stale event typed 失败。相同初态和相同事件序列产生相同
    observation、visible slices、focus 与 frame；不得用全量重建/重排掩盖 update 顺序。
22. **B-022 — Checked coordinates and realizable fail-atomic state.** row/column、rect end、
    region sum、list offset、revision 与 event sequence 只用 checked conversion/arithmetic；
    overflow、stale revision、unknown ID、invalid anchor、missing measurement 或
    Composer/List prepare failure返回具体 closed typed error。MessageList 与 Composer
    必须先提供不修改 live state 的 prepared candidate；measurement/layout/render全部成功后，
    只允许执行无 callback、无 allocation、无失败分支的 infallible commit。任一 prepare 或
    render失败时显式 abort/discard全部 candidates，state/observation/frame逐值相等。
23. **B-023 — Atomic layout/render publication and complete failure evidence.** shell
    candidate 必须经最终 GH-60 checked layout、required-layout 和 staged renderer成功后才
    替换 committed observation/frame；layout、TextFlow、missing layout、coordinate、clip、
    renderer callback 或 injected failure均保留旧 frame/state并携带原 typed source。不得
    `unwrap_or_default`、panic、warning + fallback、部分 output commit 或 catch panic 后显示
    旧/空内容为成功。若 primary layout/render error 与 terminal cleanup error 同时发生，
    top-level结果必须同时保留 primary typed source和全部 cleanup step failures，不能用后者
    覆盖前者或只保留第一项。
24. **B-024 — Concrete fullscreen session and restoration.** 应用必须能通过公开 session
    config、backend/capability、可验证snapshot evidence、constructor、dispatch/run/render、
    shutdown/recovery API进入fullscreen。native首次snapshot必须来自配对controlling TTY的
    termios及成功关联的DECRQM replies，精确区分47/1047/1049 screen、25 cursor、
    1000/1002/1003 tracking、1015 RXVT、1006 SGR、1004 focus与2004 paste；每个mode逐bit
    保存/恢复；非规范reply、timeout/
    mismatch在任何mutation前typed失败。现有public `Terminal`/`App`、terminal controller与
    panic recovery也必须在首次mutation前取得同一process-wide lease。partial enter反向回滚；
    若任一restore/flush/
    release失败，唯一recovery owner与Poisoned registry必须保留backend、lease、snapshot及全部
    unfinished steps，直到显式retry全部成功；snapshot读取前失败则用`None` snapshot的
    lease-only owner重试release，禁止丢error后让第二session进入。
25. **B-025 — Suspend, resume, restart and capability boundary.** suspend停止事件intake并
    恢复当前pre-entry snapshot；restore+flush+release全部成功才进入Suspended。resume重新获取
    exclusive lease、生成/验证新snapshot并读取新size，以bundle prepared candidate执行cap-first
    synthetic Resize/reflow，再按staged enter/render/commit；不得重绘suspend前旧frame。
    若rollback/release完整则保持Suspended，任一步不完整则进入唯一
    `RecoveryRequired`并继续拥有lease/poison guard，不能谎称Suspended。Shutdown、panic与Drop
    使用同一unfinished-step表；Drop失败把所有权转移到process registry的typed poisoned record，
    后续只能由显式recovery claim接管。fresh session只从bundle constructor inputs重建。
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
29. **B-029 — Dependency, capability and drift gate.** implementation 必须等待
    #62/#63/#64/#65 全部 CLOSED、各自 final implementation PR/evidence merged，且完整 merge
    set 是 implementation base 的祖先；还必须证明 GH-65 的传递依赖完成、GH-65 packet三路径
    在该base真实存在，以及最终 GH-64/GH-65 public API提供 B-022 所需
    prepare/view/infallible-commit/abort capability。当前base不存在的 GH-65 paths不得进入
    `spec_refs`。spec-only、open、parked、cap-exhausted review、立即commit API或部分修复不
    满足。若最终 public API/manifest 与本 packet漂移，先把真实 GH-65 refs/API加入并重新
    review GH-67，禁止 alias、private field hack、全量clone rollback、sidecar cache或复制
    未解决缺陷。
30. **B-030 — Current-head and reproducible evidence.** 完成声明必须绑定 implementation PR
    exact head：Product-to-Test Mapping 每个 exact test matched=passed=1、ignored=0；
    plain/ANSI golden不在测试中更新；changed executable coverage ≥80%，committed
    `gh57-critical-paths-v1` 的 exact `file+name+command` 集合逐项 100%，producer/validator
    可从 raw coverage、diff 和 ledger 确定性重算。SpecRail packet验证必须声明可获取的
    repository URL、immutable commit、checkout步骤与checker checksum；mirror必须从exact
    reviewed rnk head复制manifest声明的GH57/GH62/GH63/GH64/GH67全部15个refs，并逐文件断言
    existence与source/mirror SHA相等，不能使用SpecRail仓库自带的drifted/missing copies。
    coverage validate后必须export validate mode、raw/artifact absolute paths与全部immutable
    variables贯穿mapped/ledger/full workspace tests，窗口末尾重新核对head/base/merge-base/
    clean worktree。fresh fmt/check/clippy/all-target tests、example、PTY、CI、独立review、
    reviewThreads与SpecRail PR gate缺一不可。

## 验收标准

- [ ] 支持尺寸内 partition 等式成立；零、窄、矮、Composer min/max、status absent/present、
      nested overlay 均有 typed 正负证据。
- [ ] GH-65 Following/Paused、new-content、prepend、stream growth、expand/collapse 与
      continuous supported resize在 shell 序列中保持相同 public observation；zero/
      undersized resize在调用 MessageList前typed失败。
- [ ] Text/Markdown/Code/Thinking/ToolResult 单/多行 render、Composer committed text/paste、
      transcript/composer/overlay focus/key/mouse 冲突均 exact-once。
- [ ] owning bundle中的List/Composer/shell revisions始终来自同一constructor与transaction；
      upstream prepare后任一late layout/render failure前后bundle/frame相等。
- [ ] primary+多个cleanup source完整；normal/cancel/error/panic、partial enter与
      suspend/resume要么恢复exact snapshot并释放lease，要么由唯一Poisoned recovery owner继续持有。
- [ ] Modal/Pointer/Passive × focus × key/paste/mouse/fallthrough矩阵逐格只有一个目标与
      明确outcome；多Pointer的Tab/BackTab方向、stack顺序与两端wrap逐项确定。
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
