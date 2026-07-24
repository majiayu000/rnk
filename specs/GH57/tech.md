# Tech Spec：产品级终端 AI Chat UI 组件体系

## Linked Issue

GH-57: https://github.com/majiayu000/rnk/issues/57

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":57,"complete":true,"paths":["docs/CHAT_UI_COMPONENT_ARCHITECTURE.md","specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md"],"spec_refs":["specs/GH57/product.md","specs/GH57/tech.md","specs/GH57/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件是 umbrella 技术规划，不是实现授权。本 PR 只落地架构文档与
`specs/GH57/{product,tech,tasks}.md`；GH-58 至 GH-68 必须各自在人工批准后的
SpecRail 规格与 implementation PR 中实现和验证，不能由本 PR 代替。

## Codebase Context

以下锚点均按编写时的 `origin/main` 基线 `a7c05a6` 或本 PR 新增文档核实。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 总体架构与审计结论 | `docs/CHAT_UI_COMPONENT_ARCHITECTURE.md:22`, `docs/CHAT_UI_COMPONENT_ARCHITECTURE.md:135`, `docs/CHAT_UI_COMPONENT_ARCHITECTURE.md:469`, `docs/CHAT_UI_COMPONENT_ARCHITECTURE.md:611` | 文档定义组件分层、布局风险与分阶段交付；明确 proposed API 尚不存在 | GH-57 的产品边界与 child 队列必须以已验证事实为基线，不能把提案写成现状 |
| 增量布局入口 | `src/layout/engine.rs:133`, `src/renderer/pipeline.rs:26` | 动态帧通过 `compute_element_incremental` 做 VNode diff/patch，并在失败时回退完整重建 | GH-58～GH-61 承担文本流、身份/顺序、事务 patch、快照一致性与性能工作 |
| 渲染与布局错误边界 | `src/renderer/pipeline.rs:48`, `src/renderer/tree_renderer.rs:51`, `src/renderer/tree_renderer.rs:74` | 缺失布局可转为默认值；文本和 spans 直接写入 Output | B-016、B-017、B-025 要求测量/绘制一致且失败不能伪装成功 |
| 现有简单消息组件 | `src/components/display/message.rs:10`, `src/components/display/message.rs:40`, `src/components/display/message.rs:162`, `src/components/display/message.rs:206` | `Message` 为 role + string；`ToolCall` 与 `ThinkingBlock` 是展示组件，没有完整会话状态机 | GH-62、GH-63 应新增类型化模型与 block view，同时保留兼容表面 |
| 现有多行输入状态 | `src/components/textarea/state.rs:48`, `src/components/interaction.rs:9`, `src/components/interaction.rs:39` | `TextAreaState` 已管理多行、光标、选择和 viewport；交互有 `InteractionMode` / `InteractionOutcome<T>` | GH-64 应复用这些合同，不再从 example 复制输入状态 |
| 当前滚动帮助器 | `src/components/layout/scrollable.rs:178`, `src/components/layout/scrollable.rs:246` | `virtual_scroll_view` 按 item 数量计算；`fixed_bottom_layout` 提供通用固定底部组合 | GH-65 需要新增 chat 专用行高索引；GH-67 可复用固定底部布局而不破坏 fixed-height API |
| Inline 原生输出入口 | `src/runtime/context.rs:333`, `examples/claude_input_box.rs:1`, `examples/claude_input_box.rs:157` | runtime 可通过 `println` 提交静态输出；example 自己管理 live input 与 scrollback | GH-66 要把终态消息 exactly-once 提交与终端恢复收敛为 shell 合同 |
| Fullscreen 与聊天重复实现 | `examples/rnk_chat.rs:10`, `examples/rnk_chat.rs:13`, `examples/rnk_chat.rs:138` | fullscreen example 自建消息类型、滚动和消息视图 | GH-67、GH-68 要迁移到共享模型、MessageList 与 shell |
| 其他 example 输入重复 | `examples/chat.rs:13`, `examples/claude_input_box.rs:34`, `examples/glm_chat/prompt_box.rs:52`, `examples/glm_chat/prompt_box.rs:80` | 多套示例分别实现输入、光标、宽度与 ANSI 定位 | GH-64、GH-68 必须逐个迁移并保留行为证据，不能一次性删除旧路径 |

## 设计方案

### 1. 交付拓扑与门禁

GH-57 只保存产品合同、技术分层、依赖 DAG 和 closure audit 规则。实现拆成四条轨道：

1. 布局轨道：GH-58 → GH-59 → GH-60 → GH-61。
2. 会话与视图轨道：GH-62 → GH-63。
3. 交互与列表轨道：GH-64、GH-65；分别受布局与会话依赖约束。
4. Shell 与 hardening 轨道：GH-66、GH-67 → GH-68。

无依赖轨道可以并行，但“并行完成”不等于绕过上游验收。每个 child 都必须拥有自己的
`product.md`、`tech.md`、`tasks.md`、当前提交验证和最终 implementation PR。child PR 只使用
`Refs GH-57`；spec PR diff 只允许自己的三份 packet。每份 child `tasks.md` 必须提供
`gh57-critical-paths-v1` ledger，以 exact `file + name` 集合约束 coverage artifact。

### 2. 目标分层

目标实现先位于 `rnk::components::chat`，不在首轮拆独立 crate：

1. `model`：`MessageId`、`BlockId`、`MessageBlockEntry`、`MessageRevision`、`UpdateId`、
   `ConversationEvent { event_id, sequence, update }`、`ChatRole`、`ChatMessage`、
   `MessageStatus`、`ConversationUpdate` 与类型化错误；`ChatMessage` 公开 required typed
   revision，序列化缺失/负值/溢出必须拒绝，legacy wrapper 使用 `INITIAL`。`UpdateId`
   通过公开 `new` / `TryFrom<String>` 构造、`Display` / `as_str` 读取，空或仅空白输入返回
   typed `InvalidUpdateId`，外部 adapter 不接触私有字段。公开
   `MessageBlock` 以独立 variant 表达 Text、Markdown、Code、Diff、Quote、Link、Thinking、
   ToolCall、ToolResult、Error 与 TerminalAttachmentSummary，不依赖 provider、传输或存储。
2. `state`：原子应用会话事件，验证身份、block 索引、目标 revision 和生命周期；
   `ConversationUpdate` 以一等 variant 表达 append/insert rich block、edit/delete/resend；
   block mutation 使用 stable `BlockId`、insert index 和 expected revision，resend 保留原终态
   消息并创建 initial revision 新身份；成功 mutation 原子递增一次，维护 conversation
   sequence 与有界 processed-event ledger；窗口内 replay 返回原 outcome，eviction 后返回
   typed `ReplayOutsideRetention`。Complete/Cancel/Fail 也验证 expected revision。
3. `view`：`ChatMessageView`、类型化 block views、`StreamingIndicator`、空/错误状态；
   只展示应用方提供的数据。
4. `composer`：在 `TextAreaState` 和现有 interaction contracts 上构建 `ChatComposer`；
   编辑与光标按 grapheme/cell 语义处理。
5. `message_list`：按终端行维护 `message_id + width/content/variant/expansion revision -> height`，
   用前缀和或等价索引查找可见范围，并独立维护 bottom-follow 与用户锚点。
6. `shell`：`InlineChatShell` 与 `FullscreenChatShell` 共用前五层原语，但分别拥有
   native scrollback 和 alternate-screen 生命周期。
7. `adapter`：由应用将 provider 事件翻译为带 `event_id` 与 `sequence` 的
   `ConversationEvent`；核心 crate 不拥有网络、
   鉴权、密钥、工具执行或持久化。

### 3. 会话更新事务

单次更新的处理顺序固定为：

```text
adapter event(event_id, sequence, update)
  -> validate event identity / sequence / block / transition
  -> clone-or-stage affected state
  -> apply complete update
  -> commit new conversation revision
  -> derive view and shell effects
```

任何验证或应用步骤失败，都返回类型化错误并保持原 revision。retention window 内相同
`event_id`/内容返回已记录结果；eviction 后返回 `ReplayOutsideRetention`；ID 冲突、旧
sequence、gap 或终态后的 update 被拒绝。
block append/insert/replace/text-append 携带 `message_id + expected_revision +
BlockId`（insert 另带 index）；edit/delete 携带 `message_id + expected_revision:
MessageRevision`；resend 携带
`source_message_id + expected_revision + new_message`，只接受终态 source 与全新
`new_message.id` 和 `MessageRevision::INITIAL`，并在一次事务中保留 source、插入新消息。
missing/overflow/stale revision、missing source/block、重复 block identity、index 大于
当前 `len` 或非法 payload 均拒绝整个事件且 state/revision 不变；成功 mutation 返回只递增
一次的公开 revision。Complete/Cancel/Fail 携带 expected revision 并遵守相同原子规则；
retry/regenerate 创建新 `MessageId`。
processed-event ledger 必须有明确容量/持久化边界，越过该边界的重放不得伪装成已证明幂等。

### 4. 文本流与布局快照

布局轨道保留 Taffy 作为 Flexbox 引擎，但建立跨测量、布局和绘制的单一文本流合同：

```text
Element/VNode
  -> TextFlow(content, spans, width, wrap, unicode policy)
  -> transactional Taffy adapter
  -> immutable LayoutSnapshot(cell bounds + clip + scroll + text-flow id)
  -> Output compositor
```

增量 patch 先验证目标、身份和最终 child order，再作为一次事务应用。任一 patch 失败时丢弃
该次增量结果，只允许完整重建一次；重建再失败必须向调用方返回错误。fixed-height
`virtual_scroll_view` 保留原合同，variable-height chat 列表使用独立的行高索引。

### 5. Shell 生命周期

- Inline：流式消息与 Composer 留在 live region；消息达到稳定终态后生成稳定 `commit_id`，
  通过 `ScrollbackSink::commit` 获得 `Committed`、`NotCommitted` 或 `Unknown` typed outcome。
  默认 native-terminal sink 仅对 confirmed commit 提供进程内去重；`Unknown` 不自动重试。
  只有持久化 commit ID 且原子幂等的 injected sink 才提供跨重试 exactly-once。取消/失败不
  伪装完成，退出或 panic 使用统一 guard 恢复 raw mode、光标与输入状态。
- Fullscreen：拥有完整可见 transcript；Composer/status 固定底部。resize 使 TextFlow 与
  height index 按 width revision 失效并重算，同时保存消息/行锚点和焦点；退出恢复原屏幕。

### 6. 兼容与扩展

### 7. 可访问性、能力与安全边界

焦点、loading、streaming、failed、cancelled、disabled 与 read-only 均提供文本或符号语义，
核心操作具有键盘路径。终端缺失某能力时使用有文档的显式降级提示。Tool Call 只是展示值：
核心组件不执行命令、不读取密钥、不请求提权，也不从名称/参数/结果推导授权。

## Product-to-Test Mapping

所有映射都使用 child-specific 的完整 libtest 名称，并以 `-- --exact` 实际运行。
宽泛 substring 过滤、零匹配或 `#[ignore]` 的测试都不算证据。

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | GH-57 closure audit / GH-58～GH-68 | tasks.md 的 Closure 检查清单：逐项确认 11 个 child 均关闭、最终 PR/CI 证据可追溯；任一缺失或 umbrella 提前关闭时阻断 |
| B-002 | 各 child SpecRail packet 与 implementation PR | child spec PR 的 diff 只含自己的三份 packet 文件；critical paths 从各自 committed tasks 解析为唯一 `file + name` 集合 |
| B-003 | child 依赖门禁与 queue ledger | ledger 逐条 edge 满足 `git merge-base --is-ancestor <dependency-merge> <dependent-merge>`，且两个 commit 不同 |
| B-004 | GH-62 model、adapter boundary、GH-68 双后端示例 | `components::chat::model::tests::gh62_provider_independent_model_contract`；`components::chat::hardening::tests::gh68_dual_adapter_state_equivalence` 比较两个独立 adapter 对相同 updates 的 state/view snapshot |
| B-005 | GH-62 state、GH-63 empty/message/status views | `components::chat::model::tests::gh62_empty_and_missing_contract`；snapshot 核对空会话与缺失元数据为空且不伪造 model/token |
| B-006 | GH-62 原子 reducer、block identity、公开 revision 与类型化错误 | `gh62_revisioned_atomic_mutations` 覆盖 block/edit/delete/resend；`gh62_terminal_revision_race_contract` 覆盖 Complete/Cancel/Fail expected revision、stale typed error、state/revision 不变与成功只递增一次 |
| B-007 | GH-62 `MessageStatus` transition table | `components::chat::model::tests::gh62_message_transition_matrix`，枚举所有合法边和终态后的非法 update，并覆盖完整 Tool Call 生命周期 |
| B-008 | GH-62 event envelope/replay ledger、GH-66 commit ID | `gh62_update_id_public_construction` 覆盖公开 ID；`gh62_event_idempotency_contract` 覆盖窗口内 replay；`gh62_replay_retention_boundary` 覆盖 eviction 后 typed `ReplayOutsideRetention`、冲突与 gap |
| B-009 | GH-62 conversation-wide sequence contract | `gh62_ordered_update_contract` 覆盖连续/重复/旧序号/gap；`gh62_terminal_revision_race_contract` 覆盖 edit/delta 后晚到 Complete/Cancel/Fail 被 expected revision 阻断 |
| B-010 | GH-63 typed block views | `components::chat::view::tests::gh63_typed_block_view_matrix` 覆盖全部 11 variants；`components::chat::view::tests::gh63_block_identity_lifecycle_failures` 覆盖 stable identity、合法/非法 lifecycle 与具体 failure reason |
| B-011 | GH-66、GH-67 shell composition | `components::chat::inline::tests::gh66_scrollback_lifecycle_contract` 与 `components::chat::fullscreen::tests::gh67_fixed_bottom_resize_contract` 的 public API assertions 确认两 shell 接受相同 primitives 且不模拟对方生命周期 |
| B-012 | GH-66 Inline lifecycle / typed sink outcomes | exact nonignored `gh66_scrollback_lifecycle_contract` 覆盖 `Committed`/`NotCommitted`/`Unknown`、default/injected sink、重复 complete、cancel/fail、exit/panic 与 commit ID |
| B-013 | GH-67 Fullscreen lifecycle | exact nonignored `gh67_fixed_bottom_resize_contract` 覆盖固定底部、生成中导航、连续 resize、焦点保持、退出/恢复 |
| B-014 | GH-64 `ChatComposer` | `components::chat::composer::tests::gh64_grapheme_editing_contract` 覆盖 configurable submit/newline、bracketed paste、selection、bounded auto-grow、CJK/emoji/combining/ZWJ/CRLF、TextFlow parity，以及 typed error 时 draft 原子不变 |
| B-015 | GH-65 row-based `MessageList` | `components::chat::message_list::tests::gh65_variable_height_anchor_contract`，覆盖 append/prepend/stream growth/expand/resize、copy/selection/search 结果的稳定锚点、paused bottom-follow 与 new-output indicator |
| B-016 | GH-58 shared TextFlow | `layout::text_flow::tests::gh58_measure_draw_parity` 对长词、CJK、emoji、combining、styled spans 比较 measure rows 与实际 Output cells |
| B-017 | GH-59～GH-61 identity、transaction 与 LayoutSnapshot | 逐名运行 `reconciler::tests::gh59_keyed_identity_order_contract`、`layout::transaction::tests::gh60_atomic_patch_failure_contract`、`layout::snapshot::tests::gh61_incremental_full_parity_and_recovery`；覆盖 child order、原子失败与 incremental/full parity/recovery |
| B-018 | GH-63 compatibility wrapper、GH-68 examples | `gh63_legacy_message_revision_compatibility`；exact nonignored `gh68_example_convergence_contract` 逐 example 保存迁移前后行为/snapshot；workspace all-targets 且 example count >0 |
| B-019 | GH-63/64/66/67 semantic states 与 keyboard paths | ANSI snapshots 去色后仍能区分 empty/loading/streaming/disconnected/rate-limited/failed/cancelled 等状态；键盘集成测试覆盖 copy/selection/search 与其他核心操作；人工核对不支持能力有清楚提示 |
| B-020 | GH-61 与 GH-68 benchmarks/stress | 运行各 child 批准的 benchmark/stress，并逐名运行 `layout::snapshot::tests::gh61_incremental_full_parity_and_recovery` 与 `components::chat::hardening::tests::gh68_example_convergence_contract`；缺少结果或回退超阈值时失败 |
| B-021 | GH-62、GH-66 取消/中断状态 | `components::chat::model::tests::gh62_cancellation_contract`，覆盖部分 delta 后 cancel/fail、晚到 update、retry 和 inline 未稳定内容不提交 |
| B-022 | GH-63 Tool Call 展示边界 | `components::chat::view::tests::gh63_tool_call_display_boundary`；依赖审计和人工代码审查确认 chat 模块无 process/shell/secret/permission API |
| B-023 | 各 child CI、PR gate、head SHA 与最终集成提交 | closure 捕获 all-target/all-feature output 并要求 result summary 非零、ignored=0，再运行 mapped exact tests。coverage artifact 的 critical `file + name` 必须与 committed child ledger 集合相等且逐项 100%，changed executable >=80%；不采信 continue-on-error CI |
| B-024 | SpecRail human/spec-only gates | canonical readiness/human gates；GH57 review 只允许 exact four files；每个 child spec PR 只允许自己的 product/tech/tasks；所有 packets 从 committed exact head 验证 |
| B-025 | GH-58～GH-68 typed failure/degradation contract | negative fixtures 覆盖布局、恢复、状态与终端能力失败；人工核对只有文档化的可选能力进入降级，关键错误均阻断成功声明 |

## 数据流

### 输入

- 应用 adapter 输入类型化 `ConversationEvent`、可选元数据和 terminal input events。
- Shell 输入当前 terminal dimensions、capabilities、focus 与 lifecycle events。
- 不直接输入 provider JSON、密钥、网络句柄或可执行 tool callback 到核心模型/视图边界。

### 处理与输出

1. reducer 验证并原子提交会话 revision，输出新 state 或类型化错误。
2. view 将 state 转为类型化 Element tree；未知/非法状态产生可诊断错误视图。
3. TextFlow、布局 adapter 和 LayoutSnapshot 生成 terminal cell 结果。
4. Inline shell 输出 live frame，只对稳定终态生成 commit ID，并完整处理 typed sink outcome。
5. Fullscreen shell 输出完整可见 frame，并持有 row anchor、bottom-follow 与 focus state。

### 持久化与外部调用

核心组件不做会话数据库持久化、provider 请求或工具执行。内存中的 event/commit ledger、
height cache 和 layout snapshot 只服务当前 UI 生命周期，因此默认 terminal sink 不承诺跨进程
exactly-once。需要该保证或跨进程恢复时，由应用提供持久、原子幂等的 sink/state store。
外部调用仅限现有 runtime 的终端绘制/恢复能力。

## 备选方案

- 单一巨型 `Chat`：拒绝。Inline/native scrollback 与 Fullscreen/alternate-screen 的所有权、
  失败和恢复语义不同，统一条件分支会隐藏状态并放大测试矩阵。
- 直接从 examples 复制输入和 scroll 代码：拒绝。现有重复实现正是本计划要消除的问题。
- 直接扩展 item-count `virtual_scroll_view`：拒绝。会破坏现有 fixed-height 合同；chat 需要
  width-dependent row index。
- 首轮创建独立 `rnk-chat` crate：暂缓。先在 `rnk::components::chat` 稳定 runtime/layout
  边界，待 API、依赖和 examples 收敛后再评估版本成本。
- provider SDK 作为核心模型：拒绝。adapter boundary 能保持后端无关并降低密钥/网络攻击面。
- 每次更新完整重建：只作为增量失败的一次显式恢复，不作为静默常态；性能与正确性由
  GH-61 parity/benchmark 共同约束。

## 风险

- Security：Tool Call UI 可能被误解为授权或执行入口。缓解：核心类型不暴露执行 callback，
  测试/审查禁止 process、shell、secret 与提权依赖，所有授权留在应用边界。
- Compatibility：新消息模型、文本流和布局错误返回可能影响已有 `Message`、render 和 examples。
  缓解：兼容包装、逐 example 迁移、弃用窗口、public API tests 与 rollback points。
- Correctness：流式、取消、重复事件、resize 和布局 patch 组合可能产生竞争或重复提交。
  缓解：revision/sequence 验证、原子 reducer、幂等 commit ledger、transactional patch 与负例注入。
- Performance：grapheme TextFlow、variable-height index 和全量 parity 校验可能增加 CPU/内存。
  缓解：revision-keyed cache、局部失效、基线 benchmark；量化阈值由 GH-61/GH-68 child spec 批准。
- Terminal compatibility：Shift+Enter、alternate screen、panic restoration、width policy 在不同终端、
  SSH/tmux 下存在差异。缓解：可配置 fallback key、capability matrix、PTY/E2E 与显式降级提示。
- Maintenance：11 个 child 并行可能导致 spec/API 漂移。缓解：依赖 DAG、每 child 单独 PR、
  stable B-ID、独立 review 和最终 closure audit。
- Evidence integrity：旧 SHA、视觉演示或其他 child 的测试可能被误作完成证据。缓解：每个 PR
  的 gate 绑定当前 head SHA，并查询 fresh CI、review decision 与 unresolved reviewThreads。

## 测试计划

### 本 umbrella docs/spec PR

- [ ] 使用可信且固定 revision 的最新 SpecRail checkout 创建临时镜像，把本 packet 复制到镜像内
      再校验；这避免把 pack 作为 `--repo`、却把 repo 外路径作为 `--spec-dir`：

  ```sh
  test -n "$SPEC_RAIL_ROOT" && test -d "$SPEC_RAIL_ROOT/checks"
  specrail_tmp="$(mktemp -d)"
  specrail_tmp="$(cd "$specrail_tmp" && pwd -P)"
  trap 'rm -rf -- "$specrail_tmp"' EXIT
  cp -R "$SPEC_RAIL_ROOT"/. "$specrail_tmp"/
  mkdir -p "$specrail_tmp/specs/GH57"
  mkdir -p "$specrail_tmp/docs"
  cp specs/GH57/product.md specs/GH57/tech.md specs/GH57/tasks.md \
    "$specrail_tmp/specs/GH57/"
  cp docs/CHAT_UI_COMPONENT_ARCHITECTURE.md "$specrail_tmp/docs/"
  python3 "$specrail_tmp/checks/check_workflow.py" \
    --repo "$specrail_tmp" --spec-dir specs/GH57
  ```
- [ ] `python3 .github/scripts/check_markdown_links.py docs/CHAT_UI_COMPONENT_ARCHITECTURE.md specs/GH57`
- [ ] `git diff --check origin/main...HEAD`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets --all-features --locked`
- [ ] `cargo test --workspace --all-targets --all-features --locked`

### Child implementation 总门槛

- [ ] GH-62 状态机对全部合法边、非法边、空/缺失、重复/乱序/取消与原子失败达到关键路径 100% 覆盖。
- [ ] GH-58～GH-61 提供 Unicode TextFlow、结构 property、incremental/full parity、错误注入与量化 benchmark。
- [ ] GH-63～GH-67 提供 ANSI snapshot、input/scroll/resize 集成测试及 Inline/Fullscreen PTY 端到端证据。
- [ ] GH-68 提供逐 example 迁移、至少两个 adapter、长会话/high-frequency stream stress、兼容矩阵与发布文档。
- [ ] 每个 child tasks 声明唯一 `gh57-critical-paths-v1` `file + name` 集合，并生成
      `gh57-child-coverage-v1` JSON：包含 `child_issue`、最终 PR
      `head_sha`、`generated_at`、非空 coverage `command`、changed executable
      `covered/total` 与 critical `file/name/covered/total`；集合必须严格相等，changed
      executable 至少 80%，每个 critical path 100%。closure 直接验证 artifact，不以 CI
      `continue-on-error` job 代替。
- [ ] 每个 child 在当前 PR head 上通过 fresh CI、独立 review、reviewThreads 与 SpecRail PR gate；人工批准和合并不可自动替代。
- [ ] GH-57 最终 workspace check/tests 只在 clean、与 fresh `origin/main` 相同且包含所有
      child GitHub merge commit 的集成提交上执行；每个 merge commit 都必须对应 evidence
      exact head。check/tests 完成后必须再次 fetch 和执行相同 SHA/clean 断言；运行期间
      remote main 前进、HEAD 改变或 worktree 变脏均阻断 closure。

## 回滚方案

本 umbrella PR 只新增文档与 SpecRail packet，可通过回滚该 PR 移除，不涉及 production API、
数据迁移或终端行为。若任一 child 实现需要回滚，应在该 child 的 tech spec 中定义独立回滚点，
优先恢复兼容 wrapper 或旧 runtime 路径，禁止把布局/状态错误静默降级为成功。

GH-57 不随单个 child 回滚而自动关闭；closure audit 必须重新检查依赖、当前证据和人工 gate。
如果 product/tech spec 尚未获人工批准，保持 `ready-to-spec`，不得启动实现。
