# Tech Spec：聊天 Examples 收敛、兼容性与产品证据 Gate

## Linked Issue

GH-68: https://github.com/majiayu000/rnk/issues/68

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":68,"complete":true,"paths":["specs/GH68/product.md","specs/GH68/tech.md","specs/GH68/tasks.md",".github/workflows/ci.yml","README.md","benches/render.rs","docs/API_STABILITY.md","docs/TERMINAL_COMPATIBILITY.md","examples/README.md","examples/chat.rs","examples/claude_input_box.rs","examples/glm_chat.rs","examples/glm_chat/prompt_box.rs","examples/rnk_chat.rs","tests/golden/real_app_chat.ansi.txt","tests/golden/real_app_chat.txt","tests/golden_real_apps.rs"],"spec_refs":["specs/GH68/product.md","specs/GH68/tech.md","specs/GH68/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-68 的最终 composition、迁移、文档、兼容矩阵、test/golden、
stress/benchmark 与 CI evidence。它不拥有 Conversation、TextFlow、LayoutSnapshot、
`MessageList`、`ChatComposer`、`InlineChatShell` 或 `FullscreenChatShell` 的生产实现。

GH-68 implementation 必须等待 GH-61、GH-66、GH-67 的最终 implementation PR 均
merged，且三个 merge commit 都是实现 head 的严格祖先；还必须等待人工 spec approval 与
canonical `ready_to_implement`。PR #69 head
`2c4720152d43f9507fe1fb43e331a866c683c585` 和 GH-61 spec PR #78 都是
parked/draft 草案证据，不能冒充已批准或已合并依赖。

## Codebase Context

下列锚点均基于本规格起草时的 `main` commit
`54617335e9ec16825232685e94433acdd1fd7cb4`。

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
| User docs | `README.md:126`, `README.md:688` | 有通用 examples 入口与 curated list，没有公共 Chat quickstart/extension/error guide | 增加 Inline、Fullscreen、update、custom renderer、keymap、errors、non-goals 入口 |
| Golden baseline | `tests/golden_real_apps.rs:8`, `tests/golden_real_apps.rs:111` | chat golden 只渲染 legacy `Message` 静态文本 | 改为 deterministic public Chat fixtures，增加全部 GH68 exact contract tests |
| Golden files | `tests/golden/real_app_chat.txt:1`, `tests/golden/real_app_chat.ansi.txt:1` | 已有 plain/ANSI 文件，可原地更新 | 保留稳定名称并验证 ANSI 去色后的语义等价 |
| CI | `.github/workflows/ci.yml:62`, `.github/workflows/ci.yml:95`, `.github/workflows/ci.yml:98` | all-target tests、examples check、benches check 已存在，但没有逐名 GH68 gate | 增加 required GH68 exact-test/golden gate 与 benchmark smoke；保留全 workspace gates |
| Benchmark | `benches/render.rs:7`, `benches/render.rs:129`, `benches/render.rs:217` | `render` bench 已注册并覆盖通用 render workloads，没有聊天长会话/stream/resize 组合 | 在既有 `render` bench 中新增 GH68 workloads，避免新增 benchmark target 与 Cargo metadata 漂移 |
| Bench registration | `Cargo.toml:89` | `render` bench 已以 `harness = false` 注册 | 保持不变，因此 `Cargo.toml` 不在 planned changes 中 |

## 设计方案

### 1. Fail-closed implementation gate

开始任何 implementation edit 前，integration owner 必须运行下文同一
`capture_gh68_preflight`/`validate_gh68_preflight` adapter，生成
`$GH68_EVIDENCE_DIR/preflight-initial.json`，artifact `phase=initial`。initial 只用于 pre-edit
authorization，允许 `IMPLEMENTATION_HEAD == BASE_MAIN_SHA` 或 base 为 head 祖先，避免要求伪
空 commit。T8/T9/T10 的 current-head evidence 与最终窗口必须 fresh capture
`phase=final`；final 要求 base 与 head 不同且 base 是严格祖先/merge-base。

adapter 对每个 dependency issue 要求 issue `CLOSED` 且无 `parked`，并只接受唯一 closing
final implementation PR：`MERGED`、非 draft/parked、closing/file/labels pagination 均完整，
且完整 files 至少含一个匹配 `^(src/.*|crates/[^/]+/.*)\.rs$` 的 executable Rust source；
README/docs/spec-only 必须被拒绝。三个 merge SHA 必须两两不同；initial 允许
ancestor-or-equal，final 必须都是 `IMPLEMENTATION_HEAD` 的严格祖先。

同一 artifact 还必须证明 GH-68 spec PR 以 `main` 为 base、body 含 `Refs #68` 且不含
`close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved #68`，changed-file pagination
完整且 exact set 为本 packet 三文件、已 merged、非 draft/parked，并存在一个 human
`APPROVED` GitHub review。review 必须绑定 spec PR exact head，且 body 含精确 scope marker
`GH68-SPEC-APPROVAL scope=specs/GH68/{product,tech,tasks}.md`；adapter 保存 actor、
author-association、source、scope、review commit 与 submittedAt。#68 的
`ready_to_implement` 必须由同次 fresh issue query 取得，并拒绝 `parked` 及除
`ready_to_implement` 外任何 `ready_to_*` 冲突 label。

`IMPLEMENTATION_HEAD` 是当前 phase 待验证分支的 exact head，不要求等于 `origin/main`。
T10 在全部 evidence/full-suite 开始前与完成后各 fresh capture `phase=final`；两份 artifact
的 phase、implementation head、base-main SHA、dependency/spec/readiness decisive sets 与
clean status 必须完全相同。任一条件不满足时保持 blocked。

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
- `README.md` 提供可复制的 Inline/Fullscreen quickstart，以及 conversation update、
  custom block renderer、keymap、error handling、provider/tool boundary 和非目标。
- `examples/README.md` 是 examples 分类与用途的唯一索引；README 只链接，不复制第二份列表。
- `docs/TERMINAL_COMPATIBILITY.md` 扩展 Chat matrix。每个 `verified` 单元记录 evidence
  kind、environment 与 current head；通用 cross-platform compile 不能升级为真实 terminal
  verification。

### 5. Golden and interaction contracts

`tests/golden_real_apps.rs` 继续作为真实应用组合 gate。每个 owner 在修改其 owned output 的
同一绿色提交中新增对应 top-level exact test；不得在 T2 预提交未来 owner 的红测，也不得让
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

T2 只实现 reusable harness、deterministic fixtures 与拒绝旧 SHA、空 evidence、伪
verified、placeholder key、`Unknown` 自动重试、ignored exact test、环境不匹配和 smoke-only
benchmark 的负例；`gh68_harness_contract` 在未迁移 baseline 上也必须绿色。baseline root
cause 只写入 `$GH68_EVIDENCE_DIR/root-cause/` scratch evidence，不把红测提交到 repository。
T3、T4、T5、T6 分别只运行 `gh68_chat_tutorial_contract`、
`gh68_fullscreen_example_contract`、`gh68_inline_example_contract`、
`gh68_provider_example_contract`；全局 `gh68_example_convergence_contract` 只能在四项全部
完成后的 T6 与 T10 运行。docs、benchmark/coverage 与 CI tests 同理由 T7、T8、T9 在各自
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

baseline artifact 不提交生成物到仓库。T8 必须在固定环境设置
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
  cargo test --test gh68_benchmark_metadata_contract --locked -- --exact
GH68_BENCHMARK_MODE=validate \
GH68_BENCHMARK_BASELINE="$GH68_EVIDENCE_DIR/benchmark-baseline.json" \
GH68_BENCHMARK_EVIDENCE="$GH68_EVIDENCE_DIR/benchmark.json" \
GH68_IMPLEMENTATION_HEAD="$IMPLEMENTATION_HEAD" \
GH68_BASE_MAIN_SHA="$BASE_MAIN_SHA" \
  cargo test --test gh68_benchmark_comparison_contract --locked -- --exact
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

````

producer 必须从 `git diff --unified=0 "$BASE_MAIN_SHA...$IMPLEMENTATION_HEAD"gh68-critical-paths-v1` block 计算，不得接受调用者传入的摘要。`coverage.json` schema：

```json
{
  "schema": "gh68-coverage-v1",
  "head_sha": "<40-hex>",
  "base_main_sha": "<40-hex>",
  "raw_sha256": "<64-hex>",
  "generated_at": "<RFC3339>",
  "changed_executable": {"total": 1, "covered": 1, "percent": 100.0},
  "critical": [{
    "file": "tests/golden_real_apps.rs",
    "name": "gh68_harness_contract",
    "executable": 1,
    "covered": 1,
    "percent": 100.0
  }]
}
```

validator 必须重新 hash raw artifact、重新读取 task-plan exact `file + name` set，要求
changed executable `total > 0` 且 `percent >= 80.0`，critical set 严格相等、每项
`executable > 0` 且 `percent == 100.0`，head/base 等于当前 immutable window。缺少、空/旧 raw、零 changed executable、unknown function、重复/额外 critical
entry 或 `continue-on-error` 结果全部 fail closed。
`collectproduce` 与 `validate`
两次 exact invocations。coverage exact test 缺失/越界 mode、raw/evidence absolute path 或
head/base binding 时失败；`collect` 要求 writable destinations，produce/validate 要求文件存在。

### 8. CI gates

required GH68 job 必须用 `actions/checkout` 的
`${{ github.event.pull_request.head.sha }}`，并在 runner 上先执行：

```bash
set -euo pipefail; test "$(git rev-parse HEAD)" = "$GH68_PR_HEAD_SHA"
test "$GH68_PR_HEAD_SHA" = \
  "$(gh api "repos/majiayu000/rnk/pulls/$GH68_PR_NUMBER" --jq .head.sha)"
mkdir -p "$GH68_EVIDENCE_DIR/baseline-download"
gh run download "$GH68_BASELINE_RUN_ID" --repo majiayu000/rnk \
  --name "$GH68_BASELINE_ARTIFACT_NAME" \
  --dir "$GH68_EVIDENCE_DIR/baseline-download"
cp "$GH68_EVIDENCE_DIR/baseline-download/benchmark-baseline.json" \
  "$GH68_EVIDENCE_DIR/benchmark-baseline.json"
test "$(shasum -a 256 "$GH68_EVIDENCE_DIR/benchmark-baseline.json" |
  awk '{print $1}')" = "$GH68_BASELINE_SHA256"
jq -e --argjson run "$GH68_BASELINE_RUN_ID" --arg name "$GH68_BASELINE_ARTIFACT_NAME" \
  --arg workflow "$GH68_BASELINE_WORKFLOW" \
  '.coordinate.repository=="majiayu000/rnk" and .coordinate.workflow==$workflow
   and .coordinate.run_id==$run and .coordinate.artifact_name==$name' \
  "$GH68_EVIDENCE_DIR/benchmark-baseline.json"
``MODE=validate` 与全部 artifact paths → workspace full suite/15 exact tests/examples →
benchmark smoke；`CI Gate` 依赖该 job。merge-ref、旧 T8/T9 artifact、`continue-on-error`
coverage 或 smoke 都不能冒充 current-head/fixed-environment performance evidence。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | 四个 examples 与 `examples/README.md` purpose ledger | T3–T6 四个 focused tests；T6/T10 `gh68_example_convergence_contract`；T7 `gh68_example_index_contract` |
| B-002 | examples parity fixtures、goldens 与 index | 四个 focused tests 分别覆盖 owner observable parity；T6/T10 convergence 覆盖删除前置条件 |
| B-003 | `examples/README.md`、CI index gate | `cargo test --test gh68_example_index_contract --locked -- --exact`，要求闭集分类、唯一项、文件存在且 internal 不进入 public README |
| B-004 | 四个 example implementation | 每个 focused test 只检查当前 owner 的 public API behavior 与禁止 private ownership；全局 convergence 只在 T6/T10 检查四项集合 |
| B-005 | 两个 application adapter、public reducer/view fixture | `gh68_chat_tutorial_contract` 与 `gh68_provider_example_contract` 比较 updates、final state 与 semantic snapshot；离线执行 |
| B-006 | legacy `Message` golden 与 compatibility docs | `cargo test --test gh68_message_compatibility_contract --locked -- --exact`，编译 legacy constructors 并比较 role/text golden |
| B-007 | `docs/API_STABILITY.md`、README imports | `cargo test --test gh68_public_docs_contract --locked -- --exact`，逐项断言 API status/import/migration/deprecation fields 非空 |
| B-008 | README Inline/Fullscreen quickstarts | `cargo test --test gh68_public_docs_contract --locked -- --exact`；对两个 rust snippets 执行 doctest 或 compile fixture |
| B-009 | README update/renderer/keymap/error/non-goals sections | `cargo test --test gh68_public_docs_contract --locked -- --exact`，逐节断言 public APIs 与 app-owned side-effect boundary |
| B-010 | public fixtures 与 adapter empty/error paths | `gh68_chat_tutorial_contract` 与 `gh68_provider_example_contract` 覆盖 empty conversation/text/blocks/metadata/adapter 且无 invented data |
| B-011 | public composer interaction fixture | `gh68_chat_tutorial_contract` 与 `gh68_inline_example_contract` 覆盖 CJK/emoji/combining/ZWJ/CRLF/multi-char/paste |
| B-012 | Fullscreen focus/resize fixture | `gh68_fullscreen_example_contract` 覆盖 focus/keymap/min-size/连续 resize/input interleave 且 draft/transcript/anchor 不漂移 |
| B-013 | `real_app_chat` plain/ANSI goldens | T6/T10 `gh68_example_convergence_contract` 去 ANSI 后比较 semantic text/status/order/errors |
| B-014 | GH66/GH67 terminal harness 与 Inline/Fullscreen orchestration | `gh68_fullscreen_example_contract` 自身覆盖 Fullscreen normal/cancel/typed-failure/panic-unwind 与 raw/cursor/alternate-screen/input-mode restoration；`gh68_inline_example_contract` 独立覆盖 Inline；provider test 不替代任一 shell |
| B-015 | `benches/render.rs` GH68 workloads、stress oracle | `cargo test --test gh68_stress_correctness_contract --locked -- --exact`；`cargo bench --bench render --no-run --locked`；固定环境运行 `cargo bench --bench render --locked -- gh68` |
| B-016 | `$GH68_EVIDENCE_DIR/benchmark.json` | producer invocation 后 `gh68_benchmark_metadata_contract` 校验实际 samples 与完整 metadata；缺字段、旧 SHA 或环境不匹配失败 |
| B-017 | benchmark comparison gate | validate invocation 的 `gh68_benchmark_comparison_contract` 读取实际 artifact；正/负 fixtures 仅补充覆盖 20%、3*MAD、1ms floor、环境不匹配与 smoke-only |
| B-018 | `docs/TERMINAL_COMPATIBILITY.md` Chat matrix | `cargo test --test gh68_compatibility_matrix_contract --locked -- --exact`，枚举 OS/terminal/Inline/Fullscreen/paste/resize/restoration/tmux/SSH；无 evidence 的 verified fixture 必须失败 |
| B-019 | `.github/workflows/ci.yml`、examples index | `gh68_ci_public_examples_contract` 验证 PR exact-head checkout、immutable baseline download/digest 与 producer-before-suite 顺序；examples check |
| B-020 | CI mapped exact tests 与 goldens | export validate modes/paths 后对 critical manifest 15 tests 逐名运行 helper；缺 mode/path/artifact 必须失败 |
| B-021 | phased current-head verification window | T1 initial 允许 head==base；T8/T9/T10 final 要求 strict base；按 evidence-first 顺序执行，再 final after + `validate_gh68_window` |
| B-022 | public reducer、MessageList/shell stress fixture | `gh68_fullscreen_example_contract` 与 `gh68_stress_correctness_contract` 交错 delta/append/prepend/height/resize 并比较 order/identity/anchor/follow |
| B-023 | Inline typed outcomes 与 adapter failures | `gh68_inline_example_contract` 与 `gh68_provider_example_contract` 覆盖 cancel/fail/NotCommitted/Unknown/duplicate/retry/partial completion/late success |
| B-024 | `glm_chat` credential/tool boundary、offline tests | `gh68_provider_example_contract`；缺 key 时零请求，模型 tool output 不自动获得执行授权，tests 不访问网络 |
| B-025 | `$GH68_EVIDENCE_DIR/coverage.jsongh68_current_head_coverage_contract`；changed executable >=80%、critical set 相等且逐项 100% |
| B-026 | implementation authorization/dependency gate | preflight v2 producer/validator 验证 dependency issue/PR/Rust-source sets、spec main/non-closing linkage/exact files/review、readiness conflicts、digests 与 ancestry |
| B-027 | capability/degradation fixtures | 四个 focused tests 与 compatibility matrix 只允许 terminal optional capability 显式降级；data/order/layout/commit/restoration failures 全部失败 |
| B-028 | final atomic evidence audit | T10 convergence 加全 critical tests、benchmark/coverage JSON、matrix、CI 与 before/after preflight window；删除任一 artifact 后 audit 失败 |

## Verification Helpers

### Exact test helper

## Verification Plan

规格 packet：

- `git diff --check`
- B-ID 集合等于 tech mapping 集合与 tasks `Covers:` union。
- planned-changes manifest 恰好一份、`issue=68`、`complete=true`，所有 paths 为真实
  repository-relative paths。
- 使用可信固定 revision 的 SpecRail pack 在临时镜像中运行
  `checks/check_workflow.py --repo <mirror> --spec-dir specs/GH68`。
- `python3 .github/scripts/check_markdown_links.py specs/GH68`

未来 implementation：

1. T1 只执行 `phase=initial` invocation；current-head evidence owner fresh 执行
   `phase=final` invocation。
2. 按上文 benchmark producer/validator 生成并验证 current-head `benchmark.json`。
3. 按上文 llvm-cov `collect`、coverage `produce`、coverage `validate` 生成并验证
   `llvm-cov.json`/`coverage.json`。
4. `export GH68_BENCHMARK_MODE=validate GH68_COVERAGE_MODE=validate`，并 export
   baseline/benchmark/raw/coverage absolute paths、`IMPLEMENTATION_HEAD`、`BASE_MAIN_SHA`。
5. 运行 `cargo fmt --all -- --check`、workspace all-target check/test、all examples check、
   full golden target、15 个 exact contract tests 和 benchmark no-run smoke；所有
   evidence-dependent tests 此时必须读取 current-head artifacts。
6. T10/CI capture `phase=final` after artifact，运行 `validate_gh68_window`。任一步造成 head
   变化时丢弃 artifacts，从第 1 步的 current-head final invocation 重建全部。

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

- 当前只授权 spec work；本文件不授予 production edit、`ready_to_implement`、PR approval 或 merge。
- Implementation owner 必须 fresh 读取最终 merged GH61/GH66/GH67 packets/code，以它们为唯一
  上游 truth。
- Verification owner 独立运行 exact tests、benchmark/coverage durable evidence 与同一
  preflight adapter/window validator；writer 自报结果不能替代独立 review。
