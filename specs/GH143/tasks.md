# Task Plan：定向流式更新成本收敛

## Linked Issue

GH-143

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)
- Parent contract: [`../GH62/product.md`](../GH62/product.md),
  [`../GH62/tech.md`](../GH62/tech.md), [`../GH62/tasks.md`](../GH62/tasks.md)

## 实现任务

- [ ] `SP143-T1` 实现 private `MessageIndex`，接入 `ConversationState::new`、`message`、`try_restore` 以及 Push/Delete/Resend/rollback 的原子维护。Covers: B-004 B-007 B-008 B-010 B-012 | Owner: `state-index-lane` | Dependencies: product/tech spec 通过 workflow 与 depth gates | Done when: active id 始终解析到 ordered Vec 的正确位置；unknown id 返回现有 `None` / typed error；restore 与 rollback 不做 silent scan fallback；公开 snapshot parts 不变 | Verify: `cargo test --test chat_targeted_updates push_delete_resend_keep_lookup_and_order_consistent -- --exact`; `cargo test --test chat_targeted_updates snapshot_restore_rebuilds_target_lookup -- --exact`

- [ ] `SP143-T2` 在 `reducer/targeted.rs` 实现 AppendText / non-correlation Complete 单目标路径、direct revision/outcome 构造与五维 deterministic counters；保留 correlation-bearing Complete、Cancel、Fail 的显式跨消息路径。Covers: B-001 B-002 B-003 B-004 B-005 B-009 B-011 B-013 | Owner: `targeted-reducer-lane` | Dependencies: SP143-T1 | Done when: 1 与 10,001 messages、front/end target 的 unrelated visits 相同且为 0，target lookup 为 1；成功路径只推进目标；跨消息路径的访问被诚实计数 | Verify: `cargo test --lib components::chat::reducer::targeted::tests::append_cost_is_transcript_independent -- --exact`; `cargo test --lib components::chat::reducer::targeted::tests::complete_cost_is_transcript_independent -- --exact`; `cargo test --lib components::chat::reducer::targeted::tests::front_and_end_targets_have_equal_cost -- --exact`; `cargo test --lib components::chat::reducer::targeted::tests::correlated_complete_records_global_visits -- --exact`; `cargo test --lib components::chat::reducer::targeted::tests::cost_dimensions_are_independent -- --exact`; `cargo test --lib components::chat::reducer::targeted::tests::local_paths_skip_global_work -- --exact`

- [ ] `SP143-T3` 新增 public-contract integration fixtures，覆盖四种 append block、accepted/rejected atomicity、direct outcome、identity mutation、snapshot restore 与 existing replay/proof compatibility；不得修改或弱化 `tests/chat_conversation_contracts.rs`。Covers: B-003 B-005 B-006 B-007 B-008 B-010 B-012 | Owner: `targeted-contract-test-lane` | Dependencies: SP143-T1, SP143-T2 | Done when: 新 fixtures 只使用 public API；每个拒绝断言完整 state/snapshot 不变；现有 chat suite 原样全绿 | Verify: `cargo test --test chat_targeted_updates --locked`; `cargo test --test chat_conversation_contracts --locked`

- [ ] `SP143-T4` 完成规格一致性、覆盖率、文件大小、workspace 和 exact-head handoff。Covers: B-001 B-002 B-003 B-004 B-005 B-006 B-007 B-008 B-009 B-010 B-011 B-012 B-013 | Owner: `verification-lane` | Dependencies: SP143-T1, SP143-T2, SP143-T3 | Done when: product/tech/tasks 的 B-set 完全一致；planned paths 无越界；新代码行覆盖 ≥80%、index/fast path 关键分支 100%；所有生产和测试文件 ≤800 行；独立 reviewer、current-head CI、review-thread 与 PR gates 均有新鲜证据 | Verify: `python3 checks/check_workflow.py --repo . --spec-dir=specs/GH143`; `python3 tools/spec_depth_audit.py --spec-dir specs/GH143 --gate`; `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --all-features --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`; `cargo test --workspace --all-targets --all-features --locked`; `cargo test --doc --workspace --all-features --locked`; `wc -l src/components/chat/state.rs src/components/chat/state/message_index.rs src/components/chat/state/tests.rs src/components/chat/reducer.rs src/components/chat/reducer/targeted.rs tests/chat_targeted_updates.rs`

## 并行拆分

本 tranche 由一个 writer 串行实现，避免 `ConversationState` 与 reducer 事务边界被两个
lane 同时修改：T1 → T2 → T3 → T4。独立 reviewer 只读，不持有任何 writable path。

## 验证

- Product / tech / tasks 的 invariants 均为
  `B-001..B-013`，task Covers union 无缺项。
- SpecRail workflow、depth、planned-changes 与 implement route gates 通过。
- 所有 focused、workspace、doc 与 coverage commands 产生当前 head 的新鲜输出。
- PR exact head 的 CI rollup 全绿、unresolved non-outdated actionable threads 为 0。

## Handoff Notes

- `pr_tier: standard`，交付为一个 `mixed_impl` PR；不额外创建 spec-only PR。
- `Refs #143` 直到完整验收与 gates 通过；只有 final PR body 才可使用 `Fixes #143`。
- `MessageIndex` 是 private derived state，不得加入公开 snapshot 或 prelude。
- correlation-bearing Complete 是显式保留的跨消息 fallback；不得用取消校验换取计数通过。
- 主 checkout 有用户改动，所有实现、测试、提交与 push 只在隔离 GH143 worktree 进行。
