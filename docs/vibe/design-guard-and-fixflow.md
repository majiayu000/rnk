# rnk 设计守卫与分步修复（vibe）

更新时间：2026-02-26  
范围：避免同类设计问题重复发生，并按步骤持续修复。

## 目标

1. 把“已发现问题 -> 预防机制 -> 修复步骤 -> 验证命令”串成闭环。  
2. 每完成一步都更新文档状态，再进入下一步。  
3. 保持现有改动（包含本轮 `Cargo.lock` 版本同步）。

## 防再发设计

| 问题类型 | 预防机制 | 自动化手段 |
|---|---|---|
| Hook 语义漂移（`use_input/use_mouse/use_paste` 不占 slot） | 所有事件类 hook 都必须走 hook slot | 增加 `should_panic` 顺序回归测试 |
| 输入键模型可表达性不足 | 对外提供类型化键码，不再只靠 bool | `KeyCodeKind` API + 单测覆盖 |
| Example 清单长期漂移 | 统一单一策略（自动发现） | `Cargo.toml` 不再手工维护零散 `[[example]]` |
| 大型架构问题（reconciler / App 职责） | 先拆分为可交付子任务 | 每子任务独立验收和回归 |
| 核心 Hook 缺失（`use_ref/use_state/use_context/use_layout_effect`） | 提供稳定的 ref/state/context/effect API，减少重复封装与误用 | `hooks::use_ref/use_state/use_context/use_layout_effect::tests` 覆盖核心语义 |

## 分步执行清单

| Step | 任务 | 状态 | 验证 |
|---|---|---|---|
| 1 | `use_input/use_mouse` 占用 hook slot 并补回归测试 | ✅ 已完成 | `cargo test --lib hooks::use_input::tests` / `hooks::use_mouse::tests` |
| 2 | 引入 `KeyCodeKind`（保留旧 bool 字段兼容） | ✅ 已完成 | `cargo test --lib hooks::use_input::tests` / `hooks::use_keyboard_shortcut::tests` |
| 3 | 修复 examples 注册策略不一致 | ✅ 已完成 | `cargo check --examples` |
| 4 | 输出架构级问题下一阶段实施计划 | ✅ 已完成 | 里程碑、风险与验收矩阵 |
| 5 | M1：Reconciler 接入主渲染循环（含回退） | ✅ 已完成 | 定向单测 + `cargo test --lib` |
| 6 | M2-1：稳定键策略（跨分支 key 冲突治理） | ✅ 已完成 | 定向单测 + `cargo test --lib layout::engine::tests` |
| 7 | 减少 prelude 冲突：`LayoutBox` + `prelude::lite` | ✅ 已完成 | 定向单测 + 全量回归 |
| 8 | CmdExecutor 健壮性：runtime 缺失不 panic | ✅ 已完成 | 定向单测 + 全量回归 |
| 9 | M2-2：按 key 测量能力（减少临时 ID 依赖） | ✅ 已完成 | 定向单测 + 全量回归 |
| 10 | M3-1：抽离 RenderPipeline（App 职责拆分首步） | ✅ 已完成 | 定向单测 + 全量回归 |
| 11 | `use_paste` 占用 hook slot 并补顺序回归测试 | ✅ 已完成 | `cargo test --lib hooks::paste::tests` |
| 12 | M3-2：抽离 TerminalController（终端控制职责下沉） | ✅ 已完成 | `cargo test --lib renderer::app::tests` / `renderer::runtime::tests` |
| 13 | M4-1：`use_debounce` 改为单 worker 线程模型 | ✅ 已完成 | `cargo test --lib hooks::use_debounce::tests` |
| 14 | M4-2：`use_interval` 改为全局调度线程模型 | ✅ 已完成 | `cargo test --lib hooks::use_interval::tests` |
| 15 | M3-3：抽离 RuntimeBridge（runtime 队列处理下沉） | ✅ 已完成 | `cargo test --lib renderer::app::tests` / `renderer::runtime::tests` |
| 16 | M2-3：主题上下文改为 RuntimeContext 优先 | ✅ 已完成 | `cargo test --lib components::theme::tests` / `runtime::context::tests` |
| 17 | M2-4：`with_runtime` 与 `with_hooks` 生命周期对齐 | ✅ 已完成 | `cargo test --lib runtime::context::tests` + hooks 定向回归 |
| 18 | M2-5：`FocusManager` 改为实例内 ID 计数 | ✅ 已完成 | `cargo test --lib hooks::use_focus::tests` |
| 19 | M3-4：`App::render_frame` 统一走 `with_runtime` | ✅ 已完成 | `cargo test --lib renderer::app::tests` / `renderer::runtime::tests` |
| 20 | M2-6：终端命令队列迁移到 `AppRuntime` | ✅ 已完成 | `cargo test --lib renderer::registry::tests` / `cmd::executor::tests` |
| 21 | M2-7：`current_context` 支持 runtime 回退 | ✅ 已完成 | `cargo test --lib hooks::context::tests` / `runtime::context::tests` |
| 22 | M2-8：`AppId` 回收池（降低全局计数增长） | ✅ 已完成 | `cargo test --lib renderer::registry::tests` |
| 23 | M2-9：可访问性状态首次读取自动探测 | ✅ 已完成 | `cargo test --lib hooks::use_accessibility::tests` |
| 24 | M2-10：ID 分配统一加溢出防护 | ✅ 已完成 | `cargo test --lib core::element::tests` / `renderer::registry::tests` |
| 25 | M5-1：补齐 `use_ref` 核心 Hook | ✅ 已完成 | `cargo test --lib hooks::use_ref::tests` + 全量回归 |
| 26 | M5-2：补齐 `use_state` 简化 Hook | ✅ 已完成 | `cargo test --lib hooks::use_state::tests` + 全量回归 |
| 27 | M5-3：补齐 `create_context/use_context` | ✅ 已完成 | `cargo test --lib hooks::use_context::tests` + 全量回归 |
| 28 | M5-4：补齐 `use_layout_effect` | ✅ 已完成 | `cargo test --lib hooks::use_layout_effect::tests` + 全量回归 |
| 29 | M6-1：启用 dirty-row 主渲染路径 | ✅ 已完成 | `cargo test --lib renderer::output::tests` + 全量回归 |
| 30 | M7-1：`CmdExecutor` 启动容错 | ✅ 已完成 | `cargo test --lib cmd::executor::tests` + 全量回归 |
| 31 | M8-1：补齐 App runner 基础单测 | ✅ 已完成 | `cargo test --lib renderer::app::tests` + 全量回归 |

## 已完成变更摘要

### Step 1
- `use_input()` / `use_mouse()` 在注册 handler 前占用 hook slot。
- 新增顺序违规回归测试，确保条件调用会触发 panic（与其他 hook 一致）。

### Step 2
- 新增类型化键码：
  - `KeyCodeKind`
  - `MediaKeyKind`
- `Key` 新增：
  - `code` 字段（规范键码）
  - `character` 字段（字符键）
  - `code()` / `is()` / `is_char()` 方法
- 保留原 bool 字段，兼容已有调用（如 `key.up_arrow`）。

## Step 3 实施结果

- 已移除 `Cargo.toml` 内手工 `[[example]]` 列表。  
- 采用 Cargo 自动发现（`examples/*.rs`）单一策略。  
- 已通过 `cargo check --examples` 验证。

## Step 7 实施结果（Prelude 人体工学）

- 新增 `LayoutBox` 别名，避免 `use rnk::prelude::*` 时与 `std::boxed::Box` 冲突。  
- 新增 `prelude::lite`（低冲突最小导入集），便于按需导入。  
- 新增编译回归测试：
  - `prelude::tests::test_layout_box_alias_compiles`
  - `prelude::tests::test_lite_prelude_compiles`

## Step 8 实施结果（CmdExecutor 健壮性）

- 将 `execute_cmd()` 中 `runtime.expect(...)` 改为安全早退（no-op）。  
- 新增测试：`cmd::executor::tests::test_execute_with_missing_runtime_is_noop`。  
- 目标：避免边界状态下出现 panic，提升执行器鲁棒性。

## Step 9 实施结果（M2-2：按 key 测量）

- `RuntimeContext` 增加 `measurements_by_key` 存储及查询接口。  
- `App::render_frame()` 同步 `key -> layout` 映射到 runtime。  
- `use_measure` 新增 `measure_element_by_key()`，`MeasureRef` 新增 `set_key()/get_key()`。  
- 兼容性：原有 `ElementId` 测量路径保持不变。

## Step 10 实施结果（M3-1：RenderPipeline 抽离）

- 新增 `src/renderer/pipeline.rs`，承载动态帧渲染主流程。  
- `App::render_frame()` 仅负责 orchestration，布局/渲染细节下沉到 `RenderPipeline::render_dynamic_frame()`。  
- 行为保持不变，便于后续继续拆分 `App` 职责（M3）。

## Step 11 实施结果（`use_paste` hook 语义对齐）

- `use_paste()` 在注册 handler 前占用 hook slot（`use_hook(|| ())`）。  
- 新增回归测试：`test_use_paste_participates_in_hook_order`（`should_panic`）。  
- 目标：避免 paste hook 再次偏离 hook 顺序约束，和 `use_input/use_mouse` 语义保持一致。

## Step 12 实施结果（M3-2：TerminalController 抽离）

- 新增 `src/renderer/terminal_controller.rs`，承载：
  - `handle_mode_switch()`
  - `handle_terminal_cmd()`
  - `handle_println_messages()`
  - `handle_resize()`
- `App` 中对应逻辑改为委托调用，`render_frame()` 行为保持不变。  
- 目标：继续降低 `App` 的 God Object 复杂度，为 M3 后续 `RuntimeBridge` 抽离铺路。

## Step 13 实施结果（M4-1：`use_debounce` 线程模型收敛）

- `use_debounce` 改为“每个 hook 实例一个 worker 线程”，不再“值变化一次 spawn 一次线程”。  
- worker 通过 channel 接收最新 `value/delay`，仅保留最新待提交值，旧任务自动被覆盖。  
- 保留原有 API 与行为语义（含 delay 变更场景）。  
- 目标：降低高频输入场景下线程抖动风险。

## Step 14 实施结果（M4-2：`use_interval` 线程模型收敛）

- `use_interval_when` 从“每个 hook 创建独立线程”改为“全局调度线程 + 注册/注销任务”。  
- 新增内部调度组件：
  - `IntervalCommand`（`Register/Unregister`）
  - `IntervalTask`（`delay/next_fire/callback`）
  - `run_interval_scheduler()`（统一时钟驱动）
- 保留对外 API 与现有测试语义。  
- 目标：在多 interval 场景下降低 OS 线程数量与调度抖动。

## Step 15 实施结果（M3-3：RuntimeBridge 抽离）

- 新增 `src/renderer/runtime_bridge.rs`，统一处理运行时队列：
  - exec request draining
  - terminal command draining
  - mode switch request
  - println message draining
- `App::run()` 事件循环改为调用 `RuntimeBridge`，移除 `App` 内重复的队列处理实现。  
- 目标：进一步收敛 `App` 的 orchestration 复杂度，为后续 `App` 结构继续瘦身。

## Step 16 实施结果（M2-3：Theme 上下文收敛）

- `RuntimeContext` 新增主题状态与访问方法：`set_theme()/theme()`。  
- `components::set_theme/get_theme` 改为 `RuntimeContext` 优先，thread-local 仅作为无 runtime 时兜底。  
- 新增测试：`test_theme_isolated_per_runtime_context`，验证多 runtime 之间主题隔离。  
- 目标：降低全局 thread-local 主题污染风险，提升多 App 并行场景一致性。

## Step 17 实施结果（M2-4：Runtime/Hook 生命周期对齐）

- `with_runtime()` 改为内部通过 `with_hooks(runtime.hook_context)` 执行闭包。  
- `prepare_render` 仍在 `with_runtime` 开始阶段执行，hooks 的 `begin/end/effects` 统一走 `with_hooks`。  
- 目标：减少 RuntimeContext 与 HookContext 在测试/边界场景下的生命周期漂移。

## Step 18 实施结果（M2-5：Focus ID 计数收敛）

- `use_focus` 移除全局 `FOCUS_ID_COUNTER`。  
- `FocusManager` 新增实例级 `next_id`，由 runtime/app 实例内部生成 focus id。  
- 目标：降低全局静态计数器在多 App 场景下的共享增长风险。

## Step 19 实施结果（M3-4：`render_frame` 生命周期统一）

- `App::render_frame()` 改为通过 `with_runtime(runtime_context, ...)` 构建组件树。  
- 移除 `App` 内手工 `prepare_render + with_hooks` 组合调用，统一到 runtime 层入口。  
- 目标：降低 App 层生命周期编排重复，减少 Runtime/Hook 上下文漂移风险。

## Step 20 实施结果（M2-6：终端命令队列收敛）

- `registry::queue_terminal_cmd()` 不再写全局静态队列，改为写入当前 `AppRuntime` 队列。  
- `AppRuntime` 新增终端命令队列存储与 `queue/take` 方法；`RuntimeBridge` 改为从 runtime 取命令。  
- 目标：避免多 App 场景下终端命令跨实例串扰，降低全局共享状态。

## Step 21 实施结果（M2-7：Hook 上下文回退）

- `hooks::current_context()` 新增回退逻辑：若 thread-local 未设置，则尝试读取 `current_runtime().hook_context()`。  
- 新增测试：`test_current_context_falls_back_to_runtime`。  
- 目标：降低 RuntimeContext 与 HookContext 绑定过程中的“空上下文”边界问题。

## Step 22 实施结果（M2-8：`AppId` 回收池）

- `AppId::new()` 优先从回收池获取 id，不再仅依赖全局 `APP_ID_COUNTER` 单向增长。  
- app 注销时回收 id（`unregister_app` -> recycle），后续 app 可复用。  
- 新增测试：`test_app_id_recycled_after_unregister`。  
- 目标：降低全局静态 id 长期增长。

## Step 23 实施结果（M2-9：可访问性状态初始化收敛）

- `RuntimeContext` 新增 `screen_reader_initialized`。  
- `use_is_screen_reader_enabled()` 在 runtime 场景下首次读取会自动探测并缓存，不再默认直接返回 `false`。  
- 新增测试：`test_runtime_auto_initializes_on_first_read`。  
- 目标：避免可访问性状态误判，提高 runtime 场景一致性。

## Step 24 实施结果（M2-10：ID 分配溢出防护）

- `ElementId::new()`、`AppId` 新鲜分配、`FocusManager` 实例内 id 递增均改为 `checked_add` 语义。  
- 极端计数耗尽时会显式 panic，避免隐式回绕到 `0` 导致 root/保留值冲突。  
- 目标：把“极端边界静默错误”改为“可见失败”。

## Step 25 实施结果（M5-1：`use_ref` 核心 Hook）

- 新增 `src/hooks/use_ref.rs`，提供 `use_ref()` 和 `RefHandle<T>`，支持 `get/set/update/with`，且更新不触发 render callback。  
- `src/hooks/mod.rs` 与 `src/prelude.rs` 已导出 `RefHandle/use_ref`，可直接在 hooks/prelude 使用。  
- 新增回归测试覆盖：
  - 跨 render 持久化
  - initializer 仅首次执行
  - ref 更新不触发 render callback  
- 目标：补齐“非重渲染可变引用”能力，减少把 `use_signal` 当 ref 使用导致的无效重渲染。

## Step 26 实施结果（M5-2：`use_state` 简化 Hook）

- 新增 `src/hooks/use_state.rs`，提供 `use_state()` 与 `StateSetter<T>`，支持 `(value, setter)` 语义与 `set/update` 操作。  
- `src/hooks/mod.rs` 与 `src/prelude.rs` 已导出 `StateSetter/use_state`，可直接通过 hooks/prelude 使用。  
- 新增回归测试覆盖：
  - 跨 render 状态持久化
  - setter 调用触发 render callback  
- 目标：补齐“简单状态 API”，降低在基础场景直接使用 `Signal` 的心智负担。

## Step 27 实施结果（M5-3：`create_context/use_context`）

- 新增 `src/hooks/use_context.rs`，提供 `Context<T>`、`create_context()`、`use_context()`、`with_context()`。  
- `use_context()` 在 hook 上下文存在时会占用 hook slot，纳入 hook 顺序校验。  
- `src/hooks/mod.rs` 与 `src/prelude.rs` 已导出 `Context/create_context/use_context/with_context`。  
- 新增回归测试覆盖：
  - 默认值读取
  - provider 嵌套覆盖与恢复
  - `use_context` 条件调用触发顺序校验 panic  
- 目标：补齐跨组件共享值的基础能力，减少 prop drilling。

## Step 28 实施结果（M5-4：`use_layout_effect`）

- 新增 `src/hooks/use_layout_effect.rs`，提供 `use_layout_effect()` 与 `use_layout_effect_once()`。  
- 当前阶段与 `use_effect` 保持同一调度语义，并为后续“layout 前后阶段细分”保留 API 兼容位。  
- `src/hooks/mod.rs` 与 `src/prelude.rs` 已导出 `use_layout_effect/use_layout_effect_once`。  
- 新增回归测试覆盖：
  - 基本执行
  - deps 变更门控
  - once 语义  
- 目标：补齐核心副作用模式，降低 API 缺失导致的自定义重复封装。

## Step 29 实施结果（M6-1：dirty-row 主渲染路径）

- `src/renderer/output.rs` 的 `render()` 已优先走 `render_dirty_rows()` 聚合路径，减少“每次遍历全部行”的默认开销。  
- 保留 dirty 标记被外部清空时的全量回退分支，确保既有语义不回归。  
- 新增回归测试：
  - `test_render_after_clear_dirty_preserves_content`
  - `test_render_sparse_dirty_rows_preserves_line_gaps`  
- 目标：把 dirty-row 从“仅有基础设施”推进到“主路径实际生效”。

## Step 30 实施结果（M7-1：`CmdExecutor` 启动容错）

- `CmdExecutor::new()` 移除 runtime 创建 `expect`，改为 `ok().map(Arc::new)`。  
- 在极端资源不足导致 runtime 创建失败时，执行器降级为 no-op（`runtime: None`），避免应用启动 panic。  
- 复用现有安全早退路径：`execute_cmd()` 在 `runtime=None` 时直接返回。  
- 目标：将命令执行系统的“启动即崩”风险降级为“功能受限但应用可继续运行”。

## Step 31 实施结果（M8-1：App runner 基础单测）

- 在 `src/renderer/app.rs` 新增 `exit` 行为单测：
  - `test_exit_sets_should_exit_flag`
  - `test_exit_updates_runtime_context_exit_state`  
- 覆盖了 `App::exit()` 对 `should_exit` 原子标记与 `RuntimeContext` 的同步效果。  
- 目标：把 App runner 从“几乎无行为测试”推进到有最小回归保护。

## Step 4 实施结果（架构修复里程碑）

### M1：Reconciler 接入主渲染（先增量布局，再增量输出）

- 变更：
  - 在 `App::render_frame()` 持有 `previous_vnode`，每帧走增量布局入口。
  - `LayoutEngine::compute_element_incremental()` 新增：
    - `Element -> VNode` 转换
    - `diff(old, new)` + `apply_patches()`
    - 失败回退到 `compute_vnode()` 全量构建
    - `ElementId -> NodeKey -> NodeId` 同步映射，兼容现有 `render_element_tree()`。
  - 为无显式 key 节点生成路径型 synthetic key，避免 `vnode_map` 键冲突。
- 风险：
  - patch 与实际树不一致导致布局错乱。
- 回滚：
  - 通过 runtime 开关强制回退全量路径（默认可回退）。
- 验收：
  - 新增增量布局回归测试（layout 映射可用 + 第二帧走 reconciler）。
  - `cargo test --lib` 全绿。 ✅

### M2：稳定节点身份（`ElementId`/`NodeKey` 语义收敛）

- 变更：
  - M2-1（已完成）：
    - 为无 key 节点补路径型 synthetic key（父路径 + 索引 + 类型）。
    - 对有 key 节点加入父路径命名域，避免跨分支同 key 全局冲突。
    - 新增回归测试覆盖“跨分支重复 key”与“keyed 重排不回退”。
  - M2-2（已完成）：
    - `RuntimeContext`/`use_measure` 增加按 key 测量查询，减少对临时 `ElementId` 的依赖。
  - M2-3（已完成）：
    - 主题上下文改为 `RuntimeContext` 优先，降低全局 thread-local 干扰。
  - M2-4（已完成）：
    - `with_runtime` 与 `with_hooks` 生命周期对齐，减少双上下文偏差。
  - M2-5（已完成）：
    - `FocusManager` 改为实例内 ID 计数，减少全局计数器耦合。
  - M2-6（已完成）：
    - 终端命令队列迁移到 `AppRuntime`，减少全局队列共享。
  - M2-7（已完成）：
    - `current_context` 支持 runtime 回退，减少上下文缺失边界。
  - M2-8（已完成）：
    - `AppId` 支持回收复用，减少全局计数增长。
  - M2-9（已完成）：
    - 可访问性状态首次读取自动探测并缓存。
  - M2-10（已完成）：
    - ID 分配统一加溢出防护，避免静默回绕。
- 风险：
  - 与现有 clone 语义冲突，出现 ID 重复。
- 回滚：
  - 保留旧生成路径并提供兼容层，逐步迁移。
- 验收：
  - M2-1 已完成：冲突与重排测试通过。
  - M2-2 已完成：`hooks::use_measure::tests` 与 `runtime::context::tests::test_runtime_context_measurements_by_key` 通过。
  - M2-3 已完成：`components::theme::tests::test_theme_isolated_per_runtime_context` 通过。
  - M2-4 已完成：`runtime::context::tests` 与 `hooks::use_input/use_mouse` 回归通过。
  - M2-5 已完成：`hooks::use_focus::tests` 通过。
  - M2-6 已完成：`renderer::registry::tests` 与 `cmd::executor::tests` 通过。
  - M2-7 已完成：`hooks::context::tests` 与 `runtime::context::tests` 通过。
  - M2-8 已完成：`renderer::registry::tests::test_app_id_recycled_after_unregister` 通过。
  - M2-9 已完成：`hooks::use_accessibility::tests::test_runtime_auto_initializes_on_first_read` 通过。
  - M2-10 已完成：`core::element::tests`、`renderer::registry::tests`、`hooks::use_focus::tests` 通过。

### M3：App 职责拆分（事件循环/渲染管线/终端控制）

- 变更：
  - 从 `App` 中提取 `RenderPipeline`、`TerminalController`、`RuntimeBridge`。
- 当前进展：
  - M3-1 已完成：`RenderPipeline` 已抽离。
  - M3-2 已完成：`TerminalController` 已抽离。
  - M3-3 已完成：`RuntimeBridge` 已抽离。
  - M3-4 已完成：`render_frame` 已统一走 `with_runtime`。
- 风险：
  - 拆分阶段引入状态同步错误。
- 回滚：
  - 保留旧 `App` 外壳，仅内聚调用新组件。
- 验收：
  - 对 `mode switch`、`exec suspend/resume`、`println` 增加单测。

### M4：线程模型收敛（`use_debounce` / `use_interval`）

- 变更：
  - 用共享定时执行器替换“每次变化新建线程”。
- 当前进展：
  - M4-1 已完成：`use_debounce` 已收敛为单 worker 线程模型。
  - M4-2 已完成：`use_interval` 已收敛为全局调度线程模型。
- 风险：
  - 取消/清理时机不正确造成幽灵回调。
- 回滚：
  - 保留旧实现 behind feature flag。
- 验收：
  - 高频输入场景下线程数量稳定，行为测试不回归。

## 本轮统一验证

- `cargo test --lib` ✅（999 passed）
- `cargo check --examples` ✅
