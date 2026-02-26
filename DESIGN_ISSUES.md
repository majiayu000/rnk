# rnk 设计问题分析报告（v2）

> 基于 hooks/组件、核心架构/布局、测试/API 质量三个维度的全面审查
> 更新日期：2026-02-23，基于 auto-optimize-dedup 分支

---

## Fixflow 执行记录（2026-02-26）

### DoR（Definition of Ready）

- 目标：将当前设计问题记录到文档，并按步骤逐项修复；每步完成后更新本文件，再进入下一步。
- 向后兼容：`required`（除非该步明确说明是破坏性调整）。
- 提交策略：`per_step`（每步：修改 -> 验证 -> 更新文档）。
- 验证范围：
  - 步骤级：受影响模块单测 + 必要的编译检查。
  - 最终级：`cargo test`（若耗时或环境限制会说明）。
- 工作区基线：`main...origin/main`，工作区初始无本地改动。
- 当前最新提交：`8cc2633`（2026-02-24）。

### 本轮修复计划

| Step | 问题 | 状态 | 完成条件 |
|------|------|------|----------|
| 1 | `use_input` / `use_mouse` 未占用 hook slot | ✅ 已完成 | 两个 hook 参与 hook 顺序校验，并有回归测试 |
| 2 | `Key` 结构体可表达性不足（30+ bool） | ✅ 已完成 | 增加可匹配键码表达与辅助 API，保留兼容 |
| 3 | examples 与 `Cargo.toml` 注册不一致 | ✅ 已完成 | 明确并落地一致性策略，完成验证 |
| 4 | 高成本架构项（reconciler 接入、App 拆分等） | ✅ 已完成（规划） | 输出下一阶段可执行计划与风险 |
| 5 | M1：reconciler 接入主渲染循环 | ✅ 已完成 | 主循环走增量布局并具备失败回退；回归测试通过 |
| 6 | M2-1：稳定键策略（跨分支冲突治理） | ✅ 已完成 | 路径型 synthetic key 生效，回归测试通过 |
| 7 | Prelude 命名冲突与过载缓解 | ✅ 已完成 | 提供 `LayoutBox` 和 `prelude::lite`，回归测试通过 |
| 8 | `CmdExecutor` runtime 边界健壮性 | ✅ 已完成 | runtime 缺失时 no-op，避免 panic |
| 9 | M2-2：按 key 的测量能力 | ✅ 已完成 | runtime & hook 支持 key 测量，回归测试通过 |
| 10 | M3-1：RenderPipeline 抽离 | ✅ 已完成 | `App` 渲染职责下沉，回归测试通过 |
| 11 | `use_paste` 未占用 hook slot | ✅ 已完成 | `use_paste` 参与 hook 顺序校验，并有回归测试 |
| 12 | M3-2：终端控制职责从 `App` 抽离 | ✅ 已完成 | `TerminalController` 模块接管 mode/cmd/resize/println 处理 |
| 13 | M4-1：`use_debounce` 线程模型收敛 | ✅ 已完成 | 每 hook 实例单 worker 线程，回归测试通过 |
| 14 | M4-2：`use_interval` 线程模型收敛 | ✅ 已完成 | 全局调度线程接管 interval 定时任务，回归测试通过 |
| 15 | M3-3：runtime 队列处理从 `App` 抽离 | ✅ 已完成 | `RuntimeBridge` 模块接管 exec/cmd/mode/println 队列处理 |
| 16 | M2-3：theme 上下文收敛到 RuntimeContext | ✅ 已完成 | runtime 优先 + thread-local 兜底，隔离测试通过 |
| 17 | M2-4：`with_runtime` 生命周期对齐 | ✅ 已完成 | 统一走 `with_hooks` 生命周期，相关回归通过 |
| 18 | M2-5：focus id 改为实例内计数 | ✅ 已完成 | 移除全局 `FOCUS_ID_COUNTER`，focus 回归测试通过 |
| 19 | M3-4：`render_frame` 生命周期入口统一 | ✅ 已完成 | `App` 渲染帧改为统一调用 `with_runtime` |
| 20 | M2-6：终端命令队列迁移到 runtime | ✅ 已完成 | `TerminalCmd` 队列由 `AppRuntime` 持有，回归通过 |
| 21 | M2-7：hook 上下文支持 runtime 回退 | ✅ 已完成 | `current_context` 在缺省时回退 `runtime.hook_context` |
| 22 | M2-8：`AppId` 回收池 | ✅ 已完成 | app 注销后回收 id，后续 app 可复用 |
| 23 | M2-9：可访问性状态初始化收敛 | ✅ 已完成 | runtime 首次读取自动探测并缓存 |
| 24 | M2-10：ID 分配溢出防护 | ✅ 已完成 | Element/App/Focus ID 统一使用 checked_add 防回绕 |
| 25 | M5-1：补齐 `use_ref` 核心 Hook | ✅ 已完成 | 提供非重渲染可变引用 API，覆盖持久化与回调不触发测试 |
| 26 | M5-2：补齐 `use_state` 简化 Hook | ✅ 已完成 | 提供 `(value, setter)` API，覆盖持久化与 render 触发测试 |
| 27 | M5-3：补齐 `create_context/use_context` | ✅ 已完成 | 提供 provider 作用域 API，并纳入 hook 顺序校验 |
| 28 | M5-4：补齐 `use_layout_effect` | ✅ 已完成 | 提供布局副作用 API（当前语义与 `use_effect` 对齐） |
| 29 | M6-1：启用 dirty-row 主渲染路径 | ✅ 已完成 | `Output::render` 改走 dirty-row 聚合，补回归测试 |
| 30 | M7-1：`CmdExecutor` 启动容错 | ✅ 已完成 | runtime 创建失败时降级 no-op，避免启动 panic |
| 31 | M8-1：补齐 App runner 基础单测 | ✅ 已完成 | `exit` 与 runtime_context 退出状态联动测试 |

### 本轮关注问题（来自 2026-02-26 主分支复核）

- 全量渲染路径仍在主循环中（`LayoutEngine::build_tree` 每帧清空重建）。
- `reconciler` 已接入主渲染流程（M1 已完成，保留回退路径）。
- `ElementId` 生成与 clone 语义导致跨帧稳定性不足。
- 事件类 hook 语义需持续收敛（`use_input/use_mouse/use_paste` 已统一占 slot）。
- `Key` 建模可读性与可维护性弱。

### Step 执行日志

#### Step 1 完成：`use_input` / `use_mouse` hook 语义对齐

- 代码改动：
  - `src/hooks/use_input.rs`：`use_input()` 在注册 handler 前占用 hook slot（`use_hook(|| ())`）。
  - `src/hooks/use_mouse.rs`：`use_mouse()` 在注册 handler 前占用 hook slot（`use_hook(|| ())`）。
- 新增回归测试：
  - `test_use_input_participates_in_hook_order`（`should_panic`）
  - `test_use_mouse_participates_in_hook_order`（`should_panic`）
- 验证结果：
  - `cargo test --lib hooks::use_input::tests` ✅
  - `cargo test --lib hooks::use_mouse::tests` ✅

#### Step 2 完成：`Key` 增强为“类型化键码 + 兼容 bool 字段”

- 代码改动：
  - `src/hooks/use_input.rs`：新增 `KeyCodeKind` / `MediaKeyKind`。
  - `src/hooks/use_input.rs`：`Key` 新增 `code`、`character` 字段，以及 `code()` / `is()` / `is_char()`。
  - 保留原有 `up_arrow/down_arrow/ctrl...` 等字段，兼容现有调用代码。
  - `src/hooks/mod.rs` 与 `src/prelude.rs`：导出新键码类型。
- 验证结果：
  - `cargo test --lib hooks::use_input::tests` ✅
  - `cargo test --lib hooks::use_keyboard_shortcut::tests` ✅
  - `cargo test --lib hooks::use_mouse::tests` ✅

#### Step 3 完成：examples 注册策略统一

- 代码改动：
  - `Cargo.toml`：移除手工维护的 `[[example]]` 列表，改为 `examples/*.rs` 自动发现策略。
  - `Cargo.toml`：补充注释，约束后续不再回到“部分手工注册”模式。
- 验证结果：
  - `cargo check --examples` ✅

#### Step 4 完成（规划）：架构级问题里程碑化

- 输出文档：`docs/vibe/design-guard-and-fixflow.md`
- 已定义里程碑：
  - M1：reconciler 接入主渲染（带回退开关）
  - M2：节点身份语义收敛（跨帧可追踪）
  - M3：`App` 职责拆分
  - M4：`use_debounce` / `use_interval` 线程模型收敛
- 每个里程碑均补充：风险、回滚方案、验收矩阵。

#### Step 5 完成：M1 reconciler 接入主渲染循环

- 代码改动：
  - `src/layout/engine.rs`：
    - 新增 `compute_element_incremental()`，串联 `Element -> VNode`、`diff/patch`、失败回退。
    - 新增 `sync_element_node_map()`，确保现有渲染器仍通过 `ElementId` 取布局。
    - 为无显式 key 节点生成路径型 synthetic key，降低 `NodeKey` 冲突风险。
  - `src/renderer/app.rs`：
    - `App` 新增 `previous_vnode`，`render_frame` 切换为增量布局入口。
  - `src/layout/engine.rs` 测试：
    - `test_compute_element_incremental_maps_layouts`
    - `test_compute_element_incremental_uses_reconciler_on_next_frame`
- 验证结果：
  - `cargo test --lib layout::engine::tests::test_compute_element_incremental_maps_layouts` ✅
  - `cargo test --lib layout::engine::tests::test_compute_element_incremental_uses_reconciler_on_next_frame` ✅
  - `cargo test --lib renderer::app::tests::test_registry_cleanup_on_drop` ✅
  - `cargo test --lib` ✅

#### Step 6 完成：M2-1 稳定键策略落地

- 代码改动：
  - `src/layout/engine.rs`：`element_to_vnode()` 对节点 key 生成策略升级：
    - 无 key：`父路径 + 索引 + 类型` synthetic key
    - 有 key：`父路径 + 用户 key` 命名域隔离
  - 目的：避免 `NodeKey` 在全局 `vnode_map` 中跨分支冲突。
- 新增回归测试：
  - `test_incremental_layout_avoids_key_collision_across_branches`
  - `test_incremental_layout_keyed_reorder_no_fallback`
- 验证结果：
  - `cargo test --lib layout::engine::tests::test_incremental_layout_avoids_key_collision_across_branches` ✅
  - `cargo test --lib layout::engine::tests::test_incremental_layout_keyed_reorder_no_fallback` ✅

#### Step 7 完成：Prelude 命名冲突与过载缓解

- 代码改动：
  - `src/prelude.rs`：导出 `LayoutBox`（`components::Box` 别名）。
  - `src/prelude.rs`：新增 `prelude::lite`（低冲突最小导入集）。
  - `src/prelude.rs`：新增编译回归测试 2 条。
- 验证结果：
  - `cargo test --lib prelude::tests::test_layout_box_alias_compiles` ✅
  - `cargo test --lib prelude::tests::test_lite_prelude_compiles` ✅

#### Step 8 完成：`CmdExecutor` runtime 边界健壮性

- 代码改动：
  - `src/cmd/executor.rs`：`execute_cmd()` 在 `runtime` 不可用时安全早退（不再 panic）。
- 新增回归测试：
  - `cmd::executor::tests::test_execute_with_missing_runtime_is_noop`
- 验证结果：
  - `cargo test --lib cmd::executor::tests::test_execute_with_missing_runtime_is_noop` ✅

#### Step 9 完成：M2-2 按 key 的测量能力

- 代码改动：
  - `src/runtime/context.rs`：
    - 新增 `measurements_by_key`。
    - 新增 `set_measure_layouts_with_keys()` 与 `get_measurement_by_key_dims()`。
  - `src/renderer/app.rs`：
    - 渲染帧同步构建 `key -> layout` 映射并写入 runtime。
  - `src/hooks/use_measure.rs`：
    - 新增 `measure_element_by_key()`。
    - `MeasureRef` 新增 `set_key()` / `get_key()`。
  - `src/hooks/mod.rs`、`src/prelude.rs`：导出新 API。
- 验证结果：
  - `cargo test --lib hooks::use_measure::tests` ✅
  - `cargo test --lib runtime::context::tests::test_runtime_context_measurements_by_key` ✅
  - `cargo test --lib renderer::app::tests::test_registry_cleanup_on_drop` ✅

#### Step 10 完成：M3-1 RenderPipeline 抽离

- 代码改动：
  - 新增 `src/renderer/pipeline.rs`，承载动态帧渲染管线。
  - `src/renderer/app.rs`：`render_frame` 调用 `RenderPipeline::render_dynamic_frame()`。
  - `src/renderer/mod.rs`：注册 `pipeline` 模块。
- 验证结果：
  - `cargo test --lib renderer::app::tests::test_registry_cleanup_on_drop` ✅
  - `cargo test --lib layout::engine::tests` ✅
  - `cargo test --lib hooks::use_measure::tests` ✅

#### Step 11 完成：`use_paste` hook 语义对齐

- 代码改动：
  - `src/hooks/paste.rs`：`use_paste()` 在注册 handler 前占用 hook slot（`use_hook(|| ())`）。
- 新增回归测试：
  - `hooks::paste::tests::test_use_paste_participates_in_hook_order`（`should_panic`）
- 验证结果：
  - `cargo test --lib hooks::paste::tests` ✅

#### Step 12 完成：M3-2 `TerminalController` 抽离

- 代码改动：
  - 新增 `src/renderer/terminal_controller.rs`，提取终端控制相关逻辑：
    - `handle_mode_switch()`
    - `handle_terminal_cmd()`
    - `handle_println_messages()`
    - `handle_resize()`
  - `src/renderer/app.rs`：对应方法改为委托 `TerminalController`。
  - `src/renderer/mod.rs`：注册 `terminal_controller` 模块。
- 验证结果：
  - `cargo test --lib renderer::app::tests::test_registry_cleanup_on_drop` ✅
  - `cargo test --lib renderer::runtime::tests` ✅

#### Step 13 完成：M4-1 `use_debounce` 线程模型收敛

- 代码改动：
  - `src/hooks/use_debounce.rs`：从“每次变化 spawn 新线程”改为“每 hook 实例单 worker 线程”。
  - worker 通过 channel 接收最新 `value/delay`，旧等待任务由新消息覆盖。
  - 保留既有 API 与语义，支持 delay 更新后重新计时。
- 验证结果：
  - `cargo test --lib hooks::use_debounce::tests` ✅

#### Step 14 完成：M4-2 `use_interval` 线程模型收敛

- 代码改动：
  - `src/hooks/use_interval.rs`：将“每个 hook 一条线程”的实现替换为全局调度线程。
  - 新增 `IntervalCommand/IntervalTask` 与 `run_interval_scheduler()`，统一注册/注销和到期触发。
  - `use_interval_when()` 改为注册任务并在 cleanup 中注销，不再直接 `thread::spawn`。
- 验证结果：
  - `cargo test --lib hooks::use_interval::tests` ✅

#### Step 15 完成：M3-3 `RuntimeBridge` 抽离

- 代码改动：
  - 新增 `src/renderer/runtime_bridge.rs`，封装 runtime/registry 队列处理：
    - `handle_exec_requests()`
    - `handle_terminal_commands()`
    - `handle_mode_switch_request()`
    - `handle_println_messages()`
  - `src/renderer/app.rs`：`run()` 主循环改为委托 `RuntimeBridge`。
  - `src/renderer/mod.rs`：注册 `runtime_bridge` 模块。
- 验证结果：
  - `cargo test --lib renderer::app::tests::test_registry_cleanup_on_drop` ✅
  - `cargo test --lib renderer::runtime::tests` ✅

#### Step 16 完成：M2-3 theme 上下文收敛

- 代码改动：
  - `src/runtime/context.rs`：新增 runtime 级 theme 状态与 `set_theme()/theme()`。
  - `src/components/theme.rs`：`set_theme/get_theme` 改为 `RuntimeContext` 优先；无 runtime 时回退 thread-local。
  - 新增测试：`components::theme::tests::test_theme_isolated_per_runtime_context`。
- 验证结果：
  - `cargo test --lib components::theme::tests` ✅
  - `cargo test --lib runtime::context::tests` ✅

#### Step 17 完成：M2-4 `with_runtime` 生命周期对齐

- 代码改动：
  - `src/runtime/context.rs`：`with_runtime()` 改为在 runtime 作用域内调用 `with_hooks(runtime.hook_context)`。
  - 保留 `prepare_render()`，并由 `with_hooks` 统一完成 hooks `begin/end/effects` 生命周期。
- 验证结果：
  - `cargo test --lib runtime::context::tests` ✅
  - `cargo test --lib hooks::use_input::tests` ✅
  - `cargo test --lib hooks::use_mouse::tests` ✅

#### Step 18 完成：M2-5 focus id 计数收敛

- 代码改动：
  - `src/hooks/use_focus.rs`：移除全局 `FOCUS_ID_COUNTER`。
  - `FocusManager` 新增 `next_id`，改为实例内分配 focus id。
- 验证结果：
  - `cargo test --lib hooks::use_focus::tests` ✅

#### Step 19 完成：M3-4 `render_frame` 生命周期入口统一

- 代码改动：
  - `src/renderer/app.rs`：`render_frame()` 改为调用 `with_runtime(runtime_context, ...)` 构建组件树。
  - 移除 `App` 内对 `prepare_render + with_hooks` 的手工组合。
- 验证结果：
  - `cargo test --lib renderer::app::tests::test_registry_cleanup_on_drop` ✅
  - `cargo test --lib renderer::runtime::tests` ✅
  - `cargo test --lib runtime::context::tests` ✅

#### Step 20 完成：M2-6 终端命令队列收敛

- 代码改动：
  - `src/renderer/registry.rs`：`AppSink` 新增 `queue_terminal_cmd`；`AppRuntime` 内聚 `TerminalCmd` 队列。
  - `src/renderer/registry.rs`：移除全局 `TERMINAL_CMD_QUEUE` 路径，改为向当前 app runtime 入队。
  - `src/renderer/runtime_bridge.rs`：终端命令改为从 `runtime.take_terminal_cmds()` 获取。
  - `src/hooks/use_app.rs`：`NoopSink` 补齐新 trait 方法实现。
- 验证结果：
  - `cargo test --lib renderer::registry::tests` ✅
  - `cargo test --lib cmd::executor::tests` ✅
  - `cargo test --lib renderer::runtime::tests` ✅

#### Step 21 完成：M2-7 hook 上下文回退

- 代码改动：
  - `src/hooks/context.rs`：`current_context()` 增加 runtime 回退路径（`current_runtime().hook_context()`）。
  - 新增测试：`hooks::context::tests::test_current_context_falls_back_to_runtime`。
- 验证结果：
  - `cargo test --lib hooks::context::tests` ✅
  - `cargo test --lib runtime::context::tests` ✅

#### Step 22 完成：M2-8 `AppId` 回收池

- 代码改动：
  - `src/renderer/registry.rs`：新增 `RECYCLED_APP_IDS`，`AppId::new()` 优先复用回收 id。
  - `unregister_app` 增加 id 回收逻辑（注销后入池）。
  - 新增测试：`renderer::registry::tests::test_app_id_recycled_after_unregister`。
- 验证结果：
  - `cargo test --lib renderer::registry::tests` ✅

#### Step 23 完成：M2-9 可访问性状态初始化收敛

- 代码改动：
  - `src/runtime/context.rs`：新增 `screen_reader_initialized` 及访问方法。
  - `src/hooks/use_accessibility.rs`：runtime 存在时首次读取自动探测并缓存。
  - 新增测试：`hooks::use_accessibility::tests::test_runtime_auto_initializes_on_first_read`。
- 验证结果：
  - `cargo test --lib hooks::use_accessibility::tests` ✅
  - `cargo test --lib runtime::context::tests` ✅

#### Step 24 完成：M2-10 ID 分配溢出防护

- 代码改动：
  - `src/core/element.rs`：`ElementId::new()` 改为 `fetch_update + checked_add`。
  - `src/renderer/registry.rs`：`AppId` 新鲜分配改为 `checked_add` 防回绕。
  - `src/hooks/use_focus.rs`：实例内 focus id 递增改为 `checked_add` 防回绕。
- 验证结果：
  - `cargo test --lib core::element::tests` ✅
  - `cargo test --lib renderer::registry::tests` ✅
  - `cargo test --lib hooks::use_focus::tests` ✅

#### Step 25 完成：M5-1 `use_ref` 核心 Hook 补齐

- 代码改动：
  - 新增 `src/hooks/use_ref.rs`：提供 `use_ref()` 与 `RefHandle<T>`（`get/set/update/with` 系列 API）。
  - `src/hooks/mod.rs`：注册 `use_ref` 模块并导出 `RefHandle/use_ref`。
  - `src/prelude.rs`：在状态 hooks 导出中加入 `RefHandle/use_ref`。
- 新增测试：
  - `hooks::use_ref::tests::test_use_ref_persists_across_renders`
  - `hooks::use_ref::tests::test_use_ref_initializer_runs_once_in_hook_context`
  - `hooks::use_ref::tests::test_use_ref_does_not_trigger_render_callback`
- 验证结果：
  - `cargo test --lib hooks::use_ref::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

#### Step 26 完成：M5-2 `use_state` 简化 Hook 补齐

- 代码改动：
  - 新增 `src/hooks/use_state.rs`：提供 `use_state()` 与 `StateSetter<T>`（`set/update` API）。
  - `src/hooks/mod.rs`：注册 `use_state` 模块并导出 `StateSetter/use_state`。
  - `src/prelude.rs`：在状态 hooks 导出中加入 `StateSetter/use_state`。
- 新增测试：
  - `hooks::use_state::tests::test_use_state_persists_between_renders`
  - `hooks::use_state::tests::test_use_state_setter_requests_render`
- 验证结果：
  - `cargo test --lib hooks::use_state::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

#### Step 27 完成：M5-3 `create_context/use_context` 补齐

- 代码改动：
  - 新增 `src/hooks/use_context.rs`：提供 `Context<T>`、`create_context()`、`use_context()`、`with_context()`。
  - `use_context()` 在存在 hook 上下文时占用 hook slot，参与 hook 顺序校验。
  - `src/hooks/mod.rs`：注册 `use_context` 模块并导出 `Context/create_context/use_context/with_context`。
  - `src/prelude.rs`：在状态 hooks 导出中加入 `Context/create_context/use_context/with_context`。
- 新增测试：
  - `hooks::use_context::tests::test_use_context_returns_default_without_provider`
  - `hooks::use_context::tests::test_with_context_nested_provider_restores_parent`
  - `hooks::use_context::tests::test_use_context_participates_in_hook_order`
- 验证结果：
  - `cargo test --lib hooks::use_context::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

#### Step 28 完成：M5-4 `use_layout_effect` 补齐

- 代码改动：
  - 新增 `src/hooks/use_layout_effect.rs`：提供 `use_layout_effect()`、`use_layout_effect_once()`。
  - 当前阶段将调度语义与 `use_effect` 对齐，并在注释中明确为后续生命周期细化预留。
  - `src/hooks/mod.rs` 与 `src/prelude.rs`：导出 `use_layout_effect/use_layout_effect_once`。
- 新增测试：
  - `hooks::use_layout_effect::tests::test_use_layout_effect_runs_after_render`
  - `hooks::use_layout_effect::tests::test_use_layout_effect_deps_gate_rerun`
  - `hooks::use_layout_effect::tests::test_use_layout_effect_once_runs_once`
- 验证结果：
  - `cargo test --lib hooks::use_layout_effect::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

#### Step 29 完成：M6-1 dirty-row 主渲染路径接入

- 代码改动：
  - `src/renderer/output.rs`：`render()` 优先使用 `render_dirty_rows()` 聚合稀疏行输出，不再默认遍历全部行。
  - 保留 dirty 标记被外部清空时的全量回退路径，保持兼容。
  - 新增回归测试：
    - `test_render_after_clear_dirty_preserves_content`
    - `test_render_sparse_dirty_rows_preserves_line_gaps`
- 验证结果：
  - `cargo test --lib renderer::output::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

#### Step 30 完成：M7-1 `CmdExecutor` 启动容错

- 代码改动：
  - `src/cmd/executor.rs`：`CmdExecutor::new()` 不再 `expect` Tokio runtime 创建成功。
  - runtime 创建失败时降级为 `runtime: None`（no-op 执行器），并输出一次错误提示。
  - 既有 `runtime=None` 行为路径复用：`execute_cmd()` 安全早退，不触发 panic。
- 验证结果：
  - `cargo test --lib cmd::executor::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

#### Step 31 完成：M8-1 App runner 基础单测补齐

- 代码改动：
  - `src/renderer/app.rs`：新增 `renderer::app::tests` 用例：
    - `test_exit_sets_should_exit_flag`
    - `test_exit_updates_runtime_context_exit_state`
- 验证结果：
  - `cargo test --lib renderer::app::tests` ✅
  - `cargo test --lib` ✅
  - `cargo check --examples` ✅

### 关键问题状态更新（2026-02-26）

- 原问题 #1（`use_input/use_mouse` hook slot）已修复。
- 原问题 #1 扩展项（`use_paste` hook slot）已修复。
- 原问题 #3（reconciler 死代码）已完成 M1 接入，后续仍需持续优化 patch 覆盖率。
- 原问题 #12（`Key` 表达力不足）已通过 `KeyCodeKind` 增强，且保持兼容。
- 原问题 #6（`App` 职责过重）已完成 M3-1 / M3-2 / M3-3，进入下一阶段拆分。
- 原问题 #9（定时 hooks 线程开销）已完成 M4-1 / M4-2，进入稳定性观察阶段。
- 原问题 #2（双重上下文）已完成子项 M2-3（theme 上下文收敛），其余子项继续推进。
- 原问题 #2（双重上下文）新增完成子项 M2-4（`with_runtime` 生命周期对齐）。
- 原问题 #2（双重上下文）新增完成子项 M3-4（`render_frame` 生命周期入口统一）。
- 原问题 #2（双重上下文）新增完成子项 M2-6（终端命令队列 runtime 化）。
- 原问题 #2（双重上下文）新增完成子项 M2-7（hook 上下文 runtime 回退）。
- 原问题 #4（全局静态 ID）新增完成子项 M2-8（`AppId` 回收池）。
- 原问题 #4（全局静态 ID）已完成子项 M2-5（focus id 实例内计数）。
- 原问题 #4（全局静态 ID）新增完成子项 M2-10（ID 分配溢出防护）。
- 原问题 #2（双重上下文）新增完成子项 M2-9（可访问性状态初始化收敛）。
- 原问题 #7（缺少 React 核心模式）新增完成子项 M5-1（`use_ref`）。
- 原问题 #7（缺少 React 核心模式）新增完成子项 M5-2（`use_state`）。
- 原问题 #7（缺少 React 核心模式）新增完成子项 M5-3（`create_context/use_context`）。
- 原问题 #7（缺少 React 核心模式）新增完成子项 M5-4（`use_layout_effect`）。
- 原问题 #11（dirty-row 未使用）已修复（M6-1）。
- 原问题 #15（生产路径 unwrap/expect）新增完成子项 M7-1（`CmdExecutor` 启动 panic 收敛）。
- 原问题 #13（App runner 测试缺失）新增完成子项 M8-1（基础行为单测补齐）。

---

## 与 v1 报告的差异

| 原编号 | 状态 | 说明 |
|--------|------|------|
| #4 Signal unwrap | ✅ 已修复 | `lock_utils.rs` 提供 `read_or_recover`/`write_or_recover`，Signal 主 API 已使用 |
| #5 集成测试编译失败 | ✅ 已修复 | `with_hooks` 签名已统一为 `Rc<RefCell>`，所有测试通过 |
| #9 没有 Into\<Element\> | ✅ 已修复 | `impl_into_element!` 宏为 ~55 个组件实现了 `From<T> for Element` |
| #11 两个 Deps trait | ✅ 已修复 | 统一为 `src/hooks/deps.rs` 中的 `DepsHash` trait |

---

> 说明：以下“严重问题/架构问题”章节是 2026-02-23 的基线快照，最新修复状态以上方 Step 执行记录与“关键问题状态更新”为准。

## 一、严重问题（High Priority）

### 1. `use_input` / `use_mouse` handler 累积 bug（仍存在）

**位置**: `src/hooks/use_input.rs:215-219`

`use_input` 直接调用 `register_input_handler`，不经过 `HookContext.use_hook()`：
- 不占用 hook slot，不参与 hook 顺序验证
- 依赖 `RuntimeContext::prepare_render()` 在每帧开头清空 handler 列表
- 但如果 RuntimeContext 不存在（fallback 到 thread-local），`clear_input_handlers()` 只清 thread-local，而 handler 可能注册在 RuntimeContext 中
- 没有 per-handler cleanup 机制，无法实现条件性 input 监听

### 2. 双重上下文系统（RuntimeContext vs thread-locals）（仍存在）

**位置**: `src/runtime/context.rs:327-329` + 11 处独立 thread_local

当前存在 **12 个 thread_local** 分散在不同模块：

| thread_local | 位置 |
|---|---|
| `CURRENT_RUNTIME` | `runtime/context.rs:328` |
| `CURRENT_CONTEXT` | `hooks/context.rs:209` |
| `INPUT_HANDLERS` | `hooks/use_input.rs:155` |
| `MOUSE_HANDLERS` + `MOUSE_ENABLED` | `hooks/use_mouse.rs:138-139` |
| `PASTE_HANDLERS` | `hooks/paste.rs:132` |
| `FOCUS_MANAGER` | `hooks/use_focus.rs:221` |
| `APP_CONTEXT` | `hooks/use_app.rs:186` |
| `MEASURE_CONTEXT` | `hooks/use_measure.rs:54` |
| `SCREEN_READER_ENABLED` | `hooks/use_accessibility.rs:62` |
| `LAST_ACTIVITY` | `hooks/use_idle.rs:28` |
| `CURRENT_THEME` | `components/theme.rs:717` |

`RuntimeContext` 试图统一这些状态，但迁移不完整。`register_input_handler` 先尝试 RuntimeContext、fallback thread-local；`clear_input_handlers` 只清 thread-local。handler 可能注册在一个系统但通过另一个系统 dispatch，导致静默丢失。

**风险**: 多 app 实例场景下状态互相污染；测试中 thread-local 残留导致 flaky test。

### 3. 全量重建每帧，reconciler 是死代码（仍存在）

**位置**: `src/layout/engine.rs:52-57`, `src/reconciler/`

`LayoutEngine::build_tree()` 每帧执行 `taffy.clear()` + `node_map.clear()` + 全量重建。`reconciler` 模块（diff.rs, registry.rs）有完整的 diff/patch/VNode 实现，但未接入主渲染循环 `App::render_frame()`。

**影响**: 100 个元素 = 每帧 100 次 HashMap 插入 + 100 次 Taffy 节点创建 + 100 次 text_content String clone。

### 4. 全局静态 ID 计数器永不重置

**位置**: `src/core/element.rs:7`, `src/hooks/use_focus.rs:7`, `src/renderer/registry.rs:31`

三个 `AtomicU64`/`AtomicUsize` 全局计数器：
- `ELEMENT_ID_COUNTER` — 每创建一个 Element 递增
- `FOCUS_ID_COUNTER` — 每注册一个 focusable 元素递增
- `APP_ID_COUNTER` — 每创建一个 App 递增

长时间运行的应用中，每帧重建所有 Element 意味着 ID 持续增长。虽然 u64 溢出不现实，但 ID 不可复用导致 `node_map` 等 HashMap 的 key 空间无限膨胀。更重要的是，`ElementId` 在帧间不稳定（每帧新 ID），无法用于跨帧追踪同一元素。

---

## 二、架构设计问题（Medium Priority）

### 5. Style 是 God Struct（45+ 字段）（仍存在）

**位置**: `src/core/style.rs`（1053 行）

`Style` 混合四个不相关的关注点：
- 布局（flexbox/sizing/position）
- 视觉（颜色/文字装饰/dim/inverse）
- 边框（样式/颜色/可见性/label）
- 内部标记（`is_static` — `#[doc(hidden)] pub`）

`Style::merge()` 只合并颜色和文字样式，静默忽略布局字段 — 语义陷阱。

### 6. App 是 God Object（仍存在）

**位置**: `src/renderer/app.rs`（506 行，16 字段）

`App<F>` 承担：终端管理、布局计算、事件过滤、命令执行、静态内容渲染、帧率控制、运行时上下文管理、suspend/resume、mode switch。`run()` 方法内嵌事件循环 + 渲染回调，难以单元测试。

### 7. 缺少 React 核心模式（已修复）

| 缺失 | 用途 | 优先级 |
|------|------|--------|
| `use_context` / `create_context` | 跨组件状态共享，替代 prop drilling | ✅ 已完成（Step 27） |
| `use_ref` | 不触发 re-render 的持久化引用 | ✅ 已完成（Step 25） |
| `use_state` (简化版) | 比 `use_signal` 更简单的 `(T, set_fn)` API | ✅ 已完成（Step 26） |
| `use_layout_effect` | layout 计算后、paint 前执行 | ✅ 已完成（Step 28） |

### 8. `Box` 命名与 `std::boxed::Box` 冲突（仍存在）

**位置**: `src/prelude.rs:38`

`use rnk::prelude::*` 后 `Box` shadow `std::boxed::Box`，导致混淆和编译错误。

### 9. `use_debounce` / `use_interval` 每次创建 OS 线程（仍存在）

**位置**: `src/hooks/use_debounce.rs:86`, `src/hooks/use_interval.rs:56`

每次值变化或 interval tick 都 `std::thread::spawn`。快速输入场景（搜索框）短时间内创建大量短命线程。应复用 timer 线程或使用 `Cmd::every()` 模式。

### 10. HookContext(Rc) 与 Signal(Arc) 混合并发模型（仍存在）

**位置**: `src/hooks/context.rs:209` vs `src/hooks/use_signal.rs:9`

`HookContext` 是 `Rc<RefCell<>>` (单线程)，`Signal` 是 `Arc<RwLock<>>` (多线程)。这是有意设计（Signal 需要跨线程传递），但增加心智负担，且 `RenderCallback` 是 `Arc<dyn Fn()>` 而 HookContext 是 `Rc` — 两者不能直接组合。

### 11. Dirty-row 追踪存在但未使用（已修复）

**位置**: `src/renderer/output.rs:90-170`

`Output::render()` 已接入 `render_dirty_rows()` 聚合路径，优先仅序列化脏行并保持稀疏行间隙；在 dirty 标记被外部清空时保留全量回退路径。

### 12. Key 结构体设计问题（新发现）

**位置**: `src/hooks/use_input.rs:7-58`

`Key` 使用 30+ 个 `bool` 字段表示按键状态，而非 enum：
```rust
pub struct Key {
    pub up_arrow: bool,
    pub down_arrow: bool,
    // ... 30+ bool fields
    pub media_play: bool,
    pub volume_mute: bool,
}
```

问题：
- 无法表达"当前按下的是哪个键"，只能逐个检查 bool
- 模式匹配不可能（`match key { Key::Up => ... }` 不行）
- 理论上可以同时 `up_arrow = true, down_arrow = true`，语义无意义
- 对比 crossterm 的 `KeyCode` enum，这是退步

---

## 三、测试与质量问题（Lower Priority）

### 13. App runner 零测试（部分修复）

**位置**: `src/renderer/app.rs`

已新增基础单测覆盖 `exit` 行为与 `RuntimeContext` 退出状态联动；事件循环/渲染帧/suspend-resume 仍需继续补齐。

### 14. Prelude 过于庞大（仍存在，加剧）

**位置**: `src/prelude.rs`（206 行）

`use rnk::prelude::*` 引入约 200+ 个符号。`components/mod.rs` 的显式导出列表已经有 ~120 个类型。命名空间污染严重。

### 15. 生产代码中 unwrap/expect 仍有 212 处

虽然 Signal 核心路径已修复，但整个 codebase 仍有 212 处 `unwrap()`/`expect()`，分布在 29 个文件中。高频文件：
- `cmd/executor.rs`: 64 处
- `hooks/use_effect.rs`: 28 处
- `hooks/use_cmd.rs`: 22 处
- `renderer/output.rs`: 19 处

其中大部分在测试代码中（可接受），但 `cmd/executor.rs` 和 `renderer/output.rs` 是生产路径。

> 更新（Step 30）：`cmd::executor::CmdExecutor::new()` 启动期 `expect` 已移除，runtime 创建失败时改为 no-op 降级。

### 16. Examples 目录混乱（部分改善）

65 个 example 文件 + 7 个 internal examples，但仅 8 个注册在 `Cargo.toml` 的 `[[example]]` 中。已删除部分 debug 文件（在 git status 中可见），但清理未完成。

---

## 四、修复路线图（更新版）

### Phase 1: Bug Fix（紧急）
1. ~~修复集成测试编译~~ ✅
2. 修复 `use_input`/`use_mouse` handler — 通过 `use_hook` 注册，或确保 RuntimeContext 路径下 prepare_render 正确清理
3. 完成 RuntimeContext 迁移 — 移除所有 hook 模块的独立 thread-local

### Phase 2: 安全与健壮性
4. 审计 `cmd/executor.rs` 和 `renderer/output.rs` 中的 unwrap，替换为错误处理
5. ~~Signal lock poisoning~~ ✅（`lock_utils.rs`）
6. 将 `use_debounce`/`use_interval` 的线程创建改为复用模式

### Phase 3: 人体工学
7. ~~impl From\<T\> for Element~~ ✅（`impl_into_element!` 宏）
8. ~~统一 Deps trait~~ ✅（`DepsHash`）
9. 重新设计 `Key` 为 enum（或提供 `Key::is(KeyCode)` 方法）
10. ~~添加 `use_ref` / `use_state` / `use_context` / `use_layout_effect` hook~~ ✅
11. 考虑重命名 `Box` 组件

### Phase 4: 性能与架构
12. 接入 reconciler — `render_frame` 使用 diff/patch 替代全量重建
13. ~~启用 dirty-row 渲染~~ ✅（M6-1）
14. 拆分 Style 为 `LayoutStyle` + `VisualStyle` + `BorderConfig`
15. 稳定 ElementId（帧间可追踪）
16. 拆分 `App` 职责（提取 TerminalManager, RenderPipeline 等）
