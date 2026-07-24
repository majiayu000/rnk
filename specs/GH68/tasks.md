# Task Plan：聊天 Examples 收敛与产品级 Hardening 证据

## Linked Issue

GH-68: https://github.com/majiayu000/rnk/issues/68

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## 当前实现门

当前 durable issue state 是 `ready_to_spec`，不是 `ready_to_implement`。本 packet 获得人工
spec approval、#68 取得 canonical `ready_to_implement`、并且 GH-61、GH-66、GH-67 的
最终 implementation PR 全部 merged 之前，以下任务均不得修改 implementation paths。

实现 owner 必须用 [`tech.md`](tech.md) 的 executable preflight adapter fresh 生成并验证
initial/final phase artifact，不得以手工 `GH*_MERGE_SHA` 或 approval 环境变量替代。
PR #69、PR #78 或任何其他 parked/draft/spec-only PR 不能满足此门禁。若 merged 上游的真实
API/path 与本 packet 不一致，先更新 product/tech/tasks 并重新人工批准，禁止通过私有
fallback、alias 或复制旧分支代码继续实现。

所有 filtered Rust tests 都以 `-- --exact` 实际运行且非 `#[ignore]`。宽泛 workspace
测试不能替代 mapped exact test。

<!-- gh68-critical-paths-v1
{"version":1,"issue":68,"critical_paths":[{"file":"tests/golden_real_apps.rs","name":"gh68_harness_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_chat_tutorial_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_fullscreen_example_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_inline_example_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_provider_example_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_example_convergence_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_example_index_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_message_compatibility_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_public_docs_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_compatibility_matrix_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_stress_correctness_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_benchmark_metadata_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_benchmark_comparison_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_current_head_coverage_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_ci_public_examples_contract"}]}
-->

## Implementation Tasks

- [ ] `SP68-T1`（lane alias: `GH68-T1`）执行 dependency、authorization、duplicate-work 与
  implementation-head preflight。Owner: `integration-gate-owner` | Done when: 使用 tech 的
  `capture_gh68_preflight initial "$GH68_EVIDENCE_DIR/preflight-initial.json"` 与 initial
  validator fresh 证明 dependency issue/closing implementation PR、spec PR approval 与
  readiness 全部通过；initial 允许 clean head 等于 fresh main 或 base 为其祖先，不创建伪空
  commit。fresh duplicate search 不存在覆盖同一 scope 的 open issue/PR。任一条件缺失时
  blocked 且不进入 T2 | Verify: 执行 tech T1 invocation；运行可信 SpecRail duplicate
  collector 并保存 external JSON；重新运行
  `validate_gh68_preflight initial "$GH68_EVIDENCE_DIR/preflight-initial.json"`。
  - Dependencies: 本 GH68 packet 人工批准；canonical `ready_to_implement`。
  - File ownership: read-only；不写任何 repository file。
  - Covers: B-021, B-026, B-028。
  - Handoff: 向 T2 提交 initial artifact、duplicate evidence 与 merged GH61/GH66/GH67
    public API/path inventory；initial 只授权开工，不是 current-head completion evidence。

- [ ] `SP68-T2`（lane alias: `GH68-T2`）建立 GH68 root-cause、golden、adapter、interaction、
  recovery、docs/CI/benchmark schema 与负例合同。Owner: `hardening-contract-test-owner` |
  Done when: 只建立 reusable harness、deterministic fixtures、schema parsers 与负例；通过
  scratch-only `$GH68_EVIDENCE_DIR/root-cause/` 记录 private example state、legacy-only
  golden、缺 docs/matrix/gate/benchmark metadata 等 baseline root cause，不在 committed
  repository 留红测。`gh68_harness_contract` 在当前 baseline 与提交后均绿色，且缺分类、空
  evidence、伪 verified、placeholder key、Unknown 自动重试、ignored exact test、旧 SHA、
  环境不匹配、只编译 benchmark 等负例 fail closed；不得提前创建或宣称 T3–T9 的 14 个未来
  exact tests 通过 | Verify: `cargo test --test gh68_harness_contract --locked -- --exact`；
  `cargo test --test golden_real_apps --all-features --locked`；
  `git diff --check -- tests/golden_real_apps.rs`；检查 root-cause evidence 只在 external dir。
  - Dependencies: GH68-T1 完整 handoff；merged 上游 public test harness 可用。
  - File ownership: 独占 `tests/golden_real_apps.rs`；goldens、examples、docs、benches、
    workflow 与 specs 只读。
  - Covers: B-002, B-005, B-006, B-010, B-011, B-012, B-013, B-014, B-018, B-020,
    B-022, B-023, B-024, B-025, B-027, B-028。
  - Handoff: 保存 scratch root-cause evidence 与 deterministic fixture contract；把
    `tests/golden_real_apps.rs` 顺序移交 T3，并按
    `chat -> rnk_chat -> claude_input_box -> glm_chat` 交付每个 owner 的 focused assertion
    scope；无 precommitted red tests。

- [ ] `SP68-T3`（lane alias: `GH68-T3`）逐项迁移 `chat.rs` 为最小后端无关 tutorial。
  Owner: `chat-tutorial-owner` | Done when: example 只使用 merged public conversation/view/
  composer API，保留 submit/empty/quit 的最小可见行为；deterministic offline adapter 通过
  public typed updates 驱动状态；文件不再保存 `String` draft、`Vec<String>` transcript、
  字符级 backspace/cursor 或私有 delta/wrapping/commit 逻辑；无网络、密钥或 tool side effect
  | Verify: `cargo check --example chat --all-features --locked`；
  `cargo test --test gh68_chat_tutorial_contract --locked -- --exact`；full golden target 保持绿色，不运行
  尚未存在的 global convergence/future-owner tests。
  - Dependencies: GH68-T2 的 `chat.rs` root-cause fixture/handoff。
  - File ownership: 独占 `examples/chat.rs`，并从 T2 接管
    `tests/golden_real_apps.rs` 只新增 `gh68_chat_tutorial_contract`；其他 paths 只读。
  - Covers: B-001, B-002, B-004, B-005, B-010, B-011, B-023, B-024。
  - Handoff: 向 T4 提交 public composition pattern、offline adapter fixture、绿色 focused
    evidence 与 `tests/golden_real_apps.rs` ownership；T3 owner 停止写 owned paths。

- [ ] `SP68-T4`（lane alias: `GH68-T4`）逐项迁移 `rnk_chat.rs` 为 Fullscreen showcase。
  Owner: `fullscreen-example-owner` | Done when: example 只组合 merged
  `FullscreenChatShell`、public `MessageList`、Composer/status 与 deterministic fixtures；
  不再定义 private message/role、item-count `skip/take` scroll、message-height、
  bottom-follow、focus 或 resize state machine；单/多行 blocks、主动滚离、new-output indicator、
  prepend、streaming 与连续 resize 保持上游 anchor 合同；focused PTY evidence 自身覆盖
  normal/cancel/typed-failure/panic-unwind 后 raw mode、cursor、alternate screen、input mode
  全恢复，不能引用 provider test 代替 | Verify:
  `cargo check --example rnk_chat --all-features --locked`；
  `cargo test --test gh68_fullscreen_example_contract --locked -- --exact`；full golden target 保持绿色，不运行
  global convergence 或 T8 stress test。
  - Dependencies: GH68-T3 完成并 handoff；保持逐 example 串行质量检查。
  - File ownership: 独占 `examples/rnk_chat.rs`，并从 T3 接管
    `tests/golden_real_apps.rs` 只新增 `gh68_fullscreen_example_contract`；其他 paths 只读。
  - Covers: B-001, B-002, B-004, B-012, B-013, B-014, B-015, B-022, B-027。
  - Handoff: 向 T5 提交 Fullscreen composition、anchor/resize 与四路径 restoration focused
    evidence、tests ownership；T4 owner 停止写 owned paths。

- [ ] `SP68-T5`（lane alias: `GH68-T5`）逐项迁移 `claude_input_box.rs` 为 Inline showcase。
  Owner: `inline-example-owner` | Done when: example 只组合 merged `InlineChatShell`、
  `ChatComposer` 与 typed scrollback sink；不再定义 private `InlineInputState`、字符级 cursor/
  wrapping/viewport、直接 `println` commit 或 ledger；normal/cancel/failure/panic 和
  `Committed`/`NotCommitted`/`Unknown` 可见结果保持 GH66 合同，未稳定 stream 不进入
  scrollback | Verify: `cargo check --example claude_input_box --all-features --locked`；
  `cargo test --test gh68_inline_example_contract --locked -- --exact`；full golden target 保持绿色，不运行
  global convergence 或 provider tests。
  - Dependencies: GH68-T4 完成并 handoff。
  - File ownership: 独占 `examples/claude_input_box.rs`，并从 T4 接管
    `tests/golden_real_apps.rs` 只新增 `gh68_inline_example_contract`；其他 paths 只读。
  - Covers: B-001, B-002, B-004, B-011, B-014, B-023, B-027。
  - Handoff: 向 T6 提交 Inline composition、typed sink、Unicode/paste/restoration focused
    evidence 与 tests ownership；T5 owner 停止写 owned paths。

- [ ] `SP68-T6`（lane alias: `GH68-T6`）逐项迁移 `glm_chat` 为 application-owned provider
  adapter showcase 并移除私有 prompt implementation。Owner: `provider-example-owner` |
  Done when: provider DTO/network/tool demo 只在 application adapter 边界，public updates/
  blocks/shell 渲染 conversation；缺失 `GLM_API_KEY` 在发请求前返回 typed visible error 且
  request count 为 0；模型 Tool Call 不能自动获得执行授权；raw mode/cursor/spinner/cancel/
  direct ANSI/prompt wrapping 改由公共 shell/composer/view 合同承担；
  `examples/glm_chat/prompt_box.rs` 仅在 parity tests 通过后删除且无残留引用 | Verify:
  `cargo check --example glm_chat --all-features --locked`；
  `cargo test --example glm_chat --all-features --locked`；
  `cargo test --test gh68_provider_example_contract --locked -- --exact`；四个 focused tests 均绿色后新增/
  更新两个 real-app goldens，并首次运行
  `cargo test --test gh68_example_convergence_contract --locked -- --exact`。
  - Dependencies: GH68-T5 完成并 handoff。
  - File ownership: 独占 `examples/glm_chat.rs`、
    `examples/glm_chat/prompt_box.rs`，并从 T5 接管 `tests/golden_real_apps.rs` 与两个
    `tests/golden/real_app_chat*` files；只新增 provider-focused/global convergence tests，
    其他 paths 只读。
  - Covers: B-001, B-002, B-004, B-005, B-009, B-010, B-011, B-023, B-024,
    B-027。
  - Handoff: 向 T7 提交最终四-example purpose/runtime/dependency ledger、provider/tool
    boundary、删除证据、goldens、全 parity results 与 tests ownership；T6 owner 停止写
    owned paths。

- [ ] `SP68-T7`（lane alias: `GH68-T7`）完成 examples index、quickstarts、API maturity/
  migration 与 Chat terminal compatibility matrix。Owner: `docs-compat-owner` | Done when:
  四个 examples 各有唯一闭集分类、purpose/runtime/audience/prerequisite/public 状态；
  README 提供 compile-checked Inline/Fullscreen、update、custom renderer、keymap、errors、
  non-goals；API policy 保留 legacy `Message` 并逐项标记 stable/advanced/experimental 与
  migration/deprecation；terminal matrix 枚举 OS/terminal/Inline/Fullscreen/paste/resize/
  restoration/tmux/SSH，verified 单元均绑定 current evidence，无证据项为 unverified |
  Verify: `cargo test --test gh68_example_index_contract --locked -- --exact`；
  `cargo test --test gh68_message_compatibility_contract --locked -- --exact`；
  `cargo test --test gh68_public_docs_contract --locked -- --exact`；
  `cargo test --test gh68_compatibility_matrix_contract --locked -- --exact`；
  `python3 .github/scripts/check_markdown_links.py README.md examples/README.md docs/API_STABILITY.md docs/TERMINAL_COMPATIBILITY.md`。
  - Dependencies: GH68-T6 final example ledger。
  - File ownership: 独占 `README.md`、`examples/README.md`、
    `docs/API_STABILITY.md`、`docs/TERMINAL_COMPATIBILITY.md`，并从 T6 接管
    `tests/golden_real_apps.rs` 只新增四个 docs/compat exact tests；不写 examples/bench/CI。
  - Covers: B-001, B-002, B-003, B-006, B-007, B-008, B-009, B-010, B-013, B-018,
    B-024, B-027。
  - Handoff: 向 T8 提交唯一 public example ledger、compile-checked doc snippets、
    evidence-backed matrix 与 tests ownership；T7 owner 停止写 owned paths。

- [ ] `SP68-T8`（lane alias: `GH68-T8`）实现 deterministic Chat stress/benchmark 与
  comparable exact-head baseline evidence。Owner: `benchmark-evidence-owner` | Done when:
  `benches/render.rs` 包含 tech 指定的五个 `gh68_` workloads；fixture 固定且无 network/sleep/
  real terminal；相同 workload 先通过 deterministic correctness oracle；baseline metadata
  完整绑定 head/toolchain/OS/fixture/size/sample/median/MAD/unit；comparison 只有同时超过
  120% 和 `max(3*MAD,1ms)` 才判 regression，环境/metadata 不匹配时 blocked；CI smoke 不被
  表述为性能 pass；producer 读取同环境实际历史
  `$GH68_EVIDENCE_DIR/benchmark-baseline.json`，实际生成
  `$GH68_EVIDENCE_DIR/benchmark.json`、 与 `$GH68_EVIDENCE_DIR/coverage.jsonMODE=validate`、全部 artifact paths、head/base；再运行 workspace
  full check/test、examples、full golden target、15 exact tests 与 benchmark smoke；最后
  final after capture/window。
  - Dependencies: GH68-T7 完成并移交 tests ownership；T1 initial artifact 只授权开工。
  - File ownership: 独占 `benches/render.rs`，并从 T7 接管
    `tests/golden_real_apps.rs` 只新增四个 stress/benchmark/coverage exact tests；
    `Cargo.toml` 只读并断言现有 registration 未漂移。
  - Covers: B-015, B-016, B-017, B-021, B-022, B-025, B-026, B-028。
  - Handoff: 向 T9 提交 baseline coordinate/digest、T8 artifacts/results 与 tests ownership；
    T9 修改 workflow/tests 后 T8 head/artifacts 立即失效，只可作 provenance，必须全部重建。

- [ ] `SP68-T9`（lane alias: `GH68-T9`）接入 required CI exact-test/golden/index/examples/
  benchmark-smoke gates。Owner: `ci-gate-owner` | Done when: CI auto-discovers/builds全部公开
  examples；逐名运行全部 15 个 GH68 mapped exact tests 并拒绝零匹配/ignored；运行 full
  golden target 与 render benchmark smoke；`CI Gatecontinue-on-error`/smoke/hosted green 不宣称 fixed-environment performance pass | Verify:
  workflow/tests 修改完成后丢弃 T8 artifacts，在 T9 current head 重跑 tech 六步 Verification
  Plan 与 final after/window，再运行 workflow YAML parser；触发并验证 exact-head hosted CI，
  `gh68_ci_public_examples_contract` 检查 checkout、download/digest、producer-before-suite 与
  required job graph。
  - Dependencies: GH68-T8 完成并 handoff。
  - File ownership: 独占 `.github/workflows/ci.yml`，并从 T8 接管
    `tests/golden_real_apps.rs` 只新增 `gh68_ci_public_examples_contract`；其他 paths 只读。
  - Covers: B-003, B-016, B-017, B-019, B-020, B-021, B-025, B-026, B-028。
  - Handoff: 向 T10 提交 T9 exact head、baseline coordinate/digest、runner/local artifacts、
    required graph、15 exact outputs 与 tests ownership；这些是 audit input，T10 仍 fresh
    重建，任何 head 差异立即作废。

- [ ] `SP68-T10`（lane alias: `GH68-T10`）完成全量 current-head verification、coverage、
  dependency/CI/review evidence 与原子 closure handoff。Owner: `quality-evidence-owner` |
  Done when: product B-ID、tech mapping 与 tasks `Covers:` 集合完全相等；15 个 critical paths
  与 committed `gh68-critical-paths-v1validate_gh68_window`。任何修正或 head 变化都删除当前 artifacts，从 final preflight
  开始全部重建；任一缺失只报告 blocked。
  - Dependencies: GH68-T1 至 T9 全部完成并显式 handoff；所有 writers 停止。
  - File ownership: 接管 `tests/golden_real_apps.rs` 与两个 `real_app_chat` golden 仅修正
    evidence/tests；examples/docs/bench/workflow 只读。发现 implementation 缺陷时退回原 owner
    新 checkpoint，禁止跨 ownership 偷改。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010,
    B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021,
    B-022, B-023, B-024, B-025, B-026, B-027, B-028。
  - Handoff: 独立 reviewer 必须与 T1-T10 writers 分离；即使全部 evidence 通过，最终
    implementation PR approval、merge、release 与 GH-57/#68 closure 仍由人类决定。

## Execution Graph and Ownership

```text
SP68-T1 -> SP68-T2 -> SP68-T3 -> SP68-T4 -> SP68-T5 -> SP68-T6 -> SP68-T7
        -> SP68-T8 -> SP68-T9 -> SP68-T10
```

- 四个 examples 必须按 T3→T4→T5→T6 一个个迁移，禁止脚本批量改写。
- T2→T9 顺序移交 `tests/golden_real_apps.rs`；每个 task 只新增其 owned output 对应的绿色 tests。
  因共享该文件，T7/T8 不并行。
- 任一时刻每个 writable path 只有一个 owner；没有预提交红测，也没有 future-owner test 依赖。
- Specs 在 implementation 中只读；发现合同错误时停止执行并返回 spec update/reapproval。

## Invariant Coverage Audit

Expected product set:

`{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028}`

Task `Covers:` union:

`{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028}`

集合必须严格相等；新增 product invariant 时必须先更新 tech mapping、affected tasks 和本审计，
不得只在 T10 的 catch-all `Covers:` 中补号。

## Handoff Notes

- 当前 packet 只完成规划，不执行 implementation、commit、push、label、PR、approval 或 merge。
- 实现开始时必须 fresh 重读 merged GH61/GH66/GH67 specs/code；当前 parked drafts 只保留为
  provenance，不是 runtime truth。
- 所有完成声明必须包含 exact head、dependency merge ancestry、mapped exact outputs、
  baseline metadata、compat matrix、coverage、CI/reviewThreads 与 human gate evidence。
