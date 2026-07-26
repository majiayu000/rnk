# Task Plan：variable-height MessageList 与滚动锚定

## Linked Issue

GH-65: https://github.com/majiayu000/rnk/issues/65

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## Implementation Gate

本 packet 是 spec-only，不授权在当前 branch 写生产代码。`SP65-T1` 开始前 coordinator 必须
从 fresh `origin/main` 创建 implementation branch，并保存：

1. GH-58、GH-60、GH-62 fresh `state=CLOSED` 与 `closed_at`；
2. 每个 issue 的 final closure evidence 所枚举的完整 implementation PR/commit set、approved
   task completion 与 final PR-gate exact head；不能由 coordinator 自选一个 partial PR；
3. set 中每个 PR 的 fresh `state=MERGED`、`exact_head_sha`、`merge_commit_sha`、`merged_at`；
4. `git merge-base --is-ancestor <each-merge-sha> <implementation-base-sha>` 全部成功输出；
5. GH-65 issue/PR/branch/spec duplicate search 与 SpecRail route evidence；
6. 对 merged TextFlow、layout error、MessageId/MessageRevision、chat module 和 manifest paths
   的 source-drift audit。

每项依赖形成 Tech Spec 定义的 `DependencyCompletionRecord`。任一 issue OPEN、final evidence
缺失/不完整、任一 required commit 未 merge/不在 ancestry、API/path 与 packet 冲突，必须停止
并先更新/review packet。GH-58 OPEN 时，merged partial/root-cause PR #84 单独不能满足 gate。
GH-63 不阻塞 index；若已 merge，只通过 closure 做 integration。不得从 spec branch、open
dependency branch 或推测 API 开始实现。

## 实现任务

- [ ] `SP65-T1` 建立可独立编译的 parent/test/state skeleton、validated public value/error/update types、完整 composite measurement cache 与 Fenwick row index。 Covers: B-001, B-002, B-003, B-004, B-011, B-012, B-013, B-014, B-018, B-019 | Owner: height-index | Done when: parent module 可发现全部 T1 tests，state skeleton 可编译，row/key/config/outcome/update/error/cache/index 合同完整，deep equality、checked arithmetic、deterministic eviction 与 logarithmic operation counter 测试通过 | Verify: T1 exact tests + checkpoint check
  `File ownership:` 创建并独占
  `src/components/chat/mod.rs`（只增加 message_list declaration/re-export）、
  `src/components/chat/message_list.rs`（parent/module/re-export skeleton）、
  `src/components/chat/message_list/types.rs`、
  `src/components/chat/message_list/error.rs`、
  `src/components/chat/message_list/height_index.rs`、
  `src/components/chat/message_list/state.rs`（private compile skeleton）、
  `src/components/chat/message_list/tests.rs`（T1 exact tests）。
  `Dependencies:` Implementation Gate。
  `Verify:`
  `cargo test --workspace --lib --locked components::chat::message_list::tests::measurement_config_covers_textflow_and_shell_inputs -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::measurement_key_uses_all_identity_fields_and_exact_equality -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::measured_missing_failed_and_cancelled_outcomes_are_closed -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::message_rows_reject_zero -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::lookup_and_point_update_have_logarithmic_operation_bound -- --exact`；
  `cargo check --workspace --all-targets --all-features --locked`。
  `Handoff:` 保存 exact head、公开 type/error/update inventory、operation-count bound 和输出；
  把 `state.rs`/`tests.rs` 串行交给 T2；`chat/mod.rs` 与 `message_list.rs` 冻结至 T3 接管，
  types/error/index 永久冻结。禁止 alias、`Any`、default row 或未声明 delayed API。

- [ ] `SP65-T2` 实现 caller-owned state、同步 closed measurement mutations、partial slices、typed anchor navigation、stored anchor 与 bottom-follow state machine。 Covers: B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-018, B-019 | Owner: message-list-state | Done when: 每个 mutation 先 guard/overflow preflight，再 candidate measurement/index，最后一次 commit；prepend short-content、zero viewport、delete/resize/stream/expand/collapse/failure 的 flags/anchor/follow/cache/revision 合同由 exact tests 锁定；GH-57 aggregate symbol 可精确执行 | Verify: T2 exact unit tests + checkpoint check
  `File ownership:` 从 T1 串行接管且仅修改
  `src/components/chat/message_list/state.rs`、
  `src/components/chat/message_list/tests.rs`。
  `Dependencies:` SP65-T1。
  `Verify:`
  `cargo test --workspace --lib --locked components::chat::message_list::tests::empty_zero_viewport_and_zero_width_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::partial_first_and_last_message_ranges_are_row_exact -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::prepend_preserves_top_or_reports_short_content_viewport_clamp -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::typed_anchor_navigation_rejects_unknown_and_invalid_rows -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::height_changes_preserve_or_report_anchor_clamp -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::zero_viewport_retains_and_restores_stored_anchor -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::deleted_anchor_selects_next_then_previous_survivor -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::follow_pause_and_explicit_resume_state_machine -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::append_and_stream_growth_follow_or_mark_new_content -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::each_textflow_and_shell_input_invalidates_only_affected_entry -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::resize_variant_expansion_and_structure_cache_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::measurement_failure_and_cancellation_are_atomic -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::stale_state_revision_and_noop_revision_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::state_revision_overflow_precedes_measurement_and_is_atomic_at_u64_max -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::identical_inputs_produce_identical_state -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::structural_and_resize_costs_are_explicit_and_reuse_cache -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::gh65_variable_height_anchor_contract -- --exact`；
  `cargo check --workspace --all-targets --all-features --locked`。
  `Handoff:` 保存每个 state transition 的 before/after evidence、cache request counts、state
  revision/flags 与 failure equality snapshot；冻结 `state.rs`/`tests.rs` 后交给 T3。

- [ ] `SP65-T3` 完成 MessageList facade、module/prelude exports、GH-58 renderer-equivalent composite adapter 与 GH-63-compatible exact render closure。 Covers: B-001, B-002, B-013, B-015, B-016, B-017, B-022 | Owner: message-list-facade | Done when: crate 外 typed public surface可用，多 textual children + 全 shell rows 的 composite height 等于最终 renderer rows，closure 收到 exact entry/key/slice，revision drift 在 callback 前失败，render failure 无 partial frame，fixed-height API 完全兼容 | Verify: T3 integration/public/compat exact tests + checkpoint check
  `File ownership:` 从 T1 接管
  `src/components/chat/mod.rs`、`src/components/chat/message_list.rs`，并独占
  `src/components/mod.rs`、
  `src/prelude.rs`、
  `tests/message_list_public_api.rs`、
  `tests/message_list_render.rs`、
  `tests/virtual_scroll_compat.rs`。
  `Dependencies:` SP65-T2。若 GH-62 merged 后 `chat/mod.rs` 已存在，只增加 declaration/export，
  不重写其 types；若 GH-63 已 merge，只在测试 closure 内调用其 public view。
  `Verify:`
  `cargo test --test message_list_public_api --locked message_list_public_surface_is_typed -- --exact`；
  `cargo test --test message_list_public_api --locked closed_error_categories_are_exhaustive_and_keep_sources -- --exact`；
  `cargo test --test message_list_public_api --locked dependency_completion_records_require_closed_issues_and_complete_commit_sets -- --exact`；
  `cargo test --test message_list_render --locked composite_height_matches_renderer_equivalent_rows -- --exact`；
  `cargo test --test message_list_render --locked render_closure_receives_exact_entry_key_and_slice -- --exact`；
  `cargo test --test message_list_render --locked render_revision_drift_is_rejected_before_callback -- --exact`；
  `cargo test --test message_list_render --locked render_failure_has_source_and_never_returns_partial_frame -- --exact`；
  `cargo test --test virtual_scroll_compat --locked fixed_height_virtual_scroll_api_is_unchanged -- --exact`；
  `cargo check --workspace --all-targets --all-features --locked`。
  `Handoff:` 保存 exports/API inventory、每 child TextFlow build count、structural row sum、closure
  exact revision/call order/error source、fixed-height fixture 与 outputs；停止写全部 paths 后交给 T4。

- [ ] `SP65-T4` 建立固定 seed naive property oracle、10k benchmark 与 Cargo bench registration。 Covers: B-002, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-014, B-018, B-019, B-020, B-021 | Owner: property-performance | Done when: 至少256个固定seed随机序列逐步比对独立oracle，含 short-content/zero-viewport/anchor navigation/revision overflow；10k mixed-height lookup/slice/stream/prepend benchmark实际运行，复杂度硬门禁仍通过 | Verify: property exact test、operation-count exact test、10k bench + checkpoint check
  `File ownership:` 仅
  `tests/message_list_properties.rs`、
  `benches/message_list.rs`、
  `Cargo.toml`。
  `Dependencies:` SP65-T3。
  Oracle 只能用朴素 `Vec` row expansion/prefix scan，不调用 production height-index helper；
  operation 包含 append/prepend/insert/update/delete/resize/scroll，失败输出固定 32-byte
  ChaCha seed 和最小操作序列。Benchmark 使用 10,000 条 mixed heights，分别测
  lower-bound lookup、visible slices、高频 single-message streaming point update 与 prepend；
  不以 wall-clock threshold 作为 pass/fail。
  `Verify:`
  `cargo test --test message_list_properties --locked variable_height_index_matches_naive_oracle -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::lookup_and_point_update_have_logarithmic_operation_bound -- --exact`；
  `cargo bench --bench message_list -- message_list_10k`；
  `cargo check --workspace --all-targets --all-features --locked`。
  `Handoff:` 保存 seed/cases、缩减后失败格式、bench input 分布、current head 与完整输出；停止
  写三个 paths 后交给 T5。

- [ ] `SP65-T5` 在 implementation PR 当前 exact head 上执行只读 closure audit。 Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024 | Owner: verification-review | Done when: dependencies、planned paths、全部 exact/full tests、property/bench、coverage、CI、review threads、independent review与人工PR gate均绑定current exact head | Verify: tech mapping全部命令与full Rust/docs gates
  `File ownership:` 无 writable path；只读审计，不得修改/resolve thread、approve 或 merge。
  `Dependencies:` SP65-T4。
  `Verify:` 先 fresh 重建三份 `DependencyCompletionRecord`，运行
  `dependency_completion_records_require_closed_issues_and_complete_commit_sets`，再逐个运行 Tech
  Spec Product-to-Test Mapping 对应 exact tests（包括
  `components::chat::message_list::tests::gh65_variable_height_anchor_contract`），然后运行
  `cargo fmt --all -- --check`；
  `cargo check --workspace --all-targets --all-features --locked`；
  `cargo test --workspace --all-targets --all-features --locked`；
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`；
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`；
  `cargo bench --bench message_list -- message_list_10k`。
  核对 changed-line coverage 至少 80%、anchor/cache/error critical paths 100%，并在 current
  GitHub head 收集 CI、review decision、所有 unresolved review threads、merge state 与
  SpecRail PR gate。`Handoff:` 报告 exact head/base/merge-base、dependency SHAs、fresh命令
  输出与未满足项；保留人工 approve/merge gate。

## 并行拆分

Writer 不并行。T1 创建 parent/state/test skeleton、类型/index；T2 串行接管
`state.rs`/`tests.rs`；T3 再从 T1 接管冻结的 parent facade 并独占 exports/integration；
T4 最后只写 property/bench/Cargo。每次 ownership transfer 都在上游 checkpoint
`cargo check` 与 commit 后发生，文件所有权不重叠，DAG 为：

```text
Implementation Gate -> SP65-T1 -> SP65-T2 -> SP65-T3 -> SP65-T4 -> SP65-T5
```

每个 owner 在 handoff 前停止写其 paths、提交并记录 exact head。T5 仅在所有 writer 停止后
执行只读审计。若实现时必须修改 manifest 之外的生产路径，先停止、更新/review packet，
不得临时扩大 scope。

## 验证

- 对每个 exact test 先用 `--exact --include-ignored` 列表确认唯一匹配，再普通执行并得到
  `1 passed; 0 failed; 0 ignored`；property test 必须非 `#[ignore]`。
- 逐项运行 T1–T4 所列 exact tests；每个任务 checkpoint 运行 fresh
  `cargo check --workspace --all-targets --all-features --locked`；failure test 比较整个
  pre/post state/cache/stored-anchor/revision。
- Property test 使用固定 seed、至少 256 cases、独立 naive oracle，并保存可重放输出。
- 10k benchmark 在 implementation current head 实际运行；operation counter 仍是复杂度
  correctness 硬门禁。
- 运行 fmt/check/test/clippy/docs full gates；所有输出来自本次 current exact head。
- `git diff --name-only <implementation-merge-base>...HEAD` 只含 reviewed planned paths。
- 核对新代码 line coverage ≥80%、anchor/cache/error critical paths 100%。
- 收集 current-head CI、review threads、review decision、merge state 与 SpecRail PR gate；
  agent 不 approve、不 merge、不 force push。

## Handoff Notes

- Offset/height/anchor/visible ranges 的唯一单位是 terminal row；message count 语义只属于旧
  `virtual_scroll_view`。
- Cache key 必须完整包含 stable ID/revision/variant/expansion、每个 GH-58
  `TextFlowCacheIdentity` 与全部 shell structural segments，并做 deep equality；每项配置只使
  affected entry 失效，结构变化只重建 index，不让 unchanged key 全量重测。
- Mutation 先 stage 全部测量和 index，成功后一次 commit；missing/failure/cancellation/stale/
  overflow 都 typed 且逐字段零 mutation。
- Paused 只可由显式到达底部/jump 恢复 Following；resize/delete/collapse 不能暗中恢复。
- 删除 anchor 的 next-then-previous、typed anchor navigation、zero-viewport retention、
  short-content viewport clamp 与 height-shrink anchor clamp 规则不得由实现自由选择。
- 首版 measurement callback 同步返回 closed measured/missing/failed/cancelled outcome，不发布
  delayed apply API；state revision overflow 在 callback 前失败。
- GH-58 为每个 textual child 提供 TextFlow；完整 message height 还必须计入全部 shell
  structural rows。GH-63 只经 exact entry/key/slice render closure；GH-60 保持
  candidate frame error 原子性；GH-62 提供真实 ID/revision。
- 当前 spec base 缺少 required implementation。Implementation Gate 与 source-drift audit
  是硬阻塞，不得把本文伪签名当作已声明 API。
- 目标仓库不 vendor `workflow.yaml` 或 SpecRail checker。Spec 验证必须记录所用 SpecRail
  source checkout 的 exact commit，并以该 pack 为 `--repo`、本 packet 为 `--spec-dir` 运行
  真实 checker；该结果是 external-pack evidence，不能宣称目标仓库自带 workflow pack。
- 回滚普通 revert；不修改 fixed-height virtual scroll，不 force push，不弱化测试。
