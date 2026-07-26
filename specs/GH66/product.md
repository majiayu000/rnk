# Product Spec：InlineChatShell 与类型化原生 scrollback 提交

## Linked Issue

GH-66: https://github.com/majiayu000/rnk/issues/66

complexity: large

## 背景

当前 inline chat 主要由 `examples/claude_input_box.rs` 自行维护字符光标、换行、live
区域和 `app.println()`。现有 `AppContext::println` 只把消息排入 runtime queue，不返回
最终 terminal write/flush 结果；队列被取走后发生 partial write 或 flush failure 时，调用方
无法证明内容究竟是否进入原生 scrollback。因此，重复终态事件、render 重入或进程中断都可能
造成重复输出或把未知副作用误报为成功。

GH-66 提供公共 `InlineChatShell`：稳定终态 transcript 通过类型化 `ScrollbackSink` 固化，
active message 与 `ChatComposer` 留在 live region。默认 native sink 只承诺当前进程内对
confirmed commit 去重；跨重试 exactly-once 只属于明确实现持久、原子幂等合同的注入 sink。

## 目标

- 以公共 shell 组合 Conversation、`ChatMessageView`、`ChatComposer`、native scrollback
  和 live region，应用不再复制 inline 生命周期。
- 用稳定 `commit_id`、不可变 content identity 与 `Committed` / `NotCommitted` /
  `Unknown` 三态结果，诚实表达 terminal 副作用。
- 对重复终态、重复 render、高频 delta、重试、并发与中断保持确定、可测试的去重和顺序。
- 在正常、取消、失败和 panic/unwind 路径恢复 raw mode、cursor、paste 与既有 screen 状态。
- 提供可观察状态、持久 sink 重启重建合同、公共 example、focused/PTY/coverage 证据。

## 非目标

- 不模拟完整终端 scrollback buffer，也不修改已固化的历史行。
- 不包含 Fullscreen transcript viewport、alternate-screen transcript ownership 或
  MessageList virtualization。
- 不负责 provider 请求、provider retry、tool authorization/execution、conversation 数据库
  或 secret 管理。
- 不把默认 native terminal write 描述为跨进程、跨崩溃 exactly-once。
- 不替代 GH-62 Conversation、GH-63 message/block view 或 GH-64 composer 的权威合同。

## Behavior Invariants

1. **B-001** `InlineChatShell` 必须只消费 GH-62 Conversation 的 public read/update
   contract、GH-63 的 public message view 和 GH-64 的 public composer contract；不得复制
   message lifecycle、block renderer、编辑器、provider 或完整 scrollback buffer。任一上游
   public contract 缺失时 implementation 必须 blocked，不得以 alias、private-field access、
   debug-string parsing 或 sidecar model 旁路。
2. **B-002** 每个可提交 terminal message 必须有稳定 `commit_id`，绑定其
   `MessageId`、terminal `MessageRevision` 和第一次 staging 的 exact content identity。
   同一 ID/identity 的重复观察是同一提交；同一 ID 携带不同 content 必须 typed conflict，
   不得覆盖或生成第二个可见效果。
3. **B-003** Pending/Streaming message 永远留在 live region 且不得调用 sink。Complete、
   Cancelled、Failed 仅在 message 与全部 nested lifecycle 都稳定终态、且 exact revision
   projection 成功后成为提交候选；cancel/fail 的可见状态与原因不得改写成 success。
4. **B-004** `ScrollbackSink::commit(commit_id, content)` 必须同步返回闭合三态
   `Committed`、`NotCommitted` 或 `Unknown`；缺失、越界、catch-all 字符串或 warning +
   fallback 均不是有效结果。
5. **B-005** 只有全部 transcript bytes 与 line delimiter 被接受、flush 已确认，且 sink
   的 confirmed ledger 已原子记录相同 ID/identity 时，结果才可为 `Committed`；receipt
   必须公开原 ID、identity 与“首次写入/已确认重复” disposition。
6. **B-006** 只有 sink 能证明 transcript content 的零 bytes 被接受时，结果才可为
   `NotCommitted`；preflight 拒绝、已关闭 sink 或 first-write-before-accept failure 必须保留
   typed cause，且 shell/live state 除记录该 outcome 外不变。
7. **B-007** partial write、写入后 cancellation、flush failure、无法判定 accepted byte
   count 的 broken pipe 或中断必须为 `Unknown`。`Unknown` 不是 success，不写 confirmed
   ledger、不移除 live message、不自动重试，也不得被统一映射为 `NotCommitted`。
8. **B-008** 默认 native sink 必须在当前 `NativeInlineSession` 生命周期内对 confirmed
   ID/identity 去重：重复调用返回原 confirmed receipt 且不执行第二次 terminal write。
   ledger 容量非零、有显式上限；容量耗尽必须 typed blocked，禁止逐出 confirmed ID 后静默
   重写。
9. **B-009** shell 对同一 message 的重复 terminal event、重复 render、重复 projection
   和任意数量高频 delta 只产生一个 in-flight candidate 与至多一个 confirmed commit；
   terminal 后到达的非法 delta/revision 由上游 typed 拒绝，不能触发新 ID。
10. **B-010** message 仅在 `Committed` 后从 live region 移除；`NotCommitted`、
    `Unknown`、content conflict、projection failure 或 sink unavailable 时，原 terminal
    projection 和 typed outcome 必须仍可观察。
11. **B-011** commit 顺序必须按 Conversation message 顺序确定且每次最多一个 in-flight
    commit。较早 candidate 为 `Unknown` 或未显式处理时，较晚 candidate 必须返回 typed
    order-blocked，不得越过后造成 scrollback 重排。
12. **B-012** `NotCommitted` 的重试只在调用方显式请求、ID/identity 未变且当前没有
    in-flight/reentrant commit 时允许；默认 native sink 的 `Unknown` 永不允许自动或普通
    retry。调用方选择放弃、退出或转人工处理必须产生 typed outcome。
13. **B-013** 只有声明 durable atomic idempotency、能按 ID/identity 原子查询与提交的
    `DurableScrollbackSink` 才可跨普通 retry/新 shell 实例返回已提交结果；同一 ID 的 concurrent
    callers 必须共享一次可见效果和同一 receipt，identity conflict 必须全部失败。
14. **B-014** 重启重建不得依赖 clone shell state roundtrip。调用方必须从 GH-62 已验证
    `ConversationStateSnapshot` 重建 Conversation，再由 durable sink 查询每个 terminal
    ID/identity；查询为 committed 才重建 confirmed 状态，not-committed 保持 live，
    lookup unknown/error 保持 unresolved 并阻断后续顺序。native sink 不得伪造跨重启历史。
15. **B-015** composer 在 shell 整个运行期留在 live region。submit、cancel、changed、
    handled、ignored 和 focus routing 必须使用 GH-64/共享 interaction 的 typed outcome；
    submit acknowledgement 失败不得清草稿，cancel 不得隐式退出或固化 active stream。
16. **B-016** focus 的进入、离开和未命中必须可观察且 exactly once 路由；Disabled 优先
    ignored，ReadOnly 不改变 draft，显式 shell-exit 与 composer-cancel 是不同 outcome。
    resize/render 不得改变 focus、draft、commit candidate 或 confirmed ledger。
17. **B-017** 空 Conversation、无 active message、空 optional metadata 显示明确空白，
    不造模型名、token、连接或 commit 数据。空/仅控制字符的 scrollback content 必须 typed
    拒绝，不能制造空 confirmed commit；终态 empty-message legality沿用 GH-62。
18. **B-018** sink 必须逐阶段观察 begin/write/delimiter/flush；zero-write、short write、
    `WriteZero`、partial delimiter、flush error、broken pipe、cancellation前/中/后全部有
    deterministic outcome。错误保留原 `std::io::Error` source，不吞异常。
19. **B-019** 普通 `&mut InlineChatShell` 调用在 Rust 类型层串行；运行期仍必须显式拒绝
    reentrant commit。共享 durable sink 的并发 duplicate 必须原子去重；native session 不得
    被误标为跨线程/跨进程安全。
20. **B-020** shell revision、commit sequence、candidate count 和 ledger counters 全部用
    checked arithmetic。overflow、stale observation、same-ID/different-content、illegal
    live→confirmed/remove 或 shutdown 后调用均 typed fail 且完整 state 原子不变。
21. **B-021** 显式 shutdown 按“停止新事件 → 处理/暴露未决 outcome → 清 live region →
    disable paste/focus/mouse → show cursor → restore raw/screen”顺序执行，并返回每一步 typed
    restoration result。重复 shutdown 是成功 no-op；shutdown 后不得再写 scrollback。
22. **B-022** Drop 与 panic hook 必须 best-effort 恢复 terminal，但不得把不可返回的 Drop
    伪装成已验证成功；产品级成功声明必须来自显式 shutdown 或 PTY 子进程观察。panic/unwind
    仍须恢复 raw mode、cursor visibility、bracketed paste 和进入 shell 前的 screen mode。
23. **B-023** PTY/ANSI evidence 必须分别覆盖正常退出、composer cancel、typed commit
    failure 和 panic/unwind；每条路径验证 raw mode 关闭、cursor 显示、paste 关闭/恢复原值、
    Inline 不进入/离开 alternate screen，且 restoration failure 使测试失败。
24. **B-024** scrollback content 在写 terminal 前必须经过确定性安全边界：规范 LF、只允许
    library renderer 产生的受限 SGR 与可打印 Unicode，拒绝其他 ESC/C0/C1/DEL、光标移动、
    OSC、标题/剪贴板序列。错误和 audit observation 不回显原始 secret/control payload，
    只记录安全类别、range 与 identity。
25. **B-025** native write 与进程内 ledger 不是跨崩溃原子事务。crash 可能发生在 write
    可见而 confirmed ledger 未记录之间，此时重启状态必须是 unknown/unresolved；默认路径
    不得通过重写、删除 live 内容或假设 terminal history 来宣称恢复成功。
26. **B-026** live region 只包含未 confirmed terminal message、active stream、composer
    与 typed status；宽度变化可重新投影 live 内容，但不得重新 wrap/编辑已 committed
    scrollback，也不得清屏模拟历史。无 terminal capability 时只能显式报 unsupported/
    degraded，数据丢失、顺序错误、duplicate 或 restoration failure不能降级成 success。
27. **B-027** `claude_input_box` inline example 必须只组合公共 Conversation/view/
    composer/shell/session API；不得保留 `InlineInputState`、私有 chars/cursor/wrap、
    `app.println` transcript、commit ledger 或直接 ANSI terminal state machine。
28. **B-028** 现有 `AppContext::println`、`RenderHandle::println`、legacy `Message` 与
    non-chat render API 的签名/行为保持兼容；它们不被 alias 成 typed commit，也不能作为
    GH-66 confirmed evidence。
29. **B-029** implementation 开始前 #62/#63/#64 issue 必须 CLOSED，全部最终 closing
    implementation PR（不是 spec/draft/parked PR）均 MERGED，任务/PR gate evidence 完整，
    每个 merge commit 都是 implementation base 的祖先；任一缺失时保持 blocked。
30. **B-030** current-head 完成证据必须包含所有 mapped exact tests、`cargo check`、
    workspace tests、example、PTY、docs、fresh CI、independent review、changed executable
    lines >=80%，以及 committed `gh57-critical-paths-v1` 精确 `file+name` 集合逐项 100%。
    零匹配、ignored、旧 SHA、视觉录屏或别的 child coverage 不构成通过。
31. **B-031** spec、实现、验证和 terminal 恢复是原子完成集合；任一 dependency、mapped
    outcome、recovery、coverage 或 human gate 缺失时 GH-66 仍未完成。重跑必须在同一
    immutable current head 重新生成证据，不能拼接旧结果。

## 验收标准追踪

| Issue AC | 覆盖不变量 |
| --- | --- |
| 1. terminal transcript 经 stable ID sink；stream 不提前提交 | B-002、B-003、B-004 |
| 2. `Committed` / `NotCommitted` / `Unknown` | B-004–B-007、B-018 |
| 3. native confirmed 去重；Unknown 不自动 retry | B-007–B-012、B-025 |
| 4. durable sink 跨重试 exactly-once | B-013、B-014 |
| 5. 重复终态/render/delta；confirmed 后才 remove | B-009–B-011、B-020 |
| 6. composer、cancel 与 focus typed outcomes | B-015、B-016 |
| 7. 正常/重复/cancel/fail/三态/retry/stream tests | B-003–B-020 |
| 8. PTY/ANSI terminal restoration | B-021–B-023 |
| 9. public-only inline example | B-026–B-028 |
| 10. fresh focused/check/project tests | B-029–B-031 |

## Boundary Checklist

| 边界类别 | 结论 | 覆盖 |
| --- | --- | --- |
| 1. Empty / missing input | covered；空会话/metadata/content 不造数据或空 commit | B-017 |
| 2. Error and failure paths | covered；写入阶段、typed failure、restore failure 均 fail loud | B-004–B-007、B-018、B-021–B-023 |
| 3. Authorization / permission | covered；tool/provider 不归 shell，implementation/merge 保留 human gate | B-001、B-024、B-029、B-031 |
| 4. Concurrency / race / ordering | covered；单 in-flight、order block、reentrancy、durable concurrent duplicate | B-009、B-011、B-019、B-020 |
| 5. Retry / repetition / idempotency | covered；native/durable/NotCommitted/Unknown 策略分离 | B-008–B-014 |
| 6. Illegal state transitions | covered；terminality、remove、overflow、shutdown 后调用原子拒绝 | B-003、B-010、B-020、B-021 |
| 7. Compatibility / migration | covered；restart reconstruction、legacy println、dependency ancestry | B-014、B-027–B-029 |
| 8. Degradation / fallback | covered；未知副作用/恢复/顺序错误不伪装成功 | B-007、B-025、B-026 |
| 9. Evidence and audit integrity | covered；receipt、observation、exact head/coverage/dependency/human gate | B-005、B-014、B-029–B-031 |
| 10. Cancellation / interruption / partial completion | covered；cancel/fail、partial write、Unknown、shutdown/panic | B-003、B-007、B-015、B-018、B-021–B-023 |

## Human Gates

- 本 packet 只授权 `write_spec`。人工 spec approval 与 canonical `ready_to_implement`
  之前不得实现。
- 当前（2026-07-26）#62/#63/#64 均为 OPEN；#62 PR #117 为 OPEN、未合并，#63/#64
  只有 spec PR #75/#79 已合并。因此 production implementation gate 明确 blocked。
- 最终 implementation PR 的独立 review、approval、merge、release 与 GH-57 closure 仍由人类决定。
