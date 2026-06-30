# Tasks: GH48

Linked issue: https://github.com/majiayu000/rnk/issues/48

## SP48-T1 添加 crates.io 查询 helper

Owner: Codex

Done when:
- `.github/scripts/crates_io_release.py` 能读取 Cargo metadata package version。
- helper 能区分 crates.io exists / missing / error。

Verify:
- `python3 .github/scripts/crates_io_release.py package-version rnk`
- `python3 .github/scripts/crates_io_release.py version-status rnk 0.19.3`

## SP48-T2 更新 release workflow

Owner: Codex

Done when:
- workflow 不再引用 `secrets.CARGO_REGISTRY_TOKEN`。
- publish job 使用 OIDC 权限和 trusted publishing action。
- auth/publish 只在 publish plan 非空时执行。

Verify:
- `rg -n "CARGO_REGISTRY_TOKEN|crates-io-auth-action|id-token|publish-plan" .github/workflows/release.yml`

## SP48-T3 更新 release 文档和 PR 模板

Owner: Codex

Done when:
- `docs/RELEASING.md` 说明 trusted publisher 配置和 human gate。
- PR template 提醒 release-touching checks。
- `CHANGELOG.md` 记录 Unreleased 变更。

Verify:
- `rg -n "trusted publishing|Trusted Publisher|release-touching|OIDC" docs/RELEASING.md .github/PULL_REQUEST_TEMPLATE.md CHANGELOG.md`

## SP48-T4 验证

Owner: Codex

Done when:
- Python helper compile 和 focused commands 通过。
- `cargo check --workspace --locked` 通过。
- 如果本机有 `actionlint`，release workflow syntax 检查通过。
