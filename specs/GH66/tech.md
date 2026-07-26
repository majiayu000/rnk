# Tech Spec：InlineChatShell、同步 scrollback sink 与 terminal 恢复

## Linked Issue

GH-66: https://github.com/majiayu000/rnk/issues/66

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":66,"complete":true,"paths":["specs/GH66/product.md","specs/GH66/tech.md","specs/GH66/tasks.md","src/components/chat/mod.rs","src/components/chat/inline.rs","src/components/chat/inline/types.rs","src/components/chat/inline/sink.rs","src/components/chat/inline/state.rs","src/components/chat/inline/session.rs","src/components/chat/inline/sanitize.rs","src/components/chat/inline/tests.rs","src/components/mod.rs","src/prelude.rs","src/renderer/terminal.rs","src/renderer/terminal/inline_scrollback.rs","examples/claude_input_box.rs","docs/CORE_COMPONENT_CONTRACTS.md","tests/inline_chat_shell.rs","tests/inline_chat_shell_pty.rs","tests/prelude_surfaces.rs"],"spec_refs":["specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md","specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md","specs/GH63/product.md","specs/GH63/tech.md","specs/GH63/tasks.md","specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md","specs/GH66/product.md","specs/GH66/tech.md","specs/GH66/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只拥有 Inline shell、scrollback commit/sink、native inline session、terminal 安全提交
适配、公共导出、一个 example 和对应 tests/docs。Conversation model/reducer 属于 GH-62，
message/block rendering 属于 GH-63，composer 属于 GH-64；implementation 只能以它们最终
merged public API 为准。

## Implementation Gate

spec-only 工作可继续；production implementation 现在 blocked。2026-07-26 fresh GitHub
evidence：

- #62、#63、#64 都是 `OPEN`；
- #62 closing PR #117 是 `OPEN`，head
  `4467b3121aed6548326d3df30016df4fafd226b2`，`mergeCommit=null`；
- #63/#64 没有 closing implementation PR；PR #75/#79 只是已合并 spec PR。

`SP66-T1` 开始前 coordinator 必须 fresh 分页收集三个 dependency issue、全部
`closedByPullRequestsReferences`、PR files/labels/reviews/checks 与 tasks evidence。每个 issue
必须 CLOSED；每个最终 closing PR 必须 MERGED、非 draft/parked、包含明确 executable Rust
source，且所有 merge commits 是 implementation base 的祖先。spec PR、绿色 CI、open branch
或手填 SHA 均不满足。还必须取得 GH-66 人工 spec approval 与 canonical
`ready_to_implement`。API/path 漂移时先更新三个 specs 并重新批准。

## Codebase Context

锚点基于 spec branch base
`3f21b049db4e6fe426f8c95270b517d10d92959b`，均已用 Read/grep 核实。

| Area | Current anchor | Current behavior | GH-66 decision |
| --- | --- | --- | --- |
| Inline example state | `examples/claude_input_box.rs:34`, `:132`, `:249`, `:407`, `:474` | 私有 `Vec<char>`、cursor、input handler、wrap 与 live viewport | 手工迁移为 public shell composition，删除全部私有生命周期 |
| Example commit | `examples/claude_input_box.rs:147`, `:157`, `:164` | submit 后连续 `app.println` 并立即 clear draft；无 write ack | 只能经 typed sink/receipt，ack 后再改变 shell state |
| Shared interaction | `src/components/interaction.rs:7`, `:37` | closed `InteractionMode` 与 `InteractionOutcome<T>` | 直接复用，不增 alias/catch-all，不扩展既有 exhaustive enum |
| Runtime paste | `src/renderer/runtime.rs:122`, `:165`, `:170` | Key/Mouse/Resize 分支；Paste 被 wildcard 丢弃 | GH-64 最终 merged path负责 paste；GH-66 只消费 composer outcome |
| Public println | `src/hooks/use_app.rs:93`, `:113`; `src/runtime/context.rs:332` | `println` 返回 `()`，只委托 handle | 保持兼容，不作为 confirmed commit |
| Runtime queue | `src/renderer/registry.rs:123`, `:147`, `:198`, `:257` | `AppSink::println` 无返回；queue 在 terminal write 前被 `take` | typed commit 不复用该 ack-less queue |
| Queue bridge | `src/renderer/runtime_bridge.rs:62`, `:67` | 先 drain 全 queue，再调用 controller；失败后无 per-message outcome | 保持 legacy path；新 sink 直接同步调用 terminal transaction |
| Terminal println | `src/renderer/terminal.rs:311`, `:348`, `:354`, `:365`, `:370` | 清 live UI、逐行 `write!`、flush，返回一个 `io::Result` | 提取 staged write helper，记录 first accepted byte/partial/flush stage |
| Inline terminal lifecycle | `src/renderer/terminal.rs:196`, `:211`, `:697` | enter/exit 返回 `io::Result`；Drop 吞 exit error | 新 session提供显式 typed shutdown；Drop只 best effort |
| Panic recovery | `src/runtime/panic_handler.rs:16`, `:23`, `:27`, `:30`, `:40` | restore 吞 disable/execute/flush errors，却打印“restored” | PTY成功声明不依赖该文本；显式 session report逐项返回 |
| Bracketed paste | `src/hooks/paste.rs:26`, `:35`, `:56`, `:71` | enable/disable 有 Result，guard Drop吞 error | session显式拥有并恢复 prior state，Drop不作成功证据 |
| Current exports | `src/components/mod.rs:12`, `:57`; `src/prelude.rs:75`, `:136` | 没有 `components::chat`；interaction/textarea/renderer已导出 | 在上游最终 chat module 上增加 `inline` concrete exports |
| Terminal file size | `src/renderer/terminal.rs:1` | 当前 791 行，接近 800 hard ceiling | 仅增加 child-module 声明；新逻辑放 `terminal/inline_scrollback.rs` |
| Dependency code | `src/components/mod.rs:1` and `rg ChatComposer/Conversation` | 当前 base 没有 GH-62/63/64 production types | implementation gate后在 merged base重新定位，禁止按 spec 猜字段 |

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
则是无 ack queue。把 commit 做成 async 会新增 executor/cancellation 边界，却仍无法令 terminal
write 与 ledger 原子。同步调用由 `InlineChatShell::try_commit_next(&mut sink)` 在一次
borrow 内完成，三态结果在返回前写入 shell observation。

`ScrollbackSink` 不接受 `Any`、字符串 error、任意 callback 或 boxed future。生产
`NativeScrollbackSink<'a>` 只由 `NativeInlineSession::scrollback_sink()` 构造，借用同一
session 的 `Terminal` 和 process-local ledger；borrow 结束后 session 才能 `render_live`，
从类型层防止 commit/write 与 live render 同时执行。

### 2. 公共 commit types

所有 struct 字段 private，提供具名 constructor/accessor；没有 public alias。可扩展 behavior
enum 标 `#[non_exhaustive]`，必须穷举的 outcome/error family 保持 closed 并由 crate 外
compile fixture 无 wildcard match。

```text
ScrollbackCommitId {
  message_id: MessageId,
  terminal_revision: MessageRevision
}

ScrollbackContent {
  bytes: Arc<[u8]>,             // exact validated UTF-8 + allowed SGR stream
  identity: ScrollbackContentIdentity
}

ScrollbackContentIdentity {
  exact_bytes: Arc<[u8]>        // 与 content 共享 allocation，clone O(1)
}

ScrollbackCommitRequest<'a> {
  commit_id: &'a ScrollbackCommitId,
  content: &'a ScrollbackContent
}

ScrollbackGuarantee =
  ProcessLocalConfirmed |
  DurableAtomicIdempotency

CommittedDisposition = Written | AlreadyCommitted

ScrollbackCommitReceipt {
  commit_id,
  content_identity,
  disposition,
  session_sequence
}

ScrollbackCommitOutcome =
  Committed(ScrollbackCommitReceipt) |
  NotCommitted(ScrollbackCommitError) |
  Unknown(ScrollbackCommitError)
```

不增加 hash dependency，也不使用不稳定 `DefaultHasher`。identity 是 exact immutable bytes；
request、candidate、receipt 和 ledger 共享同一 `Arc<[u8]>`，避免每次 duplicate check deep
copy。`ScrollbackCommitId::new(MessageId, MessageRevision)` 的参数本身已经由 GH-62
validated constructor产生；same ID/different exact bytes返回
`ScrollbackCommitError::ContentIdentityConflict`。

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
```

`ScrollbackIoError` 保存 `ScrollbackIoStage::{Begin, WriteContent, WriteDelimiter, Flush}`、
`accepted_content_bytes`、`delimiter_bytes` 与原 `std::io::Error`；实现 `Error::source`。
Display 只输出 stage/count/error kind，不回显 content。`ScrollbackContentError` 保存安全
category 与 byte range，不保存或显示原始 control/secret。

### 3. Native write transaction 与三态分类

`src/renderer/terminal/inline_scrollback.rs` 为现有 `Terminal` 增 crate-visible staged helper，
并以 private generic `W: Write` helper做故障注入测试。production仍写 stdout，不修改 legacy
`Terminal::println` 签名/语义：

```text
begin: clear current live region
-> write validated content bytes until complete
-> write exactly one CRLF delimiter
-> flush
-> repaint marker
```

分类固定：

| 观察 | Outcome |
| --- | --- |
| begin/preflight失败；content accepted=0 | `NotCommitted` |
| first content write在接受任何 byte前失败 | `NotCommitted` |
| content short write后失败/WriteZero | `Unknown` |
| content完成但 delimiter partial/失败 | `Unknown` |
| content+delimiter完成但 flush失败 | `Unknown` |
| cancellation在begin前 | `NotCommitted(Cancelled)` |
| cancellation在任意 accepted byte后 | `Unknown(Cancelled)` |
| full content + delimiter + flush成功，ledger insert成功 | `Committed(Written)` |
| ledger已有same ID+identity | 不写terminal，`Committed(AlreadyCommitted)` |
| ledger已有same ID+different identity | `NotCommitted(IdentityConflict)` |

ledger insert 必须在 flush 成功后；native write和insert仍不是 crash-atomic。insert 前容量预检，
容量满时在写任何 byte前返回 `NotCommitted(LedgerCapacityExhausted)`；confirmed entries不
evict。这样不会出现“已经写入后才发现无处记录”。若进程在 flush/write 可见后、insert前崩溃，
重启只能 unknown；native session从不恢复该 ledger。

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
`Committed(receipt) | NotCommitted | Unknown(ScrollbackCommitError)`。实现者合同：

- `commit_id + exact identity + visible effect + receipt` 在同一 durable transaction去重；
- concurrent same ID/identity只有一次 effect，全部返回相同 durable receipt；
- same ID/different identity 原子 conflict；
- lookup只在 durable record存在且 identity相同时返回 committed；
- store unavailable、timeout、corrupt record、无法判断 transaction结果返回 Unknown，不能
  返回空/NotCommitted fallback。

shell 不提供可 serde/clone restore 的私有 ledger snapshot。restart 流程是：

```text
persisted GH-62 ConversationStateSnapshot
-> GH-62 ConversationState::try_restore (验证全部 identity/revision/history)
-> new InlineChatShell
-> stage terminal messages using exact final revision + current validated projection
-> InlineChatShell::reconcile_durable(&mut DurableScrollbackSink)
-> lookup each candidate in source order
-> only lookup Committed reconstructs confirmed/remove state
```

因此 serialization boundary 唯一属于 GH-62 validated snapshot 和 injected durable store；
GH-66 不新增 serde dependency或未验证 wire struct。`InlineShellObservation` 是公共只读观察，
明确不能作为 restore input。projection bytes与 durable record不一致即 identity conflict，
不得用新宽度覆盖历史。

### 5. Inline shell state、staging 和状态机

```text
InlineChatShell {
  revision: InlineShellRevision,
  lifecycle: Running | ShuttingDown | Shutdown,
  candidates: ordered bounded entries,
  confirmed: bounded observation index,
  last_outcome: Option<InlineCommitObservation>,
  commit_in_progress: bool,
  composer_focus: InlineFocusState
}

InlineCommitPhase =
  Live |
  Staged |
  NotCommitted |
  Unknown |
  Confirmed
```

constructor：

```text
InlineChatShell::new(InlineChatShellConfig) -> Result<Self, InlineChatShellError>
InlineChatShellConfig::new(
  candidate_capacity: NonZeroUsize,
  confirmed_capacity: NonZeroUsize
)
```

核心方法：

```text
synchronize(
  &mut self,
  conversation: &ConversationState,
  render: InlineRenderContext<'_>
) -> Result<InlineShellTransition, InlineChatShellError>

try_commit_next<S: ScrollbackSink>(
  &mut self,
  sink: &mut S
) -> Result<InlineCommitStep, InlineChatShellError>

retry_not_committed<S: ScrollbackSink>(
  &mut self,
  commit_id: &ScrollbackCommitId,
  sink: &mut S
) -> Result<InlineCommitStep, InlineChatShellError>

reconcile_durable<S: DurableScrollbackSink>(
  &mut self,
  sink: &mut S
) -> Result<InlineRecoveryReport, InlineChatShellError>

observe(&self) -> InlineShellObservation<'_>
try_project_live(...) -> Result<InlineLiveProjection, InlineChatShellError>
begin_shutdown(&mut self) -> Result<InlineShutdownTransition, InlineChatShellError>
```

`synchronize`只读上游 state：

1. 按 Conversation source order枚举 messages；
2. Pending/Streaming保持 live；
3. Complete/Cancelled/Failed 还须证明全部 nested lifecycle terminal；
4. 读取 exact `MessageId`/`MessageRevision`；
5. GH-63 `ChatMessageView` 生成一次 terminal projection，以当前 width冻结；
6. sanitizer成功后以 `(MessageId, terminal revision)` stage exact bytes；
7. 已有 same ID/identity为 no-op；same ID/different bytes fail atomic。

Cancelled/Failed content保留 status/failure cause presentation，transport
`Committed`不改变其 conversation status。staged content此后不随 resize/theme重建；live
内容可重投影。candidate/confirmed达到容量前先检查，满则 state不变。

每次方法先计算完整 candidate和下一 revision，再一次 commit；revision/sequence/counter用
`checked_add`。no-op 不增 revision。删除 live message只发生在 sink返回
`Committed`并验证receipt ID/identity一致后。`NotCommitted`/`Unknown`都记录观察但保留
candidate；Unknown将它和所有后续 candidate设为order-blocked。

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

- 普通API需要 `&mut self`，所以一个 shell在线程内串行；`commit_in_progress` 仍覆盖 sink
  reentrant callback/测试端口重入并 typed拒绝。
- sink在 `commit` 期间不得回调 shell；若测试端口尝试重入，外层state保持原子。
- `retry_not_committed`只接受当前phase确为NotCommitted、ID/identity相同、前序已confirmed；
  Unknown、Confirmed、Live、Staged普通retry都 typed拒绝。
- durable concurrent去重属于 sink原子存储合同；shell不标 `Sync`，native session/ledger不
  宣称跨线程或跨进程。
- `Committed` receipt错ID/identity是 sink contract violation，shell转 Unknown并保留live，
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
```

每个 entry observation公开 ID、content identity、source-order ordinal、terminal status、
phase、last typed outcome、confirmed receipt（若存在）。content bytes/secret不由 Debug/
Display输出。observation没有 public fields、serde derive或 `try_restore`；tests必须证明
clone/roundtrip不是恢复入口。durable restart只使用第4节 GH-62 snapshot + lookup。

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
-> freeze Arc bytes + exact identity
```

不接受tab（renderer应投影为空格），不静默strip危险控制；否则纯文本语义和identity会变化。
错误只含 category/range。sink、shell observation和Display不输出content。provider/tool/secret
不进入error；application仍负责是否把敏感内容加入Conversation。safe ANSI parser以正负
fixtures覆盖合法SGR、truncated CSI、OSC52、window title、cursor move、C1、NUL和混合Unicode。

### 10. NativeInlineSession 与 restoration

`NativeInlineSession`唯一拥有 `Terminal`、nonzero confirmed ledger、bracketed-paste prior
state和lifecycle：

```text
NativeInlineSession::try_enter(config) -> Result<Self, InlineSessionError>
scrollback_sink(&mut self) -> NativeScrollbackSink<'_>
render_live(&mut self, &Element) -> Result<(), InlineSessionError>
poll_event(&mut self, Duration) -> Result<Option<Event>, InlineSessionError>
try_shutdown(&mut self) -> Result<InlineShutdownReport, InlineSessionError>
```

`try_enter`仅进入inline raw mode，记录进入前screen/paste状态，不进入alternate screen。
`try_shutdown`阶段顺序和每阶段结果写入`InlineShutdownReport`；任何阶段失败继续尝试其余恢复，
最终返回包含全部 causes的 typed error/report，不能first-error后跳过cursor/raw restore。
第二次shutdown返回`AlreadyShutdown` no-op。shutdown后sink/render/poll均typed拒绝。

Drop与panic hook调用相同private best-effort steps但不能返回report；Debug/log不能说
“restored”。PTY子进程通过termios和captured ANSI验证normal/cancel/typed-failure/panic：
raw关闭、`\x1b[?25h`、`\x1b[?2004l`或恢复prior值、无
`\x1b[?1049h`/`\x1b[?1049l`。测试必须非ignored；无PTY环境则required job blocked，不能pass。

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
| B-002 | commit ID/content staging | lib exact `stable_commit_identity_conflict_is_atomic` |
| B-003 | terminal candidate filter | lib exact `gh66_scrollback_lifecycle_contract` |
| B-004 | sink trait/outcome | crate-outside exact `closed_scrollback_outcomes_are_exhaustive` |
| B-005 | receipt/flush/ledger | lib exact `native_confirmed_dedup_is_process_local` |
| B-006 | zero-effect classification | lib exact `partial_write_flush_broken_pipe_outcomes_are_typed` |
| B-007 | Unknown classification/policy | lib exact `partial_write_flush_broken_pipe_outcomes_are_typed`; `unknown_blocks_order_and_never_auto_retries` |
| B-008 | native bounded ledger | lib exact `native_confirmed_dedup_is_process_local` |
| B-009 | duplicate terminal/render/delta | lib exact `duplicate_terminal_render_and_delta_are_single_effect` |
| B-010 | confirmed-only removal | lib exact `gh66_scrollback_lifecycle_contract` |
| B-011 | source order blocker | lib exact `unknown_blocks_order_and_never_auto_retries` |
| B-012 | explicit native retry policy | integration exact `not_committed_retry_is_explicit_and_unknown_retry_is_rejected` |
| B-013 | durable concurrent exactly-once | integration exact `durable_sink_cross_retry_and_restart_reconstruction_is_exactly_once` |
| B-014 | restart reconstruction | integration exact `durable_sink_cross_retry_and_restart_reconstruction_is_exactly_once`; `public_observation_is_not_a_restore_snapshot` |
| B-015 | composer outcomes | integration exact `composer_focus_cancel_and_failure_outcomes_remain_typed` |
| B-016 | focus/mode/resize | integration exact `composer_focus_cancel_and_failure_outcomes_remain_typed` |
| B-017 | empty/missing content | lib exact `empty_state_and_empty_commit_do_not_invent_data` |
| B-018 | write fault matrix/source | lib exact `partial_write_flush_broken_pipe_outcomes_are_typed` |
| B-019 | reentrancy/concurrency | lib exact `revision_overflow_and_reentrancy_are_atomic`; durable integration exact |
| B-020 | checked counters/illegal transition | lib exact `revision_overflow_and_reentrancy_are_atomic` |
| B-021 | ordered explicit shutdown | lib exact `shutdown_state_machine_is_ordered_and_idempotent` |
| B-022 | Drop/panic honesty | PTY exact `inline_pty_restores_terminal_on_normal_cancel_failure_and_panic` |
| B-023 | four-path PTY restoration | PTY exact `inline_pty_restores_terminal_on_normal_cancel_failure_and_panic` |
| B-024 | control/secret sanitizer | lib exact `sanitizer_rejects_terminal_control_injection` |
| B-025 | crash boundary | integration exact `native_restart_never_claims_unknown_terminal_effect` |
| B-026 | live-only resize/degradation | integration exact `live_resize_never_rewrites_confirmed_scrollback` |
| B-027 | public example | prelude exact `claude_example_uses_public_inline_shell_contract`; `cargo check --example claude_input_box --all-features --locked` |
| B-028 | legacy compatibility | prelude exact `legacy_println_and_message_surfaces_remain_compatible` |
| B-029 | dependency gate | fresh preflight adapter + three `git merge-base --is-ancestor` commands |
| B-030 | current-head quality/coverage | full verification + exact `gh66_current_head_coverage_contract` produce/validate |
| B-031 | atomic closure | read-only closure audit removes each evidence class in negative fixtures and requires current exact head |

All “lib exact” commands use:

```sh
cargo test --workspace --lib --locked \
  components::chat::inline::tests::<NAME> -- --exact
```

All “integration exact” commands use:

```sh
cargo test --test inline_chat_shell --locked <NAME> -- --exact
```

PTY exact command:

```sh
cargo test --test inline_chat_shell_pty --locked \
  inline_pty_restores_terminal_on_normal_cancel_failure_and_panic -- --exact
```

## GH-57 Coverage Contract

`tasks.md`包含唯一 `gh57-critical-paths-v1` JSON，exact `file+name` set不得由环境变量传入。
producer读取该 committed block、current diff和真实 llvm-cov raw JSON，生成
`gh57-child-coverage-v1`：

```json
{
  "schema": "gh57-child-coverage-v1",
  "child_issue": 66,
  "head_sha": "<40-hex>",
  "base_main_sha": "<40-hex>",
  "merge_base_sha": "<40-hex>",
  "generated_at": "<head commit RFC3339 timestamp>",
  "provenance": {
    "repository": "majiayu000/rnk",
    "pr_number": 1,
    "tool": "cargo-llvm-cov",
    "command": "<nonempty exact command>",
    "raw_path": "<absolute path>",
    "raw_sha256": "<64-hex>"
  },
  "changed_executable": {"total": 1, "covered": 1, "percent": 100.0},
  "critical_paths": [{
    "file": "src/components/chat/inline/tests.rs",
    "name": "gh66_scrollback_lifecycle_contract",
    "executable": 1,
    "covered": 1,
    "percent": 100.0
  }]
}
```

`generated_at`固定为`git show -s --format=%cI "$IMPLEMENTATION_HEAD"`，禁止wall clock；
producer以`git diff --unified=0 "$BASE_MAIN_SHA...$IMPLEMENTATION_HEAD"`计算 changed executable，
从raw JSON按file/line/function重算。validator重新读取tasks ledger、diff、raw bytes/hash和
artifact，要求head/base/merge-base相等、changed total>0且>=80%，critical exact set相等、
每项executable>0且100%，无duplicate/extra/unknown entry。

可执行 current-head invocations：

```sh
export IMPLEMENTATION_HEAD="$(git rev-parse HEAD)"
export BASE_MAIN_SHA="$(git rev-parse origin/main)"
export GH66_EVIDENCE_DIR="$(mktemp -d)"
export GH66_RAW_COVERAGE="$GH66_EVIDENCE_DIR/llvm-cov.json"
export GH66_COVERAGE_EVIDENCE="$GH66_EVIDENCE_DIR/coverage.json"
export GH66_PR_NUMBER="<current implementation PR number>"

cargo llvm-cov --workspace --all-targets --all-features --locked \
  --json --output-path "$GH66_RAW_COVERAGE"

GH66_COVERAGE_MODE=produce \
  cargo test --test inline_chat_shell --locked \
  gh66_current_head_coverage_contract -- --exact

GH66_COVERAGE_MODE=validate \
  cargo test --test inline_chat_shell --locked \
  gh66_current_head_coverage_contract -- --exact
```

test在mode缺失/越界、path非absolute、文件缺失/空、PR number无效、head/base不匹配时失败。
producer必须原子写artifact；validator逐byte重建规范JSON并比较，不能只反序列化自报percent。

## Verification Plan

规格 packet：

1. `git diff --check`
2. Markdown link check。
3. product B-ID = tech mapping = tasks `Covers:` union。
4. planned-changes唯一、issue=66、complete=true、paths/spec_refs为唯一repo-relative值。
5. tasks ownership DAG无并行shared writer；每task有compile checkpoint。
6. critical ledger set与tasks实际创建的exact tests相等。
7. pinned external SpecRail `check_workflow.py --spec-dir specs/GH66`。

未来 implementation每个writer checkpoint：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
```

最终 current exact head：

```sh
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
| exact identity造成大复制/O(n²) | candidate/content/receipt/ledger共享`Arc<[u8]>`；单次projection冻结 |
| ANSI/control注入 | only-SGR allowlist parser；其他control fail loud，不回显payload |
| Drop错误不可见 | explicit shutdown是成功证据；Drop仅best effort，PTY观察真实terminal |
| current terminal.rs超800 | 新逻辑放child file，root只声明module并保持<800 |

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
