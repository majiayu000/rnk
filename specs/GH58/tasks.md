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

- [ ] `SP58-T1`（lane alias: `GH58-T1`）建立只依赖现有 public API 的可重复根因 fixture。Owner: `root-cause-test-lane` | Done when: 新增 `tests/text_flow_root_cause.rs`，只复现当前长 plain/rich 文本 measure rows 与 rendered rows 不一致；fixture 在未实现新 API 时即可编译，并在实现前产生预期 red assertion；不得引用 source metadata、TextFlow、try renderer、projection 或后续任务 API | Verify: `verify_integration_exact text_flow_root_cause measure_rows_must_equal_rendered_rows` 在实现前匹配恰好一个测试并产生预期 assertion failure，在最终实现 head 必须通过。
  - Dependencies: 实现门已开；无代码任务依赖。
  - Covers: B-001。

- [ ] `SP58-T7`（lane alias: `GH58-T7`）在不改变 Element public layout 的前提下保留 source。Owner: `text-source-lane` | Done when: `src/components/display/text.rs` 的 private `TextSourceState` 在 `str::lines()` 前保存 exact input；structured constructors 生成 canonical source；`into_element` 只写既有 `text_content` / `spans`；不修改 Element fields、不加 `#[non_exhaustive]`、不建全局 sidecar；`tests/text_source_compat.rs` 独占 exact CRLF/trailing、structured/reconstructed domain、clone 与外部完整 Element literal fixtures | Verify: `verify_integration_exact text_source_compat exact_crlf_and_trailing_break_ranges`; `verify_integration_exact text_source_compat structured_and_reconstructed_domains`; `verify_integration_exact text_source_compat external_element_struct_literal_compiles`。
  - Dependencies: GH58-T1 root-cause fixture checkpoint 已提交；不写 T1 文件。
  - Covers: B-004, B-011, B-012, B-017, B-020, B-024。

- [ ] `SP58-T2`（lane alias: `GH58-T2`）实现 logical TextFlow core 与兼容 measure helper。Owner: `text-flow-core-lane` | Done when: `src/layout/text_flow.rs` 完整产生 immutable rows/positioned safe styled runs/logical bidirectional map/dispositions/diagnostics；输入 style boundary 切入 combining/ZWJ grapheme 时先归一到首 source style 而非报错，只有 finalized token/map range 违反 grapheme boundary 才返回 typed error；hard-break/tab 先结构化消费，ESC/C0/DEL/C1 生成带原 range 的 safe replacement；`src/layout/mod.rs` 暴露必要类型；`src/layout/measure.rs` 委托同一算法；`tests/property_tests.rs` 由本 lane 独占并增加 logical map properties | Verify: `verify_lib_exact layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes`; `verify_lib_exact layout::text_flow::tests::finalized_non_grapheme_range_is_error`; `verify_lib_exact layout::text_flow::tests::text_flow_control_replacement`; `verify_integration_exact property_tests text_flow_source_cell_round_trip`。
  - Dependencies: GH58-T1 fixture checkpoint、GH58-T7 source contract。
  - Covers: B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-011, B-012, B-013, B-015, B-016, B-017, B-018, B-020, B-022。

- [ ] `SP58-T3`（lane alias: `GH58-T3`）把 LayoutEngine 测量、frame context 与 logical cache 收敛到 TextFlow。Owner: `layout-integration-lane` | Done when: direct/incremental 每帧同步当前 source/style；Taffy measure 只读 TextFlow dimensions；logical cache key 只含 source/style/width/wrap/tab/ellipsis/Unicode policy 完整值并逐值比较；完整结果原子发布；viewport/overflow/scroll/clip 状态不写入 cache | Verify: `verify_lib_exact layout::text_flow::tests::text_flow_cache_invalidation`; `verify_lib_exact layout::text_flow::tests::text_flow_cache_reuse`; `verify_lib_exact layout::engine::tests::text_flow_failure_is_atomic`。
  - Dependencies: GH58-T2。
  - Covers: B-001, B-002, B-012, B-013, B-014, B-015, B-016, B-023。

- [ ] `SP58-T4`（lane alias: `GH58-T4`）实现 grapheme-safe、terminal-safe Output trust boundary。Owner: `text-compositor-lane` | Done when: `src/renderer/output.rs` 原子写 combining/ZWJ suffix 与 wide placeholder；任何漏入的 ESC/C0/DEL/C1 在 cell/suffix 存储前按 B-022 替换；低层 `Output::write` 先结构化处理 break/tab 且不透传 controls；terminal encoder 只生成结构化 allowlisted ANSI；screen-clear、cursor move、OSC 与 C1 unit negatives 证明 payload 不可执行 | Verify: `verify_lib_exact renderer::output::tests::source_controls_are_replaced`; `verify_lib_exact renderer::output::tests::terminal_encoder_rejects_payload_sequences`。
  - Dependencies: GH58-T2、GH58-T3；不写任何 integration test 文件。
  - Covers: B-001, B-005, B-006, B-009, B-011, B-017, B-018, B-022。

- [ ] `SP58-T5`（lane alias: `GH58-T5`）收敛 renderer、frame-local projection、render-to-string 与 public typed surface。Owner: `render-parity-lane` | Done when: 新增 `try_render_element_tree` / `try_render_element` / `try_render_dynamic_frame` Result variants；现有同名非 try 函数保持原返回类型并仅作为 fail-loud compatibility wrappers，使尚未迁移的 App/static/TestRenderer callers 在 T5 exact commit 仍编译；tree/element try renderer 只消费 logical runs并构建不缓存的 RenderProjection；dynamic try pipeline 不提交 partial frame；`TextRenderError`、`try_render_to_string*` 与 exports 完成；本 lane 独占 parity/prelude tests | Verify: `cargo check --workspace --all-targets --all-features --locked`; `verify_integration_exact text_flow_parity measure_rows_equal_rendered_rows`; `verify_integration_exact text_flow_parity viewport_projection_tracks_overflow_scroll_and_clip`; `verify_integration_exact text_flow_parity resize_reflows_or_reprojects_before_render`; `verify_integration_exact prelude_surfaces try_render_to_string_surface`。
  - Dependencies: GH58-T3、GH58-T4；不写 T1/T2/T7/T8 tests。
  - Covers: B-001, B-002, B-005, B-007, B-008, B-009, B-010, B-011, B-013, B-014, B-015, B-017, B-018, B-021, B-022, B-023。

- [ ] `SP58-T8`（lane alias: `GH58-T8`）把剩余 callers 从 T5 compatibility wrappers 迁到 recoverable try boundary。Owner: `render-error-lane` | Done when: static content 新增 `try_extract_static_content` 并保留原 `extract_static_content` 为 fail-loud wrapper；App、static 内部、TerminalController 与 TestRenderer 显式调用 T5/T8 try variants 并传播同一 `TextRenderError` cause；App 保留 `io::Error` source；失败不提交 partial terminal/static output；本 lane独占 `tests/text_flow_error_paths.rs`，不修改其他 fixtures；T8 exact commit 可独立编译测试 | Verify: `cargo check --workspace --all-targets --all-features --locked`; `verify_integration_exact text_flow_error_paths typed_error_reaches_all_render_entrypoints`; `verify_integration_exact text_flow_error_paths failure_commits_no_partial_output`。
  - Dependencies: GH58-T5。
  - Covers: B-001, B-015, B-016, B-017, B-021。

- [ ] `SP58-T6`（lane alias: `GH58-T6`）完成当前 head 验证、覆盖率和 SpecRail handoff。Owner: `verification-lane` | Done when: exact implementation head 通过 fmt/check/clippy/all-target tests；所有 filtered evidence 仅经 exact helpers 且恰好匹配一个；CodeCov patch >=80%，TextFlow core 关键路径 100%；独立 review artifact、全部 review threads、fresh CI 与 SpecRail PR gate green，证据 SHA 等于 PR head | Verify: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --all-features --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`; `cargo test --workspace --all-targets --all-features --locked`；核对当前 head coverage、CI、reviewThreads 与 PR gate JSON。
  - Dependencies: GH58-T1、GH58-T2、GH58-T3、GH58-T4、GH58-T5、GH58-T7、GH58-T8 全部完成；独立 reviewer 与 implementer 分离。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024。

## 并行拆分

- GH58-T1 独占 `tests/text_flow_root_cause.rs`，只建立现有 API root-cause checkpoint。
- GH58-T7 独占 `src/components/display/text.rs`、`tests/text_source_compat.rs`。
- GH58-T2 独占 `src/layout/text_flow.rs`、`src/layout/mod.rs`、`src/layout/measure.rs`、
  `tests/property_tests.rs`，在 T7 source contract 稳定后开始。
- GH58-T3 独占 `src/layout/engine.rs`。
- GH58-T4 独占 `src/renderer/output.rs`。
- GH58-T5 独占 `src/renderer/error.rs`、`src/renderer/mod.rs`、
  `src/renderer/tree_renderer.rs`、`src/renderer/element_renderer.rs`、
  `src/renderer/pipeline.rs`、`src/renderer/render_to_string.rs`、`src/lib.rs`、
  `src/prelude.rs`、`tests/text_flow_parity.rs`、`tests/prelude_surfaces.rs`。
- GH58-T8 独占 `src/renderer/app.rs`、`src/renderer/static_content.rs`、
  `src/renderer/terminal_controller.rs`、`src/testing/renderer.rs`、
  `tests/text_flow_error_paths.rs`。
- GH58-T6 为只读 verification/review lane。上述 ownership 全程不转移；任务依赖图为
  T1 checkpoint -> T7 -> T2 -> T3 -> T4 -> T5 -> T8 -> T6，无 cycle、无共享 writable file。

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
