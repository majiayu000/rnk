# Task Plan：keyed 增量身份与子节点顺序

## Linked Issue

GH-59: https://github.com/majiayu000/rnk/issues/59

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Dependency packet: [`../GH58/product.md`](../GH58/product.md)、
  [`../GH58/tech.md`](../GH58/tech.md)、[`../GH58/tasks.md`](../GH58/tasks.md)
- Umbrella context: GH-57（只提供 F1 -> F2 -> F3 -> F4 队列依赖；GH-59 验收以本 packet 为准）

## 实现任务

- [ ] `SP59-T1`（lane alias: `GH59-T1`）建立只依赖当前 public API 的 root-cause fixture。Owner: `root-cause-test-lane` | Done when: public fixture 在实现前 red、最终 head green | Verify: `cargo test --test keyed_incremental_identity --locked keyed_reorder_must_not_create_or_remove_survivors -- --exact`。
  新增 `tests/keyed_incremental_identity.rs`，用
  public `VNode` / `diff()` / `Patch` 构造 keyed reorder，断言所有 surviving compatible keyed
  child 不产生 create/remove；当前实现因 keyed map identity 包含 index 而稳定 red。fixture
  不引用后续 internal scoped identity/plan，在实现前匹配恰好一个测试并产生预期 assertion，
  最终 head 通过。
  - Dependencies: 实现门已开；GH-58 implementation exact merged SHA 已记录。
  - Covers: B-004, B-007, B-008, B-012, B-016。

- [ ] `SP59-T2`（lane alias: `GH59-T2`）实现唯一 VNode identity source、pure scoped identity、typed validator 与 deterministic checked planner。Owner: `identity-plan-lane` | Done when: top-level root 由 traversal context 派生，metadata 判定表只用于 children，duplicate/collision、完整 final order、public checked diff、legacy fail-loud adapters 与 bounded property 均 fail closed 且确定 | Verify: 本任务列出的 exact 测试全部通过。
  `src/core/vnode.rs` 保留 public surface；`src/reconciler/error.rs` 定义实际 typed errors；
  `src/reconciler/identity.rs` 严格执行
  `props.key`/`NodeKey`/type 判定表并返回 `KeyMetadataMismatch`、`KeyTypeMismatch`，始终以
  child vector position 归一 compatibility index；`src/reconciler/plan.rs` 定义 typed duplicate、
  collision 与 missing/duplicate/extra final identity variants；`src/reconciler/diff.rs` 内联 tests
  实现 parent-scoped matcher、完整 `ParentPlan::final_children`、deterministic no-op 与 bounded
  proptest；`diff.rs` 新增返回 `Result<Vec<Patch>, ReconcilePlanError>` 的 public
  `try_diff`/`try_diff_children`，完整 validation 成功后才产出 owned patch vector；旧
  `diff` fail loudly，旧 `diff_children` 只在 checked 成功后一次性扩展 destination，
  error 时保持 destination 不变；`reconciler/mod.rs` 公开导出 checked API 与错误类型：
  `cargo test --workspace --lib --locked reconciler::diff::tests::keyed_match_ignores_position_within_parent -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::same_key_incompatible_type_is_replace -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::mixed_keyed_unkeyed_keeps_positional_contract -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::plan_contains_total_hole_free_final_order -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::identical_tree_has_empty_deterministic_plan -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::raw_hash_collision_never_aliases_exact_keys -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::opaque_token_collision_is_error -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::empty_key_is_keyed_and_duplicate_is_error -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::vnode_key_metadata_decision_table -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::mismatched_key_metadata_and_type_are_typed_errors -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::property_mixed_key_permutations_are_bijective_or_typed_error -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::discarded_plan_mutates_no_engine_state -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_invalid_nested_metadata_returns_error_without_partial_patches -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_children_duplicate_returns_error_without_patches -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_fails_loudly_on_invalid_input -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_children_fails_loudly_without_mutating_destination -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_box_root -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_text_root -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_component_root -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_accepts_public_non_container_roots -- --exact`。
  `vnode_key_metadata_decision_table` 必须覆盖 public props-only exact key；raw hash collision
  exact test 通过 crate-private controlled token source 让两个不同 props-only strings
  产生同一 compatibility token，不搜索真实 `DefaultHasher` collision。
  - Dependencies: GH59-T1 root-cause checkpoint 已提交；不写 T1 integration file。
  - Covers: B-001, B-002, B-003, B-005, B-006, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-017, B-018, B-021, B-022, B-023, B-025。

- [ ] `SP59-T3`（lane alias: `GH59-T3`）把 LayoutEngine 切到 scoped plan、preflight-atomic final order、成功 subtree map cleanup 与独立 checked error/lookup boundary。Owner: `layout-identity-lane` | Done when: top-level VNode 类型不受 root sentinel 限制，保留 GH-58 TextFlow-only error，复用 surviving NodeId，preflight typed failures 零 mutation，成功 map set 精确等于 target tree，exact order/composite lookup 可恢复 | Verify: 本任务列出的 exact 测试全部通过。
  在 `src/layout/engine/patch_error.rs` 定义并从 `src/layout/mod.rs` 导出
  `IncrementalLayoutError`/`LayoutLookupError`；两者实现 `Error`，只有 composite wrapper
  variants expose nested sources，leaf lookup error 不伪造 source chain。保留 GH-58 TextFlow-only
  try boundary，并新增 `try_compute_element_incremental_checked` composite boundary；
  engine 直接调用 checked plan core 并传播 `ReconcilePlanError`，不得调用 legacy
  `diff`/`diff_children` panic wrappers；
  actual layout ownership 是 `src/layout/engine.rs`、`src/layout/engine/context_sync.rs`、
  `src/layout/engine/identity_index.rs`、`src/layout/engine/incremental.rs`、
  `src/layout/engine/incremental_order.rs`、`src/layout/engine/patch_error.rs`、
  `src/layout/engine/patching.rs`、`src/layout/engine/tests.rs`、
  `src/layout/engine/text_flow_bridge.rs` 与 `src/layout/mod.rs`，各文件保持 800 行以下；
  internal map 使用 scoped identity，same-key/different-parent 投影为
  两个 collision-checked composite NodeKeys；legacy raw lookup 的 0/1/N 候选返回
  None/layout/typed ambiguity；missing/duplicate/extra final identity 和 missing existing
  NodeId 全部在 create/remove/set_children 前失败并证明 tree/map/root/previous VNode 不变；
  commit 后 exact read-back；成功时一次性刷新 target-exact scoped/composite/ElementId maps，
  清除 remove/replace descendants 与 cross-parent old scope。`patching.rs` 的变更只处理 GH-59
  scoped identity/addressing、direct-patch preflight 与 cleanup；public raw `Patch` topology、
  whole-frame terminal transaction、rollback/rebuild/fallback 仍留给 GH-60：
  `cargo test --workspace --lib --locked layout::engine::tests::keyed_ancestor_reorder_preserves_descendant_identity -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::same_key_in_distinct_parents_has_distinct_nodes -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::keyed_insert_delete_and_moves_reuse_survivor_nodes -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::taffy_child_order_equals_target_vnode_order -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::duplicate_sibling_key_fails_before_mutation -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::cross_parent_move_is_remove_and_create -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::same_raw_key_across_parents_has_two_composite_layouts -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::raw_legacy_lookup_reports_typed_ambiguity -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::invalid_final_order_variants_fail_before_mutation -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::textflow_and_identity_causes_remain_distinct -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::successful_remove_cleans_descendant_identity_maps -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::successful_replace_cleans_old_descendant_identity_maps -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::cross_parent_move_cleans_old_scope_without_deleting_new_scope -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::checked_layout_accepts_public_box_text_component_roots -- --exact`;
  `cargo test --test identity_error_paths --locked duplicate_incremental_target_changes_no_engine_state -- --exact`;
  `cargo test --test identity_error_paths --locked no_op_plan_rejects_a_drifted_previous_tree_without_committing_it -- --exact`;
  `cargo test --test keyed_incremental_error_paths --locked duplicate_key_reaches_checked_layout_boundary -- --exact`;
  `cargo test --test keyed_incremental_error_paths --locked direct_create_duplicate_subtree_is_an_identity_rejection -- --exact`;
  `cargo test --test keyed_incremental_error_paths --locked checked_diff_apply_handles_same_raw_key_in_distinct_parents -- --exact`。
  - Dependencies: GH59-T2；本任务独占上述 layout paths、`tests/identity_error_paths.rs`、
    `tests/keyed_incremental_error_paths.rs` 与 `tests/measurement_identity.rs`；完成 layout/error
    assertions 后再把两个 error-path integration files handoff 给 T5，不写 reconciler/T1 files。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-016, B-017, B-018, B-019, B-020, B-021, B-023, B-024, B-025。

- [ ] `SP59-T5`（lane alias: `GH59-T5`）把独立 incremental identity/lookup error 传播到 dynamic App caller，并发布 runtime-resident scoped measurement cardinality。Owner: `dynamic-runtime-lane` | Done when: `renderer/mod.rs` 接入并 public re-export 新 error，不给 GH-58 TextFlow-only errors 加 identity variants；runtime raw/string lookup 满足 0/1/N，成功 frame 原子替换 measurement snapshot，identity 失败不发布；GH-60 terminal transaction 仍不实现 | Verify: 本任务列出的 exact 测试全部通过。
  `src/renderer/error.rs` 新增独立 `DynamicFrameError` composite；`src/renderer/mod.rs` 声明
  `mod error` 并 `pub use error::DynamicFrameError`；`pipeline.rs` 新增 checked
  dynamic variant并使用 layout checked compute/try-all lookup，只在全部成功后更新 previous
  VNode，并把 engine validated scoped/composite layouts、raw candidates 与 string aliases 交给
  `RuntimeContext::set_measure_layouts_with_scoped_keys` 一次性替换。`runtime/context.rs` 保留现有
  helper 签名，checked raw/string lookup 对 0/1/N 返回 None/exact scoped value/typed ambiguity，
  legacy wrapper 在 N 时 fail loudly；`hooks/use_measure.rs` 补充该 panic contract 文档。
  unique -> ambiguous -> unique 必须清除 stale ElementId、scoped/composite/raw candidate 与
  alias；identity 失败不发布 measurement。`app.rs` 只调用 checked variant并把完整 source
  chain 映射为 `io::Error`。
  旧 renderer 和 GH-58 TextFlow-only try wrapper 保持签名。raw `Patch` topology、whole-frame
  terminal transaction、rollback/rebuild/fallback 仍属 GH-60：
  `cargo test --test keyed_incremental_error_paths --locked duplicate_key_reaches_checked_layout_boundary -- --exact`;
  `cargo test --test keyed_incremental_error_paths --locked dynamic_frame_error_is_publicly_exported -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::identity_error_commits_no_frame_or_previous_vnode -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::repeated_raw_measurement_key_is_typed_ambiguity -- --exact`;
  `cargo test --workspace --lib --locked renderer::pipeline::tests::measurement_candidates_transition_unique_ambiguous_unique -- --exact`;
  `cargo test --workspace --lib --locked runtime::context_tests::test_runtime_context_measurements_by_key -- --exact`;
  `cargo test --workspace --lib --locked runtime::context_tests::test_runtime_context_measurements_by_node_key_and_alias -- --exact`;
  `cargo test --workspace --lib --locked hooks::use_measure::tests::test_measure_element_by_key_with_runtime -- --exact`;
  `cargo test --workspace --lib --locked hooks::use_measure::tests::measure_element_by_key_panics_on_ambiguous_runtime_alias_without_leaking_runtime -- --exact`;
  `cargo test --test measurement_identity --locked same_raw_key_in_distinct_parents_keeps_both_layout_entries -- --exact`;
  `cargo test --test measurement_identity --locked raw_legacy_lookup_fails_loudly_when_multiple_scopes_match -- --exact`;
  `cargo test --test measurement_identity --locked checked_raw_lookup_reports_the_match_count -- --exact`;
  `cargo test --workspace --lib --locked renderer::app::tests::duplicate_key_reaches_app_io_error_without_frame_commit -- --exact`。
  - Dependencies: GH59-T3；T3 显式 handoff `tests/identity_error_paths.rs` 与
    `tests/keyed_incremental_error_paths.rs` 后，本任务独占这两个 files 的 final caller-surface
    assertions、renderer mod/error/pipeline/app、`src/runtime/context.rs` 与
    `src/hooks/use_measure.rs`。`src/runtime/context_tests.rs` unchanged and
    `tests/measurement_identity.rs` remain read-only verification；不写 layout/reconciler/spec。
  - Covers: B-009, B-010, B-014, B-017, B-018, B-019, B-020, B-021。

- [ ] `SP59-T4`（lane alias: `GH59-T4`）补齐 public compatibility、连续多帧 parity 与质量证据。Owner: `quality-evidence-lane` | Done when: compatibility/property/coverage/CI 全部通过 | Verify: 下列 exact 测试与全量 Rust gates 全部通过。
  在 T1 owner 明确 handoff 后扩展 `tests/keyed_incremental_identity.rs`，覆盖 public
  `NodeKey`/`VNode`/`Patch` compile surface、same key/type/index across parents 的两个
  composite all-layout entries、raw lookup typed ambiguity、keyed/unkeyed mixed list、连续
  reorder 与 incremental/full parity；新代码 changed-line coverage >=80%，
  `identity.rs`、`plan.rs`、`incremental.rs`、`incremental_order.rs` line/branch 均 100%，
  `.github/scripts/check_gh59_coverage.py`、`.github/scripts/gh59_coverage_lib.py`、
  `.github/scripts/gh59_coverage_source.py`、`.github/scripts/test_check_gh59_coverage.py` 与
  `.github/scripts/test_check_gh59_coverage_security.py` 实现并验证 trusted exact-head
  collect/produce/validate gate。四个 private coverage/error-matrix suites 位于
  `src/reconciler/identity/tests.rs`、`src/reconciler/plan/tests.rs`、
  `src/layout/engine/incremental/tests.rs`、`src/layout/engine/incremental_order/tests.rs`，只经
  `#[cfg(test)] mod tests` 接入；checker 按 `tests` path component 将这些 files 排除在 production
  changed-executable-line denominator 之外。production modules 的相应 delta 只允许
  semantics-preserving impossible-branch simplification 与 `#[cfg(test)]` fault seams；不得改变
  public/production behavior，也不得以 optional CI upload 代替：
  `cargo test --test keyed_incremental_identity --locked public_node_key_and_patch_surface_compiles -- --exact`;
  `cargo test --test keyed_incremental_identity --locked mixed_keyed_unkeyed_keeps_public_behavior -- --exact`;
  `cargo test --test keyed_incremental_identity --locked same_key_in_distinct_parents_has_layouts -- --exact`;
  `cargo test --test keyed_incremental_identity --locked consecutive_frames_match_full_rebuild -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::same_key_incompatible_type_is_replace -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::identical_tree_has_empty_deterministic_plan -- --exact`;
  `cargo test --workspace --lib --locked reconciler::plan::tests::planned_nodes_are_shallow_while_patch_payloads_retain_full_subtrees -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::mixed_keyed_unkeyed_keeps_positional_contract -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::property_mixed_key_permutations_are_bijective_or_typed_error -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_invalid_nested_metadata_returns_error_without_partial_patches -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_children_duplicate_returns_error_without_patches -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_fails_loudly_on_invalid_input -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_children_fails_loudly_without_mutating_destination -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::successful_remove_cleans_descendant_identity_maps -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::successful_replace_cleans_old_descendant_identity_maps -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::cross_parent_move_cleans_old_scope_without_deleting_new_scope -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_box_root -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_text_root -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_component_root -- --exact`;
  `cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_accepts_public_non_container_roots -- --exact`;
  `cargo test --workspace --lib --locked layout::engine::tests::checked_layout_accepts_public_box_text_component_roots -- --exact`;
  `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p 'test_check_gh59_coverage*.py'`;
  按 `tech.md` 的 `Exact-head coverage evidence` literal block 依次运行 `collect`、`produce`、
  `validate`，最后一条必须输出 `GH59 coverage evidence validated: decision=allowed`；
  `cargo fmt --all -- --check`;
  `cargo check --workspace --all-targets --all-features --locked`;
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`;
  `cargo test --workspace --all-targets --all-features --locked`。
  - Dependencies: GH59-T1、GH59-T2、GH59-T3、GH59-T5 全部完成；T1 file ownership 已显式交给
    T4；T4 独占 `tests/keyed_incremental_identity.rs`、上述四个 private critical test modules 与
    五个 coverage checker files，独立 reviewer 与 implementer 分离。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025。

## 并行拆分

- GH59-T1 首先独占 `tests/keyed_incremental_identity.rs` 建立 root-cause checkpoint。
- GH59-T2 在 T1 checkpoint 后独占 `src/core/vnode.rs`、`src/reconciler/mod.rs`、
  `src/reconciler/error.rs`、`src/reconciler/identity.rs`、`src/reconciler/plan.rs`、
  `src/reconciler/diff.rs`。
- GH59-T3 在 T2 identity/plan contract 稳定后独占 `src/layout/mod.rs`、
  `src/layout/engine.rs`、`src/layout/engine/context_sync.rs`、
  `src/layout/engine/identity_index.rs`、`src/layout/engine/incremental.rs`、
  `src/layout/engine/incremental_order.rs`、`src/layout/engine/patch_error.rs`、
  `src/layout/engine/patching.rs`、`src/layout/engine/tests.rs`、
  `src/layout/engine/text_flow_bridge.rs`、`tests/identity_error_paths.rs`、
  `tests/keyed_incremental_error_paths.rs`、`tests/measurement_identity.rs`。
- GH59-T5 在 T3 完成并 handoff error-path integration file 后，独占
  `src/renderer/mod.rs`、`src/renderer/error.rs`、`src/renderer/pipeline.rs`、
  `src/renderer/app.rs`、`src/runtime/context.rs`、`src/hooks/use_measure.rs`、
  `tests/identity_error_paths.rs`、`tests/keyed_incremental_error_paths.rs`；
  `src/runtime/context_tests.rs` 与 `tests/measurement_identity.rs` 只读。
- GH59-T4 在 T5 完成且 T1 owner 显式 handoff 后接管
  `tests/keyed_incremental_identity.rs`，并独占 `src/reconciler/identity/tests.rs`、
  `src/reconciler/plan/tests.rs`、`src/layout/engine/incremental/tests.rs`、
  `src/layout/engine/incremental_order/tests.rs`、`.github/scripts/check_gh59_coverage.py`、
  `.github/scripts/gh59_coverage_lib.py`、`.github/scripts/gh59_coverage_source.py`、
  `.github/scripts/test_check_gh59_coverage.py`、
  `.github/scripts/test_check_gh59_coverage_security.py`；其余活动为
  只读 verification/review。
- 依赖图为 `T1 -> T2 -> T3 -> T5 -> T4`，没有并行 writable lane、共享文件或 ownership cycle。
  如需 threads，只有 read-only reviewer 可与 verification 并行，禁止两个 writer 同时编辑
  engine/reconciler/spec paths。

## 验证

- Product invariant 集合与 tasks `Covers:` union 均为 B-001 至 B-025，无遗漏。
- planned-changes manifest 必须与 `git diff --name-only 9126bf2...HEAD`、unstaged diff 和
  intended untracked files 的 union 一致，只允许下列 paths。handoff 必须通过
  `(git diff --name-only 9126bf2 --; git ls-files --others --exclude-standard) | LC_ALL=C sort -u | wc -l`
  动态计算 cardinality，并将同一 sorted unique union 与本 manifest 逐项比较；不得沿用任何
  historical fixed path count：
  - packet/checker：`specs/GH59/product.md`、`specs/GH59/tech.md`、`specs/GH59/tasks.md`、
    `.github/scripts/check_gh59_coverage.py`、`.github/scripts/gh59_coverage_lib.py`、
    `.github/scripts/gh59_coverage_source.py`、`.github/scripts/test_check_gh59_coverage.py`、
    `.github/scripts/test_check_gh59_coverage_security.py`；
  - core/reconciler：`src/core/vnode.rs`、`src/reconciler/mod.rs`、
    `src/reconciler/error.rs`、`src/reconciler/identity.rs`、`src/reconciler/plan.rs`、
    `src/reconciler/diff.rs`；
  - layout：`src/layout/mod.rs`、`src/layout/engine.rs`、
    `src/layout/engine/context_sync.rs`、`src/layout/engine/identity_index.rs`、
    `src/layout/engine/incremental.rs`、`src/layout/engine/incremental_order.rs`、
    `src/layout/engine/patch_error.rs`、`src/layout/engine/patching.rs`、
    `src/layout/engine/tests.rs`、`src/layout/engine/text_flow_bridge.rs`；
  - renderer/runtime/hook：`src/renderer/mod.rs`、`src/renderer/error.rs`、
    `src/renderer/pipeline.rs`、`src/renderer/app.rs`、`src/runtime/context.rs`、
    `src/hooks/use_measure.rs`；
  - private critical coverage/error-matrix：`src/reconciler/identity/tests.rs`、
    `src/reconciler/plan/tests.rs`、`src/layout/engine/incremental/tests.rs`、
    `src/layout/engine/incremental_order/tests.rs`；
  - integration：`tests/identity_error_paths.rs`、`tests/keyed_incremental_error_paths.rs`、
    `tests/keyed_incremental_identity.rs`、`tests/measurement_identity.rs`。
  `src/runtime/context_tests.rs`、TextFlow variants、其他 runtime/hooks、chat/workflow 与任何
  其他 path 都不在 manifest。`patching.rs` 只承载 GH-59 identity/addressing changes；raw
  `Patch` topology 与 GH-60 whole-frame terminal transaction/rollback/rebuild/fallback unchanged。
- 所有 filtered cargo tests 只能通过 exact helpers；每条 command 必须在相同 target/options
  下先以 `--list --exact` 证明匹配数恰好为 1，0 个或多个匹配均失败。不得移除 `--exact`，
  该 preflight 不替代 coverage、full test 或其他 gate。
- optional Codecov/coverage job 不算成功证据；Cobertura checker 缺 changed/critical line 或
  branch observation（包括 `incremental.rs` apply/postcondition）、SHA mismatch、低于
  80%/100% 时必须 nonzero；evidence directory 创建失败同样必须立即 nonzero。
- current implementation head 必须包含 GH-58 merged SHA；只基于 GH-58 spec head 不算依赖完成。
- fresh fmt/check/clippy/all-target tests、coverage、CI、independent review、reviewThreads 与
  SpecRail PR gate 必须绑定同一 exact head。

## Handoff Notes

- Historical only（已 superseded）：最初 specs-only PR stacked on `spec/GH58-text-flow`
  `6e6e58932a009ab5c205a9227f996b1d4f604b35`，当时只允许 `specs/GH59/*` 且等待 GH-58
  implementation。该描述不是当前 corrective implementation handoff，不能再用于审计当前 diff。
- Current corrective implementation：`fix/GH59-scoped-identity-corrective` 基于 trusted
  `origin/main` `9126bf2bf5073b8243d3d42b0d93421094df12ec`；final handoff 只认 clean DCO
  implementation head 及其同 SHA receipt/evidence，不把任何历史 checkpoint/spec SHA 当
  final head。handoff 前按本文件 exact manifest 重跑 base-to-head inventory、full gates、
  both checker fixture suites 和 trusted literal collect/produce/validate flow。
- 本 spec-alignment lane only edits the three GH59 spec files and does not stage、commit、push、
  change labels、merge or resolve review threads；implementation/checker owners retain their files。
- Historical queue record中 GH-59 曾只解锁 GH-60；当前 base 已包含既有 GH-60 transaction
  implementation。此次 corrective work 不修改其 raw `Patch` topology、whole-frame terminal
  transaction、rollback/rebuild/fallback，也不据此直接解锁 GH-61 或 chat child。
- metadata/duplicate/collision/final-order/legacy ambiguity 是 GH-59 的 pre-mutation narrow
  typed errors，并经独立 checked boundary 到 App；不得污染 GH-58 TextFlow-only variants。
  public `try_diff`/`try_diff_children` 必须返回该 typed error；legacy diff wrappers 只在
  checked 成功后提交完整 patches，否则 fail loudly 且不得留下 partial destination。
  top-level root scope 由 traversal context 建立，public box/text/component roots 不走
  sibling metadata error path，legacy `diff` 对这些合法 root 不得 panic。
  成功 remove/replace/cross-parent move 必须清理完整旧 subtree maps；任一 Taffy commit
  failure 后的 rollback、失败 map 恢复、full rebuild 与通用 typed error 交给 GH-60，不能
  在实现时静默扩 scope。runtime measurement 只在成功 renderer publication 后一次性替换
  ElementId/scoped/composite/raw/string views；0/1/N lookup 与 unique -> ambiguous -> unique
  stale-state 清理属于 GH-59。raw `Patch` topology、whole-frame terminal transaction、
  rollback/rebuild/fallback 继续属于 GH-60，不得借 compatibility 修复扩张或削弱 gates。
