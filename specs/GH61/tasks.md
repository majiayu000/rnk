# Task Plan：LayoutSnapshot、Cell 量化与 producer parity

## Linked Issue

GH-61: https://github.com/majiayu000/rnk/issues/61

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Required transaction dependency: [`../GH60/product.md`](../GH60/product.md)、
  [`../GH60/tech.md`](../GH60/tech.md)、[`../GH60/tasks.md`](../GH60/tasks.md)
- Required identity dependency: [`../GH59/product.md`](../GH59/product.md)、
  [`../GH59/tech.md`](../GH59/tech.md)、[`../GH59/tasks.md`](../GH59/tasks.md)
- Required TextFlow dependency: [`../GH58/product.md`](../GH58/product.md)、
  [`../GH58/tech.md`](../GH58/tech.md)、[`../GH58/tasks.md`](../GH58/tasks.md)

## 范围说明

benchmark workload matrix、baseline artifact、promotion 流程与回归门已拆分到 #85，
不在本 packet 内实现。

## 实现任务

- [ ] `SP61-T1`（lane alias: `GH61-T1`）建立旧实现可重复失败的snapshot/parity根因fixture。Owner: `root-cause-test-lane` | Done when: 只使用GH60 merged public API的fixture证明nested fractional bounds/scroll入口仍由renderer独立解释，且full/incremental/static/testing/string尚无共同immutable snapshot；fixture在新API出现前可编译并产生预期red assertion，最终head通过 | Verify: `cargo test --test layout_snapshot_root_cause --locked nested_fractional_edges_need_one_cell_snapshot -- --exact`; `cargo test --test layout_snapshot_root_cause --locked render_entrypoints_must_share_snapshot_contract -- --exact`。
  - Dependencies: maintainer 对当前 packet/head 的明确实施确认；fresh duplicate evidence；
    三个 merged SHA ancestry gate。readiness label 只描述队列状态，不授予实施权限。
  - File ownership: 独占 `tests/layout_snapshot_root_cause.rs`；不写production、spec、
    其他tests。
  - Covers: B-001, B-004, B-005, B-012, B-016。
  - Handoff: 提交red root-cause checkpoint后把fixture所需最小public assertions交给T2；
    T1 owner停止写该文件，最终只由T6接管。

- [ ] `SP61-T2`（lane alias: `GH61-T2`）实现强制immutable snapshot types、semantic identity/index、absolute half-open quantizer、axis clip/scroll/TextFlow stamp与closed errors。Owner: `snapshot-core-lane` | Done when: snapshot/node字段private且只有crate-private checked builder能构造；public surface只有read-only accessors；aliases结构上位于PreparedSnapshotFrame；core builder只接受T3提供的ordered checked inputs，不发明GH59/GH60未声明接口、不读或改engine map/layout；x/y overflow独立组合；所有finite/range/node检查返回具体closed variant并保留source chain | Verify: `cargo test --workspace --lib --locked layout::snapshot::tests::semantic_identity_and_final_order -- --exact`; `cargo test --workspace --lib --locked layout::snapshot::quantize::tests::half_open_bounds_derive_extent_from_edges -- --exact`; `cargo test --workspace --lib --locked layout::snapshot::quantize::tests::content_border_and_gap_error_are_bounded -- --exact`; `cargo test --workspace --lib --locked layout::snapshot::tests::mixed_axis_overflow_clips_only_selected_axis -- --exact`; `cargo test --workspace --lib --locked layout::snapshot::tests::producer_report_does_not_change_semantic_equality -- --exact`; `cargo test --workspace --lib --locked layout::snapshot::tests::cancelled_builder_is_hidden_and_published_snapshot_is_immutable -- --exact`。
  - Dependencies: GH61-T1 root-cause checkpoint/handoff。
  - File ownership: 独占 `src/layout/snapshot.rs`、
    `src/layout/snapshot/error.rs`、`src/layout/snapshot/quantize.rs`、
    `src/layout/mod.rs`；不写engine、renderer、runtime、bench或integration tests。
  - Covers: B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-020, B-021。
  - Handoff: 向T3交付private-storage/read-only API、documented ordered builder input/output、
    semantic equality、closed typed error/source composition与axis clip；T2不声明任何上游
    target adapter，T2 writer停止后T3才可修改snapshot core。

- [ ] `SP61-T3`（lane alias: `GH61-T3`）把snapshot builder接入GH60 initial/incremental/recovered candidate与prepared commit，并建立全矩阵parity/state machine。Owner: `snapshot-producer-lane` | Done when: 新增并唯一拥有crate-private `SnapshotTargetPlan` adapter，只组合GH59 `final_children`、GH60 `PreparedLayoutFrame`/postcondition/checked lookup与B-014 renderer filter，不改变engine集合；initial snapshot error走GH60真实`Transaction -> Snapshot`且`rebuild_count=0`；recovered snapshot/render error aggregate保留incremental与final cause；snapshot失败无第二次rebuild且零发布；五个固定seed各64步严格使用tech SplitMix64/8 draws/权重 | Verify: `cargo test --test layout_snapshot_parity --locked full_incremental_and_recovered_are_semantically_equal -- --exact`; `cargo test --test layout_snapshot_parity --locked chat_mutation_matrix_matches_full -- --exact`; `cargo test --test layout_snapshot_state_machine --locked seeded_operations_match_after_every_step -- --exact`; `cargo test --test layout_snapshot_parity --locked resize_round_trip_restores_semantic_snapshot -- --exact`; `cargo test --test layout_snapshot_parity --locked cold_and_cached_text_flow_revisions_are_semantically_equal -- --exact`; `cargo test --test layout_snapshot_parity --locked snapshot_target_adapter_uses_gh59_order_and_gh60_lookup_contract -- --exact`; `cargo test --test layout_snapshot_parity --locked display_none_prunes_only_snapshot_render_traversal -- --exact`; `cargo test --test layout_snapshot_parity --locked nested_shared_edges_do_not_gain_overlap -- --exact`; `cargo test --test layout_snapshot_parity --locked nested_mixed_axis_overflow_matches_all_strategies -- --exact`; `cargo test --test layout_snapshot_parity --locked recovered_frame_uses_only_recovered_candidate_snapshot -- --exact`; `cargo test --test layout_snapshot_parity --locked reused_snapshot_accepts_target_exact_frame_aliases -- --exact`; `cargo test --test layout_snapshot_error_paths --locked negative_and_overflow_cells_are_not_clamped_to_success -- --exact`; `cargo test --test layout_snapshot_error_paths --locked initial_snapshot_failure_never_enters_incremental_recovery -- --exact`; `cargo test --test layout_snapshot_error_paths --locked recovered_snapshot_or_render_failure_preserves_both_causes -- --exact`; `cargo test --test layout_snapshot_error_paths --locked snapshot_failure_publishes_nothing -- --exact`; `cargo test --test layout_snapshot_error_paths --locked every_snapshot_failure_variant_preserves_payload_and_source_chain -- --exact`; `cargo test --test layout_snapshot_error_paths --locked gh60_frame_wrapper_routes_snapshot_failures_without_fictitious_initial_variant -- --exact`。
  - Dependencies: GH61-T2 concrete handoff；T2 writer停止。
  - File ownership: 接管T2 snapshot files；独占 `src/layout/engine.rs`、
    `src/layout/engine/snapshot.rs`、`src/layout/engine/transaction.rs`、
    `src/layout/engine/rebuild.rs`、`src/layout/engine/postcondition.rs`、
    `src/layout/engine/tests.rs`、`tests/layout_snapshot_parity.rs`、
    `tests/layout_snapshot_state_machine.rs`；为layout-only error fixtures初建并独占
    `tests/layout_snapshot_error_paths.rs`。不写renderer/runtime/bench。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-017, B-018, B-019, B-020, B-021, B-023。
  - Handoff: 向T4交付真实GH60 wrapper组合、prepared snapshot/report/error与current-frame
    alias API；产出 per-frame read-only deterministic work counters
    requirements；T3停止写所有
    production和integration files。

- [ ] `SP61-T4`（lane alias: `GH61-T4`）让dynamic/static/testing/string renderer、TextFlow projection与measurement只消费snapshot，并完成checked/public compatibility、compile immutability与rustdoc。Owner: `snapshot-render-lane` | Done when: renderer correctness paths不存在live engine lookup、default layout、float-to-u16或独立recursive offset；renderer消费T3 `SnapshotTargetPlan`结果，不发明上游未声明接口；GH60 whole App frame只在terminal success后提交snapshot/runtime/static state；旧surface与read-only accessor fixture编译，独立exact trybuild test无条件执行compile-fail并匹配checked-in stderr；`LayoutAliasError`/renderer error closed，并严格走GH60既有`Render(CheckedRenderError)`wrapper；新public items全部documented且exact doctest真实执行 | Verify: `cargo test --test layout_snapshot_parity --locked all_render_consumers_use_one_snapshot -- --exact`; `cargo test --test layout_snapshot_parity --locked dynamic_static_testing_and_string_share_cell_contract -- --exact`; `cargo test --test layout_snapshot_parity --locked scroll_changes_descendant_projection_only -- --exact`; `cargo test --workspace --lib --locked renderer::app::tests::snapshot_commits_only_with_prepared_app_frame -- --exact`; `cargo test --test layout_snapshot_compat --locked existing_layout_engine_renderer_and_testing_surface_compiles -- --exact`; `cargo test --test layout_snapshot_immutability --locked public_snapshot_read_only_accessors_compile -- --exact`; `cargo test --test layout_snapshot_immutability --locked public_snapshot_mutation_surface_is_compile_fail -- --exact`; `cargo test --test layout_snapshot_error_paths --locked every_layout_alias_variant_preserves_payload_and_source -- --exact`; `cargo test --test layout_snapshot_error_paths --locked gh60_frame_wrapper_routes_snapshot_failures_without_fictitious_initial_variant -- --exact`; `cargo test --test gh61_public_docs --locked gh61_public_snapshot_surface_is_documented_and_compiles -- --exact`；`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`。
  - Dependencies: GH61-T3 producer handoff；T3 writer停止。
  - File ownership: 独占 `src/renderer/mod.rs`、`src/renderer/error.rs`、
    `src/renderer/checked.rs`、`src/renderer/tree_renderer.rs`、
    `src/renderer/element_renderer.rs`、`src/renderer/pipeline.rs`,
    `src/renderer/app.rs`、`src/renderer/render_to_string.rs`、
    `src/renderer/static_content.rs`、`src/runtime/context.rs`、
    `src/testing/renderer.rs`、`src/lib.rs`、`src/prelude.rs`、
    `tests/fixtures/gh61_public_api.json`、`tests/layout_snapshot_compat.rs`、
    `tests/layout_snapshot_immutability.rs`、`tests/ui/gh61_snapshot_private_fields.rs`、
    `tests/ui/gh61_snapshot_private_fields.stderr`、`tests/gh61_public_docs.rs`；接管T3的
    `tests/layout_snapshot_error_paths.rs`只补renderer/runtime cases，不写layout files。
  - Covers: B-001, B-003, B-007, B-008, B-009, B-010, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023。
  - Handoff: 向T6交付全部renderer/public surface与error fixtures。
    不写benchmark、checker、workflow或baseline路径。

- [ ] `SP61-T6`（lane alias: `GH61-T6`）完成root-cause、compatibility、public API manifest、compile immutability、coverage、full gates与exact-head GitHub evidence。Owner: `quality-evidence-lane` | Done when: 全部invariants exact test均由tech helper证明matched=1、passed=1、ignored=0；重跑T3两条明确quantizer integration tests、五seed/64-step generator、GH60真实wrapper/recovered aggregate；public API manifest双向匹配，trybuild匹配stderr且`Cargo.toml`/`Cargo.lock`包含依赖；docs/coverage、三dependency ancestry、full Rust、current repository CI、independent review与resolved review threads绑定同一head，并取得maintainer对当前head的明确merge authorization | Verify: 重新运行T1-T4全部exact commands，包括`nested_shared_edges_do_not_gain_overlap`、`negative_and_overflow_cells_are_not_clamped_to_success`、`public_snapshot_mutation_surface_is_compile_fail`、GH60 wrapper/aggregate与seed generator tests；断言每个helper为`1 passed/0 ignored`；运行public API manifest checker、`cargo metadata --locked` trybuild断言、tech docs/coverage完整命令块及所有full commands。
  - Dependencies: GH61-T1至T4全部完成并显式handoff；T4 writers停止；
    implementation PR exact base/head已知。
  - File ownership: 接管 `tests/layout_snapshot_root_cause.rs`、
    `tests/layout_snapshot_parity.rs`、`tests/layout_snapshot_state_machine.rs`、
    `tests/layout_snapshot_error_paths.rs`、`tests/layout_snapshot_compat.rs`、
    `tests/layout_snapshot_immutability.rs`、`tests/ui/gh61_snapshot_private_fields.rs`、
    `tests/ui/gh61_snapshot_private_fields.stderr`、
    `tests/gh61_public_docs.rs`；
    production、bench、workflow与scripts只读，只允许修正tests/evidence；若生产缺陷暴露，
    退回对应owner新checkpoint，不在T6跨ownership偷改。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-029, B-030。
  - Handoff: independent reviewer必须与T1-T6 writers分离；只有current exact head的
    non-blocking review artifact、全部resolved threads、green repository CI，以及maintainer
    对同一head的明确merge authorization可进入merge step。标签和自动化状态不构成授权。

## 并行拆分

- Writable dependency graph：`T1 -> T2 -> T3 -> T4 -> T6`。
- T1只写root-cause fixture；T2只写snapshot core；T3接管snapshot并独占engine/parity。
- T6只有在T4都停止后接管全部tests。没有两个writable lane共享同一文件。
- read-only reviewer、CI观察或coverage审计可与writer并行，但不得修改source、resolve
  threads或写同一review artifact。
- 若merged upstream导致真实文件拆分不同，先更新ownership并复审spec；禁止临时共享写文件。

## 验证

- Product invariant集合与tasks `Covers:` union均精确为
  B-001..B-023、B-029、B-030；中间已拆分给GH85的五个编号不属于本packet，也不得用连续
  区间断言代替。
- planned-changes只允许GH61 packet、snapshot/quantizer、GH60 candidate接入、renderer/
  measurement迁移、明确tests/public API manifest/trybuild/coverage/public docs；benchmark、
  checker、workflow、baseline和promotion属于GH85；TextFlow算法、identity
  planner、transaction recovery策略、chat components或MessageList必须先更新spec。
- production renderer correctness paths不得出现未reviewed
  `get_layout(`、`get_all_layouts(`、`unwrap_or_default()` required layout、
  float coordinate `as u16`或第二套clip/scroll递归。
- snapshot builder不得使用`filter_map`丢required node、hash-only semantic equality、
  Taffy NodeId/ElementId-only identity、non-finite/default rect fallback或snapshot failure
  second rebuild。
- 所有filtered Rust tests先`--list --exact`且matched=1，执行输出跨workspace汇总后必须
  `passed=1`且`ignored=0`；只打印list、substring filter、零passed、ignored、旧SHA或其他
  issue test不算证据。
- coverage artifact的`pr_base_oid`、`coverage_merge_base_sha`、`head_sha`与GitHub exact
  head逐项相等；changed executable与每个critical line/branch denominator必须非零，并分别
  达到80%/100%；dependency SHAs只用于ancestry，不替代coverage base。
- public docs checker必须被直接执行并返回allowed；manifest双向、public/doctest denominator
  非零，每个doctest exact执行且非`ignore`/`no_run`。
- fresh full commands：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
```

- exact-head repository CI、public docs、coverage、independent review
  manifest/resolver map、resolved review threads与maintainer明确merge authorization必须绑定
  同一head；readiness labels仅描述状态。

## Handoff Notes

- 当前事实应从GitHub issue/PR与exact checkout重新读取，不把历史label或stacked base冻结成
  授权。implementation前由maintainer明确确认当前packet/head，重新运行fresh duplicate
  evidence并记录三个dependency merged SHAs；未合入的dependency head不解锁本issue。
- snapshot semantic equality不包含producer/recovery/timing counters；frame-local ElementId
  aliases不得污染semantic identity。
- cell合同是signed absolute half-open edges；terminal clip后才checked-convert为`u16`。
- GH-60 exactly-once recovery不可因snapshot error增加第二次rebuild；snapshot只在candidate
  内构建并随PreparedAppFrame一次提交。
- benchmark workload、checker、workflow、aggregation、baseline与promotion全部由GH85定义；
  GH61 tasks仅拥有snapshot/parity、deterministic work counters、public API manifest、
  trybuild、coverage与full verification责任。
- GH-61完成后只解锁GH-68对应dependency；不直接完成任何chat shell或message list。
