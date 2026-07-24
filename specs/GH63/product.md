# Product Spec：类型化消息与 AI 内容块视图

## Linked Issue

GH-63: https://github.com/majiayu000/rnk/issues/63

complexity: large

## 用户问题

`rnk` 现有 `Message`、`ToolCall`、`ThinkingBlock` 只接受固定字符串，聊天 examples
又分别实现角色、状态、Markdown、Thinking 和工具展示。应用作者即使采用 GH-62
的后端无关会话模型，仍需自己把每个 typed block 转换成终端元素，容易丢失生命周期、
截断语义、窄宽行为和兼容入口。

本 issue 提供纯、可组合的 `ChatMessageView` 与 typed block views。它只读取 GH-62
的不可变消息快照，不修改 conversation state、不执行工具，也不负责列表滚动、
Composer 或 shell 生命周期。

## 目标

- 直接渲染 GH-62 的 `ChatMessage`、`MessageBlock`、角色和生命周期闭集。
- 提供 Text、Markdown、Code、Thinking、ToolCall、ToolResult、Error、Diff、Quote、
  Link、TerminalAttachmentSummary 和 `StreamingIndicatorView` 的独立 typed views。
- 提供 `Compact`、`Bordered`、`Bubble` 三种可组合消息变体。
- 让 Thinking 展开状态、预览上限和 ToolResult 截断成为显式、受控输入。
- 允许应用通过 typed trait 或 typed closure 覆盖单个 block，同时保留显式默认路径。
- 保持旧 `Message::new/user/assistant/system/tool/tool_result/error`、`ToolCall` 与
  `ThinkingBlock` 使用方式兼容，并记录迁移路径。

## 非目标

- 不定义或修改 provider 事件、conversation reducer、消息身份或生命周期迁移规则。
- 不实现工具执行、权限判断、网络请求、provider JSON、鉴权或密钥处理。
- 不实现 `MessageList`、滚动锚定、Composer、Inline/Fullscreen shell 或焦点管理。
- 不提供完整 Markdown/CommonMark 或代码语法高亮；Markdown 复用现有组件能力。
- 不在 view 内维护定时器、动画时钟、异步任务或跨帧可变状态。
- 不把 legacy `Message` 隐式转换成 GH-62 `ChatMessage`，也不删除旧入口。

## Behavior Invariants

1. **B-001** 当 `ChatMessageView` 接收一个经 GH-62 构造的 `ChatMessage` 时，它必须只读取
   该消息及显式 view options，按原 block 顺序生成一个 `Element`；不得修改 message、
   conversation、revision、ledger 或 nested lifecycle。
2. **B-002** `ChatRole::User`、`Assistant`、`System`、`Tool` 必须各有可区分且
   theme-aware 的默认语义标识；可选 author/timestamp 只借用 GH-62
   `ChatMessageMetadata` 的 typed accessor，缺失时对应区域完全留空，不创建平行
   presentation metadata，也不生成 “Unknown”、当前时间或其他占位数据。
3. **B-003** `MessageStatus::Pending`、`Streaming`、`Complete`、`Failed`、`Cancelled`
   必须产生确定且可区分的状态语义。Pending/Streaming 使用 typed
   `StreamingIndicatorView` 表达等待/流式状态；Complete 不伪造活动指示器；
   Failed/Cancelled 必须明示终态，不能看起来像成功。Failed 还必须显示 GH-62
   保存的非空 typed failure reason；reason 缺失属于上游合同错误，默认 view 不得
   伪造通用原因、吞掉原因或要求另有 Error block 才能诊断。
4. **B-004** 对 `MessageBlock` 的闭集，默认 renderer 必须逐项且不重排地分派到
   Text、Markdown、Code、Thinking、ToolCall、ToolResult、Error、Diff、Quote、Link、
   TerminalAttachmentSummary typed view；
   添加未知公共 variant 导致编译期穷尽匹配失败，而不是运行时忽略。
5. **B-005** Text view 必须保留调用方提供的文本、换行和 Unicode 内容；空 Text
   显示为空内容，只有其所属 lifecycle 指示可见，不生成 `(empty)` 等伪数据。
6. **B-006** Markdown view 必须把原始 Markdown 内容交给现有 `Markdown` 组件的
   structured rendering path；空内容保持空，不把 provider 字段、HTML、ANSI 或
   未声明语法解释成额外权限或终端控制。
7. **B-007** Code view 必须保留多行 code content；`language: Some(non-empty)` 时显示
   该标签，`None` 或空 language 时不显示/猜测语言。首版不声称语法高亮成功。
8. **B-008** Thinking view 必须由调用方提供的 `Expanded|Collapsed` 受控状态和
   `NonZeroUsize` 预览行上限决定输出。Collapsed 只显示至多该上限并在确有隐藏内容时
   明示截断；Expanded 显示全部内容；状态切换不得修改 `ThinkingContent` 或
   `ThinkingStatus`，并以 typed `ThinkingId` 保持 block 身份。Thinking 为 Failed 时
   必须同时显示 nested typed failure cause，不能只显示红色或 “failed”。
9. **B-009** ToolCall view 必须按原有顺序显示 typed name 与 ordered arguments，并区分
   Pending、Running、Succeeded、Cancelled、Failed；失败/取消不得使用成功图标或文案。
   Failed 必须显示 nested typed failure cause；缺失参数显示为空参数列表，不把参数拼成
   或解析成 provider JSON，也不执行 call。
10. **B-010** ToolResult view 必须显示其 typed call correlation identity、内容与
    Pending、Streaming、Complete、Cancelled、Failed 状态；受控预览上限只按完整显示行
    截断，且仅在确有隐藏内容时显示截断标记。Failed 状态和 Error block 必须保持可区分；
    Failed 必须显示 nested typed failure cause，截断不得被描述成完整结果。
11. **B-011** Error view 必须通过 GH-62 `ErrorContent` 的 borrowed accessors，用 error
    语义显示已验证的非空 message 与可选 typed source；source 缺失时不猜值，存在时不得
    丢失。Error 不得被吞掉、改写为 warning 或回退成普通 Text。
12. **B-012** `StreamingIndicatorView` 必须是纯、确定性视图：相同状态和 options
    产生相同输出，不读取 wall clock、不创建线程、不自增 frame；应用需要动画时只能
    显式提供当前 indicator frame。
13. **B-013** `Compact`、`Bordered`、`Bubble` 必须只改变容器、间距、边框、对齐和
    theme token 等 presentation；它们不得删除 block、改变 block 顺序、伪造 metadata
    或改变 lifecycle 语义。任一角色/状态/block 组合在三种变体下均可构造。
14. **B-014** 自定义覆盖边界必须使用闭集 typed `ChatBlockRef`、typed render context
    和 `ChatRenderOverride::{UseDefault, Element}` 等价结果。trait 与 closure 都必须可用；
    公共边界不得包含 `Any`、无类型 map、provider JSON 或按字符串查找 renderer。
    `UseDefault` 必须明确调用同一默认 renderer；自定义 `Element` 必须只替换目标 block，
    不得隐式丢弃相邻 block 或状态外壳。
15. **B-015** 每个自定义 renderer 调用必须收到当前 message identity、message revision、
    role、status、`BlockId`、当前展示位置、view variant 和对应 typed block reference；
    展示位置仅用于观察，不能参与身份；renderer 返回后仍由
    `ChatMessageView` 保持消息容器、block 顺序、stable key 与 lifecycle 外壳。
16. **B-016** 同一 conversation state lifetime 内，每个 entry 的 `BlockId` 必须生成稳定、
    无 provider 字段且不含 vector index、`ThinkingId` 或 `ToolCallId` 的 reconciliation
    key；纯内容 append、插入/重排其他 entry、Thinking 展开/折叠或
    Pending→Streaming 状态更新不得让保留 `BlockId` 的 block 或未改变兄弟换 key。
17. **B-017** 多行、空内容、combining grapheme、ZWJ emoji、CJK、tab 和窄宽输入不得
    通过 `str::lines()`、重建字符串或任意 byte slicing 丢失 source range、被截断或
    panic。Thinking/ToolResult preview 必须把原始 source 原样交给 GH-58 TextFlow
    exact-source ingress，并用其 logical row/source-range projection选择可见行；
    LF、CRLF、standalone CR、连续与尾随 hard break 均保留原 terminator 和 source map。
    view 只组合 structured elements，不手写 ANSI；最终视觉换行、control sanitization
    与窄宽 parity 必须在包含 GH-58 implementation 的 retargeted head 上验收，
    GH-58 未完成时不得声称该部分完成。
18. **B-018** legacy `Message`、`MessageRole`、`ToolCall`、`ThinkingBlock` 的现有
    constructors、builder 和 `into_element` 行为必须继续编译；迁移文档必须说明
    simple string 入口可保留，以及何时改用 GH-62 `ChatMessage` +
    `ChatMessageView`，不能声称自动数据迁移。
19. **B-019** 默认颜色、边框、间距和符号必须从一个确定的 resolved `Theme` snapshot
    或具名 chat view style options 解析。构造 view 时必须恰好捕获一次全局 theme snapshot，
    或由调用方显式提供 snapshot；`into_element` 不得再次读取/修改全局 theme。
    显式 style override 只影响当前 view，不泄漏到后续消息。
20. **B-020** 对相同 immutable message、typed message metadata、options、indicator frame、
    resolved Theme snapshot 和 custom renderer 结果，重复渲染必须产生等价
    Element/output；单次 view 无内部 memo ledger、random identity、ambient theme reread
    或到达时间依赖。可选 caller-owned cache 只按 B-029 至 B-031 的显式 revision/changefeed
    合同工作，不改变该纯函数结果。dark/light snapshots 各自确定且彼此可区分，临时 theme
    scope 退出后必须恢复。
21. **B-021** view 构造和渲染没有外部副作用或 partial commit；调用中断后可用相同输入
    重试。自定义 renderer panic 属于调用方失败边界，不得被库吞掉后伪装为默认成功。
22. **B-022** view 本身不提供共享可变状态；并发应用必须先按 GH-62 得到各自不可变
    snapshot，再在调用方选择的线程/渲染循环中构造 view。库不得猜测事件顺序或在 view
    层解决 reducer 竞态。
23. **B-023** 完成声明必须绑定当前 implementation head：每个 role、message status、
    block variant、nested status与其 failure cause、三种 variant、metadata absent、
    empty/multiline/Unicode/narrow/hard-break fixture、custom trait/closure、legacy
    compatibility 均有 exact 非零匹配测试；plain 与 ANSI golden 均不可在测试时静默更新。
    docs gate 必须证明唯一普通 chat doctest 实际 one passed/zero ignored，且所有新 view
    source 无 lint/doc escape hatch。fail-closed coverage helper 必须重新生成以 full head SHA
    命名、包含全部 planned view sources 的 Cobertura artifact，合计行覆盖率至少 80%，并以
    exhaustive exact matrices 证明 dispatch/status/override/truncation/changefeed-cache
    五类分支 100%。
24. **B-024** GH-63 不得新增 Cargo dependency、provider/runtime/network/tool-execution
    入口，也不得修改 GH-62 reducer/model 合同。任何需要新 model field、状态 variant、
    provider adapter 或 tool side effect 的实现都必须停止并回到对应上游 issue/spec。
25. **B-025** GH-63 必须把 GH-62 `MessageBlockEntry` 的 public borrowed accessor 返回的
    conversation-state-lifetime `BlockId` 作为唯一 block/view identity；`MessageId`
    只界定所属 message，
    当前 position 只决定输出顺序，`ThinkingId` 只界定 message-local thinking lifecycle，
    `ToolCallId` 只界定 conversation-wide call/result correlation。任何 key、disclosure entry
    或 cache entry 均不得用 block index 或 lifecycle/correlation ID 替代 `BlockId`。
26. **B-026** 默认与自定义 renderer 必须通过 GH-62 public borrowed projections 读取
    role/status/revision、`ChatMessageMetadata`、`MessageBlockEntry` 与全部 payload；
    `ThinkingContent`、`ToolCallContent`、`ToolResultContent`、`ErrorContent`、
    `ToolArgument`/closed `TypedValue` 及四种新增复杂 payload 均不得通过 clone whole
    payload、private-field hack、debug output、provider JSON 或未声明字段投影。
27. **B-027** Diff view 保留可选 language 与完整 content；Quote view 保留完整 content
    与可选 attribution；Link view 保留 label 与 target，但只展示为结构化 inert 内容，
    不自行发起导航/网络请求；TerminalAttachmentSummary view 保留 name、可选 media type
    与 summary，但不读取文件或附件。四者均使用各自 typed payload、error-safe structured
    elements 和与其他 block 相同的 `BlockId` wrapper。
28. **B-028** view context/cache contract 必须消费当前 `MessageRevision`，并按 GH-62
    `MessageRevision::INITIAL == 1` 解释新 Push/Resend message；不得把 conversation revision、
    sequence 或零值当 message revision。成功 reducer outcome 只能通过
    `ApplyOutcome.affected_messages` 的 public accessor 驱动增量失效，不能猜受影响
    message 或全局清空。
29. **B-029** 可选 `ChatMessageViewCache` 是 caller-owned、非全局且不写 conversation 的
    presentation cache。对每个 `AffectedMessage`：`Present` 只失效该 `MessageId` 的旧
    revision/message projection 与实际变更或移除的 `BlockId` projection，然后以 applied
    revision 渲染；`Deleted` 必须逐出该 message 的 shell、全部 block、preview 与 disclosure
    entries。outcome 未列出的 message（包括 Resend source）保持 cache-valid；exact replay
    重放同一 affected 列表必须幂等，不能重复产生新 revision 或不相关失效。
30. **B-030** Edit/Insert/Append/Replace 后，保留 entry 的 `BlockId` 继续指向同一 wrapper，
    但 message revision 或 view-relevant payload/options 改变时必须重算其输出；新 entry
    必须使用 fresh `BlockId`，被移除 entry 的 cache/disclosure 必须逐出且永不转移到其他
    identity。Delete 后没有残留 view；Resend 保持 source message/revision/cache 不变，
    新 message 使用 fresh `MessageId`/`BlockId`、Pending 与 revision 1，并独立渲染。
31. **B-031** state-wide retired/seen BlockId、message-local retired ThinkingId、retired
    ToolCall/result-slot history 独立于 processed-event ledger eviction，view/cache 不得令
    tombstoned identity 复活。conversation 显式 restore 后，presentation cache 不从旧
    `Element` 或 position 推断：只从 restored live entries、message revisions 与公开 history
    validation 结果重建；缺失/矛盾 history 必须由 GH-62 restore 拒绝，GH-63 不静默修补。

## 验收标准

- [ ] 四种 role、五种 message lifecycle、十一种 typed block 与 nested lifecycle 在
      `Compact`、`Bordered`、`Bubble` 下均有确定性 render/snapshot 证据。
- [ ] Thinking 展开/折叠、ToolResult 截断、top-level/nested failure reasons、
      empty/multiline/Unicode、LF/CRLF/CR/连续/尾随 hard break、metadata absent、
      dark/light theme snapshot、custom trait/closure 和显式默认回退均由 exact tests 覆盖。
- [ ] 自定义 renderer 公共 API 只暴露闭集 typed inputs，且不能改变相邻 block 顺序、
      stable key 或 message lifecycle 外壳。
- [ ] exact tests 覆盖 BlockId 而非 position/lifecycle identity、typed metadata/
      `ErrorContent`/closed `TypedValue` borrowed projection、Edit/Delete/Resend、
      `MessageRevision::INITIAL`、`affected_messages` 精确 cache invalidation，以及
      restore/ledger eviction 不复活 tombstone。
- [ ] legacy 构造方式继续编译，API stability/migration 文档给出 simple 与 typed 两条路径。
- [ ] GH-62 implementation 已完成；最终窄宽/Unicode parity 证据基于 GH-58 exact
      implementation head，不能以旧布局结果替代。
- [ ] fresh fmt/check/clippy/all-target tests、plain/ANSI golden、rustdoc、coverage、
      CI、独立 review、reviewThreads 和 SpecRail PR gate 均绑定 implementation PR head。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-002, B-005, B-006, B-007, B-009, B-011, B-023, B-026, B-027 |
| 错误与失败路径 | covered: B-003, B-008, B-009, B-010, B-011, B-021, B-029, B-031 |
| 授权/权限 | covered: B-009, B-024, B-027；view 不判断授权、不执行工具，不能把缺失授权解释为成功 |
| 并发/竞态 | covered: B-001, B-022 |
| 重试/幂等 | covered: B-012, B-020, B-021, B-028, B-029 |
| 非法状态转换 | covered: B-001, B-003, B-024, B-028, B-030, B-031；状态合法性由 GH-62 reducer 负责，view 不迁移状态 |
| 兼容/迁移 | covered: B-018, B-024 |
| 降级/回退 | covered: B-014；只有显式 `UseDefault`，不得静默吞错或丢 block |
| 证据与审计完整性 | covered: B-017, B-023, B-025, B-026, B-028, B-031 |
| 取消/中断 | covered: B-003, B-009, B-010, B-021, B-029, B-030 |

## 发布说明

这是增量 API。旧 `Message` 系列保持可用；需要多 block、流式、Thinking 或 Tool
生命周期的应用迁移到 GH-62 typed model，再用 `ChatMessageView` 展示。发布说明必须
明确 GH-58/GH-62 的最低完成依赖、三种 view variant、受控 Thinking 状态，以及自定义
renderer 不执行工具或修改 conversation。
