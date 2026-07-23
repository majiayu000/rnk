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

所有 cargo filter 必须先用 `-- --list` 断言至少匹配一个 test；零匹配视为失败。

## 实现任务

- [ ] `SP58-T1`（lane alias: `GH58-T1`）建立可重复根因与合同测试。Owner: `text-flow-test-lane` | Done when: `tests/text_flow_parity.rs` 先证明旧路径在长 plain/rich 文本上 measure rows 与 rendered rows 不一致，再锁定新合同；`tests/property_tests.rs` 增加 grapheme/source-cell properties，fixtures 覆盖 LF/CRLF/CR、tab、CJK、emoji ZWJ、combining、width=0/1、五种 TextWrap 与三种 Overflow | Verify: `matched=$(cargo test --test text_flow_parity --locked -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --test text_flow_parity --locked`; `matched=$(cargo test --test property_tests --locked text_flow_ -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --test property_tests --locked text_flow_`。
  - Dependencies: 实现门已开；无代码任务依赖。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-014, B-018。

- [ ] `SP58-T2`（lane alias: `GH58-T2`）实现纯 TextFlow core 与兼容 measure helper。Owner: `text-flow-core-lane` | Done when: `src/layout/text_flow.rs` 完整产生 immutable rows/positioned styled runs/bidirectional source map/dispositions/diagnostics；`src/layout/mod.rs` 暴露唯一必要类型；现有 measure helper 委托同一 segmentation/wrap/truncate 语义且无第二套算法 | Verify: `matched=$(cargo test --workspace --lib --locked text_flow_ -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --workspace --lib --locked text_flow_`; `matched=$(cargo test --workspace --lib --locked layout::measure::tests -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --workspace --lib --locked layout::measure::tests`。
  - Dependencies: GH58-T1 的失败 fixtures 已可重复。
  - Covers: B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-015, B-016, B-017, B-018。

- [ ] `SP58-T3`（lane alias: `GH58-T3`）把 LayoutEngine 测量、frame context 与 cache 收敛到 TextFlow。Owner: `layout-integration-lane` | Done when: plain/rich exact source 在 direct 与 incremental 路径逐帧同步；Taffy measure 只读 TextFlow dimensions；完整结果原子发布；content/span/style/width/wrap/overflow/tab/ellipsis/policy 任一变化 miss，完全相同输入 hit；失败不发布 partial/stale flow | Verify: `matched=$(cargo test --workspace --lib --locked text_flow_cache -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --workspace --lib --locked text_flow_cache`; `matched=$(cargo test --workspace --lib --locked text_flow_failure_is_atomic -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --workspace --lib --locked text_flow_failure_is_atomic`。
  - Dependencies: GH58-T2。
  - Covers: B-001, B-002, B-012, B-013, B-014, B-015, B-016。

- [ ] `SP58-T4`（lane alias: `GH58-T4`）实现 grapheme-safe Output 写入并让 tree renderer 只消费 positioned runs。Owner: `text-compositor-lane` | Done when: Output 可原子写入带 combining/ZWJ suffix 的 grapheme 与 wide placeholder；overwrite/clip/terminal edge 无残留或越界；tree renderer 删除 plain/spans 自有布局分支，flow 缺失时不走 legacy first-line fallback；嵌入 ANSI 不被解释为 Style | Verify: `matched=$(cargo test --workspace --lib --locked renderer::output::tests -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --workspace --lib --locked renderer::output::tests`; `cargo test --test text_flow_parity --locked`。
  - Dependencies: GH58-T2、GH58-T3；文件所有权不得与其他 writable lane 重叠。
  - Covers: B-001, B-002, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-015, B-017, B-018。

- [ ] `SP58-T5`（lane alias: `GH58-T5`）收敛 render-to-string 与兼容路径。Owner: `render-parity-lane` | Done when: render-to-string 不再自行统计 wrapped lines；direct compute、dynamic incremental 和 render-to-string 对同一 Element/width 的 row count 与输出等价；现有 public surface 继续编译 | Verify: `matched=$(cargo test --workspace --lib --locked render_to_string -- --list | awk '/: test$/{n++} END{print n+0}'); test "$matched" -gt 0 && cargo test --workspace --lib --locked render_to_string`; `cargo check --workspace --all-targets --all-features --locked`。
  - Dependencies: GH58-T3、GH58-T4。
  - Covers: B-001, B-004, B-007, B-008, B-013, B-014, B-017。

- [ ] `SP58-T6`（lane alias: `GH58-T6`）完成当前 head 验证、覆盖率和 SpecRail handoff。Owner: `verification-lane` | Done when: exact implementation head 通过 fmt/check/clippy/all-target tests；所有 filter 非零；CodeCov patch >=80%，TextFlow core 关键路径 100%；独立 review artifact、全部 review threads、fresh CI 与 SpecRail PR gate 均为 green，证据 SHA 等于 PR head | Verify: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --all-features --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`; `cargo test --workspace --all-targets --all-features --locked`；核对当前 head 的 coverage、CI、reviewThreads 与 PR gate JSON。
  - Dependencies: GH58-T1 至 GH58-T5 全部完成；独立 reviewer 与 implementer 分离。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019。

## 并行拆分

- GH58-T1 独占 `tests/text_flow_parity.rs`、`tests/property_tests.rs`；GH58-T2 独占
  `src/layout/text_flow.rs`、`src/layout/mod.rs`。两者只在 T1 复现提交后并行继续。
- GH58-T3 独占 `src/layout/engine.rs`。
- GH58-T4 独占 `src/renderer/output.rs`、`src/renderer/tree_renderer.rs`。
- GH58-T5 独占 `src/renderer/render_to_string.rs`，必须在 T3/T4 API 稳定后开始。
- GH58-T6 为只读验证/review lane，不与 implementer 共享可写文件。任何所有权变化都先更新
  tranche checkpoint；禁止两个 lane 同时写同一文件。

## 验证

- Product invariant 集合与 tasks `Covers:` union 均为 B-001 至 B-019，无遗漏。
- planned-changes manifest 只允许本 packet、TextFlow/layout/renderer 集成和两个测试文件；
      出现 VNode/reconciler/chat/workflow 改动即阻断并重新 spec。
- 所有带 filter 的 cargo test 先 `-- --list` 且匹配数大于 0。
- fresh fmt/check/clippy/all-target tests、coverage、CI、独立 review、reviewThreads 与
      SpecRail gate 均绑定 implementation PR exact head。

## Handoff Notes

- GH-58 无实现前置依赖，但 GH-59、GH-64、GH-65 的最终实现/验收依赖本 issue 完成。
- 当前只完成 spec packet；`ready_to_spec` 不是实现门。implx auto 可按当前 invocation 的
  auth mode 自动审查并设置 `ready_to_implement`，但不能绕过 duplicate evidence、CI、
  independent reviewer lane、reviewThreads 或 PR gate。
- `TextFlow` 是唯一 text layout source；后续 child 只能消费 rows/source map，不能复制
  grapheme/wrap/width 算法。
- 如果实现需要触碰 manifest 外路径、改变 trailing hard-break 合同或修改
  `src/core/vnode.rs` / `src/reconciler/*`，先更新 GH-58 tech spec 并重新过 spec review，
  不得静默扩 scope。
