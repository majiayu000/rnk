# Tech Spec：grapheme-safe 多行 ChatComposer

## Linked Issue

GH-64: https://github.com/majiayu000/rnk/issues/64

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":64,"complete":true,"paths":["specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md","src/components/chat/mod.rs","src/components/chat/composer.rs","src/components/chat/composer/state.rs","src/components/chat/composer/keymap.rs","src/components/chat/composer/projection.rs","src/components/chat/composer/tests.rs","src/components/textarea/state.rs","src/components/textarea/state/grapheme.rs","src/components/textarea/state/tests.rs","src/components/textarea/component.rs","src/components/textarea/mod.rs","src/renderer/runtime.rs","src/components/mod.rs","src/prelude.rs","examples/claude_input_box.rs","docs/CORE_COMPONENT_CONTRACTS.md","tests/chat_composer_root_cause.rs","tests/chat_composer_interactions.rs","tests/chat_composer_flow.rs","tests/chat_composer_public_docs.rs","tests/textarea_unicode_compat.rs","tests/prelude_surfaces.rs"],"spec_refs":["specs/GH64/product.md","specs/GH64/tech.md","specs/GH64/tasks.md","specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md","specs/GH60/product.md","specs/GH60/tech.md","specs/GH60/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-64 的 TextArea Unicode correctness、composer state/input/projection、
runtime paste routing、一个 example 迁移和对应 public surface。GH-58 继续唯一拥有 TextFlow、
source map、Unicode width 与 renderer flow error；GH-60 继续拥有 layout transaction、
required-layout 与 checked render error。GH-64 不复制两者算法，也不把其 typed causes 压成
字符串。

## Codebase Context

以下锚点均在 stacked base `spec/GH60-transactional-patching`
`f67f973ed6903edb0cb76b5cb45c977ce92be851` 上通过 Read/grep 核实。该 base 的生产代码仍
未实现 GH-58/GH-60；implementation 必须在它们的真实 merged head 上重新定位。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Shared interaction contract | `src/components/interaction.rs:7`, `src/components/interaction.rs:37` | `InteractionMode` 只有 Enabled/Disabled/ReadOnly；`InteractionOutcome<T>` 已表达 Changed/Submitted/Cancelled | composer 必须复用真实类型，submitting 不能靠扩展既有 exhaustive enum |
| TextArea state shape | `src/components/textarea/state.rs:7`, `src/components/textarea/state.rs:20`, `src/components/textarea/state.rs:48` | public `Position`/`Selection` 可按字段构造；`TextAreaState` 字段私有 | 可在 state 内加 anchor/revision，但不能给 Position/Selection 增 required field |
| Source normalization/count | `src/components/textarea/state.rs:133`, `src/components/textarea/state.rs:164` | CRLF/CR 已转 LF，但 `lines()` 丢 trailing empty；`char_count` 按 scalar | composer 要保留 trailing caret row，同时锁定 legacy count 语义 |
| Scalar cursor/edit | `src/components/textarea/state.rs:208`, `src/components/textarea/state.rs:339`, `src/components/textarea/state.rs:376`, `src/components/textarea/state.rs:450`, `src/components/textarea/state.rs:477`, `src/components/textarea/state.rs:797` | cursor、insert、delete 与 byte conversion 全按 `char` ordinal | combining/ZWJ/flags 可被拆；需要 grapheme boundary helpers |
| Selection | `src/components/textarea/state.rs:575`, `src/components/textarea/state.rs:587`, `src/components/textarea/state.rs:610`, `src/components/textarea/state.rs:651` | selection 只保存 normalized range，反向扩展会丢 anchor 方向；range 按 char 转 bytes | 增 private anchor/focus，所有操作改用 grapheme range |
| TextArea rendering | `src/components/textarea/component.rs:229`, `src/components/textarea/component.rs:291`, `src/components/textarea/component.rs:376` | fixed logical-line height；cursor 用 `Vec<char>` 切分；selection style 未消费 | compatibility view 需 grapheme-safe；composer 另以 GH-58 projection 处理 visual rows |
| TextArea keymap | `src/components/textarea/keymap.rs:7`, `src/components/textarea/keymap.rs:68`, `src/components/textarea/keymap.rs:181`, `src/components/textarea/keymap.rs:219` | public field-addressable keymap/action；Enter 是 newline；匹配依赖遍历顺序 | 新建 private-field composer keymap，不能扩展既有 public enum/struct |
| TextArea handler | `src/components/textarea/component.rs:422`, `src/components/textarea/component.rs:448`, `src/components/textarea/component.rs:482` | handler 只在 `input.len()==1` 时插入，非 ASCII UTF-8 和 multi-scalar 被忽略 | 新 checked ingress 必须按完整 `&str` 处理 |
| Paste hook | `src/hooks/paste.rs:79`, `src/hooks/paste.rs:135`, `src/hooks/paste.rs:146` | typed `PasteEvent`、dispatch 与 hook 已存在 | 直接复用；不创建 alias 或第二个 paste bus |
| Runtime event routing | `src/renderer/runtime.rs:122`, `src/renderer/runtime.rs:165`, `src/renderer/runtime.rs:170` | Key/Mouse/Resize 被处理，`Event::Paste` 落入 wildcard 丢弃 | 必须 exactly-once dispatch paste，不触发 key handler |
| Harness | `src/testing/harness.rs:101`, `src/testing/harness.rs:121`, `src/testing/harness.rs:156`, `src/testing/harness.rs:164` | 已能 dispatch raw multi-text、paste 与 resize | 可写确定 integration tests，不需新测试框架 |
| Duplicate example state | `examples/claude_input_box.rs:34`, `examples/claude_input_box.rs:40`, `examples/claude_input_box.rs:132`, `examples/claude_input_box.rs:169` | example 私有 chars/cursor/handler/wrap | GH-64 手工迁移此一个 example，删除私有 editor state |
| Direct ANSI example | `examples/glm_chat/prompt_box.rs:52`, `examples/glm_chat/prompt_box.rs:64`, `examples/glm_chat/prompt_box.rs:80`, `examples/glm_chat/prompt_box.rs:103` | GLM prompt 自行裁剪并直接移动 ANSI cursor | 作为 remaining debt 留给后续 hardening，本 issue 不同时迁移 |
| Export surfaces | `src/components/mod.rs:12`, `src/components/mod.rs:57`, `src/components/mod.rs:58`, `src/prelude.rs:75`, `src/prelude.rs:86` | 没有 `components::chat`；textarea/interaction 已从 components 与 prelude 导出 | 增 chat module 与明确 exports，不改既有路径 |
| File-size guard | `src/components/textarea/state.rs:1` | 当前文件 936 行，已超过 800 hard ceiling | 任何修改前先机械拆 tests/grapheme helpers，production state 必须降到 800 行内 |
| GH-58 dependency | `specs/GH58/product.md:49`, `specs/GH58/product.md:84`, `specs/GH58/product.md:98`, `specs/GH58/product.md:126`, `specs/GH58/product.md:200` | spec 定义唯一 TextFlow、source map、resize、typed error，并明确 GH-64 消费、不得复制 | composer projection 必须建立在已合入的真实 API 上 |
| GH-60 dependency | `specs/GH60/product.md:93`, `specs/GH60/product.md:97`, `specs/GH60/product.md:101`, `specs/GH60/product.md:126`, `specs/GH60/product.md:131` | spec 定义 distinct error、missing-layout、frame atomicity、exact-head evidence 与 merged ancestry | composer render failure 不得用旧/default frame伪装成功 |

## 设计方案

### 1. Gate、stack 与依赖边界

GH-64 spec 可以 stacked 在 accepted GH-60 spec head 上；implementation 不可以。实现开始前
coordinator 必须：

GH-64 与 GH-62/GH-63 无实现依赖，不引入 conversation types。GH-66/GH-67 只能在 GH-64
implementation merged 后消费 composer。

### 2. TextArea grapheme core 与兼容边界

```text
grapheme_count(line)
grapheme_to_byte(line, ordinal) -> Result<byte, TextAreaEditError>
byte_to_grapheme(line, byte) -> Result<ordinal, TextAreaEditError>
grapheme_range(line, start, end) -> Result<Range<usize>, TextAreaEditError>
```

- `Position.col` 定义为当前 logical line 的 grapheme ordinal；row/col 字段形状不变。
- `set_content` 用 exact LF split 保留 trailing empty logical line；CRLF/CR 仍规范为 LF。
- state 增 private `selection_anchor` / `selection_focus`，公开 `Selection` 继续返回 normalized
  range。
- cursor move/delete/word/selected_text/selection replacement 全部经 checked grapheme helper；
  word boundary 仍以现有 whitespace 分类为兼容语义，但绝不在 grapheme 内切割。
- `char_count()`、`max_length` 与 `char_limit` 保持既有 Unicode scalar 计数语义；另提供
  checked batch mutation 在 commit 前对规范化后完整 candidate 验证 limits。composer 只用
  checked API，永不调用会逐 scalar partial insert 的 legacy path。
- 现有 `insert_char` / `insert_string` / void cursor APIs 保持签名和正常 ASCII 结果；内部
  invariant error 通过带 cause 的 fail-loud wrapper 暴露。既有 limit 行为由 compatibility
  fixture 锁定，不把 legacy wrapper 宣称为新原子 composer ingress。
- `TextAreaKeyMap`、`TextAreaAction`、`InteractionMode` 不增字段/variant；
  `Position`/`Selection` 不增 required field，也不改 `#[non_exhaustive]`。

### 3. Closed composer state、input 与 error types

在 `components::chat::composer` 引入新类型。可扩展的 public behavior/action enums
（例如 `ChatComposerAction`）从首次发布即 `#[non_exhaustive]`；closed error enums
（`ChatComposerError` 及其 config/edit/submission/projection 子错误）明确不使用
`#[non_exhaustive]`、catch-all 或 string variant，使 downstream 可穷举稳定 category。
config/keymap/state 字段保持 private 并通过构造器读取，避免后续 required field 破坏：

```text
ComposerRevision(u64)
SubmissionToken { revision, nonce }
ComposerPayload { text, revision, pending_submission }

ChatComposerState {
  textarea: TextAreaState,
  revision: ComposerRevision,
  preferred_cell_column: Option<usize>,
  pending_submission: Option<PendingSubmission>,
  successful_submission_tombstones: SuccessfulSubmissionTombstones<16>
}

ChatComposerError =
  Config(ComposerConfigError) |
  Edit(TextAreaEditError) |
  StaleProjection(ComposerRevisionMismatch) |
  Submission(SubmissionTokenError) |
  Flow(TextFlowError) |
  Projection(ComposerProjectionError)
```

每层实现 `Display`、`Error` 和适用的 `source()`；不接受 public `Any`、任意 closure injector
或 stringly error。user-derived control 不直接进入 error display，error 只输出 safe range /
分类。compile fixture 必须对 `ChatComposerError` 与各子错误做无 wildcard 的 exhaustive
match；另对 `ChatComposerAction` 使用 wildcard，锁定两种 enum 策略没有混用。

统一 checked ingress 的顺序固定为：

```text
raw committed UTF-8 / PasteEvent.content
  -> classify shortcut vs text
  -> normalize CRLF/CR to LF; structure LF/tab
  -> reject ESC/disallowed C0/DEL/C1 with original range
  -> clone/stage TextArea candidate
  -> delete selection on candidate
  -> insert complete batch on grapheme boundaries
  -> validate scalar/line limits and state invariants
  -> commit candidate + exactly one revision increment
  -> InteractionOutcome::Changed(ComposerPayload)
```

空 paste 是 `Ignored`；合法输入但 state 未改变是 `Handled`。任何 Err drop candidate，原 state
逐字段不变。一次 multi-scalar committed input 与一次 paste 都只增加一次 revision。

`revision` 是所有 projection-observable state 的单一 checked generation。每个 handler 先在
candidate 上做 `checked_add(1)`，再与 mutation 一次 commit；overflow 返回
`ChatComposerError::Edit(TextAreaEditError::RevisionOverflow)`（或 merged API 中等价的 closed
typed cause），原 state 逐字段不变：

| Transition | State change | Revision |
| --- | --- | --- |
| committed text、paste、newline、delete、clear | content/cursor/selection/可能 preferred column | 恰好 +1 |
| Left/Right、Home/End、word movement、selection-only movement | cursor/anchor/focus/可能 preferred column | 实际改变时恰好 +1 |
| visual Up/Down 及 Shift variants | cursor/selection/preferred column | fresh projection 且实际改变时恰好 +1 |
| valid submit | `pending_submission` | 恰好 +1 |
| first success acknowledgement | content/cursor/selection/pending/tombstone | 一个原子 transition，恰好 +1 |
| failure acknowledgement | pending | 恰好 +1 |
| repeated success tombstone hit | 无 | +0，返回成功 no-op |
| blank submit、Ignored、Handled-no-change、Cancelled、stale/error | 无 | +0 |
| projection build/render | 无 state mutation | +0 |

一次 action 即使同时改变多个字段也只递增一次。revision overflow fixture必须分别覆盖
source-only edit、geometry movement、submit与 acknowledgement，并证明 commit 前失败。

### 4. Keymap、modes 与 submission acknowledgement

`ChatComposerKeyMap` 使用 private vectors + builder，而不是在 `TextAreaKeyMap` 增字段。
构造完成时将每个 normalized `KeyBinding` 映射到 closed `ChatComposerAction`，发现同 binding
多 action 立即返回 `ComposerConfigError::ConflictingBinding`。默认：

- Enter -> Submit
- Shift+Enter -> InsertNewline
- Alt+Enter -> InsertNewlineFallback
- Escape -> Cancel
- Ctrl+U -> ClearDraft
- arrows/Home/End/word/delete/selection -> 对应 composer action

action priority 只能在 key modifier 明确不同后发生；不得靠 vector 先后解决真正 collision。
`ChatComposerAction` 自首次 public 即 `#[non_exhaustive]`。

纯 handler 的计划签名：

```text
handle_chat_composer_input(
  state,
  input,
  key,
  keymap,
  mode,
  current_projection: Option<&ComposerProjection>
) -> Result<InteractionOutcome<ComposerPayload>, ChatComposerError>

handle_chat_composer_paste(
  state,
  PasteEvent,
  mode
) -> Result<InteractionOutcome<ComposerPayload>, ChatComposerError>
```

projection 带 state revision。只有会读取 visual row/cell mapping 的 Up/Down 及对应 Shift
selection variants 是 geometry actions：它们要求 `Some(projection)` 且 revision exact-current，
缺失或 stale 时 typed 拒绝、不猜测旧 cell。printable input、paste、Left/Right、logical
Home/End、word movement、delete、clear、submit/cancel/ack 等 source-only actions 不读取也不
校验 projection，因此同一 render 间隔内的连续按键仍逐个 commit。每次 source mutation 后旧
projection只对后续 geometry action失效，不阻止下一次 source-only action。

有效 submit 先 stage `PendingSubmission { token, exact_text }`，保留 textarea 内容，再返回
`Submitted(payload-with-token)`。state 方法分别处理：

- `acknowledge_success(token)`：若匹配 current pending，先 checked revision，再原子清空并把
  token 加入 private `SuccessfulSubmissionTombstones<16>`。ring 只保存不可逆 opaque token，
  去重后按 FIFO 淘汰；若 token 已在 ring 内，返回相同成功 no-op，不改变 revision、draft、
  新 pending 或 ring 顺序。
- `acknowledge_failure(token)`：保留 draft，清 pending，revision 只按文档化规则前进一次。
- 从未见过或已被第 17 个 success 淘汰的 token：typed stale/unknown Err 且 state 不变。

ack fixture必须覆盖 first success、immediate repeat、写入新 draft 后 repeat、新 pending 存在时
repeat、unknown token 与 FIFO eviction；旧 token 命中 tombstone时绝不能清除新 draft/pending。

Mode precedence：Disabled 首先 `Ignored`；其余 mode 的 Escape 返回 `Cancelled`。ReadOnly 与
submitting 的 value-changing action 返回 `Ignored`，navigation/selection 可继续；submitting
由 state 的 pending token 表达，不扩展 `InteractionMode`。blank submit 返回 `Handled`。

### 5. Selection 与 visual cursor

selection mutation 在 TextArea state 中使用 private anchor/focus；对外继续返回 normalized
range。`ComposerProjection` 使用 GH-58 logical map 将 source grapheme ranges 投影为 styled
cells：

- Left/Right 只改相邻 source grapheme；
- Up/Down 查当前 visible/logical row 与 preferred cell column，再反查该 row 最近合法 source
  boundary；
- Shift variants复用同一 movement target并保留 anchor；
- selection background、cursor cell 与 wide-cell continuation 永不拆 grapheme；
- selection replacement 在 staged candidate 上先删除完整 range，再插整批输入。

preferred cell column 是 private ephemeral navigation state；content revision或显式横向移动后
按确定规则重置。它不增加到 public `Position`。

### 6. 单一 TextFlow projection、auto-grow 与 trailing caret row

`try_project_chat_composer(state, layout_inputs)` 是本 issue 唯一 flow 入口。它从 composer
source、structured styles、checked content width、wrap/tab/Unicode policy 构造一次 GH-58
`TextFlowInput` 并持有返回的 immutable `TextFlow`：

```text
state content + source grapheme cursor/selection
  -> GH-58 TextFlow::try_build
  -> immutable rows + source/cell map
  -> cursor/selection projection
  -> visible row window + exact height
  -> ChatComposerProjection { state_revision, flow, rows, cursor, selection }
  -> ChatComposer view consumes same projection
```

不得调用 `count_wrapped_lines_by_width`、example `wrap_text`、独立 `unicode-width` loop 或按
logical line count 猜高度。`ChatComposer` 消费 projection 已定位的 rows；它不从 source 再
计算另一份 wrapping。若 merged GH-58 API 无法让 component 与 handler共享同一 immutable
flow/source map，停止实现并先更新 GH-58/GH-64 specs，禁止旁路 sidecar 或 Element required
field。

content width 从 outer width 减去 border、padding、prompt/hint 的 GH-58 cell width，全部用
checked arithmetic。`max_visible_lines` 使用 `NonZeroUsize` 或等价 validated config。
height=`max(1, min(flow.row_count, max_visible_lines))`；visible window包含 cursor row。

GH-58 默认 trailing hard break 不生成最终空 row，composer 对 source end insertion position
增加独立 `SyntheticCaretRow` disposition；它只有 end position，无 source byte range，不写入
GH-58 cache。width=0 时保留高度1并标记 cursor clipped。每次 width/prompt/padding/state
revision变化都建立新 frame projection；不跨 viewport缓存 visibility。

GH-58 `TextFlowError` 经 `ChatComposerError::Flow` 保留 source。element 进入 layout 后的
required-layout/renderer failure继续经 GH-60 `CheckedRenderError`，composer不捕获并显示旧
projection。

### 7. Runtime `Event::Paste`

`EventLoop::handle_event` 增显式分支：

```text
Event::Paste(content)
  -> dispatch_paste(&content)
  -> record_activity()
  -> request_render()
```

该分支不调用 `dispatch_key_event`。无 handler时 dispatch安全 no-op但仍记录activity/render。
production event path和测试 harness必须得到相同观察结果。GH-64 不自动执行
Enable/DisableBracketedPaste；shell或应用继续负责 terminal mode lifecycle。

### 8. Export、docs 与 example migration

- T3 在任何 `components::chat::composer::*` exact test 前先创建
  `src/components/chat/mod.rs` 并在 `src/components/mod.rs` 声明 `pub mod chat`；同一 T3
  commit 导出 composer concrete types，并在 `composer/projection.rs` 创建至少包含
  `state_revision` 与 accessor 的最小 typed `ComposerProjection` skeleton，使 T3 handler
  signature与lib tests可编译/discover。T4 接管该 projection file 后填充唯一 GH-58 flow
  payload；禁止 T3 用 `Any`/trait-object/placeholder alias绕过。module两文件不延后到
  adoption task，也不再被 T5 接管。
- components root 与 T5 的 prelude提供同一 concrete types，不提供 alias。
- `src/components/chat/mod.rs` 对新增 public surface启用 scoped `#![forbid(missing_docs)]`。docs gate扫描 `chat/mod.rs`、`composer.rs` 与 `composer/*.rs`，要求 root guard恰好一次，并在全部文件中拒绝 `allow(missing_docs)`、`expect(missing_docs)` 与 `doc(hidden)`；child不得降低或隐藏文档义务。
- 固定 public API inventory 为 `ChatComposerState`、`ChatComposerKeyMap`、`ChatComposerAction`、`ComposerRevision`、`SubmissionToken`、`ComposerPayload`、`ComposerProjection`、`ChatComposerError`、`ComposerConfigError`、`ComposerRevisionMismatch`、`SubmissionTokenError`、`ComposerProjectionError`、`handle_chat_composer_input`、`handle_chat_composer_paste` 与 `ChatComposer`。
  T5 在 `tests/chat_composer_public_docs.rs` 为每个 symbol 创建固定 `public_<snake_name>_executes` exact test；测试体必须直接 type-check/构造/调用目标并断言至少一个真实 observable，不得委托 helper。`ChatComposer` 另有一个 runnable、nonignored doctest。
- `docs/CORE_COMPONENT_CONTRACTS.md` 增 controlled state、keymap、mode、submit acknowledgement、
  bounded success tombstone、source-only/geometry projection freshness、revision transition、
  paste lifecycle、closed-error/exhaustive-match策略和 IME limitation。
- 手工迁移 `examples/claude_input_box.rs`：以 public state/keymap/handlers/view 替换 `InlineInputState` 与 cursor/wrap helpers，保留 native scrollback。production `exercise_composer_contract` 由 `main` 与 exact test共用，执行 multi-scalar input、multiline paste、projection/render 与 submit并返回 probe；`tests::claude_example_uses_public_composer_contract` 逐字段断言真实 state text/revision、projection rows/cursor 与 exact submit payload。
- 迁移的证据是 `tests::claude_example_uses_public_composer_contract` 这个语义测试本身：它断言 example 真实使用 public composer API 并产生正确 state/projection/payload。不引入源码 token 扫描器——扫描器无法区分真实调用与字面量，语义测试可以。`examples/glm_chat.rs` 与 `examples/glm_chat/` 相对 implementation merge-base 无 diff。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `components::chat::composer`, exports/docs inventory | `cargo test --test prelude_surfaces --locked chat_composer_surface_uses_shared_interaction_types -- --exact`; `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked` |
| B-002 | TextArea grapheme helpers + checked edits | `cargo test --workspace --lib --locked components::textarea::state::tests::grapheme_cursor_delete_selection_contract -- --exact`; `cargo test --test textarea_unicode_compat --locked randomized_grapheme_edit_selection_replace_matches_utf8_oracle -- --exact` |
| B-003 | grapheme byte/ordinal + GH-58 map | `cargo test --test textarea_unicode_compat --locked grapheme_positions_never_split_utf8_or_wide_cells -- --exact` |
| B-004 | checked committed-text ingress | `cargo test --workspace --lib --locked components::chat::composer::tests::multi_scalar_and_crlf_input_is_atomic -- --exact` |
| B-005 | paste handler | `cargo test --test chat_composer_interactions --locked multiline_paste_uses_atomic_text_ingress -- --exact` |
| B-006 | control rejection | `cargo test --workspace --lib --locked components::chat::composer::tests::control_payload_rejects_without_mutation -- --exact` |
| B-007 | validated composer keymap | `cargo test --workspace --lib --locked components::chat::composer::tests::submit_newline_fallback_and_conflict_contract -- --exact` |
| B-008 | blank/valid submit payload | `cargo test --test chat_composer_interactions --locked blank_and_valid_submit_contract -- --exact` |
| B-009 | bounded submission tombstone + ack/reject | `cargo test --workspace --lib --locked components::chat::composer::tests::submission_ack_preserves_or_clears_exact_draft -- --exact`; `cargo test --workspace --lib --locked components::chat::composer::tests::submission_success_tombstone_is_bounded_and_never_clears_new_state -- --exact` |
| B-010 | mode matrix | `cargo test --test chat_composer_interactions --locked enabled_readonly_disabled_submitting_matrix -- --exact` |
| B-011 | Escape | `cargo test --workspace --lib --locked components::chat::composer::tests::cancel_never_clears_draft -- --exact` |
| B-012 | selection anchor/replacement | `cargo test --test textarea_unicode_compat --locked reverse_and_cross_line_selection_is_grapheme_safe -- --exact`; `cargo test --test textarea_unicode_compat --locked randomized_grapheme_edit_selection_replace_matches_utf8_oracle -- --exact` |
| B-013 | visual Up/Down mapping | `cargo test --test chat_composer_flow --locked wrapped_vertical_navigation_preserves_cell_column -- --exact` |
| B-014 | auto-grow/window | `cargo test --test chat_composer_flow --locked auto_grow_caps_and_keeps_cursor_visible -- --exact` |
| B-015 | resize reflow | `cargo test --test chat_composer_flow --locked resize_reflows_same_source_cursor_and_selection -- --exact` |
| B-016 | empty/trailing/zero width | `cargo test --test chat_composer_flow --locked empty_trailing_newline_and_zero_width_contract -- --exact` |
| B-017 | closed typed errors/source/atomicity | `cargo test --test chat_composer_interactions --locked typed_failures_preserve_state_and_sources -- --exact`; `cargo test --test chat_composer_interactions --locked closed_error_family_is_exhaustively_matchable -- --exact` |
| B-018 | TextArea public compatibility | `cargo test --test textarea_unicode_compat --locked public_textarea_surface_and_ascii_behavior_compile -- --exact` |
| B-019 | committed IME-like input | `cargo test --test chat_composer_interactions --locked committed_ime_like_sequences_are_grapheme_safe -- --exact` |
| B-020 | runtime paste branch | `cargo test --workspace --lib --locked renderer::runtime::tests::paste_event_dispatches_once_without_key_dispatch -- --exact` |
| B-021 | revision/projection/repetition | `cargo test --workspace --lib --locked components::chat::composer::tests::source_only_key_bursts_do_not_require_fresh_projection -- --exact`; `cargo test --workspace --lib --locked components::chat::composer::tests::source_state_transitions_increment_revision_once -- --exact`; `cargo test --workspace --lib --locked components::chat::composer::tests::source_revision_overflow_is_atomic -- --exact`; `cargo test --workspace --lib --locked components::chat::composer::tests::stale_projection_and_repeated_events_are_deterministic -- --exact`; `cargo test --workspace --lib --locked components::chat::composer::tests::projection_observable_transitions_increment_revision_once -- --exact`; `cargo test --workspace --lib --locked components::chat::composer::tests::visual_geometry_revision_overflow_is_atomic -- --exact` |
| B-022 | Claude-only migrated example | `cargo check --example claude_input_box --all-features --locked`；`cargo test --test prelude_surfaces --locked claude_example_uses_only_public_composer -- --exact` |
| B-023 | exact-head quality | root-cause/property exact tests；`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`；`cargo test --test prelude_surfaces --locked claude_example_uses_only_public_composer -- --exact`；full Rust/docs/examples、CI、independent review |
| B-024 | dependency/duplicate gate | `git merge-base --is-ancestor "$GH58_MERGED_SHA" HEAD`; `git merge-base --is-ancestor "$GH60_MERGED_SHA" HEAD`; fresh SpecRail duplicate/route evidence |

## 数据流

### 输入

- key path 输入 `&str`、typed `Key`、validated `ChatComposerKeyMap`、`InteractionMode`，
  以及仅供 visual geometry action 使用的 optional immutable projection；source-only action
  不读取 projection。
- paste path 输入既有 `PasteEvent` 与 `InteractionMode`。
- projection 输入 controlled state revision、outer/content width、prompt/padding/border、
  `max_visible_lines` 与 GH-58 flow options。
- acknowledgement 输入只包含 opaque submission token 与 success/failure选择。

### 输出

- handler 返回 `Result<InteractionOutcome<ComposerPayload>, ChatComposerError>`。
- projection 返回 immutable rows、source/cell map、cursor、selection、visible range与height。
- view输出普通 `Element` tree；ANSI只能由既有structured renderer产生。
- runtime paste只产生既有paste handler callback、activity记录与render request。

### 持久化与外部调用

GH-64 不写磁盘、网络、conversation或系统剪贴板。state由调用方持有；submission token仅在
state生命周期内去重，不能宣称跨进程 exactly-once。terminal mode enable/disable不是composer
副作用。

## 备选方案

- **复制 Claude example 的 `Vec<char>` editor**：拒绝。继续拆 combining/ZWJ，且复制根因。
- **只把 `input.len()==1` 改成 `chars().count()==1`**：拒绝。仍不支持multi-scalar commit、
  paste原子性、selection、typed limits或IME-like序列。
- **给 TextAreaKeyMap / InteractionMode 加 submit/submitting字段或variant**：拒绝。两者是
  public exhaustive/field-addressable surface，会破坏下游literal或match。
- **Composer 自己调用 `unicode-width` 再写 wrap loop**：拒绝。违反GH-58唯一TextFlow合同。
- **提交时立即clear并让调用方失败后重建**：拒绝。网络/adapter失败会丢草稿，也无法避免
  stale acknowledgement清掉新输入。
- **把 native IME preedit当成普通 key输入**：拒绝。crossterm 0.28没有可证明的composition
  event contract；只声明 committed text。
- **自动在composer构造/Drop启停 bracketed paste**：拒绝。组件render可能重复构造，无法
  独立拥有terminal lifecycle；留给shell/app guard。

## 风险

- Security：paste可携带ESC/control payload。通过ingress原range typed拒绝、GH-58 Output
  trust boundary与无直接ANSI三层防线降低风险；相关代码需人工review。
- Compatibility：`Position.col` 从scalar修正为grapheme ordinal会改变非ASCII观察值。
  保留字段/签名、ASCII行为、legacy scalar limits，并提供crate外fixture与迁移说明。
- Correctness：GH-58默认trailing break不产生final empty row。composer synthetic caret row
  必须只映射end insertion position，不能污染source map。
- Correctness：state revision后旧projection可能把visual action指向错误source。只有读取
  visual geometry 的 action 校验projection revision并typed fail closed；source-only key burst
  不依赖render cadence，但仍让每次实际 state mutation递增一次revision。
- Performance：每个frame重建projection可能增加Unicode segmentation成本。先保证唯一语义；
  只复用GH-58合法logical cache，不缓存viewport visibility，后续基准才能决定优化。
- Maintenance：Textarea/Composer/renderer/export/example跨模块且TextArea state超长。严格串行
  ownership与先拆文件，禁止两个writer共享同一文件。
- Terminal compatibility：Shift+Enter与emoji width随terminal能力变化。提供Alt+Enter fallback，
  只使用GH-58 width policy，并明确未验证preedit。
- Dependency drift：本spec基于未实现的GH-58/GH-60 contracts。implementation若真实API不支持
  shared projection/error chain，必须更新spec而非发明sidecar或fallback。

## 测试计划

- [ ] Unit：grapheme boundary、selection anchor、multi-text/CRLF/control ingress、keymap
      conflict、submit/ack/mode、stale projection、runtime paste routing。
- [ ] Property：T2 在 `tests/textarea_unicode_compat.rs` 实现 exact test
      `randomized_grapheme_edit_selection_replace_matches_utf8_oracle`。fixture 使用固定 32-byte
      ChaCha seed、256 cases、禁用 failure persistence，以独立 UTF-8/grapheme vector oracle
      比较随机 insert/delete/select/replace；每步断言 cursor/selection byte range 为 boundary，
      失败candidate与原state逐字段相等。必须以 `-- --exact` 实际运行，
      不能只跑 substring filter、`--list` 或默认随机 seed。
- [ ] Integration：paste、committed IME-like、auto-grow、trailing caret、width=0/1、continuous
      resize、shared TextFlow source/cell mapping、crate外compat和typed source chain。
- [ ] Example：`claude_input_box`只组合public composer，无local editor/wrap/cursor state。
- [ ] Docs：scoped `forbid(missing_docs)`、
      `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`、exact ChatComposer doctest，以及 `cargo test --test prelude_surfaces --locked claude_example_uses_only_public_composer -- --exact`。
- [ ] Full：fmt、check、clippy、workspace all-target tests、all examples。

## 回滚方案

- GH-64 implementation未merge时直接关闭implementation PR，保留issue/spec与失败evidence；
  不修改已合入GH-58/GH-60。
- 已merge但需回滚时，用普通revert撤销composer、runtime paste branch、TextArea Unicode改动、
  exports、docs与example migration；禁止force push。
- 回滚后恢复原example前，先证明不把新composer state序列化或外部数据误当成legacy
  `Vec<char>`；本issue不含持久化migration。
- 若只能回滚composer view而保留TextArea grapheme correctness，必须以独立PR证明既有public
  TextArea tests/compat仍绿，不能恢复split-grapheme bug或silent multi-text loss。
- 回滚后GH-64保持open/`ready_to_implement`，保存exact head、dependency SHAs、coverage、CI、
  typed error与review evidence；GH-66/GH-67继续blocked。
