# Task Plan：grapheme-safe 多行 ChatComposer

## Linked Issue

GH-64: https://github.com/majiayu000/rnk/issues/64

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## Implementation Gate

本 packet 的 spec-only stacked ancestry 不授权实现。`SP64-T1` 开始前，coordinator 必须从
fresh `origin/main` 建 implementation branch，保存 fresh issue/PR/branch/spec duplicate
search 与 SpecRail route evidence，并从 GitHub 记录 GH-58、GH-60 implementation PR 的真实
merged commits：

随后重新读取 merged TextFlow/source map、checked layout/render error 与本 manifest 的真实
paths。任一 dependency 未 merge、ancestry 失败或 API/path 漂移都停止 implementation；
先更新并重新 review packet，不能从 spec branch、open implementation branch或推测 API 开工。

## 实现任务

- [ ] `SP64-T1` 建立只依赖当前 public API 的 Unicode 根因回归夹具。 Covers: B-002, B-004, B-019, B-023 | Owner: root-cause-fixture | Done when: 当前 API fixture 稳定复现根因且最终同一测试转绿 | Verify: `cargo test --test chat_composer_root_cause --locked scalar_cursor_and_single_byte_handler_break_unicode_input -- --exact`
  该 fixture 证明现有 scalar cursor
  与单字节 handler 会丢失或拆分 multi-scalar/CJK/combining/ZWJ 输入，并锁定最终修复必须
  通过的 exact test。`Owner: root-cause-fixture`。`Done when:` 新 fixture 在未修复生产代码
  上稳定复现根因，不引入 future API、mock sidecar 或弱化断言；实现完成后同一测试转绿。
  `Verify:` `cargo test --test chat_composer_root_cause --locked scalar_cursor_and_single_byte_handler_break_unicode_input -- --exact`。`Dependencies:` Implementation
  Gate。`File ownership:` 仅 `tests/chat_composer_root_cause.rs`。`Covers: B-002, B-004,
  B-019, B-023`。`Handoff:` 保存 red/green exact command、head SHA 与失败原因；停止写文件后
  才可交给 T2。

- [ ] `SP64-T2` 拆分 TextArea state、实现 checked grapheme edit core 与固定 seed property fixture。 Covers: B-002, B-003, B-012, B-018, B-019, B-023 | Owner: textarea-grapheme | Done when: state 小于 800 行、Unicode edit/selection 原子、property oracle 可复现且 public/ASCII/scalar-limit 兼容 | Verify: T2 下列五个 exact tests
  先拆分超限的 TextArea state，再把 cursor、edit、selection 与 batch
  mutation 改为 checked grapheme boundary；保持 public struct/enum/handler 签名、ASCII
  行为和 scalar-based `char_count`/limit 语义，保留 trailing logical empty line，并为
  compatibility view 提供完整 grapheme range。`Owner: textarea-grapheme`。`Done when:`
  `state.rs` 小于 800 行，private anchor/focus 支持反向及跨行 selection，所有失败零 mutation，
  crate 外 compatibility fixture证明既有 field construction和ASCII contract。`Verify:`
  `cargo test --workspace --lib --locked components::textarea::state::tests::grapheme_cursor_delete_selection_contract -- --exact`；
  `cargo test --test textarea_unicode_compat --locked grapheme_positions_never_split_utf8_or_wide_cells -- --exact`；
  `cargo test --test textarea_unicode_compat --locked reverse_and_cross_line_selection_is_grapheme_safe -- --exact`；
  `cargo test --test textarea_unicode_compat --locked public_textarea_surface_and_ascii_behavior_compile -- --exact`；
  `cargo test --test textarea_unicode_compat --locked randomized_grapheme_edit_selection_replace_matches_utf8_oracle -- --exact`。property fixture使用 tech
  spec 固定 32-byte ChaCha seed、256 cases、independent UTF-8/grapheme vector oracle 与
  failure-persistence-off config；测试必须以 `-- --exact` 实际运行且非 `#[ignore]`。
  `Dependencies: SP64-T1`。
  `File ownership:`
  `src/components/textarea/state.rs`、`src/components/textarea/state/grapheme.rs`、
  `src/components/textarea/state/tests.rs`、`src/components/textarea/mod.rs`、
  `tests/textarea_unicode_compat.rs`。`Covers: B-002, B-003, B-012, B-018, B-019, B-023`。
  `Handoff:` 记录公开兼容面、grapheme helper 与 limit 语义；停止写上述 paths 后交给 T3。

- [ ] `SP64-T3` 先建立 chat module/public skeleton，再实现 composer source interaction 与 runtime paste。 Covers: B-001, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-017, B-019, B-020, B-021 | Owner: composer-interaction | Done when: T3 module tests可编译/discover，typed atomic ingress、keymap、modes、bounded ack、source revision与 paste routing满足合同 | Verify: T3 下列 exact unit/integration tests
  在任何 composer exact test 前先创建 `src/components/chat/mod.rs`，并在
  `src/components/mod.rs` 声明/export `chat`；同一 task 提供可编译的 composer state/keymap/
  handler public skeleton，并在 `composer/projection.rs` 创建含 `state_revision` accessor
  的最小 typed projection skeleton，不等待 T4/T5。随后实现 closed exhaustive error enums、完整
  committed-text/paste ingress、mode matrix、容量16 FIFO success tombstone、source-only
  revision transition、optional projection handler 参数与 runtime `Event::Paste`
  exactly-once routing。`Done when:` multi-scalar/CRLF/paste整批 staged commit，非法 controls
  带原range typed拒绝，Enter/Shift+Enter/Alt+Enter和binding conflict确定，blank submit不提交，
  first/immediate-repeat/new-draft/new-pending/unknown/evicted ack不丢较新草稿，source-only
  连续key无需fresh projection，所有实际source/pending/ack mutation checked revision恰好+1，
  closed errors可由crate外穷举，Disabled/ReadOnly/submitting及Escape符合合同，runtime paste
  不触发key handler。`Verify:` `cargo test --workspace --lib --locked components::chat::composer::tests::multi_scalar_and_crlf_input_is_atomic -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::control_payload_rejects_without_mutation -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::submit_newline_fallback_and_conflict_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::submission_ack_preserves_or_clears_exact_draft -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::submission_success_tombstone_is_bounded_and_never_clears_new_state -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::cancel_never_clears_draft -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::source_only_key_bursts_do_not_require_fresh_projection -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::source_state_transitions_increment_revision_once -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::source_revision_overflow_is_atomic -- --exact`；
  `cargo test --workspace --lib --locked renderer::runtime::tests::paste_event_dispatches_once_without_key_dispatch -- --exact`；
  `cargo test --test chat_composer_interactions --locked multiline_paste_uses_atomic_text_ingress -- --exact`；
  `cargo test --test chat_composer_interactions --locked blank_and_valid_submit_contract -- --exact`；
  `cargo test --test chat_composer_interactions --locked enabled_readonly_disabled_submitting_matrix -- --exact`；
  `cargo test --test chat_composer_interactions --locked typed_failures_preserve_state_and_sources -- --exact`；
  `cargo test --test chat_composer_interactions --locked closed_error_family_is_exhaustively_matchable -- --exact`；
  `cargo test --test chat_composer_interactions --locked committed_ime_like_sequences_are_grapheme_safe -- --exact`。`Dependencies: SP64-T2`。`File ownership:`
  `src/components/chat/mod.rs`、`src/components/mod.rs`、
  `src/components/chat/composer.rs`、`src/components/chat/composer/state.rs`、
  `src/components/chat/composer/keymap.rs`、`src/components/chat/composer/projection.rs`、
  `src/components/chat/composer/tests.rs`、
  `src/renderer/runtime.rs`、`tests/chat_composer_interactions.rs`。`Covers: B-001, B-004,
  B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-017, B-019, B-020, B-021`。
  `Handoff:` 保存 module/export compile、closed error inventory、tombstone/revision transition
  与exact output；停止写 `composer.rs`/`composer/projection.rs`/`composer/tests.rs` 后只把
  这三个 shared paths显式转交 T4。`chat/mod.rs`、`components/mod.rs`、state/keymap/runtime/
  interaction test ownership在 T3 结束，不由后续 task 接管。

- [ ] `SP64-T4` 建立唯一 TextFlow composer projection/view 与 exact-one public doctest。 Covers: B-003, B-012, B-013, B-014, B-015, B-016, B-017, B-021, B-023 | Owner: composer-projection | Done when: 同一 revision projection 驱动 geometry navigation、selection、auto-grow、resize、trailing caret，且 ChatComposer doctest恰好一个并实际运行 | Verify: T4 下列 exact flow/revision tests 及 `cargo test --workspace --doc --all-features --locked ChatComposer`
  只消费 GH-58 的一个 immutable TextFlow/source-cell map，建立 composer
  projection/view、visual selection/cursor、preferred cell column、auto-grow、visible
  window、resize reflow 与 trailing-LF synthetic caret row；同步把 compatibility TextArea
  cursor/selection rendering 改为 grapheme-safe。`Owner: composer-projection`。
  `Done when:` handler与view共享同一revision-tagged projection，不存在第二套wrap/width
  loop；高度 clamp 到 1..max，cursor保持可见；resize保留source cursor/selection；width=0
  fail-safe为一行clipped cursor；flow及GH-60 errors不被fallback吞掉。`Verify:`
  `cargo test --test chat_composer_flow --locked wrapped_vertical_navigation_preserves_cell_column -- --exact`；
  `cargo test --test chat_composer_flow --locked auto_grow_caps_and_keeps_cursor_visible -- --exact`；
  `cargo test --test chat_composer_flow --locked resize_reflows_same_source_cursor_and_selection -- --exact`；
  `cargo test --test chat_composer_flow --locked empty_trailing_newline_and_zero_width_contract -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::stale_projection_and_repeated_events_are_deterministic -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::projection_observable_transitions_increment_revision_once -- --exact`；
  `cargo test --workspace --lib --locked components::chat::composer::tests::visual_geometry_revision_overflow_is_atomic -- --exact`；
  `cargo test --workspace --doc --all-features --locked ChatComposer`。doctest 必须映射到 public `ChatComposer` item 并实际运行。`Dependencies: SP64-T3`。
  `File ownership:`
  从已停止的 T3 接管 `src/components/chat/composer.rs`、
  `src/components/chat/composer/projection.rs`、`src/components/chat/composer/tests.rs`，
  并独占 `src/components/textarea/component.rs`、
  `tests/chat_composer_flow.rs`。`Covers: B-003, B-012, B-013, B-014, B-015, B-016, B-017,
  B-021, B-023`。`Handoff:` 保存同一 projection identity、resize、synthetic row与exact
  doctest evidence；停止
  写全部 paths 后交给 T5。

- [ ] `SP64-T5` 完成 prelude/docs 和 Claude-only example 迁移。 Covers: B-001, B-018, B-019, B-022, B-023 | Owner: composer-adoption | Done when: prelude/docs、逐 symbol executable docs inventory、Claude真实 semantic/token evidence与exact PR context通过，GLM diff为空 | Verify: public-doc/example exact tests、`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked` 与 `cargo test --test prelude_surfaces --locked claude_example_uses_only_public_composer -- --exact`
  `Done when:` prelude复用 T3 已存在的 `components::chat` concrete exports，scoped
  `forbid(missing_docs)`恰好一次、child lint/doc-hidden escape为零，固定 public API inventory
  每个 symbol由 `tests/chat_composer_public_docs.rs` 中固定独立 crate 外 exact test直接
  type-check/构造/调用并断言 observable；`cfg`、dead code或macro token不得充当evidence，
  `ChatComposer` doctest另为 exact-one runnable；`claude_input_box` 不再拥有 `InlineInputState` 或私有
  input chars/cursor/wrap/visible-row helpers，且不直接输出ANSI定位composer；`glm_chat`
  相对implementation merge-base无diff。example exact test 调用与 `main` 相同的 production path，并断言真实 composer
  state/revision、projection rows/cursor 与 submit payload。`Verify:`
  `cargo test --test prelude_surfaces --locked chat_composer_surface_uses_shared_interaction_types -- --exact`；
  `cargo check --example claude_input_box --all-features --locked`；
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`；
  `cargo test --test prelude_surfaces --locked claude_example_uses_only_public_composer -- --exact`（example 的 production 代码只组合 public composer API，
  不保留本地 editor/wrap/cursor state）。
  `Dependencies: SP64-T4`。`File ownership:` `src/prelude.rs`、`examples/claude_input_box.rs`、
  `docs/CORE_COMPONENT_CONTRACTS.md`、`tests/prelude_surfaces.rs`、
  `tests/chat_composer_public_docs.rs`。`Covers: B-001, B-018,
  B-019, B-022, B-023`。`Handoff:` 保存export compile、docs与example source-scan evidence；停止
  写所有 paths 后交给 T6。

- [ ] `SP64-T6` 在 implementation PR 当前 exact head 上执行只读 closure audit。 Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024 | Owner: verification-review | Done when: dependency、tests、coverage、CI、reviewThreads、review与 PR gate 都绑定 current exact head | Verify: tech spec Product-to-Test Mapping 的全部 exact tests 与 full Rust/docs gates
  `Owner: verification-review`。`Done when:` 核对 B-001..B-024 coverage union、
  root-cause red/green 证据、dependency merged ancestry、全部 exact/full tests、public docs、
  Claude example semantic test、changed-line 及 critical coverage、CI 与 independent review。
  `Verify:` 执行 tech spec Product-to-Test Mapping 的全部 exact tests 与 full Rust/docs gates。`Dependencies:
  SP64-T5`。`File ownership:` 无 writable path；只读审计，不得 resolve threads、approve 或 merge。`Covers: B-001,
  B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013,
  B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024`。
  `Handoff:` 报告 exact head、base OID/merge-base、两 dependency SHAs、命令与fresh outputs；
  人工 merge gate保持不变。

## 并行拆分

本 implementation 不允许 writer 并行：T2 修改 TextArea invariants，T3 依赖其 checked API
并先独占创建 chat module/root export与typed projection skeleton，T4 只接管 T3 明确释放的
`composer.rs`、`composer/projection.rs` 与 `composer/tests.rs`，T5 只消费最终 public
surface且不回写 T3 module files。唯一 DAG 是：

```text
Implementation Gate -> SP64-T1 -> SP64-T2 -> SP64-T3 -> SP64-T4 -> SP64-T5 -> SP64-T6
```

每次 handoff 前原 owner 必须停止写其 paths，并提交 exact head；coordinator 才能把明确列出的
shared file ownership 转给下一 owner。T6 可在所有 writer 停止后并行采集互不修改 repo 的
GitHub evidence，但不得与任何 writer共享 writable file。

## 验证

- list阶段使用 `--exact --include-ignored` 且只有一个 match；ordinary run证明目标
  nonignored并得到 exactly one `1 passed; 0 failed; 0 ignored` summary；include-ignored run
  再证明同一 assertion实际执行并得到相同 summary。
- 运行 T1 到 T5 每个 exact test，包括固定 seed property test。
- 运行 `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`、`cargo test --test prelude_surfaces --locked claude_example_uses_only_public_composer -- --exact`、八个 critical paths 的 coverage、docs、examples 与 full
  Rust gates；覆盖率由既有 CI Coverage job 报告。
- 用 `git diff --name-only <implementation-merge-base>...HEAD` 核对 planned paths；任何
      未声明生产 path 先更新/review packet。
- 对 current GitHub head 收集 CI、review decision、20/20 或当前实际 reviewThreads、
      merge state、independent review 与 SpecRail PR gate；不从旧 SHA 继承结论。

## Handoff Notes

- `Position.col` 的新 Unicode 语义是 logical-line grapheme ordinal；public字段形状、
  TextArea APIs、ASCII行为以及 legacy scalar count/limit语义保持兼容。
- Composer 只支持 committed IME-like text，不声称 native preedit/candidate UI。
- Paste input 复用既有 `PasteEvent`；runtime `Event::Paste` exactly once dispatch，
  terminal bracketed-paste lifecycle不属于组件。
- Submit保留draft直到exact token success acknowledgement；first success进入容量16 FIFO
  tombstone ring，ring内repeat success为revision不变的no-op，failure/unknown/evicted token
  不得清理较新 draft/pending。
- 每个 projection-observable mutation 使用单一 checked revision恰好+1；source-only key
  不要求fresh projection，只有读取visual row/cell的geometry action校验exact projection。
- Error enums 保持closed/exhaustive；只有可扩展behavior/action enums使用
  `#[non_exhaustive]`。
- GH-58 是唯一 wrapping/source-cell算法，GH-60 是独立 checked layout/render error边界；
  若 merged API不能共享同一 immutable projection，停止并更新spec，不能新增sidecar/fallback。
- 回滚使用普通 revert，保留 GH-58/GH-60 correctness与失败evidence，禁止force push。
