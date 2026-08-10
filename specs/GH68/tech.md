# Tech Spec：聊天 Examples 收敛、兼容性与产品证据 Gate

## Linked Issue

GH-68: https://github.com/majiayu000/rnk/issues/68

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":68,"complete":true,"paths":[".github/workflows/ci.yml","benches/render.rs","docs/API_STABILITY.md","docs/CHAT_QUICKSTART.md","docs/TERMINAL_COMPATIBILITY.md","examples/README.md","examples/chat.rs","examples/claude_input_box.rs","examples/glm_chat.rs","examples/glm_chat/prompt_box.rs","examples/rnk_chat.rs","specs/GH68/product.md","specs/GH68/tasks.md","specs/GH68/tech.md","tests/golden/real_app_chat.ansi.txt","tests/golden/real_app_chat.txt","tests/golden_real_apps.rs"],"spec_refs":["specs/GH68/product.md","specs/GH68/tech.md","specs/GH68/tasks.md"]}
-->

该planned-changes JSON只是closed file-scope manifest，不是SpecRail authorization/readiness/
coverage gate；实施授权遵循当前`CONTRIBUTING.md`的明确maintainer confirmation与human review。

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-68 的最终 composition、迁移、文档、兼容矩阵、test/golden、
stress/benchmark 与 CI evidence。它不拥有 Conversation、TextFlow、LayoutSnapshot、
`MessageList`、`ChatComposer`、`InlineChatShell` 或 `FullscreenChatShell` 的生产实现。

GH-61、GH-66、GH-67 和 partial GH-68 PR #166 已合入当前 main；#68 当前 canonical label
是`ready_to_implement`。本 follow-up 固定 base 为
`e1d987447141b35d8049f8bd8ff89b2015ae87b9`，只修复下文17-path closed scope。根
`README.md`、`Cargo.toml`/lock、`src/**`和#166新增的`benches/chat_scrollback.rs`均只读。

## Codebase Context

下表是PR #166之前、基于`54617335e9ec16825232685e94433acdd1fd7cb4`的root-cause
历史锚点，不再冒充当前实现事实。#166只迁移了`chat.rs`/`rnk_chat.rs`并增加一部分
docs/bench/CI；当前base上的`claude_input_box.rs`、`glm_chat`、golden harness、terminal
matrix、`render` GH68 workloads和严格CI discovery仍是本follow-up缺口。

| Area | Current anchor | Current behavior | GH-68 decision |
| --- | --- | --- | --- |
| 最小 chat example | `examples/chat.rs:13`, `examples/chat.rs:21` | 自建 `String` draft、`Vec<String>` messages 与字符级 input handler | 迁移为最小 public Chat API tutorial；不保留私有编辑/消息状态机 |
| Fullscreen chat example | `examples/rnk_chat.rs:13`, `examples/rnk_chat.rs:27`, `examples/rnk_chat.rs:138`, `examples/rnk_chat.rs:141` | 自建 message/role、item-count scroll、input/footer/typing state | 迁移为 `FullscreenChatShell` showcase，并消费上游 row-based `MessageList` |
| Inline chat example | `examples/claude_input_box.rs:34`, `examples/claude_input_box.rs:132`, `examples/claude_input_box.rs:157`, `examples/claude_input_box.rs:249` | 自建字符 cursor/wrapping、live input 和 `println` transcript commit | 迁移为 `InlineChatShell` showcase；commit 与 composer lifecycle 只由上游公共合同拥有 |
| Provider example | `examples/glm_chat.rs:26`, `examples/glm_chat.rs:334`, `examples/glm_chat.rs:341`, `examples/glm_chat.rs:356`, `examples/glm_chat.rs:399` | 引入私有 prompt module，直接 ANSI 输出，自管 raw mode、spinner/cancel 和 provider DTO | 保留应用 adapter/tool demo 边界，但用公共 chat state/view/shell；缺失密钥在请求前显式失败 |
| 私有 GLM prompt | `examples/glm_chat/prompt_box.rs:52`, `examples/glm_chat/prompt_box.rs:64`, `examples/glm_chat/prompt_box.rs:80`, `examples/glm_chat/prompt_box.rs:103` | 自管 render、ANSI 清屏/光标、Unicode suffix clipping | 公共 `ChatComposer` 完成 parity 后删除该辅助实现；删除不影响顶层 example 的 provider 教学目的 |
| Example index | `examples/README.md:22`, `examples/README.md:28` | 四个目标 examples 均列为 Showcase，但未记录 runtime、audience、public/private 状态或迁移目的 | 建立四项唯一分类、目的、模式、前置条件与 public 状态；CI 检查索引漂移 |
| API policy | `docs/API_STABILITY.md:7`, `docs/API_STABILITY.md:81`, `docs/API_STABILITY.md:166` | prelude 是推荐 stable surface；module public API 默认 advanced；新 API 必须分类并由 examples/tests 证明 | 记录 `Message` compat、Chat API 成熟度、import、弃用/迁移策略；不得把 example 使用自动等同 stable |
| Terminal policy | `docs/TERMINAL_COMPATIBILITY.md:18`, `docs/TERMINAL_COMPATIBILITY.md:55`, `docs/TERMINAL_COMPATIBILITY.md:90` | 有通用环境、Unicode 与 resize 合同，但没有 Chat Inline/Fullscreen evidence 状态 | 扩展同一矩阵，使用 verified/best-effort/terminal-dependent/unsupported/unverified 闭集 |
| User docs | `docs/CHAT_QUICKSTART.md` | #166 已增加初版quickstart，但必须按当前public API补齐compile-checked Inline、Fullscreen、update、custom renderer、keymap、errors与non-goals证据 | 原地修正专用文档；不修改根README |
| Golden baseline | `tests/golden_real_apps.rs:8`, `tests/golden_real_apps.rs:111` | chat golden 只渲染 legacy `Message` 静态文本 | 改为 deterministic public Chat fixtures，增加全部 GH68 exact contract tests |
| Golden files | `tests/golden/real_app_chat.txt:1`, `tests/golden/real_app_chat.ansi.txt:1` | 已有 plain/ANSI 文件，可原地更新 | 保留稳定名称并验证 ANSI 去色后的语义等价 |
| CI | `.github/workflows/ci.yml:62`, `.github/workflows/ci.yml:95`, `.github/workflows/ci.yml:98` | all-target tests、examples check、benches check 已存在，但没有逐名 GH68 gate | 增加 required GH68 exact-test/golden gate 与 benchmark smoke；保留全 workspace gates |
| Benchmark | `benches/render.rs:7`, `benches/render.rs:129`, `benches/render.rs:217` | `render` bench 已注册并覆盖通用 render workloads，没有聊天长会话/stream/resize 组合 | 在既有 `render` bench 中新增 GH68 workloads，避免新增 benchmark target 与 Cargo metadata 漂移 |
| Bench registration | `Cargo.toml:89` | `render` bench 已以 `harness = false` 注册 | 保持不变，因此 `Cargo.toml` 不在 planned changes 中 |

## 设计方案

### 1. Current-head implementation gate

每个checkpoint开始时必须验证`HEAD`是fixed base
`e1d987447141b35d8049f8bd8ff89b2015ae87b9`的后代、worktree只含当前owner预期修改，并记录
上一DCO checkpoint。完成证据必须fresh运行当前head命令；head变化后旧test、coverage、bench
和CI输出全部失效。#68 readiness和人工授权只授予明列scope，不授权改`src/**`、Cargo、根
README或外部状态。最终review/merge仍是独立human gate。

### 2. Example ownership and purpose

GH-68 对四个顶层 examples 逐个人工质量迁移，不做机械批量重写：

| Example | Category | Independent purpose | Runtime boundary |
| --- | --- | --- | --- |
| `chat.rs` | `tutorial` | 最小后端无关 conversation + composer 组合 | deterministic offline adapter；不含网络、工具或私有输入状态 |
| `rnk_chat.rs` | `showcase` | variable-height Fullscreen transcript、fixed-bottom composer/status、focus/navigation | 只组合 `FullscreenChatShell` 与公共 fixtures |
| `claude_input_box.rs` | `showcase` | native scrollback Inline lifecycle、typed commit outcomes、live composer | 只组合 `InlineChatShell`；不直接管理 ledger 或视觉光标 |
| `glm_chat.rs` | `showcase` | application-owned provider adapter、typed blocks、tool display/授权边界和 errors | provider DTO/network/tool actions 留在 example；核心 Chat API 不感知供应商 |

`examples/glm_chat/prompt_box.rs` 没有独立顶层教学目的；在公共 composer parity 已证明后，
从 `glm_chat.rs` 移除引用并删除该私有辅助文件。若实现时发现新的独立目的，必须先更新
product/tech/tasks 与索引并重新获得人工 spec approval，不能静默扩大范围。

### 3. Composition and adapter flow

```text
deterministic fixture / provider response
                  |
                  v
        application-owned adapter
                  |
                  v
        public ConversationUpdate
                  |
                  v
   upstream reducer + typed conversation state
          /                     \
         v                       v
InlineChatShell            FullscreenChatShell
 native scrollback          owned transcript
```

- examples 只拥有 fixture、theme、provider DTO mapping 与应用动作。
- update ordering、revision、delta assembly、message height、anchor、bottom-follow、commit
  idempotency、composer editing 和 terminal lifecycle 全部复用上游公共合同。
- `gh68_chat_tutorial_contract` 与 `gh68_provider_example_contract` 共同使用一个
  deterministic offline adapter 和一个 provider-shaped adapter，将相同语义事件映射为相同
  public updates；测试只使用静态 fixture，不访问网络或真实密钥。
- `glm_chat` 可执行应用明确授权的 demo tool，但 Tool Call view 只呈现 typed state；模型输出
  不能直接获得权限。缺失 `GLM_API_KEY` 时必须在创建请求前返回可见错误，不再用 placeholder
  key 发起网络请求。

### 4. Compatibility and documentation

- `docs/API_STABILITY.md` 记录 legacy `Message` compatibility wrapper 及每个 Chat API 的
  stable/advanced/experimental 状态。沿用现有规则：未具备批准 evidence 的 module-public
  API 保持 advanced/experimental，不自动进入 prelude stable surface。
- `docs/CHAT_QUICKSTART.md` 提供可复制的 Inline/Fullscreen quickstart，以及 conversation update、
  custom block renderer、keymap、error handling、provider/tool boundary 和非目标。
- `examples/README.md` 是 examples 分类与用途的唯一索引；其他文档只链接，不复制第二份列表。
- `docs/TERMINAL_COMPATIBILITY.md` 扩展 Chat matrix。每个 `verified` 单元记录 evidence
  kind、environment 与 current head；通用 cross-platform compile 不能升级为真实 terminal
  verification。

### 5. Golden and interaction contracts

`tests/golden_real_apps.rs` 继续作为真实应用组合 gate。每个 owner 在修改其 owned output 的
同一绿色提交中新增对应 top-level exact test；不得在 F2 预提交未来 owner 的红测，也不得让
任一 task 依赖尚未完成 task 才能通过的断言：

- `gh68_harness_contract`
- `gh68_chat_tutorial_contract`
- `gh68_fullscreen_example_contract`
- `gh68_inline_example_contract`
- `gh68_provider_example_contract`
- `gh68_example_convergence_contract`
- `gh68_example_index_contract`
- `gh68_message_compatibility_contract`
- `gh68_public_docs_contract`
- `gh68_stress_correctness_contract`
- `gh68_benchmark_metadata_contract`
- `gh68_benchmark_comparison_contract`
- `gh68_current_head_coverage_contract`
- `gh68_compatibility_matrix_contract`
- `gh68_ci_public_examples_contract`

F2 只实现 reusable harness、deterministic fixtures 与拒绝旧 SHA、空 evidence、伪
verified、placeholder key、`Unknown` 自动重试、ignored exact test、环境不匹配和 smoke-only
benchmark 的负例；`gh68_harness_contract` 在未迁移 baseline 上也必须绿色。baseline root
cause 只写入 `$GH68_EVIDENCE_DIR/root-cause/` scratch evidence，不把红测提交到 repository。
F3、F4、F5、F6 分别只运行 `gh68_chat_tutorial_contract`、
`gh68_fullscreen_example_contract`、`gh68_inline_example_contract`、
`gh68_provider_example_contract`；全局 `gh68_example_convergence_contract` 只能在四项全部
完成后的 F7 与最终验证运行。docs、benchmark/coverage 与 CI tests 同理由 F7、F8、F9 在各自
output 同一绿色提交中新增。

测试可使用 `include_str!` 核对 docs/index/workflow contract，但必须同时运行 public API
行为测试；单纯 grep source 不能证明 runtime correctness。focused tests 复用 GH-66/GH-67
public harness 与 PTY fixture。`gh68_fullscreen_example_contract` 自身必须覆盖 Fullscreen
normal/cancel/typed-failure/panic-unwind 后 raw mode、cursor visibility、alternate screen 与
input mode 全部恢复；Inline/provider evidence 不得替代。其余 focused scope 覆盖
Unicode/paste、focus/resize、typed outcomes、provider/tool security 与显式 degradation。

plain 与 ANSI golden 继续使用现有
`tests/golden/real_app_chat.txt`、`tests/golden/real_app_chat.ansi.txt`。ANSI normalization
必须只去除 escape/style，不得重排或删除语义文本。

### 6. Stress and benchmark evidence

`benches/render.rs` 增加前缀为 `gh68_` 的 deterministic workloads：

- `gh68_long_conversation`
- `gh68_high_frequency_streaming`
- `gh68_variable_height_prepend`
- `gh68_continuous_resize`
- `gh68_inline_commit_churn`

每个 workload 使用固定 seed/fixture，并在 benchmark 外由
`gh68_stress_correctness_contract` 对相同输入验证最终 state、message order、anchor、
bottom-follow 与 commit count。benchmark 不使用 provider network、wall-clock sleeps 或真实
terminal input。

baseline artifact 不提交生成物到仓库。F8在固定环境设置
`GH68_BENCHMARK_MODE=produce` 与
`GH68_BENCHMARK_BASELINE=$GH68_EVIDENCE_DIR/benchmark-baseline.json`、
`GH68_BENCHMARK_EVIDENCE=$GH68_EVIDENCE_DIR/benchmark.json` 后运行
`gh68_benchmark_metadata_contract`；该 test 执行与 `benches/render.rs` 同一 fixture/workload
的实际当前-head 计时，读取同环境批准 baseline artifact，并写 comparison artifact，而不是
复制内嵌样本。artifact schema 固定为：

```json
{
  "schema": "gh68-benchmark-v1",
  "head_sha": "<40-hex>",
  "base_main_sha": "<40-hex>",
  "generated_at": "<RFC3339>",
  "environment": {"rustc_vv": "<nonempty>", "os": "<nonempty>", "arch": "<nonempty>"},
  "fixture": {
    "version": "gh68-chat-workloads-v1",
    "seed": 68,
    "message_count": 0,
    "block_count": 0,
    "character_count": 0,
    "width_height_sequence": [[80, 24]]
  },
  "workloads": [{
    "name": "gh68_long_conversation",
    "warmup_samples": 3,
    "measured_samples_ns": [1],
    "median_ns": 1,
    "mad_ns": 0,
    "unit": "ns"
  }],
  "baseline": {
    "coordinate": {"repository": "majiayu000/rnk", "workflow": "<file>",
      "run_id": 1, "artifact_name": "<immutable-name>"},
    "source_sha256": "<64-hex>",
    "head_sha": "<40-hex>",
    "environment": {"rustc_vv": "<nonempty>", "os": "<nonempty>", "arch": "<nonempty>"},
    "fixture": {"version": "gh68-chat-workloads-v1", "seed": 68},
    "workloads": [{
      "name": "gh68_long_conversation",
      "measured_samples_ns": [1],
      "median_ns": 1,
      "mad_ns": 0,
      "unit": "ns"
    }]
  },
  "comparison": {
    "environment_equal": true,
    "fixture_equal": true,
    "relative_threshold": 1.2,
    "absolute_floor_ns": 1000000,
    "results": [{
      "name": "gh68_long_conversation",
      "candidate_median_ns": 1,
      "baseline_median_ns": 1,
      "regression": false
    }]
  }
}
```

上面的数字只是类型示意；producer 必须写真实计数、五个 workload、至少 3 个 warm-up 与
15 个正整数 measured samples，且从 candidate/baseline samples 重新计算 median/MAD。
baseline artifact 也必须是同 schema family 的实际历史运行，绑定 distinct prior head、
完整 environment/fixture/workloads，以及 immutable repository/workflow/run-id/artifact-name
top-level coordinate；producer 保存下载 SHA-256，并规范化到 `baseline.coordinate`。validator invocation 固定为：

```sh
GH68_BENCHMARK_MODE=produce \
GH68_BENCHMARK_BASELINE="$GH68_EVIDENCE_DIR/benchmark-baseline.json" \
GH68_BENCHMARK_EVIDENCE="$GH68_EVIDENCE_DIR/benchmark.json" \
GH68_IMPLEMENTATION_HEAD="$IMPLEMENTATION_HEAD" \
GH68_BASE_MAIN_SHA="$BASE_MAIN_SHA" \
  cargo test --test golden_real_apps --all-features --locked gh68_benchmark_metadata_contract -- --exact
GH68_BENCHMARK_MODE=validate \
GH68_BENCHMARK_BASELINE="$GH68_EVIDENCE_DIR/benchmark-baseline.json" \
GH68_BENCHMARK_EVIDENCE="$GH68_EVIDENCE_DIR/benchmark.json" \
GH68_IMPLEMENTATION_HEAD="$IMPLEMENTATION_HEAD" \
GH68_BASE_MAIN_SHA="$BASE_MAIN_SHA" \
  cargo test --test golden_real_apps --all-features --locked gh68_benchmark_comparison_contract -- --exact
```

`gh68_benchmark_comparison_contract` 必须读取该实际 artifact，校验 head/base/environment/
fixture、immutable baseline coordinate/digest、两侧五个 workload、样本数、median/MAD 重算、
results 与 schema；只测试内嵌正负 fixtures 不算通过。两个 benchmark exact tests 在 mode
缺失/越界、baseline/evidence path 非 absolute、文件缺失或 head/base 不匹配时都必须失败。

同环境比较至少使用 15 个 measured samples，并在至少 3 个 warm-up samples 后计算 median。
候选只有同时满足以下两项才判为 regression：

1. candidate median 大于 baseline median 的 120%；
2. candidate 与 baseline 的绝对差大于 `max(3 * baseline MAD, 1 ms)`。

若 workload 的正常量级小于 1 ms，第二项的 1 ms floor 避免噪声误判；但 baseline/metadata
缺失、环境不一致或 workload correctness 失败时，结果是 blocked，不是 pass。

### 7. Durable current-head coverage evidence

`cargo llvm-cov` raw/summary只写runner-local或外部evidence目录，不提交生成物。
`gh68_current_head_coverage_contract`读取tasks中的唯一`gh68-critical-paths-v1` block，要求15个
`file + name`恰好都位于`tests/golden_real_apps.rs`且各定义一次。最终current-head run要求
changed executable line coverage >=80%，15项critical逐项line/branch 100%且denominator非零。
缺工具、空结果、旧head/base、unknown/duplicate/extra critical项或`continue-on-error`均失败。

### 8. CI gates

required GH68 job保留现有workspace gates，并从`examples/README.md`唯一public ledger自动发现
和逐个`cargo check --example`，防止手写列表遗漏。它逐名运行15个统一target exact selectors、
full `golden_real_apps`、docs link checker与`render` GH68 benchmark smoke；任一零匹配、ignored、
example/index drift或`continue-on-error`失败。smoke只证明workload可运行，不是performance pass；
固定环境baseline若缺失则明确blocked，不得用hosted时序伪造比较结论。

## Product-to-Test Mapping

Exact selector catalog（全部定义于`tests/golden_real_apps.rs`）：

```sh
cargo test --test golden_real_apps --all-features --locked gh68_harness_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_chat_tutorial_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_fullscreen_example_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_inline_example_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_provider_example_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_example_convergence_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_example_index_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_message_compatibility_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_public_docs_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_compatibility_matrix_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_stress_correctness_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_benchmark_metadata_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_benchmark_comparison_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_current_head_coverage_contract -- --exact
cargo test --test golden_real_apps --all-features --locked gh68_ci_public_examples_contract -- --exact
```

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | 四examples与purpose ledger | 四focused selectors、`gh68_example_convergence_contract`、`gh68_example_index_contract` |
| B-002 | parity fixtures/goldens | 四focused selectors与convergence |
| B-003 | `examples/README.md`、CI index | `gh68_example_index_contract`、`gh68_ci_public_examples_contract` |
| B-004 | public composition | 四focused selectors与convergence |
| B-005 | 两个adapter/reducer/view | `gh68_chat_tutorial_contract`、`gh68_provider_example_contract` |
| B-006 | legacy `Message` | `gh68_message_compatibility_contract` |
| B-007 | API maturity/import/migration | `gh68_public_docs_contract` |
| B-008 | Inline/Fullscreen quickstarts | `gh68_public_docs_contract` |
| B-009 | update/renderer/keymap/error/non-goals | `gh68_public_docs_contract`、`gh68_provider_example_contract` |
| B-010 | public fixtures 与 adapter empty/error paths | `gh68_chat_tutorial_contract` 与 `gh68_provider_example_contract` 覆盖 empty conversation/text/blocks/metadata/adapter 且无 invented data |
| B-011 | public composer interaction fixture | `gh68_chat_tutorial_contract` 与 `gh68_inline_example_contract` 覆盖 CJK/emoji/combining/ZWJ/CRLF/multi-char/paste |
| B-012 | Fullscreen focus/resize fixture | `gh68_fullscreen_example_contract` 覆盖 focus/keymap/min-size/连续 resize/input interleave 且 draft/transcript/anchor 不漂移 |
| B-013 | `real_app_chat` plain/ANSI | `gh68_example_convergence_contract` |
| B-014 | GH66/GH67 terminal harness 与 Inline/Fullscreen orchestration | `gh68_fullscreen_example_contract` 自身覆盖 Fullscreen normal/cancel/typed-failure/panic-unwind 与 raw/cursor/alternate-screen/input-mode restoration；`gh68_inline_example_contract` 独立覆盖 Inline；provider test 不替代任一 shell |
| B-015 | `benches/render.rs`、oracle | `gh68_stress_correctness_contract`与render benchmark smoke |
| B-016 | benchmark metadata | `gh68_benchmark_metadata_contract` |
| B-017 | comparison/smoke boundary | `gh68_benchmark_comparison_contract` |
| B-018 | terminal matrix | `gh68_compatibility_matrix_contract` |
| B-019 | CI public discovery | `gh68_ci_public_examples_contract` |
| B-020 | 15 exact selectors | `gh68_ci_public_examples_contract`与critical manifest |
| B-021 | exact base/head window | `gh68_current_head_coverage_contract`、`gh68_ci_public_examples_contract` |
| B-022 | public reducer、MessageList/shell stress fixture | `gh68_fullscreen_example_contract` 与 `gh68_stress_correctness_contract` 交错 delta/append/prepend/height/resize 并比较 order/identity/anchor/follow |
| B-023 | Inline typed outcomes 与 adapter failures | `gh68_inline_example_contract` 与 `gh68_provider_example_contract` 覆盖 cancel/fail/NotCommitted/Unknown/duplicate/retry/partial completion/late success |
| B-024 | `glm_chat` credential/tool boundary、offline tests | `gh68_provider_example_contract`；缺 key 时零请求，模型 tool output 不自动获得执行授权，tests 不访问网络 |
| B-025 | current-head coverage/critical set | `gh68_current_head_coverage_contract`与fresh llvm-cov |
| B-026 | authorization/dependency ancestry | `gh68_ci_public_examples_contract`与final human audit |
| B-027 | capability/degradation fixtures | 四个 focused tests 与 compatibility matrix 只允许 terminal optional capability 显式降级；data/order/layout/commit/restoration failures 全部失败 |
| B-028 | atomic completion | convergence、15 selectors、coverage、matrix、CI和human audit |

## Verification Helpers

CI和本地验证从tasks critical manifest读取literal names；对每个name先用统一target
`-- --list --exact`确认恰好一个match，再执行catalog中的exact command并要求
`1 passed; 0 failed; 0 ignored`。禁止从文件名构造`--test gh68_*`伪target。

## Verification Plan

每个checkpoint：

- `git diff --check`
- 当前owner focused example/check与mapped selectors；
- `cargo fmt --all -- --check`；
- `cargo check --workspace --all-targets --all-features --locked`；
- focused clippy；spec/docs checkpoint运行markdown link checker；
- 只在focused green后做一个DCO checkpoint commit。

最终current head额外运行workspace all-target tests、no-default-features、Rust 1.88 MSRV、全部
examples、15 exact selectors、plain/ANSI golden、docs links、render benchmark no-run/smoke、
fresh llvm-cov与workflow YAML/required graph检查。任何修正产生新head后重跑受影响证据。

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| 上游 API 在 parked 草案后改变 | 实现前 fresh 读取 merged GH61/GH66/GH67 specs/code；GH68 不计划任何 `src/` path，不复制上游合同 |
| 四个 examples 迁移造成范围重叠 | 按 `chat -> rnk_chat -> claude_input_box -> glm_chat` 串行 handoff，每步独立 parity evidence |
| provider demo 泄露密钥或执行未授权 tool | key 只从环境读取；缺失即零请求失败；tool authorization 留在应用显式 callback，offline tests 禁止网络 |
| ANSI/plain 或 terminal matrix 夸大能力 | semantic normalization test；只有绑定 current evidence 的单元可为 verified，其余 unverified/best-effort |
| benchmark 在 hosted CI 抖动 | CI 只做 smoke；固定环境多样本 median/MAD 比较，双阈值避免单点误判 |
| 旧 evidence 被拼接成通过 | final audit 绑定 exact head、完整 critical set、依赖 merge ancestry 与前后 SHA/clean window |
| 删除私有 prompt helper 后行为丢失 | 先通过 composer/inline parity 与 `glm_chat` exact example test，再删除；失败时回滚该 example migration |

## Rollback

- 四个 example migration 使用独立提交/rollback point；某一 example 失败时只回滚该迁移，
  不回滚已验证的其他 examples 或上游公共组件。
- 文档、golden、benchmark 和 CI 必须与对应 example rollback 同步恢复，禁止保留声称已迁移
  的索引或证据。
- 如果上游 merged API 与本 spec 假设不一致，停止实现并更新 GH68 packet/重新人工批准；
  不通过 compatibility alias 或私有 fallback 掩盖差异。
- rollback 不得删除失败 evidence，也不得把 GH68 标记完成；依赖与人工 gate 继续生效。

## Handoff

- 本 packet 与issue audit已授权17-path post-merge follow-up；不授权`src/**`、Cargo、根README、
  push、GitHub mutation或merge。
- 每个checkpoint按tasks串行交接唯一shared test file；禁止并行共享写。
- writer自报结果不能替代exact-head independent review、maintainer merge authorization或CI。
