# Tech Spec：定向更新索引与单目标 reducer 路径

## Linked Issue

GH-143

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":143,"complete":true,"paths":["specs/GH143/product.md","specs/GH143/tech.md","specs/GH143/tasks.md","src/components/chat/state.rs","src/components/chat/state/message_index.rs","src/components/chat/state/tests.rs","src/components/chat/reducer.rs","src/components/chat/reducer/targeted.rs","src/components/chat/reducer/targeted/correlation_tests.rs","tests/chat_targeted_updates.rs"],"spec_refs":["specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md","specs/GH143/product.md","specs/GH143/tech.md","specs/GH143/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Conversation storage | `src/components/chat/state.rs:179` | `ConversationState` 只保存 ordered `Vec<ChatMessage>` 与 identity histories，没有 message position index | 私有索引必须与现有值语义、Clone/Eq 和 snapshot restore 一致 |
| Message lookup | `src/components/chat/state.rs:212` | `message()` 用 `iter().find()`，每次 lookup 为 `O(M)` | guard、mutation helper 与 caller 的重复 lookup 是热路径成本来源 |
| Event orchestration | `src/components/chat/reducer.rs:63` | `apply_event()` 校验 guard 后又扫描 transcript 构造 affected order、revision map、revision commit 与 outcome | AppendText/Complete 即使不做 global validation 仍有多次全量扫描 |
| Affected discovery | `src/components/chat/reducer.rs:197` | 普通更新只产生 target id，但随后仍通过完整 `messages` 过滤排序 | 单目标路径可以直接保留稳定顺序而无需集合与 transcript scan |
| Update dispatch | `src/components/chat/reducer.rs:220` | `AppendText` / `Complete` 再进入 helper，最终通过 `message_mut()` 重查目标 | resolved position 未跨阶段复用 |
| Correlated complete | `src/components/chat/reducer.rs:463` | `Complete` 对 streaming message 调用 `correlated_nested_terminal()` 并扫描全部消息 blocks | 仅 correlation-bearing target 需要保留跨消息判定；普通 target 不应扫描 |
| Delete maintenance | `src/components/chat/reducer.rs:603` | `DeleteMessage` 通过线性 position lookup 删除 Vec entry，rollback 由 `MutationBackup` 恢复值 | position index 必须随成功删除和失败 rollback 原子重建 |
| Existing cost proof | `src/components/chat/reducer.rs:774` | test-only counter 只统计 global validation 与 backup capture | 现有测试无法观察 message / block scans 或 target lookup 次数 |
| Public contract tests | `tests/chat_conversation_contracts.rs:488` | 已覆盖 typed model、streaming、correlation、rollback、snapshot 与 proof contracts | 新实现必须保持整套现有行为，不通过修改旧断言过关 |

`state.rs` 与 `reducer.rs` 写作时分别为 796 / 795 行，已接近 U-16 的 800 行硬
上限。新增索引和 fast path 必须自然拆到子模块，不得用压缩、长单行或
`#[rustfmt::skip]` 规避拆分。

## 设计方案

### 1. 私有 message position index

在 `src/components/chat/state/message_index.rs` 定义 crate-private
`MessageIndex`，内部为 `BTreeMap<MessageId, usize>`：

- `ConversationState::new()` 构造空索引。
- `ConversationState::try_restore()` 在 snapshot 通过现有完整校验后，从 ordered
  messages 一次性重建索引；duplicate / position contradiction 作为
  `InvalidSnapshot` fail closed。
- `ConversationState::message()` 通过 index 定位并验证目标 slot 的 id，不再扫描
  transcript；不存在的 id 返回现有 `None`，已记录位置与 Vec 不一致则 fail loudly，
  不做静默线性 fallback。
- reducer 使用一个返回 typed `UnknownMessage` 的 private resolved-position helper；
  一个 event 只调用一次，然后在后续阶段传递 `usize`。
- `Push` / `Resend` 成功后登记末尾位置；`DeleteMessage` 成功后删除目标并重排其后
  positions。`MutationBackup::restore()` 在恢复 messages 后重建索引，使后置
  `validate_conversation()` 失败时值和索引一起 rollback。

索引是派生状态，不加入公开 `ConversationStateSnapshot` /
`ConversationIdentityHistory`。合法 snapshot 的可观察序列化 parts 不变。
为遵守 U-16，`state.rs` 现有的 test-forwarder 机械迁到
`src/components/chat/state/tests.rs`；只移动 wrapper，不改变任何 test case 或断言。

### 2. 单目标 apply path

在 `src/components/chat/reducer/targeted.rs` 放置：

- `TargetedUpdate` 分类，仅包含 `AppendText` 与 `Complete`。
- resolved target 的 guard / revision 校验结果。
- `apply_at(index, update)`，直接访问一个 `ChatMessage`，在所有可失败校验完成后才
  修改 block content 或 status。
- direct revision/outcome builder：预先 `checked_next`，成功后只写目标 revision，
  构造单个 `AffectedMessage { previous, applied, Present }`。
- test-only `ReducerCost` 与线程局部 recorder，分别统计 message visits、target
  lookups、block visits、global validation、backup capture。

`apply_event()` 保留 sequence、conversation revision、event-id replay 与 ledger proof
的共同入口/出口；仅在 target guard 已解析且 update 被分类为单目标时走 fast path。
generic path 的行为和 affected ordering 保持。

### 3. Complete 的 correlation 边界

`Complete` 先只检查目标 blocks：

- Pending 静态内容和不含 tool correlation 的 Streaming 内容走单目标路径。
- target 包含 `ToolCall` / `ToolResult` 时，继续执行现有
  `correlated_nested_terminal()`；该显式 fallback 记录 message / block visits，并只
  用于跨消息 readiness 语义。
- `Cancel` / `Fail` 保持现有 full correlation discovery 和多目标 affected ordering。

本 tranche 不新增 call/result location index，以避免把 focused streaming fix 扩大为
完整 correlation registry 重写。

### 4. 测试与计数完整性

`targeted.rs` 的 private tests 可直接构造合法内部 state，避免为 10,001 条 fixture
逐条运行昂贵的 global mutation path。计数器在每个 event 前 reset，并对每一种工作
单独 increment；一次 transcript loop 中每访问一条 message 都必须计数，不能只在
函数入口记一次。

`tests/chat_targeted_updates.rs` 只通过 public API 验证 Text、Markdown、Code、
Thinking、replay、rejected atomicity、Push/Delete/Resend 与 snapshot roundtrip。
既有 `tests/chat_conversation_contracts.rs` 不修改，作为兼容性回归。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | message index + AppendText fast path | `cargo test --lib components::chat::reducer::targeted::tests::append_cost_is_transcript_independent -- --exact` |
| B-002 | non-correlation Complete fast path | `cargo test --lib components::chat::reducer::targeted::tests::complete_cost_is_transcript_independent -- --exact` |
| B-003 | target block mutation | `cargo test --test chat_targeted_updates append_supported_blocks_preserves_typed_payloads -- --exact` |
| B-004 | private cost fixtures | `cargo test --lib components::chat::reducer::targeted::tests::front_and_end_targets_have_equal_cost -- --exact` |
| B-005 | direct revision/outcome builder | `cargo test --test chat_targeted_updates targeted_outcome_advances_only_target -- --exact` |
| B-006 | preflight-before-mutation path | `cargo test --test chat_targeted_updates append_rejection_matrix_is_fully_atomic -- --exact`; `cargo test --test chat_targeted_updates targeted_preflight_errors_preserve_the_complete_state -- --exact`; `cargo test --lib components::chat::reducer::targeted::correlation_tests::target_revision_exhaustion_is_atomic_and_locally_counted -- --exact` |
| B-007 | MessageIndex mutation + backup restore | `cargo test --test chat_targeted_updates push_delete_resend_keep_lookup_and_order_consistent -- --exact` |
| B-008 | `try_restore()` index rebuild | `cargo test --test chat_targeted_updates snapshot_restore_rebuilds_target_lookup -- --exact` |
| B-009 | correlation fallback | `cargo test --lib components::chat::reducer::targeted::correlation_tests::correlated_complete_counts_each_fallback_visit -- --exact`; `cargo test --lib components::chat::reducer::targeted::correlation_tests::cancel_and_fail_count_each_correlation_visit -- --exact`; `cargo test --test chat_conversation_contracts cancel_cascades_across_correlated_messages_atomically -- --exact` |
| B-010 | common replay/proof entry | `cargo test --test chat_conversation_contracts replay_is_idempotent_and_bounded -- --exact`; `cargo test --test chat_conversation_contracts event_id_conflict_is_typed -- --exact` |
| B-011 | test-only `ReducerCost` | `cargo test --lib components::chat::reducer::targeted::tests::cost_dimensions_are_independent -- --exact`; `cargo test --lib components::chat::reducer::targeted::correlation_tests::correlated_complete_counts_each_fallback_visit -- --exact`; `cargo test --lib components::chat::reducer::targeted::correlation_tests::cancel_and_fail_count_each_correlation_visit -- --exact` |
| B-012 | existing contract suites | `cargo test --workspace --all-targets --all-features --locked` |
| B-013 | targeted classifier + counters | `cargo test --lib components::chat::reducer::targeted::tests::local_paths_skip_global_work -- --exact` |

## 数据流

`ConversationEvent` → common replay/sequence/revision preflight → one indexed target resolution →
targeted or generic reducer path → prevalidated mutation → message/conversation revision commit →
`ApplyOutcome` → retained proof/ledger commit。

`ConversationStateSnapshot` → existing proof/identity validation → ordered messages →
private `MessageIndex::rebuild` → restored state。无网络、文件持久化或公开 schema 变化。

## 备选方案

- 只把多次扫描合并成一次：仍为 `O(M × C)`，且 target-at-end 成本随 transcript
  增长，不满足 B-001/B-004。
- 把 `Vec` 改成 map：会改变稳定消息顺序、snapshot shape 与遍历 API，范围和兼容风险
  远大于私有派生索引。
- 为所有 block/correlation 建完整 location registry：能进一步优化 correlated
  Complete，但会扩大 Push/Edit/Replace/Delete/Cancel/Fail 的事务面；本 issue 只保留
  显式 correlation fallback。
- 用 wall-clock benchmark：10k fixture 在 CI 上易受调度噪声影响，不能证明访问次数
  上界；确定性 operation counters 更直接。

## 风险

- Security: 无输入执行、权限、网络或 unsafe 变化；typed error 与 fail-closed
  validation 保留。
- Compatibility: `ConversationState` private layout 改变，但公开 API、snapshot、
  identity history 与 proof parts 不变。Clone/Eq 包含派生 index，测试确保合法状态一致。
- Performance: targeted path 从多次 `O(M)` 降为一次 `O(log M)` index lookup 加
  `O(target blocks)`；Delete 仍需重排尾部 positions，为其固有 Vec shift 成本，不在
  streaming 热路径。
- Maintenance: 双重数据结构可能漂移；所有 messages 结构变化集中维护，并由
  restore/rollback/全局 validation 与 index consistency tests fail closed。
- File size: 两个现有文件已在硬上限附近；索引、targeted path 与 state test-forwarder
  必须落入自然子模块，最终逐文件 `wc -l` 均不得超过 800。

## 测试计划

- [ ] Unit tests: transcript-size counters、front/end、correlation fallback、全局工作跳过。
- [ ] Integration tests: 四类 block、accepted/rejected atomicity、identity mutation、
  snapshot restore、replay/outcome。
- [ ] Compatibility: 现有 chat contract、snapshot/proof、doc tests 原样通过。
- [ ] Coverage: changed production lines ≥80%，`message_index.rs` 与 targeted fast path
  executable line/branch 100%。
- [ ] Full verification:
  `cargo fmt --all -- --check`；
  `cargo check --workspace --all-targets --all-features --locked`；
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
  `cargo test --workspace --all-targets --all-features --locked`；
  `cargo test --doc --workspace --all-features --locked`。

## 回滚方案

回滚 `ConversationState` 的 private `MessageIndex` 字段、`message()` indexed lookup、
Push/Delete/Resend/restore 的维护点及 `apply_event()` targeted 分支，即恢复 PR #117
后的 reducer。因为 snapshot 与公开 API 未变，无数据迁移或双写清理；回滚必须同时
移除两个新 private modules 和对应测试，不能留下未接线派生状态。
