# Product Spec: crates.io 发布 fail-fast 和 trusted publishing

Linked issue: https://github.com/majiayu000/rnk/issues/48

## 1. 背景

`v0.19.3` 发布时，GitHub Release workflow 因 `CARGO_REGISTRY_TOKEN` 为空
而在发布阶段失败。维护者随后使用本地 cargo credentials 发布，再 rerun
workflow 创建 GitHub Release。开源发布路径不能依赖这种本地兜底。

## 2. 目标

1. tag release 在真实 `cargo publish` 前发现 crates.io 查询错误、包元数据错误、
   package surface 错误和缺失发布凭证。
2. workflow 使用 crates.io trusted publishing / GitHub OIDC 获取短期 token，而不是
   长期 `CARGO_REGISTRY_TOKEN` secret。
3. 已发布的 crate version 必须可重复 rerun 并被显式跳过。
4. 未发布 crate 继续按 `rnk-style-core -> rnk-style -> rnk-icons -> rnk`
   顺序发布，并等待每个新版本在 crates.io 可见。
5. 文档说明 maintainer 必须在 crates.io 配置 trusted publisher，以及 tag 发布、
   rerun 和失败恢复步骤。

## 3. 非目标

1. 不在仓库中保存 crates.io token。
2. 不自动创建 crates.io trusted publisher 配置；这是 crates.io maintainer gate。
3. 不改变 crate 版本号或发布顺序。
4. 不创建 GitHub Release，除非 publish job 成功。

## 4. 验收标准

1. `.github/workflows/release.yml` 不再读取 `secrets.CARGO_REGISTRY_TOKEN`。
2. publish job 拥有最小权限：`contents: read` 和 `id-token: write`。
3. crates.io 版本查询为三态：exists、missing、error；只有 missing 允许 publish。
4. trusted publishing auth 只在存在未发布 package 时执行。
5. `docs/RELEASING.md` 记录 trusted publishing 设置、release gates、rerun 行为和
   recovery。
6. PR 模板提醒 release-touching PR 检查 trusted publishing 或 release policy。
