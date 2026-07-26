# Tech Spec：固定底部区域的 FullscreenChatShell

## Linked Issue

GH-67: https://github.com/majiayu000/rnk/issues/67

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":67,"complete":true,"paths":["specs/GH67/product.md","specs/GH67/tech.md","specs/GH67/tasks.md","src/components/chat/fullscreen.rs","src/components/chat/fullscreen/types.rs","src/components/chat/fullscreen/error.rs","src/components/chat/fullscreen/layout.rs","src/components/chat/fullscreen/state.rs","src/components/chat/fullscreen/router.rs","src/components/chat/fullscreen/session.rs","src/components/chat/fullscreen/tests.rs","src/components/chat/mod.rs","src/components/mod.rs","src/prelude.rs","examples/rnk_chat.rs","tests/fullscreen_chat_shell_public_api.rs","tests/fullscreen_chat_shell_interactions.rs","tests/fullscreen_chat_shell_pty.rs","tests/golden/fullscreen_chat_shell.txt","tests/golden/fullscreen_chat_shell.ansi.txt"],"spec_refs":["specs/GH67/product.md","specs/GH67/tech.md","specs/GH67/tasks.md","specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md","specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md","specs/GH63/product.md","specs/GH63/tech.md","specs/GH63/tasks.md","specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md","specs/GH65/product.md","specs/GH65/tech.md","specs/GH65/tasks.md"]}
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
contract；implementation 必须从 final merged main 重新审计。

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
6. 重新读取最终 public constructors/accessors/errors和真实 paths，并与本 manifest比较。

若 `ChatMessageView`、Composer projection/handler、MessageList state/render closure/observation、
GH-60 checked frame或 terminal session接口有语义漂移，先更新本 packet并重新人工 review。
只改命名且完全等价也必须记录 source-drift diff；不得创建 alias、`Any` adapter、private-field
hack、第二 height cache或复制 GH-65 尚未解决的缺陷。

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
```

`chat/mod.rs` 增加 module/re-export且继承最终 chat root不可降级的
`#![forbid(missing_docs)]`。`components/mod.rs` 与 `prelude.rs` 只导出 app-facing concrete
types，不创建 type alias。生产文件目标 200–400 行且均 <800。

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
FullscreenOverlayCapture =
  Modal | Pointer | Passive
FullscreenOverlayRequest {
  id, capture, dismissible, rect: FullscreenRect, body: Element
}
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
Paused；不得在调用前用 current visible top覆盖 requested anchor。Following/zero viewport、
Paused/stored anchor、new-content与clamp顺序完全继承最终已修复 GH-65合同。

### 6. State transaction and event ordering

`handle_fullscreen_shell_event` 语义签名：

```text
handle_fullscreen_shell_event(
  state: &mut FullscreenChatShellState,
  expected_revision: FullscreenShellRevision,
  event: FullscreenShellEvent,
  dependencies: FullscreenShellInputs,
) -> Result<InteractionOutcome<FullscreenShellPayload>, FullscreenShellError>
```

`FullscreenShellInputs` 是具体 private-field struct，含当前 immutable Conversation snapshot、
Composer projection、MessageList config和typed callbacks；没有 `Any`/dynamic map。
event至少含 Resize、ConversationApplied、Key、Paste、Mouse、SetStatus、OpenOverlay、
CloseTopOverlay、Suspend、Resume、Shutdown。SetStatus和OpenOverlay的typed payload分别是
status/overlay state的唯一更新来源。处理优先级：

```text
expected shell revision
-> event kind/system precedence
-> target/id/overlay/focus validation
-> checked next shell revision (only if observable mutation is possible)
-> upstream composer/list expected revisions
-> ordered measurement/layout candidates
-> render closure + GH-60 checked frame candidate
-> publish state/list/composer/observation/frame once
```

stale shell revision先于callback；unknown overlay/focus先于upstream mutation；arithmetic/layout
先于frame commit。Ignored、Handled-no-change、Cancelled不推进 revision；一次成功 event即使
改变list/layout/focus多个字段也只推进一次。相同序列确定。rapid
`Resize -> ConversationApplied(stream) -> ConversationApplied(prepend) -> Key` 精确按该顺序；
没有background reorder、coalescing或全局fallback。

### 7. Focus, key, paste and mouse routing

单一 router table：

| Precondition/event | Sole target | Outcome |
| --- | --- | --- |
| Resize/Suspend/Resume/Shutdown | session transaction | 不进入component handlers |
| top modal + Escape + dismissible | top overlay close | close一层，恢复saved focus |
| top modal + other key/paste/mouse | top overlay | 即使overlay返回Ignored也consumed，不穿透 |
| top pointer overlay + mouse in rect | topmost hit overlay | 一个overlay handler |
| top passive/pointer + Escape dismissible | top overlay close | close一层 |
| no modal + Tab/BackTab | shell focus traversal | Transcript↔Composer；overlay focus按stack |
| focused Composer + key/committed input | GH-64 key handler | exactly once |
| focused Composer + Paste | GH-64 paste handler | exactly once；不转key/submit |
| focused Transcript + navigation key/wheel | GH-65 typed navigation | exactly once |
| mouse in status | passive status | Handled-no-change，不抢focus |
| mouse outside committed rects | none | Ignored |

打开 overlay前保存 current focus；ID必须唯一。关闭只允许 top ID，按LIFO逐层恢复仍存在且可用
的saved focus，否则返回 `InvalidFocusRestore` 且candidate不提交。IME只支持 runtime交付的
committed text；preedit/candidate明确 unsupported。hit-test只读已提交 layout，resize
candidate未成功时旧layout仍是唯一有效坐标。

### 8. Checked frame and terminal session

`FullscreenFrameTransaction` 私有地持有 lightweight candidate、Element tree、GH-60 checked
layout/render result与previous observation；只有 `commit()` 可替换state/frame。任一
MessageList/Composer/renderer closure panic自然unwind，由session guard恢复terminal，不能
catch后default render。typed error保留完整 source chain：

```text
FullscreenShellError =
  Config(FullscreenConfigError) |
  State(FullscreenStateError) |
  Layout(FullscreenLayoutError) |
  MessageList(final GH65 closed errors) |
  Composer(ChatComposerError) |
  CheckedRender(final GH60 error) |
  Terminal(FullscreenTerminalError)
```

`FullscreenTerminalError` 分别记录 enter、render/write、suspend、resume与
`RestorationFailures(Vec<FullscreenRestorationStepFailure>)`；step闭集为 RawMode、Cursor、
AlternateScreen、MouseCapture、FocusReporting、BracketedPaste。cleanup执行全部步骤并聚合，
不能在第一项失败后提前返回。显式 `run` 返回 cleanup error；Drop只best-effort保险且不覆盖
显式结果。normal/cancel/typed failure/panic由同一guard；resume清旧frame并强制full repaint。

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
| B-009 | Following | `following_stream_growth_and_zero_viewport_restore_latest_bottom` |
| B-010 | Paused/new output | `paused_stream_growth_preserves_anchor_and_reports_new_content` |
| B-011 | prepend | `prepend_preserves_stable_message_and_intra_row_anchor` |
| B-012 | continuous resize | `continuous_resize_reflows_list_and_composer_in_one_frame`；`gh67_fixed_bottom_resize_contract` |
| B-013 | GH-63 typed views | `typed_multiline_block_views_render_once_in_source_order` |
| B-014 | public example | `rnk_chat_example_uses_only_public_fullscreen_composition`；`cargo check --example rnk_chat --all-features --locked` |
| B-015 | focus observation/revision | `public_observation_reports_focus_regions_follow_and_overlay` |
| B-016 | key precedence | `focus_overlay_key_routing_is_single_target_and_deterministic` |
| B-017 | overlay stack/z-order | `nested_overlay_z_order_and_invalid_updates_are_atomic` |
| B-018 | Escape/focus restore | `nested_overlay_escape_restores_focus_lifo_without_fallthrough` |
| B-019 | committed input/paste | `paste_and_committed_ime_text_dispatch_exactly_once` |
| B-020 | mouse hit test | `mouse_hit_testing_uses_committed_z_order_without_double_dispatch` |
| B-021 | rapid event/revision | `rapid_resize_stream_prepend_sequence_is_deterministic` |
| B-022 | overflow/error atomicity | `coordinate_revision_and_upstream_failures_are_atomic` |
| B-023 | checked frame | `layout_render_failure_preserves_committed_state_and_frame` |
| B-024 | terminal paths | `fullscreen_terminal_restores_all_modes_on_every_exit_path` |
| B-025 | suspend/restart | `suspend_resume_and_fresh_restart_rebuild_explicit_state` |
| B-026 | accessibility/golden | `accessibility_and_plain_ansi_semantics_do_not_depend_on_color` |
| B-027 | bounded work/handles | `visible_frame_work_is_bounded_and_handles_are_o1_non_evictable` |
| B-028 | security audit | `fullscreen_shell_has_no_provider_tool_or_secret_execution_surface` |
| B-029 | dependency gate | `dependency_completion_requires_closed_final_merged_ancestor_sets` |
| B-030 | exact evidence | mapping全部 tests；`gh67_current_head_coverage_contract`；full gates/CI/review |

## Data Flow

### 输入

- final GH-62 immutable Conversation snapshot/`ApplyOutcome`。
- final GH-63 borrowed typed message render path。
- final GH-64 caller-owned Composer state与 exact-current projection/handlers。
- final GH-65 caller-owned MessageList、measurement config/closure与 observation。
- terminal size、optional status、typed overlay requests和 serialized shell events。

### 处理

1. expected shell revision与event target preflight。
2. 在轻量 candidate中执行 Composer/List mutation、完整 measurement与checked partition。
3. 只从GH-65 visible slices经GH-63 closure生成transcript；组合Composer/status/overlay。
4. GH-60 checked layout和staged render成功后一次publish shell/list/composer/frame。
5. session写terminal；退出时逐项恢复并返回完整cleanup结果。

### 输出

`InteractionOutcome<FullscreenShellPayload>`、immutable
`FullscreenShellObservation`、一个structured frame或具体 `FullscreenShellError`。

### 持久化与外部调用

无 provider/network/tool/secret/storage。session仅调用terminal/runtime lifecycle。state、
anchor、draft与overlay由进程内caller持有；fresh restart不声明跨进程恢复。

## 备选方案

- 扩展 `fixed_bottom_layout` 加聊天状态：拒绝；通用布局helper不应拥有MessageList/focus/session。
- 用 item-count `virtual_scroll_view`：拒绝；可变高度消息会错误定位。
- transcript/composer/overlay各注册hook：拒绝；当前广播机制会double dispatch。
- shell复制GH-65 height index或调用TextFlow测消息：拒绝；造成两套identity/invalidation。
- `Option<Element>`表达render错误：拒绝；None含义不明确且会静默丢消息。
- layout失败显示旧frame并返回成功：拒绝；观察状态与屏幕会漂移。
- Drop-only cleanup：拒绝；无法把restoration failure报告给调用方。
- 将Inline/Fullscreen合并为mode flag：拒绝；native scrollback与owned frame生命周期不同。

## 风险

- Dependency drift：四项direct dependencies均未最终完成。以closed/final merged ancestry与
  source-drift reapproval阻断推测实现。
- Correctness：region arithmetic、list/composer candidate与frame可能不同步。以单transaction、
  checked rect和failure equality tests缓解。
- Interaction：broadcast hooks会double dispatch。shell只注册一个top-level adapter，内部用
  closed route table。
- Terminal：真实terminal cleanup有多步独立失败。聚合全部step并用PTY/fake backend验证。
- Performance：全conversation clone会随历史增长。只保留GH-65 O(1) handles/visible slices，
  exact operation-count test禁止线性key/resize路径。
- Security：paste/overlay/tool text可能含controls。committed input走GH-64，render走GH-58/
  Output边界，shell无raw ANSI/tool execution。

## 测试计划

- [ ] config/partition property覆盖0、minimum、max、`u16::MAX`与status absent/present。
- [ ] MessageList sequences覆盖Following/Paused、zero viewport、append/stream/prepend/
      expand/collapse/delete与continuous resize。
- [ ] focus/key/paste/mouse table枚举overlay capture×focus×event闭集并断言一个target。
- [ ] Text/Markdown/Code/Thinking/ToolResult单/多行和plain/ANSI/accessibility golden。
- [ ] failure injection覆盖measure、Composer、list、layout、projection、render callback、
      coordinate/revision overflow，前后state/frame相等。
- [ ] Linux/macOS PTY和portable fake backend覆盖normal/cancel/error/panic、suspend/resume与
      raw/cursor/alternate/mouse/focus/paste restoration；unsupported平台保留fake contract，
      不伪称真实terminal verified。
- [ ] exact mapping、coverage producer/validator、fmt/check/clippy/all-target tests/example、
      fresh CI、独立review、reviewThreads和SpecRail PR gate。

## 回滚方案

GH-67为新增模块/exports/example迁移，无数据migration。未merge时关闭implementation PR；
已merge时普通revert全部planned paths，并先回滚依赖GH-67的GH-68 work。不得force push，
不得保留导出但silent-disable router/session，也不得恢复example私有item scroll/cleanup作为
“兼容fallback”。失败evidence与dependency记录保留，issue保持open。
