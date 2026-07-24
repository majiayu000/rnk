# Product Spec

## Linked Issue

GH-57

complexity: large

## 用户问题

`rnk` 已具备布局、输入、滚动、Markdown、Thinking、ToolCall，以及 inline / fullscreen
运行模式等通用能力，但聊天相关行为仍分散在 examples 与通用组件中。应用方目前仍需自行处理
Unicode 换行、视觉光标、流式增量、消息高度、滚动锚定、inline scrollback 固化与焦点竞争，
相同问题也被多套示例重复实现。

用户需要的是一组后端无关、契约稳定、可组合并可验证的终端 AI Chat UI 组件，而不是一个
隐藏所有生命周期差异的巨型 `Chat` 组件。该能力必须同时支持原生终端滚屏风格的
`InlineChatShell` 和拥有完整视口的 `FullscreenChatShell`，且两种模式共享消息、Composer
与状态原语，但不混淆各自的渲染和恢复语义。

本 issue 是 umbrella tracking issue。它定义完整产品边界、child issue 依赖和最终关闭条件，
不以自身的 spec PR 代替任一 child 的实现或验证。

## 目标

- 建立后端无关的会话模型、显式状态转换和类型化消息内容契约。
- 提供流式消息、Thinking、Tool Call、错误与取消等 AI 交互状态的共享视图。
- 提供 grapheme-safe、可多行编辑、可配置提交键且高度可控的 `ChatComposer`。
- 提供支持可变高度消息、滚动锚定和按需跟随底部的 `MessageList`。
- 分别提供语义清晰的 `InlineChatShell` 与 `FullscreenChatShell`。
- 统一终端文本测量与绘制契约，保证布局快照、增量更新和完整重建的一致性。
- 在不破坏现有简单消息 API 的前提下，让 examples 收敛为共享组件的组合示范。
- 以确定性测试、兼容性证据、压力测试和 benchmark 定义“产品级”，不以视觉演示代替证据。

## 非目标

- 不实现模型供应商鉴权、HTTP/WebSocket 客户端、API key 或密钥管理。
- 不定义或发送供应商特有的请求结构，也不负责外部服务重试策略和 token 计费真值。
- 不执行工具、shell 命令或任何有副作用的 tool call；核心组件仅显示应用方提供的类型化状态。
- 不负责会话数据库、附件存储或跨进程持久化。
- 不承诺所有终端完全一致或“永久完美”；支持范围由明确的兼容性矩阵和新鲜证据界定。
- 不用一个条件分支密集的巨型组件统一 Inline 与 Fullscreen 的不同生命周期。
- 不在本 umbrella spec 中冻结尚未经 child spec 验证的最终 Rust API 细节。
- 不在 spec 获得人工批准前启动实现，也不授权 agent 最终批准或合并 PR。

## 交付范围与依赖

| 轨道 | Child issue | 产品边界 | 依赖 |
| --- | --- | --- | --- |
| F1 | GH-58 | 统一终端文本测量与绘制流 | 无 |
| F2 | GH-59 | keyed 增量身份与子节点顺序正确性 | GH-58 |
| F3 | GH-60 | 事务式增量 patch 与类型化错误 | GH-59 |
| F4 | GH-61 | `LayoutSnapshot` 一致性、cell 量化与 benchmark | GH-58、GH-59、GH-60 |
| M1 | GH-62 | 后端无关会话模型与状态机 | 无 |
| V1 | GH-63 | 类型化消息与 AI 内容块视图 | GH-62；窄宽最终验收依赖 GH-58 |
| C1 | GH-64 | grapheme-safe 多行 `ChatComposer` | GH-58、GH-60 |
| L1 | GH-65 | 可变高度 `MessageList` 与滚动锚定 | GH-58、GH-60、GH-62 |
| S1 | GH-66 | exactly-once scrollback 的 `InlineChatShell` | GH-62、GH-63、GH-64 |
| S2 | GH-67 | 固定 Composer/状态区的 `FullscreenChatShell` | GH-62、GH-63、GH-64、GH-65 |
| H1 | GH-68 | examples 收敛与产品级 hardening 证据 | GH-61、GH-66、GH-67 |

## Behavior Invariants

1. **B-001** GH-57 只有在 GH-58 至 GH-68 全部完成、各自验收证据可追溯且最终
   closure audit 未发现未完成依赖时才可关闭；任一 child 被跳过、仅部分完成或证据缺失时，
   GH-57 必须保持打开。closure audit 开始与结束时都必须 fresh 确认 GH-57 仍为 OPEN；
   audit 前被提前关闭必须失败，不能把已关闭状态倒推为验收通过。
2. **B-002** 每个 child 必须拥有独立的 SpecRail artifacts、明确依赖和与其范围一致的最终
   implementation PR；committed queue ledger 必须精确包含 GH-58～GH-68、F1/F2/F3/F4/M1/
   V1/C1/L1/S1/S2/H1 与全部依赖边。spec-only、部分交付、ledger 缺项或其他 child 的 PR
   不得冒充该 child 已完成；child spec PR exact diff 只能包含自己的三份 packet。每个 child
   tasks 还必须声明 committed `file + critical-path name` 完整集合，coverage 不得自选子集。
3. **B-003** child 的下游验收不得早于其依赖项完成。允许在无依赖轨道间并行，但并行结果
   不得复用身份、顺序、文本流或状态机等尚未通过上游验收的假设来宣称最终通过。closure
   必须对 committed ledger 的每条 dependency edge 证明 dependency merge commit 是
   dependent merge commit 的严格祖先，不能在所有 PR 最终都进入 main 后倒推依赖顺序正确。
4. **B-004** 公共会话与视图契约不得依赖模型供应商 SDK、传输协议或供应商 JSON；同一组
   类型化会话更新必须能由两个独立后端适配器产生，并得到相同的可观察 UI 状态；GH-68
   `gh68_dual_adapter_state_equivalence` 必须 matched=1、passed=1、ignored=0。
5. **B-005** 空会话必须显示明确的空状态；空文本、空 block 列表与缺失的可选 author、
   timestamp、model、token 等元数据必须按各自语义显示为空或拒绝输入，不得捏造占位数据、
   模型状态或用量。
6. **B-006** 公开 `ConversationUpdate` 合同必须以一等、类型化 variant 表达 edit、delete 与
   resend，`ChatMessage` 必须公开可序列化的 typed revision，且每个会话更新都产生成功或
   类型化失败结果。Push 后必须能用 first-class event append/insert rich block；每个 block
   具有稳定 typed identity，insert 携带 index，所有 block/edit/delete/resend mutation 都
   携带调用方从当前消息读取的 revision。消息/block 不存在、block identity 冲突、index
   越界、revision 缺失/溢出/过期或输入结构非法时不得部分修改现有会话，也不得以 warning、
   空白输出或“成功”状态掩盖失败；成功 mutation 只递增一次，resend 保留原终态消息并以
   initial revision 创建新的消息身份。Complete、Cancel 与 Fail 同样必须携带
   `expected_revision`；stale terminal event 返回 typed error 且 state/revision 不变，
   成功 terminal mutation 只递增一次。
7. **B-007** 消息生命周期只允许 `Pending -> Streaming -> Complete`、
   `Pending|Streaming -> Cancelled` 与 `Pending|Streaming -> Failed`；对 `Complete`、
   `Cancelled` 或 `Failed` 消息继续追加、完成、取消或失败必须被拒绝且保留原状态。
8. **B-008** 每个会话更新必须携带稳定 `event_id` 和 conversation-wide 单调 `sequence`；
   公开 `UpdateId` 必须提供 `new`、`TryFrom<String>`、`Display` 与只读字符串访问，构造时拒绝
   空或仅空白值并返回 typed error；外部 adapter 不得依赖私有字段构造 ID。相同 ID 与内容的
   重放在文档化 retention window 内返回原结果且不产生第二次效果；ledger eviction 后重放
   返回 typed `ReplayOutsideRetention`，不得伪装为新事件或已证明幂等。ID 内容冲突、旧
   sequence 或无法解释的 gap 必须显式失败；retry 必须创建新消息身份并保留原终态消息。
9. **B-009** 同一会话的并发或快速连续更新必须按 B-008 的 sequence 呈现；无法确定顺序、
   与终态竞争或复用 ID 表达不同内容的事件必须显式失败。Complete/Cancel/Fail 的
   `expected_revision` 必须拒绝晚到 terminal race，不得静默覆盖较新的 edit/delta。
10. **B-010** 公开 `MessageBlock` 合同与消息视图必须以独立类型支持文本、Markdown、代码、
    diff、quote、link、Thinking、Tool Call、Tool Result、Error 与终端附件摘要及其状态；
    这些内容不得隐藏在通用 Text/Markdown 或 out-of-band 数据中。Thinking 可折叠，Tool Call
    具有完整生命周期展示。adapter 可在消息 Push 后按 B-006 顺序加入新 rich blocks，不需
    预建 placeholder 或使用 out-of-band mutation。GH-63 必须以 exact tests 覆盖全部 variant
    的 stable identity、合法 lifecycle 和具体 failure reason；不支持或非法的 block 必须
    产生可见且可诊断的失败，不得渲染成看似成功的普通文本。
11. **B-011** `InlineChatShell` 与 `FullscreenChatShell` 必须共享会话、消息块、Composer
    和状态原语，同时暴露彼此独立的生命周期；使用者选择一种 shell 时不需要模拟另一种
    shell 的滚动模型。
12. **B-012** Inline 模式只对达到终态且可稳定显示的消息生成稳定 `commit_id` 并调用类型化
    scrollback sink。默认 native-terminal sink 对已确认提交提供进程内去重；写入中断或结果
    不可判定时必须返回 `Unknown` 且不得自动重试。只有持久化 `commit_id` 并提供原子幂等合同
    的注入 sink 才能声明跨重试 exactly-once。活跃流式消息和 Composer 保持在 live region，
    退出、取消、失败或 panic 后终端模式、光标可见性和输入状态必须可恢复；GH-66 exact
    lifecycle test 必须执行且非 ignored。
13. **B-013** Fullscreen 模式必须拥有并重绘可见 transcript，Composer 与状态区固定在底部；
    resize 后消息重排且焦点保持，生成期间仍可导航，退出后恢复进入前的终端屏幕；GH-67
    exact resize/recovery test 必须执行且非 ignored。
14. **B-014** `ChatComposer` 必须支持多行编辑、可配置的 submit/newline 键、bracketed
    paste、selection、bounded auto-grow，以及 CJK、emoji、combining sequence、CRLF 和
    grapheme cluster；光标、删除、换行和提交边界必须复用 TextFlow 的可见字符语义，不得
    拆分 grapheme。非法编辑返回 typed error 并保持 draft 原子不变；GH-64 exact
    `gh64_grapheme_editing_contract` 必须 matched=1、passed=1、ignored=0。
15. **B-015** `MessageList` 必须按终端行而非消息数量处理可变高度内容；prepend、append、
    block 展开/折叠、流式增长与 resize 后，用户主动滚离底部时视口锚点保持，只有底部跟随
    仍激活时才自动追随新输出，并在暂停时提示下方有新内容。消息级复制、选择与搜索结果跳转
    必须保留稳定 message/row anchor，不能因 reflow 指向其他内容。
16. **B-016** 相同内容、样式、宽度和换行策略的测量与绘制必须使用等价文本流结果；长词、
    CJK、emoji、组合字符和富文本在支持宽度下不得出现“布局计算有高度但内容被截断”或
    后续组件错位。
17. **B-017** 对同一有效界面状态，增量布局与完整重建必须产生等价的终端 cell 边界与可见
    输出。增量失败时只允许显式进入一次完整重建恢复；若恢复仍失败，调用方必须收到错误，
    UI 不得用默认布局、空白画面或旧快照伪装成功。
18. **B-018** 现有简单 `Message` 使用方式和已支持的通用组件行为必须通过兼容包装或有期限、
    有迁移说明的弃用流程保留；examples 逐个迁移并验证行为等价，在共享组件完成前不得删除
    仍承担兼容示范的旧路径。兼容 wrapper 创建 typed chat message 时必须使用公开的 initial
    revision；新 `ChatMessage` 序列化缺少 revision 时必须类型化失败，不得静默猜测版本。
19. **B-019** 交互状态不能只依赖颜色区分；焦点、empty、loading、streaming、disconnected、
    rate-limited、failed、cancelled、disabled 与 read-only 必须具有可读文本或符号语义，
    copy、selection、search 等所有核心操作均可通过键盘完成，终端不支持的能力必须清楚标注
    而非假装可用。
20. **B-020** 大历史、窄/宽 resize、高频流式 delta、长消息、中间插入和 block 展开必须有
    可复现的压力或 benchmark 证据；性能门槛由对应 child spec 量化，缺失基线或结果退化时
    不得宣称产品级完成。
21. **B-021** 取消或中断发生在部分流式内容已经可见时，必须停止后续更新、保留明确的
    `Cancelled` 或失败状态，并避免将未稳定内容错误地重复提交；恢复或重试遵循 B-008，
    不得把部分完成状态改写为成功。
22. **B-022** Tool Call 仅呈现应用方提供的名称、参数、状态和结果；核心聊天组件不得执行
    工具、提升权限、读取密钥或从显示内容推导授权，缺失授权或执行结果时只显示无数据/待定
    状态。
23. **B-023** 每个 child 的完成声明必须绑定当前提交的新鲜验证证据，并覆盖其 spec 中的
    正例、边界和失败路径；GH-57 最终 workspace 验证必须运行在 clean、与 fresh
    `origin/main` 完全相同的集成提交上，且该提交必须包含每个 evidence exact head 所对应的
    GitHub merge commit。workspace check/tests 前后都必须 fresh fetch 并重新确认 remote main、
    local HEAD 与 clean worktree 仍绑定同一集成 SHA；测试窗口内任一漂移必须阻断 closure。
    最终集成验证必须执行 workspace `--all-targets --all-features` check/tests 并产生非零测试
    证据；还必须在 integration SHA 上逐名重跑 committed GH-58～GH-68 mapped exact test
    清单，每项恰好 matched=passed=1 且 ignored=0。每个 child 另需 fail-closed coverage
    artifact，其 `head_sha` 等于最终 PR exact head、changed executable lines 至少 80%，且
    critical `file + name` 集合与 approved child tasks 完全相等并全部 100%。all-target/
    all-feature、mapped 与 critical acceptance evidence 均要求 ignored=0；CI 中
    `continue-on-error` coverage 不构成证明。旧提交、dirty worktree、缩减列表、其他 child、
    视觉演示或“预期会通过”不得替代当前证据。
24. **B-024** `ready-to-spec` 只授权起草规格；实现必须等待人工 spec approval 与
    `ready-to-implement`，最终 PR approval 和 merge 仍由人类执行。child PR 只能引用
    GH-57，不得通过 `Fixes` / `Closes` 提前关闭 umbrella；GH-57 spec-only PR 同样不得关闭
    GH-57，且 base 必须为 `main`、exact diff 只包含 architecture doc 与三份 GH57 spec、
    committed exact head 必须通过 latest SpecRail packet 与 exact four-file Markdown link
    验证。每个 child spec PR 同样只能修改自己的 `product.md`、`tech.md`、`tasks.md`。
25. **B-025** 终端能力或可选展示能力不可用时，只能进入已记录的降级模式并明确提示能力
    差异；会导致数据丢失、布局错误、终端无法恢复或状态错误的情况不得降级为成功。

## 验收标准

- [ ] GH-58 至 GH-68 均具备独立的 SpecRail 规格、任务、实现与验证记录，依赖关系符合
      B-001 至 B-003，closure audit 可从 GH-57 追溯到每个 child 的最终状态。
- [ ] 至少两个不同后端适配示例使用同一会话更新契约，证明 B-004，且核心 crate 不引入
      供应商 SDK、传输或密钥依赖。
- [ ] 会话状态机测试覆盖所有合法转换、每类非法转换、乱序/重复/重试/取消组合，并证明
      B-005 至 B-009、B-021。
- [ ] 类型化消息内容的正常、空、loading、streaming、success、failed、cancelled 和未知输入
      场景均有确定性渲染证据，覆盖 B-010、B-019、B-022。
- [ ] Inline 端到端证据覆盖流式、完成、重复完成、取消、失败、明确未写入、结果未知、resize、
      退出与 panic 恢复；分别证明默认 sink 的进程内去重/`Unknown` 合同，以及幂等测试 sink 的
      exactly-once（B-011、B-012、B-021）。
- [ ] Fullscreen 端到端证据覆盖固定底部区域、持续生成时导航、resize/reflow 与退出恢复，
      覆盖 B-011、B-013。
- [ ] Composer 的编辑、选择、paste、快捷键和 auto-grow 测试覆盖 CJK、emoji、combining
      sequence、grapheme 与窄宽边界，证明 B-014。
- [ ] MessageList 在 append、prepend、流式增长、展开/折叠和 resize 下的滚动锚定与底部跟随
      测试证明 B-015。
- [ ] 文本流与布局的 property/snapshot/parity 测试证明 B-016、B-017；失败恢复的负例不能
      以默认布局、空白或旧快照通过。
- [ ] 现有 API 兼容测试、逐 example 迁移记录和发布迁移说明证明 B-018。
- [ ] 大历史、高频流式、长消息、结构变化和 resize 的 benchmark/stress 结果满足各 child
      已批准的量化门槛，证明 B-020、B-023、B-025。
- [ ] 每个 spec、implementation 与 merge gate 都保留人工边界，最终审计证明 B-024 未被绕过。

## 边界情况

- 空会话、空输入、仅空白输入、空 blocks、缺失可选元数据和未知 block 类型。
- 同一消息收到重复 delta、重复 complete、complete 与 cancel 竞争、终态后继续追加或重试。
- 流式输出跨越多屏、单个超长 token、零宽/组合字符、emoji ZWJ 序列和宽字符落在行边界。
- 用户滚离底部后持续到达新内容，以及 prepend 历史、展开 Thinking/Tool Result 与 resize 同时发生。
- Inline 消息在提交前取消、提交时退出、提交后收到重复事件，或异常中断后重新进入应用。
- Fullscreen 在极小终端尺寸、连续 resize、生成中导航及焦点切换时保持可操作且可恢复。
- 增量布局在中间插入、keyed reorder、subtree 删除或部分失败时不得污染后续帧。
- 旧 API 与新组件共存期间，旧调用方不迁移也应继续得到已承诺行为。
- tool 状态缺少权限、参数、结果或执行方时只呈现已知信息，不能推导或执行任何操作。
- 验证证据对应旧 SHA、依赖未完成、CI 缺失或人工 gate 未记录时，完成声明必须被阻断。

## Boundary Checklist

| 类别 | 结论 | 覆盖 |
| --- | --- | --- |
| 1. Empty / missing input | covered | B-005、B-010、B-022 |
| 2. Error and failure paths | covered | B-006、B-010、B-017、B-021、B-025 |
| 3. Authorization / permission | covered；核心不拥有授权，且不得从展示数据推导或执行权限 | B-022、B-024 |
| 4. Concurrency / race / ordering | covered | B-003、B-009、B-012、B-015、B-021 |
| 5. Retry / repetition / idempotency | covered | B-008、B-009、B-012、B-021 |
| 6. Illegal state transitions | covered | B-006、B-007、B-008 |
| 7. Compatibility / migration | covered | B-018、B-024 |
| 8. Degradation / fallback | covered | B-017、B-019、B-025 |
| 9. Evidence and audit integrity | covered | B-001、B-002、B-003、B-020、B-023、B-024 |
| 10. Cancellation / interruption / partial completion | covered | B-007、B-012、B-021 |

## 发布说明

首个交付周期在现有 `rnk` 公共表面内增量引入聊天组件。现有简单 `Message` API 不直接移除；
若后续确需弃用，必须提供兼容包装、迁移示例、弃用窗口和版本说明。Inline 与 Fullscreen
将作为两个明确入口发布，文档说明它们共享哪些原语、各自拥有何种 scrollback/viewport 语义，
以及支持终端、SSH/tmux 和平台组合的证据边界。

GH-57 的 spec PR 只建立产品合同和 child 队列，不代表实现已获批准。所有 child 必须分别经过
SpecRail spec approval、`ready-to-implement`、当前提交验证、人工最终 review 与人工 merge；
只有 closure audit 证明 GH-58 至 GH-68 全部完成后，才能发布“产品级终端 AI Chat UI
组件体系”并关闭本 umbrella issue。
