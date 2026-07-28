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
2. **B-002** 每个可提交 terminal message 必须有稳定 `commit_id`，绑定调用方提供且可跨重启
   恢复的 conversation/store namespace、`MessageId`、terminal `MessageRevision`、第一次
   staging 的 SHA-256 content digest 与冻结的 width/theme projection context。原始 bytes 只在
   content/store payload 中，不进入 identity、audit、Debug 或 Display。同一 namespace/ID/
   digest/context 的重复观察是同一提交；同一 ID 携带不同 digest 或 context 必须 typed
   conflict。不同 namespace 即使其余字段相同也不得互相去重或冲突。
3. **B-003** Pending/Streaming message 永远留在 live region 且不得调用 sink。Complete、
   Cancelled、Failed 仅在 message 与全部 nested lifecycle 都稳定终态、且 exact revision
   projection 成功后成为提交候选；cancel/fail 的可见状态与原因不得改写成 success。
4. **B-004** `ScrollbackSink::commit(request)` 必须同步返回闭合三态
   `Committed`、`NotCommitted` 或 `Unknown`；缺失、越界、catch-all 字符串或 warning +
   fallback 均不是有效结果。
5. **B-005** 只有全部 transcript transport bytes 与 line delimiter 被接受、flush 已确认，且
   sink 的 confirmed ledger 已原子记录相同 ID/identity 时，结果才可为 `Committed`。首次
   committed record 产生一个稳定 original receipt；每次调用另行报告 attempt disposition
   `Written` 或 `AlreadyCommitted`。重复/并发调用共享 original receipt，但不得把本次
   `AlreadyCommitted` 写回或改写 receipt 身份。
6. **B-006** 只有 sink 能证明 transcript content 的零 bytes 被接受时，结果才可为
   `NotCommitted`；preflight 拒绝、已关闭 sink 或 first-write-before-accept failure 必须保留
   typed cause，且 shell/live state 除记录该 outcome 外不变。native transaction 一旦开始清除
   live region，任何 `Committed`、`NotCommitted`、`Unknown`、cancel 或 error 出口都必须在
   返回前立即 repaint；repaint failure 以 typed ordered cleanup aggregate 附在原三态结果上，
   不得覆盖或升级原 primary classification。
7. **B-007** partial write、写入后 cancellation、flush failure、无法判定 accepted byte
   count 的 broken pipe 或中断必须为 `Unknown`。每次 native commit/retry 都必须在阻塞写之前
   由 coordinator 公开 prepare：登记 checked monotonic generation，并返回不借用 coordinator
   的 non-clone concrete ticket 与 `Clone + Send + Sync` cancellation handle；调用方保留 handle、
   把 ticket 交给阻塞调用，因此另一线程/event入口可取消 exact generation。成功、失败、unwind、
   未消费 ticket 的 Drop 与 retry 都须 compare-generation revoke；旧 handle clone只能返回
   `StaleGeneration`，不得取消后续 attempt。未消费 ticket cleanup不得产生terminal mutation；
   禁止任意 callback、`Any` 或 process-global/current-handle endpoint。shutdown只取消已登记的
   当前 generation。
   native sink 在 begin、每次 write 返回、每个编码换行/reset/delimiter、flush 前后和 ledger
   insert 前采样，使 mid-commit cancellation 真实可达。`Unknown` 不是 success，不写 confirmed
   ledger、不移除 live message、不自动重试。
8. **B-008** 默认 native sink 必须在当前 `NativeInlineSession` 生命周期内对 confirmed
   ID/digest/context 去重：重复调用返回原 confirmed receipt且不执行第二次terminal write。
   ledger 容量非零、有显式上限；容量耗尽必须 typed blocked，禁止逐出 confirmed ID 后静默
   重写。
9. **B-009** shell 只允许一次显式 O(n) bootstrap；此后每次 synchronize必须按 GH-62
   `ApplyOutcome::affected_messages()` 返回的确定顺序消费 `AffectedMessage::message_id()` 与
   `AffectedMessage::disposition()`，并只查找/投影受影响或新 terminal message，复杂度为
   O(affected × lookup/projection)，不得每个 delta重扫完整 history。重复 terminal event、
   render或projection只产生一个 in-flight candidate与至多一个 confirmed commit。每个 affected
   entry 必须先按 `AffectedMessageDisposition::{Present, Deleted}` 分支；`Deleted` 不得再查
   post-apply snapshot，并须对 Live、Staged、NotCommitted、Unknown、UnrecoverableUnknown、
   ResolvedCommitted、Abandoned、Confirmed 各 phase 执行闭合删除规则。
10. **B-010** message 仅在 sink `Committed` 或 durable-audited `TreatAsCommitted` 后从 live
    region移除；后者必须在audit append+flush与receipt验证后，以无失败内存transition同时
    写入`ResolvedCommitted`、移除live/source index并归档safe observation；重放同一audit
    transition幂等且不产生第二个可见效果。`NotCommitted`、
    `Unknown`、content conflict、projection failure 或 sink unavailable 时，原 terminal
    projection 和 typed outcome 必须仍可观察。Conversation 删除可移除尚无 terminal effect 的
    Live/Staged/NotCommitted；两种Unknown即使源message已删除也必须保留frozen evidence与顺序
    blocker直至audited resolution；Resolved audit与Confirmed ledger/scrollback均不可变。
11. **B-011** commit 顺序必须按 Conversation message 顺序确定且每次最多一个 in-flight
    commit。较早 candidate 为任一Unknown或未显式处理时，较晚 candidate 必须返回 typed
    order-blocked，不得越过后造成 scrollback 重排。解除 Unknown 只能经 typed manual
    `TreatAsCommitted` 或 `Abandon`，且 resolution 必须先原子写入 durable audit；audit失败时
    原状态与 blocker不变。resolution必须先按完整ID查询audit：已存在的exact choice/evidence/
    digest/context在phase检查前返回typed idempotent success，conflict则零mutation；只有无记录
    时才要求可解决Unknown并执行phase/order gate。commit、NotCommitted retry 与 Unknown
    resolution 必须共享唯一闭集
    `order_satisfied = Confirmed | ResolvedCommitted | Abandoned`；两种 resolution 都会解除后继
    操作，其他 phase 一律仍是 blocker。
12. **B-012** `NotCommitted` 的重试只在调用方显式请求、ID/digest/context未变且当前没有
    in-flight/reentrant commit 时允许；默认 native sink 的 `Unknown` 永不允许自动或普通
    retry。Unknown resolution 的 evidence ID、reason、actor/scope、choice 与 namespace/ID/
    SHA-256 digest/projection identity 必须进入 append-only durable audit；raw content bytes
    禁止进入audit。未知 ID、冲突 resolution 或 audit unavailable 均 typed 拒绝且零 mutation；
    exact duplicate返回已存在receipt。
13. **B-013** 只有声明 durable atomic idempotency、能把 namespace+ID/content digest/frozen
    projection context/raw content payload/original receipt 在同一transaction原子查询与
    提交的 `DurableScrollbackSink` 才可跨普通 retry/新 shell 实例返回已提交结果；同一完整 ID
    的 concurrent callers必须共享一次可见效果和同一 original receipt，各自 disposition仍可
    不同；跨 namespace 不得误 dedupe，identity conflict 必须全部失败。
14. **B-014** 重启重建不得依赖 clone shell state roundtrip。调用方必须显式声明
    `Fresh`、`RestoredDurable` 或 `RestoredAfterUncleanNativeExit` provenance，并从 GH-62
    已验证 snapshot 重建 Conversation。restored constructor 必须实际消费该 validated
    Conversation 与 recovery render context，按 source order seed 每个完整 candidate ID；
    durable lookup 再按 ID取得 store 中冻结的 original digest、canonical bytes、width/theme
    projection context 与 receipt，不得先用当前环境重投影历史；native restored terminal
    candidate只有在注入的pre-crash recovery record含exact first-staged bytes/digest/context时才
    初始化为可`TreatAsCommitted`的recoverable Unknown；缺失任何字段必须成为typed
    `UnrecoverableUnknown`，禁止`TreatAsCommitted`或当前环境重投影，只能durable-audited
    `Abandon`。非空 restored history 必须完整重建，不能与fresh session混淆或伪造跨重启历史。
15. **B-015** composer 在 shell 整个运行期留在 live region。submit、cancel、changed、
    handled、ignored 和 focus routing 必须使用 GH-64/共享 interaction 的 typed outcome；
    submit acknowledgement 失败不得清草稿，cancel 不得隐式退出或固化 active stream。
16. **B-016** focus 的进入、离开和未命中必须可观察且 exactly once 路由；Disabled 优先
    ignored，ReadOnly 不改变 draft，显式 shell-exit 与 composer-cancel 是不同 outcome。
    resize/render 不得改变 focus、draft、commit candidate 或 confirmed ledger。
17. **B-017** 空 Conversation、无 active message、空 optional metadata 显示明确空白，
    不造模型名、token、连接或 commit 数据。空/仅控制字符的 scrollback content 必须 typed
    拒绝，不能制造空 confirmed commit；终态 empty-message legality沿用 GH-62。
18. **B-018** sink 必须区分 canonical LF content bytes 与 terminal transport bytes：每个
    semantic LF 编码为 CRLF，末尾 SGR reset 与 commit delimiter各有独立阶段/计数。
    offset-aware write-all 对每次 `0 < Ok(n) < remaining` 继续写；首个 nonempty write 的
    `Ok(0)`/`WriteZero`/error/cancel 且累计 accepted transport bytes 为零时是
    `NotCommitted`，一旦任意 commit transport byte 被接受，后续 zero/error/cancel 都是
    `Unknown`。progressive shorts、CRLF 中只接受 CR、partial reset/delimiter、flush error、
    broken pipe与cancellation全部有 deterministic outcome；错误公开 canonical/transport
    offset与原 `std::io::Error` source，不吞异常。
19. **B-019** 普通 shell mutation 在 Rust 类型层串行；每个 commit request还必须包含一个
    concrete reentry control。outer attempt持有 permit时，custom sink可安全调用该 control
    的 nested-attempt入口并真实取得 typed `ReentrantCommit`，不得依赖 unsafe alias、
    `RefCell` panic或不可达的第二个 `&mut shell`。共享 durable sink 的并发 duplicate 原子去重；
    native session不被误标为跨线程/跨进程安全。
20. **B-020** shell revision、commit sequence、candidate count 和 ledger counters 全部用
    checked arithmetic。overflow、stale observation、same-ID/different-digest/context、illegal
    live→confirmed/remove 或 shutdown 后调用均 typed fail 且完整 state 原子不变。
21. **B-021** shell 与 native session 必须由单一 public coordinator/typestate共同拥有。
    coordinator 必须提供可实际调用的bootstrap、synchronize、native/durable commit、
    NotCommitted retry、durable reconcile、Unknown resolution、render、typed `try_suspend`/
    `try_resume`、两阶段 native attempt preparation/ticket consumption与shutdown操作，并在
    内部安全拆借 shell/session；不得要求调用方同时持有 coordinator 内部 sink 与 shell borrow，
    也不得在阻塞的`&mut coordinator`调用期间再借用它才能取得cancellation handle。
    suspend仅从Running进入Suspended并保留lease/shell；resume仅从Suspended进入Running，任一
    stage失败保持typed可重试状态且不得伪报Running。
    shutdown 按“停止新事件 → 暴露未决 outcome → 清 live region → restore paste/cursor/raw/
    screen”执行并逐步返回 typed result。session 从不启用、禁用或更改 terminal focus/mouse
    reporting mode，因此 snapshot/rollback/shutdown/suspend/resume 均不虚假声称恢复它们。
22. **B-022** 所有terminal mutation owner共享同一process-wide lease registry：新coordinator、
    legacy `Terminal::{enter,enter_inline,suspend,resume,exit,Drop}`、`TerminalController` mode/cmd
    路径及panic restoration均不得直接绕过。session enter在首次 terminal mutation前取得lease，
    并按 lease→snapshot→raw→cursor→paste→flush staged acquire；任一步失败须逆序尝试回滚、
    聚合 primary+全部 rollback errors。完整恢复/entry rollback后lease回到Free；Drop/panic
    best-effort失败则lease进入Poisoned并阻止新session，直到显式typed recovery完成。suspend
    使用同一阶段表且保留Held lease；任何路径都不能伪装恢复成功。restoration阶段构成显式
    dependency DAG；任一retry stage可能写bytes时必须先把下游Flush与LeaseRelease重置Pending，
    LeaseRelease只可依赖fresh Completed Flush。
23. **B-023** PTY/ANSI evidence 必须通过public coordinator方法分别覆盖正常退出、public
    suspend/resume、composer cancel、typed commit
    failure 和 panic/unwind；每条路径验证 raw mode 关闭、cursor 显示、paste 关闭/恢复原值、
    Inline 不进入/离开 alternate screen，且 restoration failure 使测试失败；还必须覆盖一次
    flush成功后output-producing stage retry会重新flush才release，以及legacy/coordinator/panic
    contention不能并发mutation。
24. **B-024** scrollback content 在写 terminal 前必须经过确定性安全边界：规范 LF、只允许
    library renderer 产生的完整受限 SGR 与可打印 Unicode，拒绝其他 ESC/C0/C1/DEL、光标移动、
    OSC、标题/剪贴板序列。transport在内容后无条件写 canonical SGR reset，reset失败按已接受
    bytes判为Unknown，避免样式泄漏到delimiter/live/status/composer。错误和audit不回显原始
    secret/control payload，只记录安全类别、range与domain-separated SHA-256 digest；raw bytes
    不得出现在observation、Debug、Display或persisted audit record。
25. **B-025** native write 与进程内 ledger 不是跨崩溃原子事务。crash 可能发生在 write
    可见而 confirmed ledger 未记录之间，此时重启状态必须是 unknown/unresolved；默认路径
    不得通过重写、删除 live 内容或假设 terminal history 来宣称恢复成功。没有exact pre-crash
    bytes/digest/context的candidate必须是`UnrecoverableUnknown`，且manual TreatAsCommitted
    fail closed。
26. **B-026** live region 只包含未 confirmed terminal message、active stream、composer
    与 typed status；宽度变化只可重新投影从未 staged 的 live 内容。已有 commit ID/candidate
    必须先查 frozen projection再决定是否投影，NotCommitted/Unknown 在resize/theme变化后继续
    使用原 canonical bytes/context。无 terminal capability 时只能显式报 unsupported/degraded，
    数据丢失、顺序错误、duplicate 或 restoration failure不能降级成 success。
27. **B-027** `claude_input_box` inline example 必须只组合公共 Conversation/view/
    composer/shell/session API；不得保留 `InlineInputState`、私有 chars/cursor/wrap、
    `app.println` transcript、commit ledger 或直接 ANSI terminal state machine。
28. **B-028** 现有 `AppContext::println`、`RenderHandle::println`、legacy `Message` 与
    non-chat render API 的签名/行为保持兼容；它们不被 alias 成 typed commit，也不能作为
    GH-66 confirmed evidence。兼容不等于绕过terminal exclusivity：legacy Terminal/controller/
    panic mutation必须经B-022同一lease registry，contention typed拒绝或进入poisoned recovery。
29. **B-029** implementation 开始前 #62/#63/#64 issue 必须 CLOSED，全部最终 closing
    implementation PR（不是 spec/draft/parked PR）均 MERGED，任务/PR gate evidence 完整，
    每个 merge commit 都是 implementation base 的祖先；任一缺失时保持 blocked。
30. **B-030** current-head 完成证据必须分别记录并校验PR head、PR base SHA、fresh
    `origin/main` SHA与两者merge-base，不得把current main、PR base或merge-base合并成一个字段。
    start/end任一remote head或current main漂移、dirty worktree、缺失env、命令非fail-fast、
    portable temp destination失败或负fixture意外成功都必须blocked。其余完成证据包含所有
    mapped exact tests、`cargo check`、
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
- 当前（2026-07-28）#62 已 CLOSED，最终 implementation PR #117 已 MERGED 为
  `381e281771c7fc6c3a4ac2b6811ef13376bf6501`；#63 仍 OPEN，只有 partial T1 PR #145
  MERGED 为 `27151646fa9b6713abfdec464d4877e17b3c9d7c`；#64 仍 OPEN。#66 仍是
  canonical `ready_to_spec`，不是 `ready_to_implement`，因此 production implementation gate
  继续明确 blocked。
- 最终 implementation PR 的独立 review、approval、merge、release 与 GH-57 closure 仍由人类决定。
