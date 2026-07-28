# Tech Spec：InlineChatShell、同步 scrollback sink 与 terminal 恢复

## Linked Issue

GH-66: https://github.com/majiayu000/rnk/issues/66

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":66,"complete":true,"paths":["specs/GH66/product.md","specs/GH66/tech.md","specs/GH66/tasks.md","Cargo.toml","Cargo.lock","src/components/chat/mod.rs","src/components/chat/inline.rs","src/components/chat/inline/types.rs","src/components/chat/inline/sink.rs","src/components/chat/inline/state.rs","src/components/chat/inline/session.rs","src/components/chat/inline/sanitize.rs","src/components/chat/inline/tests.rs","src/components/mod.rs","src/prelude.rs","src/renderer/terminal.rs","src/renderer/terminal/inline_scrollback.rs","src/renderer/terminal/lease.rs","src/renderer/terminal_controller.rs","src/runtime/panic_handler.rs","examples/claude_input_box.rs","docs/CORE_COMPONENT_CONTRACTS.md","tests/inline_chat_shell.rs","tests/inline_chat_shell_pty.rs","tests/prelude_surfaces.rs"],"spec_refs":["specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md","specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md","specs/GH63/product.md","specs/GH63/tech.md","specs/GH63/tasks.md","specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md","specs/GH66/product.md","specs/GH66/tech.md","specs/GH66/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只拥有 Inline shell、scrollback commit/sink、native inline session、terminal 安全提交
适配、公共导出、一个 example 和对应 tests/docs。Conversation model/reducer 属于 GH-62，
message/block rendering 属于 GH-63，composer 属于 GH-64；implementation 只能以它们最终
merged public API 为准。

## Implementation Gate

spec-only 工作可继续；production implementation blocked：2026-07-28 fresh evidence中，
#62 已 CLOSED且最终 PR #117 MERGED为`381e281771c7fc6c3a4ac2b6811ef13376bf6501`；
#63 仍 OPEN，只有partial T1 PR #145 MERGED为`27151646fa9b6713abfdec464d4877e17b3c9d7c`；
#64 仍 OPEN，#66仍为canonical `ready_to_spec`。`SP66-T1`前须fresh分页收集dependency issue、
closing PR files/labels/reviews/checks/tasks。三个issue必须CLOSED；最终closing PR必须MERGED、
非draft/parked、含executable Rust，且merge commits均为implementation base祖先；partial/spec
PR、green CI、open branch或手填SHA均不满足。另需GH-66人工spec approval与canonical
`ready_to_implement`；API/path drift先更新并重批packet。

## Codebase Context

锚点基于2026-07-28 current `origin/main`
`27151646fa9b6713abfdec464d4877e17b3c9d7c`，均已用 Read/grep 核实。

| Area | Current anchor | Current behavior | GH-66 decision |
| --- | --- | --- | --- |
| Inline example state | `examples/claude_input_box.rs:35`, `:148`, `:407`, `:474` | 私有 `Vec<char>`、cursor、input handler、wrap 与 live viewport | 手工迁移为 public shell composition，删除全部私有生命周期 |
| Example commit | `examples/claude_input_box.rs:148`, `:157`, `:164` | submit 后连续 `app.println` 并立即 clear draft；无 write ack | 只能经 typed sink/receipt，ack 后再改变 shell state |
| Shared interaction | `src/components/interaction.rs:9`, `:39` | closed `InteractionMode` 与 `InteractionOutcome<T>` | 直接复用，不增 alias/catch-all，不扩展既有 exhaustive enum |
| Runtime paste | `src/renderer/runtime.rs:125`, `:155`, `:165`, `:170` | Key/Mouse/Resize 分支；Paste 被 wildcard 丢弃 | GH-64 最终 merged path负责 paste；GH-66 只消费 composer outcome |
| Public println | `src/hooks/use_app.rs:113`; `src/runtime/context.rs:333` | `println` 返回 `()`，只委托 handle | 保持兼容，不作为 confirmed commit |
| Runtime queue | `src/renderer/registry.rs:125`, `:257` | `AppSink::println` 无返回；queue 在 terminal write 前被 `take` | typed commit 不复用该 ack-less queue |
| Queue bridge | `src/renderer/runtime_bridge.rs:62`, `:67` | 先 drain 全 queue，再调用 controller；失败后无 per-message outcome | 保持 legacy path；新 sink 直接同步调用 terminal transaction |
| Terminal println | `src/renderer/terminal.rs:354`, `:363`, `:367`, `:370` | 清 live UI、逐行 `write!`、flush，返回一个 `io::Result` | 提取 staged write helper，记录 first accepted byte/partial/flush stage |
| Inline terminal lifecycle | `src/renderer/terminal.rs:168`, `:178`, `:197`, `:212`, `:697` | enter/exit 返回 `io::Result`；Drop 吞 exit error | 新 session提供显式 typed shutdown；Drop只 best effort |
| Panic recovery | `src/runtime/panic_handler.rs:16`, `:23`, `:27`, `:30`, `:40` | restore直接mutate且吞error，却打印“restored” | 通过共享lease/recovery stage；legacy wrapper不作成功声明 |
| Bracketed paste | `src/hooks/paste.rs:26`, `:35`, `:56`, `:71` | enable/disable 有 Result，guard Drop吞 error | session显式拥有并恢复 prior state，Drop不作成功证据 |
| Current exports | `src/components/mod.rs:4`, `:23`; `src/prelude.rs:55` | GH-62 chat types已导出；GH-63 partial view只在`components::chat::view`，尚无inline/composer导出 | 在上游最终 chat module 上增加 `inline` concrete exports |
| Terminal mutation owners | `src/renderer/terminal.rs`, `src/renderer/terminal_controller.rs`, `src/runtime/panic_handler.rs` | lifecycle/controller/panic均可直接写terminal | 全部通过`terminal/lease.rs` registry；contention与poison typed |
| Terminal file size | `src/renderer/terminal.rs:1` | 当前 791 行，接近 800 hard ceiling | 拆出lease/inline child并路由现有lifecycle，root保持<800 |
| Dependency code | `src/components/chat/model.rs:705`, `:735`; `src/components/chat/view/mod.rs:1`; `rg ChatComposer` | GH-62 public Conversation/ApplyOutcome已在PR #117合入；GH-63仅PR #145 T1 typed view contract；GH-64 composer仍缺失 | 等#63/#64最终closing implementation合入后重新定位，禁止按 partial API或spec猜字段 |

## 设计方案

### 1. 同步 sink 选择与调用边界

采用同步 trait，不使用 `async fn`、future、tokio task 或 callback ack：

```rust
pub trait ScrollbackSink {
    fn guarantee(&self) -> ScrollbackGuarantee;
    fn commit(
        &mut self,
        request: ScrollbackCommitRequest<'_>,
    ) -> ScrollbackCommitOutcome;
}
```

理由：实际 `Terminal::println`、`Write`、flush、render loop 和 terminal enter/exit 均为同步
`std::io::Result`；现有 tokio 只服务 command/background runtime，`AppContext::println`
则是无 ack queue。同步调用仍可真实取消：coordinator每次attempt以checked counter创建
`ScrollbackCancellationGeneration(NonZeroU64)`与新
`ScrollbackCancellationToken(Arc<AttemptCancellationState>)`；request携带generation/token，
session/event ingress或测试线程取得同generation的`InlineCancellationHandle`。attempt结束
立即revoke；handle的`cancel()`原子比较generation并返回`Applied | StaleGeneration`，旧clone
不能影响retry。shutdown只signal当前generation。request还携带
`ScrollbackCommitControl`；outer call先取得其RAII permit，custom sink可调用
`try_begin_nested_attempt()`并安全得到typed `ReentrantCommit`，不需要第二个`&mut shell`。
generation/token/control均为concrete private-field值，不是任意callback。

`ScrollbackSink` 不接受 `Any`、字符串 error、任意 callback 或 boxed future。生产
`NativeScrollbackSink<'a>`是coordinator内部私有适配器，借用同一session的`Terminal`与ledger；
不从public API返回。coordinator安全拆借shell/session并在同一scope完成primary transition与
repaint，从类型层防止commit/write和live render交错。

### 2. 公共 commit types

所有 struct 字段 private，提供具名 constructor/accessor；没有 public alias。可扩展 behavior
enum 标 `#[non_exhaustive]`，必须穷举的 outcome/error family 保持 closed 并由 crate 外
compile fixture 无 wildcard match。

```text
ScrollbackCommitId { namespace, message_id, terminal_revision }
ScrollbackContent { bytes: Arc<[u8]>, identity: ScrollbackContentIdentity }
ScrollbackContentIdentity { sha256: [u8; 32] } // domain + exact bytes
ScrollbackThemeIdentity { sha256: [u8; 32] } // canonical resolved terminal style tokens
ScrollbackProjectionContext { width: NonZeroU16, theme_identity: ScrollbackThemeIdentity }
NativeAttemptRecoveryRecord { recovery_id, commit_id, private_content, digest, projection }
ScrollbackCommitRequest<'a> {
  commit_id, content, projection, cancellation_generation, cancellation, control
}

ScrollbackGuarantee =
  ProcessLocalConfirmed |
  DurableAtomicIdempotency

ScrollbackAttemptDisposition = Written | AlreadyCommitted

ScrollbackCommitReceipt { commit_id, content_identity, projection_context, session_sequence }

ScrollbackCommitOutcome =
  Committed {
    receipt: ScrollbackCommitReceiptHandle,
    disposition: ScrollbackAttemptDisposition,
    cleanup: ScrollbackCleanupReport
  } |
  NotCommitted { primary: ScrollbackCommitError, cleanup: ScrollbackCleanupReport } |
  Unknown { primary: ScrollbackCommitError, cleanup: ScrollbackCleanupReport }

ScrollbackCleanupReport =
  Complete |
  Failed(ScrollbackCleanupErrors) // private nonempty ordered errors + accessors

ScrollbackCleanupError =
  LiveRepaint { source: std::io::Error }
```

`ScrollbackContent`私有持有exact immutable bytes；identity由crate使用Cargo.lock固定的维护中
SHA-256实现计算，domain separation固定且不能由caller注入digest。identity/receipt/audit只持有
32-byte digest，不持有raw bytes；durable payload可私有保存raw以完成exact replay，但Debug、
Display、observation和audit serializer必须显式redact。`ScrollbackNamespace::try_new`拒绝
空白/控制字符且必须由应用或durable store持久恢复；
`ScrollbackCommitId::new(namespace, MessageId, MessageRevision)` 的消息参数由 GH-62 validated
constructor产生。不同namespace互不dedupe；同一完整ID/different digest或projection context返回
`ScrollbackCommitError::ContentIdentityConflict`。`ScrollbackCommitReceiptHandle`内部为
`Arc<ScrollbackCommitReceipt>`；first write建立一次original receipt，duplicate/concurrent
attempt共享相同allocation/fields，但各自outcome携带自己的disposition，ledger不改写receipt。

`ScrollbackCommitError` closed variants 至少为：

```text
InvalidContent(ScrollbackContentError)
IdentityConflict(ScrollbackIdentityConflict)
Io(ScrollbackIoError)
Cancelled(ScrollbackCancellationStage)
ReentrantCommit
LedgerCapacityExhausted { capacity }
SinkClosed
UnsupportedGuarantee { required, actual }
TerminalLeaseAlreadyHeld { holder }
ManualResolutionAuditFailed { source }
StaleCancellationGeneration { expected, actual }
UnrecoverableNativeAttempt { recovery_id }
```

`ScrollbackIoError` 保存
`ScrollbackIoStage::{Begin, WriteContent, WriteReset, WriteDelimiter, Flush}`、
canonical offset、`accepted_canonical_prefix_bytes`、`accepted_transport_bytes`、
`transport_segment_offset`、`reset_bytes`、`delimiter_bytes` 与原 `std::io::Error`；实现
`Error::source`。
Display 只输出 stage/count/error kind，不回显 content。`ScrollbackContentError` 保存安全
category 与 byte range，不保存或显示原始 control/secret。

### 3. Native write transaction 与三态分类

`src/renderer/terminal/inline_scrollback.rs` 为现有 `Terminal` 增 crate-visible staged helper，
并以 private generic `W: Write` helper做故障注入测试。production仍写 stdout，不修改 legacy
`Terminal::println` 签名/语义：

```text
begin: clear current live region
-> encode canonical content deterministically:
     printable/SGR bytes unchanged；每个 canonical LF -> CRLF
-> write canonical SGR reset ESC[0m
-> write exactly one CRLF delimiter
-> flush
-> insert confirmed ledger（仅成功路径）
-> repaint live state（所有begin后出口）
```

`ScrollbackContent`与identity始终保存normalized canonical LF bytes，不保存机器相关transport。
encoder逐个semantic segment输出，保留`canonical_offset`与segment内
`transport_offset`；只有一个完整semantic segment全部写完才推进
`accepted_canonical_prefix_bytes`。例如LF的CR成功而LF失败时，canonical prefix不包含该LF，
但transport count包含CR且outcome必为Unknown。reset和最终delimiter不属于content identity，
却分别计数并进入receipt前的成功条件；这与legacy `Terminal::println`逐行CRLF语义一致。

每个非空segment使用同一个offset-aware循环：以`offset < bytes.len()`调用
`write(&bytes[offset..])`；`Ok(n)`须满足`n <= remaining`，`n > 0`累加offset/counters并继续，
包括positive short write；`Ok(0)`规范为`WriteZero`。分类只看累计
`accepted_commit_transport_bytes`，不把清除live region的控制bytes误算为transcript effect：

| 观察 | Outcome |
| --- | --- |
| preflight/begin/clear失败；或首个nonempty write `Ok(0)`/error/cancel且accepted=0 | `NotCommitted` |
| 任意positive short writes最终写完 | 继续到下一segment，不提前分类 |
| 任意accepted byte后的`Ok(0)`/error/cancel；CRLF只写入CR | `Unknown` |
| content完成但canonical SGR reset partial/失败 | `Unknown` |
| content完成但 delimiter partial/失败 | `Unknown` |
| content+delimiter完成但 flush失败 | `Unknown` |
| full content + reset + delimiter + flush成功，ledger insert成功 | `Committed { Written }` |
| ledger已有same完整ID+identity | 不写terminal，`Committed { AlreadyCommitted }`且共享original receipt |
| ledger已有same完整ID+different identity | `NotCommitted(IdentityConflict)` |

coordinator拆借得到private `InlineNativeCommitContext { shell, session }`；transaction在首次clear
前保存pre-attempt projection并安装guard，计算primary后先原子apply shell transition，再从shell
取得post-attempt projection并立即full repaint，之后才构造public outcome。unwind用pre-attempt
projection恢复；没有public出口可停在clear与repaint之间。repaint失败进入ordered nonempty
`ScrollbackCleanupErrors`并保留source，绝不改primary三态；Committed仍保留confirmed事实，
coordinator转typed restoration-required。fake/PTY覆盖first-write/repaint/双失败，要求返回前
已恢复live transcript/composer或显式报告cleanup failure。

ledger insert 必须在 flush 成功后；native write和insert仍不是 crash-atomic。insert 前容量预检，
容量满时在写任何 byte前返回 `NotCommitted(LedgerCapacityExhausted)`；confirmed entries不
evict。这样不会出现“已经写入后才发现无处记录”。若进程在 flush/write 可见后、insert前崩溃，
重启只能 unknown；native session从不恢复该 ledger。

cancellation固定在begin前、每次`Write::write`返回后、每个content segment后、reset/delimiter
前后、flush前后和ledger insert前采样；fake writer用barrier让另一线程在每个点flip token，
从而mid-commit分支不是预置fixture。fault matrix还逐项覆盖progressive shorts、
short-then-zero/error/cancel以及content/CRLF/reset/delimiter每个offset。control permit在调用
sink前进入、所有outcome/unwind后RAII释放；custom sink在outer permit存活时调用
`try_begin_nested_attempt`可稳定得到`ReentrantCommit`，外层state不变。

### 4. Durable sink 与跨重试/重启

持久实现显式实现第二个 trait：

```rust
pub trait DurableScrollbackSink: ScrollbackSink {
    fn lookup(
        &mut self,
        request: ScrollbackLookupRequest<'_>,
    ) -> DurableScrollbackLookupOutcome;
}
```

`DurableScrollbackLookupOutcome` closed 为
`Committed(DurableScrollbackRecord) | NotCommitted | Unknown(ScrollbackCommitError)`。
`DurableScrollbackRecord`包含完整namespace/ID、original receipt handle、SHA-256 content
identity、private canonical bytes payload、`ScrollbackProjectionContext { width,
theme_identity }`和store
sequence。实现者合同：

- request必须携带第一次staging冻结的projection context；`namespace + commit_id + digest +
  raw payload + visible effect + receipt + projection context`
  在同一 durable transaction去重；
- concurrent same完整ID/identity只有一次 effect，全部共享相同 original receipt，各attempt
  disposition独立；不同namespace永不碰撞；
- same ID/different identity 原子 conflict；
- lookup先按完整commit ID返回stored record，shell随后以stable message ID/revision验证；
  不允许caller先传当前width/theme projection筛掉历史record；
- store unavailable、timeout、corrupt record、无法判断 transaction结果返回 Unknown，不能
  返回空/NotCommitted fallback。

shell 不提供可 serde/clone restore 的私有 ledger snapshot。restart 流程是：

restart顺序固定为：GH-62 validated snapshot restore → GH-66 restored constructor按source-order
seed IDs且不project → durable ID-first lookup → Committed验证digest/context/receipt并remove；
NotCommitted才按current context创建candidate；Unknown保留frozen provenance并block。

因此 serialization boundary 唯一属于 GH-62 validated snapshot 和 injected durable store；
GH-66 不新增 serde dependency或未验证 wire struct。`InlineShellObservation` 是公共只读观察，
明确不能作为 restore input。`try_new`固定Fresh；`try_restore`要求显式
`InlineSessionProvenance::{RestoredDurable { recovery_id },
RestoredAfterUncleanNativeExit { recovery_id, recovery_records }}`。每个pre-crash record必须来自
应用注入的可信store并含exact first-staged bytes、crate重算digest及projection context；匹配者
进入recoverable Unknown。缺失、digest/context不匹配或无法验证者进入
`UnrecoverableUnknown`，不lookup/重投影current content，`TreatAsCommitted` typed拒绝且仅允许
durable-audited Abandon。
Fresh与restored provenance在observation可读，不能由default隐式选择。

### 5. Inline shell state、staging 和状态机

```text
InlineChatShell {
  revision: InlineShellRevision,
  lifecycle: Fresh | Running | ShuttingDown | Restoring | Shutdown,
  provenance: InlineSessionProvenance,
  namespace: ScrollbackNamespace,
  candidates: ordered bounded entries + O(1) ID index,
  confirmed: bounded observation index,
  resolution_audit: bounded append-only references,
  last_outcome: Option<InlineCommitObservation>,
  commit_control: ScrollbackCommitControl,
  composer_focus: InlineFocusState
}

InlineCommitPhase =
  Live |
  Staged |
  NotCommitted |
  Unknown |
  UnrecoverableUnknown |
  ResolvedCommitted |
  Abandoned |
  Confirmed
```

constructor：

```text
InlineChatShell::try_new(
  InlineChatShellConfig,
  ScrollbackNamespace
) -> Result<Self, InlineChatShellError>
InlineChatShell::try_restore(
  InlineChatShellConfig,
  ScrollbackNamespace,
  InlineSessionProvenance, // runtime拒绝Fresh
  &ConversationState,
  InlineRenderContext<'_>
) -> Result<Self, InlineChatShellError>
InlineChatShellConfig::new(
  candidate_capacity: NonZeroUsize,
  confirmed_capacity: NonZeroUsize
)
```

核心方法：

```text
bootstrap(
  &mut self,
  conversation: &ConversationState,
  render: InlineRenderContext<'_>
) -> Result<InlineShellTransition, InlineChatShellError>

synchronize(
  &mut self,
  conversation: &ConversationState,
  outcome: &ApplyOutcome,
  render: InlineRenderContext<'_>
) -> Result<InlineShellTransition, InlineChatShellError>

try_commit_next<S: ScrollbackSink>(
  &mut self,
  sink: &mut S,
  cancellation: &ScrollbackCancellationToken
) -> Result<InlineCommitStep, InlineChatShellError>

retry_not_committed<S: ScrollbackSink>(
  &mut self,
  commit_id: &ScrollbackCommitId,
  sink: &mut S,
  cancellation: &ScrollbackCancellationToken
) -> Result<InlineCommitStep, InlineChatShellError>

reconcile_durable<S: DurableScrollbackSink>(
  &mut self,
  sink: &mut S
) -> Result<InlineRecoveryReport, InlineChatShellError>

resolve_unknown<A: UnknownResolutionAuditSink>(
  &mut self,
  commit_id: &ScrollbackCommitId,
  resolution: UnknownResolution,
  audit: &mut A
) -> Result<InlineUnknownResolutionReport, InlineChatShellError>

observe(&self) -> InlineShellObservation<'_>
try_project_live(...) -> Result<InlineLiveProjection, InlineChatShellError>
```

`try_new`只能建立Fresh空shell；`bootstrap`只允许该shell且candidate/index为空时调用一次。
`try_restore`只接受闭集`RestoredDurable | RestoredAfterUncleanNativeExit`，实际消费GH-62
validated Conversation与recovery render context，按source order做一次O(n) scan并为每个稳定
terminal message seed完整namespace/message/revision ID与source-order slot。durable seed不先
project，等待ID-first lookup；unclean native只从validated recovery record冻结原bytes/digest/
context并进入Unknown，缺record进入UnrecoverableUnknown。
非空history为空结果、ID缺失或projection失败均typed fail atomic。此后`synchronize`只消费
最终GH-62 `ApplyOutcome::affected_messages()`返回的borrowed slice和当前immutable snapshot，
不能遍历未受影响history：

1. 验证outcome revision与affected entries唯一；严格保持accessor返回的deterministic order，
   每项通过`AffectedMessage::message_id()`取ID并先读`AffectedMessage::disposition()`；
   只有`Present`才从post-apply snapshot lookup并按既有frozen-first规则project/insert；
2. `Deleted`绝不snapshot lookup，只用该affected ID/revisions定位index：Live、Staged和
   NotCommitted（均无accepted effect）删除并留typed tombstone；Unknown/UnrecoverableUnknown保留frozen
   bytes/evidence、标`source_deleted`且继续block；ResolvedCommitted/Abandoned保留append-only
   resolution audit并归档；Confirmed只移除live/source index，terminal scrollback与ledger不变；
3. source-order accessor只用于Present新candidate；same ID/identity no-op，conflict fail atomic。

Cancelled/Failed content保留 status/failure cause presentation，transport
`Committed`不改变其 conversation status。staged content此后不随 resize/theme重建；live
内容可重投影。bootstrap复杂度O(n)，delta路径为O(a log n + p)，`a`是affected IDs、`p`是
首次terminal projection工作；operation counter锁定不访问`n-a`条历史。candidate/confirmed
达到容量前先检查，满则state不变。

每次方法先计算完整 candidate和下一 revision，再一次 commit；revision/sequence/counter用
`checked_add`。no-op 不增 revision。删除 live message只发生在 sink返回`Committed`并验证
receipt ID/digest/context一致，或exact `TreatAsCommitted` audit已append+flush后。后者预计算
无失败transition，一次写`ResolvedCommitted`、移除live/source index并归档safe observation；
若进程在audit后失效，exact duplicate lookup重放同一transition且零第二visible effect。
`NotCommitted`/两种Unknown都记录观察并保留candidate；两种Unknown阻塞全部后继。

### 6. Precedence、retry、reentrancy 与并发

失败优先级固定：

```text
Shutdown/closing
-> ReentrantCommit
-> shell revision/counter overflow
-> upstream stale/illegal terminal state
-> capacity
-> same-ID content conflict
-> predecessor Unknown/order blocked
-> retry policy
-> sink outcome/receipt validation
```

- 普通API需要 `&mut self`，所以一个shell在线程内串行；reentry guard不再依赖不可达的第二次
  shell borrow。shell在调用sink前通过concrete
  `ScrollbackCommitControl`取得permit；request把同一control共享给sink。
- sink不得回调shell，但可以安全调用`request.control().try_begin_nested_attempt()`；outer
  permit存在时该production API返回`ReentrantCommit`，不会unsafe alias或borrow panic。fake
  sink必须通过这条入口证明外层state/receipt/ledger原子。
- private pure `order_satisfied(phase)`唯一闭集为
  `Confirmed | ResolvedCommitted | Abandoned`；`try_commit_next`、`retry_not_committed`、
  `resolve_unknown`都逐个前序调用它，禁止各自写literal Confirmed特判。
- `retry_not_committed`只接受当前phase确为NotCommitted、ID/digest/context相同且全部前序满足上述
  predicate；其他phase typed拒绝。`resolve_unknown`先验证evidence shape，再按完整ID查询audit：
  existing exact choice/evidence fingerprint/digest/context在phase检查前返回
  `AlreadyResolved(receipt)`；existing conflict typed拒绝且零mutation。只有audit absent才检查
  phase/order；TreatAsCommitted只接受recoverable Unknown，UnrecoverableUnknown只接受Abandon；
  `UnknownResolution`闭集为
  `TreatAsCommitted { evidence: ManualCommitEvidence }`与
  `Abandon { evidence: ManualAbandonEvidence }`。两种evidence均含validated nonempty
  audit ID/reason/actor scope和exact namespace/ID/digest/projection identity，不含raw bytes。
  `UnknownResolutionAuditSink`必须先append+flush durable record并返回receipt，再执行上述无失败
  shell transition；duplicate exact resolution幂等，conflict/unknown ID/audit failure零mutation。
  TreatAsCommitted建立`ResolvedCommitted`且从live projection/source index原子移除，但不伪造
  native original receipt；Abandon保留safe audit/status且不再允许写该candidate。
- exact matrix覆盖两种resolution后后继NotCommitted retry、后继Unknown resolution，以及
  restored history含多个Unknown时只允许按source order逐项解决。
- durable concurrent去重属于 sink原子存储合同；shell不标 `Sync`，native session/ledger不
  宣称跨线程或跨进程。
- `Committed` receipt错ID/digest/context是 sink contract violation，shell转 Unknown并保留live，
  绝不信任错误receipt。

### 7. Public observation，不可伪造的恢复边界

`InlineShellObservation<'a>` 借用 shell，公开 accessor：

```text
revision()
lifecycle()
focus()
live_message_ids()
commit_entries() -> &[InlineCommitEntryObservation]
last_commit_outcome()
order_blocker()
native_restart_guarantee()
session_provenance()
unknown_resolution_audit_refs()
```

每个 entry observation公开 ID、content identity、source-order ordinal、terminal status、
phase、last typed outcome、confirmed receipt（若存在）。content bytes/secret不由 Debug/
Display输出。observation没有 public fields、serde derive或 `try_restore`；tests必须证明
clone/roundtrip不是恢复入口。durable restart只使用第4节 GH-62 snapshot + lookup；fresh/native
restored provenance与manual resolution audit ref可读但不能改写。

### 8. Composer、focus 与 live projection

GH-66不扩展 `InteractionMode`/`InteractionOutcome<T>`。shell handler：

```text
handle_composer_outcome(
  &mut self,
  outcome: InteractionOutcome<ComposerPayload>
) -> Result<InlineShellInteractionOutcome, InlineChatShellError>

route_focus(
  &mut self,
  command: InlineFocusCommand,
  mode: InteractionMode
) -> Result<InlineFocusOutcome, InlineChatShellError>
```

`InlineShellInteractionOutcome` closed variants：
`Composer(InteractionOutcome<ComposerPayload>) | Focus(InlineFocusOutcome) |
ExitRequested(InlineExitReason)`。它是具体 wrapper，不是 alias。Disabled先Ignored；
ReadOnly按GH-64阻止value mutation；composer Cancelled只上报Composer(Cancelled)，不会自动
commit active message或exit。explicit `InlineExitReason::{UserRequested, EndOfInput,
ApplicationCancelled}` 与 composer cancel分离。

`try_project_live`按 source order只渲染未confirmed terminal candidates、active messages、
typed commit/status line和composer。它不读取terminal history、不清屏、不对confirmed内容
重新wrap。resize只改变live projection，不改变candidate bytes、draft/focus/ledger。

### 9. Sanitization 与 terminal trust boundary

`ScrollbackContent::try_from_rendered`执行：

```text
valid UTF-8
-> normalize CRLF/CR to LF
-> parse bytes
-> accept printable Unicode + LF
-> accept only complete CSI SGR: ESC '[' [0-9;]* 'm'
-> reject ESC not followed by allowed SGR, OSC/DCS/APC, cursor/title/clipboard,
   C0 other than LF, DEL, C1
-> reject empty/only-SGR/only-whitespace transcript
-> freeze private Arc bytes + domain-separated SHA-256 digest
```

不接受tab（renderer应投影为空格），不静默strip危险控制；否则纯文本语义和identity会变化。
parser对renderer允许的SGR参数执行closed validation并跟踪default/nondefault style，拒绝
malformed/unsupported参数；不要求canonical content自行balanced，因为transport encoder在每次
content后无条件写唯一`\x1b[0m`，该reset不进入identity但进入fault accounting。错误只含
category/range。sink、shell observation和Display不输出content。safe ANSI fixtures覆盖color、
conceal、inverse、nested reset、truncated CSI、OSC52、title、cursor、C1、NUL、混合Unicode，
PTY证明reset后delimiter/live/status/composer无style泄漏，partial reset为Unknown。

### 10. Native session coordinator、terminal lease 与 restoration

public `InlineSessionCoordinator`唯一拥有`InlineChatShell`、`NativeInlineSession`、event intake、
cancellation handle与process-wide `InlineTerminalLease`；应用不能分别取得可独立shutdown的shell/
session owner。native session内部拥有`Terminal`、nonzero confirmed ledger、validated
`InlineTerminalSnapshot`和逐阶段lifecycle：

```text
InlineSessionCoordinator::try_enter(fresh_shell_config, namespace, session_config)
  -> Result<InlineSessionCoordinator, InlineSessionEnterError>
InlineSessionCoordinator::try_enter_restored(
  shell_config, namespace, InlineSessionProvenance, &ConversationState, InlineRenderContext<'_>, session_config
) -> Result<InlineSessionCoordinator, InlineSessionEnterError>

bootstrap(&mut self, &ConversationState, InlineRenderContext<'_>) -> Result<...>
synchronize(&mut self, &ConversationState, &ApplyOutcome, InlineRenderContext<'_>) -> Result<...>
try_commit_next_native(&mut self) -> Result<InlineCommitStep, ...>
retry_not_committed_native(&mut self, &ScrollbackCommitId) -> Result<InlineCommitStep, ...>
try_commit_next_with<S: ScrollbackSink>(&mut self, &mut S) -> Result<...>; retry_not_committed_with<S: ScrollbackSink>(&mut self, id, &mut S) -> Result<...>
reconcile_durable<S: DurableScrollbackSink>(&mut self, &mut S) -> Result<InlineRecoveryReport, ...>
resolve_unknown<A: UnknownResolutionAuditSink>(&mut self, id, resolution, &mut A) -> Result<...>
current_cancellation_handle(&self) -> Option<InlineCancellationHandle>
try_suspend(&mut self) -> InlineSuspendOutcome
try_resume(&mut self) -> InlineResumeOutcome
observe(&self) -> InlineShellObservation<'_>
render_live(&mut self, &Element) -> Result<(), InlineSessionError>; poll_event(&mut self, Duration) -> Result<Option<Event>, InlineSessionError>
try_shutdown(&mut self) -> InlineShutdownOutcome; try_recover_poisoned_lease(backend) -> Result<(), InlineLeaseRecoveryError>
```

不公开shell/session/sink的可变borrow。每个wrapper先验证coordinator typestate，再在private实现中
安全拆借disjoint `&mut shell`/`&mut session`，native commit只在该scope构造sink并在render前
drop；durable wrapper同样持有single-in-flight guard。crate外exact test只经coordinator完成
fresh bootstrap、delta、native/durable commit、retry、reconcile、两种resolution、per-attempt
cancel、suspend/resume、render和shutdown，证明public surface实际可调用，而非仅能取得sink。

`InlineShutdownOutcome`闭集为`Complete(report) | RetryRequired(report) | AlreadyShutdown`；
`InlineSuspendOutcome`闭集为`Suspended(report) | RetryRequired(report) | AlreadySuspended`；
`InlineResumeOutcome`闭集为`Running(report) | RetryRequired(report) | AlreadyRunning`。
lease process state闭集为`Free | Held(holder) | Poisoned(restoration stages)`。

`terminal/lease.rs`拥有唯一process-wide non-poisoning registry。`Terminal`在enter/enter_inline
前取得guard并保存到exit/Drop；suspend保留同一guard；resume复用它。`TerminalController`
mode/cmd/println/resize只能经`Terminal::with_lease`验证holder；panic hook调用registry的typed
emergency recovery，不直接crossterm。legacy `restore_terminal()`签名保持但只是best-effort
wrapper，失败会poison registry且不得打印成功；新增private/typed source供coordinator/PTY断言。
`try_enter`在任何terminal mutation前取得exclusive lease；
同线程nested、不同shell和跨线程竞争均返回`TerminalLeaseAlreadyHeld`。lease中记录opaque holder
ID，不泄漏线程/内容。entry acquisition闭集为：

```text
Lease -> CapabilitySnapshot -> RawMode -> CursorHidden
      -> BracketedPasteConfigured -> EntryFlush
```

`InlineTerminalSnapshot`由可注入backend查询并验证screen/raw/cursor/paste初值；session配置与
backend没有focus/mouse mutation命令，并拒绝产生相应enable/disable escape。alternate screen
或无法建立required state时typed Unsupported，不能猜默认。任一entry stage失败都按已完成阶段
逆序rollback并继续尝试全部步骤；`InlineSessionEnterError`同时保存primary cause与ordered
rollback failures。rollback全部成功后lease回Free；任一步失败则lease变Poisoned并携带未完成
stage，error显式标记`RestorationUncertain`且后续entry blocked，直到typed recovery成功。
fake/PTY在每个acquire stage注入失败。

coordinator shutdown使用显式dependency DAG：

```text
stop event intake + cancel current generation + revoke it
-> shell expose pending/Unknown/manual-resolution-required outcomes
-> clear live region
-> restore paste
-> show/restore cursor
-> restore raw/screen
-> flush
-> release terminal lease
```

每阶段状态为`Pending | Completed | Failed(source)`并声明dependencies；一次调用即使失败也继续
全部可安全执行的后续步骤并返回`RetryRequired`，coordinator保持Restoring且仍拥有lease。retry
执行Pending/Failed；任何可能写terminal bytes的stage重跑前原子把其下游Flush与LeaseRelease
改回Pending，写后必须fresh flush，release的唯一precondition是该generation的Flush Completed。
全部required stages成功才Shutdown。partial shutdown期间sink/render/poll/second enter typed拒绝。
suspend从Running用同一DAG恢复modes、保留lease/shell并进入Suspended；resume只从Suspended按
entry DAG重获modes并full repaint，失败保持typed retryable state；fresh restart选择显式provenance。

Drop与panic hook调用同一private阶段表best-effort；全部成功则lease回Free，任一恢复失败则
lease标Poisoned并阻止新entry。只有显式`try_recover_poisoned_lease`重跑未完成阶段且外部
snapshot验证成功后才能Free，仍不得把先前Drop说成成功。PTY/fake覆盖normal/cancel/failure/panic、
entry partial failure、shutdown first-failure/second-success（含旧flush后output stage重试必须
再次flush）、new/legacy/controller/panic lease contention、public suspend/resume：raw关闭、
cursor/paste恢复prior值、无alt-screen/focus/mouse序列、style已reset。
无PTY环境时required job blocked，不能pass。

### 11. Example、exports 与兼容

- `src/components/chat/inline.rs`启用与GH-62 chat root一致的missing-doc纪律，导出全部 concrete
  public types；child不能`allow/expect(missing_docs)`或`doc(hidden)`逃逸。
- `components` root与prelude re-export同一类型，不提供第二命名。
- `docs/CORE_COMPONENT_CONTRACTS.md`说明sync sink、三态、native/durable边界、retry、
  observation、focus、shutdown与non-goals。
- 手工迁移`examples/claude_input_box.rs`。production helper
  `exercise_inline_chat_contract`由main与exact test共用，执行stream delta、complete、
  duplicate complete、commit、composer submit/cancel与shutdown；test逐字段断言public
  observation/receipt/live removal。example不保留private char/wrap/ledger/ANSI。
- legacy `println`/Message/render exports不变；source audit只作为辅助，public behavior test才是
 主要迁移证据。

## Product-to-Test Mapping

| Invariant | Implementation area | Executable verification |
| --- | --- | --- |
| B-001 | chat inline module/upstream public API | `cargo test --test prelude_surfaces --locked inline_chat_shell_public_surface_executes -- --exact` |
| B-002 | namespace + SHA-256 digest + frozen context | lib exact `stable_commit_identity_conflict_is_atomic`（含双namespace/同bytes不同context） |
| B-003 | terminal candidate filter | lib exact `gh66_scrollback_lifecycle_contract` |
| B-004 | sink trait/outcome | crate-outside exact `closed_scrollback_outcomes_are_exhaustive` |
| B-005 | stable receipt + per-attempt disposition | lib exact `native_confirmed_dedup_is_process_local` |
| B-006 | primary classification + immediate repaint cleanup | lib exact `partial_write_flush_broken_pipe_outcomes_are_typed`; PTY exact |
| B-007 | per-attempt cancellation generation/Unknown | lib exact `partial_write_flush_broken_pipe_outcomes_are_typed`; retry exact |
| B-008 | native bounded ledger | lib exact `native_confirmed_dedup_is_process_local` |
| B-009 | O(n) bootstrap + disposition-first delta/delete phases | lib exact `duplicate_terminal_render_and_delta_are_single_effect` |
| B-010 | committed/manual-resolved atomic live removal | lib exact `gh66_scrollback_lifecycle_contract` |
| B-011 | audit-first duplicate resolution + order | lib exact `unknown_blocks_order_and_never_auto_retries` |
| B-012 | digest-only audit + exact/conflict duplicate | integration exact `not_committed_retry_is_explicit_and_unknown_retry_is_rejected` |
| B-013 | request-context durable transaction | integration exact `durable_sink_cross_retry_and_restart_reconstruction_is_exactly_once` |
| B-014 | exact recovery vs UnrecoverableUnknown | durable restart exact; `public_observation_is_not_a_restore_snapshot` |
| B-015 | composer outcomes | integration exact `composer_focus_cancel_and_failure_outcomes_remain_typed` |
| B-016 | focus/mode/resize | integration exact `composer_focus_cancel_and_failure_outcomes_remain_typed` |
| B-017 | empty/missing content | lib exact `empty_state_and_empty_commit_do_not_invent_data` |
| B-018 | progressive-short/write-zero/CRLF/reset/delimiter matrix | lib exact `partial_write_flush_broken_pipe_outcomes_are_typed` |
| B-019 | safe control reentry/concurrency | lib exact `revision_overflow_and_reentrancy_are_atomic`; durable integration exact |
| B-020 | checked counters/illegal transition | lib exact `revision_overflow_and_reentrancy_are_atomic` |
| B-021 | coordinator suspend/resume/cancel/shutdown | prelude exact `inline_chat_shell_public_surface_executes`; shutdown exact |
| B-022 | shared legacy lease + restoration DAG | PTY exact `inline_pty_restores_terminal_on_normal_cancel_failure_and_panic` |
| B-023 | public suspend/retry-flush/contention PTY | same PTY exact |
| B-024 | sanitizer + SHA-256 audit redaction | lib exact `sanitizer_rejects_terminal_control_injection` |
| B-025 | typed unrecoverable crash boundary | integration exact `native_restart_never_claims_unknown_terminal_effect` |
| B-026 | frozen-candidate-first resize | integration exact `live_resize_never_rewrites_confirmed_scrollback` |
| B-027 | public example | prelude exact `claude_example_uses_public_inline_shell_contract`; `cargo check --example claude_input_box --all-features --locked` |
| B-028 | legacy signatures + shared-lease contention | prelude exact `legacy_println_and_message_surfaces_remain_compatible`; PTY exact |
| B-029 | dependency gate | fresh preflight adapter + three `git merge-base --is-ancestor` commands |
| B-030 | distinct head/base/main/merge-base evidence | full verification + exact `gh66_current_head_coverage_contract` produce/validate |
| B-031 | atomic closure | read-only closure audit removes each evidence class in negative fixtures and requires current exact head |

“lib exact”统一为`cargo test --workspace --lib --locked
components::chat::inline::tests::<NAME> -- --exact`；integration统一为`cargo test --test
inline_chat_shell --locked <NAME> -- --exact`；PTY统一为`cargo test --test inline_chat_shell_pty
--locked inline_pty_restores_terminal_on_normal_cancel_failure_and_panic -- --exact`。

## GH-57 Coverage Contract

`tasks.md`包含唯一`gh57-critical-paths-v1`：version=1、issue=66、22个unique
`file+name+verification_command`，严格等于T2–T7创建的exact tests；集合和命令不得由环境传入。
producer读取committed ledger、merge-base diff与真实raw，生成canonical
`gh57-child-coverage-v1`。artifact固定包含schema/child、PR/head/base/merge-base、HEAD commit
timestamp、repository/tool、normalized collect command、稳定
`raw_artifact_name:"llvm-cov.json"`、raw SHA-256、changed executable及按ledger顺序的22项
`file/name/verification_command/executable/covered/percent`；不保存mktemp absolute path。

test的closed mode为`fixture|collect|produce|validate`，缺失/越界fail。fixture用scratch
raw/diff/ledger覆盖正常与missing/duplicate/extra critical、empty raw、wrong
hash/PR/head/base/merge-base、zero executable、79.99% changed、99.99% critical、empty command、
relative path。collect只验证fresh immutable facts与absolute writable destinations，不读取尚未
产生的raw、不写artifact。producer原子写；validator重读ledger/raw/diff并逐byte重建canonical
JSON，要求changed total>0且>=80%、22项total>0且100%，拒绝old SHA/command drift。

唯一current-head顺序：

```sh
set -euo pipefail
: "${GH66_PR_NUMBER:?GH66_PR_NUMBER is required}"
case "$GH66_PR_NUMBER" in ''|*[!0-9]*) exit 64;; esac
export GH66_PR_NUMBER
git fetch --prune origin main
export GH66_IMPLEMENTATION_HEAD_SHA="$(gh pr view "$GH66_PR_NUMBER" \
  --repo majiayu000/rnk --json headRefOid --jq .headRefOid)"
export GH66_PR_BASE_SHA="$(gh api "repos/majiayu000/rnk/pulls/$GH66_PR_NUMBER" --jq .base.sha)"
export GH66_CURRENT_MAIN_SHA="$(git rev-parse origin/main)"
export GH66_MERGE_BASE_SHA="$(git merge-base "$GH66_CURRENT_MAIN_SHA" \
  "$GH66_IMPLEMENTATION_HEAD_SHA")"
test "$(git rev-parse HEAD)" = "$GH66_IMPLEMENTATION_HEAD_SHA"
test -z "$(git status --porcelain)"
export GH66_EVIDENCE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/rnk-gh66.XXXXXX")"
trap 'rm -rf "$GH66_EVIDENCE_DIR"' EXIT
export GH66_RAW_COVERAGE="$GH66_EVIDENCE_DIR/llvm-cov.json"
export GH66_COVERAGE_EVIDENCE="$GH66_EVIDENCE_DIR/coverage.json"
GH66_COVERAGE_MODE=fixture cargo test --test inline_chat_shell --locked \
  gh66_current_head_coverage_contract -- --exact
GH66_COVERAGE_MODE=collect cargo llvm-cov --workspace --all-targets --all-features --locked \
  --json --output-path "$GH66_RAW_COVERAGE"
test -s "$GH66_RAW_COVERAGE" && test ! -e "$GH66_COVERAGE_EVIDENCE"
GH66_COVERAGE_MODE=produce cargo test --test inline_chat_shell --locked \
  gh66_current_head_coverage_contract -- --exact
GH66_COVERAGE_MODE=validate cargo test --test inline_chat_shell --locked \
  gh66_current_head_coverage_contract -- --exact
git fetch --prune origin main
test "$(gh pr view "$GH66_PR_NUMBER" --repo majiayu000/rnk \
  --json headRefOid --jq .headRefOid)" = "$GH66_IMPLEMENTATION_HEAD_SHA"
test "$(gh api "repos/majiayu000/rnk/pulls/$GH66_PR_NUMBER" --jq .base.sha)" = "$GH66_PR_BASE_SHA"
test "$(git rev-parse origin/main)" = "$GH66_CURRENT_MAIN_SHA"
test "$(git merge-base "$GH66_CURRENT_MAIN_SHA" "$GH66_IMPLEMENTATION_HEAD_SHA")" = \
  "$GH66_MERGE_BASE_SHA"
test -z "$(git status --porcelain)"
```

以上PR/head/PR-base/current-main/merge-base/raw/artifact env对fixture外三mode均mandatory；
40-hex、positive PR及portable absolute temp逐值校验。artifact分别保存四个SHA；fixture必须让
缺env、dirty、wrong head/base/main/merge-base、base与main误合并、nonportable path、empty output
及79.99/99.99阈值全部fail closed。collect command规范化后写入artifact。

## Verification Plan

规格 packet：

1. `git diff --check`
2. Markdown link check。
3. product B-ID = tech mapping = tasks `Covers:` union。
4. planned-changes唯一、issue=66、complete=true、paths/spec_refs为唯一repo-relative值。
5. tasks ownership DAG无并行shared writer；每task有compile checkpoint。
6. critical ledger set与tasks实际创建的exact tests相等。
7. 提取packet全部mandatory shell fence，逐个`bash -n`；正fixture exit 0，每个删env/命令失败/
   dirty/provenance drift/portable-temp失败的负fixture必须nonzero，且后续sentinel不得执行。
8. 从`https://github.com/majiayu000/specrail.git` checkout
   `23caa70e76904eaa82323208d645d5781a365649`，验证
   `checks/check_workflow.py` SHA-256为
   `8c791545f78d93649385ef0f9780454a7d4552f8da06da1fdee0de9cb8030a7e`，再运行
   `python3 checks/check_workflow.py --repo <GH66 mirror> --spec-dir specs/GH66`。

未来 implementation每个writer checkpoint：

```sh
set -euo pipefail
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
```

最终 current exact head：

```sh
set -euo pipefail
cargo test --workspace --all-targets --all-features --locked
cargo check --all-targets --all-features --locked
cargo check --example claude_input_box --all-features --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked
```

再逐名执行 Product-to-Test Mapping、PTY、coverage produce/validate，收集fresh CI、
reviewThreads、independent review、merge state和SpecRail PR gate。spec-only PR不运行或声称
未来cargo tests。

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| 上游spec与最终API漂移 | dependency CLOSED+merged ancestry gate；实现前重读final code/spec，漂移即更新packet |
| terminal write可见但ledger未写 | explicit Unknown/crash boundary；native不跨重启retry |
| flush/short write被当成zero effect | staged writer记录accepted bytes/delimiter，fault matrix全覆盖 |
| live render和commit交错 | session可变borrow使sink与render互斥；shell single in-flight |
| ledger无限增长或淘汰后重写 | NonZero有界容量；满时pre-write typed block，不逐出confirmed |
| raw bytes造成大复制/O(n²) | payload共享`Arc<[u8]>`；identity/audit仅固定digest |
| ANSI/control/style泄漏 | closed SGR parser+canonical reset；partial reset Unknown，PTY检查live无泄漏 |
| entry/shutdown部分失败 | exclusive lease、逆序entry rollback、逐stage retry；显式report才是成功证据 |
| current terminal.rs超800 | 拆lease/inline child并路由旧lifecycle，root保持<800 |

## Rollback

- implementation未merge：关闭/保留失败PR evidence，GH-66继续open；不改已merge依赖。
- 已merge：普通revert撤销inline module、terminal child adapter、exports/docs/example/tests；
  禁止force push。
- rollback不得恢复example的private state作为“公共fallback”；如需临时回到legacy example，
  必须明确标记不满足GH-66并保持issue未完成。
- durable store数据不由crate删除；revert前应用owner按commit ID保留/迁移其记录。native
  terminal历史不可修改。

## Handoff

- 当前只完成spec planning；不授权production edit、approval或merge。
- implementation owner必须先交dependency/preflight证据，再按tasks串行ownership执行。
- verification owner必须与writers分离，绑定current exact head重跑全部commands；writer自报、
  old artifact或visual demo不能替代。
