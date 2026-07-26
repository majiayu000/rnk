# Task Plan：InlineChatShell 与类型化 scrollback 提交

## Linked Issue

GH-66: https://github.com/majiayu000/rnk/issues/66

## Spec Packet

- Product: [`product.md`](product.md)
- Tech: [`tech.md`](tech.md)

## Implementation Gate

当前只允许 spec work。#66 获得人工 spec approval 与 canonical `ready_to_implement`，且
#62/#63/#64 issue 全部 CLOSED、全部最终 closing implementation PR MERGED、task/PR gate
evidence 完整、merge commits 均为 fresh implementation base 祖先之前，`SP66-T1` 不得写
任何 production/test path。

2026-07-26 当前状态为 blocked：#62/#63/#64 都是 OPEN；#62 PR #117 仍 OPEN 且未合并；
#63/#64 只有 spec PR #75/#79 已合并。spec/draft/parked/open PR 不构成 dependency完成。
实现 owner 必须 fresh 重读三个 merged packets/code；任何 API/path drift 先更新并重新批准本
packet，不得用 alias/private field/sidecar 绕过。

所有 filtered tests 必须用 `-- --exact` 实际运行且非 `#[ignore]`。每个 writer task完成后均
运行：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
```

<!-- gh57-critical-paths-v1
{"version":1,"issue":66,"critical_paths":[{"file":"src/components/chat/inline/tests.rs","name":"gh66_scrollback_lifecycle_contract"},{"file":"src/components/chat/inline/tests.rs","name":"stable_commit_identity_conflict_is_atomic"},{"file":"src/components/chat/inline/tests.rs","name":"native_confirmed_dedup_is_process_local"},{"file":"src/components/chat/inline/tests.rs","name":"partial_write_flush_broken_pipe_outcomes_are_typed"},{"file":"src/components/chat/inline/tests.rs","name":"duplicate_terminal_render_and_delta_are_single_effect"},{"file":"src/components/chat/inline/tests.rs","name":"unknown_blocks_order_and_never_auto_retries"},{"file":"src/components/chat/inline/tests.rs","name":"revision_overflow_and_reentrancy_are_atomic"},{"file":"src/components/chat/inline/tests.rs","name":"sanitizer_rejects_terminal_control_injection"},{"file":"src/components/chat/inline/tests.rs","name":"shutdown_state_machine_is_ordered_and_idempotent"},{"file":"tests/inline_chat_shell.rs","name":"durable_sink_cross_retry_and_restart_reconstruction_is_exactly_once"},{"file":"tests/inline_chat_shell.rs","name":"composer_focus_cancel_and_failure_outcomes_remain_typed"},{"file":"tests/inline_chat_shell.rs","name":"native_restart_never_claims_unknown_terminal_effect"},{"file":"tests/inline_chat_shell.rs","name":"live_resize_never_rewrites_confirmed_scrollback"},{"file":"tests/inline_chat_shell_pty.rs","name":"inline_pty_restores_terminal_on_normal_cancel_failure_and_panic"},{"file":"tests/prelude_surfaces.rs","name":"inline_chat_shell_public_surface_executes"},{"file":"tests/prelude_surfaces.rs","name":"claude_example_uses_public_inline_shell_contract"},{"file":"tests/inline_chat_shell.rs","name":"gh66_current_head_coverage_contract"}]}
-->

## Implementation Tasks

- [ ] `SP66-T1` 执行 dependency/authorization/duplicate preflight，并建立首个可编译 parent/test scaffold。Covers: B-001, B-029, B-031 | Owner: `inline-scaffold-owner` | Done when: dependency/authorization/duplicate gate通过且parent/test scaffold可编译发现 | Verify: 三个ancestry命令、test list与公共checkpoint
  - Covers: B-001, B-029, B-031。
  - Dependencies: Implementation Gate 全部通过。
  - File ownership: 独占 `src/components/chat/mod.rs`、
    `src/components/chat/inline.rs`、`src/components/chat/inline/tests.rs`、
    `tests/inline_chat_shell.rs`；其他 manifest paths只读。
  - Done when: fresh evidence证明三个 dependency CLOSED+final implementation PRs MERGED+
    executable Rust source+完整 pagination/PR gates/ancestry；不存在同scope open work；
    在最终merged `chat` root增加`inline` parent module、private test module和最小无placeholder
    skeleton，以上两个test files可被Cargo发现；不使用`Any`、alias、TODO panic或伪API。
  - Verify:
    `git merge-base --is-ancestor "$GH62_MERGE_SHA" HEAD`；
    `git merge-base --is-ancestor "$GH63_MERGE_SHA" HEAD`；
    `git merge-base --is-ancestor "$GH64_MERGE_SHA" HEAD`；
    `cargo test --test inline_chat_shell --locked -- --list`；
    task公共checkpoint命令。
  - Handoff: 保存fresh JSON、exact base/head和merged public API inventory；停止写四个paths，
    将`inline.rs`/`inline/tests.rs`/integration scaffold明确交给T2。门禁失败则blocked且零repo edit。

- [ ] `SP66-T2` 实现 concrete commit types、safe content identity、closed outcome/error和ANSI sanitizer。Covers: B-002, B-004, B-006, B-017, B-018, B-024 | Owner: `scrollback-contract-owner` | Done when: sync sink/types/error/source/sanitizer合同完整且exact tests通过 | Verify: T2四个exact tests与公共checkpoint
  - Covers: B-002, B-004, B-006, B-017, B-018, B-024。
  - Dependencies: SP66-T1 handoff。
  - File ownership: 独占新增 `src/components/chat/inline/types.rs`、
    `src/components/chat/inline/sanitize.rs`、`src/components/chat/inline/sink.rs`；接管
    `src/components/chat/inline.rs`、`src/components/chat/inline/tests.rs`、
    `tests/inline_chat_shell.rs`。
  - Done when: private-field validated types和sync `ScrollbackSink`签名与tech完全一致；
    exact `Arc<[u8]>` identity无deep-copy；closed outcomes/errors可crate外穷举并保留
    `io::Error::source`；sanitizer只接受printable Unicode/LF/library SGR并拒绝所有其他控制；
    empty/only-SGR/only-whitespace不生成commit。
  - Verify:
    `cargo test --workspace --lib --locked components::chat::inline::tests::stable_commit_identity_conflict_is_atomic -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::sanitizer_rejects_terminal_control_injection -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::empty_state_and_empty_commit_do_not_invent_data -- --exact`；
    `cargo test --test inline_chat_shell --locked closed_scrollback_outcomes_are_exhaustive -- --exact`；
    公共checkpoint命令。
  - Handoff: 保存public inventory、error-source/exhaustive compile output和sanitizer负例；
    停止写shared paths，把`sink.rs`/`inline/tests.rs`交T3，`inline.rs`/integration scaffold交T4。

- [ ] `SP66-T3` 实现 native staged write、process-local confirmed ledger和`NativeInlineSession`显式恢复。Covers: B-005, B-006, B-007, B-008, B-018, B-021, B-022, B-025 | Owner: `native-terminal-owner` | Done when: fault matrix、dedupe、bounded ledger和显式shutdown合同完整 | Verify: T3三个exact tests、file-size gate与公共checkpoint
  - Covers: B-005, B-006, B-007, B-008, B-018, B-021, B-022, B-025。
  - Dependencies: SP66-T2 handoff。
  - File ownership: 独占 `src/renderer/terminal.rs`（只增加child module/export且保持<800）、
    新增 `src/renderer/terminal/inline_scrollback.rs`、
    `src/components/chat/inline/session.rs`；从T2接管
    `src/components/chat/inline/sink.rs`、`src/components/chat/inline/tests.rs`。
  - Done when: generic fault-injection helper区分zero/short/write-zero/delimiter/flush/broken
    pipe/cancel stages；ledger preflight后才write、flush后才confirm，满容量不逐出；duplicate
    same ID/identity不二次write；session只进入inline、sink/render borrow互斥；
    explicit shutdown逐项尝试且报告，repeat shutdown no-op，Drop不声称成功。
  - Verify:
    `cargo test --workspace --lib --locked components::chat::inline::tests::native_confirmed_dedup_is_process_local -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::partial_write_flush_broken_pipe_outcomes_are_typed -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::shutdown_state_machine_is_ordered_and_idempotent -- --exact`；
    `test "$(wc -l < src/renderer/terminal.rs)" -lt 800`；
    公共checkpoint命令。
  - Handoff: 保存fault matrix、accepted-byte counters、shutdown report和file-size output；
    停止写全部paths，将`inline/tests.rs`交T4，sink/session/renderer paths封存只读。

- [ ] `SP66-T4` 实现 shell staging、single-in-flight、confirmed-only removal、order/retry/overflow状态机。Covers: B-002, B-003, B-007, B-009, B-010, B-011, B-012, B-017, B-019, B-020, B-026 | Owner: `inline-lifecycle-owner` | Done when: terminal staging、dedupe、三态、顺序、retry与checked transition合同完整 | Verify: T4六个exact tests与公共checkpoint
  - Covers: B-002, B-003, B-007, B-009, B-010, B-011, B-012, B-017, B-019, B-020, B-026。
  - Dependencies: SP66-T3 handoff与最终merged GH62/GH63 public read/view APIs。
  - File ownership: 独占新增 `src/components/chat/inline/state.rs`；从T1/T3接管
    `src/components/chat/inline.rs`、`src/components/chat/inline/tests.rs`、
    `tests/inline_chat_shell.rs`；上游model/view和native paths只读。
  - Done when: terminal-only candidate按conversation source order冻结exact projection；
    duplicate terminal/render/delta为no-op；Committed receipt exact match后才remove；
    NotCommitted/Unknown/live observations保留；Unknown阻断后续且普通retry拒绝；
    explicit NotCommitted retry可用；capacity/revision/counter/content conflict/reentrancy/
    shutdown-after-call均完整原子；resize只重投影live。
  - Verify:
    `cargo test --workspace --lib --locked components::chat::inline::tests::gh66_scrollback_lifecycle_contract -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::duplicate_terminal_render_and_delta_are_single_effect -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::unknown_blocks_order_and_never_auto_retries -- --exact`；
    `cargo test --workspace --lib --locked components::chat::inline::tests::revision_overflow_and_reentrancy_are_atomic -- --exact`；
    `cargo test --test inline_chat_shell --locked not_committed_retry_is_explicit_and_unknown_retry_is_rejected -- --exact`；
    `cargo test --test inline_chat_shell --locked live_resize_never_rewrites_confirmed_scrollback -- --exact`；
    公共checkpoint命令。
  - Handoff: 保存transition table、no-op revision和exact outputs；停止写shared paths，
    将`state.rs`/`inline.rs`/integration test交T5，lib tests path封存。

- [ ] `SP66-T5` 实现 durable lookup/restart reconstruction、public observation和composer/focus typed routing。Covers: B-012, B-013, B-014, B-015, B-016, B-019, B-025, B-026 | Owner: `inline-recovery-interaction-owner` | Done when: durable/restart/observation/composer/focus合同完整且无clone restore | Verify: T5四个exact tests与公共checkpoint
  - Covers: B-012, B-013, B-014, B-015, B-016, B-019, B-025, B-026。
  - Dependencies: SP66-T4 handoff与最终merged GH64 composer API。
  - File ownership: 从T4接管 `src/components/chat/inline/state.rs`、
    `src/components/chat/inline.rs`、`tests/inline_chat_shell.rs`；从T3只接管
    `src/components/chat/inline/sink.rs`以增加`DurableScrollbackSink`/lookup types；
    其他paths只读。
  - Done when: durable concurrent same-ID只有一次effect并返回同receipt；restart只从
    GH62 validated snapshot重建conversation后逐ID lookup；Committed才confirmed，
    NotCommitted仍live，lookup Unknown unresolved/order-blocked，identity conflict fail；
    observation有完整accessors但无serde/restore入口；composer/focus/mode/cancel/exit保持
    shared typed outcome且failure ack不清draft；native restart不伪造history。
  - Verify:
    `cargo test --test inline_chat_shell --locked durable_sink_cross_retry_and_restart_reconstruction_is_exactly_once -- --exact`；
    `cargo test --test inline_chat_shell --locked public_observation_is_not_a_restore_snapshot -- --exact`；
    `cargo test --test inline_chat_shell --locked composer_focus_cancel_and_failure_outcomes_remain_typed -- --exact`；
    `cargo test --test inline_chat_shell --locked native_restart_never_claims_unknown_terminal_effect -- --exact`；
    公共checkpoint命令。
  - Handoff: 保存durable atomic mock transaction log、restart/lookup matrix和draft/focus
    snapshots；停止写全部paths，public API冻结后交T6。

- [ ] `SP66-T6` 完成exports/docs并逐个人工迁移Claude inline example。Covers: B-001, B-015, B-016, B-024, B-026, B-027, B-028 | Owner: `inline-adoption-owner` | Done when: concrete exports/docs/example public composition与legacy compatibility完整 | Verify: T6三个exact tests、example/docs与公共checkpoint
  - Covers: B-001, B-015, B-016, B-024, B-026, B-027, B-028。
  - Dependencies: SP66-T5 public surface冻结。
  - File ownership: 独占 `src/components/mod.rs`、`src/prelude.rs`、
    `docs/CORE_COMPONENT_CONTRACTS.md`、`examples/claude_input_box.rs`、
    `tests/prelude_surfaces.rs`；chat inline production files只读。
  - Done when: components/prelude导出同一concrete types、public docs无lint逃逸；
    example production helper组合Conversation/view/composer/shell/session并由main/test共用；
    删除`InlineInputState`、chars/cursor/wrap/`app.println` transcript/private ledger/direct
    ANSI；legacy println/Message行为仍编译且不被当成typed commit。
  - Verify:
    `cargo test --test prelude_surfaces --locked inline_chat_shell_public_surface_executes -- --exact`；
    `cargo test --test prelude_surfaces --locked claude_example_uses_public_inline_shell_contract -- --exact`；
    `cargo test --test prelude_surfaces --locked legacy_println_and_message_surfaces_remain_compatible -- --exact`；
    `cargo check --example claude_input_box --all-features --locked`；
    `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`；
    公共checkpoint命令。
  - Handoff: 保存public inventory/docs/example semantic outputs与legacy compatibility；
    停止写所有paths后交T7。

- [ ] `SP66-T7` 增加四路径PTY、current-head coverage producer/validator并执行完整本地验证。Covers: B-021, B-022, B-023, B-024, B-029, B-030, B-031 | Owner: `inline-quality-evidence-owner` | Done when: PTY四路径、deterministic coverage与全部current-head gates通过 | Verify: PTY、coverage produce/validate、full tests/check/docs
  - Covers: B-021, B-022, B-023, B-024, B-029, B-030, B-031。
  - Dependencies: SP66-T1–T6完成且所有writers停止。
  - File ownership: 独占新增 `tests/inline_chat_shell_pty.rs`；从T5接管
    `tests/inline_chat_shell.rs`只增加coverage producer/validator及负例；production/example/
    docs/prelude只读。
  - Done when: 非ignored PTY同一test分别spawn normal/cancel/typed-failure/panic子进程，
    以termios/captured bytes验证raw/cursor/paste/无altscreen；restoration error失败；
    `gh66_current_head_coverage_contract`从committed ledger+diff+raw生成确定性
    `gh57-child-coverage-v1`，validate重算全部sets/hash/percent；critical exact set逐项100%，
    changed executable>=80%；全部mapped/full commands在同一clean exact head通过。
  - Verify:
    `cargo test --test inline_chat_shell_pty --locked inline_pty_restores_terminal_on_normal_cancel_failure_and_panic -- --exact`；
    tech coverage produce/validate三条invocations；
    `cargo test --workspace --all-targets --all-features --locked`；
    `cargo check --all-targets --all-features --locked`；
    `cargo check --example claude_input_box --all-features --locked`；
    docs命令与所有mapped exact commands；`git diff --check`。
  - Handoff: 保存exact head/base/merge-base、PTY四路径、raw/evidence digests、每项coverage与
    full command outputs；任何修正使artifact失效并从current-head coverage起重跑。

- [ ] `SP66-T8` 在implementation PR current exact head执行只读closure audit。Covers: B-001..B-031 | Owner: `independent-inline-reviewer` | Done when: dependency/spec/tasks/tests/PTY/coverage/CI/review/PR gate绑定同一head且完整 | Verify: tech Verification Plan、coverage validate与fresh GitHub evidence
  - Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010,
    B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021,
    B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029, B-030, B-031。
  - Dependencies: SP66-T7 immutable handoff；reviewer与T1–T7 writers分离。
  - File ownership: 无writable repo path；不得resolve thread、approve或merge。
  - Done when: product/tech/tasks sets严格相等；manifest/diff/ownership/compile checkpoints、
    dependency final PR evidence/ancestry、全部exact/full/PTY/coverage、fresh CI、
    reviewThreads、merge state和SpecRail PR gate绑定同一head；删除任一证据的负审计blocked。
  - Verify: 重跑tech Verification Plan与coverage validate；fresh GitHub读取current
    head/checks/reviews/reviewThreads/mergeability；`git status --short`为空且head未变。
  - Handoff: 只报告merge-ready或blocked evidence；即使全部green，最终approval、merge、
    release、#66/#57 closure仍由人类决定。

## Execution Graph and Ownership

```text
Implementation Gate
  -> SP66-T1 scaffold
  -> SP66-T2 types/sanitizer
  -> SP66-T3 native terminal/session
  -> SP66-T4 shell lifecycle
  -> SP66-T5 durable recovery/composer
  -> SP66-T6 exports/docs/example
  -> SP66-T7 PTY/coverage/full verification
  -> SP66-T8 independent read-only closure
```

writer tasks不并行。shared files只按task中明确handoff顺序转移，原owner提交checkpoint并停止写后
下一owner才接管；任一时刻每个path只有一个writer。每个task在其production output同一提交中
创建并跑绿对应exact tests，不预提交依赖future task的红测。`renderer/terminal.rs`只能由T3
修改且保持<800；specs在implementation中只读。

## Invariant Coverage Audit

Expected product set:

`{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029, B-030, B-031}`

Task `Covers:` union:

`{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012, B-013, B-014, B-015, B-016, B-017, B-018, B-019, B-020, B-021, B-022, B-023, B-024, B-025, B-026, B-027, B-028, B-029, B-030, B-031}`

两集合必须严格相等；新增B-ID时同步更新tech mapping、相关task和本审计，禁止只在T8 catch-all补号。

## Handoff Notes

- `Committed`只表示scrollback transport确认；Cancelled/Failed conversation status保持原义。
- native ledger只在同一session confirmed去重，Unknown不自动retry且跨crash不恢复。
- durable exactly-once必须由原子persist+lookup合同证明；shell observation不是restore snapshot。
- sink为同步contract，因为实际terminal write/render lifecycle同步；legacy println queue继续无ack。
- content identity共享exact bytes而不是弱hash；same ID/different bytes fail closed。
- final PR仍需fresh independent review与人工merge；禁止force push。
