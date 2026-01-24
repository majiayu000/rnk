# RNK 架构总览

> 版本: 2.0.0 (Hybrid Architecture)
> 日期: 2026-01-24
> 状态: 设计阶段

## 执行摘要

RNK 将从当前的 React-like 架构升级为 **Hybrid Architecture**，结合 React Hooks 的便利性和 Elm Command 系统的副作用管理能力。这次重构将解决 10 个关键架构问题，提升代码质量、可维护性和开发体验。

---

## 当前架构问题总结

### 高优先级问题

| 问题 | 影响 | 解决方案 |
|------|------|---------|
| **架构定位模糊** | 缺少统一的副作用管理 | 引入 Command 系统 |
| **app.rs 过于庞大** (1635 行) | 可维护性差 | 拆分为 6 个模块 |
| **全局状态过多** | 难以测试，多实例冲突 | 显式传递 Context |

### 中优先级问题

| 问题 | 影响 | 解决方案 |
|------|------|---------|
| **缺少 Reconciliation** | 每帧全量重建 | Phase 2+ 可选实现 |
| **Hook 系统无验证** | 条件调用导致 bug | 添加开发模式验证 |
| **事件系统简陋** | 功能不完整 | 实现 EventDispatcher |

### 低优先级问题

| 问题 | 影响 | 解决方案 |
|------|------|---------|
| **布局计算无缓存** | 性能瓶颈 | LRU 缓存 |
| **关注点分离不足** | 代码质量 | 重构 Style 结构 |
| **缺少错误边界** | 健壮性 | 实现 error_boundary |
| **异步支持不完整** | 开发体验差 | Command 系统解决 |

---

## 新架构设计：Hybrid Architecture

### 核心理念

**"Hooks 管理局部状态，Commands 管理副作用"**

```rust
// 组件使用 Hooks 管理状态
fn counter_app() -> Element {
    let count = use_signal(|| 0);
    let loading = use_signal(|| false);

    // Commands 管理副作用
    use_cmd(count.get(), |val| {
        if val % 10 == 0 && val > 0 {
            Cmd::batch(vec![
                Cmd::perform(async move {
                    save_to_file(val).await;
                }),
                Cmd::perform(async {
                    show_notification("Milestone reached!").await;
                }),
            ])
        } else {
            Cmd::none()
        }
    });

    Box::column()
        .child(Text::new(format!("Count: {}", count.get())))
        .into_element()
}
```

---

## 模块架构

### 新模块结构

```
src/
├── cmd/                    # 新增：Command 系统
│   ├── mod.rs              # Cmd 枚举和 API
│   ├── executor.rs         # CmdExecutor (tokio 集成)
│   └── tasks.rs            # 预定义任务 (HTTP, Timer, File I/O)
│
├── hooks/
│   ├── context.rs          # Hook 上下文
│   ├── use_signal.rs       # 信号 Hook
│   ├── use_effect.rs       # 副作用 Hook
│   ├── use_cmd.rs          # 新增：命令 Hook
│   ├── use_input.rs        # 输入 Hook
│   ├── use_mouse.rs        # 鼠标 Hook
│   ├── use_app.rs          # App 控制 Hook
│   └── validation.rs       # 新增：Hook 验证
│
├── renderer/
│   ├── app.rs              # 精简：App 主结构 (~300 行)
│   ├── runtime.rs          # 新增：事件循环
│   ├── scheduler.rs        # 新增：渲染调度 (FPS 控制)
│   ├── terminal.rs         # 终端抽象
│   ├── output.rs           # 输出缓冲
│   ├── registry.rs         # 重构：注册表 (非全局)
│   ├── static_content.rs   # 新增：Static 内容处理
│   └── mode_switch.rs      # 新增：模式切换逻辑
│
├── events/                 # 新增：事件系统
│   ├── mod.rs              # 事件类型
│   └── dispatcher.rs       # 事件分发器
│
├── components/
│   ├── error_boundary.rs   # 新增：错误边界
│   └── ...                 # 现有组件
│
├── layout/
│   ├── engine.rs           # 布局引擎
│   ├── measure.rs          # 文本测量
│   └── cache.rs            # 新增：布局缓存
│
└── core/
    ├── element.rs          # Element 定义
    ├── style.rs            # Style 系统
    └── color.rs            # Color 类型
```

---

## 核心类型设计

### 1. Command 系统

```rust
/// 副作用描述符
pub enum Cmd {
    /// 空命令
    None,

    /// 批量执行多个命令
    Batch(Vec<Cmd>),

    /// 执行异步任务
    Perform {
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    },

    /// 延时执行
    Sleep {
        duration: Duration,
        then: Box<Cmd>,
    },
}

impl Cmd {
    /// 创建空命令
    pub fn none() -> Self;

    /// 批量命令
    pub fn batch(cmds: impl IntoIterator<Item = Cmd>) -> Self;

    /// 执行异步函数
    pub fn perform<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static;

    /// 延时
    pub fn sleep(duration: Duration) -> Self;

    /// 链式组合
    pub fn and_then(self, next: Cmd) -> Self;
}
```

### 2. Command Executor

```rust
/// 命令执行器
pub struct CmdExecutor {
    runtime: tokio::runtime::Runtime,
    render_tx: mpsc::UnboundedSender<()>,
}

impl CmdExecutor {
    /// 创建新的执行器
    pub fn new(render_tx: mpsc::UnboundedSender<()>) -> Self;

    /// 执行命令
    pub fn execute(&self, cmd: Cmd);

    /// 关闭执行器
    pub fn shutdown(self);
}
```

### 3. use_cmd Hook

```rust
/// 基于依赖的命令 Hook
pub fn use_cmd<D, F>(deps: D, f: F)
where
    D: Deps + 'static,
    F: FnOnce(D::Output) -> Cmd + 'static;

// 使用示例
use_cmd(count.get(), |count_val| {
    if count_val > 100 {
        Cmd::perform(async {
            send_alert("Count exceeded 100").await;
        })
    } else {
        Cmd::none()
    }
});
```

### 4. 错误边界

```rust
/// 错误边界组件
pub fn error_boundary<F>(
    fallback: impl Fn(&Error) -> Element + 'static,
    child: F,
) -> Element
where
    F: Fn() -> Element + 'static;

// 使用示例
error_boundary(
    |err| Text::new(format!("Error: {}", err)).color(Color::Red),
    || risky_component(),
)
```

---

## 数据流向

```
┌─────────────────────────────────────────────────────────────┐
│                        User Event                            │
│                  (Keyboard, Mouse, Resize)                   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   EventDispatcher                            │
│              ┌──────────────────────────┐                    │
│              │  Dispatch to Handlers    │                    │
│              └──────────┬───────────────┘                    │
└─────────────────────────┼────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Hook Handlers                             │
│     ┌─────────────┐    ┌─────────────┐    ┌──────────┐     │
│     │ use_input   │    │ use_mouse   │    │ use_app  │     │
│     └──────┬──────┘    └──────┬──────┘    └────┬─────┘     │
└────────────┼──────────────────┼─────────────────┼───────────┘
             │                  │                 │
             └──────────────────┴─────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Update Signal State  │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Trigger Re-render    │
                    └───────────┬───────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Render Phase                              │
│  1. HookContext.begin_render()                              │
│  2. Call Component Function                                  │
│  3. Collect Commands (use_cmd)                              │
│  4. HookContext.end_render()                                │
└────────────────────────┬────────────────────────────────────┘
                         │
           ┌─────────────┴─────────────┐
           │                           │
           ▼                           ▼
┌──────────────────────┐   ┌───────────────────────┐
│  Execute Commands    │   │   Layout + Render     │
│                      │   │                       │
│  CmdExecutor.execute │   │  LayoutEngine.compute │
│         │            │   │  Output.render        │
│         ▼            │   │  Terminal.write       │
│  Spawn async tasks   │   └───────────────────────┘
│  (in tokio runtime)  │
│         │            │
│         ▼            │
│  Task completes      │
│         │            │
│         ▼            │
│  Send render request │
└──────────┬───────────┘
           │
           └──────────► Back to Event Loop
```

---

## 事件处理流程

```
Terminal Event
    │
    ▼
Terminal.poll_event()
    │
    ▼
EventDispatcher.dispatch()
    │
    ├─► KeyEvent
    │   └─► INPUT_HANDLERS (per render)
    │       └─► Update Signals
    │           └─► request_render()
    │
    ├─► MouseEvent
    │   └─► MOUSE_HANDLERS (per render)
    │       └─► Update Signals
    │           └─► request_render()
    │
    └─► ResizeEvent
        └─► Clear screen
            └─► request_render()
```

---

## 命令执行流程

```
Component Render
    │
    ▼
use_cmd(deps, |val| Cmd::...)
    │
    ▼
HookContext.queue_cmd(cmd)
    │
    ▼
render_frame() completes
    │
    ▼
HookContext.take_cmds()
    │
    ▼
CmdExecutor.execute(cmd)
    │
    ├─► Cmd::None
    │   └─► (no-op)
    │
    ├─► Cmd::Batch(cmds)
    │   └─► execute each command
    │
    ├─► Cmd::Perform(future)
    │   └─► tokio::spawn(future)
    │       └─► future.await
    │           └─► send render request
    │               └─► next frame
    │
    └─► Cmd::Sleep(duration)
        └─► tokio::time::sleep(duration)
            └─► then execute next cmd
```

---

## 性能优化策略

### 1. 渲染调度

```rust
pub struct RenderScheduler {
    fps: u32,                          // 目标帧率 (default: 60)
    last_render: Instant,
    render_requested: AtomicBool,
}

impl RenderScheduler {
    /// 请求渲染 (可从任何线程调用)
    pub fn request_render(&self);

    /// 检查是否应该渲染 (FPS 限流)
    pub fn should_render(&mut self) -> bool;
}
```

**优化效果**：
- 避免过度渲染
- 限制 CPU 占用
- 流畅的 60 FPS 体验

### 2. 布局缓存

```rust
pub struct LayoutCache {
    cache: LruCache<LayoutKey, Layout>,
}

#[derive(Hash, Eq, PartialEq)]
struct LayoutKey {
    element_id: ElementId,
    width: u16,
    height: u16,
    style_hash: u64,
}
```

**优化效果**：
- 减少重复计算
- 大型 UI 性能提升 30-50%
- 内存占用可控 (LRU 限制大小)

### 3. 文本测量缓存

```rust
pub struct TextMeasureCache {
    cache: LruCache<(String, Option<usize>), (f32, f32)>,
}
```

**优化效果**：
- 文本密集型 UI 性能提升 40-60%
- 避免重复的 Unicode 宽度计算

---

## 测试策略

### 测试金字塔

```
           E2E Tests
          (5% - 关键路径)
         ┌───────────────┐
        │  Integration   │
       │   Tests (20%)  │
      │                │
     │  Unit Tests     │
    │    (75%)        │
   └─────────────────┘
```

### 测试覆盖率目标

| 层级 | 目标 | 工具 |
|------|------|------|
| **单元测试** | 90% | `cargo test --lib` |
| **集成测试** | 85% | `cargo test --test '*'` |
| **文档测试** | 100% | `cargo test --doc` |
| **总体覆盖率** | **87%+** | `cargo tarpaulin` |

### 测试类型

#### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_none() {
        let cmd = Cmd::none();
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn test_cmd_batch_empty() {
        let cmd = Cmd::batch(vec![]);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn test_cmd_batch_single() {
        let cmd = Cmd::batch(vec![Cmd::none()]);
        assert!(matches!(cmd, Cmd::None));
    }
}
```

#### 2. 集成测试

```rust
#[tokio::test]
async fn test_async_data_loading() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let cmd = Cmd::perform(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
    });

    executor.execute(cmd);

    // 等待渲染请求
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("render request");
}
```

#### 3. 属性测试

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_cmd_batch_associative(
        cmds1 in prop::collection::vec(any_cmd(), 0..10),
        cmds2 in prop::collection::vec(any_cmd(), 0..10),
    ) {
        let batch1 = Cmd::batch(cmds1.clone());
        let batch2 = Cmd::batch(cmds2.clone());

        // batch(a ++ b) == batch([batch(a), batch(b)])
        let combined = Cmd::batch(
            cmds1.into_iter().chain(cmds2.into_iter()).collect()
        );
        let nested = Cmd::batch(vec![batch1, batch2]);

        // (需要实现 Cmd 的 PartialEq)
        assert_eq!(combined, nested);
    }
}
```

---

## 质量保证

### CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Format Check
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Unit Tests
        run: cargo test --lib

      - name: Integration Tests
        run: cargo test --test '*'

      - name: Doc Tests
        run: cargo test --doc

      - name: Coverage
        run: |
          cargo tarpaulin --out Html --output-dir coverage
          echo "Coverage: $(grep -oP '(?<=<span class="pc_cov">)\d+\.\d+' coverage/index.html | head -1)%"

      - name: Benchmark
        run: cargo bench --no-run

  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v3
      - run: cargo build --release
```

### 代码审查清单

每个 PR 必须通过：

- [ ] `cargo fmt` - 代码格式化
- [ ] `cargo clippy -- -D warnings` - Linting 无警告
- [ ] `cargo test --all-targets` - 所有测试通过
- [ ] 测试覆盖率 ≥ 87%
- [ ] 性能基准无退化
- [ ] 文档更新完整
- [ ] Changelog 更新

---

## 迁移指南

### 破坏性变更

#### 1. 全局状态移除

**旧代码**:
```rust
use rnk::renderer::request_render;

request_render();  // 全局函数
```

**新代码**:
```rust
use rnk::hooks::use_app;

fn component() -> Element {
    let app = use_app();

    // ...
    app.request_render();  // 显式 context
}
```

#### 2. App 构造方式

**旧代码**:
```rust
let mut app = App::new(|| component());
app.run()?;
```

**新代码**:
```rust
App::new(|| component())
    .fps(60)
    .alternate_screen(true)
    .run()?;
```

#### 3. 异步操作

**旧代码**:
```rust
use_effect_once(|| {
    std::thread::spawn(|| {
        let data = fetch_data();
        // 如何更新 UI？需要手动 request_render
    });
    None
});
```

**新代码**:
```rust
use_cmd((), |_| {
    Cmd::perform(async {
        let data = fetch_data().await;
        // 自动触发 re-render
    })
});
```

### 渐进式迁移

1. **Phase 1**: 添加 Command 系统 (无破坏性)
   - 旧代码继续工作
   - 新功能使用 Cmd

2. **Phase 2**: 重构 App Runtime
   - 提供兼容层
   - 逐步废弃旧 API

3. **Phase 3**: 完全迁移
   - 移除兼容层
   - 发布 2.0

---

## 性能基准

### 目标指标

| 指标 | 目标 | 测量方法 |
|------|------|---------|
| **渲染延迟** | < 16ms (60 FPS) | `criterion` benchmark |
| **命令执行延迟** | < 1ms | 从 execute 到 spawn |
| **内存占用** | < 10MB (空 app) | `valgrind` |
| **Hook 创建开销** | < 100ns | `criterion` benchmark |
| **布局计算** | < 5ms (100 元素) | `criterion` benchmark |

### 性能回归检测

```rust
// benches/render_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_render_frame(c: &mut Criterion) {
    c.bench_function("render_frame_100_elements", |b| {
        let app = setup_test_app();
        b.iter(|| {
            app.render_frame(black_box(100));
        });
    });
}

criterion_group!(benches, bench_render_frame);
criterion_main!(benches);
```

---

## 未来规划

### Phase 5+: 高级特性 (可选)

#### 1. Reconciliation (虚拟 DOM diff)

```rust
pub struct VirtualDom {
    current_tree: Option<Element>,
    work_in_progress: Option<Element>,
}

impl VirtualDom {
    pub fn diff(&self, old: &Element, new: &Element) -> Vec<Patch>;
}

pub enum Patch {
    UpdateStyle { id: ElementId, style: Style },
    UpdateText { id: ElementId, text: String },
    InsertChild { parent: ElementId, child: Element },
    RemoveChild { parent: ElementId, index: usize },
}
```

**收益**:
- 增量更新，性能提升 2-5x
- 保留组件状态 (focus, scroll)

**成本**:
- 实现复杂度高
- 需要 3-4 周开发

#### 2. Suspense (异步数据加载)

```rust
pub struct Suspense {
    fallback: Element,
    children: Vec<Element>,
}

pub struct Resource<T> {
    state: Arc<Mutex<ResourceState<T>>>,
}

impl<T> Resource<T> {
    pub fn read(&self) -> T {
        // 如果未就绪，抛出 Suspend 信号
    }
}

// 使用
fn user_profile() -> Element {
    let user = use_resource(|| fetch_user());

    Suspense::new()
        .fallback(Spinner::new())
        .child(user.read())  // 可能挂起
}
```

#### 3. 并发渲染 (React Fiber-like)

```rust
pub struct WorkLoop {
    work_in_progress: Option<FiberId>,
    current_lanes: Lanes,
}

impl WorkLoop {
    pub fn perform_work_until_deadline(&mut self) -> bool;
    pub fn schedule_update(&mut self, fiber_id: FiberId, lane: Lanes);
}
```

**收益**:
- 长任务不阻塞 UI
- 优先级调度
- 时间切片

**成本**:
- 极高复杂度
- 需要 6+ 个月开发

---

## 参考资料

### 架构设计
- [Elm Architecture Guide](https://guide.elm-lang.org/architecture/)
- [React Fiber Architecture](https://github.com/acdlite/react-fiber-architecture)
- [Bubbletea Documentation](https://github.com/charmbracelet/bubbletea)
- [Terminal UI Design Patterns](https://dev.to/charmbracelet/terminal-ui-design-patterns)

### Rust 实现
- [Tokio Async Runtime](https://tokio.rs/tokio/tutorial)
- [Dioxus Virtual DOM](https://dioxuslabs.com/learn/0.5/reference/virtual_dom)
- [Ratatui Immediate Mode](https://ratatui.rs/concepts/rendering/)

### 性能优化
- [LRU Cache in Rust](https://docs.rs/lru/latest/lru/)
- [Criterion Benchmarking](https://bheisler.github.io/criterion.rs/book/)
- [Terminal Rendering Performance](https://janouch.name/articles/tui-rendering)

### 测试
- [Rust Testing Best Practices](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Proptest Property Testing](https://altsysrq.github.io/proptest-book/)
- [Tarpaulin Coverage](https://github.com/xd009642/tarpaulin)

---

## 附录

### A. 术语表

| 术语 | 定义 |
|------|------|
| **Command** | 副作用的描述符，不直接执行副作用 |
| **Hook** | 组件函数中用于管理状态和副作用的函数 |
| **Signal** | 响应式状态容器，变化时触发重新渲染 |
| **Reconciliation** | 比较新旧虚拟 DOM 树，计算最小更新集 |
| **Fiber** | React 的虚拟 DOM 节点，支持并发渲染 |
| **Suspense** | 异步数据加载的占位符机制 |
| **Effect** | 副作用，如 HTTP 请求、定时器等 |

### B. 设计决策记录 (ADR)

#### ADR-001: 选择 Hybrid Architecture

**Context**: RNK 需要统一的副作用管理机制

**Decision**: 采用 Hybrid Architecture (Hooks + Commands)

**Rationale**:
- 保留 Hooks 的便利性
- 引入 Command 统一管理副作用
- 实现成本可控 (2-3 周)
- 适用于 80% 的 TUI 应用

**Consequences**:
- ✅ 开发体验提升
- ✅ 代码可维护性提升
- ❌ 需要学习 Command 概念
- ❌ 略微增加复杂度

#### ADR-002: 使用 Tokio 作为异步运行时

**Context**: Command 系统需要异步运行时

**Decision**: 使用 Tokio

**Rationale**:
- Rust 事实上的标准
- 成熟稳定，生态丰富
- 性能优秀
- 社区支持好

**Consequences**:
- ✅ 强大的异步能力
- ✅ 丰富的生态 (reqwest, etc.)
- ❌ 增加依赖体积 (~500KB)

#### ADR-003: 不立即实现 Reconciliation

**Context**: Virtual DOM diff 可以提升性能

**Decision**: Phase 1-4 不实现，留给 Phase 5+

**Rationale**:
- 实现复杂度高 (3-4 周)
- 当前性能瓶颈不在此
- 先解决架构问题
- 可以后续渐进式添加

**Consequences**:
- ✅ 降低初期开发成本
- ✅ 快速交付核心功能
- ❌ 每帧全量重建 (可接受)

---

## 变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 2.0.0 | 2026-01-24 | Hybrid Architecture 设计 |
| 1.0.0 | 2025-XX-XX | 初始版本 (React-like) |
