# Product Spec: stacked PR 的 CI 触发修复

## Linked Issue

GH-80（https://github.com/majiayu000/rnk/issues/80）

complexity: small

### Readiness record

- 2026-07-24：仓库维护者在当前 `implx auto` 会话中授权处理完整队列；issue
  使用仓库规范的连字符标签 `ready-to-spec` 进入规格编写状态。
- 本 packet、真实 non-main base probe 和 exact-head 独立复审均通过之前，
  `ready-to-implement` 不得应用；已有的一行 workflow 改动也不得因此合并。
- 独立复审接受 exact head 后，维护者才把 issue 从 `ready-to-spec` 转为
  `ready-to-implement`。这两个标签不得同时存在。

## 用户问题

`.github/workflows/ci.yml` 把 `pull_request` 限定为 base `main`。因此 base
指向另一条 `spec/*` 分支的 stacked PR（#75–#79）不会产生 hosted checks，
依赖 CI 证据的 review/merge gate 永远无法结案。这是触发配置造成的结构性缺失，
不是重试可以恢复的偶发失败。

## 目标

1. 任何 base 分支上的 pull request 都运行现有 CI。
2. `push` 仍只对 `main` 运行，避免分支 push 与 PR 事件重复执行完整 matrix。
3. all-base 触发只扩大事件覆盖，不改变 CI 的 named check 集合或 release workflow。
4. 合并前用真实 non-main base probe 证明 GitHub hosted dispatch，而不是仅解析 YAML。

## 非目标

1. 不改动 CI job、步骤、工具链版本、matrix、required checks 或分支保护。
2. 不改动 `.github/workflows/release.yml` 的 tag 触发或发布权限。
3. 不调整 stacked PR 的 merge-ref 语义、merge 策略或 review 流程。
4. 不引入 `pull_request_target`、额外 workflow 或 secrets。

## Behavior Invariants

1. B-001 当 pull request 被 `opened`、`synchronize` 或 `reopened` 时，无论其
   base 是 `main`、`spec/*` 还是其他合法分支，GitHub 都必须派发 `CI`
   workflow；base 名称不得成为过滤条件。
2. B-002 当事件是 branch `push` 时，只有 `refs/heads/main` 必须派发该
   workflow；非 `main` 分支 push 不得新增一套重复的完整 CI。
3. B-003 对绑定 exact base/head SHA 的 non-main probe 与 base 为 `main` 的
   control，terminal rollup 的规范化 `{workflowName, name}` 集合必须完全相等，
   且每个 check 的 terminal conclusion 必须为 `SUCCESS`；相同 count 不能代替
   名称、workflow 和 conclusion 的逐项比较。
4. B-004 `.github/workflows/release.yml` 及 `.github/workflows/ci.yml` 的
   jobs、matrix、steps、permissions 和 concurrency 必须与变更前相同；唯一
   workflow 行为差异是移除 `pull_request.branches: [main]`。
5. B-005 合并证据必须同时记录 control PR 与 probe PR 的 URL、base ref、
   exact base SHA、exact head SHA、规范化 check 列表及 terminal conclusions。
   任一 SHA/字段缺失、rollup 为空、仍为 pending/cancelled/failure，或只比较
   数量时，验收必须 fail closed。
6. B-006 CI 的不受信任 PR 边界必须继续使用 `pull_request`，不得改为
   `pull_request_target`；本变更不得新增写权限、secret 引用或高权限
   `GITHUB_TOKEN`。仓库当前 `default_workflow_permissions` 为 `read`，本变更
   不得改变该 posture。
7. B-007 issue 必须先以唯一的 canonical `ready-to-spec` 标签进入 packet
   编写；只有真实 probe 和 exact-head 独立复审均接受后，维护者才能原子地移除
   `ready-to-spec` 并应用唯一的 `ready-to-implement`。标签缺失、两者并存或
   使用 underscore alias 时，readiness gate 必须阻断。

## 验收标准

- [ ] 真实 child/probe PR 以 `fix/GH80-stacked-pr-ci` 为 non-main base，
      并在 PR #81 合并前产生 hosted checks。
- [ ] B-003、B-005 的 exact-head 规范化比较通过，且证据可由 PR URL 重查。
- [x] `push` 仍限定 `main`，`pull_request` 不再包含 base filter。
- [x] CI job 定义和 `.github/workflows/release.yml` 无差异。
- [x] workflow 仍使用 `pull_request`，且没有新增权限或 secret。
- [ ] exact-head 独立复审接受后，issue 从唯一 `ready-to-spec` 转为唯一
      `ready-to-implement`。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-005；空 rollup、缺失 SHA 或缺失字段必须阻断 |
| 错误与失败路径 | covered: B-003, B-005；非成功 terminal conclusion 不得视为通过 |
| 授权/权限 | covered: B-006；不扩大 PR token、secret 或 workflow 权限 |
| 并发/竞态 | covered: B-003, B-005；只比较同一 exact-head snapshot 的 terminal rollup |
| 重试/幂等 | covered: B-005；重跑后必须重新绑定当前 exact heads，不得复用陈旧证据 |
| 非法状态转换 | covered: B-007；readiness 必须按顺序且标签互斥 |
| 兼容/迁移 | covered: B-002, B-003, B-004；保留 push、named jobs 和 release 行为 |
| 降级/回退 | covered: B-005；无 hosted proof 时 fail closed，不以 YAML proof 降级放行 |
| 证据与审计完整性 | covered: B-003, B-005；禁止 count-only 或未绑定 SHA 的结论 |
| 取消/中断 | covered: B-005；cancelled/pending run 不能成为完成证据，恢复后重新采集 |

## 发布说明

无需用户迁移。合并后，base 非 `main` 的 PR 也会消耗 CI runner；如果出现异常
负载或 required-check pending，按 tech spec 的监控和回滚步骤恢复 base filter。
