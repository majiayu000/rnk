# Tech Spec: stacked PR 的 CI 触发修复

## Linked Issue

GH-80（https://github.com/majiayu000/rnk/issues/80）

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":80,"complete":true,"paths":[".github/workflows/ci.yml"],"spec_refs":["specs/GH80/product.md","specs/GH80/tasks.md","specs/GH80/tech.md"]}
-->

## Product Spec

`specs/GH80/product.md`

## Codebase Context

以下锚点已在 PR #81 写作时从 exact head
`7b0df2b0702c38120504103bc51b35f22887eb99` 读取确认。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| CI event trigger | `.github/workflows/ci.yml:3` | `push` 仅 `main`；PR #81 已把 `pull_request` 改为无 base filter | B-001、B-002、B-006 的唯一实现点 |
| CI concurrency | `.github/workflows/ci.yml:8` | 非 `main` ref 的旧 run 会被取消 | 扩大 PR 触发时保留原并发控制 |
| CI named jobs | `.github/workflows/ci.yml:16` | test matrix、workspace、MSRV、feature matrix、lint/docs/coverage/Miri 与 CI Gate | B-003、B-004 的 control 集合来源 |
| Release trigger/permissions | `.github/workflows/release.yml:3`, `.github/workflows/release.yml:21` | tag-only release，publish job 显式声明权限 | B-004 明确排除 release 与发布权限变化 |
| Product contract | `specs/GH80/product.md:1` | 记录 all-base、main-only push、named checks、证据与安全边界 | 所有 B invariants 的权威定义 |
| Execution plan | `specs/GH80/tasks.md:1` | 记录实现、probe、readiness 与 handoff gate | 防止 hosted proof 被推迟到合并后 |

## 设计方案

只从 `.github/workflows/ci.yml` 删除
`pull_request.branches: [main]`，保留：

```yaml
on:
  push:
    branches: [main]
  pull_request:
```

GitHub 由此对所有 PR base 派发同一个 `CI` workflow。`push`、jobs、matrix、
steps、concurrency 与 release workflow 均不变。实现不增加脚本、public API、
持久化数据或 fallback。

合并前创建一个最小临时 probe branch，其父提交为 PR #81 的 exact head，probe
PR 的 base 明确为 `fix/GH80-stacked-pr-ci`。probe 只增加无害验证标记，绝不
合并。等待 control 和 probe 的 rollup 全部 terminal 后，对每项提取
`{workflowName,name,conclusion}`，按 `(workflowName,name)` 排序并比较；
同时记录两边 exact base/head SHA。probe 的非 `main` base 才是 B-001 的真实
hosted dispatch 证明。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 all-base PR dispatch | `.github/workflows/ci.yml:6`、临时 child/probe PR | `gh pr view <probe> --json baseRefName,baseRefOid,headRefOid,statusCheckRollup`；base 必须为 `fix/GH80-stacked-pr-ci` 且 rollup 非空、全 terminal |
| B-002 main-only push | `.github/workflows/ci.yml:3` | `python3 -c "import yaml; t=yaml.safe_load(open('.github/workflows/ci.yml'))[True]; assert t['push']=={'branches':['main']} and t['pull_request'] is None"` |
| B-003 exact named check parity | PR #81 control 与 probe rollup | 用 `jq '[.statusCheckRollup[]|{workflowName,name,conclusion}]|sort_by(.workflowName,.name)'` 规范化两份 exact-head JSON，再以 `diff -u` 比较；两边 conclusion 均为 `SUCCESS` |
| B-004 jobs/release unchanged | `.github/workflows/ci.yml:8` 起与 `.github/workflows/release.yml` | `git diff --exit-code origin/main -- .github/workflows/release.yml`；`git diff --unified=0 origin/main -- .github/workflows/ci.yml` 必须只含删除 `branches: [main]` |
| B-005 evidence fail closed | 本地 `.specrail/runtime/evidence/gh80-*` 证据与 PR URL | evidence JSON 必须含 control/probe PR、base ref、exact base/head、规范化集合、terminal conclusion；使用 `jq -e` 拒绝空/pending/non-success/count-only 数据 |
| B-006 pull_request trust boundary | `.github/workflows/ci.yml:3`、repository Actions permissions | `rg -n 'pull_request_target|secrets\\.|permissions:' .github/workflows/ci.yml` 必须无匹配，`pull_request:` 必须有且不带 filter；`gh api repos/majiayu000/rnk/actions/permissions/workflow --jq .default_workflow_permissions` 必须为 `read` |
| B-007 canonical readiness transition | issue #80 labels、exact-head review/probe evidence | `gh issue view 80 --json labels` 在 spec review 阶段只含 `ready-to-spec`；probe/review 接受后只含 `ready-to-implement`，underscore alias 与双标签均失败 |

## 数据流

GitHub `pull_request` 事件 → 读取 PR merge ref → 现有 `CI` workflow/job matrix →
GitHub check rollup。仓库不新增持久化或外部 API 调用。验证侧只通过 `gh` 读取
control/probe PR 元数据和 checks，规范化后写入本地 runtime evidence；该证据
不包含 token、secret 或用户数据。

## 备选方案

- 为每条 stacked base 写入 `branches`：分支集合会持续变化，容易再次漏配。
- 增加 `push` 的所有分支触发：会让同一提交同时由 push 与 PR 执行完整 matrix。
- 使用 `pull_request_target`：会扩大不受信任 PR 的权限边界，明确拒绝。
- 仅用 YAML parser 或 check count：能证明语法/数量，不能证明 GitHub 对
  non-main base 的真实派发或 named check parity，明确拒绝。

## 风险

- Security: `pull_request` 继续是 trust boundary；不使用
  `pull_request_target`，不新增 `permissions` 或 secret 引用。2026-07-24
  通过 GitHub API 读取仓库 `default_workflow_permissions=read` 且
  `can_approve_pull_request_reviews=false`；本变更不修改 repository setting，
  也不请求 workflow 级提权。
- Compatibility: required-check 名称必须保持一致；B-003 比较
  `{workflowName,name}`，B-004 保证 jobs/matrix/release 未变。
- Performance: 之前被过滤的 stacked PR 会新增约 16 个 checks。保留
  `concurrency.cancel-in-progress`，连续更新会取消旧 run；监控排队时间、取消率
  和 Actions 用量。
- Maintenance: all-base 语义减少分支名单维护，但新 PR 都可能触发 CI。probe
  与 exact-head evidence 明确区分预期覆盖和意外触发。

## 测试计划

- [x] Static trigger parse：确认 `pull_request` 无 filter、`push` 仅 `main`。
- [x] Diff scope：CI 仅删除一行，release workflow 无差异。
- [ ] Hosted integration：non-main base probe 产生 checks，并与 PR #81 control
      的 normalized names/conclusions 精确相等。
- [ ] Final exact-head：PR #81 最终 push 后重新等待 control/probe terminal，
      重采 exact SHA-bound evidence。

## 回滚方案

若 all-base 触发导致异常 runner 负载、非预期 required-check pending、fork 安全
姿态变化或 check 名称漂移，revert PR #81 的 workflow commit，恢复：

```yaml
pull_request:
  branches: [main]
```

回滚后重新打开一个 non-main probe，预期其 rollup 为空；同时确认 base 为
`main` 的 control 仍为原 named check 集合。不要修改 jobs/matrix 或
`release.yml` 作为临时缓解。

## 监控

合并后的首批 stacked PR 逐一记录 Actions run URL、queue duration、cancelled
run 数和 normalized check set。若任一 required check 长期 pending、命名集合
与 control 不同、Actions 用量异常增长，立即停止继续合并 stacked PR 并执行
回滚；不得把缺失 check 视为成功。
