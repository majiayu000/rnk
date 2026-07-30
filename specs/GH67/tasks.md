# Task Plan：固定底部区域的 FullscreenChatShell

## Linked Issue

GH-67: https://github.com/majiayu000/rnk/issues/67

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Required direct dependencies: GH-62、GH-63、GH-64、GH-65 final implementations
- Transitive gate: GH-65 final closure列出的全部依赖完成

## Implementation Gate

本 packet 是 spec-only，`ready_to_spec` 不授权 production edit。`SP67-T1` 开始前 coordinator
必须从 fresh `origin/main` 创建 implementation branch，并按 tech spec生成/验证四份
`DependencyCompletionRecord`。#62/#63/#64/#65 必须 CLOSED；其 final closure evidence列出的
完整 implementation PR set必须 merged且全部 merge commit是implementation base祖先。
GH-65 的三份 spec paths必须在该base真实存在且由ancestor merge引入；当前spec base不存在，
因此不得进入当前 `spec_refs`。final GH-64/GH-65还必须公开符合tech §1的
prepare/read-only-view/infallible-commit/abort capability；只提供立即修改live state的API时
门禁blocked，必须先修上游并更新/re-review本packet。

GH-65 当前 packet/PR 的 cap-exhausted review、未解决 constructor/zero-row/active-handle/
navigation/coverage-mode缺陷，或任一 open/partial PR都不能满足门禁。coordinator必须重读最终
merged public API并做manifest source-drift audit；漂移则停止、更新本 packet并重新人工批准。

所有 filtered tests必须先以 `--list --exact --include-ignored` 得到唯一 match，再实际执行
`--exact --include-ignored` 并得到恰好 `1 passed; 0 failed; 0 ignored`。普通 substring、
zero-match、ignored或宽泛 workspace green不能替代。

<!-- gh57-critical-paths-v1
{"version":1,"issue":67,"critical_paths":[{"file":"src/components/chat/fullscreen/tests.rs","name":"gh67_fixed_bottom_resize_contract","verification_command":"cargo test --workspace --lib --locked components::chat::fullscreen::tests::gh67_fixed_bottom_resize_contract -- --exact"},{"file":"src/components/chat/fullscreen/tests.rs","name":"zero_and_undersized_terminals_fail_before_callbacks","verification_command":"cargo test --workspace --lib --locked components::chat::fullscreen::tests::zero_and_undersized_terminals_fail_before_callbacks -- --exact"},{"file":"src/components/chat/fullscreen/tests.rs","name":"owning_state_bundle_preserves_single_component_revisions","verification_command":"cargo test --workspace --lib --locked components::chat::fullscreen::tests::owning_state_bundle_preserves_single_component_revisions -- --exact"},{"file":"src/components/chat/fullscreen/tests.rs","name":"composer_cap_reprojects_cursor_window_before_partition","verification_command":"cargo test --workspace --lib --locked components::chat::fullscreen::tests::composer_cap_reprojects_cursor_window_before_partition -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"upstream_prepare_commit_abort_gate_and_late_failure_are_atomic","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked upstream_prepare_commit_abort_gate_and_late_failure_are_atomic -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"focus_overlay_key_routing_is_single_target_and_deterministic","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked focus_overlay_key_routing_is_single_target_and_deterministic -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"overlay_route_matrix_is_total_and_passive_focus_is_rejected","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked overlay_route_matrix_is_total_and_passive_focus_is_rejected -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"pointer_overlay_tab_order_wraps_deterministically","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked pointer_overlay_tab_order_wraps_deterministically -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"shell_events_and_session_commands_are_disjoint_and_total","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked shell_events_and_session_commands_are_disjoint_and_total -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"paste_and_committed_ime_text_dispatch_exactly_once","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked paste_and_committed_ime_text_dispatch_exactly_once -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"rapid_resize_stream_prepend_sequence_is_deterministic","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked rapid_resize_stream_prepend_sequence_is_deterministic -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"layout_render_failure_preserves_committed_state_and_frame","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked layout_render_failure_preserves_committed_state_and_frame -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"fullscreen_terminal_restores_all_modes_on_every_exit_path","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked fullscreen_terminal_restores_all_modes_on_every_exit_path -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"partial_enter_and_suspend_resume_restore_exact_snapshot","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked partial_enter_and_suspend_resume_restore_exact_snapshot -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"primary_failure_and_all_cleanup_failures_are_preserved","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked primary_failure_and_all_cleanup_failures_are_preserved -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"native_snapshot_bootstrap_legacy_lease_and_poison_recovery_are_total","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked native_snapshot_bootstrap_legacy_lease_and_poison_recovery_are_total -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"fullscreen_session_public_surface_and_capability_gate_are_typed","verification_command":"cargo test --test fullscreen_chat_shell_public_api --locked fullscreen_session_public_surface_and_capability_gate_are_typed -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"public_observation_reports_focus_regions_follow_and_overlay","verification_command":"cargo test --test fullscreen_chat_shell_public_api --locked public_observation_reports_focus_regions_follow_and_overlay -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"visible_frame_work_is_bounded_and_handles_are_o1_non_evictable","verification_command":"cargo test --test fullscreen_chat_shell_public_api --locked visible_frame_work_is_bounded_and_handles_are_o1_non_evictable -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"specrail_checker_checkout_is_reproducible","verification_command":"cargo test --test fullscreen_chat_shell_public_api --locked specrail_checker_checkout_is_reproducible -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"specrail_mirror_binds_all_reviewed_dependency_refs","verification_command":"cargo test --test fullscreen_chat_shell_public_api --locked specrail_mirror_binds_all_reviewed_dependency_refs -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"gh67_current_head_coverage_contract","verification_command":"GH67_COVERAGE_MODE=fixture cargo test --test fullscreen_chat_shell_public_api --locked gh67_current_head_coverage_contract -- --exact"},{"file":"tests/fullscreen_chat_shell_public_api.rs","name":"coverage_validate_environment_survives_full_verification","verification_command":"cargo test --test fullscreen_chat_shell_public_api --locked coverage_validate_environment_survives_full_verification -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"initial_enter_requeries_size_and_stages_cap_first_frame","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked initial_enter_requeries_size_and_stages_cap_first_frame -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"initial_overlay_sequence_builds_lifo_focus_restoration","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked initial_overlay_sequence_builds_lifo_focus_restoration -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"passive_escape_falls_through_without_close","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked passive_escape_falls_through_without_close -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"conversation_outcome_snapshot_revision_binding_is_atomic","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked conversation_outcome_snapshot_revision_binding_is_atomic -- --exact"},{"file":"tests/fullscreen_chat_shell_interactions.rs","name":"revisioned_runtime_event_rejects_queued_stale_shell_input","verification_command":"cargo test --test fullscreen_chat_shell_interactions --locked revisioned_runtime_event_rejects_queued_stale_shell_input -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"partial_grouped_transition_restores_full_snapshot_before_release","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked partial_grouped_transition_restores_full_snapshot_before_release -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"rejected_enter_returns_backend_and_pending_input_in_order","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked rejected_enter_returns_backend_and_pending_input_in_order -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"public_native_snapshot_constructor_is_reachable_and_restorable","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked public_native_snapshot_constructor_is_reachable_and_restorable -- --exact"},{"file":"tests/fullscreen_chat_shell_pty.rs","name":"canonical_tty_query_phase_reads_newline_free_replies_and_restores_termios","verification_command":"cargo test --test fullscreen_chat_shell_pty --locked canonical_tty_query_phase_reads_newline_free_replies_and_restores_termios -- --exact"}]}
-->

## Continue-once finding closure

第三轮correction必须保留既有F001–F003/F005–F015回归，并以同一exact test中的正、反fixture
逐项关闭下表；反例必须在callback/terminal publication前失败且状态、frame、backend/input
ownership可逐值核对：

| Finding | Positive fixture | Negative fail-closed fixture |
| --- | --- | --- |
| F004 | public native constructor从canonical TTY取得verified snapshot并完整恢复 | query/restore失败保留唯一recovery owner，不能默认modes或释放lease |
| F016 | enter在lease内fresh size上cap-first prepare/render/commit | constructor旧size与try_size失败均不发布frame |
| F017 | ordered initial opens生成逐层saved-focus chain | 无history stack、非法base/overlay focus在constructor中原子拒绝 |
| F018 | top/focused Pointer Escape关闭一层，Passive Escape fall through | top Passive不关闭、不消费、不改变focus/revision |
| F019 | outcome、snapshot与last revision checked successor一致 | stale/skipped/replayed/mismatched pair零prepare、零mutation |
| F020 | grouped mouse transition部分写失败后恢复完整snapshot target | 任一attempted step未恢复时flush/release不得成功 |
| F021 | Rejected归还原backend且pending ordinary input顺序不变 | error/recovery路径不得drop、复制或重排pending input |
| F022 | 入队revision经poll/run/dispatch原样到handler | intervening mutation后的queued event不得用current revision刷新 |
| F023 | temporary noncanonical query读取newline-free DECRPM后恢复termios | canonical timeout/partial reply或query restore/flush失败进入recovery |

## Durable coverage evidence

T3实现 `gh67_current_head_coverage_contract` 的 closed
`fixture|collect|produce|validate` mode；mode缺失或越界必须失败。所有路径环境变量必须是
absolute path，PR number必须是正整数，全部SHA必须是40-hex且等于当前immutable window：

```text
GH67_COVERAGE_MODE
GH67_PR_NUMBER
GH67_PR_HEAD_SHA
GH67_PR_BASE_SHA
GH67_CURRENT_MAIN_SHA
GH67_COVERAGE_MERGE_BASE_SHA
GH67_COVERAGE_RAW
GH67_COVERAGE_ARTIFACT
```

`fixture`用scratch raw/diff/ledger正负样本证明：正常样本成功；缺/重复/额外critical、
空raw、wrong hash/PR/head/base/merge-base、zero executable、79.99%、99.99% critical、
空command、未带mode和absolute-path violation均失败。`collect`在llvm-cov全套运行中只
验证writable destinations与PR/head/base/merge-base不可变窗口后通过，不读取尚未生成raw。

T4在clean current implementation head上按唯一顺序执行：

```bash
set -euo pipefail
case "$GH67_PR_NUMBER" in ''|*[!0-9]*) exit 64 ;; esac
git fetch --prune origin main
GH67_PR_HEAD_SHA="$(gh pr view "$GH67_PR_NUMBER" --repo majiayu000/rnk \
  --json headRefOid --jq .headRefOid)"
GH67_PR_BASE_SHA="$(gh api \
  "repos/majiayu000/rnk/pulls/$GH67_PR_NUMBER" --jq .base.sha)"
GH67_CURRENT_MAIN_SHA="$(git rev-parse origin/main)"
for sha in "$GH67_PR_HEAD_SHA" "$GH67_PR_BASE_SHA" "$GH67_CURRENT_MAIN_SHA"; do
  case "$sha" in ''|*[!0-9a-f]*) exit 65 ;; esac
  test "${#sha}" -eq 40
done
test "$(git rev-parse HEAD)" = "$GH67_PR_HEAD_SHA"
test "$(git rev-parse origin/main)" = "$GH67_CURRENT_MAIN_SHA"
test -z "$(git status --porcelain)"
GH67_COVERAGE_MERGE_BASE_SHA="$(
  git merge-base "$GH67_CURRENT_MAIN_SHA" "$GH67_PR_HEAD_SHA"
)"
case "$GH67_COVERAGE_MERGE_BASE_SHA" in ''|*[!0-9a-f]*) exit 65 ;; esac
test "${#GH67_COVERAGE_MERGE_BASE_SHA}" -eq 40
GH67_EVIDENCE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/rnk-gh67-coverage.XXXXXX")"
GH67_EVIDENCE_DIR="$(cd "$GH67_EVIDENCE_DIR" && pwd -P)"
trap 'rm -rf "$GH67_EVIDENCE_DIR"' EXIT
GH67_COVERAGE_RAW="$GH67_EVIDENCE_DIR/llvm-cov.json"
GH67_COVERAGE_ARTIFACT="$GH67_EVIDENCE_DIR/gh57-child-coverage-v1.json"
export GH67_PR_NUMBER GH67_PR_HEAD_SHA GH67_PR_BASE_SHA GH67_CURRENT_MAIN_SHA
export GH67_COVERAGE_MERGE_BASE_SHA GH67_EVIDENCE_DIR

GH67_COVERAGE_MODE=collect \
GH67_PR_NUMBER="$GH67_PR_NUMBER" \
GH67_PR_HEAD_SHA="$GH67_PR_HEAD_SHA" \
GH67_PR_BASE_SHA="$GH67_PR_BASE_SHA" \
GH67_CURRENT_MAIN_SHA="$GH67_CURRENT_MAIN_SHA" \
GH67_COVERAGE_MERGE_BASE_SHA="$GH67_COVERAGE_MERGE_BASE_SHA" \
GH67_COVERAGE_RAW="$GH67_COVERAGE_RAW" \
GH67_COVERAGE_ARTIFACT="$GH67_COVERAGE_ARTIFACT" \
  cargo llvm-cov --workspace --all-targets --all-features --locked --json \
    --output-path "$GH67_COVERAGE_RAW"

GH67_COVERAGE_MODE=produce \
GH67_PR_NUMBER="$GH67_PR_NUMBER" \
GH67_PR_HEAD_SHA="$GH67_PR_HEAD_SHA" \
GH67_PR_BASE_SHA="$GH67_PR_BASE_SHA" \
GH67_CURRENT_MAIN_SHA="$GH67_CURRENT_MAIN_SHA" \
GH67_COVERAGE_MERGE_BASE_SHA="$GH67_COVERAGE_MERGE_BASE_SHA" \
GH67_COVERAGE_RAW="$GH67_COVERAGE_RAW" \
GH67_COVERAGE_ARTIFACT="$GH67_COVERAGE_ARTIFACT" \
  cargo test --test fullscreen_chat_shell_public_api --locked \
    gh67_current_head_coverage_contract -- --exact

GH67_COVERAGE_MODE=validate \
GH67_PR_NUMBER="$GH67_PR_NUMBER" \
GH67_PR_HEAD_SHA="$GH67_PR_HEAD_SHA" \
GH67_PR_BASE_SHA="$GH67_PR_BASE_SHA" \
GH67_CURRENT_MAIN_SHA="$GH67_CURRENT_MAIN_SHA" \
GH67_COVERAGE_MERGE_BASE_SHA="$GH67_COVERAGE_MERGE_BASE_SHA" \
GH67_COVERAGE_RAW="$GH67_COVERAGE_RAW" \
GH67_COVERAGE_ARTIFACT="$GH67_COVERAGE_ARTIFACT" \
  cargo test --test fullscreen_chat_shell_public_api --locked \
    gh67_current_head_coverage_contract -- --exact

export GH67_COVERAGE_MODE=validate
export GH67_PR_NUMBER GH67_PR_HEAD_SHA GH67_PR_BASE_SHA GH67_CURRENT_MAIN_SHA
export GH67_COVERAGE_MERGE_BASE_SHA GH67_COVERAGE_RAW GH67_COVERAGE_ARTIFACT
```

以上八个变量从validate成功后保持export，贯穿全部mapped/ledger/full workspace tests；窗口末尾
重新fetch/query并分别断言PR head、PR base、current `origin/main`、merge-base和clean worktree，
否则丢弃全部coverage/test evidence。

窗口末尾必须在同一export环境机械重验四个不同SHA：

```bash
set -euo pipefail
git fetch --prune origin main
test "$(gh pr view "$GH67_PR_NUMBER" --repo majiayu000/rnk \
  --json headRefOid --jq .headRefOid)" = "$GH67_PR_HEAD_SHA"
test "$(gh api "repos/majiayu000/rnk/pulls/$GH67_PR_NUMBER" \
  --jq .base.sha)" = "$GH67_PR_BASE_SHA"
test "$(git rev-parse origin/main)" = "$GH67_CURRENT_MAIN_SHA"
test "$(git merge-base "$GH67_CURRENT_MAIN_SHA" "$GH67_PR_HEAD_SHA")" = \
  "$GH67_COVERAGE_MERGE_BASE_SHA"
test -z "$(git status --porcelain)"
```

producer从 committed tasks中解析唯一 ledger，读取实际raw，使用
`git diff --unified=0 "$GH67_COVERAGE_MERGE_BASE_SHA...$GH67_PR_HEAD_SHA"` 计算planned `.rs` added
executable lines。它不能接收caller传入critical摘要。ledger parser必须验证唯一block、
version=1、issue=67、非空有序entries及每项非空`file/name/verification_command`和tuple唯一；
任何缺失/多block/malformed/wrong metadata/空值/重复均fail closed。mandatory runner由解析结果
机械派生cardinality与顺序，逐项先list证明唯一match再执行；禁止硬编码23/32或caller subset。
canonical artifact schema：

```json
{
  "schema": "gh57-child-coverage-v1",
  "child_issue": 67,
  "head_sha": "<40-hex exact implementation PR head>",
  "pr_base_sha": "<40-hex GitHub PR base head>",
  "current_main_sha": "<40-hex fresh origin/main head>",
  "coverage_merge_base_sha": "<40-hex merge-base>",
  "generated_at": "<HEAD commit RFC3339 timestamp>",
  "provenance": {
    "repository": "majiayu000/rnk",
    "implementation_pr": 1,
    "tool": "cargo-llvm-cov",
    "raw_artifact_name": "llvm-cov.json",
    "raw_sha256": "<64-hex>"
  },
  "command": "GH67_COVERAGE_MODE=collect cargo llvm-cov --workspace --all-targets --all-features --locked --json --output-path $GH67_EVIDENCE_DIR/llvm-cov.json",
  "changed_executable": {
    "covered": 1,
    "total": 1,
    "percent": 100.0
  },
  "critical": [{
    "file": "src/components/chat/fullscreen/tests.rs",
    "name": "gh67_fixed_bottom_resize_contract",
    "verification_command": "cargo test --workspace --lib --locked components::chat::fullscreen::tests::gh67_fixed_bottom_resize_contract -- --exact",
    "covered": 1,
    "total": 1,
    "percent": 100.0
  }]
}
```

数组包含ledger全部32项且严格保持ledger顺序；示例只展开一项。`generated_at`取HEAD commit
timestamp而非wall clock，path只保存固定basename，JSON使用固定key order/UTF-8/LF与末尾
newline，使相同head/base/raw/ledger byte-for-byte确定。validator重新hash raw、重算
PR/head/base/merge-base/diff/executable/critical set/commands/count/percent并生成canonical
bytes比较；
changed `total>0`且≥80%，每个critical `total>0`且100%。active test必须实际执行，raw中的
旧/同名未执行function不算。result parser要求每个派生entry按ledger顺序恰有一条
`matched=1, passed=1, failed=0, ignored=0, exit=0`记录，并核对aggregate count/digest；
missing/extra/duplicate/unmatched/ignored/nonzero/incomplete result全部fail closed。

## Implementation Tasks

- [ ] `SP67-T1`（lane alias: `GH67-T1`）执行dependency/source-drift gate并建立contract/layout scaffold。 Covers: B-001, B-002, B-003, B-004, B-005, B-022, B-029 | Owner: `fullscreen-contract-layout-owner` | Done when: 下列completion criteria全部满足 | Verify: 下列exact commands和checkpoint check全部通过。
  Completion criteria: 四份final completion record通过，GH-65三条spec path在base存在，
  GH-64/GH-65 prepared mutation capability inventory满足tech §1；新增八个chat fullscreen
  production/test files；owning bundle/closed shell+session event types及planned 24 paths完整；
  parent `fullscreen.rs`、`state.rs`、
  `router.rs`、`session.rs`先提供private compile skeleton；`types.rs`/`error.rs`/`layout.rs`
  完成private-field constructors/accessors、zero-size reachable input、closed config/layout/
  state/session errors、checked rect/end/partition和optional status；chat/components/prelude
  concrete exports可从crate外编译；任何callback在invalid config/size前不调用；无alias、
  Any、default layout或未声明field。
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::validated_config_and_rects_are_closed -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::fixed_bottom_partition_uses_exact_remaining_rows -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::zero_and_undersized_terminals_fail_before_callbacks -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::status_absence_uses_zero_rows_and_invents_no_data -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked fullscreen_shell_public_surface_is_typed_and_controlled -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked dependency_completion_requires_closed_final_merged_ancestor_sets -- --exact`；
  `cargo check --workspace --all-targets --all-features --locked`。
  - Dependencies: Implementation Gate。
  - File ownership: 独占
    `src/components/chat/fullscreen.rs`、`fullscreen/{types,error,layout,state,router,session,tests}.rs`、
    `src/components/chat/mod.rs`、`src/components/mod.rs`、`src/prelude.rs`、
    `tests/fullscreen_chat_shell_public_api.rs`。创建后冻结 `types.rs`/`error.rs`/`layout.rs`和
    export files；把四个skeleton、module tests与public test串行移交T2。
  - Handoff: 保存dependency/path/capability records、final API inventory、manifest drift
    export files保留只读直至T3接管。

- [ ] `SP67-T2`（lane alias: `GH67-T2`）实现state/router与GH-64/GH-65 candidate integration。 Covers: B-002, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-027 | Owner: `fullscreen-state-router-owner` | Done when: 下列completion criteria全部满足 | Verify: 下列exact commands和checkpoint check全部通过。
  Completion criteria: 接管 `state.rs`/`router.rs`/module tests/
  public test，constructor返回唯一owning bundle并显式消费entries/config/projection inputs/
  measurement，保有non-evictable active O(1) handles；zero/undersized在List prepare前typed
  fail；先算composer cap、再从prepared candidate view重投影含cursor window；MessageList唯一处理
  measurement/invalidation/slices/anchor/follow；Composer/List分别生成不修改live state的
  prepared token/read-only view，commit infallible且abort discard-only；Composer clamp和
  resize同candidate；expected shell revision、Following/Paused/nonzero viewport/prepend、
  nested overlay LIFO与Modal/Pointer/Passive × focus × key/paste/mouse/fallthrough总表逐项
  完成；Pointer focus ring按stack正/反向wrap，shell/session closed domains只经total dispatch；
  constructor从base focus+ordered opens生成逐层restore chain；Passive Escape始终fall through；
  ConversationApplied三方revision绑定；revisioned envelope从入队到dispatch不可重写；explicit
  navigation优先且不会被visible-top刷新覆盖；每个事件最多一个target/一次
  revision，prepare后late render failure仍使List/Composer/shell/frame逐值相等。
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::constructor_requires_complete_entries_config_projection_and_measurement -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::owning_state_bundle_preserves_single_component_revisions -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::gh67_fixed_bottom_resize_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::composer_projection_clamps_without_overlap_and_keeps_draft -- --exact`；
  `cargo test --workspace --lib --locked components::chat::fullscreen::tests::composer_cap_reprojects_cursor_window_before_partition -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked variable_height_transcript_uses_rows_not_item_count -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked measurement_invalidation_and_active_handles_follow_exact_identity -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked following_stream_growth_tracks_latest_bottom_in_supported_viewport -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked paused_stream_growth_preserves_anchor_and_reports_new_content -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked prepend_preserves_stable_message_and_intra_row_anchor -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked continuous_resize_reflows_list_and_composer_in_one_frame -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked upstream_prepare_commit_abort_gate_and_late_failure_are_atomic -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked focus_overlay_key_routing_is_single_target_and_deterministic -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked overlay_route_matrix_is_total_and_passive_focus_is_rejected -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked pointer_overlay_tab_order_wraps_deterministically -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked shell_events_and_session_commands_are_disjoint_and_total -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked nested_overlay_escape_restores_focus_lifo_without_fallthrough -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked paste_and_committed_ime_text_dispatch_exactly_once -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked mouse_hit_testing_uses_committed_z_order_without_double_dispatch -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked rapid_resize_stream_prepend_sequence_is_deterministic -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked initial_overlay_sequence_builds_lifo_focus_restoration -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked passive_escape_falls_through_without_close -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked conversation_outcome_snapshot_revision_binding_is_atomic -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked revisioned_runtime_event_rejects_queued_stale_shell_input -- --exact`；
  `cargo check --workspace --all-targets --all-features --locked`。
  - Dependencies: SP67-T1完整handoff。
  - File ownership: 接管 `fullscreen/state.rs`、`fullscreen/router.rs`、
    `fullscreen/tests.rs`、`tests/fullscreen_chat_shell_public_api.rs`；独占新
    `tests/fullscreen_chat_shell_interactions.rs`。T1冻结文件只读。
  - Handoff: 保存upstream capability inventory/prepared-token trace、完整route cross-product、
    constructor/callback count、pre/post equality和operation counters；冻结router，移交
    state/tests/public/interactions给T3。

- [ ] `SP67-T3`（lane alias: `GH67-T3`）实现facade、checked frame、terminal session与goldens。 Covers: B-001, B-005, B-007, B-008, B-013, B-015, B-017, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-030 | Owner: `fullscreen-frame-session-owner` | Done when: 下列completion criteria全部满足 | Verify: 下列exact commands和checkpoint check全部通过。
  Completion criteria: 接管 `fullscreen.rs`/`session.rs`/
  `state.rs`/module与integration tests；facade只从GH-65 visible slices调用一个GH-63 render
  closure，base→overlay z-order确定；GH-60 checked layout/render成功后才进入无失败upstream/
  shell/frame commit section，所有typed/injected failures保留三个live states与旧frame；
  public backend/session与closed transition可crate外使用；native paired controlling TTY从termios+
  correlated DECRQM取得完整47/1047/1049/25/1000/1002/1003/1015/1006/1004/2004 snapshot，非reply
  event不丢；legacy Terminal/App、terminal_controller及panic handler共享process lease。
  partial enter反向rollback；suspend完整restore+flush+release；resume重新query size并用bundle
  prepare cap-first Resize frame后才enter/render/commit；首次enter同样在lease内fresh size上
  prepare cap-first frame，query用reversible temporary noncanonical phase。每个transition在I/O前
  登记完整restore target；Rejected归还backend/pending input。backend `RecoveryOwner`、session
  `Option<B>`与Poisoned registry在rollback/Drop/panic失败时保留唯一backend/lease/optional
  snapshot/unfinished steps；snapshot失败+release失败由lease-only owner恢复；Start/Run primary、
  session-transition及全部cleanup/retry sources无损；
  Viewport/TextArea/Status/Dialog metadata与public observation可读；plain/ANSI golden
  deterministic且测试禁止更新；实现coverage fixture/producer/validator。
  `cargo test --test fullscreen_chat_shell_interactions --locked typed_multiline_block_views_render_once_in_source_order -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked nested_overlay_z_order_and_invalid_updates_are_atomic -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked coordinate_revision_and_upstream_failures_are_atomic -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked layout_render_failure_preserves_committed_state_and_frame -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked upstream_prepare_commit_abort_gate_and_late_failure_are_atomic -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked fullscreen_session_public_surface_and_capability_gate_are_typed -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked public_observation_reports_focus_regions_follow_and_overlay -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked accessibility_and_plain_ansi_semantics_do_not_depend_on_color -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked visible_frame_work_is_bounded_and_handles_are_o1_non_evictable -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked fullscreen_shell_has_no_provider_tool_or_secret_execution_surface -- --exact`；
  `GH67_COVERAGE_MODE=fixture cargo test --test fullscreen_chat_shell_public_api --locked gh67_current_head_coverage_contract -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked fullscreen_terminal_restores_all_modes_on_every_exit_path -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked partial_enter_and_suspend_resume_restore_exact_snapshot -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked primary_failure_and_all_cleanup_failures_are_preserved -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked native_snapshot_bootstrap_legacy_lease_and_poison_recovery_are_total -- --exact`；
  `cargo test --test fullscreen_chat_shell_interactions --locked initial_enter_requeries_size_and_stages_cap_first_frame -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked partial_grouped_transition_restores_full_snapshot_before_release -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked rejected_enter_returns_backend_and_pending_input_in_order -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked public_native_snapshot_constructor_is_reachable_and_restorable -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked canonical_tty_query_phase_reads_newline_free_replies_and_restores_termios -- --exact`；
  `cargo test --test fullscreen_chat_shell_pty --locked suspend_resume_and_fresh_restart_rebuild_explicit_state -- --exact`；
  `cargo check --workspace --all-targets --all-features --locked`。
  - Dependencies: SP67-T2完整handoff；final GH-60 checked frame与terminal runtime inventory可用。
  - File ownership: 接管 `src/components/chat/fullscreen.rs`、
    `fullscreen/{state,session,tests}.rs`、`src/components/chat/mod.rs`、
    `src/components/mod.rs`、`src/prelude.rs`与public/interactions tests；独占
    `src/renderer/terminal.rs`（保持<800）、`src/renderer/terminal_controller.rs`、
    `src/runtime/panic_handler.rs`、
    `src/renderer/terminal/fullscreen_backend.rs`、`tests/fullscreen_chat_shell_pty.rs`与两个
    fullscreen golden files。router/types/errors/layout冻结只读。
  - Handoff: 保存frame transaction failure injection、public API inventory、golden checksums、
    fake/PTY snapshot/capability/lease ledger、nested/second-session rejection、suspended lease
    handoff、resume-conflict零mutation、partial-enter rollback、primary+cleanup inspection和
    coverage negative fixture；冻结production/session/exports后移交tests给T4。

- [ ] `SP67-T4`（lane alias: `GH67-T4`）迁移public-only example并生成current-head evidence。 Covers: B-001, B-013, B-014, B-023, B-024, B-026, B-028, B-029, B-030 | Owner: `fullscreen-example-evidence-owner` | Done when: 下列completion criteria全部满足 | Verify: 下列coverage、exact和full gate commands全部通过。
  Completion criteria: example只组合public Conversation/
  views/Composer/MessageList/shell，不再含private role/message/editor/item-scroll/height/focus/
  resize/cleanup或direct ANSI；semantic exact test调用与main相同production path；先执行
  coverage collect→produce→validate，再export
  `GH67_COVERAGE_MODE=validate`、全部immutable variables和absolute paths运行所有mapped/
  critical/full tests，末尾fresh requery确认head/base/main/merge-base/clean均未漂移；
  golden前后checksum相同；implementation diff exact等于manifest；CI与所有本地evidence绑定
  同一clean exact head。
  `cargo check --example rnk_chat --all-features --locked`；
  `cargo test --test fullscreen_chat_shell_public_api --locked rnk_chat_example_uses_only_public_fullscreen_composition -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked specrail_checker_checkout_is_reproducible -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked specrail_mirror_binds_all_reviewed_dependency_refs -- --exact`；
  `cargo test --test fullscreen_chat_shell_public_api --locked coverage_validate_environment_survives_full_verification -- --exact`；
  `cargo fmt --all -- --check`；
  `cargo check --workspace --all-targets --all-features --locked`；
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
  `cargo test --workspace --all-targets --all-features --locked`；
  `cargo test --doc --workspace --all-features --locked`；
  Durable coverage本节三条命令；
  Product-to-Test Mapping全部exact tests与唯一ledger机械派生的全部命令（当前reviewed
  packet为32项；禁止硬编码数量或传入subset），并由result parser验证有序一一对应；
  tech §10 fixed
  URL/commit/checksum workflow/depth命令。
  - Dependencies: SP67-T3完整handoff；所有production writers停止。
  - File ownership: 独占 `examples/rnk_chat.rs`；接管三个integration tests与goldens只修正
    example/evidence assertions，不修改冻结production files。发现生产缺陷退回对应owner新
    checkpoint，禁止跨ownership偷改。
  - Handoff: 提交exact head/base/merge-base、dependency records、all command fresh outputs、
    golden checksums、coverage artifact/raw digests和planned-path audit给T5。

- [ ] `SP67-T5`（lane alias: `GH67-T5`）执行只读closure audit。 Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029, B-030 | Owner: `fullscreen-independent-reviewer` | Done when: 下列closure criteria全部满足 | Verify: read-only重跑下列current-head evidence。
  Closure criteria: 独立核对B-001..B-030与tech mapping/tasks
  Covers集合严格相等；manifest/spec refs/line limits/ownership DAG/compile checkpoints完整；
  当前`spec_refs`全部存在，GH-65三path只作为hard gate在implementation base验证
  existence/ancestry后才可加入；dependency final evidence、prepared capability inventory与
  ancestor set fresh；mapped/critical tests、raw/canonical coverage、example/golden/PTY/
  full suite、fixed SpecRail checkout/checksums、CI、reviewThreads与SpecRail PR gate均指向
  current PR exact head；任何head/worktree/remote drift都丢弃evidence并退回T4重建。
  read-only从唯一committed ledger重新派生并重跑全部commands，以同一fail-closed result
  parser核对count/digest/逐项结果，再重跑tech mapping、coverage validate、full
  Rust/docs/example gates及fresh PR evidence；不得approve、resolve threads或merge。
  - Dependencies: SP67-T4完整evidence handoff；所有writers停止。
  - File ownership: 无writable path。
  - Handoff: 即使全部通过，最终implementation PR approval、merge、release、#67和GH-57
    closure仍由人类决定。

## Execution Graph and Ownership

```text
Dependency Gate -> SP67-T1 -> SP67-T2 -> SP67-T3 -> SP67-T4 -> SP67-T5
```

- writer tasks串行，因为state/module/integration tests按checkpoint handoff；任一时刻每个path
  只有一个writer。
- threads可并行收集read-only dependency/CI/coverage review证据，但不得与writer共享文件；
  T5 independent reviewer不得是T1–T4 writer。
- 每个task在handoff前运行自己新增的exact tests和`cargo check`；后续不得删除/ignore/
  弱化前序断言。T1先创建parent/test skeleton，避免T2/T3测试在module不存在时不可发现。
- 没有future-owner red tests：每个test与使其绿色的owned behavior在同一task加入。

## Invariant Coverage Audit

Expected product set:

`{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029, B-030}`

Tech mapping set与Task `Covers:` union必须严格等于该集合。新增/删除B-ID时必须同时更新
product、tech mapping、affected tasks、本审计与critical ledger（若critical scope改变）；
不能只依赖T5 catch-all补号。

## Verification

- `git diff --check`
- `python3 .github/scripts/check_markdown_links.py specs/GH67`
- 按tech §10从`https://github.com/majiayu000/specrail.git` fetch/checkout immutable commit
  `bfc60f26164af5df1ebd3b5cb79d07379fc416b7`；要求Git、Python 3.9+与tar，零`pip install`；
  校验`checks/check_workflow.py` SHA-256为
  `c5bd73060037b0e8febace0e5ee8473e17973e1ca17257ea1517a94e05fa7549`、
  `tools/spec_depth_audit.py`为
  `380169fcbad509e6bc1b6a555ae0fa469744662af7120e20e999206c226e66c3`；另从reviewed rnk exact
  head复制manifest全部15个refs，逐文件校验source/mirror SHA后在fresh mirror运行workflow与
  `--gate` depth；任何fetch/checkout/ref/checksum失败都blocked，禁止cached fallback。
- B-ID连续为B-001..B-030；product=tech mapping=tasks Covers union。
- planned-changes恰一份、issue=67、complete=true；24个path/15个spec_refs逐项存在于planned
  future manifest或reviewed exact head，implementation diff exact相等。
- 每个production file <800；T1/T2/T3 checkpoint `cargo check`可独立编译；ownership DAG无
  shared writer/cycle/future-owner red tests。
- dependency records fresh且#62/#63/#64/#65 closed/final merged/ancestor；GH-65 transitive
  records完整。
- Product-to-Test Mapping每项exact test matched=passed=1 ignored=0。
- ledger parser/result parser正负fixture覆盖缺/多/malformed block、wrong metadata、空值、
  duplicate/missing/extra/unmatched/ignored/nonzero；当前解析为32个unique `file+name`并逐项执行；coverage
  fixture/collect/produce/validate mode与absolute paths全部显式。
- changed executable≥80%、32个critical各100%，artifact可canonical byte-for-byte重算；
  validate环境贯穿后续全套测试且末尾immutable window fresh不漂移。
- golden checksum前后相等；public session/capability/exclusive lease、partial enter、
  normal/cancel/error/panic、suspend/resume、primary+cleanup restoration通过。
- fresh fmt/check/clippy/workspace all-target/all-feature tests/doc/example/CI/独立review/
  reviewThreads/PR gate绑定同一exact head；任何漂移重建evidence。

## Handoff Notes

- 本 packet不授权implementation、label change、approval或merge；spec PR body只用
  `Refs #67`，不能Fixes/Closes #67或GH-57。
- GH-65 final实现必须先修复active handle、constructor、zero-row、typed navigation和
  coverage mode缺陷，并提供prepared mutation capability；GH-67不得copy workaround。
- shell成功viewport永远nonzero；GH-65 zero-row只由其component suite验证。
- offset/height/anchor/slices单位唯一为terminal row；GH-65 observation是follow truth。
- Composer committed text/paste只经GH-64，message render只经GH-63，Conversation顺序只经
  GH-62；shell是orchestrator，不是第二实现。
- terminal restoration失败必须显式返回且唯一Poisoned recovery owner继续持有lease/snapshot/
  unfinished steps；primary不能被cleanup覆盖；Drop/panic hook不能构成成功证据。
- rollback使用普通revert，保留failure/dependency/coverage evidence，禁止force push。
