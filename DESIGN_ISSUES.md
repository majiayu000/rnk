# rnk 设计问题分析报告（v2）

> 基于 hooks/组件、核心架构/布局、测试/API 质量三个维度的全面审查
> 更新日期：2026-02-23，基于 auto-optimize-dedup 分支

---

## 与 v1 报告的差异

| 原编号 | 状态 | 说明 |
|--------|------|------|
| #4 Signal unwrap | ✅ 已修复 | `lock_utils.rs` 提供 `read_or_recover`/`write_or_recover`，Signal 主 API 已使用 |
| #5 集成测试编译失败 | ✅ 已修复 | `with_hooks` 签名已统一为 `Rc<RefCell>`，所有测试通过 |
| #9 没有 Into\<Element\> | ✅ 已修复 | `impl_into_element!` 宏为 ~55 个组件实现了 `From<T> for Element` |
| #11 两个 Deps trait | ✅ 已修复 | 统一为 `src/hooks/deps.rs` 中的 `DepsHash` trait |

---

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

### 7. 缺少 React 核心模式（仍存在）

| 缺失 | 用途 | 优先级 |
|------|------|--------|
| `use_context` / `create_context` | 跨组件状态共享，替代 prop drilling | 高 |
| `use_ref` | 不触发 re-render 的持久化引用 | 高 |
| `use_state` (简化版) | 比 `use_signal` 更简单的 `(T, set_fn)` API | 中 |
| `use_layout_effect` | layout 计算后、paint 前执行 | 中 |

### 8. `Box` 命名与 `std::boxed::Box` 冲突（仍存在）

**位置**: `src/prelude.rs:38`

`use rnk::prelude::*` 后 `Box` shadow `std::boxed::Box`，导致混淆和编译错误。

### 9. `use_debounce` / `use_interval` 每次创建 OS 线程（仍存在）

**位置**: `src/hooks/use_debounce.rs:86`, `src/hooks/use_interval.rs:56`

每次值变化或 interval tick 都 `std::thread::spawn`。快速输入场景（搜索框）短时间内创建大量短命线程。应复用 timer 线程或使用 `Cmd::every()` 模式。

### 10. HookContext(Rc) 与 Signal(Arc) 混合并发模型（仍存在）

**位置**: `src/hooks/context.rs:209` vs `src/hooks/use_signal.rs:9`

`HookContext` 是 `Rc<RefCell<>>` (单线程)，`Signal` 是 `Arc<RwLock<>>` (多线程)。这是有意设计（Signal 需要跨线程传递），但增加心智负担，且 `RenderCallback` 是 `Arc<dyn Fn()>` 而 HookContext 是 `Rc` — 两者不能直接组合。

### 11. Dirty-row 追踪存在但未使用（仍存在）

**位置**: `src/renderer/output.rs:90-170`

`Output` 有完整的 dirty-row 基础设施（`dirty_rows`, `is_dirty()`, `render_dirty_rows()`），但主渲染路径 `Output::render()` 始终渲染所有行。`render_dirty_rows()` 在整个 codebase 中零调用。

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

### 13. App runner 零测试（仍存在）

**位置**: `src/renderer/app.rs`

框架最关键的路径（事件循环、渲染帧、命令执行、suspend/resume）没有任何测试。

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
10. 添加 `use_ref` / `use_context` hook
11. 考虑重命名 `Box` 组件

### Phase 4: 性能与架构
12. 接入 reconciler — `render_frame` 使用 diff/patch 替代全量重建
13. 启用 dirty-row 渲染
14. 拆分 Style 为 `LayoutStyle` + `VisualStyle` + `BorderConfig`
15. 稳定 ElementId（帧间可追踪）
16. 拆分 `App` 职责（提取 TerminalManager, RenderPipeline 等）
