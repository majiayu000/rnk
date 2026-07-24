# Task Plan：后端无关的会话模型与状态机

## Linked Issue

GH-62: https://github.com/majiayu000/rnk/issues/62

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## 实现任务

- [ ] `SP62-T1`（child alias: `GH62-T1`）建立完整 public data model 与 private downstream skeletons | Owner: `chat-model-worker` | Done when: 创建全部五个 planned chat files；`mod.rs`/`model.rs` 发布完整 data-only types、private-field constructors/accessors 与 `non_exhaustive` enums（闭集 ChatRole 除外），包括 closed ChatRole/legacy mapping、Error/Thinking/ToolCall/ToolResult payload/accessors、canonical DecimalValue、NonZero MessageRevision、stable BlockId、closed TypedValue、FailureCause/status、closed metadata 与 typed updates；GH-63 只经 public borrowed payload/role/status accessors 投影；`error.rs`/`state.rs`/`reducer.rs` 为 private skeleton；公共模型无 Any/dynamic map/provider handle；cargo-metadata + non-code-stripped Rust audit 的 safe fixtures 成功，renamed dependency、std process/terminal、env macro、crate runtime fixtures 失败 | Verify: `verify_chat_missing_docs_gate`；`verify_forbidden_dependency_alias_detection`；`verify_no_forbidden_chat_dependencies origin/main`；`verify_chat_test message_revision_and_affected_outcome_are_typed`；`verify_chat_test every_block_variant_preserves_typed_data`；`verify_chat_test closed_typed_values_reject_invalid_payloads`；`verify_chat_test chat_message_metadata_is_closed_and_optional`；`verify_chat_test chat_roles_and_legacy_mapping_are_closed`；`verify_chat_test error_content_is_typed_and_source_aware`；`verify_chat_test decimal_values_have_one_canonical_representation`；`verify_chat_test lifecycle_payloads_are_closed_and_projectable`；`verify_chat_test empty_and_missing_inputs_have_explicit_results`；`verify_chat_test core_model_requires_adapter_owned_typed_values`；`verify_chat_test tool_and_thinking_models_have_no_execution_surface`
  - Dependencies: 本 packet 通过 SpecRail implementation gate。
  - Covers: B-002, B-003, B-018, B-019, B-021, B-025, B-026, B-028。
  - Test handoff: 创建 contract test 文件并只加入 SP62-T1 的 tests；完成验证后把
    `error.rs`/`state.rs` skeleton 串行交给 SP62-T2。contract test 与 `reducer.rs` 在 T2
    期间只读，分别等待 SP62-T3 接管；`model.rs` production definitions 冻结，其
    `#[cfg(test)]` module 在 T3 reducer 完成后串行交给 T3 追加 GH-57 exact bridge tests。

- [ ] `SP62-T2`（child alias: `GH62-T2`）发布 typed error/state read/restore surface 并实现 pure helpers | Owner: `chat-state-worker` | Done when: 独占 T1 handoff 的 `error.rs`/`state.rs`，发布 `ConversationError`、`ConversationState::new/snapshot/try_restore` 及只读 accessors，但不声明 apply 已可用；private-field fallible snapshot/retention/identity history types 闭合 messages/revisions/sequence、ordered ledger/eviction boundary、Block/Thinking/ToolCall/result-slot histories，restore 对缺失/重复/矛盾输入 fail closed；typed errors 覆盖 invalid values、duplicate/retired identity、correlation 与 counter exhaustion；`state.rs` 实现 transition、histories、checked revision、propagation、correlation、block mutation、completion 与 5×6 matrix helpers；module-qualified tests 直接验证十五项 helpers后 handoff | Verify: `verify_chat_lib_test components::chat::state::tests::thinking_replacement_requires_same_identity`；`verify_chat_lib_test components::chat::state::tests::thinking_id_message_lifetime_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::message_transition_matrix_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::nested_status_transition_matrices_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::terminal_updates_are_single_effect_and_race_safe`；`verify_chat_lib_test components::chat::state::tests::cross_level_terminality_never_freezes_active_nested_blocks`；`verify_chat_lib_test components::chat::state::tests::identity_and_correlation_helpers_cover_all_namespaces`；`verify_chat_lib_test components::chat::state::tests::append_block_cross_level_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::replace_block_kind_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::static_completion_readiness_matrix_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::tool_call_result_correlation_matrix_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::message_revision_checked_increment_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::block_id_state_lifetime_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::restore_history_validation_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::tool_result_slot_history_rules_are_exhaustive`；`verify_chat_missing_docs_gate`
  - Dependencies: SP62-T1。
  - Covers: B-002, B-003, B-005, B-006, B-007, B-008, B-009, B-014, B-023, B-025, B-026, B-028。
  - Test handoff: 全部 pure-helper tests 位于 T2 可写的 `state.rs`；本行通过后把 `state.rs`
    串行交给 SP62-T3 并冻结 `error.rs`。SP62-T3 此时才接管 `state.rs`、`reducer.rs` 与
    T1 已验证的 integration contract test。

- [ ] `SP62-T3`（child alias: `GH62-T3`）实现有界 conversation state、identity/correlation indexes、replay ledger、原子 reducer 与对应 tests | Owner: `chat-reducer-worker` | Done when: 接管 T2 已验证的 `state.rs` 与 T1 的 `reducer.rs`/integration test，`state.rs` 暴露只读 state/revision/ledger boundary 及 typed snapshot/restore，不暴露可变内部引用；`reducer.rs` 固定执行 replay/conflict→stale/gap/retention→sequence exhaustion→conversation revision exhaustion→conversation guard→target→message guard→affected MessageRevision advancement→完整 validation→single commit；每个实际改变的既有 message revision checked 加一且恰好一次，新 Push/Resend 为 1，outcome 按 mutation 前顺序列出 typed affected entries；block mutations 维护 state-wide BlockId history，Edit 原子维护 per-message ThinkingId tombstones；Delete ToolResult 原子退役 result slot，Delete call/result 同时退役两者，eviction/restore 均不释放；Resend 保持 terminal source、允许新 MessageId 复用 ThinkingId、禁止复用 conversation-wide ToolCallId；Cancel/Fail 跨 message 传播仍只推进一个 conversation revision，但逐条推进 affected message revisions；任一 stale/overflow/unknown/retention/correlation 错误 full-state atomic；exact replay 返回原 outcome；本任务只新增 reducer/integration tests及明确列出的 `model.rs` GH-57 bridge tests | Verify: `verify_chat_test push_is_unique_and_atomic`；`verify_chat_test lifecycle_identity_namespaces_are_scoped_and_correlated`；`verify_chat_test duplicate_lifecycle_identities_are_rejected_atomically`；`verify_chat_test correlated_lifecycle_updates_are_atomic`；`verify_chat_test cancel_cascades_across_correlated_messages_atomically`；`verify_chat_test fail_cascades_across_correlated_messages_atomically`；`verify_chat_test message_complete_rejects_inconsistent_tool_pairs`；`verify_chat_test streaming_deltas_are_ordered_lossless_and_typed`；`verify_chat_test append_block_supports_late_discovered_typed_blocks`；`verify_chat_test append_block_rejects_invalid_blocks_atomically`；`verify_chat_test replace_block_validates_before_commit`；`verify_chat_test replace_block_requires_same_variant_and_identity`；`verify_chat_test edit_and_insert_are_revisioned_and_identity_safe`；`verify_chat_test delete_preserves_global_correlation_atomically`；`verify_chat_test resend_preserves_source_and_creates_fresh_identity`；`verify_chat_test revision_guards_and_mutation_failures_are_atomic`；`verify_chat_test mutation_replay_retention_is_consistent`；`verify_chat_test static_message_completes_without_dummy_append`；`verify_chat_test empty_static_message_requires_content_before_complete`；`verify_chat_test pending_message_with_active_nested_block_cannot_complete`；`verify_chat_test sequence_is_conversation_wide_and_contiguous`；`verify_chat_test exact_replay_returns_original_outcome_without_mutation`；`verify_chat_test reused_event_id_with_different_content_conflicts`；`verify_chat_test stale_gap_and_retention_errors_do_not_advance_state`；`verify_chat_test every_failure_is_atomic_for_full_state`；`verify_chat_test bounded_ledger_exposes_honest_replay_boundary`；`verify_chat_test fresh_restart_state_has_no_replay_or_eviction_evidence`；`verify_chat_test identical_sequences_produce_identical_state_and_outcomes`；`verify_chat_test cancellation_preserves_partial_content_and_rejects_late_events`；`verify_chat_test sequence_exhaustion_is_checked_and_atomic_at_u64_max`；`verify_chat_test sequence_exhaustion_precedes_malformed_update_at_u64_max`；`verify_chat_test replay_conflict_stale_and_gap_precede_exhaustion`；`verify_chat_lib_test components::chat::state::tests::revision_exhaustion_is_checked_and_atomic_at_u64_max`；`verify_chat_test exact_replay_does_not_advance_exhausted_counters`
  - Dependencies: SP62-T1、SP62-T2。
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-020, B-023, B-024, B-025, B-026, B-027, B-028。
  - Public/reducer completion gate: T3 导出已实现的 apply API 后，首次执行完整 B-001 gate；
    同轮证明 state-wide BlockId tombstone 不受 ledger eviction 影响，并证明 Fail 的同一
    `FailureCause` 可从目标 message 和所有 affected nested status accessor 读回；snapshot roundtrip
    保留 ledger/identity histories；删除 ToolResult 原子退役 slot 且 eviction/restore 不释放 | Verify:
    `verify_chat_test public_model_is_typed_and_constructible`；
    `verify_chat_test failure_causes_are_typed_and_propagated`；
    `verify_chat_test block_ids_are_conversation_unique_and_retained`；
    `verify_chat_test edit_retires_thinking_ids_atomically`；
    `verify_chat_test restore_snapshot_roundtrip_preserves_histories`；
    `verify_chat_test deleted_tool_result_retires_result_slot_atomically`。
  - GH-57 bridge handoff: T3 在不改 `model.rs` production definitions 的前提下独占其
    `#[cfg(test)]` module，加入 umbrella 固定的十个 module-qualified exact tests | Verify:
    `verify_chat_lib_test components::chat::model::tests::gh62_provider_independent_model_contract`；
    `verify_chat_lib_test components::chat::model::tests::gh62_update_id_public_construction`；
    `verify_chat_lib_test components::chat::model::tests::gh62_empty_and_missing_contract`；
    `verify_chat_lib_test components::chat::model::tests::gh62_revisioned_atomic_mutations`；
    `verify_chat_lib_test components::chat::model::tests::gh62_message_transition_matrix`；
    `verify_chat_lib_test components::chat::model::tests::gh62_event_idempotency_contract`；
    `verify_chat_lib_test components::chat::model::tests::gh62_replay_retention_boundary`；
    `verify_chat_lib_test components::chat::model::tests::gh62_ordered_update_contract`；
    `verify_chat_lib_test components::chat::model::tests::gh62_terminal_revision_race_contract`；
    `verify_chat_lib_test components::chat::model::tests::gh62_cancellation_contract`。
  - Test handoff: SP62-T2 handoff 后才写 `state.rs`/`reducer.rs`/integration tests；验证全部
    新 tests 后冻结生产文件与 contract test，再串行交给 SP62-T4。

- [ ] `SP62-T4`（child alias: `GH62-T4`）接入推荐 public surface、补齐 scoped rustdoc 并追加兼容 test | Owner: `public-api-worker` | Done when: `components::chat` 是权威路径，`components` / `prelude` 只做无歧义 re-export；现有 `Message`、`MessageRole`、`ToolCall`、`ThinkingBlock` 源码和行为不变；新增 public data/guard/outcome 类型仅以 constructor/accessor 使用，扩展性 enum 为 `non_exhaustive`，避免 required fields 破坏公开 struct literals；不可降级的 scoped `forbid(missing_docs)` 下全部 public API 通过 `cargo check`；唯一普通 rust doctest 用 constructor 依次演示 Push -> AppendText -> Complete 并实际得到 1 passed/0 failed/0 ignored | Verify: `verify_chat_missing_docs_gate`；`verify_chat_test legacy_message_and_new_chat_surface_coexist`；`verify_chat_test constructor_based_public_api_remains_compatible`；`verify_chat_rustdoc_example`；`cargo test --doc --workspace --all-features --locked`
  - Dependencies: SP62-T1、SP62-T3。
  - Covers: B-019, B-025, B-028。
  - Test handoff: 在 SP62-T3 的已验证文件上追加 compatibility test；验证后串行交给 SP62-T5。

- [ ] `SP62-T5`（child alias: `GH62-T5`）完成双 adapter tests、全矩阵审计与 scoped coverage artifact | Owner: `chat-verification-worker` | Done when: 两种 mock provider input 产生相等核心 events/outcomes/state；adapter 对 invalid revision/BlockId/TypedValue 明确失败；每个 mapped exact test 唯一并实际通过；完整矩阵覆盖 Block/Thinking/result-slot tombstone/restore、closed metadata/payload/TypedValue、FailureCause、revisions/outcomes、mutations/replay/correlation/Cancel/Fail/overflow/error branches；dependency fixtures 证明 non-code/safe source 成功，renamed direct/grouped/nested、std process/terminal、env macro、crate runtime source 失败；唯一 rustdoc、docs/dependency gates 通过；全部 planned chat sources new-code line-rate ≥80% 且 artifact 绑定 current head | Verify: `verify_chat_test distinct_mock_adapters_produce_equal_core_events`；定义并运行 `tech.md` 的全部 exact/doc/dependency/coverage helpers，随后执行本文件“验证”全部命令
  - Dependencies: SP62-T1 至 SP62-T4。
  - Covers: B-017, B-018, B-019, B-022, B-023, B-024, B-025, B-026, B-027, B-028；并为 B-001 至 B-028 提供完整回归证据。

## 并行拆分

- SP62-T1 → T2 → T3 → T4 → T5 必须串行执行：T1→T2→T3 依次交接 `state.rs`，T1/T3/T4/T5
  依次扩展 integration contract test；不存在共享文件并行写或等待后续任务才完成验收的环。
- 每项任务必须在交接 writable 文件前运行并通过自己新增的 exact tests。后续任务不得删除或
  弱化前序断言；`state.rs` 与 contract test 任一时刻各自只有一个 writable owner。
- 独立 reviewer、依赖审计和 coverage artifact 审查可作为 read-only lanes 并行；生产代码与
  tests 的 writable lanes 不并行。

## 验证

先在当前 shell 定义 `tech.md` 的 `assert_exact_one_test_passed`、`verify_chat_test`、
`verify_chat_lib_test`、`verify_chat_rustdoc_example`、`verify_chat_missing_docs_gate`、
`audit_forbidden_package_aliases`、`verify_forbidden_dependency_alias_detection`、
`verify_no_forbidden_chat_dependencies`、`verify_chat_new_code_coverage`，并对
Product-to-Test Mapping 的全部 exact test 名逐个调用。任一 libtest `--exact` 过滤器匹配数
不是 1，或以 `--include-ignored` 执行后不是 1 passed/0 failed/0 ignored，均失败。

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features --locked`
- `cargo test --workspace --all-targets --all-features --locked`
- `verify_chat_rustdoc_example`；必须唯一解析 chat module 的普通 `rust` doctest，证明
      `components::chat` 过滤域只有该测试；只从该唯一 fence 抽取非注释代码，并确认
      Push/AppendText/Complete 顺序，再以 `--include-ignored` 实际执行并得到
      1 passed/0 failed/0 ignored。fence 外 token 或当前 merged rustdoc harness 对完整展示
      名称的 `--exact` 零匹配结果均不是证据。
- `cargo test --doc --workspace --all-features --locked`
- `verify_chat_missing_docs_gate`；必须证明五个 planned chat source files 存在、
      `chat/mod.rs` 恰有一个 `#![forbid(missing_docs)]`、全域没有
      `allow/expect(missing_docs)` 或 `doc(hidden)`，并实际运行 workspace/all-targets/
      all-features/locked `cargo check`。
- 刷新 `origin/main` 后运行 `verify_no_forbidden_chat_dependencies origin/main`；零 forbidden
      package identity/alias matches 必须 exit 0，manifest/lockfile diff、匹配项、metadata
      或扫描错误必须 exit nonzero；`verify_forbidden_dependency_alias_detection` 必须证明
      non-code/safe-source fixtures 成功，renamed direct/grouped/nested、std process/terminal、
      env macro 与 crate runtime source 被拒绝。
- `verify_chat_new_code_coverage`；保留
      `target/specrail/GH62/coverage-<full-head-sha>/cobertura.xml` 当前-head artifact。
- table-driven transition tests 的输入集合等于每个状态枚举的完整笛卡尔积；cross-level
      Push/AppendText/AppendMessageBlock/InsertMessageBlock/ReplaceBlock/Complete/Cancel/
      Fail/EditMessage/DeleteMessage/Resend、message-local Thinking、
      conversation-wide call namespace/correlation 与完整 5×6 ToolCall/ToolResult matrix、
      call/result 分居不同消息且从任一侧触发 Cancel/Fail 的单 revision 原子传播、
      empty/non-empty static completion、
      per-message nonzero revision/affected outcome、state-wide BlockId 与 per-message
      ThinkingId tombstone/restore、
      current/fresh-state retention boundary 与 `u64::MAX - 1`/`u64::MAX` precedence path
      没有未列出的 success/error 分支。
- `git diff --check`，planned paths 与 implementation diff 完全一致；出现 Cargo.toml、
      examples、view/shell/layout 或 provider 代码即阻断。
- 当前 head 的 CI、独立 review、所有 reviewThreads 和 SpecRail PR gate 均通过。

## Handoff Notes

- GH-62 没有 completion dependency；GH-57 仅为 umbrella tracking。下游 GH-63、GH-65、
  GH-66、GH-67 不得在 GH-62 完成前宣称最终验收。
- 本 packet 本身不授权 implementation；队列 coordinator 必须按当前 SpecRail `auth_mode`
  和 route gate 记录 readiness，再启动实现。
- 首版 sequence 起点由构造 conversation 时显式提供；不要静默假定 provider 从 0 或 1 开始。
- replay 必须先于 stale 判断；失败不能写 ledger；逐出后的旧事件必须返回 honest boundary
  error，不能当作成功或普通重复。
- `ReplayOutsideRetention` 只来自当前 state eviction 或携带可信 boundary 的显式恢复 state；
  fresh restart state 无恢复证据时不得承诺 replay/outside-retention 或跨进程幂等。
- Push 只接受非空 entries 的 Pending/Pending nested；adapter 在第一个 block 已知时 Push，
  后续 late-discovered block 通过 AppendMessageBlock 或 InsertMessageBlock 加入，已有 entry
  不重排。AppendText/ReplaceBlock 用 stable BlockId 定位；跨 message 重复 ID 拒绝，retired
  tombstone 不随 ledger eviction 丢失，显式 restore 保留，只有 fresh state 可重用相同数值。
- ThinkingId namespace 持续整个 MessageId lifetime；Edit 移除后进入 per-message retired set，
  不随 ledger eviction 释放，显式 restore 保留，same-message 重建原子失败；不同 message
  可复用 ThinkingId。ToolCallId/ToolResult call identity 在 conversation 内为一 call/一 result
  correlation；重复 call/result 或 orphan ToolResult 必须原子失败。
- ToolCall/ToolResult matrix 固定为 Pending→absent，Running→absent/Pending/Streaming，
  Succeeded→absent/Pending/Streaming/Complete/Cancelled/Failed(_)；
  Cancelled→absent/Cancelled，Failed(_)→absent/Failed(_)。Succeeded + Cancelled/Failed(_)
  表示工具
  执行成功但结果传输/消费取消或失败；新增 result 必须引用 current state 已有 call，同一
  Push 不得同时引入 Pending call/result。T2 在 `state.rs` 用 module-qualified lib tests
  验收全部 pure helpers，再把该文件交给 T3；helpers 只为 `pub(super)`，不得为了 integration
  tests 扩大 public API。T3 只负责 reducer integration/atomicity，不得替代 T2。
- Cancel/Fail 跨 message 传播顺序固定为 target nested -> indexed active counterpart ->
  affected-message/global matrix validation -> single commit；非目标消息 top-level/无关内容不变。
  Cancel 与 Fail 的 exact reducer test 各自必须从 call/result 所在消息两侧触发，断言同一
  revision、逐值相等且可经各层 accessor 读取的 `FailureCause`、无非法中间态，并覆盖失败时
  full state/ledger 相等。
- Complete 只接受 Streaming + nested 全终态，或 Pending + 至少一个 payload 非空的静态
  Text/Markdown/Code/Error block 且没有 lifecycle block；仅含空 Text/Markdown/Code
  payload 不能直 Complete，空 Error message 在结构验证时已失败。静态完成路径不得追加
  占位、空 delta 或复制已有内容。Cancel/Fail 必须原子终结 active nested，不要产生
  terminal message + active nested block 的冻结组合。
- sequence/conversation revision 必须先按固定顺序 checked-compute；随后验证 conversation/
  message guards，再为每个 affected existing message checked-next 一次 `MessageRevision`；
  Push/Resend 从 `INITIAL=1` 开始，outcome 返回 typed affected entries。
- 错误优先级固定为 replay/conflict -> stale/gap/retention -> sequence exhaustion ->
  conversation revision exhaustion -> conversation guard -> target -> message guard ->
  affected message revision exhaustion -> update validation；expected `u64::MAX` 的 malformed event 必须是
  `SequenceExhausted`。
- 不要为方便测试向 core 加 serde/provider dependencies，也不要把现有 `Message` 改造成新模型；
  依赖审计必须以 cargo metadata 推导 crate alias，剥离非代码 token 后解析 Rust use tree
  源路径（含 grouped/nested group，忽略 `as` 后绑定名）及 extern/path/macro，不能 raw grep。
- 审查时必须从 public surface 运行 integration tests，并核对 current head；其他 PR 或旧 SHA
  的绿色结果不能替代 GH-62 新鲜证据。
- Edit 保留 retained entry kind/lifecycle identity并退役 removed ThinkingId；Delete 不得留下
  跨 message orphan result；Resend 保持 terminal source 逐值/revision 不变并创建 fresh
  identity。三者 exact replay 返回原 outcome，evicted old event 先返回 retention error。
- private `state.rs` tests 必须使用
  `components::chat::state::tests::revision_exhaustion_is_checked_and_atomic_at_u64_max`
  与 `components::chat::state::tests::message_revision_checked_increment_is_exhaustive`
  精确执行；所有 exact test 与 chat module rustdoc 必须证明实际 passed，而不是仅在
  `--list` 中存在或被 ignored。
- rustdoc operation token 必须来自 helper 抽取并实际执行的唯一普通 rust fence；prose、其他
  fence 或注释中的 Push/AppendText/Complete 不构成证据。
