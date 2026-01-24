# RNK 架构分析报告

> 日期: 2026-01-24
> 分析师: 架构评审团队
> 版本: 1.0

## 执行摘要

RNK 是一个受 Ink 和 Bubbletea 启发的 Rust 终端 UI 框架，采用 React-like 组件模型 + Hooks 系统。经过深入分析，发现 10 个架构层面的问题，主要集中在：架构定位模糊、代码组织混乱、缺少核心机制（Reconciliation、Command 系统）。

---

## 1. 架构定位模糊 - 混合模式的困境

### 问题描述

RNK 试图结合两种不同的架构模式：

| 框架 | 架构模式 | 核心概念 |
|------|---------|---------|
| **Ink** | React-like | Components + Hooks + Virtual DOM + Reconciliation |
| **Bubbletea** | Elm Architecture | Model + Update + View + Command |

**当前状态**：
- ✅ 采用了 Ink 的组件模型和 Hooks
- ❌ 缺少 Bubbletea 的 **Command 系统**（副作用管理）
- ❌ 缺少 Ink 的 **Reconciliation**（虚拟 DOM diff）

### 代码示例

```rust
// 当前：副作用散落在各处，没有统一管理
let count = use_signal(|| 0);
use_effect((), || {
    // 副作用直接写在这里
    std::thread::spawn(|| {
        // 异步任务如何通知 UI？
    });
    Some(Box::new(|| { /* cleanup */ }))
});

// Bubbletea 的方式：返回 Cmd 统一处理
// fn update(msg: Msg) -> (Model, Cmd)
// Cmd::Batch(vec![
//     Cmd::Http(request),
//     Cmd::Tick(Duration::from_secs(1)),
// ])
```

### 影响

- **没有统一的副作用管理机制**
- **异步操作（HTTP、定时器、文件 I/O）没有标准模式**
- **状态更新和副作用混在一起，难以测试和追踪**

---

## 2. `app.rs` 过于庞大 (1635 行)

### 问题描述

`src/renderer/app.rs` 承担了过多职责：

```
App (1635 lines)
├── App 生命周期管理
├── 事件循环 (poll_event + dispatch)
├── 渲染调度 (FPS 限流)
├── 模式切换 (inline ↔ fullscreen)
├── Static 内容处理
├── Hook 上下文管理
├── 全局注册表管理 (AppRegistry)
└── 跨线程通信 (render_flag, println_queue)
```

### 代码片段

```rust
// app.rs 的部分结构
pub struct App<F: Fn() -> Element> {
    component: F,
    terminal: Terminal,
    layout_engine: LayoutEngine,
    hook_context: Rc<RefCell<HookContext>>,
    options: AppOptions,
    should_exit: Arc<AtomicBool>,
    runtime: Arc<AppRuntime>,         // 跨线程通信
    render_handle: RenderHandle,
    static_lines: Vec<String>,        // Static 内容
    last_width: u16,
    last_height: u16,
}

// 全局注册表
static APP_REGISTRY: OnceLock<Mutex<AppRegistry>> = OnceLock::new();
static CURRENT_APP: AtomicU64 = AtomicU64::new(0);
```

### 建议拆分

```
renderer/
├── app.rs              # App 结构 + 公共 API (300 行)
├── event_loop.rs       # 事件循环 (400 行)
├── render_scheduler.rs # 渲染调度 + FPS 控制 (200 行)
├── registry.rs         # 全局注册表 (300 行)
├── mode_switch.rs      # 模式切换逻辑 (200 行)
└── static_content.rs   # Static 内容处理 (200 行)
```

---

## 3. 全局状态过多

### 问题描述

代码中大量使用全局/线程局部状态：

```rust
// 全局注册表
static APP_REGISTRY: OnceLock<Mutex<AppRegistry>> = OnceLock::new();
static CURRENT_APP: AtomicU64 = AtomicU64::new(0);

// 线程局部 handlers
thread_local! {
    static INPUT_HANDLERS: RefCell<Vec<Box<dyn Fn(&str, &Key)>>> = ...;
    static MOUSE_HANDLERS: RefCell<Vec<Box<dyn Fn(MouseEvent)>>> = ...;
    static APP_CONTEXT: RefCell<Option<AppContext>> = ...;
    static HOOK_CONTEXT: RefCell<Option<Rc<RefCell<HookContext>>>> = ...;
}
```

### 问题

| 问题 | 描述 |
|------|------|
| **难以测试** | 需要 mock 全局状态 |
| **多实例冲突** | 只能有一个 "current" app |
| **隐式依赖** | 代码难以追踪调用链 |
| **内存泄漏风险** | Handler 列表可能未清理 |

### 改进方向

```rust
// 改进：显式传递 Context
pub struct RenderContext<'a> {
    terminal: &'a mut Terminal,
    layout_engine: &'a mut LayoutEngine,
    event_handlers: &'a mut EventHandlers,
}

fn render_frame(ctx: &mut RenderContext) -> Result<()> {
    // 显式依赖，易于测试
}
```

---

## 4. 缺少 Reconciliation（协调/Diff）机制

### 问题描述

Ink 和 React 的核心是 Virtual DOM diff，但 RNK：

```rust
// Element clone 会生成新 ID
impl Clone for Element {
    fn clone(&self) -> Self {
        Self {
            id: ElementId::new(),  // ❌ 每次 clone 都是新 ID！
            element_type: self.element_type,
            style: self.style.clone(),
            children: self.children.clone(),
            ...
        }
    }
}
```

**后果**：
- 每帧都是全量重建，无法复用节点
- 无法实现高效的增量更新
- `key` 属性形同虚设
- 无法保留组件状态（如 focus、scroll）

### 对比：Ink 的实现

```javascript
// Ink 使用 React 的 reconciliation
// key 用于识别同一元素
<Box key="header">...</Box>

// React fiber 架构保证：
// - 相同 key + 类型 = 复用节点
// - 不同 key = 重新创建
```

---

## 5. Hook 系统缺少安全保障

### 问题描述

```rust
pub struct HookContext {
    hooks: Vec<HookStorage>,
    hook_index: usize,  // ❌ 依赖调用顺序，无验证
}
```

**问题**：
- 没有验证 hook 调用顺序一致性
- 条件调用 hook 会导致索引错乱
- 没有开发模式警告

### 危险代码示例

```rust
// ❌ 这种代码会导致 bug，但没有任何警告
fn my_component(show: bool) -> Element {
    if show {
        let x = use_signal(|| 0);  // 条件调用！
    }
    let y = use_signal(|| 1);
    // 当 show 改变时，y 的索引会错乱
}
```

### 改进建议

```rust
// 1. 开发模式下记录 hook 数量
#[cfg(debug_assertions)]
fn end_render(&mut self) {
    if let Some(prev_count) = self.prev_hook_count {
        if prev_count != self.hooks.len() {
            panic!("Hook count mismatch! Previous: {}, Current: {}",
                   prev_count, self.hooks.len());
        }
    }
    self.prev_hook_count = Some(self.hooks.len());
}

// 2. 使用宏强制编译时检查
#[proc_macro]
pub fn component(input: TokenStream) -> TokenStream {
    // 检测条件 hook 调用
}
```

---

## 6. 事件系统设计简陋

### 问题描述

```rust
// 全局 handler 列表，没有作用域
pub fn use_input<F: Fn(&str, &Key) + 'static>(handler: F) {
    INPUT_HANDLERS.with(|handlers| {
        handlers.borrow_mut().push(Box::new(handler));
    });
}
```

**缺失功能**：

| 功能 | 状态 | 说明 |
|------|------|------|
| 事件冒泡/捕获 | ❌ | 所有 handler 都会被调用 |
| `stopPropagation` | ❌ | 无法阻止事件传播 |
| 焦点系统集成 | ❌ | 无法只处理焦点元素的事件 |
| Handler 绑定到元素 | ❌ | 无法精确控制哪个元素处理事件 |

### 对比：Web 事件模型

```
Event Flow:
Window → Document → Body → ... → Target → ... → Body → Document → Window
        └─ Capture ─┘              └─ Bubble ─┘

API:
- addEventListener(type, handler, { capture: true })
- event.stopPropagation()
- event.preventDefault()
```

---

## 7. 布局计算没有缓存

### 问题描述

```rust
fn measure_text_node(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<AvailableSpace>,
    node_context: Option<&mut NodeContext>,
) -> taffy::Size<f32> {
    // ❌ 每次布局都重新计算文本宽度
    let text_width = measure_text_width(text) as f32;

    // ❌ 重新计算换行
    let wrapped = wrap_text(text, max_width);
    ...
}
```

### 性能影响

| 场景 | 计算次数 | 影响 |
|------|---------|------|
| 100 个文本节点 | 100 次宽度计算 | 每帧 ~5ms |
| 复杂表格 (500 单元格) | 500 次测量 | 每帧 ~25ms |
| 60 FPS 目标 | 预算 16.67ms/帧 | ⚠️ 可能掉帧 |

### 改进方向

```rust
// 使用 LRU 缓存
use lru::LruCache;

struct TextMeasureCache {
    cache: LruCache<(String, Option<usize>), (f32, f32)>,
}

impl TextMeasureCache {
    fn measure(&mut self, text: &str, max_width: Option<usize>) -> (f32, f32) {
        let key = (text.to_string(), max_width);
        if let Some(&size) = self.cache.get(&key) {
            return size;
        }
        let size = compute_size(text, max_width);
        self.cache.put(key, size);
        size
    }
}
```

---

## 8. 关注点分离不足

### 问题描述

`Style` 结构混合了多种概念：

```rust
pub struct Style {
    // ✅ Flexbox 布局属性
    pub flex_direction: FlexDirection,
    pub align_items: AlignItems,

    // ✅ 视觉样式
    pub color: Option<Color>,
    pub bold: bool,

    // ❌ 内部标记（不应该在这里）
    pub is_static: bool,  // 这是组件行为，不是样式

    // ❌ 混合了多个关注点
    pub overflow_x: Overflow,  // 布局行为
    pub border_style: BorderStyle,  // 视觉样式
    pub text_wrap: TextWrap,  // 文本处理
}
```

### 改进建议

```rust
// 分离关注点
pub struct LayoutStyle {
    pub display: Display,
    pub position: Position,
    pub flex_direction: FlexDirection,
    pub padding: Edges,
    pub margin: Edges,
    pub width: Dimension,
    pub height: Dimension,
}

pub struct VisualStyle {
    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub bold: bool,
    pub border_style: BorderStyle,
}

pub struct ElementProps {
    pub layout: LayoutStyle,
    pub visual: VisualStyle,
    pub key: Option<String>,
    // is_static 移到这里或者组件层
}
```

---

## 9. 缺少错误边界

### 问题描述

React 有 Error Boundaries，但 RNK 没有：

```rust
// ❌ 当前：一个组件 panic 会导致整个应用崩溃
fn my_component() -> Element {
    panic!("oops!");  // 整个 app 挂掉
}
```

### 建议实现

```rust
pub fn error_boundary<F, E>(
    fallback: impl Fn(&Error) -> Element,
    child: F,
) -> Element
where
    F: Fn() -> Result<Element, E>,
    E: std::error::Error,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| child())) {
        Ok(Ok(element)) => element,
        Ok(Err(e)) => fallback(&e),
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            fallback(&Error::Panic(msg))
        }
    }
}

// 使用
error_boundary(
    |err| Text::new(format!("Error: {}", err)).into_element(),
    || risky_component(),
)
```

---

## 10. 异步支持不完整

### 问题描述

没有像 Bubbletea 的 `Cmd` 或 React 的 Suspense：

```rust
// ❌ 当前：需要手动管理异步状态
let data = use_signal(|| None);
let loading = use_signal(|| false);

use_effect_once(|| {
    loading.set(true);
    std::thread::spawn(move || {
        let result = fetch_data();
        // 如何安全地更新 UI？
        // 需要 request_render() + Arc/Mutex
    });
    None
});
```

### 建议实现

```rust
// Command 系统
pub enum Cmd {
    None,
    Batch(Vec<Cmd>),
    Async(Box<dyn Future<Output = Msg>>),
    Tick(Duration),
}

pub fn update(model: &mut Model, msg: Msg) -> Cmd {
    match msg {
        Msg::FetchData => {
            model.loading = true;
            Cmd::Async(Box::pin(async {
                let data = fetch_data().await;
                Msg::DataLoaded(data)
            }))
        }
        Msg::DataLoaded(data) => {
            model.data = Some(data);
            model.loading = false;
            Cmd::None
        }
    }
}
```

---

## 总结：问题优先级

| 优先级 | 问题 | 影响范围 | 工作量 |
|--------|------|---------|--------|
| **🔴 高** | 拆分 app.rs | 可维护性、可测试性 | 中 |
| **🔴 高** | 添加 Command 系统 | 异步/副作用管理 | 高 |
| **🟡 中** | 实现 Reconciliation | 性能、状态保留 | 高 |
| **🟡 中** | Hook 顺序验证 | 开发体验、错误预防 | 低 |
| **🟡 中** | 事件冒泡机制 | 功能完整性 | 中 |
| **🟢 低** | 布局缓存 | 性能优化 | 低 |
| **🟢 低** | 错误边界 | 健壮性 | 中 |
| **🟢 低** | 关注点分离 | 代码质量 | 低 |

---

## 下一步

请参阅 `architecture-redesign.md` 获取详细的重新设计方案。

---

## 参考资料

### 架构模式
- [Terminal UI Architecture Patterns](https://dev.to/charmbracelet/terminal-ui-design-patterns-2024)
- [MVC/MVVM in CLI Applications](https://stackoverflow.com/questions/console-app-architecture)

### 框架研究
- [Ink React Terminal UI](https://github.com/vadimdemedes/ink)
- [Bubbletea Elm Architecture](https://github.com/charmbracelet/bubbletea)
- [Ratatui Rust TUI](https://docs.rs/ratatui)
- [Dioxus Virtual DOM](https://dioxuslabs.com)

### 性能优化
- [TUI Performance Optimization](https://janouch.name/articles/tui-rendering)
- [Flicker-free Terminal Rendering](https://evilmartians.com/chronicles/smooth-terminal-animations)
