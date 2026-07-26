# Task Plan：variable-height MessageList 与滚动锚定

## Linked Issue

GH-65: https://github.com/majiayu000/rnk/issues/65

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## Implementation Gate

本 packet 是 spec-only，不授权在当前 branch 写生产代码。`SP65-T1` 开始前 coordinator 必须
从 fresh `origin/main` 创建 implementation branch，并保存：

1. GH-58、GH-60、GH-62 implementation PR 的 GitHub `mergedAt`、merge commit SHA；
2. `git merge-base --is-ancestor <dependency-sha> HEAD` 三次成功输出；
3. GH-65 issue/PR/branch/spec duplicate search 与 SpecRail route evidence；
4. 对 merged TextFlow、layout error、MessageId/MessageRevision、chat module 和 manifest paths
   的 source-drift audit。

任一 required dependency 未 merge、ancestry 失败、API/path 与 packet 冲突，必须停止并先更新/
重新 review packet。GH-63 不阻塞 index；若已 merge，只通过 closure 做 integration。不得从
spec branch、open dependency branch 或推测 API 开始实现。

## 实现任务

- [ ] `SP65-T1` 建立 validated public value/error types、exact measurement cache 与 Fenwick row index。 Covers: B-001, B-002, B-003, B-004, B-013, B-018, B-019 | Owner: height-index | Done when: row/key/error/cache/index 合同完整，exact equality、checked arithmetic、deterministic eviction 与 logarithmic operation counter 测试通过 | Verify: T1 exact tests
  `File ownership:` 仅
  `src/components/chat/message_list/types.rs`、
  `src/components/chat/message_list/error.rs`、
  `src/components/chat/message_list/height_index.rs`。
  `Dependencies:` Implementation Gate。
  `Verify:`
  `cargo test --workspace --lib --locked components::chat::message_list::tests::measurement_key_uses_all_identity_fields_and_exact_equality -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::empty_zero_viewport_and_zero_width_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::lookup_and_point_update_have_logarithmic_operation_bound -- --exact`。
  `Handoff:` 保存 exact head、公开 type/error inventory、operation-count bound 和输出；停止写
  三个 paths 后交给 T2，禁止无声明 alias、`Any` 或 default row fallback。

- [ ] `SP65-T2` 实现 caller-owned state、原子 measurement mutations、partial slices、anchor 与 bottom-follow state machine。 Covers: B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-018, B-019 | Owner: message-list-state | Done when: 所有 mutation 先 candidate 后一次 commit，prepend/delete/resize/stream/expand/collapse/failure 的 anchor/follow/cache/revision 合同由 exact tests 锁定 | Verify: T2 exact unit tests
  `File ownership:` 仅
  `src/components/chat/message_list/state.rs`、
  `src/components/chat/message_list/tests.rs`。
  `Dependencies:` SP65-T1。
  `Verify:`
  `cargo test --workspace --lib --locked components::chat::message_list::tests::partial_first_and_last_message_ranges_are_row_exact -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::prepend_preserves_top_message_and_intra_row -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::height_changes_preserve_or_report_anchor_clamp -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::deleted_anchor_selects_next_then_previous_survivor -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::follow_pause_and_explicit_resume_state_machine -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::append_and_stream_growth_follow_or_mark_new_content -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::resize_variant_expansion_and_structure_cache_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::measurement_failure_and_cancellation_are_atomic -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::stale_measurement_is_rejected_without_mutation -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::identical_inputs_produce_identical_state -- --exact`；
  `cargo test --workspace --lib --locked components::chat::message_list::tests::structural_and_resize_costs_are_explicit_and_reuse_cache -- --exact`。
  `Handoff:` 保存每个 state transition 的 before/after evidence、cache request counts、state
  revision 与 failure equality snapshot；停止写两个 paths 后交给 T3。

- [ ] `SP65-T3` 建立 MessageList facade、module/prelude exports、GH-58 measurement 与 GH-63-compatible typed render closure。 Covers: B-001, B-013, B-015, B-016, B-017, B-022 | Owner: message-list-facade | Done when: crate 外 typed public surface可用，TextFlow 是唯一测量来源，closure 收到 exact slices，render failure 无 partial frame，fixed-height API 完全兼容 | Verify: T3 integration/public/compat exact tests
  `File ownership:` 仅
  `src/components/chat/message_list.rs`、
  `src/components/chat/mod.rs`、
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
  `cargo test --test message_list_render --locked message_height_comes_from_single_text_flow_result -- --exact`；
  `cargo test --test message_list_render --locked message_list_render_closure_receives_exact_visible_slices -- --exact`；
  `cargo test --test message_list_render --locked render_failure_has_source_and_never_returns_partial_frame -- --exact`；
  `cargo test --test virtual_scroll_compat --locked fixed_height_virtual_scroll_api_is_unchanged -- --exact`。
  `Handoff:` 保存 exports/API inventory、TextFlow build count、closure call order/error source、
  fixed-height fixture 与 exact outputs；停止写全部 paths 后交给 T4。

- [ ] `SP65-T4` 建立固定 seed naive property oracle、10k benchmark 与 Cargo bench registration。 Covers: B-002, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-014, B-018, B-019, B-020, B-021 | Owner: property-performance | Done when: 至少256个固定seed随机序列逐步比对独立oracle，10k mixed-height lookup/slice/stream/prepend benchmark实际运行，复杂度硬门禁仍通过 | Verify: property exact test、operation-count exact test与10k bench
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
  `cargo bench --bench message_list -- message_list_10k`。
  `Handoff:` 保存 seed/cases、缩减后失败格式、bench input 分布、current head 与完整输出；停止
  写三个 paths 后交给 T5。

- [ ] `SP65-T5` 在 implementation PR 当前 exact head 上执行只读 closure audit。 Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024 | Owner: verification-review | Done when: dependencies、planned paths、全部 exact/full tests、property/bench、coverage、CI、review threads、independent review与人工PR gate均绑定current exact head | Verify: tech mapping全部命令与full Rust/docs gates
  `File ownership:` 无 writable path；只读审计，不得修改/resolve thread、approve 或 merge。
  `Dependencies:` SP65-T4。
  `Verify:` 先逐个运行 Tech Spec Product-to-Test Mapping 对应 exact tests，再运行
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

Writer 不并行。T1 创建类型/index，T2 只消费且独占 state/tests，T3 再建立 facade/exports，
T4 最后只写 property/bench/Cargo。文件所有权不重叠，DAG 为：

```text
Implementation Gate -> SP65-T1 -> SP65-T2 -> SP65-T3 -> SP65-T4 -> SP65-T5
```

每个 owner 在 handoff 前停止写其 paths、提交并记录 exact head。T5 仅在所有 writer 停止后
执行只读审计。若实现时必须修改 manifest 之外的生产路径，先停止、更新/review packet，
不得临时扩大 scope。

## 验证

- 对每个 exact test 先用 `--exact --include-ignored` 列表确认唯一匹配，再普通执行并得到
  `1 passed; 0 failed; 0 ignored`；property test 必须非 `#[ignore]`。
- 逐项运行 T1–T4 所列 exact tests；failure test 比较整个 pre/post state/cache/revision。
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
- Cache key 必须完整包含 stable ID、width、content revision、variant、expansion，并做 exact
  equality；结构变化只重建 index，不让 unchanged key 全量重测。
- Mutation 先 stage 全部测量和 index，成功后一次 commit；missing/failure/cancellation/stale/
  overflow 都 typed 且逐字段零 mutation。
- Paused 只可由显式到达底部/jump 恢复 Following；resize/delete/collapse 不能暗中恢复。
- 删除 anchor 的 next-then-previous 和 height shrink clamp 规则不得由实现自由选择。
- GH-58 是唯一 TextFlow measurement authority；GH-63 只经 render closure；GH-60 保持
  candidate frame error 原子性；GH-62 提供真实 ID/revision。
- 当前 spec base 缺少 required implementation。Implementation Gate 与 source-drift audit
  是硬阻塞，不得把本文伪签名当作已声明 API。
- 目标仓库不 vendor `workflow.yaml` 或 SpecRail checker。Spec 验证必须记录所用 SpecRail
  source checkout 的 exact commit，并以该 pack 为 `--repo`、本 packet 为 `--spec-dir` 运行
  真实 checker；该结果是 external-pack evidence，不能宣称目标仓库自带 workflow pack。
- 回滚普通 revert；不修改 fixed-height virtual scroll，不 force push，不弱化测试。
