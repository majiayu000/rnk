# Task Plan

## Linked Issue

GH-57: https://github.com/majiayu000/rnk/issues/57

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`
- Architecture baseline: `docs/CHAT_UI_COMPONENT_ARCHITECTURE.md`

<!-- gh57-child-ledger-v1
{"version":1,"children":[
{"issue":58,"lane":"F1","depends_on":[]},
{"issue":59,"lane":"F2","depends_on":[58]},
{"issue":60,"lane":"F3","depends_on":[59]},
{"issue":61,"lane":"F4","depends_on":[58,59,60]},
{"issue":62,"lane":"M1","depends_on":[]},
{"issue":63,"lane":"V1","depends_on":[58,62]},
{"issue":64,"lane":"C1","depends_on":[58,60]},
{"issue":65,"lane":"L1","depends_on":[58,60,62]},
{"issue":66,"lane":"S1","depends_on":[62,63,64]},
{"issue":67,"lane":"S2","depends_on":[62,63,64,65]},
{"issue":68,"lane":"H1","depends_on":[61,66,67]}
]}
-->

## 范围与授权边界

本 umbrella task plan 只规划 GH-57 的规格合同、GH-58 至 GH-68 child queue、独立审查和
最终 closure audit。它不授权修改生产代码，也不授权任何 child 开始实现。每个 child 必须
拥有独立的 `product.md`、`tech.md`、`tasks.md` 和 implementation PR；只有人工批准该 child
规格并授予 `ready-to-implement` 后才能实现。所有最终 PR approval、merge 和 GH-57 关闭
仍由人类决定。

## 规格与队列任务

- [ ] `SP57-T1` Owner: `spec-coordinator` | Done when: committed exact HEAD 的 product、tech、tasks 与 architecture 四文件形成一致 umbrella packet，包含 child 起点、终点、实现授权、人类 gate 与 closure audit 五个边界概念，并获得人工 spec approval | Verify: 四份文件均出现 child 起点/终点、实现授权、人类 gate 与 closure audit 五个边界概念
  - Dependencies: 无；仅允许在 `ready-to-spec` 下起草。
  - Covers: B-001, B-002, B-003, B-024。
  - Handoff: 审查者必须先核对三份规格与 issue #57；人工未批准前，不得设置 GH-57 或任何 child 为 `ready-to-implement`，不得从本任务派生实现分支。

- [ ] `SP57-T2` Owner: `queue-coordinator` | Done when: GH-58 至 GH-68 全部映射到 F1、F2、F3、F4、M1、V1、C1、L1、S1、S2、H1，依赖、readiness、独立 artifact/PR 责任和 umbrella 引用规则均写入 committed、machine-checked queue ledger | Verify: queue ledger 已 committed，且 GH-58 至 GH-68 逐项映射到对应 lane 与依赖边
  - Dependencies: SP57-T1 的合同内容稳定；不要求 child implementation 开始。
  - Covers: B-001, B-002, B-003, B-024。
  - Handoff: 每个 child 的 spec-only PR 和 implementation PR 必须分开追踪；child PR 可 `Refs #57`，不得对 GH-57 使用 `Fixes` 或 `Closes`。

- [ ] `SP57-T3` Owner: `layout-spec-lane` | Done when: GH-58 至 GH-61 各自具备独立 SpecRail product/tech/tasks，覆盖 TextFlow、keyed identity/order、transactional patch/error、LayoutSnapshot/parity/benchmark，每个 spec PR exact diff 仅含自己的三份 packet 文件，并保留依赖门禁 | Verify: 逐个确认这些 child 的 spec PR diff 只含自己的 `specs/GHnn/` 三份文件
  - Dependencies: SP57-T2；规格起草可并行，implementation 必须服从 child 依赖和人工 `ready-to-implement`。
  - Covers: B-002, B-003, B-016, B-017, B-020, B-023, B-025。
  - Handoff: 只交付 child spec packets 与门禁证据；不得在本 lane 修改布局、renderer、reconciler 或测试生产代码。

- [ ] `SP57-T4` Owner: `chat-primitives-spec-lane` | Done when: GH-62 至 GH-65 各自具备独立且 spec-only 的 packet；GH-63 覆盖 typed blocks；GH-64 覆盖 submit/newline、paste、selection、auto-grow、CJK/emoji/combining/CRLF、TextFlow、typed error 与 atomic draft，并声明 exact test matched=1、passed=1、ignored=0；整体保留上游依赖 | Verify: 逐个确认这些 child 的 spec PR diff 只含自己的 `specs/GHnn/` 三份文件
  - Dependencies: SP57-T2；规格起草可并行，最终验收必须等待各 child 声明的 GH-58/GH-60/GH-62 上游门禁。
  - Covers: B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-014, B-015, B-018, B-019, B-020, B-021, B-022, B-023, B-025。
  - Handoff: 核心只定义和显示 typed state；不得加入 provider SDK、网络、密钥、持久化或工具执行，也不得重新定义已有 `InteractionMode` / `InteractionOutcome<T>`。

- [ ] `SP57-T5` Owner: `shell-hardening-spec-lane` | Done when: GH-66 至 GH-68 各自具备 spec-only packet；GH-66/67 exact evidence 覆盖 sink/terminal lifecycle 与恢复，GH-68 exact evidence 覆盖逐 example 收敛和双 adapter 状态等价，均要求 matched=passed=1、ignored=0，并保留依赖门禁 | Verify: 逐个确认这些 child 的 spec PR diff 只含自己的 `specs/GHnn/` 三份文件
  - Dependencies: SP57-T2；S1/S2/H1 的最终验收分别等待 issue #57 依赖图中的上游完成。
  - Covers: B-001, B-002, B-003, B-005, B-008, B-009, B-011, B-012, B-013, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025。
  - Handoff: child specs 必须把正常、取消、失败、重复完成、resize、退出和 panic 恢复写成可验证路径；spec lane 不得迁移 examples 或实现 shell。

## 独立审查与关闭任务

- [ ] `SP57-T6` Owner: `independent-spec-reviewer` | Done when: 独立 reviewer 对 GH-57 与全部 child spec packets 完成 B-ID、依赖、范围、负例、验证和人类 gate 审查，所有 blocking finding 已修复或由人类明确裁决；B-ID、SpecRail 与 Markdown links 都从 committed exact HEAD 的 planned four paths 验证，spec PR base 为 `main` 且不关闭 GH-57，当前 exact head 的 CI、APPROVED decision 与全部 review threads 均通过可失败检查 | Verify: spec PR base 为 `main`、不关闭 GH-57，当前 head 的 CI 通过、review 已 APPROVED、全部 review threads 已解决
  - Dependencies: SP57-T3、SP57-T4、SP57-T5；reviewer 必须与 spec writer 分离且保持只读。
  - Covers: B-002, B-003, B-023, B-024。
  - Handoff: 人工负责 spec approval 和 spec PR merge；即使审查与 CI 通过，agent 也不得自行批准、合并或授予 child 实现权限。

- [ ] `SP57-T7` Owner: `closure-auditor` | Done when: GH-57 audit 前后 OPEN；GH-58～68 全部完成/关闭，merge ancestry 符合 dependency ledger；每个 current-head coverage artifact 的 critical file/name 集合与 approved child ledger 严格相等并 100%，changed executable >=80%；workspace all-target/all-feature summary 非零且 ignored=0，mapped exact tests 逐名 matched=passed=1、ignored=0；验证窗口无 SHA/worktree 漂移，之后由人类决定关闭 | Verify: 在 clean `origin/main` checkout 上逐项核对下方 Closure 检查清单
  - Dependencies: SP57-T6、所有 child 的最终 implementation PR 与各自 merge gate；必须最后串行执行。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025。
  - Handoff: closure audit 只提交证据与关闭建议，不执行关闭或 merge；任一 child、验证、review thread、依赖或人工授权缺失时，GH-57 保持打开。

## Closure 检查清单

GH-57 是 umbrella issue，它自身不引入生产代码。关闭前由人类逐项确认：

- GH-58 至 GH-68 全部 child issue 已关闭，且各自的 implementation PR 已合入 main。
- 合并顺序符合本计划的 dependency ledger：依赖方的 merge commit 是被依赖方的后代。
- 每个 child 的 acceptance evidence 绑定其自身 PR 的当前 head，不复用其他 child 的结果。
- 各 child 的 Product-to-Test Mapping 中所有 exact test 在 main 上通过，
  `cargo test --workspace --all-targets --all-features --locked` 全绿。
- 新代码 changed-line coverage >=80%，各 child 声明的 critical paths 达到其声明阈值，
  由既有 CI Coverage job 报告。

任一项缺失时 GH-57 保持打开。关闭动作本身始终是人类决定，不由自动化执行。

## 并行拆分

- SP57-T3、SP57-T4、SP57-T5 可在 SP57-T2 后作为只读/规格 lane 并行，文件所有权分别限定为
  `specs/GH58..GH61/`、`specs/GH62..GH65/`、`specs/GH66..GH68/`，不得共享可写文件。
- child implementation 不属于本计划。未来实现必须按 GH-57 的依赖图调度；依赖未完成的下游
  只能只读规划，不得提前编辑或宣称最终验收通过。
- SP57-T6 是独立只读审查，SP57-T7 是所有 child 完成后的串行 closure audit。

## 验证

- GH-57 product invariant 集合与本任务计划覆盖集合均为完整的 B-001 至 B-025，无遗漏。
- Markdown 内部链接和 GH-57/GH-58..GH-68 引用可解析，`git diff --check` 通过。
- umbrella spec PR 仅包含架构/规格文档；若出现生产代码、实现测试或 workflow 修改则阻断。
- 人工 spec approval、每个 child 的 `ready-to-implement`、最终 PR approval/merge 和 GH-57
  closure 均保留为独立人类 gate。

## Handoff Notes

- 当前授权只到 `write_spec`；不得把 GH-57 的 `ready-to-spec` 解释为任何 child 的实现授权。
- GH-57 spec PR 合并后，由 coordinator 建立/更新 child queue ledger，并逐个收集人工 spec approval。
- 每个 child 使用自己的 acceptance evidence、当前 head SHA、CI、thread-aware review 状态和人工 merge
  授权；其他 child 的绿色结果不得替代。
- closure auditor 必须重新获取远端状态和新鲜验证，不得引用旧会话中的“之前通过”。
