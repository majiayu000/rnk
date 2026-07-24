# Product Spec：后端无关的会话模型与状态机

## Linked Issue

GH-62: https://github.com/majiayu000/rnk/issues/62

complexity: large

## 用户问题

`rnk` 目前只有面向展示的简单 `Message`，聊天 examples 又分别维护自己的消息结构、
流式拼接和 provider 响应类型。应用作者无法把不同后端转换为同一套可验证的会话事件，
也无法可靠区分重复事件、乱序事件、非法生命周期更新和真正成功的更新。

本 issue 交付一个不依赖模型 SDK、网络协议或 provider JSON 的公共会话模型，以及一个纯、
确定性、原子提交的事件 reducer。它是 GH-63、GH-65、GH-66、GH-67 的上游契约，但本身
不负责渲染、滚动、shell 生命周期或外部服务。

## 目标

- 提供稳定身份、类型化内容块、消息生命周期和 conversation-wide 事件顺序合同。
- 让流式文本、Thinking、Tool Call、Tool Result、失败和取消均由显式状态表达。
- 对重复、乱序、sequence gap、身份冲突、非法 block 和非法迁移返回类型化错误。
- 保证失败更新不产生部分状态、ledger 或 revision 效果；相同事件序列得到相同结果。
- 明确 processed-event ledger 的容量与进程内边界，不伪造跨边界幂等保证。
- 保持现有 `Message` / `MessageRole` 调用方式可编译，并提供公共 rustdoc 与最小示例。

## 非目标

- 不实现 `ChatMessageView`、内容块视图、Composer、MessageList 或 Inline/Fullscreen shell。
- 不实现 HTTP、WebSocket、鉴权、密钥、模型请求、外部重试或 token 计费。
- 不执行工具、shell 命令或其他副作用；Tool Call 只保存应用方提供的展示状态。
- 不提供会话数据库、跨进程 ledger 持久化、事件总线或分布式顺序协调。
- 不把 provider JSON、`serde_json::Value`、动态 `Any` 或未声明字段作为公共核心 API。
- 不实现编辑器 UI、删除确认 UI 或 resend 按钮；本 issue 只提供它们依赖的 typed
  `EditMessage`、`DeleteMessage`、`Resend` reducer 合同。
- 不迁移 examples；双 adapter mock 只验证转换边界和核心结果一致性。

## Behavior Invariants

1. **B-001** 公共模型必须至少提供 `MessageId`、`UpdateId`、
   `ConversationEvent { event_id, sequence, update }`、`ChatRole`、`ChatMessage`、
   `BlockId`、`MessageBlockEntry`、`MessageBlock`、`MessageStatus`、`ConversationRevision`、
   `MessageRevision`、`TypedValue`、`FailureCause`、`ChatMessageMetadata`、
   `MessageAuthor`、`MessageTimestamp`、`ConversationUpdate`、conversation state、
   typed apply outcome 与类型化 error；除闭集 `ChatRole::{User, Assistant, System, Tool}`
   外，公共 protocol enum 为可演进的 `non_exhaustive` 类型，不存在 `Any`、未声明字段或
   provider 专用类型。ChatRole 到旧 `MessageRole` 的四项转换是 total；反向仅
   User/Assistant/System/Tool 成功，旧 ToolResult/Error 返回 typed conversion error，不猜 role。
   `ChatMessage.metadata` 的闭合字段
   仅为 `{ author: Option<MessageAuthor>, timestamp: Option<MessageTimestamp> }`；两个
   private-field string value 均通过 trim-nonempty constructor/accessor 构造；author 是显示名，
   timestamp 是应用已格式化的显示文本，`None` 保持缺失且 core 不猜当前时间/provider 字段。
   GH-63 可显式把两个 `as_str()` 投影到其 optional
   presentation metadata，但 core/view 均不得自动补值。
2. **B-002** `MessageBlock` 至少覆盖 `Text`、`Markdown`、`Code`、`Thinking`、
   `ToolCall`、`ToolResult`、`Error`、`Diff`、`Quote`、`Link`、
   `TerminalAttachmentSummary`。复杂 payload 的闭合字段合同为：
   `CodeContent { language: Option<String>, content: String }`、
   `DiffContent { language: Option<String>, content: String }`、
   `QuoteContent { content: String, attribution: Option<String> }`、
   `LinkContent { label: String, target: String }`、
   `TerminalAttachmentSummary { name: String, media_type: Option<String>, summary: String }`、
   `ErrorContent { message: String, source: Option<ErrorSource> }`；`ErrorSource` 是
   trim-nonempty 的应用来源标签，message/source 均经 constructor/accessor 暴露。
   Tool 参数为 `ToolArgument { name: String, value: TypedValue }`，其中 `TypedValue` 是
   `Null | Bool(bool) | Integer(i64) | Decimal(DecimalValue) | String(String) |
   List(Vec<TypedValue>) | Object(Vec<TypedField>)` 的 closed constructible enum；
   `TypedField` 仍是具名 key/value，不接受动态 map、`Any` 或未声明字段。
   每个 block（包括静态 Text/Markdown/Code/Error）都包在带稳定 `BlockId` 的
   `MessageBlockEntry` 中，而不是用 vector index 充当身份；
   lifecycle payload 精确为
   `ThinkingContent { id, content, status }`、`ToolCallContent { call_id, name, arguments, status }`、
   `ToolResultContent { call_id, output, status }`，private fields 的每个值均经 constructor/accessor
   暴露。`ThinkingId` 在所属 message 的 Thinking namespace 内唯一；
   `ToolCallId` 在 conversation 的 ToolCall namespace 内唯一，每个 call identity 最多对应
   一个 ToolResult，且 ToolResult 必须引用该 conversation 中恰好一个已存在的 ToolCall。
   ToolCall 与其 ToolResult 共享 correlation identity 是合法配对，不算重复。replacement
   只能沿用同一 block kind 和同一 identity。tool 参数和 object fields 保留调用方顺序且
   同层名字唯一，不接受 provider JSON。保留 entry 的 `BlockId` 不得改变其 block kind；Thinking
   同时保留 `ThinkingId`，ToolCall/ToolResult 同时保留 conversation-wide `ToolCallId`
   correlation，entry identity 与 lifecycle/correlation identity 不得互相替代。
3. **B-003** 空 conversation 是有效初态。`Push` 的 `blocks` 必须至少包含一个类型化
   entry，但 Text/Markdown/Code/Thinking 的内容允许为空以建立流式目标。Code/Diff language、
   Quote attribution、attachment media_type 的 `Some` 必须 trim 后非空；Diff content、
   Quote content、Link label/target、attachment name/summary 和 Error message 必须非空。
   `DecimalValue` 只接受唯一规范文本：零仅 `"0"`；非零允许前导 `-`、整数无前导零，
   可选小数部分但末位必须非零；拒绝 `-0`、正号、指数、空白、NaN/Infinity 以及
   `1.0`/`1.00`/`1e0` 等同值异形。TypedValue
   list/object 可为空，String 值可为空，缺失 value 不等同于 `Null`。空 `UpdateId`、空 Tool Call
   identity/name、空 failure cause/author/timestamp、空 delta、重复 argument/object field 名和
   缺失字段必须返回具体错误，缺失的 author/timestamp 保持 `None`。空 Thinking/Tool Call/
   Tool Result identity 同样非法。重复 Thinking、Tool Call、Tool Result identity，或没有
   对应 ToolCall 的 ToolResult，必须返回具体错误。`Push` 只接受 Pending message，且
   Thinking、Tool Call、Tool Result 等有生命周期的 nested block 也必须从各自 Pending
   状态开始。
4. **B-004** 当 `Push` 携带尚不存在的 `MessageId` 和结构合法的 `ChatMessage` 时，
   reducer 必须只追加一次；当消息 ID 已存在或消息结构非法时，必须拒绝整个事件，不能覆盖、
   合并或部分追加。
5. **B-005** 消息状态允许 `Pending -> Streaming -> Complete`，以及仅对完全不含
   lifecycle block 的完整静态消息允许 `Pending -> Complete`；同时允许
   `Pending|Streaming -> Cancelled` 和 `Pending|Streaming -> Failed(FailureCause)`。
   `FailureCause` 是 private-field、trim 后非空、可 clone/equality compare 的 typed value，
   通过 constructor/accessor 暴露。`Complete`、`Cancelled`、`Failed(_)` 均为终态；
   终态后的 append、replace、complete、cancel 或 fail
   必须返回非法迁移错误并保持原消息。
6. **B-006** `ConversationUpdate` 必须同时提供 `AppendText`、`AppendMessageBlock` 与
   `InsertMessageBlock`：
   `AppendText` 指向存在的 Pending/Streaming 消息和可追加的 Text/Markdown/Code/Thinking
   block，使用 `BlockId` 定位且 delta 按 event sequence 完整追加；
   `AppendMessageBlock` 把 provider 后发现的 entry 追加到现有 Pending/Streaming message
   尾部，`InsertMessageBlock` 以 checked position 插入新 entry；两者都不要求 `Push`
   预声明全部 blocks，也不允许已有 entry 重排或改 ID。`Push` 仍须等 adapter 知道第一个
   typed block 后才创建 message。新 lifecycle block 必须从 Pending 开始并满足 B-002 的
   identity/correlation 唯一性；新静态 block 必须携带非空 payload，空静态流式目标只能在
   首次 `Push` 中声明。任一 append 成功时 Pending message 原子进入 Streaming；目标为
   Pending Thinking 的 `AppendText` 还使其 nested status 进入 Streaming。未知 message、
   未知 `BlockId`、越界插入位置、不可追加 block、空 delta、重复/已退役 identity、
   无对应 ToolCall 的 ToolResult、
   nested/message 已终态或非法 block 必须整体失败，不丢失、重排或重复内容。
7. **B-007** `ReplaceBlock` 只能替换存在消息中的有效 `BlockId`，且只能在消息非终态时提交。
   replacement 必须与原 block 是同一 `MessageBlock` variant；Thinking/ToolCall/ToolResult
   还必须保持同一 typed identity，并通过与 `Push` 相同的结构、全消息唯一性和 correlation
   验证。首版没有 Text↔Markdown、ToolCall↔ToolResult 或其他跨 kind 转换；需要新 kind 时
   使用 `AppendMessageBlock`。未知 BlockId、kind/identity 改变、非法结构或不兼容的嵌套状态变化必须
   拒绝整个事件，不能先删除旧 block 再失败。当合法 replacement 将 nested block 从
   Pending 推进到 Streaming 或 Running 时，同一提交还必须将 Pending message 推进到
   Streaming。
8. **B-008** 同一 `ThinkingId` / Tool Result call identity 的状态只允许
   `Pending -> Streaming -> Complete`、`Pending|Streaming -> Cancelled|Failed(FailureCause)`；
   同一 Tool Call identity 的状态只允许
   `Pending -> Running -> Succeeded`、`Pending|Running -> Cancelled|Failed(FailureCause)`；
   同一 block identity 的终态不可被重写，替换成另一 identity 或倒退状态必须显式失败。
   当 ToolResult 存在时，call/result 关联状态还必须满足完整矩阵：
   Pending call 不允许 result；Running call 只允许 absent/Pending/Streaming result；
   Succeeded call 允许 absent/Pending/Streaming/Complete/Cancelled/Failed result；
   Cancelled call 只允许 absent/Cancelled result；Failed call 只允许 absent/Failed result。
   矩阵外组合必须原子失败。Succeeded+Cancelled/Failed 表示工具执行成功但 result
   传输/消费被取消或失败，不与 call 状态矛盾。
9. **B-009** `Complete` 接受两条且仅两条路径：Streaming message 的全部 Thinking、
   Tool Call、Tool Result nested blocks 已处于各自终态；或 Pending message 已完整提供至少
   一个 Text/Markdown/Code/Error/Diff/Quote/Link/TerminalAttachmentSummary 等静态 block
   的 payload 非空，且不存在 lifecycle block。
   “非空”按原始字符串长度判断，空格是调用方提供的内容；Code 的 language 不算 payload。
   第二条路径不得追加 dummy/重复内容，也不得改变已有 block。仅含 `Text("")`、
   `Markdown("")` 或空 Code content 的 Pending message 是尚未收到内容的流式目标，不能
   直接 Complete；它必须先经非空 `AppendText` 进入 Streaming，或被 Cancel/Fail。Error
   message 在结构验证阶段已经要求非空。其他 Pending message、空/active nested message 的
   Complete 必须整体失败。
   若存在 ToolCall/ToolResult 关联，Complete 还必须先满足 B-008 矩阵且 call/result 均为
   各自终态；Running+Streaming、Succeeded+Streaming、Failed+Complete 等 active 或矛盾
   组合均不得被“都是已关联”降级为成功。
   `Cancel` 和 `Fail` 只接受 Pending/Streaming 消息，
   且 `Fail` 必须携带已验证的 `FailureCause`；成功时它们必须在同一原子提交中将所有仍活跃
   nested blocks 分别推进到 Cancelled 或携带逐值相等 cause 的 Failed，再以同一 cause
   终结目标 message。所有 MessageStatus/ThinkingStatus/ToolCallStatus/ToolResultStatus 的
   `Failed(FailureCause)` 均通过 accessor 保留并暴露该 cause，不能只存 unit Failed。若活跃 ToolCall/
   ToolResult 的 conversation-wide counterpart 位于其他非终态消息，同一事件还必须把该
   active counterpart 推进到相同 Cancelled/Failed 状态；其他消息的 top-level status 和
   无关 blocks 保持不变。所有 affected messages 与 B-008 矩阵验证成功后一次提交；
   conversation revision 只推进一次，每个实际改变的既有 message revision 恰好推进一次。
   任一传播或验证失败必须保持完整 conversation 不变。重复完成、重复取消、
   重复失败或 Complete/Cancel/Fail 竞争中的晚到事件必须失败且不改状态。
10. **B-010** 新 conversation 必须以 `u64` 记录调用方声明的首个 expected sequence；
    首个新事件必须恰好等于该值，此后每个非重放事件必须等于前一已接受 sequence 通过
    checked increment 得到的值。比较顺序以 conversation 为范围，不按 message 或 provider
    分组；不能使用 wrapping、saturating 或可能 panic 的隐式 `+ 1`。
11. **B-011** 当已保留的 `event_id` 以完全相同的 `ConversationEvent` 重放时，reducer
    必须返回首次记录的同一个成功结果，不产生第二次 mutation、revision 或 ledger entry；
    该检查必须先于 stale-sequence 检查。
12. **B-012** 当已保留的 `event_id` 被复用于不同 sequence、update 或内容时，必须返回
    `EventIdConflict` 等价的类型化错误；冲突事件不得借由“sequence 正确”覆盖原记录。
13. **B-013** 未命中有效重放时，低于 expected sequence 的事件必须返回 stale/retention
    boundary 错误，高于 expected sequence 的事件必须返回 sequence gap 错误；两者都不能
    缓存为已处理事件，也不能推进 expected sequence。stale/gap relation 检查先于 counter
    exhaustion，因此只有 sequence 恰好等于 expected 的新事件才进入 checked advancement。
14. **B-014** reducer 必须在完整验证、sequence/revision checked advancement 和 staged
    apply 全部成功后一次提交。未知 message、重复 message ID、越界 block、错误 block 类型、
    重复 lifecycle identity、orphan ToolResult、非法 call/result 关联状态、跨 kind
    replacement、非法迁移、event conflict、stale、gap 或 counter exhaustion 的任一失败
    都必须保持所有 messages、nested 状态、revision、expected sequence 和 ledger 与调用前
    相同；跨 message Cancel/Fail 传播不得留下局部提交。
15. **B-015** processed-event ledger 的容量必须由非零配置显式给出，并只承诺当前
    conversation state 实例内的重放识别。事件从当前 state 的 ledger 被逐出后，旧 sequence
    的重放必须返回“已越过可证明边界”的错误；同一规则只延伸到显式恢复且携带经过验证的
    retention boundary 的 state。公共 `snapshot()` 和 fallible `try_restore(snapshot)` 必须
    携带并验证 messages/revisions/expected sequence、非零容量、按序 processed records、
    eviction boundary 与全部 identity histories；缺失、重复、越界、矛盾历史均 typed fail closed。
    进程重启后若没有恢复该 state/boundary，新的空 state 不持有
    旧事件证据，核心既不能返回已确认重放，也不能保证 `ReplayOutsideRetention`。核心不提供
    持久化或跨进程 exactly-once，任何路径都不能把未知历史声称为已证明幂等。
16. **B-016** 对相同初态和完全相同的有序事件序列，reducer 必须产生相等的 outcomes、
    revision、messages 和 ledger 边界。并发 adapter 必须先给出唯一 conversation-wide
    sequence；无法确定顺序时核心拒绝 gap/stale，不按到达时间猜测顺序。
17. **B-017** 至少两个结构不同、无共同 provider SDK 的 mock adapter 必须把各自事件转换为
    同一核心 `ConversationEvent` 序列，并得到相同状态与结果；核心生产依赖不得因此新增
    provider、HTTP、runtime 或 JSON 依赖。
18. **B-018** 序列化、反序列化、provider 字段校验和 wire-version 迁移属于 adapter 边界。
    核心首版只接受已经构造并验证的 owned typed values，不直接反序列化 provider payload，
    也不把未知 wire 字段静默丢弃后当作成功。禁止依赖审计必须在 Cargo manifests 相对 PR
    base 无变更，并基于 `cargo metadata` 的实际 package identity 解析 direct dependency
    的 crate import name/rename，再对去除 rustdoc/comment/string/char literal 的 Rust token
    stream 解析 `use` tree 的源路径组件（忽略 `as` 后绑定名）并检查 `extern crate`、crate
    path 或 macro；direct、`use {json_alias as json}`、`use crate::{json_alias as json}` 及
    nested group 不得绕过。只在非代码 token 提及名称或 safe source 被 alias 成同名绑定的
    fixtures 必须成功；上述真实 source imports 必须失败。metadata/词法/use-tree 扫描错误阻断，
    零 forbidden source/reference 匹配返回成功。
19. **B-019** 现有 `Message`、`MessageRole`、`ToolCall` 和 `ThinkingBlock` 公共使用方式
    必须继续编译；新模型位于独立的 chat module 并从推荐 public surface 导出。所有新增
    公共类型、variant、field、构造/访问/apply API 必须有 rustdoc，且 chat module 必须启用
    scoped `forbid(missing_docs)`，使任一 child module/item 无法用 `allow`、`expect` 或
    `doc(hidden)` 降低要求，缺失任一 public item 文档时 `cargo check` 失败；至少一个
    位于 `components::chat` 模块文档、使用普通 `rust` fence（非 ignore/no_run）的示例演示
    Push、AppendText 与 Complete。门禁必须抽取这一个实际 rust fence，只在抽取代码中按顺序
    验证三项操作，再证明 `components::chat` doctest 过滤域唯一并执行它；结果严格为一项
    passed、零 failed、零 ignored。token 只出现在 fence 外 prose/注释、零示例、多个候选、
    ignored 或仅编译均不得通过。
20. **B-020** 当部分流式内容已存在后收到 Cancel 或 Fail，已接受内容必须保留且消息进入对应
    终态；跨 message 的 active call/result counterpart 按 B-009 在同一提交中终结，但其所在
    消息的 top-level status、已有内容和无关 blocks 保持不变。所有晚到 append/complete 必须
    失败。应用重试必须通过 `Resend` 使用新的 `MessageId` 和新的事件 identity；原终态 source
    保持不变。
21. **B-021** Thinking 和 Tool Call 数据仅表达应用已提供的名称、typed 参数、状态和结果；
    核心不得执行工具、读写环境/文件/终端、读取密钥、推导权限、发起网络请求，或把缺失
    授权/结果解释为成功。门禁必须对全部 planned chat Rust 文件剥离非代码 token 后，
    结合 cargo metadata/rename fail-closed 拒绝 std process/env/fs/net/terminal/secret surface、
    `env!`/`option_env!`、crate execution modules 及等价 runtime/network/terminal/secret dependencies；
    direct/grouped/nested/adversarial fixtures 均须证明真实 source 失败而 non-code/safe source 成功。
22. **B-022** 完成声明必须绑定当前 implementation head 的新鲜证据：合法迁移、每类非法
    迁移、原子失败、重复/冲突/乱序/gap、ledger eviction、取消晚到、双 adapter、public API
    与 rustdoc 均有 exact 非零匹配测试。所有 mapped integration/lib tests 必须通过
    `--include-ignored` 强制执行，并解析结果严格证明 exactly one passed、zero failed、
    zero ignored；`--list` 中存在名称但运行时 ignored 不构成证据。覆盖率 gate 必须只统计
    本 issue 新增的全部
    `src/components/chat/*.rs` 文件，生成绑定当前 head 的 Cobertura artifact，并以非零退出
    强制这些新代码行的合计覆盖率至少 80%；workspace aggregate 不得替代。reducer 迁移、
    跨层 terminality、counter exhaustion 和错误关键路径按完整矩阵 100% 覆盖。
23. **B-023** top-level message 与 nested lifecycle 必须构成一个闭合状态机：只有 Pending
    message + Pending nested blocks 可由 `Push` 引入；`AppendMessageBlock` 只向非终态消息尾部加入
    满足 identity/correlation 与 call/result 状态矩阵的 block 并推进 message；Thinking 首次 `AppendText`
    同时推进 message 与 nested status；`ReplaceBlock` 只允许同 kind/同 identity，首次启动
    Thinking、Tool Call 或 Tool Result 时也同时推进 Pending message；静态 Pending Complete
    必须至少有一个非空 payload 且不得制造或复制内容，其他 Complete 不得冻结活跃 nested
    block；`Cancel`/`Fail` 必须原子终结目标消息全部活跃 nested blocks，并跨 message
    终结其 active call/result counterpart。任何成功后的 terminal message 都不得含
    Pending、Streaming 或 Running nested block，conversation-wide 关联矩阵始终合法。
24. **B-024** `ConversationRevision` 内值与 `expected sequence` 均为 `u64` 且只使用
    checked increment。
    对未命中 retained replay/conflict 且 sequence 恰等于 expected 的新事件，reducer 必须在
    message/block/nested update validation 之前先计算 next expected sequence，再计算 next
    conversation revision。任一 `checked_add(1)` 返回 `None` 时，在任何 update validation side effect、
    message、nested block、ledger 或 counter mutation 前返回 `SequenceExhausted` 或
    `RevisionExhausted` 等价 typed error。因此 expected 为 `u64::MAX` 时，同序新事件即使
    update malformed 也确定返回 `SequenceExhausted`；exact replay 或 event-ID conflict
    仍先于 exhaustion，stale/gap 也按 B-013 先返回。`u64::MAX - 1` 的最后可推进边界、
    `u64::MAX` sequence 拒绝、malformed-at-max、内部 revision 为 `u64::MAX` 的拒绝，以及
    耗尽时完整 state 相等都必须有确定性非零测试；exact replay 返回原 outcome，因为它不推进
    counter。
25. **B-025** `MessageRevision` 是 public private-field `NonZeroU64` newtype，
    `INITIAL == 1`；`ConversationRevision` 是独立的 public private-field `u64` newtype，
    初态为 0。每个成功事件只把 conversation revision checked 加一；每个实际改变的既有
    message 分别从其 revision checked 加一且每条恰好一次，新 Push/Resend message 从
    `MessageRevision::INITIAL` 开始。`ApplyOutcome` 通过 accessor 暴露 typed
    `affected_messages: Vec<AffectedMessage>`，每项含 `MessageId`、可选 previous revision、
    applied revision 与 `Present | Deleted` disposition；顺序按 mutation 前 conversation
    message 顺序，新增项最后。exact replay 返回原列表，不能再次加 revision。
26. **B-026** `ChatMessage.blocks` 只保存 `MessageBlockEntry`。`BlockId` 属于整个
    `ConversationState` lifetime 的单一 namespace：任意两条 message 不能共享 active ID，
    删除/replacement/edit 移除的 ID 进入 state-wide retired set 且不得复用；保留 ID 必须保持
    block kind，并为 Thinking 保持 `ThinkingId`、为 ToolCall/ToolResult 保持 correlation
    identity。ToolCall/ToolResult 的 entry `BlockId` 是 UI/mutation identity，`ToolCallId`
    是 conversation correlation identity；两者职责不同且均稳定。每个 MessageId 另有
    message-lifetime ThinkingId namespace：Edit 移除 ThinkingBlock 时其 ThinkingId 进入该
    message 的 retired set，不得以新 Pending block 重建；不同 message 仍可复用相同 ThinkingId。
    Block 与 Thinking tombstone retention 均独立于 processed-event ledger eviction，显式恢复
    必须恢复并验证 state-wide seen/retired BlockId、per-message seen/retired ThinkingId 与
    per-ToolCall result-slot history；slot 是 `Vacant | Occupied(location) | Retired`，
    location 精确包含 result 的 MessageId/BlockId；
    fresh state 无历史时才可重新接受相同 `(MessageId, ThinkingId)`。任何 unknown、
    cross-message duplicate BlockId、same-message retired ThinkingId 或 identity-changing mutation
    原子失败。
27. **B-027** `EditMessage`、`DeleteMessage`、`Resend` 是 first-class typed updates。
    Edit 以完整非空 entry 列表替换内容但保留 message ID/role/status/metadata；保留 entry 遵守 B-026，
    active message 的新 lifecycle entry 从 Pending 开始，terminal message 只可新增静态 entry，
    且 lifecycle status/identity 不得重写；被 Edit 移除的 ThinkingId 先 stage 为 retired，
    同一 candidate 或后续事件重新加入该 ID 都失败且不得提交部分 tombstone。Delete 在删除前
    checked 推进被删 message revision，
    outcome 记录 `Deleted`；若删除会留下跨 message orphan ToolResult 则整个事件失败，删除
    result 而保留 call 会原子把该 call 的 result slot 退役，后续同 call result 即使 ledger
    已逐出也拒绝；删除同 message 的 call/result 同样退役 call 与 slot。restore 必须携带并
    验证 slot/history，不能从 live call correlation 猜回已删除 result。Resend 只接受 terminal
    source，source 逐值及 revision 不变且不列入 affected；调用方提供同 role、Pending、
    新 `MessageId`/`BlockId` 和合法 identities 的 message，成功后 revision 为 1；新 message
    可复用 source 的 ThinkingId，但 ToolCallId 仍须 conversation-wide fresh。删除的 MessageId
    与所有已退役 block/thinking/call/result identity 按各自 namespace 规则不得复用。
28. **B-028** 每个 update 都携带 expected conversation revision；所有 targeting existing
    message 的 update 还携带 expected message revision。校验顺序固定为 retained
    replay/conflict → stale/gap/`ReplayOutsideRetention` → sequence exhaustion →
    conversation revision exhaustion → expected conversation revision → target lookup →
    expected message revision → affected-set/message revision checked advancement →
    structural/lifecycle/global-correlation validation → single commit。stale guard、
    unknown/retired identity、message revision exhaustion 和 retention error 均保持 messages、
    Block/Thinking tombstones、revisions、sequence、ledger 完全不变。公共
    message/metadata/entry/outcome/guard/revision
    类型使用 private fields 与 constructor/accessor；扩展性 enum 使用 `non_exhaustive`。
    Core 不冻结 serde wire format；adapter 反序列化必须显式读取 revision 与 BlockId：
    revision 的缺失/0/负数/溢出和 BlockId 的缺失/负数/溢出均交给 fallible constructor，
    禁止默认、静默截断或因 ledger eviction 丢弃 BlockId/ThinkingId tombstone；显式 restore
    缺少或矛盾的 per-message Thinking history 必须失败。删除后的新 current-sequence
    update 命中 tombstoned MessageId 时返回 `UnknownMessage`；若同一旧事件已越过 ledger
    retention，则先返回 `ReplayOutsideRetention`，不得继续 target lookup。

## 验收标准

- [ ] 公共 API 覆盖 B-001 至 B-003、B-025、B-026：`MessageRevision` 为
      `NonZeroU64` 且 `INITIAL=1`，所有 block 使用 conversation-lifetime stable `BlockId`
      entry；复杂 payload 与 ToolArgument.value 可由 closed `TypedValue` API 构造，所有
      Failed status 保存 typed `FailureCause`；metadata 仅含 optional typed author/timestamp，
      ChatRole/legacy mapping、Error/Thinking/ToolCall/ToolResult payload 均为闭合可读合同，
      DecimalValue 只有一种规范文本；没有 `Any`、`serde_json::Value`、provider SDK 或未声明字段。
- [ ] table-driven 测试枚举消息、Thinking、Tool Call、Tool Result 的全部合法边和非法边，
      并覆盖 Push/AppendText/AppendMessageBlock/InsertMessageBlock/ReplaceBlock/Complete/
      Cancel/Fail/EditMessage/DeleteMessage/Resend 的跨层组合、late-discovered block
      追加/插入、same-kind replacement 和跨 kind 拒绝，证明 B-004 至 B-009、
      B-020、B-023。
- [ ] 同一 message 的重复 Thinking identity、conversation-wide 重复 ToolCall identity、
      重复 ToolResult call identity 与 orphan ToolResult 均原子失败；不同 message 可复用
      ThinkingId，且一个 ToolCall/一个相关 ToolResult 的共享 call identity 合法；Edit 移除
      Thinking 后同 message 不可复用该 ID，跨 message 复用仍合法。
- [ ] table-driven exact tests 枚举 ToolCall
      Pending/Running/Succeeded/Cancelled/Failed × ToolResult
      absent/Pending/Streaming/Complete/Cancelled/Failed 的完整矩阵，并证明
      Push/AppendMessageBlock/InsertMessageBlock/ReplaceBlock/Cancel/Fail/EditMessage/
      DeleteMessage 只产生合法组合；message Complete 拒绝所有
      active 或矛盾关联且失败前后 full state 相等。call/result 分居不同消息时，从 call
      message 或 result message 发起 Cancel/Fail 都会在一个 revision 内终结所需 active
      counterpart、保持非目标消息 top-level/无关内容不变，且不会暴露非法中间态。
- [ ] static user/system/assistant Text 消息在不 append dummy 内容时可 Pending -> Complete；
      仅含空 Text/Markdown/Code payload 或带 active nested block 的 Pending message
      被拒绝且 state 不变，证明 B-005、B-009、B-023。
- [ ] 顺序、重放、冲突、stale、gap、容量逐出和并发交错测试证明 B-010 至 B-016，
      每个失败前后完整 state 相等。
- [ ] current-state ledger eviction 返回 honest boundary error；fresh restart state 没有恢复
      boundary 时不声称 replay/outside-retention；snapshot/try_restore roundtrip 保留 ledger 与
      所有 identity histories，矛盾 history typed fail closed，证明 B-015 的进程边界。
- [ ] sequence/revision 在 `u64::MAX - 1`、`u64::MAX` 的 checked advancement 测试证明
      B-024；malformed-at-max 精确断言 `SequenceExhausted`，replay/conflict/stale/gap 精确
      断言更高优先级，所有测试必须证明无 panic/wrap/saturate 且完整 state/ledger 不变。
- [ ] typed revision guards、per-message checked increment 与 `ApplyOutcome.affected_messages`
      exact tests 证明 B-025/B-028；stale conversation/message guard、message revision
      overflow、unknown/retired identity 都原子失败。
- [ ] edit/insert 保持 BlockId/kind/lifecycle identity；delete 覆盖跨消息 call/result
      orphan 拒绝与合法删除；resend 保持 source 且创建 revision 1 的全新 identity，
      cross-message duplicate/retired BlockId 与 same-message retired ThinkingId 原子拒绝，
      删除 result 后对应 result slot 永久退役；ledger eviction 不释放 tombstone，显式 restore
      保留 Block/Thinking/call/result-slot identity history；
      exact replay 与 ledger eviction 对三类 mutation 一致，证明 B-026 至 B-028。
- [ ] 两个 mock provider adapter 的输入结构不同，但产生相等核心事件、outcomes 和最终 state；
      metadata-aware Rust-token audit 对 rustdoc/comment/string 与 safe grouped-use fixtures
      成功，用 direct/grouped-root/nested-group renamed-package、grouped std process、
      std terminal I/O、env macro 与 crate execution-module fixtures 证明真实能力引用会失败，
      且生产依赖未新增
      provider/HTTP/JSON，覆盖 B-017、B-018、B-021。
- [ ] 现有简单 Message 使用方式和新推荐 public surface 都通过 compile/public API 测试，
      `cargo check` 在 chat-scoped `forbid(missing_docs)` 下通过；child
      `allow(missing_docs)`、`expect(missing_docs)` 与 `doc(hidden)` 的确定性审计为零；唯一、
      普通 rust fence 的
      非注释代码按顺序包含 Push -> AppendText -> Complete，且绑定的唯一 doctest 被执行并
      得到 1 passed/0 ignored；fence 外或注释中的 token 不构成证据，覆盖 B-019。
- [ ] fresh `cargo fmt --check`、`cargo check`、完整测试、doc test 和 coverage 证据绑定当前 head，
      每个 module-qualified exact 测试过滤器先证明只匹配一个 test，再以
      `--include-ignored` 执行并断言 1 passed/0 failed/0 ignored；Cobertura artifact 覆盖
      全部五个 planned chat source files 且 line-rate ≥ 80%，覆盖 B-022。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-003、B-006、B-009、B-027、B-028 |
| 错误与失败路径 | covered: B-004、B-006、B-007、B-009、B-013、B-014、B-023、B-024、B-026、B-027、B-028 |
| 授权/权限 | covered: B-021；核心没有授权/执行能力，缺失权限不得推导为成功 |
| 并发/竞态 | covered: B-009、B-010、B-013、B-016、B-020、B-023、B-024、B-025、B-028 |
| 重试/幂等 | covered: B-011、B-012、B-015、B-020、B-025、B-027、B-028 |
| 非法状态转换 | covered: B-005、B-006、B-007、B-008、B-009、B-023、B-026、B-027 |
| 兼容/迁移 | covered: B-018、B-019、B-025、B-028 |
| 降级/回退 | covered: B-013、B-014、B-015、B-024、B-028；关键错误不允许降级为成功 |
| 证据与审计完整性 | covered: B-015、B-019、B-022、B-024、B-025、B-028 |
| 取消/中断 | covered: B-005、B-009、B-020、B-023、B-027 |

## 发布说明

该变更以新增 `rnk::components::chat` 与推荐 prelude 导出交付，不移除或重命名现有简单
`Message` API。新增 public 类型只允许 constructor/accessor，不要求外部 struct literal；
扩展性 enum 是 `non_exhaustive`。首版 ledger 是显式有界、进程内能力；发布文档不得使用
“持久幂等”或“跨进程 exactly-once”描述。provider adapter、wire serialization 和工具执行
继续由应用拥有。
