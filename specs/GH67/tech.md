# Tech Spec：固定底部区域的 FullscreenChatShell

## Linked Issue

GH-67: https://github.com/majiayu000/rnk/issues/67

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":67,"complete":true,"paths":["specs/GH67/product.md","specs/GH67/tech.md","specs/GH67/tasks.md","src/components/chat/fullscreen.rs","src/components/chat/fullscreen/types.rs","src/components/chat/fullscreen/error.rs","src/components/chat/fullscreen/layout.rs","src/components/chat/fullscreen/state.rs","src/components/chat/fullscreen/router.rs","src/components/chat/fullscreen/session.rs","src/components/chat/fullscreen/tests.rs","src/components/chat/mod.rs","src/components/mod.rs","src/renderer/terminal.rs","src/renderer/terminal/fullscreen_backend.rs","src/renderer/terminal_controller.rs","src/runtime/panic_handler.rs","src/prelude.rs","examples/rnk_chat.rs","tests/fullscreen_chat_shell_public_api.rs","tests/fullscreen_chat_shell_interactions.rs","tests/fullscreen_chat_shell_pty.rs","tests/golden/fullscreen_chat_shell.txt","tests/golden/fullscreen_chat_shell.ansi.txt"],"spec_refs":["specs/GH67/product.md","specs/GH67/tech.md","specs/GH67/tasks.md","specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md","specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md","specs/GH63/product.md","specs/GH63/tech.md","specs/GH63/tasks.md","specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本 packet 只规划 GH-67。GH-62 拥有 Conversation，GH-63 拥有 typed message/block view，
GH-64 拥有 Composer，GH-65 拥有 variable-height MessageList。GH-67 只拥有 fullscreen
composition、region partition、focus/overlay router、checked frame transaction、terminal
session 与一个 public-only example；它不修改四个上游的生产文件。

## Codebase Context

以下锚点在写作基线 `3f21b049db4e6fe426f8c95270b517d10d92959b` 上核实；proposed API仅作dependency contract。该base无GH-65三spec paths；final ancestry/path/capability通过后才加入并重审。

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

implementation edit前，coordinator对#62/#63/#64/#65逐项fresh生成：

```text
DependencyCompletionRecord {
  issue: 62 | 63 | 64 | 65, state: CLOSED, closed_at: nonempty,
  final_evidence_source: nonempty,
  implementation_prs: nonempty ordered Vec<{ number, exact_head_sha, merge_commit_sha, merged_at }>,
  final_pr_gate_head_sha, task_completion_evidence
}
```

只接受issue final closure evidence明确列出的完整implementation PR/commit set；spec PR、
open/parked、cap-exhausted、partial fix或自选commit均不是完成证据。每项执行：

1. fresh fetch `origin/main`，从 exact main SHA 创建 implementation branch；
2. issue 必须 `CLOSED` 且有 `closed_at`，final evidence覆盖 approved tasks；
3. 每个 listed PR 必须 fresh `MERGED`、非 draft/parked，head/merge/time逐值相等；
4. 每个 merge commit 均为 implementation base ancestor，final head与最后 completion PR一致；
5. 对 GH-65 递归验证其 GH-58/GH-60/GH-62 completion set，不能只因 #65 closed推断；
6. 三个`test -f specs/GH65/{product,tech,tasks}.md`在base通过且引入commit为ancestor；
7. 重新读取最终 public constructors/accessors/errors和真实 paths，并与本 manifest比较；
8. final GH-64/GH-65提供下述capability且merge commit在base；立即修改`&mut state`不满足。

若ChatMessageView、Composer、MessageList、GH-60 frame或terminal session语义漂移，先更新并
人工重审；纯重命名也记录diff。禁止alias、`Any`、private-field hack、第二cache或复制上游缺陷。

跨组件原子publication的hard gate是真实上游candidate boundary，而非GH-67补偿rollback：

```text
PreparedUpstreamMutation {
  base_revision, candidate_revision,
  changed_handles, // O(changed)，不clone全state/transcript
  read_only_candidate_view
}
try_prepare_*(live: &State, expected_revision, typed_event, typed_inputs)
  -> Result<PreparedUpstreamMutation, ClosedUpstreamError>
commit_prepared(live: &mut State, prepared: PreparedUpstreamMutation) -> ()
  // infallible、无callback/allocation/checked failure
abort(prepared) -> () // discard-only；live从未改动
```

两上游prepare均不改live，candidate view足够measure/layout/render；commit只swap已分配
candidate/handle。GH-67完成全部fallible步骤后才在无失败section提交两token与shell/frame。
任一上游缺此capability即blocked；禁止clone全state作undo、立即commit后重建或private test port。

### 2. 模块和文件所有权

```text
src/components/chat/
├── fullscreen.rs          public FullscreenChatShell facade
└── fullscreen/
    ├── types.rs           validated config/event/observation/overlay values
    ├── error.rs           closed typed config/layout/router/session errors
    ├── layout.rs          checked three-region partition and hit testing
    ├── state.rs           owning bundle + revision/focus/overlay/frame metadata
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
FullscreenBaseFocusTarget = Transcript | Composer
OverlayKind =
  Modal | Pointer | Passive
FullscreenOverlayRequest {
  id, kind, dismissible, focusable, rect: FullscreenRect, body: Element,
  handler_capability: None | KeyPasteMouse
}
FullscreenInitialOverlayOpen { request: FullscreenOverlayRequest, focus_on_open: bool }
FullscreenOverlayInput = Key | Paste | Mouse { kind, point, hit }
FullscreenOverlayAction = KeepOpen | CloseTop | RequestFocus(Transcript | Composer | OverlayId)
FullscreenStatusRegion { rows: NonZeroU16, body: Element, accessible_label }
FullscreenRect { column, row, width, height } // checked constructor/end accessors
FullscreenRegionLayout { terminal, transcript, composer, status: Option<Rect> }
FullscreenShellObservation {
  revision, focus, layout, follow_state, stored_anchor, new_content_below,
  composer_cap, composer_visible_range, composer_cursor, composer_clamped,
  top_overlay, session_state
}

FullscreenChatStateBundle {
  shell: FullscreenChatShellState,
  list: MessageListState,
  composer: ChatComposerState
}

FullscreenShellEvent =
  Resize | ConversationApplied { outcome: ApplyOutcome } | Key | Paste | Mouse |
  SetStatus | OpenOverlay | CloseTopOverlay

RevisionedFullscreenShellEvent { expected_revision: FullscreenShellRevision,
                                 event: FullscreenShellEvent }
FullscreenSessionCommand = Suspend | Resume | Shutdown
FullscreenRuntimeEvent = Shell(RevisionedFullscreenShellEvent) | Session(FullscreenSessionCommand)
```

`FullscreenTerminalSize::new(columns, rows)` 显式允许零，以便零尺寸进入可达
`UnsupportedTerminalSize`，而不是在构造前消失。config `try_new` 拒绝 min=0、max=0、
min composer > max composer。`FullscreenStatusRegion::try_new` 拒绝 zero rows或空白
accessible label；`None` 精确表达 absent。overlay rect的 zero-area由 typed error拒绝。
Modal/Pointer必须声明handler capability；Passive必须`focusable=false`且handler=None；
其他kind/focus/handler组合在constructor/open前返回`InvalidOverlayState`。

唯一state constructor签名在最终dependency命名下必须语义等价于：

```text
FullscreenChatStateBundle::try_new(
  config: FullscreenChatShellConfig,
  terminal: FullscreenTerminalSize,
  initial_entries: Vec<MessageListEntry>,
  initial_measurement_config: MessageMeasurementConfig,
  composer_state: ChatComposerState,
  composer_projection_inputs: ComposerProjectionInputs,
  status: Option<FullscreenStatusRegion>,
  initial_conversation_revision: ConversationRevision,
  initial_base_focus: FullscreenBaseFocusTarget,
  initial_overlay_sequence: Vec<FullscreenInitialOverlayOpen>,
  measure: &mut impl FnMut(MessageMeasureRequest) -> MessageMeasureOutcome
) -> Result<FullscreenChatStateBundle, FullscreenShellError>
```

这条签名必须实际创建并返回后续handler/session使用的同一MessageList与Composer states；
shell state只保存自己的revision/focus/overlay/frame metadata及last processed conversation revision，
不复制component value/revision。constructor按ordered open sequence调用与runtime相同的overlay
validation/focus transition，逐层从前一focus生成saved-focus chain和唯一final focus；不得由
caller提供矛盾的final focus或无history的nonempty stack。
`FullscreenChatStateBundle`字段private，只公开 `shell()`、`message_list()`、`composer()`只读
accessor和整体observation；不公开component `&mut`。`pub(super) split_mut()`通过一次字段解构产生
三条disjoint mutable borrows交给transaction。upstream prepared tokens必须拥有candidate/
base revision而不借用live state，candidate view借token；这样全部fallible工作结束后可消费token
并依次调用infallible commit，不需要unsafe、`RefCell`或复制state。空entries合法；callback返回
zero row或缺active key时typed失败。constructor完成前不发布partial bundle，也不长期持有
callback。MessageList active key handle由最终GH-65 concrete `Arc`-backed value拥有。

`FullscreenChatShell::try_into_element(...)`只借用bundle、conversation和本次candidate中
由同一prepared Composer token产生的projection，并接受唯一stable typed MessageList render
closure；禁止caller另传第二份projection。status与有序overlay bodies只从state的validated
values读取。closure精确接收GH-65
entry/key-handle/visible-slice并在内部使用 GH-63 `ChatMessageView`；shell不接收第二个
row-height callback，不重测 block。

### 4. Checked region partition

令 `T=terminal.rows`、`S=status.rows or 0`。必须先计算cap，再建立projection：

```text
required = checked(min_transcript + min_composer + S)
if columns < min_columns or T < required -> UnsupportedTerminalSize
composer_cap = min(max_composer, checked(T - min_transcript - S))
projection = GH64::try_project(
  current composer state/revision,
  current content width,
  max_visible_lines = NonZero(composer_cap),
  caller projection inputs
)
P = projection.height
if P == 0 or projection.cursor not in projection.visible_range
  -> InvalidComposerProjection
composer_rows = clamp(P, min_composer, composer_cap)
transcript_rows = checked(T - composer_rows - S)

transcript = rect(0, 0, columns, transcript_rows)
composer   = rect(0, transcript_rows, columns, composer_rows)
status     = S == 0 ? None
           : rect(0, transcript_rows + composer_rows, columns, S)
```

cap计算前不得调用GH-64；不得clip旧projection或复用较大cap的visible range。`P=0` 是
`InvalidComposerProjection`，不能被clamp成1；projection revision必须等于bundle中current
Composer state revision，cursor-containing visible range必须由该cap重算。所有u16/usize转换、
rect end和sum使用checked arithmetic。
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

`handle_fullscreen_shell_event`只处理pure shell domain，语义签名：

```text
handle_fullscreen_shell_event(
  bundle: &mut FullscreenChatStateBundle,
  event: RevisionedFullscreenShellEvent,
  dependencies: FullscreenShellInputs,
) -> Result<InteractionOutcome<FullscreenShellPayload>, FullscreenShellError>
```

handler内部通过bundle `pub(super) split_mut()`一次取得shell/list/composer disjoint borrows；
prepared tokens拥有candidate且不借live，禁止caller分别提供可能不匹配的三个states。
`FullscreenShellInputs` 是具体 private-field struct，含当前 immutable Conversation snapshot、
MessageList config、measurement/render closure和唯一typed overlay handler；没有
`Any`/dynamic map。
event闭集只含 Resize、ConversationApplied、Key、Paste、Mouse、SetStatus、OpenOverlay、
CloseTopOverlay。Suspend/Resume/Shutdown只属于`FullscreenSessionCommand`，不能构造为shell
event。envelope在event创建/入队时捕获expected revision，backend poll、run与dispatch只能透传，
禁止读取bundle current revision重写。ConversationApplied的`ApplyOutcome::revision()`必须等于
inputs immutable Conversation snapshot revision，且精确为shell last-processed conversation
revision的checked successor；stale、skip、replay或任意pair mismatch在upstream prepare前失败。
SetStatus和OpenOverlay的typed payload分别是status/overlay state唯一更新来源。
处理优先级：

```text
envelope expected shell revision
-> ConversationApplied outcome/snapshot/last-processed successor（若适用）
-> event kind/system precedence
-> target/id/overlay/focus validation
-> checked next shell revision (only if observable mutation is possible)
-> terminal/status minimum preflight + checked composer_cap (zero callback)
-> upstream composer/list expected revisions
-> GH-64 prepare composer mutation（非Composer event使用no-op token），再从token candidate view
   按current width + composer_cap建立cursor-containing projection
-> GH-65 try_prepare_* without live mutation
-> ordered measurement/layout using prepared read-only views
-> render closure + GH-60 checked frame candidate
-> infallible publication: commit list token, composer token, shell/observation/frame
```

stale shell revision先于callback；unknown overlay/focus先于upstream mutation；Resize的
zero/undersized/minimum/cap overflow在Composer/List prepare、measure、projection/render
callback前失败且所有callback count为0。任何prepare/measurement/layout/render error只discard
两个tokens和shell
candidate，live List/Composer/shell/frame逐值相等。publication section不能再执行callback、
allocation、conversion、validation或返回`Result`；若最终上游commit仍fallible，本issue
implementation gate失败，禁止声称rollback。Ignored、Handled-no-change、Cancelled不推进
revision；一次成功 event即使改变list/layout/focus多个字段也只推进一次。相同序列确定。rapid
`Resize -> ConversationApplied(stream) -> ConversationApplied(prepend) -> Key` 精确按该顺序；
没有background reorder、coalescing或全局fallback。

session只暴露一个total dispatch boundary：

```text
FullscreenSession::dispatch(
  &mut self,
  bundle: &mut FullscreenChatStateBundle,
  runtime_event: FullscreenRuntimeEvent,
  inputs: &mut FullscreenShellInputs<'_>
) -> Result<FullscreenDispatchOutcome, FullscreenRunError>

FullscreenDispatchOutcome =
  Shell(InteractionOutcome<FullscreenShellPayload>) |
  Session(FullscreenSessionCommandOutcome)
```

`Shell(event)`恰好调用一次上述handler；`Session(Suspend|Resume|Shutdown)`恰好调用session
state machine且不进入shell/component/overlay handler。`run`只poll `FullscreenRuntimeEvent`
并调用dispatch，调用方不能对同一event再调用第二入口。poll/decode新shell event时捕获当时
bundle revision；backend已排队event保留入队时revision，故intervening mutation后的旧event
必须stale失败，不能在dispatch处“刷新”。Resume取得新lease/snapshot/size后，
必须以`&mut bundle`和inputs prepare一个cap-first synthetic Resize candidate及CheckedFrame，
再staged enter/render/commit；terminal可在Suspended期间resize，禁止repaint旧frame。缺frame/
size或prepare失败typed返回且按session recovery合同回滚。session command error保留source。

### 7. Focus, key, paste and mouse routing

stack validation要求一旦存在 Modal，它必须是top；Modal之上不能再打开Pointer/Passive，
只允许嵌套Modal。这样“top Modal”是唯一modal barrier，避免中层modal与上层pointer的未定义
穿透。无Modal时可混合Pointer/Passive。keyboard/paste总表：

| Overlay/focus state | Event | Sole handler | Overall outcome / propagation |
| --- | --- | --- | --- |
| 任意 | Resize | shell prepare transaction | 不进入input handler |
| 任意 | `Session(Suspend/Resume/Shutdown)` | session dispatch | 不进入shell/component handler |
| top Modal、dismissible | Escape | shell CloseTop | close一层并恢复saved focus；不调用overlay/base |
| top Modal、不可dismiss | Escape | top Modal handler | `Ignored`提升为`Handled`；consumed |
| top Modal | 其他key或Paste | top Modal handler | 任意handler outcome均停止；`Ignored`提升为`Handled` |
| 无Modal、top且focused dismissible Pointer | Escape | shell CloseTop | close一层；不向lower target继续 |
| 无Modal、top Passive | Escape | none/当前eligible focus | Passive不关闭、不消费；按普通focused route |
| 无Modal | Tab/BackTab | shell traversal | 使用下述固定ring、方向与wrap；Passive跳过 |
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

无Modal的focus ring每次从committed overlay stack机械建立：

```text
forward = [Transcript, Composer]
          + focusable Pointer overlays in stack bottom -> top order
backward = reverse(forward)
```

Tab/BackTab先验证current target仍存在且eligible，然后从current后的下一项开始；不把current
再次作为首项。Tab在forward最后一项wrap到Transcript，BackTab在Transcript wrap到forward
最后一项；没有Pointer时精确在Transcript/Composer间wrap。Passive、`focusable=false`与closed
ID不进入ring；Modal存在时Tab/BackTab由top Modal捕获而不建ring。一次traversal只推进一次shell
revision。tests覆盖0/1/多Pointer、bottom/top stack边界、两个方向、所有wrap和current closed。

### 8. Checked frame and terminal session

`FullscreenFrameTransaction` 私有地持有两个upstream prepared tokens、lightweight shell
candidate、Element tree、GH-60 checked layout/render result与previous observation；只有
所有fallible工作完成后才能进入infallible `commit()`。任一callback panic在publication前
unwind，由session guard恢复terminal，不能catch后default render。

公共terminal surface必须可crate外实现/调用，并且snapshot provenance与recovery ownership
可检查：

```text
FullscreenTerminalCapability = SupportedRestorable | Unsupported
FullscreenTerminalCapabilities { raw, cursor, alternate, mouse, focus, paste }
FullscreenSnapshotEvidence =
  NativeQuery { tty_identity, lease_epoch, correlated_mode_replies } |
  ManagedRegistry { tty_identity, lease_epoch } |
  DeterministicFake { fixture_id }
FullscreenScreenModes { mode_47, mode_1047, mode_1049 }
FullscreenMouseModes { mode_1000, mode_1002, mode_1003, mode_1015, mode_1006 }
VerifiedFullscreenTerminalSnapshot {
  screen: FullscreenScreenModes,
  raw_termios, cursor_mode_25,
  mouse: FullscreenMouseModes,
  focus_mode_1004, paste_mode_2004,
  evidence: FullscreenSnapshotEvidence
}
OptionalCapabilityPolicy = Require | Disable
FullscreenSessionConfig { mouse, focus, paste: OptionalCapabilityPolicy, poll_timeout: Duration }
trait FullscreenTerminalBackend {
  type Error: std::error::Error + Send + Sync + 'static;
  type Lease: FullscreenTerminalLease;
  type RecoveryOwner: FullscreenTerminalRecoveryOwner<Error = Self::Error>;
  fn try_acquire_lease(&mut self) -> Result<Self::Lease, Self::Error>;
  fn try_release_lease(&mut self, lease: &mut Self::Lease) -> Result<(), Self::Error>;
  fn try_size(&mut self, lease: &mut Self::Lease)
    -> Result<FullscreenTerminalSize, Self::Error>;
  fn capabilities(&self, lease: &Self::Lease) -> FullscreenTerminalCapabilities;
  fn try_snapshot(&mut self, lease: &mut Self::Lease)
    -> Result<VerifiedFullscreenTerminalSnapshot, Self::Error>;
  fn try_apply(&mut self, lease: &mut Self::Lease, transition: FullscreenTerminalTransition)
    -> Result<(), Self::Error>;
  fn try_render(&mut self, lease: &mut Self::Lease, frame: &CheckedFrame)
    -> Result<(), Self::Error>;
  fn try_poll(&mut self, lease: &mut Self::Lease, timeout: Duration,
              current_revision_for_new_events: FullscreenShellRevision)
    -> Result<Option<FullscreenRuntimeEvent>, Self::Error>;
  fn try_flush(&mut self, lease: &mut Self::Lease) -> Result<(), Self::Error>;
  fn into_recovery_owner(
    self, lease: Self::Lease, context: FullscreenRecoveryContext,
    unfinished: NonEmptyRestorationSteps, failures: NonEmptyRestorationFailures
  ) -> Self::RecoveryOwner;
  fn transfer_poisoned(owner: Self::RecoveryOwner) -> FullscreenPoisonOwnerId;
}
FullscreenRecoveryPrimary = Start(FullscreenSessionStartError) | Run(FullscreenRunPrimaryError)
FullscreenRecoveryContext { snapshot: Option<VerifiedFullscreenTerminalSnapshot>,
                            primary: Option<FullscreenRecoveryPrimary> }
trait FullscreenTerminalRecoveryOwner {
  type Error: std::error::Error + Send + Sync + 'static;
  fn try_restore_next(&mut self) -> Result<FullscreenRecoveryProgress, Self::Error>;
  fn try_release(&mut self) -> Result<(), Self::Error>;
  fn context(&self) -> &FullscreenRecoveryContext;
  fn unfinished_steps(&self) -> &NonEmptyRestorationSteps;
}
NativeFullscreenTerminalBackend::try_controlling_terminal()
  -> Result<Self, FullscreenSessionStartError>
FullscreenSession<B>::try_enter(
  backend: B, config: FullscreenSessionConfig,
  bundle: &mut FullscreenChatStateBundle, inputs: &mut FullscreenShellInputs<'_>
)
  -> FullscreenEnterOutcome<B>
FullscreenEnterOutcome<B> =
  Active(FullscreenSession<B>) |
  Rejected(FullscreenRejectedEnter<B>) |
  RecoveryRequired(FullscreenTerminalRecovery<B::RecoveryOwner>)
FullscreenRejectedEnter<B>::{error,backend,pending_event_count,into_backend}()
FullscreenTerminalRecovery<O>::retry(&mut self) -> Result<FullscreenRecoveryComplete, NonEmptyRestorationFailures>
FullscreenTerminalRecovery<O>::{unfinished_steps,snapshot,lease_owner,primary,failure_history}()
run(
  &mut self,
  bundle: &mut FullscreenChatStateBundle,
  inputs: &mut FullscreenShellInputs<'_>,
  panic_reporter: &mut impl FullscreenPanicCleanupReporter
) -> Result<FullscreenSessionExit, FullscreenRunError>
dispatch(&mut self, bundle, FullscreenRuntimeEvent, inputs)
  -> Result<FullscreenDispatchOutcome, FullscreenRunError>
render_frame(&mut self, &CheckedFrame) -> Result<(), FullscreenRunPrimaryError>
try_suspend(&mut self) -> FullscreenSuspendOutcome
try_resume(&mut self, bundle: &mut FullscreenChatStateBundle, inputs: &mut FullscreenShellInputs<'_>)
  -> FullscreenResumeOutcome
try_shutdown(&mut self) -> FullscreenShutdownOutcome
try_recover(&mut self) -> FullscreenRecoveryOutcome
NativeFullscreenTerminalBackend::try_claim_poisoned_controlling_terminal()
  -> Result<FullscreenTerminalRecovery<Self::RecoveryOwner>, FullscreenLeaseClaimError>
```

`FullscreenTerminalLease`只公开active/owner/epoch accessor，不可clone/construct；token由Active
session、failed-entry guard、RecoveryRequired或process registry四者之一唯一拥有。registry为
`Free | Held(owner,epoch) | Poisoned(owner,unfinished)`。
`FullscreenTerminalTransition`闭集为`SetScreen(FullscreenScreenModes)`、
`SetRawTermios`、`SetCursorMode25`、`SetMouseModes(FullscreenMouseModes)`、
`SetFocusMode1004`、`SetPasteMode2004`；
lease acquire/release不能伪装成mode transition。每次调用`try_apply`前，session先把完整
snapshot target和attempt编号登记为`AttemptedMayHaveMutated` unfinished step；backend error
不能被解释为零mutation。成功后标记Applied，失败仍按完整target回滚。`SetMouseModes`等grouped
transition即使只写入部分mode也必须恢复整组snapshot；全部attempted/applied step恢复并flush
成功前禁止release lease。

Native backend在`src/renderer/terminal/fullscreen_backend.rs`拥有process-wide controlling-TTY
lease/mode registry与配对input/output I/O。`try_controlling_terminal()`打开并验证input/output
属于同一TTY identity，零lease、零terminal mutation；只有stdout或TTY identity不一致时typed
拒绝。首次或registry epoch失效时，acquire后用input termios读取raw，并通过output向同一TTY
发送无mode mutation的DECRQM查询：
47/1047/1049 screen、25 cursor、1000/1002/1003 tracking、1015 RXVT、1006 SGR、1004 focus、
2004 paste；
query前先保存原termios，在同一lease/TTY上进入temporary noncanonical/no-echo、`VMIN=0`且
bounded `VTIME`的QueryInputPhase，使无newline的DECRPM bytes可读取；该phase不是fullscreen raw
成功状态。所有reply收齐或任何parse/timeout/read失败后，都先恢复exact saved termios并flush，
两者成功后才完成snapshot/capability preflight或返回Rejected。query-phase apply、partial read、
termios restore或flush失败均进入带saved termios、pending input、unfinished QueryTermios/
QueryFlush/LeaseRelease的RecoveryRequired，禁止继续alternate/raw entry。
只接受从配对input读取、与query ID/TTY/lease epoch完全关联且明确enabled/disabled的reply；
非reply input用`try_enter`收到的bundle revision封装后按原序进入pending-event queue。timeout、unsupported、
malformed、ambiguous/extra reply或无法读取termios返回`UnknownPreEntryState`；只有query
termios restore+flush成功才可release。成功release的`Rejected`归还原backend，pending queue
保持byte/event顺序；recovery owner接管时queue随backend唯一转移。registry仍在同一exclusive epoch内时可产生`ManagedRegistry`
evidence；跨release后必须重新query，不能信任旧bool。unsupported平台只允许deterministic fake
contract或typed Unsupported。snapshot必须逐bit保存上述raw modes并原样恢复，包括crossterm
同时启用1000/1002/1003/1015/1006的组合；禁止折叠成enum/bool或默认disabled。

现有public `Terminal::{enter,enter_inline,suspend,resume,exit}`与`App::run`必须改用同一registry。
`Terminal::new`零mutation；首次enter取得并保存managed lease/snapshot，nested legacy/new
session相互返回AlreadyActive。legacy partial failure由该`Terminal`值继续持有recovery state；
若消费式`App::run`或Drop结束仍未恢复，必须把backend/token/snapshot/unfinished steps原子转移到
registry Poisoned record，后续entry全部blocked，只有
`try_claim_poisoned_controlling_terminal`能转移唯一
ownership并retry。legacy `io::Error`保留typed source；禁止绕过registry直接crossterm enter。

`src/renderer/terminal_controller.rs`的screen/cursor/mouse命令也必须要求当前managed owner/
lease并经同一transition API执行；无owner时typed/io source拒绝。`src/runtime/panic_handler.rs`
不得在active managed lease外直接disable raw/leave screen/show cursor或清mouse/focus/paste：
有owner时只触发该owner unwind recovery table；无可借owner时原子claim/transfer registry
Poisoned record后恢复，失败继续Poisoned并保存全部sources。panic hook的文本不得声称restored，
除非typed report逐项成功。

entry顺序为acquire→query-phase verified snapshot→capability preflight→fresh size→以bundle和
inputs prepare cap-first synthetic Resize/CheckedFrame→alternate→raw→cursor-hide→optional
mouse/focus/paste→flush→render prepared frame→infallible bundle/frame commit。每步只在目标值
与snapshot不同时执行并记录；size/prepare失败零fullscreen mutation并归还backend；transition/
render失败discard candidate并按反向顺序尝试全部
completed steps并flush。只有restore+flush+release全部成功才返回`Rejected(primary)`并使registry
Free，同时`FullscreenRejectedEnter`归还backend及未消费pending input；任一rollback或release
失败返回`RecoveryRequired(guard)`，guard保持backend/queue、lease、optional snapshot、
Start-or-Run primary和完整ordered failures；cleanup-only的primary=None。query完成前失败则
context保存saved termios且unfinished至少含QueryTermios/QueryFlush/LeaseRelease中的实际剩余项，
只在query state恢复后release；release失败仍保留同一token/Poisoned ownership。没有Active
session不能render/poll。首次成功publication必须使用lease内fresh size，不能提交constructor
旧size frame。

session拥有`Option<B>`，使失败/Drop可take backend并构造`B::RecoveryOwner`；Active、
recovery guard、process registry之间始终恰好一个owner。lifecycle闭集为Entering、Active、
Suspending、Suspended、Resuming、
`RecoveryRequired { operation, lease, snapshot, unfinished }`、Closing、Shutdown。shutdown与
suspend停止poll并尝试全部restoration；只在restore+flush+release全部成功后进入Shutdown/
Suspended。resume lease冲突保持Suspended且零mutation；取得lease后的entry/repaint失败若完整
rollback+release则回Suspended，任一步不完整则进入RecoveryRequired而不是伪称Suspended。
该state只允许`try_recover`；render/poll/dispatch/second entry均typed拒绝。recovery逐次只重试
unfinished/failed stages，保存每次所有sources，全部成功才release并转到操作对应稳定state。

`run`只poll/dispatch `FullscreenRuntimeEvent`，bundle是唯一state来源。unwind guard使用同一
recovery table；panic cleanup全部成功后report并`resume_unwind`，不完整则先把session变为
RecoveryRequired/registry Poisoned并报告owner ID、unfinished steps与全部sources，再继续
unwind。Drop只best-effort retry；失败时转移Poisoned ownership，绝不释放不确定lease或构成
恢复成功证据。public report不暴露panic `Any` payload。

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
  Backend(FullscreenTerminalOperationError) |
  SessionTransition(FullscreenSessionTransitionError)

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

restoration step闭集为QueryTermios、QueryFlush、RawMode、Cursor、AlternateScreen、
MouseCapture、FocusReporting、BracketedPaste、Flush、LeaseRelease；每项在I/O前保存typed
attempt state、完整snapshot restoration target、backend source、attempt number与owner/lease
epoch。`NonEmptyRestorationFailures`拥有每个source而非string化，
同一step多次失败也按attempt顺序保留。
`PrimaryAndCleanup`的`source()`返回primary source，`primary()`与`cleanup_failures()`分别允许
无损检查原layout/render类别和全部cleanup steps；`Display`只输出类别与failure count，不
回显terminal payload。fake test必须注入primary render failure、grouped mouse transition只写入
部分mode后失败、三个不同cleanup sources和一次release retry failure，断言完整snapshot target
在flush/release前恢复、primary仍可downcast、所有source/step/attempt/order完整、
RecoveryRequired保持唯一owner且没有覆盖/吞掉。

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
`bfc60f26164af5df1ebd3b5cb79d07379fc416b7`；被审规格唯一来源是
`https://github.com/majiayu000/rnk.git` 的PR current exact head。前置只需PATH中的Git、
GitHub CLI、Python 3.9+与tar；checkers仅使用Python标准库，不执行`pip install`。任何reviewer
从fresh checkout运行：

```bash
set -euo pipefail
command -v git
command -v gh
command -v python3
command -v tar
python3 -c 'import sys; assert sys.version_info >= (3, 9)'
SPEC_RAIL_URL="https://github.com/majiayu000/specrail.git"
SPEC_RAIL_COMMIT="bfc60f26164af5df1ebd3b5cb79d07379fc416b7"
GH67_RNK_URL="https://github.com/majiayu000/rnk.git"
case "$GH67_PR_NUMBER" in ''|*[!0-9]*) exit 64 ;; esac
GH67_REVIEWED_RNK_HEAD="$(gh pr view "$GH67_PR_NUMBER" --repo majiayu000/rnk \
  --json headRefOid --jq .headRefOid)"
GH67_REVIEWED_PR_BASE_SHA="$(gh api \
  "repos/majiayu000/rnk/pulls/$GH67_PR_NUMBER" --jq .base.sha)"
git fetch --prune origin main
GH67_REVIEWED_CURRENT_MAIN_SHA="$(git rev-parse origin/main)"
GH67_REVIEWED_MERGE_BASE_SHA="$(
  git merge-base "$GH67_REVIEWED_CURRENT_MAIN_SHA" "$GH67_REVIEWED_RNK_HEAD"
)"
for sha in "$GH67_REVIEWED_RNK_HEAD" "$GH67_REVIEWED_PR_BASE_SHA" \
  "$GH67_REVIEWED_CURRENT_MAIN_SHA" "$GH67_REVIEWED_MERGE_BASE_SHA"; do
  case "$sha" in ''|*[!0-9a-f]*) exit 65 ;; esac
  test "${#sha}" -eq 40
done
test "$(git rev-parse HEAD)" = "$GH67_REVIEWED_RNK_HEAD"
test "$(git rev-parse origin/main)" = "$GH67_REVIEWED_CURRENT_MAIN_SHA"
test "$(git merge-base "$GH67_REVIEWED_CURRENT_MAIN_SHA" \
  "$GH67_REVIEWED_RNK_HEAD")" = "$GH67_REVIEWED_MERGE_BASE_SHA"
test -z "$(git status --porcelain)"
GH67_VERIFY_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/rnk-gh67-specrail.XXXXXX")"
trap 'rm -rf "$GH67_VERIFY_ROOT"' EXIT
SPEC_RAIL_CHECKOUT="$GH67_VERIFY_ROOT/specrail"
GH67_RNK_CHECKOUT="$GH67_VERIFY_ROOT/rnk"
GH67_SPEC_MIRROR="$GH67_VERIFY_ROOT/mirror"
mkdir -p "$SPEC_RAIL_CHECKOUT" "$GH67_RNK_CHECKOUT" "$GH67_SPEC_MIRROR"
git -C "$SPEC_RAIL_CHECKOUT" init -q
git -C "$SPEC_RAIL_CHECKOUT" remote add origin "$SPEC_RAIL_URL"
git -C "$SPEC_RAIL_CHECKOUT" fetch --depth=1 origin "$SPEC_RAIL_COMMIT"
git -C "$SPEC_RAIL_CHECKOUT" checkout --detach FETCH_HEAD
test "$(git -C "$SPEC_RAIL_CHECKOUT" rev-parse HEAD)" = "$SPEC_RAIL_COMMIT"
git -C "$GH67_RNK_CHECKOUT" init -q
git -C "$GH67_RNK_CHECKOUT" remote add origin "$GH67_RNK_URL"
git -C "$GH67_RNK_CHECKOUT" fetch --depth=1 origin "$GH67_REVIEWED_RNK_HEAD"
git -C "$GH67_RNK_CHECKOUT" checkout --detach FETCH_HEAD
test "$(git -C "$GH67_RNK_CHECKOUT" rev-parse HEAD)" = "$GH67_REVIEWED_RNK_HEAD"
test "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
  "$SPEC_RAIL_CHECKOUT/checks/check_workflow.py")" = \
  "c5bd73060037b0e8febace0e5ee8473e17973e1ca17257ea1517a94e05fa7549"
test "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' \
  "$SPEC_RAIL_CHECKOUT/tools/spec_depth_audit.py")" = \
  "380169fcbad509e6bc1b6a555ae0fa469744662af7120e20e999206c226e66c3"

git -C "$SPEC_RAIL_CHECKOUT" archive "$SPEC_RAIL_COMMIT" | tar -x -C "$GH67_SPEC_MIRROR"
GH67_SPEC_REFS="
specs/GH67/product.md specs/GH67/tech.md specs/GH67/tasks.md
specs/GH57/product.md specs/GH57/tech.md specs/GH57/tasks.md
specs/GH62/product.md specs/GH62/tech.md specs/GH62/tasks.md
specs/GH63/product.md specs/GH63/tech.md specs/GH63/tasks.md
specs/GH64/product.md specs/GH64/tech.md specs/GH64/tasks.md
"
test "$(printf '%s\n' $GH67_SPEC_REFS | sed '/^$/d' | sort -u | wc -l | tr -d ' ')" = 15
for ref in $GH67_SPEC_REFS; do
  test -f "$GH67_RNK_CHECKOUT/$ref"
  mkdir -p "$GH67_SPEC_MIRROR/$(dirname "$ref")"
  cp "$GH67_RNK_CHECKOUT/$ref" "$GH67_SPEC_MIRROR/$ref"
  source_sha="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$GH67_RNK_CHECKOUT/$ref")"
  mirror_sha="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$GH67_SPEC_MIRROR/$ref")"
  test -n "$source_sha"
  test "$source_sha" = "$mirror_sha"
  printf '%s  %s\n' "$source_sha" "$ref"
done
python3 - "$GH67_RNK_CHECKOUT/specs/GH67/tech.md" <<'PY'
import json, re, sys
text = open(sys.argv[1], encoding="utf-8").read()
blocks = re.findall(r"<!-- specrail-planned-changes\s*(\{.*?\})\s*-->", text, re.S)
assert len(blocks) == 1
refs = json.loads(blocks[0])["spec_refs"]
expected = [
    f"specs/GH{issue}/{name}.md"
    for issue in (67, 57, 62, 63, 64)
    for name in ("product", "tech", "tasks")
]
assert refs == expected
PY
python3 "$SPEC_RAIL_CHECKOUT/checks/check_workflow.py" \
  --repo "$GH67_SPEC_MIRROR" --spec-dir specs/GH67
python3 "$SPEC_RAIL_CHECKOUT/tools/spec_depth_audit.py" \
  --repo "$GH67_SPEC_MIRROR" --spec-dir specs/GH67 --gate
```

URL/commit/checksum是常量；任一步失败即失败，禁止fallback；两exact tests在隔离temp内断言
15 refs/SHA、remote commits、checksums与两checker。

## Product-to-Test Mapping

所有 filtered tests先 `--list --exact` 要求 matched=1，再
`--include-ignored --exact` 要求 `1 passed; 0 failed; 0 ignored`。

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | facade/owning bundle/types/exports | `fullscreen_shell_public_surface_is_typed_and_controlled`；`owning_state_bundle_preserves_single_component_revisions` |
| B-002 | owning bundle constructor | `constructor_requires_complete_entries_config_projection_and_measurement`；`owning_state_bundle_preserves_single_component_revisions`；`initial_overlay_sequence_builds_lifo_focus_restoration` |
| B-003 | layout partition | `fixed_bottom_partition_uses_exact_remaining_rows`；`gh67_fixed_bottom_resize_contract` |
| B-004 | config/zero/undersized/error precedence | `zero_and_undersized_terminals_fail_before_callbacks` |
| B-005 | optional status | `status_absence_uses_zero_rows_and_invents_no_data` |
| B-006 | cap-first Composer reproject | `composer_projection_clamps_without_overlap_and_keeps_draft`；`composer_cap_reprojects_cursor_window_before_partition` |
| B-007 | MessageList facade only | `variable_height_transcript_uses_rows_not_item_count` |
| B-008 | cache identity/active handles | `measurement_invalidation_and_active_handles_follow_exact_identity` |
| B-009 | Following/nonzero shell viewport | `following_stream_growth_tracks_latest_bottom_in_supported_viewport`；`zero_and_undersized_terminals_fail_before_callbacks` |
| B-010 | Paused/new output | `paused_stream_growth_preserves_anchor_and_reports_new_content` |
| B-011 | prepend | `prepend_preserves_stable_message_and_intra_row_anchor` |
| B-012 | continuous resize | `continuous_resize_reflows_list_and_composer_in_one_frame`；`gh67_fixed_bottom_resize_contract` |
| B-013 | GH-63 typed views | `typed_multiline_block_views_render_once_in_source_order` |
| B-014 | public example | `rnk_chat_example_uses_only_public_fullscreen_composition`；`cargo check --example rnk_chat --all-features --locked` |
| B-015 | focus observation/revision | `public_observation_reports_focus_regions_follow_and_overlay`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-016 | total input/session domains + traversal | `focus_overlay_key_routing_is_single_target_and_deterministic`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected`；`pointer_overlay_tab_order_wraps_deterministically`；`shell_events_and_session_commands_are_disjoint_and_total` |
| B-017 | overlay state/stack/z-order | `nested_overlay_z_order_and_invalid_updates_are_atomic`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-018 | Escape/focus/fallthrough | `nested_overlay_escape_restores_focus_lifo_without_fallthrough`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected`；`passive_escape_falls_through_without_close` |
| B-019 | committed input/paste | `paste_and_committed_ime_text_dispatch_exactly_once` |
| B-020 | total mouse hit/fallthrough | `mouse_hit_testing_uses_committed_z_order_without_double_dispatch`；`overlay_route_matrix_is_total_and_passive_focus_is_rejected` |
| B-021 | rapid event/revision | `rapid_resize_stream_prepend_sequence_is_deterministic`；`conversation_outcome_snapshot_revision_binding_is_atomic`；`revisioned_runtime_event_rejects_queued_stale_shell_input` |
| B-022 | prepared upstream/overflow atomicity | `upstream_prepare_commit_abort_gate_and_late_failure_are_atomic`；`coordinate_revision_and_upstream_failures_are_atomic` |
| B-023 | checked frame/dual failure | `layout_render_failure_preserves_committed_state_and_frame`；`primary_failure_and_all_cleanup_failures_are_preserved`；`partial_grouped_transition_restores_full_snapshot_before_release` |
| B-024 | snapshot/entry/lease/recovery | `fullscreen_session_public_surface_and_capability_gate_are_typed`；`fullscreen_terminal_restores_all_modes_on_every_exit_path`；`native_snapshot_bootstrap_legacy_lease_and_poison_recovery_are_total`；`initial_enter_requeries_size_and_stages_cap_first_frame`；`rejected_enter_returns_backend_and_pending_input_in_order`；`public_native_snapshot_constructor_is_reachable_and_restorable`；`canonical_tty_query_phase_reads_newline_free_replies_and_restores_termios` |
| B-025 | dispatch/suspend/restart | `partial_enter_and_suspend_resume_restore_exact_snapshot`；`suspend_resume_and_fresh_restart_rebuild_explicit_state`；`shell_events_and_session_commands_are_disjoint_and_total`；`initial_enter_requeries_size_and_stages_cap_first_frame` |
| B-026 | accessibility/golden | `accessibility_and_plain_ansi_semantics_do_not_depend_on_color` |
| B-027 | bounded work/handles | `visible_frame_work_is_bounded_and_handles_are_o1_non_evictable` |
| B-028 | security audit | `fullscreen_shell_has_no_provider_tool_or_secret_execution_surface` |
| B-029 | dependency/capability/path gate | `dependency_completion_requires_closed_final_merged_ancestor_sets`；`upstream_prepare_commit_abort_gate_and_late_failure_are_atomic` |
| B-030 | exact/reproducible evidence | mapping全部 tests；`gh67_current_head_coverage_contract`；`coverage_validate_environment_survives_full_verification`；`specrail_checker_checkout_is_reproducible`；`specrail_mirror_binds_all_reviewed_dependency_refs`；full gates/CI/review |

## Data Flow

- 输入：Conversation/ApplyOutcome、bundle-owned Composer/List prepared APIs、borrowed render path、typed terminal/status/overlay/events与session config。
- 处理：唯一dispatch→revision/preflight/cap→两upstream prepare→measure/layout/render→infallible commit；失败discard，session逐项恢复。
- 输出：dispatch/interaction/observation/frame或typed shell/run error；双失败同时拥有primary与nonempty cleanup。
- 外部：无provider/network/tool/secret/storage；进程内持有state，fresh restart不跨进程恢复。

## 备选方案

- 拒绝给layout加chat state、item-count scroll、广播hooks或复制GH-65 height/TextFlow。
- 拒绝立即commit后clone rollback、`Option<Element>`、旧frame fallback及Drop-only cleanup。
- 拒绝Inline/Fullscreen flag混淆scrollback与owned-frame lifecycle。

## 风险

- Dependency drift：path、final ancestry、capability与source-drift reapproval阻断未完成上游。
- Correctness：单transaction、checked rect、failure equality与closed route table。
- Terminal：lease、query-phase snapshot、attempted-step rollback、retry recovery与PTY/fake。
- Performance/security：O(1) handles/visible slices；shell无raw ANSI/tool execution。

## 测试计划

- [ ] config/partition property覆盖边界；owning bundle证明同一三state revision且无复制/unsafe。
- [ ] MessageList/Composer覆盖全部mutation/follow/anchor/cap-first resize及zero preflight。
- [ ] overlay/focus/key/paste/mouse逐格single target；多Pointer traversal及两domain exact once。
- [ ] typed blocks、plain/ANSI/accessibility、prepared late-failure equality全部覆盖。
- [ ] PTY/fake覆盖canonical query、pending events、legacy/shared lease、entry/exit/panic/suspend/resume、Poisoned与所有sources。
- [ ] fresh exact checkouts校验15 refs/SHA、workflow/depth及local/CI/review/thread/gate evidence。

## 回滚方案

GH-67无数据migration；未merge时关闭PR，已merge时普通revert paths并先回滚GH-68依赖。禁止force push/silent-disable/private fallback；保留evidence，issue保持open。
