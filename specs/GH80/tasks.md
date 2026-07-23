# Task Plan: GH80

## Linked Issue

GH-80（https://github.com/majiayu000/rnk/issues/80）

## Spec Packet

- Product: `specs/GH80/product.md`
- Tech: `specs/GH80/tech.md`
- Current readiness: `ready-to-spec`（2026-07-24 已记录于 issue #80）
- Next readiness: exact-head 独立复审接受 packet 与 probe evidence 后，维护者
  移除 `ready-to-spec` 并应用 `ready-to-implement`；两者不得并存。

## 实现任务

- [x] `SP80-T1` 删除 PR base filter，保留 main-only push 与
  `pull_request` trust boundary。Covers: B-001, B-002, B-006. Owner: Codex.
  Dependencies: issue #80 的 `ready-to-spec` 维护者决定。Done when:
  `.github/workflows/ci.yml` 的 `pull_request` 无 `branches`，`push` 仍只含
  `main`，且无 `pull_request_target`、permissions 或 secrets。Verify:
  `python3 -c "import yaml; t=yaml.safe_load(open('.github/workflows/ci.yml'))[True]; assert t['pull_request'] is None and t['push']=={'branches':['main']}" && ! rg 'pull_request_target|secrets\\.|permissions:' .github/workflows/ci.yml`.
  Handoff: 实现已存在于 PR #81 exact head `7b0df2b0702c38120504103bc51b35f22887eb99`；
  后续只允许在原分支 normal push。
- [x] `SP80-T2` 证明 jobs/matrix/concurrency/release scope 未变化。Covers:
  B-004. Owner: Codex. Dependencies: SP80-T1. Done when: 相对 `origin/main`，
  CI workflow 仅删除 `pull_request` 下的一行 `branches: [main]`，release 无
  diff。Verify:
  `git diff --exit-code origin/main -- .github/workflows/release.yml && git diff --unified=0 origin/main -- .github/workflows/ci.yml`.
  Handoff: 若 diff 出现任何 job、matrix、step、permission 或 concurrency
  改动，立即阻断，不扩大本 issue scope。
- [ ] `SP80-T3` 在合并前执行真实 non-main hosted probe 并保存 exact-head
  parity evidence。Covers: B-001, B-003, B-005. Owner: Codex.
  Dependencies: SP80-T1, SP80-T2，以及 PR #81 首轮 spec 修订已 normal push。
  Done when: 临时 probe PR 的 base 为 `fix/GH80-stacked-pr-ci`，control/probe
  checks 全 terminal 且 normalized `{workflowName,name,conclusion}` 完全相等、
  全部 `SUCCESS`，证据记录双方 URL/base ref/exact base SHA/exact head SHA。
  Verify: 分别运行
  `gh pr view 81 --json url,baseRefName,baseRefOid,headRefOid,statusCheckRollup`
  与 `gh pr view <probe> --json url,baseRefName,baseRefOid,headRefOid,statusCheckRollup`，
  用 `jq` 排序后 `diff -u`。Handoff: **这是 merge blocker**；probe 不得合并，
  count-only、空 rollup、pending/cancelled/failure 或陈旧 SHA 均不得勾选。
- [ ] `SP80-T4` 完成 readiness、复审与交接。Covers: B-003, B-005, B-007.
  Owner: Maintainer/implx coordinator. Dependencies: SP80-T3、fresh exact-head
  独立 reviewer。Done when: reviewer 接受所有 B invariants 与证据，issue 只保留
  `ready-to-implement`，PR #81 的 threads/CI/gates 由 coordinator 单独处理。
  Verify: `gh issue view 80 --json labels`、`gh pr view 81 --json headRefOid,statusCheckRollup,reviewDecision`
  及 SpecRail review/pr gate evidence。Handoff: 本 lane 不 resolve threads、
  不 approve、不 merge；由 reviewer/coordinator 在新 exact head 上执行。

## 并行拆分

本修复只有一个 workflow 写入点，不进行并行写入。probe 使用独立临时分支，仅
拥有 probe marker；PR #81 原分支仅拥有 `.github/workflows/ci.yml` 与
`specs/GH80/*`。reviewer 为只读 lane，不与 writer 共享文件所有权。

## 验证

- [x] Static trigger、diff scope、release unchanged。
- [ ] SP80-T3 hosted control/probe exact-head parity。
- [ ] Fresh PR #81 checks、独立 review 与 SpecRail gates。

## Handoff Notes

- F001 的 hosted proof 不能延后到合并后；SP80-T3 保持 merge blocker，直到真实
  probe evidence 完成。
- Probe branch/PR 只用于验证，不能合并；验证结束后由 coordinator 决定关闭。
- Canonical readiness labels 使用 `ready-to-spec` / `ready-to-implement`
  （连字符），不得把 underscore alias 当作本 issue 的状态证据。
- 本 lane 只 normal push 原 PR #81，不 resolve review threads、不 approve、
  不 merge。
