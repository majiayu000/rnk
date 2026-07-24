# Task Plan：事务式增量布局与类型化错误

## Linked Issue

GH-60: https://github.com/majiayu000/rnk/issues/60

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Required dependency: [`../GH59/product.md`](../GH59/product.md)、
  [`../GH59/tech.md`](../GH59/tech.md)、[`../GH59/tasks.md`](../GH59/tasks.md)
- Transitive TextFlow dependency: [`../GH58/product.md`](../GH58/product.md)、
  [`../GH58/tech.md`](../GH58/tech.md)、[`../GH58/tasks.md`](../GH58/tasks.md)

## 实现任务

- [ ] `SP60-T1`（lane alias: `GH60-T1`）建立 public root-cause 与 compatibility fixture。Owner: `root-cause-test-lane` | Done when: 旧实现的 partial mutation 被确定性复现，最终 checked transaction 保持 committed state且兼容 surface可编译 | Verify: 运行本任务下列三个 exact integration commands。
  基于 GH-59 merged public surface 构造一个
  mixed patch batch，使旧实现先成功 update、后遇 missing target；实现前 fixture 确定性
  证明 partial mutation，最终 head 改为调用 checked transaction并证明 recovered/error
  后 committed state 不含 partial update；同一文件预先声明旧 signatures/struct literals
  compile fixtures，checkpoint 后显式 handoff：
  `cargo test --test incremental_transaction --locked mixed_batch_failure_commits_no_partial_state -- --exact`;
  `cargo test --test incremental_transaction --locked invalid_plan_returns_without_rebuild_or_mutation -- --exact`;
  `cargo test --test incremental_transaction --locked public_transaction_compatibility_surface_compiles -- --exact`。
  - Dependencies: GH-59 已合入 main。
  - File ownership: 独占 `tests/incremental_transaction.rs`；不写 layout/renderer/spec。
  - Covers: B-002, B-003, B-007, B-012, B-017, B-018, B-023。
  - Handoff: 提交 red root-cause checkpoint 后把 fixture 所需 public assertions 交给 T2；
    T1 owner 不与 T2/T5 同时编辑该文件。

- [ ] `SP60-T2`（lane alias: `GH60-T2`）实现独立 typed wrapper、per-kind scoped raw-Patch batch preflight、alias overlay、clone-staging transaction与五类 atomic apply。Owner: `layout-transaction-lane` | Done when: GH59 exhaustive enum不变，raw cardinality/collision/dependency在mutation前精确失败，no-op刷新fresh ElementId aliases，五类patch/recompute全部typed且atomic | Verify: 运行本任务下列十六个 exact commands。
  在 GH-59 concrete error/engine modules上增加
  `IncrementalPatchKind`、`PatchStage`、`PatchTransactionError`、
  `DirectPatchPreflightCause/Error`、`DirectPatchError`、concrete
  `DirectPatchApplyReport`、从首版 `#[non_exhaustive]` 的 `TransactionalLayoutError` 与
  checked report；不得修改 GH-59 `IncrementalLayoutError` variant集合。
  targetless `try_apply_patches_transactional` 在 clone/mutation 前按 original ordinal
  模拟 virtual scoped identities：create target absent/subtree collision-free，
  update/remove/replace target和create/reorder parent unique，batch-local
  create/replace/remove dependency与reorder bounds完整；error含 exact ordinal/kind/key/parent。
  target-aware checked API 才可把 failure交给 T3 recovery。
  `prepare_element_incremental` 只读 committed engine、在 clone candidate 上处理
  create/update/remove/replace/reorder/viewport recompute，所有 backend Result 显式传播；
  unchanged VNode/viewport 不改 Taffy/scoped state但构造 current ElementId alias overlay；
  closed cfg(test) fault backend 可分别在每一类 patch 的中间步骤失败；candidate
  success 尚不由 renderer发布；本 lane 新增 public item 必须有 rustdoc 并受
  `forbid(missing_docs)` 覆盖：
  `cargo test --workspace --lib --locked layout::engine::tests::unchanged_target_and_viewport_is_noop -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::unchanged_vnode_refreshes_current_element_id_aliases -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::viewport_only_recompute_is_transactional -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::each_patch_failure_has_exact_locator_and_cause -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::failed_or_dropped_candidate_preserves_committed_fingerprint -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::all_backend_failures_are_observed -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::fault_backend_is_test_only_and_diagnostics_are_terminal_safe -- --exact`;
  `cargo test --test incremental_transaction --locked mixed_batch_failure_commits_no_partial_state -- --exact`;
  `cargo test --test incremental_transaction --locked direct_patch_per_kind_cardinality_is_checked_before_mutation -- --exact`;
  `cargo test --test incremental_transaction --locked direct_create_and_subtree_collisions_report_exact_ordinal_and_kind -- --exact`;
  `cargo test --test incremental_transaction --locked direct_batch_dependencies_are_preflighted_in_order -- --exact`;
  `cargo test --test incremental_transaction --locked direct_patch_ambiguous_target_fails_before_mutation -- --exact`;
  `cargo test --test incremental_transaction --locked direct_patch_ambiguous_parent_fails_before_mutation -- --exact`;
  `cargo test --test incremental_transaction --locked direct_patch_apply_report_is_concrete_and_exact -- --exact`;
  `cargo test --test incremental_transaction --locked legacy_wrappers_delegate_to_checked_core -- --exact`;
  `cargo test --test incremental_transaction --locked legacy_apply_patches_ambiguity_fails_loudly_without_mutation -- --exact`。
  - Dependencies: GH60-T1 root-cause checkpoint/handoff。
  - File ownership: 独占 `src/layout/mod.rs`、`src/layout/incremental_error.rs`、
    `src/layout/engine.rs`、`src/layout/engine/incremental.rs`、
    `src/layout/engine/transaction.rs`、`src/layout/engine/tests.rs`；不写 renderer；
    T1 integration file 只在收到 handoff 后做满足 checked API 所需的最小更新。
  - Covers: B-001, B-002, B-003, B-004, B-007, B-012, B-013, B-017, B-018, B-019, B-020, B-021, B-026, B-028, B-030。
  - Handoff: 向 T3 交付 concrete transaction error、prepared candidate 与 fault seam；
    不在 T2 内实现 full rebuild fallback或 renderer default removal。

- [ ] `SP60-T3`（lane alias: `GH60-T3`）实现 initial checked build、一次 fresh rebuild、双 cause 与 target-exact postcondition。Owner: `layout-recovery-lane` | Done when: initial失败无虚构增量cause，target-aware commit failure只rebuild一次，success满足target exact，final failure保留双cause与旧state | Verify: 运行本任务下列十三个 exact commands。
  Candidate mutation/compute/read-back/postcondition failure只调用一次 `LayoutEngine::new()` checked
  rebuild；定义 concrete `RebuildFailure::InvalidTargetRoot/Taffy/TextFlow/Invariant`；
  rebuild success返回保留原 error的 recovered report；rebuild failure返回双
  cause并保持 committed fingerprint；postcondition 检查 root、reachable/total Taffy set、
  child order、scoped/ElementId/composite maps、required layouts/frame context，拒绝
  remove/replace orphan descendants；本 lane 新增 public item 的 rustdoc/
  `forbid(missing_docs)` 随实现提交；所有 exit drop候选资源：
  `cargo test --workspace --lib --locked layout::engine::tests::incremental_success_has_target_exact_tree_root_and_order -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::remove_replace_success_has_no_descendant_or_orphan_state -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::commit_failure_attempts_exactly_one_fresh_rebuild -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::rebuild_success_must_pass_target_exact_postcondition -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::repeated_fault_has_stable_result_and_rebuild_count -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::candidate_and_recovery_resources_drop_on_every_exit -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::initial_frame_success_commits_target_exact_state -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::initial_build_failure_has_no_incremental_cause_or_commit -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::initial_compute_failure_has_no_incremental_cause_or_commit -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::initial_postcondition_failure_has_no_incremental_cause_or_commit -- --exact`;
  `cargo test --test incremental_transaction --locked target_aware_patch_failure_rebuilds_once -- --exact`;
  `cargo test --test incremental_transaction --locked recovered_rebuild_preserves_incremental_cause -- --exact`;
  `cargo test --test incremental_transaction --locked rebuild_failure_returns_both_causes_and_preserves_committed_state -- --exact`。
  - Dependencies: GH60-T2 concrete handoff；T2 writer停止。
  - File ownership: 接管 T2 layout files；新增并独占
    `src/layout/engine/rebuild.rs`、`src/layout/engine/postcondition.rs`；接管
    `tests/incremental_transaction.rs`。不写 renderer。
  - Covers: B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012,
    B-013, B-019, B-021, B-024, B-025, B-026, B-028。
  - Handoff: prepared frame commit 在 layout 层必须是不可失败 move/swap；T4 只组合
    renderer/runtime publication，不重写 transaction/rebuild。

- [ ] `SP60-T4`（lane alias: `GH60-T4`）实现 generalized checked renderer，并把 initial/static/dynamic/alias/mouse 整帧延迟到 terminal success 后提交。Owner: `render-error-lane` | Done when: initial/layout/lookup typed fail closed、VirtualText正确过滤、任一prepare错误零mouse/terminal/static/runtime publication | Verify: 运行本任务下列二十一个 exact commands。
  `LayoutRenderError` / `CheckedRenderError` 与新
  `#[non_exhaustive] TransactionalFrameError` 组合 GH-59 `DynamicFrameError`，不向后者追加
  variant并保留独立 cause；`CheckedRenderError::LayoutBuild` 保留
  `TransactionalLayoutError`，使 static/string/testing initial build failure可恢复；
  tree/element/dynamic/static/string/testing generalized checked entrypoints 对 required
  layout fail closed，`Display::None`/`VirtualText` 在 lookup 前过滤；dynamic pipeline与
  static extraction都只构造局部 candidate，`PreparedAppFrame` 携带 desired mouse
  transition与 ElementId alias overlay；完整 prepare 后才写 mouse/frame bytes，成功后一次
  提交 static-lines/engine/previous/aliases/measurements，失败drop整帧；
  GH-58 Text-only try与旧 wrappers保持签名且不可表达 layout error时 fail loudly；本 lane
  为全部新增 public render entrypoint/error/re-export 编写 rustdoc并加
  `forbid(missing_docs)`：
  `cargo test --test layout_error_paths --locked text_identity_transaction_and_rebuild_causes_stay_distinct -- --exact`;
  `cargo test --test layout_error_paths --locked gh59_exhaustive_error_matches_still_compile_with_gh60_wrappers -- --exact`;
  `cargo test --test layout_error_paths --locked missing_layout_reaches_all_checked_render_entrypoints -- --exact`;
  `cargo test --test layout_error_paths --locked virtual_text_is_filtered_before_required_layout_lookup -- --exact`;
  `cargo test --test layout_error_paths --locked static_and_string_layout_failure_returns_no_partial_output -- --exact`;
  `cargo test --test layout_error_paths --locked checked_renderers_preserve_initial_layout_build_cause -- --exact`;
  `cargo test --test layout_error_paths --locked every_required_layout_failure_is_observed_without_fallback -- --exact`;
  `cargo test --test layout_error_paths --locked legacy_wrappers_compile_and_fail_loudly_on_final_error -- --exact`;
  `cargo test --test layout_error_paths --locked public_layout_vnode_patch_outcome_literals_compile -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::failure_commits_no_engine_previous_measurement_or_frame -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::cancelled_candidate_cannot_interleave_with_next_batch -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::terminal_error_drops_prepared_layout_frame -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::initial_prepared_app_frame_success_commits_once -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::initial_build_compute_and_postcondition_failures_write_and_publish_nothing -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::mixed_static_and_dynamic_failure_writes_no_terminal_or_static_state -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::mixed_static_and_dynamic_success_commits_once -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::layout_or_render_prepare_failure_emits_no_mouse_or_frame_bytes -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::mouse_mode_change_is_emitted_only_during_prepared_frame_terminal_commit -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::unchanged_frame_new_element_ids_render_and_commit_aliases -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::failed_unchanged_frame_keeps_previous_aliases -- --exact`;
  `cargo test --test gh60_public_docs --locked gh60_public_checked_surface_is_documented_and_compiles -- --exact`。
  - Dependencies: GH60-T3 prepared/recovery handoff；T3 writer停止。
  - File ownership: 独占 `src/renderer/mod.rs`、`src/renderer/error.rs`、
    `src/renderer/checked.rs`、
    `src/renderer/tree_renderer.rs`、`src/renderer/element_renderer.rs`,
    `src/renderer/pipeline.rs`、`src/renderer/app.rs`、`src/renderer/terminal.rs`,
    `src/renderer/render_to_string.rs`、`src/renderer/static_content.rs`,
    `src/testing/renderer.rs`、`src/lib.rs`、`src/prelude.rs`,
    `tests/layout_error_paths.rs`、`tests/gh60_public_docs.rs`；layout files只读。
  - Covers: B-001, B-004, B-007, B-010, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-025, B-027, B-028, B-030。
  - Handoff: 向 T5 交付三个 integration files与 exact public exports；不得提交 partial
    static/terminal output的 fixture必须在当前 head可重复。

- [ ] `SP60-T5`（lane alias: `GH60-T5`）完成 compatibility、changed-hunk fallback、public docs、coverage 与 full gates。Owner: `quality-evidence-lane` | Done when: public diff/runnable docs、fallback、coverage 与 full Rust gates 全部通过 | Verify: 重新运行全部 exact commands 与四个 full Rust commands。
  接管三个 integration files 补齐所有 exact negative/compatibility fixtures；证明
  dedicated modules 受 `forbid(missing_docs)`、`ignore/no_run/compile_fail` 被拒绝、
  required doctest exact runnable；执行 changed-hunk silent-fallback 语义 negative tests；
  新代码 changed-line coverage >=80% 且 transaction/rebuild/postcondition line+branch 均
  100%，由既有 CI Coverage job 报告；fresh full Rust gates、CI 与 independent review
  绑定同一 implementation head：
  重新运行 T1-T4 的全部 exact commands；
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`;
  `cargo test --workspace --doc --all-features --locked`;
  `cargo fmt --all -- --check`;
  `cargo check --workspace --all-targets --all-features --locked`;
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`;
  `cargo test --workspace --all-targets --all-features --locked`。
  - Dependencies: GH60-T1、T2、T3、T4全部完成并显式handoff；implementation PR exact head已知。
  - File ownership: 接管 `tests/incremental_transaction.rs`、
    `tests/layout_error_paths.rs` 与 `tests/gh60_public_docs.rs`；production paths只读。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029, B-030。
  - Handoff: independent reviewer必须与T1-T5 writer分离；只有current exact head的
    non-blocking review artifact、全部resolved threads、green CI和allowed pr_gate可进入
    `implx auto` merge step。

## 并行拆分

- Writable dependency graph 严格为 `T1 -> T2 -> T3 -> T4 -> T5`。GH-58/GH-59 与本
  issue 修改相同 engine/renderer paths，任何 dependency 未 merge 时不得提前开 writer。
- T1 首先独占 root-cause integration file；T2 收到 checkpoint 后接管所需 assertions；
  T3 接管全部 layout/recovery files；T4 只在 T3停止后写 renderer；T5最后接管 tests。
- 没有两个 writable lane并行。只有 read-only reviewer、CI观察或coverage结果审计可与当前
  writer并行，且不得 resolve threads、修改 source或生成共享 review artifact。
- 若 implementation 后真实文件拆分不同于 spec，先更新/review spec和ownership，
  不能让两个 lane临时共享一个文件。

## 验证

- Product invariant 集合与 tasks `Covers:` union 均为 B-001 至 B-030，无遗漏。
- planned-changes只允许 GH60 packet、layout transaction/recovery/postcondition、
  generalized renderer error/checked facade/caller、terminal prepared-frame adapter、
  public docs与三个明确integration tests；TextFlow algorithm、
  identity planner、runtime context、workflow、chat component或GH-61 benchmark变化必须先
  更新 spec。
- 所有 filtered tests 必须先 `--list --exact` 且 matched=1；只打印列表、零测试、substring
  filter、旧 SHA或其他issue tests不算证据。
- production transaction/rebuild/required-layout 新增行不得引入 `let _ =` wrapped Taffy、
  `.ok()?`、`unwrap_or_default()` 或 `filter_map` 形式的静默降级，并由 backend/
  required-lookup/legacy-delegation exact tests 提供语义证明；不得改写 unrelated 既有
  Unicode aggregation/fallback。
- implementation 必须基于已合入 GH-59 的 head；spec branch ancestry 不算。

## Handoff Notes

- 当前 PR 只交付 `specs/GH60/*`，base为 `spec/GH59-keyed-identity-order`；不得实现、
  改 label、merge、关闭 issue或resolve review threads。
- coordinator 在 implementation 前重新运行
  `python3 "$SPEC_RAIL_ROOT/checks/github_duplicate_evidence.py" --github-repo majiayu000/rnk --issue 60 --remote origin --json`
  并保存 exact JSON，再以 canonical `ready_to_implement` 和 current `implx auto` auth mode
  运行 implement route gate。
- clone-staging 是本实现 correctness primitive；normal incremental success保留clone内
  NodeId，rebuild recovery不承诺内部NodeId复用。任何换成journal/COW的提议必须先证明同一
  failure/postcondition suite并更新spec。
- GH-59 preflight error不rebuild；targetless raw Patch 的 missing/ambiguity/commit error
  及 create collision/order/batch dependency error不rebuild；只有 target-aware
  post-preflight mutation/compute/read-back/postcondition
  failure只rebuild一次；initial failure无伪造 incremental cause；final failure保留old
  committed state与双 cause。
- GH-58 Text error、GH-59 identity/lookup error、GH-60 transaction/rebuild/layout-render
  error保持独立；不得扩展 GH-59 exhaustive public enums，不得为减少variant把 cause转成
  String或generic I/O message。
- no-op frame仍生成 delayed ElementId alias overlay；mouse transition 仍是 terminal write，
  两者都只能随完整 `PreparedAppFrame` 成功提交。
- GH-60完成后只解锁GH-61、GH-64与GH-65的对应dependency gate；不直接宣称任何chat
  component完成。
