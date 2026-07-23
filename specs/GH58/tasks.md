# Task Plan：统一终端文本测量与绘制流

## Linked Issue

GH-58: https://github.com/majiayu000/rnk/issues/58

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Umbrella context: GH-57（仅父范围；GH-58 的验收以本 packet 为准）

## 实现门

当前 durable issue state 是 `ready_to_spec`。本 packet 通过独立 spec review，并由队列
coordinator 按当前 SpecRail auth mode 设置 canonical `ready_to_implement` 前，以下任务均
不得修改生产代码。实现还必须先保存 fresh duplicate-work evidence，并确认没有已有 GH-58
implementation PR/branch 可继续。

所有 filtered cargo test 只能调用以下 helper；helper 必须先 `--list --exact` 并断言恰好
一个匹配，禁止直接执行未守卫 filter：

```sh
verify_lib_exact() {
  test_name="$1"
  matched="$(
    cargo test --workspace --lib --locked "$test_name" -- --list --exact |
      awk '/: test$/{count++} END{print count+0}'
  )"
  test "$matched" -eq 1 || return 1
  cargo test --workspace --lib --locked "$test_name" -- --exact
}

verify_integration_exact() {
  target="$1"
  test_name="$2"
  matched="$(
    cargo test --test "$target" --locked "$test_name" -- --list --exact |
      awk '/: test$/{count++} END{print count+0}'
  )"
  test "$matched" -eq 1 || return 1
  cargo test --test "$target" --locked "$test_name" -- --exact
}
```

## 实现任务

- [ ] `SP58-T1`（lane alias: `GH58-T1`）在隔离 scratch worktree 中建立只依赖现有 public
  API 的可重复根因 fixture。Owner: `root-cause-test-lane` | Done when: 临时
  `tests/text_flow_root_cause.rs` 只复现当前长 plain/rich 文本 measure rows 与 rendered rows
  不一致；fixture 在未实现新 API 时可编译且产生预期 assertion failure；保存 exact base
  SHA、fixture patch digest 与失败命令摘要到外部 artifact 后恢复 scratch worktree，目标
  PR 分支不得提交红测或其他改动；最终 fixture 由 GH58-T5 与首个使其转绿的 renderer
  集成原子提交 | Verify: 在 scratch worktree 中
  `verify_integration_exact text_flow_root_cause measure_rows_must_equal_rendered_rows`
  恰好匹配一个测试并产生预期 assertion failure；`git status --short` 随后为空。
  - Dependencies: 实现门已开；无代码任务依赖。
  - Covers: B-001。

- [ ] `SP58-T7`（lane alias: `GH58-T7`）在不改变 Element public layout 的前提下保留 source。Owner: `text-source-lane` | Done when: `src/components/display/text.rs` 的 private `TextSourceState` 在 `str::lines()` 前保存 exact input；structured constructors 生成 canonical source；`into_element` 只写既有 `text_content` / `spans`；T7 checkpoint 保留 `Text::new` 的 normalized multiline spans 作为 legacy renderer compatibility view，确保 TextFlow renderer 尚未落地时 `"a\nb"` 仍显示两行；不修改 Element fields、不加 `#[non_exhaustive]`、不建全局 sidecar；`tests/text_source_compat.rs` 独占 exact CRLF/trailing、structured source、multiline compatibility、clone 与外部完整 Element literal fixtures，不断言 T3 才能发布的 Reconstructed diagnostic | Verify: `verify_integration_exact text_source_compat exact_crlf_and_trailing_break_ranges`; `verify_integration_exact text_source_compat structured_source_domain`; `verify_integration_exact text_source_compat plain_multiline_compatibility`; `verify_integration_exact text_source_compat external_element_struct_literal_compiles`。
  - Dependencies: GH58-T1 root-cause artifact 已记录；目标分支保持全绿。
  - Covers: B-004, B-017, B-020, B-024。

- [ ] `SP58-T2`（lane alias: `GH58-T2`）实现 logical TextFlow core 与兼容 measure helper。Owner: `text-flow-core-lane` | Done when: `src/layout/text_flow.rs` 完整产生 immutable rows/positioned safe styled runs/logical bidirectional map/dispositions/diagnostics；输入 style boundary 切入 combining/ZWJ grapheme 时先归一到首 source style 而非报错，只有 finalized token/map range 违反 grapheme boundary 才返回 typed error；把 T7 legacy multiline compatibility view 按可见 grapheme/hard-break 顺序对齐 exact source；hard-break/tab 先结构化消费，ESC/C0/DEL/C1 生成带原 range 的 safe replacement；`src/layout/mod.rs` 暴露必要类型；`src/layout/measure.rs` 委托同一算法；本 lane 在 `src/layout/text_flow.rs` 创建并修复 shared-result、cache invalidation 与 cache reuse unit tests；`tests/property_tests.rs` 由本 lane 独占并只增加 logical map properties | Verify: `verify_lib_exact layout::text_flow::tests::text_flow_shared_result`; `verify_lib_exact layout::text_flow::tests::text_flow_cache_invalidation`; `verify_lib_exact layout::text_flow::tests::text_flow_cache_reuse`; `verify_lib_exact layout::text_flow::tests::text_flow_styled_runs`; `verify_lib_exact layout::text_flow::tests::text_flow_empty_inputs`; `verify_lib_exact layout::text_flow::tests::text_flow_graphemes`; `verify_lib_exact layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes`; `verify_lib_exact layout::text_flow::tests::finalized_non_grapheme_range_is_error`; `verify_lib_exact layout::text_flow::tests::text_flow_control_replacement`; `verify_lib_exact layout::text_flow::tests::text_flow_tabs`; `verify_lib_exact layout::text_flow::tests::text_flow_wrap`; `verify_lib_exact layout::text_flow::tests::text_flow_truncate`; `verify_lib_exact layout::text_flow::tests::text_flow_narrow_width`; `verify_lib_exact layout::text_flow::tests::text_flow_interruption`; `verify_integration_exact property_tests text_flow_logical_source_round_trip`。
  - Dependencies: GH58-T1 root-cause artifact、GH58-T7 source contract。
  - Covers: B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-011, B-012, B-013, B-015, B-016, B-017, B-018, B-020, B-022。

- [ ] `SP58-T3`（lane alias: `GH58-T3`）把 LayoutEngine 测量、frame context 与 logical cache 收敛到 TextFlow。Owner: `layout-integration-lane` | Done when: `src/layout/engine/text_flow_bridge.rs` 是从 `engine.rs` 拆出的专用桥接模块，不得靠 `#[rustfmt::skip]`、压缩既有逻辑或把 `engine.rs` 卡在 800 行规避拆分；`src/layout/engine/tests.rs` 只机械迁入既有 engine unit tests 和承载 T3 新 exact gates，不改变既有测试语义、不触碰 GH-59/GH-60、不得弱化断言；本 lane 拥有并实现返回 `TextFlowError` 的 `try_compute`、`try_compute_element_incremental`、`try_compute_vnode` entrypoints，旧 `compute*` wrappers 只委托对应 try entrypoint 并在 Err 时 fail loudly；direct/incremental 每帧同步当前 source/style，即使 incremental diff 返回空 patch 集合也必须分别识别 source-only 与 span-style-only 变化并刷新 NodeContext/current flow/cache identity；普通 Element/VNode 的 element-level Style 必须进入 TextFlow；legacy normalized spans 必须按 visible grapheme/hard-break 序列对齐 exact CRLF、CR 与 trailing source ranges，只有确实无法完整、无歧义对齐时才发布 Reconstructed diagnostic 且仍以 `text_content` 为 source truth；Taffy measure 只读 TextFlow dimensions；logical cache key 只含 source/style/width/wrap/`overflow_x/y`/tab/ellipsis/Unicode policy 完整值并逐值比较；overflow-only 变化也使 flow cache miss；layout 成功后必须基于最终 content width 原子发布 current flow，即使 known dimensions 让 Taffy 不调用 measure callback，也不得静默缺失或保留错误宽度；viewport height/scroll/content rect/clip/terminal bounds 不写入 cache；T2-owned cache tests 仅作为 dependency regression gates，T3 无权修改 `src/layout/text_flow.rs` | Verify: `verify_lib_exact layout::text_flow::tests::text_flow_cache_invalidation`; `verify_lib_exact layout::text_flow::tests::text_flow_cache_reuse`; `verify_lib_exact layout::engine::tests::incremental_no_patch_refreshes_source_and_style`; `verify_lib_exact layout::engine::tests::plain_text_style_is_published`; `verify_lib_exact layout::engine::tests::alignable_crlf_spans_keep_exact_source_domain`; `verify_lib_exact layout::engine::tests::known_dimensions_publish_final_width_flow`; `verify_lib_exact layout::engine::tests::reconstructed_source_domain_uses_text_content_truth`; `verify_lib_exact layout::engine::tests::text_flow_failure_is_atomic`; `verify_lib_exact layout::engine::tests::try_compute_entrypoints_return_text_flow_error`。
  - Dependencies: GH58-T2。
  - Covers: B-001, B-002, B-012, B-013, B-014, B-015, B-016, B-020, B-021, B-023。

- [ ] `SP58-T4`（lane alias: `GH58-T4`）实现 grapheme-safe、terminal-safe Output trust boundary。Owner: `text-compositor-lane` | Done when: `src/renderer/output.rs` 原子写 combining/ZWJ suffix 与 wide placeholder；`src/renderer/output/tests.rs` 只机械迁入既有 Output unit tests 并承载 T4 新 exact gates，不改变或弱化既有测试语义、不用 `#[rustfmt::skip]` 或压缩旧实现把 `output.rs` 卡在 800 行、不引入 integration fixtures 或 T5 renderer 行为；任何漏入的 ESC/C0/DEL/C1 在 cell/suffix 存储前按 B-022 替换；低层 `Output::write` 先结构化处理 break/tab 且不透传 controls；terminal encoder 只生成结构化 allowlisted ANSI；screen-clear、cursor move、OSC 与 C1 unit negatives 证明 payload 不可执行；新增 crate-private 只读 whole-EGC active-clip visibility query，必须同时考虑 terminal bounds 与完整 active clip stack，不修改 Output/clip state，wide grapheme 只有所有 display cells 可见才返回 visible | Verify: `verify_lib_exact renderer::output::tests::source_controls_are_replaced`; `verify_lib_exact renderer::output::tests::terminal_encoder_rejects_payload_sequences`; `verify_lib_exact renderer::output::tests::active_clips_report_grapheme_visibility`。
  - Dependencies: GH58-T2、GH58-T3；不写任何 integration test 文件。
  - Covers: B-001, B-005, B-006, B-009, B-011, B-017, B-018, B-022。

- [ ] `SP58-T5`（lane alias: `GH58-T5`）收敛 renderer、frame-local projection、render-to-string 与 public typed surface。Owner: `render-parity-lane` | Done when: 新增 `try_render_element_tree` / `try_render_element` / `try_render_dynamic_frame` Result variants；现有同名非 try 函数保持原返回类型并仅作为 fail-loud compatibility wrappers，使尚未迁移的 App/static/TestRenderer callers 在 T5 exact commit 仍编译；tree/element try renderer 只消费 logical runs，并在任何 Output mutation 前构建和验证 private、frame-local、immutable 的完整 `RenderProjection` 双向 map；实现可且应拆入 `src/renderer/tree_renderer/projection.rs`，该文件仍归 T5 独占；正向 map 从 source range/完整 grapheme/logical disposition 到 visible/clipped cells，反向 map 从每个 occupied `(x,y)` cell 到 source 或 synthetic ellipsis；tab 展开 cells 共享同一 source range，多 EGC ellipsis 逐 EGC synthetic，wide grapheme 全宽原子，hidden/hard-break/zero-width 保留无-cell disposition，gap/重叠/缺项均在写 Output 前 typed failure；投影必须同时考虑当前 Text 自身 content rect/`overflow_x/y`/scroll、祖先 clips、terminal bounds 与调用前已有 Output active clips，并调用 T4 whole-EGC visibility query；x/y overflow 独立，任一轴 Hidden/Scroll 不得裁另一轴，所有 origin/offset/scroll/run/terminal 坐标全程 signed，左/上负坐标禁止 clamp 到 0；projection exact gate 必须覆盖 x-visible+y-hidden、x-hidden+y-visible 和向左/向上滚出 terminal；overflow-only 变化先取得新的 flow 再投影；dynamic try pipeline 不提交 partial frame 或更新 previous VNode，layout/render Err 必须 invalidate/reset 已可能推进的 LayoutEngine incremental tree，使下次调用从当前 Element/VNode 强制 full rebuild，同时 previous_vnode/runtime context 保持最后成功帧；clean-retry exact gate 必须分别覆盖 NaN child layout Err 与 layout 成功后通过 private test-only renderer seam/closure 注入 `MissingCurrentFlow` 的 render Err，两者修正同 child 后都从 clean tree 重建且节点不重复、layout/measure/alias/VNode/runtime 正确；private seam 只进入 pipeline 内部共享实现，不扩 public API，该恢复也不扩成 GH-60 通用 patch transaction/rollback；`TextRenderError`、`try_render_to_string*` 与 exports 完成且失败不返回 partial String；从 GH58-T1 artifact 恢复 `tests/text_flow_root_cause.rs`，与首个使它通过的 renderer 集成原子提交；本 lane 独占 root-cause/parity/prelude 及 `tests/text_flow_renderer_error_paths.rs`；crate-private tree renderer Output/source-chain negative 必须放在 `tree_renderer.rs` 的 `#[cfg(test)]`，projection 双向 map/gap negative 必须放在 `tree_renderer/projection.rs` 的 `#[cfg(test)]`，pipeline/previous VNode/clean retry negatives 必须放在 `pipeline.rs` 的 `#[cfg(test)]`，integration 文件只能调用 public `try_render_to_string*` 并断言 source chain/无 partial String | Verify: `cargo check --workspace --all-targets --all-features --locked`; `verify_lib_exact renderer::output::tests::active_clips_report_grapheme_visibility`; `verify_lib_exact renderer::tree_renderer::projection::tests::projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells`; `verify_integration_exact text_flow_root_cause measure_rows_must_equal_rendered_rows`; `verify_integration_exact text_flow_parity measure_rows_equal_rendered_rows`; `verify_integration_exact text_flow_parity unicode_graphemes_render_intact`; `verify_integration_exact text_flow_parity source_controls_are_not_terminal_sequences`; `verify_integration_exact text_flow_parity projection_source_cell_round_trip`; `verify_integration_exact text_flow_parity viewport_projection_tracks_overflow_scroll_and_clip`; `verify_integration_exact text_flow_parity overflow_change_recomputes_flow_and_projection`; `verify_integration_exact text_flow_parity resize_reflows_or_reprojects_before_render`; `verify_lib_exact renderer::tree_renderer::tests::text_flow_error_preserves_source_and_commits_no_partial_output`; `verify_lib_exact renderer::pipeline::tests::text_flow_error_keeps_previous_vnode`; `verify_lib_exact renderer::pipeline::tests::incremental_failure_retries_from_clean_layout_tree`; `verify_integration_exact text_flow_renderer_error_paths try_render_to_string_preserves_source_and_returns_no_partial_string`; `verify_integration_exact prelude_surfaces try_render_to_string_surface`。
  - Dependencies: GH58-T3、GH58-T4；允许且要求从 GH58-T1 artifact 恢复并提交 `tests/text_flow_root_cause.rs`；不写 T2/T7/T8 tests。
  - Covers: B-001, B-002, B-005, B-007, B-008, B-009, B-010, B-011, B-013, B-014, B-015, B-017, B-018, B-021, B-022, B-023。

- [ ] `SP58-T8`（lane alias: `GH58-T8`）把剩余 callers 从 T5 compatibility wrappers 迁到 recoverable try boundary。Owner: `render-error-lane` | Done when: static content 新增 `try_extract_static_content` 并保留原 `extract_static_content` 为 fail-loud wrapper；App、static 内部、TerminalController 与 TestRenderer 显式调用 T5/T8 try variants 并传播同一 `TextRenderError` cause；App 保留 `io::Error` source；失败不提交 partial terminal/static output；本 lane 独占 `tests/text_flow_error_paths.rs`，只增加 App/static/TerminalController/TestRenderer caller fixtures，不修改或补救 T5-owned source-module unit negatives 或 public string integration fixture；T8 exact commit 可独立编译测试 | Verify: `cargo check --workspace --all-targets --all-features --locked`; `verify_integration_exact text_flow_error_paths typed_error_reaches_remaining_callers`; `verify_integration_exact text_flow_error_paths caller_failure_commits_no_partial_output`。
  - Dependencies: GH58-T5。
  - Covers: B-001, B-015, B-016, B-017, B-021。

- [ ] `SP58-T6`（lane alias: `GH58-T6`）完成当前 head 验证、覆盖率和 SpecRail handoff。Owner: `verification-lane` | Done when: exact implementation head 通过 fmt/check/clippy/all-target tests；所有 filtered evidence 仅经 exact helpers 且恰好匹配一个；CodeCov patch >=80%，TextFlow core 关键路径 100%；独立 review artifact、全部 review threads、fresh CI 与 SpecRail PR gate green，证据 SHA 等于 PR head | Verify: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --all-features --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`; `cargo test --workspace --all-targets --all-features --locked`；核对当前 head coverage、CI、reviewThreads 与 PR gate JSON。
  - Dependencies: GH58-T1、GH58-T2、GH58-T3、GH58-T4、GH58-T5、GH58-T7、GH58-T8 全部完成；独立 reviewer 与 implementer 分离。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024。

## 并行拆分

- GH58-T1 只在隔离 scratch worktree 生成 root-cause patch/evidence artifact，不向目标分支
  提交文件；GH58-T5 独占并原子提交最终绿色的 `tests/text_flow_root_cause.rs`。
- GH58-T7 独占 `src/components/display/text.rs`、`tests/text_source_compat.rs`。
- GH58-T2 独占 `src/layout/text_flow.rs`、`src/layout/mod.rs`、`src/layout/measure.rs`、
  `tests/property_tests.rs`，在 T7 source contract 稳定后开始。
- GH58-T3 独占 `src/layout/engine.rs`、`src/layout/engine/text_flow_bridge.rs`、
  `src/layout/engine/tests.rs`；bridge 是从 `engine.rs` 拆出的专用桥接模块，不得用
  `#[rustfmt::skip]` 或压缩旧逻辑代替拆分；tests 只机械迁出既有 unit tests 并增加 T3
  exact gates，不改变既有测试语义、不触碰 GH-59/GH-60、不得弱化断言。
- GH58-T4 独占 `src/renderer/output.rs`、`src/renderer/output/tests.rs`；tests 只机械迁出
  既有 Output unit tests 并增加 T4 exact gates，不改变或弱化既有测试语义、不用
  `#[rustfmt::skip]` 或压缩旧实现卡 800 行、不引入 integration fixtures 或 T5 行为；
  T4 必须先交付 crate-private whole-EGC active-clip visibility query，T5 才能开始 projection。
- GH58-T5 独占 `src/renderer/error.rs`、`src/renderer/mod.rs`、
  `src/renderer/tree_renderer.rs`、`src/renderer/tree_renderer/projection.rs`、
  `src/renderer/element_renderer.rs`、
  `src/renderer/pipeline.rs`、`src/renderer/render_to_string.rs`、`src/lib.rs`、
  `src/prelude.rs`、`tests/text_flow_root_cause.rs`、`tests/text_flow_parity.rs`、
  `tests/text_flow_renderer_error_paths.rs`、`tests/prelude_surfaces.rs`。
- GH58-T8 独占 `src/renderer/app.rs`、`src/renderer/static_content.rs`、
  `src/renderer/terminal_controller.rs`、`src/testing/renderer.rs`、
  `tests/text_flow_error_paths.rs`。
- GH58-T6 为只读 verification/review lane。上述 ownership 全程不转移；任务依赖图为
  T1 evidence -> T7 -> T2 -> T3 -> T4 -> T5 -> T8 -> T6，无 cycle、无共享 writable file；
  每个写入目标分支的 task checkpoint 都必须先通过其声明的 focused verification，禁止提交
  已知红测后继续生产代码。

## 验证

- Product invariant 集合与 tasks `Covers:` union 均为 B-001 至 B-024，无遗漏。
- planned-changes manifest 只允许本 packet、private Text source、TextFlow/layout/
  renderer/projection/typed 集成和明确列出的测试文件；Element layout、VNode、reconciler、
  chat 或 workflow 改动即阻断并重新 spec。
- 所有 filtered cargo test 只能走 `verify_lib_exact` / `verify_integration_exact`；
  `--list --exact` 匹配数不为 1 即失败。
- fresh fmt/check/clippy/all-target tests、coverage、CI、独立 review、reviewThreads 与
  SpecRail gate 均绑定 implementation PR exact head。

## Handoff Notes

- GH-58 无实现前置依赖，但 GH-59、GH-64、GH-65 的最终实现/验收依赖本 issue 完成。
- 当前只完成 spec packet；`ready_to_spec` 不是实现门。implx auto 可按当前 invocation 的
  auth mode 自动审查并设置 `ready_to_implement`，但不能绕过 duplicate evidence、CI、
  independent reviewer lane、reviewThreads 或 PR gate。
- `TextFlow` 是唯一 logical text layout source；visible/clipped 状态只属于 frame projection。
- Element public layout 保持不变；若实现需要 Element 新字段、`#[non_exhaustive]`、全局
  source sidecar、manifest 外路径或改变 source/control/projection/typed error 合同，必须先
  更新 GH-58 specs 并重新过独立 spec review。
