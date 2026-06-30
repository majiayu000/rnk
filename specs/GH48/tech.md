# Tech Spec: crates.io 发布 fail-fast 和 trusted publishing

Linked issue: #48 (https://github.com/majiayu000/rnk/issues/48)

## 1. 设计

把 release workflow 拆成三段：

1. Preflight：校验 tag/version、package surface、README link 和 workspace tests。
2. Publish plan：查询 crates.io，输出需要发布的 package 列表；查询错误直接失败。
3. Publish：只有存在未发布 package 时才通过 `rust-lang/crates-io-auth-action@v1`
   取得短期 token，并按依赖顺序 dry-run/publish/wait。

## 2. 影响文件

1. `.github/workflows/release.yml`
   - 添加 workflow concurrency。
   - publish job 添加 `permissions: contents: read, id-token: write` 和 `environment: release`。
   - 使用 `.github/scripts/crates_io_release.py` 查询 package version 和 crates.io 状态。
   - 仅在 publish plan 非空时执行 trusted publishing auth 和 publish。
2. `.github/scripts/crates_io_release.py`
   - 提供 `package-version` 和 `version-status` 命令。
   - `version-status` exit code：`0` exists，`10` missing，`20` network/server/auth error。
3. `docs/RELEASING.md`
   - 记录 trusted publisher 配置：owner/repo/workflow/environment。
   - 记录 rerun 已发布版本的 skip 行为。
4. `.github/PULL_REQUEST_TEMPLATE.md`
   - 增加 release-touching PR checklist。
5. `CHANGELOG.md`
   - 在 Unreleased 记录发布自动化变更。

## 3. 风险

1. crates.io trusted publisher 未配置会导致 auth step 失败。
   - 缓解：文档保留 maintainer gate，并要求在下一次 tag release 前配置。
2. `environment: release` 需要 crates.io publisher 配置中的 environment 一致。
   - 缓解：文档明确 environment name。
3. 本地无法完整模拟 GitHub OIDC。
   - 缓解：本地验证 workflow syntax、Python helper、cargo package/check；最终验收在 tag run。

## 4. 验证

1. `python3 .github/scripts/crates_io_release.py package-version rnk`
2. `python3 .github/scripts/crates_io_release.py version-status rnk 0.19.3`
3. `python3 -m py_compile .github/scripts/crates_io_release.py`
4. `cargo package --list -p rnk --locked`
5. `cargo check --workspace --locked`
6. `actionlint .github/workflows/release.yml` when available

## 5. 回滚

回滚本 PR 可恢复长期 token 发布路径。若 trusted publisher 已在 crates.io 配置，回滚不
会删除 crates.io 端配置，需要 maintainer 手动调整。
