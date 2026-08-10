# Task Plan：GH68 post-merge examples hardening

## Linked Issue

GH-68: https://github.com/majiayu000/rnk/issues/68

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## Current implementation gate

PR #166 已合并部分 GH-68 scope；当前 issue canonical label 是
`ready_to_implement`，且 maintainer 已批准本次 corrective-spec + post-merge implementation
follow-up。实现固定从 exact main
`e1d987447141b35d8049f8bd8ff89b2015ae87b9` 开始，只能修改本文件各 checkpoint 明列路径。
历史 issue body 的 `ready_to_spec` 和 PR #166 的旧结果都不能代替 current-head 验证。

所有 mapped tests 都定义在 `tests/golden_real_apps.rs`，命令统一为：

```text
cargo test --test golden_real_apps --all-features --locked <name> -- --exact
```

每个 selector 必须 matched=1、passed=1、ignored=0。

<!-- gh68-critical-paths-v1
{"version":1,"issue":68,"critical_paths":[{"file":"tests/golden_real_apps.rs","name":"gh68_harness_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_chat_tutorial_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_fullscreen_example_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_inline_example_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_provider_example_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_example_convergence_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_example_index_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_message_compatibility_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_public_docs_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_compatibility_matrix_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_stress_correctness_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_benchmark_metadata_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_benchmark_comparison_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_current_head_coverage_contract"},{"file":"tests/golden_real_apps.rs","name":"gh68_ci_public_examples_contract"}]}
-->

## Sequential checkpoints

- [ ] `SP68-F1` 修正 post-merge packet。Owner: `followup-spec-owner` | Done when: product、
  tech、tasks 记录 #166 partial merge、current readiness、exact base、`CHAT_QUICKSTART.md`、
  17-path closed scope、15 个 `golden_real_apps` exact selectors 和真实 ownership；删除损坏的
  pre-merge/SpecRail 文本 | Verify: `git diff --check -- specs/GH68`; planned JSON `jq`；B-ID/
  task/mapping/critical set equality；`python3 .github/scripts/check_markdown_links.py specs/GH68`。
  - File ownership: `specs/GH68/product.md`、`specs/GH68/tech.md`、
    `specs/GH68/tasks.md`。
  - Covers: B-001, B-003, B-019, B-020, B-021, B-025, B-026, B-028。
  - Handoff: spec checkpoint提交后转为只读；后续发现冲突必须停下并新做spec checkpoint。

- [ ] `SP68-F2` 建立 reusable green harness 与 root-cause evidence。Owner:
  `hardening-harness-owner` | Done when: deterministic conversation/view/shell fixtures、example
  source/index/docs/CI/benchmark parsers和负例均可复用；不预提交未来 selector 的红断言 |
  Verify: `cargo test --test golden_real_apps --all-features --locked gh68_harness_contract -- --exact`；
  `cargo test --test golden_real_apps --all-features --locked`。
  - File ownership: `tests/golden_real_apps.rs`。
  - Covers: B-002, B-005, B-006, B-010, B-011, B-012, B-013, B-014, B-018, B-020,
    B-022, B-023, B-024, B-027, B-028。
  - Handoff: 向F3移交harness和测试文件；未迁移examples仍保持当前行为。

- [ ] `SP68-F3` 完成最小 `chat.rs` tutorial。Owner: `chat-tutorial-owner` | Done when:
  example 使用 public `ConversationState`、typed updates、semantic view和shared composer；无
  `Vec<String>` transcript、字符级cursor/wrapping/delta/commit逻辑；empty/submit/quit保持可见 |
  Verify: `cargo check --example chat --all-features --locked`；
  `cargo test --test golden_real_apps --all-features --locked gh68_chat_tutorial_contract -- --exact`。
  - File ownership: `examples/chat.rs`、`tests/golden_real_apps.rs`。
  - Covers: B-001, B-002, B-004, B-005, B-010, B-011, B-023, B-024。
  - Handoff: F3停止写，向F4移交tests。

- [ ] `SP68-F4` 完成 `rnk_chat.rs` Fullscreen showcase。Owner:
  `fullscreen-example-owner` | Done when: 只组合 public `FullscreenChatShell`、
  `MessageList`/`ChatMessageView`和public composer；authoritative root snapshot决定rows；
  slice message/viewport rows由固定高度hidden-scroll viewport真实裁剪；width/height/composer/
  status/typing作为一个candidate原子提交，无item-count scroll或私有anchor |
  Verify: `cargo check --example rnk_chat --all-features --locked`；
  `cargo test --test golden_real_apps --all-features --locked gh68_fullscreen_example_contract -- --exact`。
  - File ownership: `examples/rnk_chat.rs`、`tests/golden_real_apps.rs`。
  - Covers: B-001, B-002, B-004, B-012, B-013, B-014, B-015, B-022, B-027。
  - Handoff: F4停止写，向F5移交tests。

- [ ] `SP68-F5` 完成 `claude_input_box.rs` Inline showcase。Owner:
  `inline-example-owner` | Done when: 只组合 public `InlineChatShell`/composer与typed
  `Fixed`/`Retained`/`Latched` outcomes；只有`Fixed`被acknowledge；无私有cursor/wrapping、直接
  `println` ledger或对`Unknown`的成功猜测 | Verify:
  `cargo check --example claude_input_box --all-features --locked`；
  `cargo test --test golden_real_apps --all-features --locked gh68_inline_example_contract -- --exact`。
  - File ownership: `examples/claude_input_box.rs`、`tests/golden_real_apps.rs`。
  - Covers: B-001, B-002, B-004, B-011, B-014, B-023, B-027。
  - Handoff: F5停止写，向F6移交tests。

- [ ] `SP68-F6` 完成 `glm_chat` provider adapter并删除private prompt。Owner:
  `provider-example-owner` | Done when: provider response只映射到typed conversation/view/shell；
  缺key时零请求；`PendingToolRequest` default-deny，显式exact-call批准后最多执行一次；workspace
  path canonical containment拒绝symlink escape，read/list限制depth/count/bytes且所有IO错误显式；
  prompt parity通过后删除`prompt_box.rs`及全部引用 | Verify:
  `cargo check --example glm_chat --all-features --locked`；`cargo test --example glm_chat --all-features --locked`；
  `cargo test --test golden_real_apps --all-features --locked gh68_provider_example_contract -- --exact`。
  - File ownership: `examples/glm_chat.rs`、删除`examples/glm_chat/prompt_box.rs`、
    `tests/golden_real_apps.rs`。
  - Covers: B-001, B-002, B-004, B-005, B-009, B-010, B-011, B-023, B-024, B-027。
  - Handoff: 向F7移交四example ledger与tests；F6停止写。

- [ ] `SP68-F7` 完成 golden、convergence、index、quickstart/API/matrix evidence。Owner:
  `docs-golden-owner` | Done when: plain/ANSI同fixture且去ANSI语义相等；四examples唯一分类；
  `Message`兼容；`CHAT_QUICKSTART`提供可编译Inline/Fullscreen/update/renderer/keymap/error/
  non-goals；API maturity和terminal matrix不夸大未验证能力 | Verify: 逐名运行
  `gh68_example_convergence_contract`、`gh68_example_index_contract`、
  `gh68_message_compatibility_contract`、`gh68_public_docs_contract`、
  `gh68_compatibility_matrix_contract`，均使用统一`--test golden_real_apps`命令；运行
  `python3 .github/scripts/check_markdown_links.py examples/README.md docs/API_STABILITY.md docs/CHAT_QUICKSTART.md docs/TERMINAL_COMPATIBILITY.md`。
  - File ownership: `examples/README.md`、`docs/API_STABILITY.md`、
    `docs/CHAT_QUICKSTART.md`、`docs/TERMINAL_COMPATIBILITY.md`、
    `tests/golden/real_app_chat.txt`、`tests/golden/real_app_chat.ansi.txt`、
    `tests/golden_real_apps.rs`。
  - Covers: B-001, B-002, B-003, B-006, B-007, B-008, B-009, B-010, B-013,
    B-018, B-024, B-027, B-028。
  - Handoff: 向F8移交deterministic fixtures/tests；F7停止写docs/goldens。

- [ ] `SP68-F8` 在既有 `render` target增加deterministic GH68 workloads。Owner:
  `benchmark-owner` | Done when: workloads覆盖long conversation、streaming、variable-height
  prepend、resize和Inline commit churn；每项先运行state/order/anchor/commit correctness oracle；
  smoke只声明可运行，不提交虚构固定baseline或性能pass | Verify: 逐名运行
  `gh68_stress_correctness_contract`、`gh68_benchmark_metadata_contract`、
  `gh68_benchmark_comparison_contract`、`gh68_current_head_coverage_contract`；
  `cargo bench --bench render --no-run --locked`；`cargo bench --bench render --locked -- gh68 --test`。
  - File ownership: `benches/render.rs`、`tests/golden_real_apps.rs`。
  - Covers: B-015, B-016, B-017, B-021, B-022, B-025, B-028。
  - Handoff: 向F9移交workload/oracle、15项critical manifest与coverage summary只读validator；
    F9拥有pinned-nightly raw/summary producer、exact-head ordering与artifact上传，F8停止写bench。

- [ ] `SP68-F9` 接入 required CI gate。Owner: `ci-owner` | Done when: CI从
  `examples/README.md`唯一public ledger自动发现并逐个构建examples；逐名运行15 exact tests、
  full golden target和render benchmark smoke；`nightly-2026-01-18` branch producer先生成并
  digest-bind raw/summary，changed executable line/branch各>=80%且denominator非零，critical
  逐项line 100%，真实有branch项branch 100%，真实零分支项显式N/A；保留原workspace gates，
  禁止`continue-on-error`、stable branch claim或把smoke称为performance pass | Verify:
  `cargo test --test golden_real_apps --all-features --locked gh68_ci_public_examples_contract -- --exact`；
  YAML parse；执行tech固定nightly coverage命令；fresh运行fmt/check/clippy/workspace/all-target/
  no-default/MSRV/examples/docs links/benchmark smoke和全部15 selectors。
  - File ownership: `.github/workflows/ci.yml`、`tests/golden_real_apps.rs`。
  - Covers: B-003, B-016, B-017, B-019, B-020, B-021, B-025, B-026, B-028。
  - Handoff: exact-head independent review和最终merge仍是独立human gate。

## Execution graph and ownership

```text
F1 -> F2 -> F3 -> F4 -> F5 -> F6 -> F7 -> F8 -> F9
```

- checkpoints串行移交`tests/golden_real_apps.rs`，任一时刻只有一个writer。
- 根`README.md`、`Cargo.toml`/lock、`src/**`、现有`benches/chat_scrollback.rs`和所有未列路径
  只读；需要它们才能修复时必须停下重新裁决。
- `examples/glm_chat/prompt_box.rs`只能在F6 parity通过后删除。

## Invariant coverage audit

Product、Tech mapping、task `Covers:` union 的expected set均为：

`{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028}`

critical manifest必须恰好15项、全部位于`tests/golden_real_apps.rs`，且每个name在Rust文件中
恰好定义一次。coverage summary必须对相同15项逐项给出line denominator/covered、branch
denominator/covered与闭集branch状态`covered|not_applicable`；`not_applicable`只允许
`branch_total=0`且raw LLVM函数记录存在。任一缺失时GH-68保持未完成。
