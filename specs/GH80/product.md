# Product Spec: stacked PR 的 CI 触发修复

Linked issue: #80 (https://github.com/majiayu000/rnk/issues/80)

## 1. 背景

`.github/workflows/ci.yml` 的 `pull_request` 触发限定 `branches: [main]`，
因此只有 base 为 `main` 的 PR 会运行 CI。当前的 spec 分支采用 stacked PR
（`#75`–`#79` 的 base 指向另一条 `spec/*` 分支），这些 PR 的
`statusCheckRollup` 恒为空。

这不是偶发的 CI 失败，而是配置导致的结构性缺失：无论重试多少次都不会产生
checks。依赖 hosted check 证据的 review 流程因此无法结案，PR #74 已重复到
round 10。

## 2. 目标

1. base 指向任意分支的 PR 都运行 CI，stacked PR 不再恒为 0 checks。
2. `push` 触发保持只在 `main`，不因每个分支推送重复跑完整 matrix。
3. CI 的 job 定义、matrix 与 `release.yml` 保持不变。

## 3. 非目标

1. 不改动 CI 的具体步骤、工具链版本或 matrix 组合。
2. 不改动 `release.yml` 的 tag 触发。
3. 不调整分支保护、merge 策略或 review 流程。
4. 不为 stacked PR 引入额外的 workflow 文件。

## 4. 验收标准

1. `.github/workflows/ci.yml` 的 `pull_request` 不再限定 `branches`。
2. `.github/workflows/ci.yml` 的 `push` 仍限定 `branches: [main]`。
3. 合并后新开的 base 非 `main` 的 PR 能看到与 base 为 `main` 的 PR 相同的 check 集合。
4. `release.yml` 无改动。
