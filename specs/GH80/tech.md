# Tech Spec: stacked PR 的 CI 触发修复

Linked issue: #80 (https://github.com/majiayu000/rnk/issues/80)

## 1. 设计

从 `.github/workflows/ci.yml` 的 `pull_request` 触发中移除 `branches: [main]`
过滤：

```yaml
on:
  push:
    branches: [main]
  pull_request:
```

GitHub Actions 在 `pull_request` 省略 `branches` 时对所有 base 分支触发，
因此 stacked PR 与 base 为 `main` 的 PR 得到同一套 checks。

`push` 保留 `branches: [main]`：分支上的验证由 PR 事件覆盖，无需在每次
推送时重复运行完整 matrix。

现有 `concurrency` 配置按 `github.ref` 分组且对非 `main` 取消进行中的运行，
stacked PR 的连续推送不会累积并发任务。

## 2. 影响文件

1. `.github/workflows/ci.yml`
2. `specs/GH80/*`

## 3. 风险

1. CI 运行次数增加（此前 stacked PR 完全不跑）。
   - 缓解：这是本 issue 要恢复的覆盖，且 `concurrency.cancel-in-progress`
     对非 `main` 生效，旧运行会被取消。
2. stacked PR 的 diff 包含其 base 分支尚未合并的提交，CI 结果反映的是
   整条链的状态而非单个 PR 的增量。
   - 缓解：这与 GitHub 的 merge-ref 语义一致；review 侧按 base 关系解读，
     本 issue 不改变该语义。
3. 本改动生效范围是合并到 `main` 之后创建或更新的 PR。
   - 缓解：验收标准 3 以合并后新开 PR 为准；已存在的 PR 需重新推送触发。
