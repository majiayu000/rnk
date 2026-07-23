# Tasks: GH80

Linked issue: https://github.com/majiayu000/rnk/issues/80

- [ ] `SP80-T1` Owner: Codex. Done when: `.github/workflows/ci.yml` 的 `pull_request` 不再限定 `branches`，`push` 仍限定 `main`。 Verify: `python3 -c "import yaml; t=yaml.safe_load(open('.github/workflows/ci.yml'))[True]; assert t['pull_request'] is None and t['push']=={'branches':['main']}; print('ok', t)"`
- [ ] `SP80-T2` Owner: Codex. Done when: `release.yml` 无改动，CI 的 jobs/matrix 无改动。 Verify: `git diff --stat origin/main -- .github/workflows`
- [ ] `SP80-T3` Owner: Maintainer. Done when: 合并后新开的 base 非 `main` 的 PR 出现与 base 为 `main` 的 PR 相同的 check 集合。 Verify: `gh pr view <n> --json statusCheckRollup -q '.statusCheckRollup|length'`
