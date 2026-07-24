# Task Plan：类型化消息与 AI 内容块视图

## Linked Issue

GH-63: https://github.com/majiayu000/rnk/issues/63

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Required upstream: GH-62 implementation
- Final narrow-width gate: GH-58 implementation

## 实现任务

- [ ] `SP63-T1` 建立 typed custom-renderer contract、view/cache module skeleton 与 compile-contract tests。Owner: `chat-view-contract-worker` | Done when: 下列 typed contract、skeleton 与 T1-owned tests 全部完成 | Verify: T1 exact compile-contract tests、check 和 docs source preflight 通过。
  `src/components/chat/view/{mod,message,block,custom,cache}.rs` 全部存在并在 inherited
  `forbid(missing_docs)` 下编译；`custom.rs` 定义 closed `ChatBlockRef`、typed
  `ChatRenderContext`、`ChatRenderOverride::{UseDefault,Element}`、trait 与 closure impl；
  `ChatBlockRef` 穷尽借用 GH-62 十一种 payload，context 显式携带 MessageId、
  MessageRevision、BlockId 与 observational position；`message.rs`/`block.rs`/`cache.rs`
  只含 private compile skeleton，不承诺 dispatch、key、cache 或 wrapper 行为；创建
  `tests/chat_message_views.rs`，只加入可在 `custom.rs` 独立通过的 tests；无 `Any`、
  whole-payload clone、untyped map、provider JSON、string registry、Cargo dependency 或
  普通 chat rustdoc fence | Verify:
  `verify_gh63_exact typed_trait_and_closure_override_or_explicitly_default`;
  `verify_gh63_exact typed_renderer_contract_contains_no_dynamic_erasure`。
  - Dependencies: implementation gate；GH-62 implementation 已完成。
  - Covers: B-014, B-015, B-024, B-025, B-026。
  - File ownership: 本任务独占五个新 source skeleton 与 integration test；完成后
    `custom.rs` 冻结，`block.rs`/`mod.rs`/test 串行 handoff 给 T2；
    `message.rs`/`cache.rs` skeleton 保持只读至 T3。

- [ ] `SP63-T2` 在 retargeted GH-58 TextFlow 上实现全部 typed block views、nested failure causes 与受控 preview。Owner: `chat-block-view-worker` | Done when: 下列 block/status/reason/exact-source-preview 合同全部完成 | Verify: upstream gate 与下列 block exact tests 全部通过。
  先运行 `verify_gh63_upstream_gate origin/main`；独占 `view/block.rs`、接管
  `view/mod.rs` block exports 与 integration test。Text/Markdown/Code/Thinking/
  ToolCall/ToolResult/Error/Diff/Quote/Link/TerminalAttachmentSummary/StreamingIndicator
  只经 GH-62 borrowed accessors 读取 typed values；Error 保留 message/optional source，
  ToolArgument 递归穷尽 closed TypedValue，四种新增 payload 保留各自 optional/required
  fields 且 Link/attachment 保持 inert；
  Thinking/ToolResult 把 raw source 原样交给 GH-58 TextFlow logical row/source-range
  projection，禁止 `str::lines()`、reconstructed source 或 arbitrary byte slice；
  tests table-drive 两类 view × LF/CRLF/standalone CR/consecutive/trailing breaks，并断言
  terminator/source maps、true truncation 和 no-hidden-content marker；Thinking/ToolCall/
  ToolResult Failed 均显示原 typed cause；indicator 只由 explicit frame 驱动；不负责
  message-level wrapper/key/context delivery；不写 raw ANSI、不执行工具、不解析 provider JSON |
  Verify: `verify_gh63_exact text_view_preserves_empty_multiline_and_unicode_content`;
  `verify_gh63_exact markdown_view_uses_structured_component_without_fallback`;
  `verify_gh63_exact code_view_preserves_language_absence_and_multiline_content`;
  `verify_gh63_exact thinking_disclosure_is_controlled_identity_stable_and_exact`;
  `verify_gh63_exact tool_call_status_matrix_and_argument_order_are_typed`;
  `verify_gh63_exact tool_result_status_and_true_truncation_are_explicit`;
  `verify_gh63_exact error_block_never_degrades_to_normal_text`;
  `verify_gh63_exact error_content_message_and_source_are_projected`;
  `verify_gh63_exact typed_value_tool_arguments_render_without_json`;
  `verify_gh63_exact all_extended_block_variants_render_typed_payloads`;
  `verify_gh63_exact streaming_indicator_is_frame_controlled_and_deterministic`;
  `verify_gh63_exact preview_source_ranges_cover_every_hard_break_and_truncation_branch`;
  `verify_gh63_exact every_nested_failed_status_preserves_typed_reason`。
  - Dependencies: SP63-T1；GH-58/GH-62 merged dependency + retarget gate。
  - Covers: B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012,
    B-017, B-024, B-026, B-027。
  - File ownership: T2 不修改 `custom.rs`/`message.rs`；完成后冻结 `block.rs`，
    将 `mod.rs` 和 test 串行 handoff 给 T3。

- [ ] `SP63-T3` 实现 `ChatMessageView`、BlockId wrapper、typed metadata、revision/changefeed cache、resolved Theme snapshot 与三种 variants。Owner: `chat-message-view-worker` | Done when: 下列 message/dispatch/key/cache/theme/purity 合同全部完成 | Verify: 下列 message/cache exact tests 全部通过。
  独占 `view/message.rs`/`view/cache.rs`、接管 `view/mod.rs` 和 integration test；
  view 借用 immutable `ChatMessage`，按 source order 调用 T1/T2 contracts；
  Compact/Bordered/Bubble 只改 presentation；author/timestamp 只借用 GH-62
  ChatMessageMetadata accessors，None 不生成 child且无平行 metadata type；
  Pending/Streaming/Complete/Failed/Cancelled distinct，top-level/nested Failed 均显示 typed
  reason；message.rs 只以 MessageBlockEntry BlockId 负责 stable key 和 library-owned
  keyed/status wrapper，position/ThinkingId/ToolCallId 不参与 identity，并向 custom renderer
  交付当前 MessageRevision/BlockId/position 的真实 typed context；cache.rs 实现 caller-owned
  `ChatMessageViewCache`，只消费 `ApplyOutcome::affected_messages`，按 Present/Deleted 精确
  失效，覆盖 append/replace/edit/remove/delete/resend/exact replay/ledger eviction/restore；
  source Resend cache 保留，新 message 从 INITIAL=1/fresh IDs 独立渲染；任何 retired ID 不
  复活且不 global flush；`ChatMessageView::new` 恰捕获一次 owned global Theme
  snapshot，显式 `.theme(...)` 可替换，`into_element` 不再读取 ambient theme；dark/light
  golden 显式输入，`with_theme` 测试验证 capture 和 restore；相同完整输入输出确定 |
  Verify: `verify_gh63_exact message_view_is_pure_and_preserves_block_order`;
  `verify_gh63_exact roles_and_missing_metadata_are_explicit`;
  `verify_gh63_exact typed_metadata_is_borrowed_without_placeholders`;
  `verify_gh63_exact every_message_status_has_distinct_semantics`;
  `verify_gh63_exact message_revision_initial_and_checked_updates_are_observed`;
  `verify_gh63_exact every_message_and_nested_failed_status_preserves_typed_reason`;
  `verify_gh63_exact every_block_variant_dispatches_once_in_order`;
  `verify_gh63_exact custom_renderer_receives_typed_context_without_reordering`;
  `verify_gh63_exact keys_survive_content_status_and_disclosure_updates`;
  `verify_gh63_exact block_id_not_position_or_lifecycle_identity_keys_views`;
  `verify_gh63_exact edit_insert_reorder_preserve_or_retire_block_identity`;
  `verify_gh63_exact affected_messages_drive_exact_cache_invalidation`;
  `verify_gh63_exact delete_evicts_view_cache_and_preserves_tombstones`;
  `verify_gh63_exact resend_keeps_source_cache_and_starts_fresh_message_revision`;
  `verify_gh63_exact restore_rebuilds_cache_without_resurrecting_retired_ids`;
  `verify_gh63_exact variants_change_presentation_not_semantics`;
  `verify_gh63_exact theme_and_style_overrides_are_local_to_one_view`;
  `verify_gh63_exact theme_snapshot_is_captured_once_and_explicitly_deterministic`;
  `verify_gh63_exact theme_scope_restores_after_dark_and_light_snapshots`;
  `verify_gh63_exact identical_inputs_render_identically`;
  `verify_gh63_exact interrupted_retry_has_no_library_side_effect`;
  `verify_gh63_exact custom_renderer_panic_is_not_silently_swallowed`;
  `verify_gh63_exact independent_snapshots_do_not_share_view_state`。
  - Dependencies: SP63-T1、SP63-T2。
  - Covers: B-001, B-002, B-003, B-004, B-008, B-009, B-010, B-013, B-014,
    B-015, B-016, B-019, B-020, B-021, B-022, B-025, B-026, B-028, B-029,
    B-030, B-031。
  - File ownership: T3 不修改冻结的 `custom.rs`/`block.rs`；完成后冻结全部 view source，
    将 `mod.rs`/test 交给 T4。

- [ ] `SP63-T4` 接入 app-facing exports、compatibility/migration docs 和 checked-in plain/ANSI golden。Owner: `chat-view-surface-worker` | Done when: 下列 exports/docs/compatibility/golden 合同全部完成 | Verify: 下列 compatibility/golden/doc gates 全部通过。
  `src/components/chat/mod.rs` 只加入 view module/re-export，`src/components/mod.rs` /
  `src/prelude.rs` 增量导出新 view types且不遮蔽 legacy names；
  `docs/API_STABILITY.md` 记录 simple-string 与 typed-view 两条迁移路径和稳定级别；
  integration test 同时编译 legacy/new surface；加入 full role/status/eleven-block/variant matrix
  的 `tests/golden/chat_message_views.txt` 与 `.ansi.txt`，测试显式拒绝
  `UPDATE_GOLDEN`/missing fixture，且运行前后 fixture checksum 不变；不新增普通 chat rustdoc
  fence | Verify: `verify_gh63_exact legacy_and_typed_message_surfaces_coexist`;
  `verify_gh63_exact plain_and_ansi_golden_cover_full_matrix`;
  先定义 GH-62 `verify_chat_rustdoc_example` 再运行 `verify_gh63_docs_gate`；
  `cargo test --doc --workspace --all-features --locked`；
  测试前后分别运行
  `shasum -a 256 tests/golden/chat_message_views.txt tests/golden/chat_message_views.ansi.txt`
  并要求两份 checksum 完全相等。
  - Dependencies: SP63-T3。
  - Covers: B-002, B-003, B-004, B-013, B-018, B-023, B-027。
  - File ownership: T4 独占三处 export、API doc、两份 golden；只在 T3 handoff 后追加
    integration compatibility/golden tests。

- [ ] `SP63-T5` 完成 current-head dependency、docs、coverage、窄宽和 SpecRail handoff。Owner: `chat-view-verification-worker` | Done when: 下列 dependency/docs/coverage/full-head evidence 全部完成 | Verify: 所有 helpers、mapping exact tests 与本文件全套验证命令通过。
  fresh `verify_gh63_upstream_gate origin/main` 证明 GH-58/GH-62 completion；先实际运行
  `verify_gh63_exact_helper_self_test`，再运行 1/2/4/8/12/20/40 columns 的
  combining/ZWJ/CJK/tab/control/multiline fixtures，无 byte split、panic、raw terminal
  control、source-map loss 或 measure/render mismatch；Product-to-Test Mapping 全部 exact tests 实际
  1 passed/0 failed/0 ignored；plain/ANSI golden 未更新；coverage artifact 只包含全部
  `src/components/chat/view/*.rs`，line-rate >=80%，dispatch/status/override/truncation
  semantic matrices 100%，changefeed/cache matrix 100%；`verify_gh63_docs_gate` 和
  `verify_gh63_new_code_coverage`
  fail closed；fresh fmt/check/clippy/all-target tests/doc、planned-path audit、CI、
  independent review、reviewThreads、PR gate 全部绑定 exact implementation head |
  Verify: `verify_gh63_exact narrow_unicode_and_control_fixtures_use_textflow_safely`；
  `verify_gh63_exact preview_source_ranges_cover_every_hard_break_and_truncation_branch`；
  `verify_gh63_branch_matrices`；`verify_gh63_docs_gate`；
  `verify_gh63_new_code_coverage`；调用 mapping 全部 `verify_gh63_exact`；运行本文件“验证”全套命令。
  - Dependencies: SP63-T1 至 SP63-T4；GH-58 与 GH-62 implementation completion。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
    B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019,
    B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029,
    B-030, B-031。
  - File ownership: 只读 verification lane；发现缺陷退回对应 T1/T2/T3/T4 owner 的新轮次，
    不在 verification lane 并行改 shared files。

## 并行拆分

- T1 → dependency retarget gate → T2 → T3 → T4 → T5 串行，因为 `view/mod.rs` 与
  `tests/chat_message_views.rs` 逐步 handoff；任一时刻只有一个 writable owner。
- T1 冻结 `custom.rs` 后 T2 不修改；T2 冻结 `block.rs` 后 T3 不修改；
  T3 冻结 `message.rs`/`cache.rs` 后 T4 只写 exports/docs/golden 和 handoff test。
- 独立 reviewer、GH-58/GH-62 dependency evidence 与 coverage artifact审查可作为
  read-only lanes 并行；不得与 writable owner 共享写文件。

## 验证

- `verify_gh63_upstream_gate origin/main`
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`
- `cargo test --workspace --all-targets --all-features --locked`
- `cargo test --doc --workspace --all-features --locked`
- `verify_gh63_docs_gate`；复用 GH-62 exact-one helper 实际得到一个普通 chat rustdoc
  passed/zero ignored，并审计全部十个 chat/view files。
- `verify_gh63_branch_matrices`；dispatch/status+typed-reason/override/
  TextFlow-preview/changefeed-cache 五类输入集合分别等于其声明的闭集。
- `verify_gh63_new_code_coverage`；fresh Cobertura path/head.sha 等于 current full SHA，
  五个 planned view sources 全部存在且被报告，aggregate line-rate >=80%。
- `git diff --exit-code <implementation-base> -- Cargo.toml Cargo.lock src/components/chat/model.rs src/components/chat/error.rs src/components/chat/state.rs src/components/chat/reducer.rs src/components/display/message.rs`
- implementation diff 只含 planned-changes paths；examples、shell/list/composer、provider、
  workflow 或 dependency drift 即阻断。
- GH-58/GH-62 completion evidence、full head coverage artifact、fresh CI、独立 reviewer
  artifact、所有 reviewThreads 与 SpecRail PR gate 均指向当前 PR exact head。

## Handoff Notes

- 本 packet 是 stacked spec：PR base 必须是 `spec/GH62-conversation-model`，使 diff 只含
  `specs/GH63/*`；它不代表 GH-62 implementation 已完成。
- 本轮只对齐 GH-62 exact base `8eab00ea6bd8bc90ec38c00447f752149ba0efb7`
  的 provisional-current contract；PR #74 round 12 未最终 review。实现前必须对 final merged
  GH-62 做 contract diff，漂移则重开 GH-63 spec review。
- implementation 只有在 GH-62 merged completion 后开始；若 final API 缺少本 spec 所需
  typed read accessors，先回到 GH-62，不得解析 debug output 或复制 model。
- T1 只需 GH-62；T2 前必须 retarget 到 current main 并通过 GH-58/GH-62 external gate。
  GH-58 不在当前 stacked manifest 的 `spec_refs`，但其 merged packet/source/closing PR
  必须由 `verify_gh63_upstream_gate` 在 implementation head 上逐项证明。
- Thinking/ToolResult preview 只消费 GH-58 exact-source logical projection；禁止
  `str::lines()` 或本地 tokenizer。LF/CRLF/CR/consecutive/trailing fixtures 任一失败都阻断。
- stable view identity 只来自 `MessageBlockEntry` 的 public BlockId accessor；position、
  ThinkingId、ToolCallId 只能分别作为排序观察值、message-local lifecycle、
  conversation correlation，不能入 key。
- typed author/timestamp、ErrorContent 与全部复杂 payload 只经 GH-62 borrowed accessors；
  不创建 presentation metadata mirror，不解析 provider JSON。
- cache 由 caller 显式持有，只按 `affected_messages` 精确失效；Delete 清空该 message，
  Resend source 保留，新 message revision 1；restore/ledger eviction 不释放 tombstone。
- top-level Message、Thinking、ToolCall、ToolResult 的 Failed typed reason 都是默认输出
  必需内容；Error block 不能补偿丢失 reason。
- Theme 在 view 构造时恰好捕获一次或显式传入；determinism equality 包含该 snapshot，
  `into_element` 不读取 ambient theme。
- renderer fallback 只有显式 `UseDefault`；不要改成 `Option<Element>`、catch panic、
  wildcard-ignore block 或 failure-to-default。
- Thinking disclosure 是 controlled input；interaction/callback/focus 归 GH-66/GH-67。
- custom element 只替换 block body；stable key、ordered slot、message status shell 始终由
  library 持有。
- legacy `Message` 继续服务 simple strings；GH-63 不自动迁移数据、不修改 legacy source。
- examples 迁移在 GH-68；不要借 GH-63 修改 `examples/glm_chat.rs` 或 `examples/rnk_chat.rs`。
