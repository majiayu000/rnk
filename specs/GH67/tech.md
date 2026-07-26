# Tech Spec：固定底部区域的 FullscreenChatShell

## Linked Issue

GH-67: https://github.com/majiayu000/rnk/issues/67

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":67,"complete":true,"paths":["specs/GH67/product.md","specs/GH67/tech.md","specs/GH67/tasks.md","src/components/chat/fullscreen.rs","src/components/chat/fullscreen/types.rs","src/components/chat/fullscreen/error.rs","src/components/chat/fullscreen/layout.rs","src/components/chat/fullscreen/state.rs","src/components/chat/fullscreen/router.rs","src/components/chat/fullscreen/session.rs","src/components/chat/fullscreen/tests.rs","src/components/chat/mod.rs","src/components/mod.rs","src/renderer/terminal.rs","src/renderer/terminal/fullscreen_backend.rs","src/prelude.rs","examples/rnk_chat.rs","tests/fullscreen_chat_shell_public_api.rs","tests/fullscreen_chat_shell_interactions.rs","tests/fullscreen_chat_shell_pty.rs","tests/golden/fullscreen_chat_shell.txt","tests/golden/fullscreen_chat_shell.ansi.txt"],"spec_refs":["specs/GH67/product.md","specs/GH67/tech.md","specs/GH67/tasks.md","specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md","specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md","specs/GH63/product.md","specs/GH63/tech.md","specs/GH63/tasks.md","specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本 packet 只规划 GH-67。GH-62 拥有 Conversation，GH-63 拥有 typed message/block view，
GH-64 拥有 Composer，GH-65 拥有 variable-height MessageList。GH-67 只拥有 fullscreen
composition、region partition、focus/overlay router、checked frame transaction、terminal
session 与一个 public-only example；它不修改四个上游的生产文件。

## Codebase Context

以下锚点在写作基线 `3f21b049db4e6fe426f8c95270b517d10d92959b` 上通过 Read/grep
核实。GH-62～65 尚未全部实现/合并，因此表中 proposed chat API 只能作为 dependency
contract；implementation 必须从 final merged main 重新审计。当前 base
`3f21b049db4e6fe426f8c95270b517d10d92959b` 不存在
`specs/GH65/{product,tech,tasks}.md`，因此三者不在当前 `spec_refs`。GH-65 仍是未满足的
hard dependency；只有 implementation base 同时证明 GH-65 final merge ancestry、三条 spec
path存在且 public candidate API通过下述 capability gate 后，才允许把真实 refs/API 加入本
packet并重新 review。

| Area | Current files | Current behavior | GH-67 decision |
| --- | --- | --- | --- |
| Fullscreen example | `examples/rnk_chat.rs:10`, `:13`, `:63`, `:138` | 自建 role/message，`.skip(offset).take(12)`，单行 draft 与 footer | 手工迁移为 public shell，删除所有私有 chat mechanics |
| Fixed bottom helper | `src/components/layout/scrollable.rs:246` | 两个 child 的 column flex；不验证 min rows、status、overflow 或 rect | shell 使用自己的 checked region partition，再用 structured Elements 组合 |
| Fixed virtual scroll | `src/components/layout/scrollable.rs:178` | offset/viewport 单位为 item count | 保持兼容但 GH-67 不调用；transcript 只用 GH-65 facade |
| Input hooks | `src/hooks/use_input.rs:260` | clone 全部 handlers 并逐个调用，无 consumed result | shell 注册唯一顶层 route path；子组件只经 pure handler 调用 |
| Paste/mouse hooks | `src/hooks/paste.rs:136`, `src/hooks/use_mouse.rs:147` | 同样广播全部 handlers | shell一次 hit-test/dispatch，禁止同时注册子 handler |
| Runtime events | `src/renderer/runtime.rs:121` | Key/Mouse/Resize；当前 Paste 落 wildcard | GH-64 final runtime paste 是前置；GH-67 只消费其 exactly-once event |
| Focus | `src/hooks/use_focus.rs:27`, `:303`, `:463` | scoped focus 与 traversal可用，但 hook handler同样广播 | shell state保存 closed focus target；hook只把 event送进 shell router |
| Accessibility | `src/core/element.rs:58`, `docs/FOCUS_ACCESSIBILITY_INPUT.md` | Viewport/TextArea/Dialog/Status roles与 fallback text 已存在 | 四个 shell region 使用既有 typed roles，不另造无类型 metadata |
| Overlay primitives | `src/components/feedback/modal.rs:166`, `src/components/layout/box_component.rs:277` | Modal不是真正 stack/router；Box支持 absolute但无公共 z-index | shell以 child order定义 z-order、terminal rect clip和LIFO focus restore |
| Layout/render | `src/layout/engine.rs:126`, `src/renderer/tree_renderer/projection.rs:167` | 有 candidate TextFlow与 staged Output；当前 App pipeline仍含 panic/default路径 | GH-60 final checked frame API 是传递前置；session只消费其成功/typed error |
| Terminal lifecycle | `src/renderer/app.rs:130`, `src/renderer/terminal.rs:167`, `:697`, `src/runtime/panic_handler.rs:25` | App进入 fullscreen；Drop/panic cleanup会吞 restoration errors | session在显式路径返回 aggregated typed cleanup；Drop仅最后保险，不作为成功证据 |
| Architecture | `docs/CHAT_UI_COMPONENT_ARCHITECTURE.md:343`, `:445`, `:738` | 定义 row-based list、fixed bottom、focus/resize与alternate screen方向 | 本 packet把方向变为可执行 public contract |

## 设计方案

### 1. Dependency completion 和 source-drift gate

implementation edit 前，coordinator 对 #62/#63/#64/#65 逐项 fresh 生成
`DependencyCompletionRecord`：

```text
DependencyCompletionRecord {
  issue: 62 | 63 | 64 | 65,
  state: CLOSED,
  closed_at: nonempty,
  final_evidence_source: nonempty,
  implementation_prs: nonempty ordered Vec<{
    number, exact_head_sha, merge_commit_sha, merged_at
  }>,
  final_pr_gate_head_sha,
  task_completion_evidence
}
```

只接受 issue final closure evidence 明确列出的完整 implementation PR/commit set。普通
spec PR、open/parked PR、cap-exhausted review、单个 partial fix 或 coordinator 自选 commit
均不是完成证据。对每项记录执行：

1. fresh fetch `origin/main`，从 exact main SHA 创建 implementation branch；
2. issue 必须 `CLOSED` 且有 `closed_at`，final evidence覆盖 approved tasks；
3. 每个 listed PR 必须 fresh `MERGED`、非 draft/parked，head/merge/time逐值相等；
4. 每个 merge commit 均为 implementation base ancestor，final head与最后 completion PR一致；
5. 对 GH-65 递归验证其 GH-58/GH-60/GH-62 completion set，不能只因 #65 closed推断；
6. `test -f specs/GH65/product.md`、`tech.md`、`tasks.md` 必须在 implementation base全通过，
   且引入三路径的 GH-65 merge commit是该base祖先；当前spec base不存在这些paths，不能把
   rejected/unmerged packet当引用；
7. 重新读取最终 public constructors/accessors/errors和真实 paths，并与本 manifest比较；
8. final GH-64/GH-65 必须提供下面 `PreparedUpstreamMutation` capability，且对应 merge commit
   在base；只提供立即修改 `&mut state` 的 handler/facade不满足。

若 `ChatMessageView`、Composer projection/handler、MessageList state/render closure/observation、
GH-60 checked frame或 terminal session接口有语义漂移，先更新本 packet并重新人工 review。
只改命名且完全等价也必须记录 source-drift diff；不得创建 alias、`Any` adapter、private-field
hack、第二 height cache或复制 GH-65 尚未解决的缺陷。

跨组件原子 publication 的 hard gate 是真实上游 candidate boundary，而不是 GH-67 内部
补偿性 rollback。最终命名可不同，但语义必须逐项等价：

```text
PreparedUpstreamMutation {
  base_revision,
  candidate_revision,
  changed_handles,             // O(changed)，不clone全state/transcript
  read_only_candidate_view
}

try_prepare_*(
  live: &State,
  expected_revision,
  typed_event,
  typed_inputs
) -> Result<PreparedUpstreamMutation, ClosedUpstreamError>

commit_prepared(
  live: &mut State,
  prepared: PreparedUpstreamMutation
) -> ()                       // infallible、无callback、无allocation、无checked failure

abort(prepared) -> ()         // discard-only；live从未被改动
```

MessageList 与 Composer 两者都须满足：prepare不修改live revision/value/cache/observation；
candidate view足够完成measurement、layout和render；commit前的所有错误都可通过丢弃token
结束；commit自身只做已分配candidate/handle swap，不能调用用户callback、分配、measure、
render或再验证。GH-67持有两个prepared token和shell/frame candidate，先完成全部fallible
步骤，再在一个无失败publication section中依次commit两个upstream token与shell/frame。
由于该section没有可观察失败点，不需要伪造跨已commit state的rollback。任何upstream没有
此capability，implementation gate直接blocked并先修上游；禁止clone完整List/Composer作为
undo、调用立即commit API后重建、或只在测试private port伪造candidate。

### 2. 模块和文件所有权

```text
src/components/chat/
├── fullscreen.rs          public FullscreenChatShell facade
└── fullscreen/
    ├── types.rs           validated config/event/observation/overlay values
    ├── error.rs           closed typed config/layout/router/session errors
    ├── layout.rs          checked three-region partition and hit testing
    ├── state.rs           caller-owned revision/focus/overlay/frame metadata
    ├── router.rs          one-event/one-target precedence and upstream adapters
    ├── session.rs         candidate frame + fullscreen terminal lifecycle
    └── tests.rs           module contracts and GH-57 bridge exact test

src/renderer/terminal/
└── fullscreen_backend.rs  public native backend、snapshot、exclusive lease与mode transitions
```

`chat/mod.rs` 增加 module/re-export且继承最终 chat root不可降级的
`#![forbid(missing_docs)]`。`components/mod.rs` 与 `prelude.rs` 只导出 app-facing concrete
types，不创建 type alias。`src/renderer/terminal.rs` 只增加 child module/re-export，native
逻辑全部放入新child；生产文件目标 200–400 行且均 <800。

### 3. Public value types and constructors

所有 public structs字段私有，以 constructor/accessor读取；closed error enums不使用
`#[non_exhaustive]`、catch-all/string variant。未来可扩展 event/payload enums从首次发布
可标记 `#[non_exhaustive]`。核心类型：

```text
FullscreenTerminalSize { columns: u16, rows: u16 }
FullscreenChatShellConfig {
  min_columns: NonZeroU16,
  min_transcript_rows: NonZeroU16,
  min_composer_rows: NonZeroU16,
  max_composer_rows: NonZeroU16
}
FullscreenShellRevision(u64)                 // initial 0, checked_next only
FullscreenOverlayId(trimmed nonempty String)
FullscreenFocusTarget =
  Transcript | Composer | Overlay(FullscreenOverlayId)
OverlayKind =
  Modal | Pointer | Passive
FullscreenOverlayRequest {
  id, kind, dismissible, focusable, rect: FullscreenRect, body: Element,
  handler_capability: None | KeyPasteMouse
}
FullscreenOverlayInput = Key | Paste | Mouse { kind, point, hit }
FullscreenOverlayAction = KeepOpen | CloseTop | RequestFocus(Transcript | Composer | OverlayId)
FullscreenStatusRegion { rows: NonZeroU16, body: Element, accessible_label }
FullscreenRect { column, row, width, height } // checked constructor/end accessors
FullscreenRegionLayout { terminal, transcript, composer, status: Option<Rect> }
FullscreenShellObservation {
  revision, focus, layout, follow_state, stored_anchor, new_content_below,
  composer_clamped, top_overlay, session_state
}
```

`FullscreenTerminalSize::new(columns, rows)` 显式允许零，以便零尺寸进入可达
`UnsupportedTerminalSize`，而不是在构造前消失。config `try_new` 拒绝 min=0、max=0、
min composer > max composer。`FullscreenStatusRegion::try_new` 拒绝 zero rows或空白
accessible label；`None` 精确表达 absent。overlay rect的 zero-area由 typed error拒绝。
Modal/Pointer必须声明handler capability；Passive必须`focusable=false`且handler=None；
其他kind/focus/handler组合在constructor/open前返回`InvalidOverlayState`。

state constructor签名在最终 dependency命名下必须语义等价于：

```text
FullscreenChatShellState::try_new(
  config: FullscreenChatShellConfig,
  terminal: FullscreenTerminalSize,
  initial_entries: Vec<MessageListEntry>,
  initial_measurement_config: MessageMeasurementConfig,
  composer_state: ChatComposerState,
  composer_projection: ComposerProjection,
  status: Option<FullscreenStatusRegion>,
  initial_focus: FullscreenFocusTarget,
  initial_overlays: Vec<FullscreenOverlayRequest>,
  measure: &mut impl FnMut(MessageMeasureRequest) -> MessageMeasureOutcome
) -> Result<Self, FullscreenShellError>
```

这条签名必须实际创建可用的 MessageList state与完整 active measurement handles；空 entries
合法。callback返回 zero row或缺 active key时必须是可达 typed error。constructor完成前
不发布 partial state，也不把 callback放进 state长期持有。MessageList key handle由最终
GH-65 concrete `Arc`-backed value拥有；shell只保存/clone lightweight handle，active frame
完成前拥有强引用，不能只留在可 eviction cache。

`FullscreenChatShell::try_into_element(...)` 借用 state/conversation/composer projection，
并接受唯一 stable typed MessageList render closure。status与有序 overlay bodies只从
state的validated values读取，不再作为第二份调用参数。closure精确接收 GH-65
entry/key-handle/visible-slice并在内部使用 GH-63 `ChatMessageView`；shell不接收第二个
row-height callback，不重测 block。

### 4. Checked region partition

令 `T=terminal.rows`、`S=status.rows or 0`、`P=exact-current ComposerProjection.height`：

```text
required = checked(min_transcript + min_composer + S)
if columns < min_columns or T < required -> UnsupportedTerminalSize
composer_cap = min(max_composer, checked(T - min_transcript - S))
composer_rows = clamp(P, min_composer, composer_cap)
transcript_rows = checked(T - composer_rows - S)

transcript = rect(0, 0, columns, transcript_rows)
composer   = rect(0, transcript_rows, columns, composer_rows)
status     = S == 0 ? None
           : rect(0, transcript_rows + composer_rows, columns, S)
```

`P=0` 是 `InvalidComposerProjection`，不能被 clamp成1；projection revision必须等于 current
Composer state revision。所有 u16/usize转换、rect end和sum使用 checked arithmetic。
成功必须证明 transcript rows≥min、composer在min..=cap、status end==T、三个rect两两不重叠。
overlay不消耗base rows；它clip到 terminal rect并在base children后按stack bottom→top追加。

### 5. MessageList integration

shell只通过最终 GH-65 public facade执行：

- constructor接收完整 ordered entries/config/measurement；
- resize在candidate中建立完整新 measurement config并测量所需 keys；
- conversation `ApplyOutcome` 只从 public affected-message accessor映射到 typed list mutation；
- append/prepend/update/delete/expand/collapse用 expected list revision；
- visible slices和public observation直接读取 list；
- render closure只收到 active O(1) handle与partial row slice。

shell不解析 ChatMessage payload来猜 height、不调用 `TextFlow`、不以 item count换算 rows。
explicit transcript navigation直接调用 GH-65 typed navigation，必须优先覆盖 Following并进入
Paused；不得在调用前用 current visible top覆盖 requested anchor。Following、Paused/stored
anchor、new-content与clamp顺序继承最终 GH-65合同。GH-67成功partition始终传入
`viewport_rows >= min_transcript_rows > 0`；zero/undersized terminal在任何List prepare前返回
`UnsupportedTerminalSize`。GH-65 zero-row logical-bottom只保留为dependency component
evidence，不出现在GH-67 shell test或成功state中。

### 6. State transaction and event ordering

`handle_fullscreen_shell_event` 语义签名：

```text
handle_fullscreen_shell_event(
  states: FullscreenShellStates<'_>, // &mut shell、&mut MessageList、&mut Composer
  expected_revision: FullscreenShellRevision,
  event: FullscreenShellEvent,
  dependencies: FullscreenShellInputs,
) -> Result<InteractionOutcome<FullscreenShellPayload>, FullscreenShellError>
```

`FullscreenShellInputs` 是具体 private-field struct，含当前 immutable Conversation snapshot、
MessageList config、measurement/render closure和唯一typed overlay handler；没有
`Any`/dynamic map。
event至少含 Resize、ConversationApplied、Key、Paste、Mouse、SetStatus、OpenOverlay、
CloseTopOverlay、Suspend、Resume、Shutdown。SetStatus和OpenOverlay的typed payload分别是
status/overlay state的唯一更新来源。处理优先级：

```text
expected shell revision
-> event kind/system precedence
-> target/id/overlay/focus validation
-> checked next shell revision (only if observable mutation is possible)
-> upstream composer/list expected revisions
-> GH-64/GH-65 try_prepare_* without live mutation
-> ordered measurement/layout using prepared read-only views
-> render closure + GH-60 checked frame candidate
-> infallible publication: commit list token, composer token, shell/observation/frame
```

stale shell revision先于callback；unknown overlay/focus先于upstream mutation；arithmetic/layout
先于frame commit。任何prepare/measurement/layout/render error只discard两个tokens和shell
candidate，live List/Composer/shell/frame逐值相等。publication section不能再执行callback、
allocation、conversion、validation或返回`Result`；若最终上游commit仍fallible，本issue
implementation gate失败，禁止声称rollback。Ignored、Handled-no-change、Cancelled不推进
revision；一次成功 event即使改变list/layout/focus多个字段也只推进一次。相同序列确定。rapid
`Resize -> ConversationApplied(stream) -> ConversationApplied(prepend) -> Key` 精确按该顺序；
没有background reorder、coalescing或全局fallback。

### 7. Focus, key, paste and mouse routing

stack validation要求一旦存在 Modal，它必须是top；Modal之上不能再打开Pointer/Passive，
只允许嵌套Modal。这样“top Modal”是唯一modal barrier，避免中层modal与上层pointer的未定义
穿透。无Modal时可混合Pointer/Passive。keyboard/paste总表：

| Overlay/focus state | Event | Sole handler | Overall outcome / propagation |
| --- | --- | --- | --- |
| 任意 | Resize | shell prepare transaction | 不进入input handler |
| 任意 | Suspend/Resume/Shutdown | session | 不进入shell/component handler |
| top Modal、dismissible | Escape | shell CloseTop | close一层并恢复saved focus；不调用overlay/base |
| top Modal、不可dismiss | Escape | top Modal handler | `Ignored`提升为`Handled`；consumed |
| top Modal | 其他key或Paste | top Modal handler | 任意handler outcome均停止；`Ignored`提升为`Handled` |
| 无Modal、top dismissible Pointer/Passive | Escape | shell CloseTop | close一层；不向focused target继续 |
| 无Modal | Tab/BackTab | shell traversal | 只遍历Transcript、Composer、focusable Pointer；Passive跳过 |
| 无Modal、Transcript focus | navigation key | GH-65 prepared navigation | exact once；返回其typed outcome |
| 无Modal、Transcript focus | 其他key/committed/Paste | none | `Ignored` |
| 无Modal、Composer focus | key/committed | GH-64 prepared key ingress | exact once；不转paste/submit第二次 |
| 无Modal、Composer focus | Paste | GH-64 prepared paste ingress | exact once；不转key |
| 无Modal、Pointer focus | key或Paste | focused Pointer handler | exact once；`Ignored`也不向base fallthrough |
| 无Modal、Passive focus | 任意input | none | constructor/router `InvalidFocusTarget`，零handler、零mutation |
| 无Modal、unknown/closed overlay focus | 任意input | none | `UnknownOverlayFocus`，零handler、零mutation |

mouse kind闭集为Press、Release、Drag、Move、Wheel；先用committed layout和point执行：

| Scan/result | Sole handler | Overall outcome / propagation |
| --- | --- | --- |
| top Modal（point在内或外） | top Modal handler，携带typed `hit` | stop；`Ignored`提升为`Handled` |
| 无Modal，当前overlay是Passive | none | 继续扫描lower layer |
| 无Modal，当前Pointer miss | none | 继续扫描lower layer |
| 无Modal，topmost Pointer hit | 该Pointer handler | stop；保留其outcome，`Ignored`也不再fallthrough |
| overlay扫描结束，point在terminal外 | none | `Ignored` |
| status任意mouse kind | shell passive status | `Handled` no-change；不focus、不继续 |
| Composer Press | shell focus transition | focus改变为`Changed`，已focus为`Handled`；不调用第二handler |
| Composer Release/Drag/Move/Wheel | none | `Ignored` |
| Transcript Wheel | GH-65 prepared navigation | exact once；不再改变focus |
| Transcript Press | shell focus transition | focus改变为`Changed`，已focus为`Handled` |
| Transcript Release/Drag/Move | none | `Ignored` |

overlay handler签名固定为一次
`FnMut(OverlayId, FullscreenOverlayInput) ->
InteractionOutcome<FullscreenOverlayAction>`；action只能KeepOpen、CloseTop或请求合法focus。
返回非法/Passive/unknown focus时整个prepared transaction失败且live state不变。打开overlay
前保存current focus；ID唯一。关闭只允许top ID，按LIFO恢复仍存在且可用的saved focus，否则
`InvalidFocusRestore`。IME只支持runtime交付的committed text；preedit/candidate unsupported。
resize candidate未成功时旧committed layout仍是唯一mouse坐标真相。

### 8. Checked frame and terminal session

`FullscreenFrameTransaction` 私有地持有两个upstream prepared tokens、lightweight shell
candidate、Element tree、GH-60 checked layout/render result与previous observation；只有
所有fallible工作完成后才能进入infallible `commit()`。任一callback panic在publication前
unwind，由session guard恢复terminal，不能catch后default render。

公共terminal surface不是抽象生命周期，而是以下可crate外实现/调用的合同：

```text
FullscreenTerminalCapability = SupportedRestorable | Unsupported
FullscreenTerminalCapabilities { raw, cursor, alternate, mouse, focus, paste }
FullscreenTerminalSnapshot {
  screen: Normal | Alternate,
  raw_enabled,
  cursor_visible,
  mouse_capture,
  focus_reporting,
  bracketed_paste
}
OptionalCapabilityPolicy = Require | Disable
FullscreenSessionConfig {
  mouse, focus, paste: OptionalCapabilityPolicy,
  poll_timeout: Duration
}

trait FullscreenTerminalBackend {
  type Error: std::error::Error + Send + Sync + 'static;
  type Lease: FullscreenTerminalLease;
  fn try_acquire_lease(&mut self) -> Result<Self::Lease, Self::Error>;
  fn try_release_lease(&mut self, lease: &mut Self::Lease) -> Result<(), Self::Error>;
  fn capabilities(&self, lease: &Self::Lease) -> FullscreenTerminalCapabilities;
  fn try_snapshot(&mut self, lease: &mut Self::Lease)
    -> Result<FullscreenTerminalSnapshot, Self::Error>;
  fn try_apply(&mut self, lease: &mut Self::Lease, transition: FullscreenTerminalTransition)
    -> Result<(), Self::Error>;
  fn try_render(&mut self, lease: &mut Self::Lease, frame: &CheckedFrame)
    -> Result<(), Self::Error>;
  fn try_poll(&mut self, lease: &mut Self::Lease, timeout: Duration)
    -> Result<Option<FullscreenShellEvent>, Self::Error>;
  fn try_flush(&mut self, lease: &mut Self::Lease) -> Result<(), Self::Error>;
}

NativeFullscreenTerminalBackend::try_stdout()
  -> Result<Self, FullscreenSessionStartError>
FullscreenSession<B>::try_enter(
  backend: B,
  config: FullscreenSessionConfig
) -> Result<Self, FullscreenSessionStartError>
run(
  &mut self,
  shell: &mut FullscreenChatShellState,
  list: &mut MessageListState,
  composer: &mut ChatComposerState,
  inputs: &mut FullscreenShellInputs<'_>,
  panic_reporter: &mut impl FullscreenPanicCleanupReporter
)
  -> Result<FullscreenSessionExit, FullscreenRunError>
render_frame(&mut self, &CheckedFrame) -> Result<(), FullscreenRunPrimaryError>
try_suspend(&mut self) -> Result<FullscreenSuspendReport, FullscreenRunError>
try_resume(&mut self, &CheckedFrame) -> Result<FullscreenResumeReport, FullscreenRunError>
try_shutdown(&mut self) -> Result<FullscreenShutdownReport, FullscreenRunError>
```

`FullscreenTerminalLease`只公开active/owner accessor，不可clone/construct；具体token由
backend associated type拥有。`FullscreenTerminalTransition`是闭集
`SetScreen(Normal|Alternate)`、`SetRaw(bool)`、`SetCursorVisible(bool)`、
`SetMouseCapture(bool)`、`SetFocusReporting(bool)`、`SetBracketedPaste(bool)`；lease acquire/
release只能走专用方法，不能伪装成mode transition。

Native backend在`src/renderer/terminal/fullscreen_backend.rs`拥有process-wide lease registry、
mode registry与实际terminal I/O；`try_stdout()`只构造backend且零lease/terminal mutation，
真正lease唯一由`FullscreenSession::try_enter`调用`try_acquire_lease`取得并存入session。
第二个/nested session返回`AlreadyActive`。backend必须能证明完整pre-entry snapshot；任一mode
无法查询或不在library-owned registry时返回
`UnknownPreEntryState`且零mutation。raw/cursor/alternate是required；mouse/focus/paste按
Require/Disable预检，Unsupported+Require在entry前失败，Disable保持snapshot原值且不宣称
启用。fake backend用同一trait注入每一步失败。

entry顺序为acquire lease token→snapshot→capability preflight→alternate→raw→cursor-hide→optional
mouse/focus/paste→flush。每一步只在目标值与snapshot不同时执行并记录completed-step；
任一失败按相反顺序恢复所有completed steps并flush，聚合primary enter error和全部rollback
failures；最后显式release token，release失败也进入rollback failures。没有完整Active session
就不能render/poll。session lifecycle闭集为Entering{lease}、Active{lease,snapshot}、
Suspending{lease,snapshot}、Suspended{no_lease}、Resuming{lease,new_snapshot}、
Closing{lease,snapshot}、Shutdown。shutdown停止poll，
按snapshot逐项恢复并尝试全部步骤；只把成功step标完成，第二次调用重试unfinished steps，
全部成功才释放lease并进入Shutdown。

suspend停止poll并执行同一完整snapshot restoration；restoration与release均成功才进入
`Suspended{no_lease}`。此时第二session可以取得lease。resume先acquire新token；若被第二session
占用则返回`ResumeLeaseUnavailable`、保持Suspended且零terminal mutation；取得后读取新snapshot、
重跑staged enter并full repaint，失败则反向恢复、release新token并保持Suspended。fresh session
必须重新constructor shell state。`run`的完整参数是session event loop的唯一state/input来源，
不从global或省略参数重建。`run`用unwind guard保证panic时
执行全部cleanup，把`FullscreenPanicCleanupReport`交给调用方注入的
`FullscreenPanicCleanupReporter`后`resume_unwind`；public report不暴露panic `Any` payload。
Drop只重试unfinished best-effort steps并释放可安全释放的lease，不构成恢复成功证据。

shell与run error family分离，避免terminal cleanup覆盖primary：

```text
FullscreenShellError =
  Config(FullscreenConfigError) |
  State(FullscreenStateError) |
  Layout(FullscreenLayoutError) |
  MessageList(final GH65 closed errors) |
  Composer(ChatComposerError) |
  CheckedRender(final GH60 error)

FullscreenRunPrimaryError =
  Shell(FullscreenShellError) |
  Backend(FullscreenTerminalOperationError)

NonEmptyRestorationFailures(Vec<FullscreenRestorationStepFailure>)

FullscreenRunError =
  Start(FullscreenSessionStartError) |
  Primary(FullscreenRunPrimaryError) |
  Cleanup(NonEmptyRestorationFailures) |
  PrimaryAndCleanup {
    primary: FullscreenRunPrimaryError,
    cleanup: NonEmptyRestorationFailures
  }
```

restoration step闭集为RawMode、Cursor、AlternateScreen、MouseCapture、FocusReporting、
BracketedPaste、Flush、LeaseRelease；每项保存typed backend source与attempted target。
`PrimaryAndCleanup`的`source()`返回primary source，`primary()`与`cleanup_failures()`分别允许
无损检查原layout/render类别和全部cleanup steps；`Display`只输出类别与failure count，不
回显terminal payload。fake test必须注入primary render failure加三个cleanup failure，断言
primary source仍可downcast、三项step/order完整且无覆盖/吞掉。

### 9. Accessibility, public example and golden

base children分别附加既有 `AccessibilityRole::Viewport`、`TextArea`、`Status`，modal overlay
使用 Dialog。label/value/description来自typed observation；paused/new-content、focus、
submitting/failed/cancelled包含文字，不只颜色。plain/ANSI golden使用同一 deterministic
conversation fixture；ANSI strip后semantic token、顺序、状态和errors相等，测试unset并拒绝
任何 golden-update env，前后SHA-256不变。

`examples/rnk_chat.rs` 使用 public prelude constructors、offline deterministic updates和
shell session；不访问私有module，不直接 crossterm/ANSI，不定义 message/list/composer/focus/
resize/cleanup mechanics。semantic exact test调用与 `main` 相同 production composition path，
而不是源码字符串伪证据。

### 10. Reproducible SpecRail packet verification

mandatory checker来自可获取的 `https://github.com/majiayu000/specrail.git`，固定commit
`bfc60f26164af5df1ebd3b5cb79d07379fc416b7`。前置只需PATH中的Git、Python 3.9+与tar；
checkers仅使用Python标准库，不执行`pip install`。任何reviewer从fresh checkout运行：

```sh
command -v git
command -v python3
command -v tar
python3 -c 'import sys; assert sys.version_info >= (3, 9)'

SPEC_RAIL_URL="https://github.com/majiayu000/specrail.git"
SPEC_RAIL_COMMIT="bfc60f26164af5df1ebd3b5cb79d07379fc416b7"
SPEC_RAIL_CHECKOUT="$(mktemp -d)"
GH67_SPEC_MIRROR="$(mktemp -d)"
git -C "$SPEC_RAIL_CHECKOUT" init -q
git -C "$SPEC_RAIL_CHECKOUT" remote add origin "$SPEC_RAIL_URL"
git -C "$SPEC_RAIL_CHECKOUT" fetch --depth=1 origin "$SPEC_RAIL_COMMIT"
git -C "$SPEC_RAIL_CHECKOUT" checkout --detach FETCH_HEAD
test "$(git -C "$SPEC_RAIL_CHECKOUT" rev-parse HEAD)" = "$SPEC_RAIL_COMMIT"

test "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
  "$SPEC_RAIL_CHECKOUT/checks/check_workflow.py")" = \
  "c5bd73060037b0e8febace0e5ee8473e17973e1ca17257ea1517a94e05fa7549"
test "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
  "$SPEC_RAIL_CHECKOUT/tools/spec_depth_audit.py")" = \
  "380169fcbad509e6bc1b6a555ae0fa469744662af7120e20e999206c226e66c3"

git -C "$SPEC_RAIL_CHECKOUT" archive "$SPEC_RAIL_COMMIT" | tar -x -C "$GH67_SPEC_MIRROR"
mkdir -p "$GH67_SPEC_MIRROR/specs/GH67"
cp specs/GH67/product.md specs/GH67/tech.md specs/GH67/tasks.md \
  "$GH67_SPEC_MIRROR/specs/GH67/"
python3 "$SPEC_RAIL_CHECKOUT/checks/check_workflow.py" \
  --repo "$GH67_SPEC_MIRROR" --spec-dir specs/GH67
python3 "$SPEC_RAIL_CHECKOUT/tools/spec_depth_audit.py" \
  --repo "$GH67_SPEC_MIRROR" --spec-dir specs/GH67 --gate
```

URL、commit与两个checksum均是本packet常量，不能由environment覆盖。checkout/fetch/checksum
任一步失败即verification失败；不得fallback到缓存、vendored copy、machine-local固定路径或
另一revision。`specrail_checker_checkout_is_reproducible`以隔离temporary directory执行同一
流程并断言remote commit、checksum、workflow与depth四项全部通过。

## Product-to-Test Mapping

所有 filtered tests先 `--list --exact` 要求 matched=1，再
`--include-ignored --exact` 要求 `1 passed; 0 failed; 0 ignored`。

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | facade/state/types/exports | `fullscreen_shell_public_surface_is_typed_and_controlled` |
| B-002 | state constructor | `constructor_requires_complete_entries_config_projection_and_measurement` |
| B-003 | layout partition | `fixed_bottom_partition_uses_exact_remaining_rows`；`gh67_fixed_bottom_resize_contract` |
| B-004 | config/zero/undersized/error precedence | `zero_and_undersized_terminals_fail_before_callbacks` |
| B-005 | optional status | `status_absence_uses_zero_rows_and_invents_no_data` |
| B-006 | Composer projection/clamp | `composer_projection_clamps_without_overlap_and_keeps_draft` |
| B-007 | MessageList facade only | `variable_height_transcript_uses_rows_not_item_count` |
| B-008 | cache identity/active handles | `measurement_invalidation_and_active_handles_follow_exact_identity` |
| B-009 | Following/nonzero shell viewport | `following_stream_growth_tracks_latest_bottom_in_supported_viewport`；`zero_and_undersized_terminals_fail_before_callbacks` |
| B-010 | Paused/new output | `paused_stream_growth_preserves_anchor_and_reports_new_content` |
| B-011 | prepend | `prepend_preserves_stable_message_and_intra_row_anchor` |
| B-012 | continuous resize | `continuous_resize_reflows_list_and_composer_in_one_frame`；`gh67_fixed_bottom_resize_contract` |
| B-013 | GH-63 typed views | `typed_multiline_block_views_render_once_in_source_order` |
| B-014 | public example | `rnk_chat_example_uses_only_public_fullscreen_composition`；`cargo check --example rnk_chat --all-features --locked` |
| B-015 | focus observation/revision | `public_observation_reports_focus_regions_follow_and_overlay`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-016 | total key/paste precedence | `focus_overlay_key_routing_is_single_target_and_deterministic`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-017 | overlay state/stack/z-order | `nested_overlay_z_order_and_invalid_updates_are_atomic`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-018 | Escape/focus/fallthrough | `nested_overlay_escape_restores_focus_lifo_without_fallthrough`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-019 | committed input/paste | `paste_and_committed_ime_text_dispatch_exactly_once` |
| B-020 | total mouse hit/fallthrough | `mouse_hit_testing_uses_committed_z_order_without_double_dispatch`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-021 | rapid event/revision | `rapid_resize_stream_prepend_sequence_is_deterministic` |
| B-022 | prepared upstream/overflow atomicity | `upstream_prepare_commit_abort_gate_and_late_failure_are_atomic`；`coordinate_revision_and_upstream_failures_are_atomic` |
| B-023 | checked frame/dual failure | `layout_render_failure_preserves_committed_state_and_frame`；`primary_failure_and_all_cleanup_failures_are_preserved` |
| B-024 | public session/terminal paths | `fullscreen_session_public_surface_and_capability_gate_are_typed`；`partial_enter_and_suspend_resume_restore_exact_snapshot`；`fullscreen_terminal_restores_all_modes_on_every_exit_path`；`primary_failure_and_all_cleanup_failures_are_preserved` |
| B-025 | suspend/resume/restart | `partial_enter_and_suspend_resume_restore_exact_snapshot`；`suspend_resume_and_fresh_restart_rebuild_explicit_state` |
| B-026 | accessibility/golden | `accessibility_and_plain_ansi_semantics_do_not_depend_on_color` |
| B-027 | bounded work/handles | `visible_frame_work_is_bounded_and_handles_are_o1_non_evictable` |
| B-028 | security audit | `fullscreen_shell_has_no_provider_tool_or_secret_execution_surface` |
| B-029 | dependency/capability/path gate | `dependency_completion_requires_closed_final_merged_ancestor_sets`；`upstream_prepare_commit_abort_gate_and_late_failure_are_atomic` |
| B-030 | exact/reproducible evidence | mapping全部 tests；`gh67_current_head_coverage_contract`；`specrail_checker_checkout_is_reproducible`；full gates/CI/review |

## Data Flow

### 输入

- final GH-62 immutable Conversation snapshot/`ApplyOutcome`。
- final GH-63 borrowed typed message render path。
- final GH-64 caller-owned Composer state与 prepared mutation/view/infallible commit capability。
- final GH-65 caller-owned MessageList、prepared mutation/view、measurement config/closure与
  observation。
- terminal size、optional status、typed overlay requests和 serialized shell events。
- public terminal backend capabilities、exact pre-entry snapshot与session config。

### 处理

1. expected shell revision与event target preflight。
2. 分别prepare Composer/List tokens，live states保持未修改。
3. 用prepared views执行完整measurement/checked partition，只从GH-65 visible slices经GH-63
   closure生成transcript并组合Composer/status/overlay。
4. GH-60 checked layout和staged render成功后执行无失败commit section；否则discard tokens。
5. session写terminal；退出时逐项恢复snapshot，同时保留primary和全部cleanup failures。

### 输出

`InteractionOutcome<FullscreenShellPayload>`、immutable
`FullscreenShellObservation`、一个structured frame或具体 `FullscreenShellError`；session
边界返回`FullscreenRunError`，双失败时同时包含primary和nonempty cleanup集合。

### 持久化与外部调用

无 provider/network/tool/secret/storage。session仅调用terminal/runtime lifecycle。state、
anchor、draft与overlay由进程内caller持有；fresh restart不声明跨进程恢复。

## 备选方案

- 扩展 `fixed_bottom_layout` 加聊天状态：拒绝；通用布局helper不应拥有MessageList/focus/session。
- 用 item-count `virtual_scroll_view`：拒绝；可变高度消息会错误定位。
- transcript/composer/overlay各注册hook：拒绝；当前广播机制会double dispatch。
- shell复制GH-65 height index或调用TextFlow测消息：拒绝；造成两套identity/invalidation。
- 在GH-64/GH-65立即commit后clone/rebuild rollback：拒绝；不是O(changed)，且late failure
  无法无损还原revision/cache/handles。
- `Option<Element>`表达render错误：拒绝；None含义不明确且会静默丢消息。
- layout失败显示旧frame并返回成功：拒绝；观察状态与屏幕会漂移。
- Drop-only cleanup：拒绝；无法把restoration failure报告给调用方。
- 将Inline/Fullscreen合并为mode flag：拒绝；native scrollback与owned frame生命周期不同。

## 风险

- Dependency drift：四项direct dependencies均未最终完成，当前base也没有GH-65 spec paths。
  以path existence、closed/final merged ancestry、candidate capability与source-drift
  reapproval阻断推测实现。
- Correctness：region arithmetic、list/composer candidate与frame可能不同步。以单transaction、
  checked rect和failure equality tests缓解。
- Interaction：broadcast hooks会double dispatch。shell只注册一个top-level adapter，内部用
  closed route table。
- Terminal：真实terminal entry/cleanup有多步独立失败。exclusive lease、exact snapshot、
  reverse partial-entry rollback、retryable cleanup和PTY/fake backend共同验证。
- Performance：全conversation clone会随历史增长。只保留GH-65 O(1) handles/visible slices，
  exact operation-count test禁止线性key/resize路径。
- Security：paste/overlay/tool text可能含controls。committed input走GH-64，render走GH-58/
  Output边界，shell无raw ANSI/tool execution。

## 测试计划

- [ ] config/partition property覆盖0、minimum、max、`u16::MAX`与status absent/present。
- [ ] MessageList shell sequences覆盖Following/Paused、append/stream/prepend/expand/collapse/
      delete与continuous supported resize；zero/undersized在List callback前失败，zero viewport
      只在GH-65 component suite验证。
- [ ] Modal/Pointer/Passive × Transcript/Composer/Overlay focus × key/paste及
      Press/Release/Drag/Move/Wheel逐格断言唯一handler、outcome、consumed与fallthrough。
- [ ] Text/Markdown/Code/Thinking/ToolResult单/多行和plain/ANSI/accessibility golden。
- [ ] failure injection覆盖Composer/List prepare、measure、layout、projection、render、
      coordinate/revision overflow与prepare后late failure，证明三个live states/frame相等。
- [ ] Linux/macOS PTY和portable fake backend覆盖public constructor、capability preflight、
      nested lease、partial enter、normal/cancel/error/panic、suspend/resume与exact snapshot
      restoration；primary+三个cleanup failures同时可检查。unsupported平台保留fake contract，
      不伪称真实terminal verified。
- [ ] 隔离目录从固定SpecRail URL/commit checkout，校验两个checker SHA-256并通过workflow/
      depth；不存在machine-local absolute fallback。
- [ ] exact mapping、coverage producer/validator、fmt/check/clippy/all-target tests/example、
      fresh CI、独立review、reviewThreads和SpecRail PR gate。

## 回滚方案

GH-67为新增模块/exports/example迁移，无数据migration。未merge时关闭implementation PR；
已merge时普通revert全部planned paths，并先回滚依赖GH-67的GH-68 work。不得force push，
不得保留导出但silent-disable router/session，也不得恢复example私有item scroll/cleanup作为
“兼容fallback”。失败evidence与dependency记录保留，issue保持open。
