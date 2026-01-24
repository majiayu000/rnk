# RNK 架构重新设计方案

> 日期: 2026-01-24
> 架构师: 终端 UI 专家团队
> 版本: 1.0

## 前言：作为 UI 架构师的思考

作为设计过 Ink 和 Bubbletea 的架构师，我认为终端 UI 框架的核心挑战是：

1. **平衡声明式 vs 命令式**：声明式易用，命令式高效
2. **管理副作用**：异步操作、定时器、I/O
3. **性能 vs 开发体验**：Virtual DOM diff vs 简单重建
4. **类型安全 vs 灵活性**：Rust 的严格性 vs UI 的动态性

基于这些思考，我提出 **三个架构方案**，每个方案都有明确的权衡取舍。

---

## 方案对比总览

| 维度 | 方案 A: Pure Elm | 方案 B: Hybrid | 方案 C: React Fiber |
|------|-----------------|----------------|---------------------|
| **架构模式** | Elm Architecture | Elm + React Hooks | React Fiber-like |
| **学习曲线** | 🟢 低（简单） | 🟡 中 | 🔴 高（复杂） |
| **性能** | 🟢 优秀 | 🟡 良好 | 🟢 优秀 |
| **副作用管理** | 🟢 Command 系统 | 🟡 Hooks + Cmd | 🟡 Effects |
| **开发体验** | 🟡 函数式风格 | 🟢 熟悉（React-like） | 🟢 强大 |
| **实现复杂度** | 🟢 低 | 🟡 中 | 🔴 高 |
| **适用场景** | CLI 工具 | 通用 TUI | 复杂应用 |
| **推荐指数** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |

---

## 方案 A: Pure Elm Architecture（纯函数式）

### 设计理念

**"简单胜过复杂，显式胜过隐式"**

采用 Bubbletea 的 Elm Architecture，完全抛弃 Hooks，用 Model-Update-View 三元组构建应用。

### 核心架构

```rust
// ============================================================
// 1. Model（应用状态）
// ============================================================
pub trait Model: 'static {
    type Msg: Message;

    /// 初始化模型和命令
    fn init() -> (Self, Cmd<Self::Msg>);

    /// 更新模型，返回新状态和命令
    fn update(&mut self, msg: Self::Msg) -> Cmd<Self::Msg>;

    /// 渲染视图
    fn view(&self) -> Element;

    /// 可选：订阅外部事件
    fn subscriptions(&self) -> Sub<Self::Msg> {
        Sub::none()
    }
}

// ============================================================
// 2. Message（消息类型）
// ============================================================
pub trait Message: 'static + Send {}

// ============================================================
// 3. Command（副作用描述）
// ============================================================
pub enum Cmd<Msg> {
    None,
    Batch(Vec<Cmd<Msg>>),
    Perform(Task<Msg>),
}

pub enum Task<Msg> {
    /// 异步任务
    Async(Box<dyn Future<Output = Msg> + Send>),

    /// 定时器
    Tick {
        duration: Duration,
        msg: Msg,
    },

    /// HTTP 请求
    Http {
        request: Request,
        on_response: Box<dyn Fn(Response) -> Msg + Send>,
    },

    /// 读取文件
    ReadFile {
        path: PathBuf,
        on_read: Box<dyn Fn(io::Result<String>) -> Msg + Send>,
    },
}

// ============================================================
// 4. Subscription（持续事件源）
// ============================================================
pub enum Sub<Msg> {
    None,
    Batch(Vec<Sub<Msg>>),

    /// 键盘输入
    Keyboard(Box<dyn Fn(KeyEvent) -> Option<Msg> + Send>),

    /// 鼠标事件
    Mouse(Box<dyn Fn(MouseEvent) -> Option<Msg> + Send>),

    /// 时间间隔
    Every {
        duration: Duration,
        msg: Msg,
    },
}

// ============================================================
// 5. Runtime（运行时）
// ============================================================
pub struct Runtime<M: Model> {
    model: M,
    terminal: Terminal,
    layout_engine: LayoutEngine,
    cmd_executor: CmdExecutor<M::Msg>,
    msg_queue: mpsc::UnboundedReceiver<M::Msg>,
}

impl<M: Model> Runtime<M> {
    pub fn new() -> Self {
        let (model, init_cmd) = M::init();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut runtime = Self {
            model,
            terminal: Terminal::new(),
            layout_engine: LayoutEngine::new(),
            cmd_executor: CmdExecutor::new(tx.clone()),
            msg_queue: rx,
        };

        runtime.execute_cmd(init_cmd);
        runtime
    }

    pub fn run(mut self) -> io::Result<()> {
        self.terminal.enter()?;

        // 渲染初始界面
        self.render_frame()?;

        // 启动订阅
        let subs = self.model.subscriptions();
        self.start_subscriptions(subs);

        loop {
            // 处理消息队列
            while let Ok(msg) = self.msg_queue.try_recv() {
                let cmd = self.model.update(msg);
                self.execute_cmd(cmd);
                self.render_frame()?;
            }

            // 等待下一个消息
            if let Ok(msg) = self.msg_queue.recv_timeout(Duration::from_millis(16)) {
                let cmd = self.model.update(msg);
                self.execute_cmd(cmd);
                self.render_frame()?;
            }
        }

        self.terminal.exit()?;
        Ok(())
    }

    fn execute_cmd(&mut self, cmd: Cmd<M::Msg>) {
        self.cmd_executor.execute(cmd);
    }

    fn render_frame(&mut self) -> io::Result<()> {
        let element = self.model.view();
        let (width, height) = self.terminal.size()?;

        self.layout_engine.compute(&element, width, height);
        let output = render_to_buffer(&element, &self.layout_engine, width, height);

        self.terminal.render(&output)
    }
}
```

### 示例应用

```rust
// ============================================================
// 计数器应用
// ============================================================
struct Counter {
    count: i32,
    auto_increment: bool,
}

enum CounterMsg {
    Increment,
    Decrement,
    ToggleAuto,
    Tick,
    KeyPress(KeyEvent),
}

impl Message for CounterMsg {}

impl Model for Counter {
    type Msg = CounterMsg;

    fn init() -> (Self, Cmd<Self::Msg>) {
        (
            Self {
                count: 0,
                auto_increment: false,
            },
            Cmd::None,
        )
    }

    fn update(&mut self, msg: Self::Msg) -> Cmd<Self::Msg> {
        match msg {
            CounterMsg::Increment => {
                self.count += 1;
                Cmd::None
            }
            CounterMsg::Decrement => {
                self.count -= 1;
                Cmd::None
            }
            CounterMsg::ToggleAuto => {
                self.auto_increment = !self.auto_increment;
                Cmd::None
            }
            CounterMsg::Tick => {
                if self.auto_increment {
                    self.count += 1;
                }
                Cmd::None
            }
            CounterMsg::KeyPress(event) => {
                match event.code {
                    KeyCode::Char('q') => Cmd::exit(),
                    KeyCode::Up => {
                        self.count += 1;
                        Cmd::None
                    }
                    KeyCode::Down => {
                        self.count -= 1;
                        Cmd::None
                    }
                    KeyCode::Char(' ') => {
                        self.auto_increment = !self.auto_increment;
                        Cmd::None
                    }
                    _ => Cmd::None,
                }
            }
        }
    }

    fn view(&self) -> Element {
        Box::column()
            .padding(2.0)
            .gap(1.0)
            .child(
                Text::new(format!("Count: {}", self.count))
                    .bold(true)
                    .color(Color::Cyan)
                    .into_element()
            )
            .child(
                Text::new(if self.auto_increment {
                    "Auto: ON"
                } else {
                    "Auto: OFF"
                })
                .color(if self.auto_increment {
                    Color::Green
                } else {
                    Color::Red
                })
                .into_element()
            )
            .child(
                Text::new("Press ↑/↓ to change, Space to toggle auto, Q to quit")
                    .dim(true)
                    .into_element()
            )
            .into_element()
    }

    fn subscriptions(&self) -> Sub<Self::Msg> {
        Sub::batch(vec![
            Sub::keyboard(|event| Some(CounterMsg::KeyPress(event))),
            Sub::every(Duration::from_secs(1), CounterMsg::Tick),
        ])
    }
}

fn main() -> io::Result<()> {
    Runtime::<Counter>::new().run()
}
```

### 优势

1. ✅ **简单易懂**：Model-Update-View 清晰分离
2. ✅ **副作用可控**：所有副作用通过 Cmd 描述
3. ✅ **易于测试**：纯函数，可预测
4. ✅ **时间旅行调试**：可记录所有 Msg
5. ✅ **类型安全**：编译时保证

### 劣势

1. ❌ **学习曲线**：需要理解 Elm 概念
2. ❌ **样板代码多**：需要定义 Msg 枚举
3. ❌ **缺少局部状态**：所有状态在顶层 Model
4. ❌ **组件复用困难**：需要手动提升状态

---

## 方案 B: Hybrid Architecture（混合架构）⭐ 推荐

### 设计理念

**"两个世界的最佳组合"**

保留 Hooks 的便利性，引入 Command 系统管理副作用。这是 **最平衡** 的方案。

### 核心架构

```rust
// ============================================================
// 1. 保留组件 + Hooks
// ============================================================
pub fn component() -> Element {
    // Hooks 用于局部状态
    let count = use_signal(|| 0);
    let name = use_signal(|| String::new());

    // 使用 use_cmd 处理副作用
    use_cmd(count.get(), |count_val| {
        if count_val > 10 {
            Cmd::perform(async {
                notify("Count exceeded 10!").await;
            })
        } else {
            Cmd::none()
        }
    });

    Box::column()
        .child(Text::new(format!("Count: {}", count.get())).into_element())
        .into_element()
}

// ============================================================
// 2. Command System（副作用管理）
// ============================================================
pub enum Cmd {
    None,
    Batch(Vec<Cmd>),
    Perform(Box<dyn Future<Output = ()> + Send>),
    Tick {
        duration: Duration,
        callback: Box<dyn FnOnce() + Send>,
    },
    Http {
        request: Request,
        on_response: Box<dyn FnOnce(Response) + Send>,
    },
}

impl Cmd {
    pub fn none() -> Self {
        Cmd::None
    }

    pub fn batch(cmds: Vec<Cmd>) -> Self {
        Cmd::Batch(cmds)
    }

    pub fn perform<F>(future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Cmd::Perform(Box::new(future))
    }

    pub fn tick<F>(duration: Duration, callback: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Cmd::Tick {
            duration,
            callback: Box::new(callback),
        }
    }
}

// ============================================================
// 3. use_cmd Hook（副作用 Hook）
// ============================================================
pub fn use_cmd<D, F>(deps: D, f: F)
where
    D: Deps + 'static,
    F: FnOnce(D::Output) -> Cmd + 'static,
{
    with_hook_context(|ctx| {
        let hook = ctx.use_hook(move || {
            CmdHook {
                deps_hash: deps.hash(),
                last_cmd: None,
            }
        });

        let new_hash = deps.hash();
        if hook.deps_hash != new_hash {
            hook.deps_hash = new_hash;
            let cmd = f(deps.output());
            ctx.queue_cmd(cmd);
        }
    });
}

// ============================================================
// 4. CmdExecutor（命令执行器）
// ============================================================
pub struct CmdExecutor {
    runtime: tokio::runtime::Runtime,
    render_tx: mpsc::UnboundedSender<()>,
}

impl CmdExecutor {
    pub fn execute(&self, cmd: Cmd) {
        match cmd {
            Cmd::None => {}
            Cmd::Batch(cmds) => {
                for cmd in cmds {
                    self.execute(cmd);
                }
            }
            Cmd::Perform(future) => {
                let tx = self.render_tx.clone();
                self.runtime.spawn(async move {
                    future.await;
                    let _ = tx.send(());
                });
            }
            Cmd::Tick { duration, callback } => {
                let tx = self.render_tx.clone();
                self.runtime.spawn(async move {
                    tokio::time::sleep(duration).await;
                    callback();
                    let _ = tx.send(());
                });
            }
            Cmd::Http { request, on_response } => {
                let tx = self.render_tx.clone();
                self.runtime.spawn(async move {
                    let response = reqwest::get(request.url).await.unwrap();
                    on_response(response);
                    let _ = tx.send(());
                });
            }
        }
    }
}

// ============================================================
// 5. App 运行时（改进版）
// ============================================================
pub struct App<F: Fn() -> Element> {
    component: F,
    terminal: Terminal,
    layout_engine: LayoutEngine,
    hook_context: Rc<RefCell<HookContext>>,
    cmd_executor: CmdExecutor,
    render_rx: mpsc::UnboundedReceiver<()>,
}

impl<F: Fn() -> Element> App<F> {
    pub fn new(component: F) -> Self {
        let (render_tx, render_rx) = mpsc::unbounded_channel();

        Self {
            component,
            terminal: Terminal::new(),
            layout_engine: LayoutEngine::new(),
            hook_context: Rc::new(RefCell::new(HookContext::new(render_tx.clone()))),
            cmd_executor: CmdExecutor::new(render_tx),
            render_rx,
        }
    }

    pub fn run(mut self) -> io::Result<()> {
        self.terminal.enter()?;

        loop {
            // 渲染帧
            self.render_frame()?;

            // 等待事件或命令完成
            select! {
                _ = self.render_rx.recv() => {
                    // 命令完成，触发重新渲染
                }
                event = self.terminal.poll_event() => {
                    // 处理输入事件
                    self.handle_event(event?);
                }
            }
        }

        self.terminal.exit()
    }

    fn render_frame(&mut self) -> io::Result<()> {
        // 执行组件函数
        let element = with_hooks(self.hook_context.clone(), || {
            (self.component)()
        });

        // 执行排队的命令
        let cmds = self.hook_context.borrow_mut().take_cmds();
        for cmd in cmds {
            self.cmd_executor.execute(cmd);
        }

        // 布局和渲染
        let (width, height) = self.terminal.size()?;
        self.layout_engine.compute(&element, width, height);
        let output = render_to_buffer(&element, &self.layout_engine, width, height);

        self.terminal.render(&output)
    }
}
```

### 示例应用

```rust
// ============================================================
// 异步数据加载示例
// ============================================================
fn github_viewer() -> Element {
    let username = use_signal(|| String::from("octocat"));
    let repos = use_signal(|| Vec::new());
    let loading = use_signal(|| false);

    // 当 username 变化时，加载仓库列表
    use_cmd(username.get(), |name| {
        loading.set(true);

        Cmd::perform(async move {
            let url = format!("https://api.github.com/users/{}/repos", name);
            match reqwest::get(&url).await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<Vec<Repo>>().await {
                        repos.set(data);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
            loading.set(false);
        })
    });

    // 渲染 UI
    Box::column()
        .padding(2.0)
        .child(
            Text::new(format!("Repositories for {}", username.get()))
                .bold(true)
                .into_element()
        )
        .child(if loading.get() {
            Spinner::new().into_element()
        } else {
            repos_list(repos.get())
        })
        .into_element()
}

// ============================================================
// 定时器示例
// ============================================================
fn timer_app() -> Element {
    let seconds = use_signal(|| 0);
    let running = use_signal(|| false);

    // 每秒 tick
    use_cmd(running.get(), |is_running| {
        if is_running {
            Cmd::tick(Duration::from_secs(1), move || {
                seconds.update(|s| *s += 1);
            })
        } else {
            Cmd::none()
        }
    });

    Box::column()
        .child(Text::new(format!("Time: {}s", seconds.get())).into_element())
        .child(
            Box::row()
                .gap(1.0)
                .child(button("Start", || running.set(true)))
                .child(button("Stop", || running.set(false)))
                .child(button("Reset", || seconds.set(0)))
                .into_element()
        )
        .into_element()
}
```

### 优势

1. ✅ **熟悉的 API**：保留 Hooks，React 开发者易上手
2. ✅ **局部状态**：use_signal 管理组件状态
3. ✅ **副作用可控**：Cmd 系统统一管理
4. ✅ **渐进式**：简单场景用 Hooks，复杂场景用 Cmd
5. ✅ **异步友好**：内置 tokio 支持

### 劣势

1. ❌ **概念混合**：需要理解 Hooks + Cmd
2. ❌ **实现复杂**：两套系统需要集成

---

## 方案 C: React Fiber-like（高级方案）

### 设计理念

**"最大性能，最强能力"**

实现类似 React Fiber 的调度系统，支持并发渲染、优先级、Suspense。

### 核心架构

```rust
// ============================================================
// 1. Fiber 节点
// ============================================================
pub struct Fiber {
    pub id: FiberId,
    pub element_type: ElementType,
    pub props: Props,
    pub state: Option<Box<dyn Any>>,

    // Fiber 树结构
    pub parent: Option<FiberId>,
    pub child: Option<FiberId>,
    pub sibling: Option<FiberId>,

    // 工作标记
    pub alternate: Option<FiberId>,  // 上一次的 Fiber
    pub effect_tag: EffectTag,        // 需要执行的操作
    pub lanes: Lanes,                 // 优先级通道
}

pub enum EffectTag {
    NoEffect,
    Placement,  // 插入
    Update,     // 更新
    Deletion,   // 删除
}

// ============================================================
// 2. Lanes（优先级系统）
// ============================================================
pub struct Lanes(u32);

impl Lanes {
    pub const SYNC: Lanes = Lanes(0b0001);           // 同步（用户输入）
    pub const INPUT_CONTINUOUS: Lanes = Lanes(0b0010);  // 连续输入（拖拽）
    pub const DEFAULT: Lanes = Lanes(0b0100);        // 默认（网络请求）
    pub const TRANSITION: Lanes = Lanes(0b1000);     // 过渡（页面切换）
    pub const IDLE: Lanes = Lanes(0b10000);          // 空闲
}

// ============================================================
// 3. Work Loop（工作循环）
// ============================================================
pub struct WorkLoop {
    work_in_progress: Option<FiberId>,
    work_in_progress_root: Option<FiberId>,
    current_lanes: Lanes,
    fiber_store: HashMap<FiberId, Fiber>,
}

impl WorkLoop {
    pub fn schedule_update(&mut self, fiber_id: FiberId, lane: Lanes) {
        // 标记 fiber 需要更新
        self.mark_update_lane(fiber_id, lane);

        // 调度工作
        self.ensure_root_is_scheduled();
    }

    pub fn perform_work_until_deadline(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_millis(5);

        while let Some(fiber_id) = self.work_in_progress {
            if Instant::now() >= deadline {
                // 时间片用完，让出控制权
                return false;
            }

            // 执行工作单元
            let next = self.perform_unit_of_work(fiber_id);
            self.work_in_progress = next;
        }

        // 工作完成
        true
    }

    fn perform_unit_of_work(&mut self, fiber_id: FiberId) -> Option<FiberId> {
        let fiber = self.fiber_store.get(&fiber_id).unwrap();

        // 1. 开始工作
        self.begin_work(fiber);

        // 2. 处理子节点
        if let Some(child) = fiber.child {
            return Some(child);
        }

        // 3. 处理兄弟节点或返回父节点
        let mut current = Some(fiber_id);
        while let Some(id) = current {
            self.complete_work(id);

            let fiber = self.fiber_store.get(&id).unwrap();
            if let Some(sibling) = fiber.sibling {
                return Some(sibling);
            }
            current = fiber.parent;
        }

        None
    }

    fn begin_work(&mut self, fiber: &Fiber) {
        match fiber.element_type {
            ElementType::FunctionComponent => {
                // 调用组件函数
                self.update_function_component(fiber);
            }
            ElementType::Box => {
                // 更新普通元素
                self.update_host_component(fiber);
            }
            _ => {}
        }
    }
}

// ============================================================
// 4. Suspense（异步数据加载）
// ============================================================
pub struct Suspense {
    pub fallback: Element,
    pub children: Vec<Element>,
}

impl Suspense {
    pub fn new() -> Self {
        Self {
            fallback: Text::new("Loading...").into_element(),
            children: Vec::new(),
        }
    }

    pub fn fallback(mut self, element: Element) -> Self {
        self.fallback = element;
        self
    }

    pub fn child(mut self, element: Element) -> Self {
        self.children.push(element);
        self
    }
}

// Resource（可挂起的资源）
pub struct Resource<T> {
    state: Arc<Mutex<ResourceState<T>>>,
}

enum ResourceState<T> {
    Pending(Receiver<T>),
    Ready(T),
    Error(String),
}

impl<T: Clone + Send + 'static> Resource<T> {
    pub fn read(&self) -> T {
        let state = self.state.lock().unwrap();
        match &*state {
            ResourceState::Ready(data) => data.clone(),
            ResourceState::Pending(_) => {
                // 抛出 Suspense 信号
                panic!("Suspend");
            }
            ResourceState::Error(e) => {
                panic!("Resource error: {}", e);
            }
        }
    }
}

// ============================================================
// 5. 使用示例
// ============================================================
fn user_profile() -> Element {
    let user = use_resource(|| fetch_user("octocat"));

    Suspense::new()
        .fallback(Spinner::new().into_element())
        .child({
            let data = user.read();  // 可能挂起

            Box::column()
                .child(Text::new(format!("Name: {}", data.name)).into_element())
                .child(Text::new(format!("Bio: {}", data.bio)).into_element())
                .into_element()
        })
        .into_element()
}
```

### 优势

1. ✅ **并发渲染**：长任务不阻塞 UI
2. ✅ **优先级调度**：用户输入优先响应
3. ✅ **Suspense**：优雅的异步加载
4. ✅ **时间切片**：避免卡顿
5. ✅ **Reconciliation**：高效 diff

### 劣势

1. ❌ **极高复杂度**：需要实现完整的 Fiber 架构
2. ❌ **调试困难**：异步调度难以追踪
3. ❌ **学习成本高**：概念复杂
4. ❌ **开发周期长**：至少 6 个月

---

## 详细对比：方案选择指南

### 场景 1：简单 CLI 工具（如 git clone 进度条）

| 方案 | 评分 | 理由 |
|------|------|------|
| A: Pure Elm | ⭐⭐⭐⭐⭐ | 简单直接，无需复杂状态管理 |
| B: Hybrid | ⭐⭐⭐ | 过度设计，Hooks 用不上 |
| C: Fiber | ⭐ | 严重过度设计 |

**推荐**：方案 A

---

### 场景 2：中等复杂度 TUI（如 Kubernetes Dashboard）

| 方案 | 评分 | 理由 |
|------|------|------|
| A: Pure Elm | ⭐⭐⭐ | 状态提升繁琐 |
| B: Hybrid | ⭐⭐⭐⭐⭐ | 平衡最好，局部状态 + Cmd |
| C: Fiber | ⭐⭐⭐ | 性能收益不明显，复杂度高 |

**推荐**：方案 B

---

### 场景 3：复杂应用（如终端版 Figma）

| 方案 | 评分 | 理由 |
|------|------|------|
| A: Pure Elm | ⭐⭐ | 大量状态管理困难 |
| B: Hybrid | ⭐⭐⭐⭐ | 可行但可能遇到性能瓶颈 |
| C: Fiber | ⭐⭐⭐⭐⭐ | 高性能需求，值得投入 |

**推荐**：方案 C

---

## 最终推荐：方案 B（Hybrid）

### 理由

1. **平衡性最好**：既有 Hooks 的便利，又有 Cmd 的控制力
2. **渐进式采用**：可以从简单的 Hooks 开始，逐步引入 Cmd
3. **实现成本合理**：约 2-3 周可完成核心功能
4. **适用范围广**：80% 的 TUI 应用都适合
5. **开发体验好**：熟悉的 API，容易上手

### 实施路线图

#### Phase 1: Command 系统（1 周）

- [ ] 实现 `Cmd` 枚举和基础 API
- [ ] 实现 `CmdExecutor` 和 tokio 集成
- [ ] 实现 `use_cmd` hook
- [ ] 编写测试和文档

#### Phase 2: 重构 App Runtime（1 周）

- [ ] 拆分 `app.rs` 为多个模块
- [ ] 集成 CmdExecutor
- [ ] 实现 render 请求队列
- [ ] 移除全局状态

#### Phase 3: Hook 系统改进（3 天）

- [ ] 添加 hook 顺序验证（debug mode）
- [ ] 改进错误消息
- [ ] 优化性能

#### Phase 4: 示例和文档（3 天）

- [ ] 迁移现有示例
- [ ] 编写最佳实践指南
- [ ] 创建教程

---

## 架构细节：方案 B 深入设计

### 1. 模块划分

```
src/
├── core/
│   ├── element.rs        # Element 定义
│   ├── style.rs          # Style 系统
│   ├── color.rs          # Color 类型
│   └── props.rs          # 新增：统一属性系统
│
├── cmd/                  # 新增：Command 系统
│   ├── mod.rs            # Cmd 枚举和 API
│   ├── executor.rs       # CmdExecutor
│   ├── tasks.rs          # 预定义任务（HTTP、Timer）
│   └── subscription.rs   # 订阅系统（未来）
│
├── hooks/
│   ├── context.rs        # Hook 上下文
│   ├── use_signal.rs     # 信号 Hook
│   ├── use_effect.rs     # 副作用 Hook
│   ├── use_cmd.rs        # 新增：命令 Hook
│   ├── use_input.rs      # 输入 Hook
│   └── validation.rs     # 新增：Hook 验证
│
├── layout/
│   ├── engine.rs         # 布局引擎
│   ├── measure.rs        # 文本测量
│   └── cache.rs          # 新增：布局缓存
│
├── renderer/
│   ├── app.rs            # App 主结构（精简）
│   ├── runtime.rs        # 新增：运行时（事件循环）
│   ├── scheduler.rs      # 新增：渲染调度器
│   ├── terminal.rs       # 终端抽象
│   ├── output.rs         # 输出缓冲
│   └── diff.rs           # 新增：增量渲染（可选）
│
├── events/               # 新增：事件系统
│   ├── mod.rs            # 事件类型
│   ├── dispatcher.rs     # 事件分发
│   └── bubble.rs         # 事件冒泡（可选）
│
├── components/
│   └── ...               # 现有组件
│
└── testing/
    └── ...               # 测试工具
```

### 2. 核心类型定义

```rust
// ============================================================
// cmd/mod.rs
// ============================================================
pub enum Cmd {
    None,
    Batch(Vec<Cmd>),
    Perform {
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    },
    Sleep {
        duration: Duration,
        then: Box<Cmd>,
    },
}

impl Cmd {
    pub fn none() -> Self {
        Cmd::None
    }

    pub fn batch(cmds: impl IntoIterator<Item = Cmd>) -> Self {
        let cmds: Vec<_> = cmds.into_iter().collect();
        if cmds.is_empty() {
            Cmd::None
        } else if cmds.len() == 1 {
            cmds.into_iter().next().unwrap()
        } else {
            Cmd::Batch(cmds)
        }
    }

    pub fn perform<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Cmd::Perform {
            future: Box::pin(async move { f().await }),
        }
    }

    pub fn sleep(duration: Duration) -> Self {
        Cmd::Sleep {
            duration,
            then: Box::new(Cmd::None),
        }
    }

    pub fn and_then(self, next: Cmd) -> Self {
        match self {
            Cmd::Sleep { duration, then } => Cmd::Sleep {
                duration,
                then: Box::new(then.and_then(next)),
            },
            other => Cmd::batch(vec![other, next]),
        }
    }
}

// ============================================================
// hooks/use_cmd.rs
// ============================================================
pub fn use_cmd<D, F>(deps: D, f: F)
where
    D: Deps + 'static,
    F: FnOnce(D::Output) -> Cmd + 'static,
{
    use_effect(deps, move |output| {
        let cmd = f(output);
        queue_cmd(cmd);
        None  // 无需 cleanup
    });
}

fn queue_cmd(cmd: Cmd) {
    with_hook_context(|ctx| {
        ctx.cmd_queue.push(cmd);
    });
}

// ============================================================
// renderer/scheduler.rs
// ============================================================
pub struct RenderScheduler {
    render_requested: Arc<AtomicBool>,
    fps: u32,
    last_render: Instant,
}

impl RenderScheduler {
    pub fn new(fps: u32) -> Self {
        Self {
            render_requested: Arc::new(AtomicBool::new(false)),
            fps,
            last_render: Instant::now(),
        }
    }

    pub fn request_render(&self) {
        self.render_requested.store(true, Ordering::Relaxed);
    }

    pub fn should_render(&mut self) -> bool {
        let requested = self.render_requested.load(Ordering::Relaxed);
        if !requested {
            return false;
        }

        let frame_duration = Duration::from_millis(1000 / self.fps as u64);
        let elapsed = self.last_render.elapsed();

        if elapsed >= frame_duration {
            self.render_requested.store(false, Ordering::Relaxed);
            self.last_render = Instant::now();
            true
        } else {
            false
        }
    }
}
```

### 3. 错误处理策略

```rust
// ============================================================
// 错误边界
// ============================================================
pub fn error_boundary<F>(
    fallback: impl Fn(&Error) -> Element + 'static,
    child: F,
) -> Element
where
    F: Fn() -> Element + 'static,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| child())) {
        Ok(element) => element,
        Err(err) => {
            let error = if let Some(s) = err.downcast_ref::<&str>() {
                Error::Panic(s.to_string())
            } else if let Some(s) = err.downcast_ref::<String>() {
                Error::Panic(s.clone())
            } else {
                Error::Panic("Unknown panic".to_string())
            };
            fallback(&error)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Panic(String),
    Render(String),
    Layout(String),
}

// 使用
fn app() -> Element {
    error_boundary(
        |err| {
            Box::column()
                .padding(2.0)
                .child(
                    Text::new(format!("Error: {}", err))
                        .color(Color::Red)
                        .into_element()
                )
                .into_element()
        },
        || {
            // 可能出错的组件
            risky_component()
        },
    )
}
```

---

## 性能优化策略

### 1. 布局缓存

```rust
use lru::LruCache;

pub struct LayoutCache {
    cache: LruCache<LayoutKey, Layout>,
}

#[derive(Hash, Eq, PartialEq)]
struct LayoutKey {
    element_id: ElementId,
    width: u16,
    height: u16,
    // 包含影响布局的属性
    style_hash: u64,
}

impl LayoutEngine {
    pub fn compute_cached(&mut self, element: &Element, width: u16, height: u16) {
        let key = LayoutKey {
            element_id: element.id,
            width,
            height,
            style_hash: calculate_hash(&element.style),
        };

        if let Some(layout) = self.cache.get(&key) {
            // 使用缓存
            return;
        }

        // 计算布局
        self.compute(element, width, height);
        let layout = self.get_layout(element.id).unwrap();
        self.cache.put(key, layout);
    }
}
```

### 2. 增量渲染（可选）

```rust
pub struct DiffRenderer {
    prev_tree: Option<Element>,
}

impl DiffRenderer {
    pub fn render(&mut self, current: &Element) -> Vec<Patch> {
        if let Some(prev) = &self.prev_tree {
            diff_trees(prev, current)
        } else {
            // 首次渲染，全量
            vec![Patch::ReplaceAll(current.clone())]
        }
    }
}

pub enum Patch {
    ReplaceAll(Element),
    UpdateNode { id: ElementId, new_style: Style },
    InsertChild { parent: ElementId, child: Element },
    RemoveChild { parent: ElementId, index: usize },
}
```

---

## 迁移指南：从当前架构到方案 B

### Step 1: 添加 Command 系统（无破坏性）

```rust
// 新增 src/cmd/mod.rs
// 不影响现有代码
```

### Step 2: 添加 use_cmd Hook

```rust
// 新增 src/hooks/use_cmd.rs
// 现有 hooks 继续工作
```

### Step 3: 重构 App（分阶段）

```rust
// 先拆分，后重构
// 1. 提取 CmdExecutor
// 2. 提取 RenderScheduler
// 3. 提取 EventDispatcher
// 4. 清理 app.rs
```

### Step 4: 迁移示例

```rust
// 逐个迁移示例到新 API
// 保留旧 API 作为 deprecated
```

---

## 总结

作为一个设计过多个 UI 框架的架构师，我的建议是：

1. **短期（1-2 个月）**：实施方案 B，获得立竿见影的改进
2. **中期（6 个月）**：收集反馈，优化 API
3. **长期（1 年+）**：根据需求考虑是否需要方案 C

**核心原则**：
- 简单优于复杂
- 显式优于隐式
- 实用优于理论

**不要过早优化**：
- 先解决架构问题（拆分、Command）
- 再考虑性能问题（缓存、diff）
- 最后考虑高级特性（Fiber、Suspense）

---

## 参考资料

### 架构设计
- [Elm Architecture Guide](https://guide.elm-lang.org/architecture/)
- [React Fiber Architecture](https://github.com/acdlite/react-fiber-architecture)
- [Bubbletea Tutorial](https://github.com/charmbracelet/bubbletea/tree/master/tutorials)

### Rust 实现
- [Dioxus Virtual DOM](https://dioxuslabs.com/learn/0.5/reference/virtual_dom)
- [Ratatui Immediate Mode](https://ratatui.rs/concepts/rendering/)
- [Tokio Async Runtime](https://tokio.rs/tokio/tutorial)

### 性能优化
- [LRU Cache in Rust](https://docs.rs/lru/latest/lru/)
- [Terminal Performance](https://janouch.name/articles/tui-rendering)
- [React Scheduling](https://17.reactjs.org/docs/design-principles.html#scheduling)
