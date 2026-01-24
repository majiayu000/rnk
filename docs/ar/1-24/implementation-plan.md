# RNK 架构重构实施计划

> 日期: 2026-01-24
> 目标: 实施 Hybrid Architecture (方案 B)
> 预计工期: 2-3 周
> 当前状态: 计划中

## 总览

```
Phase 1: Command 系统基础    [1 周]  ████████░░ 0%
Phase 2: App Runtime 重构    [1 周]  ░░░░░░░░░░ 0%
Phase 3: Hook 系统改进       [3 天]  ░░░░░░░░░░ 0%
Phase 4: 文档和示例          [3 天]  ░░░░░░░░░░ 0%
```

---

## Phase 1: Command 系统基础 (1 周)

### 目标

实现统一的副作用管理系统，支持异步任务、定时器、HTTP 请求等。

### 任务清单

#### 1.1 Core Command Types (2 天)

**文件**: `src/cmd/mod.rs`

- [ ] 定义 `Cmd` 枚举
  - `None`: 空命令
  - `Batch`: 批量命令
  - `Perform`: 异步任务
  - `Sleep`: 延时
- [ ] 实现 `Cmd` 构造方法
  - `none()`, `batch()`, `perform()`, `sleep()`
  - `and_then()`: 命令组合
- [ ] 编写单元测试
  - 测试命令构造
  - 测试命令组合
  - 测试边界情况

**测试覆盖率目标**: 95%+

**验收标准**:
```rust
// 能够这样使用
let cmd = Cmd::batch(vec![
    Cmd::sleep(Duration::from_secs(1)),
    Cmd::perform(async { println!("Done!") }),
]);
```

---

#### 1.2 Command Executor (2 天)

**文件**: `src/cmd/executor.rs`

- [ ] 实现 `CmdExecutor` 结构
  - 集成 tokio runtime
  - 管理异步任务生命周期
  - 支持任务取消
- [ ] 实现任务调度
  - 并发执行多个命令
  - 任务完成通知
  - 错误处理
- [ ] 编写集成测试
  - 测试异步任务执行
  - 测试批量命令
  - 测试取消机制
  - 测试错误恢复

**测试覆盖率目标**: 90%+

**验收标准**:
```rust
let executor = CmdExecutor::new(render_tx);
executor.execute(Cmd::perform(async {
    tokio::time::sleep(Duration::from_secs(1)).await;
}));
// 1 秒后触发 render
```

---

#### 1.3 use_cmd Hook (1 天)

**文件**: `src/hooks/use_cmd.rs`

- [ ] 实现 `use_cmd` hook
  - 依赖追踪
  - 命令队列
  - 清理机制
- [ ] 集成到 HookContext
  - 添加 `cmd_queue` 字段
  - 实现 `take_cmds()` 方法
- [ ] 编写测试
  - 测试依赖变化触发命令
  - 测试命令队列
  - 测试多次调用

**测试覆盖率目标**: 95%+

**验收标准**:
```rust
fn component() -> Element {
    let count = use_signal(|| 0);

    use_cmd(count.get(), |val| {
        if val > 5 {
            Cmd::perform(async { notify() })
        } else {
            Cmd::none()
        }
    });
    // ...
}
```

---

#### 1.4 预定义任务 (1 天)

**文件**: `src/cmd/tasks.rs`

- [ ] 实现常用任务辅助函数
  - `Cmd::http()`: HTTP 请求
  - `Cmd::read_file()`: 文件读取
  - `Cmd::write_file()`: 文件写入
  - `Cmd::spawn()`: 进程启动
- [ ] 编写示例
- [ ] 编写测试

**测试覆盖率目标**: 85%+

---

#### 1.5 Phase 1 集成测试

**文件**: `tests/cmd_integration_test.rs`

- [ ] 端到端测试场景
  - 异步数据加载
  - 定时器更新
  - 命令组合
  - 错误处理
- [ ] 性能基准测试
  - 命令执行延迟
  - 内存占用
  - 并发能力

**测试场景**:
1. GitHub API 请求模拟
2. 倒计时定时器
3. 文件监控
4. 多任务并发

---

## Phase 2: App Runtime 重构 (1 周)

### 目标

拆分庞大的 `app.rs`，移除全局状态，集成 Command 系统。

### 任务清单

#### 2.1 模块拆分 (2 天)

**重构 `src/renderer/app.rs` (1635 行 → ~300 行)**

拆分为：

**文件**: `src/renderer/runtime.rs`
- [ ] 提取事件循环逻辑
- [ ] 实现 `AppRuntime` 结构
- [ ] 集成 CmdExecutor
- [ ] 测试事件处理

**文件**: `src/renderer/scheduler.rs`
- [ ] 提取渲染调度逻辑
- [ ] 实现 `RenderScheduler`
- [ ] FPS 限流
- [ ] 测试调度策略

**文件**: `src/renderer/registry.rs`
- [ ] 提取全局注册表
- [ ] 清理全局状态
- [ ] 改为显式传递
- [ ] 测试多实例

**文件**: `src/renderer/static_content.rs`
- [ ] 提取 Static 内容处理
- [ ] 实现 `StaticRenderer`
- [ ] 测试持久化输出

**文件**: `src/renderer/mode_switch.rs`
- [ ] 提取模式切换逻辑
- [ ] 实现 inline ↔ fullscreen 切换
- [ ] 测试模式转换

**测试覆盖率目标**: 每个模块 85%+

---

#### 2.2 重构 App 主结构 (2 天)

**文件**: `src/renderer/app.rs` (新)

- [ ] 精简 App 结构
  - 移除冗余字段
  - 使用组合替代继承
- [ ] 集成新模块
  - AppRuntime
  - RenderScheduler
  - CmdExecutor
- [ ] 重写 `run()` 方法
  - 清晰的事件循环
  - 命令执行集成
  - 错误处理
- [ ] 编写集成测试
  - 测试完整渲染流程
  - 测试命令执行
  - 测试事件处理

**验收标准**:
```rust
let app = App::new(|| my_component())
    .fps(60)
    .alternate_screen(true);

app.run()?;
```

---

#### 2.3 移除全局状态 (1 天)

**改进以下全局变量**:

- [ ] `APP_REGISTRY` → 显式传递
- [ ] `CURRENT_APP` → Context 参数
- [ ] `INPUT_HANDLERS` → EventDispatcher
- [ ] `MOUSE_HANDLERS` → EventDispatcher
- [ ] `APP_CONTEXT` → 参数传递

**文件**: `src/events/dispatcher.rs` (新)
- [ ] 实现 `EventDispatcher`
- [ ] 管理事件处理器
- [ ] 支持作用域
- [ ] 测试事件分发

**测试覆盖率目标**: 90%+

---

#### 2.4 Phase 2 集成测试

**文件**: `tests/app_integration_test.rs`

- [ ] 完整应用测试
  - 组件渲染
  - 命令执行
  - 事件处理
  - 模式切换
- [ ] 多实例测试
  - 同时运行多个 App
  - 隔离性验证
- [ ] 性能测试
  - 渲染帧率
  - 内存占用
  - 命令执行延迟

**测试场景**:
1. 完整的计数器应用
2. 异步数据加载应用
3. 交互式表单应用
4. 并发多 App 实例

---

## Phase 3: Hook 系统改进 (3 天)

### 目标

添加 Hook 调用顺序验证，改进错误消息，优化性能。

### 任务清单

#### 3.1 Hook 顺序验证 (1 天)

**文件**: `src/hooks/validation.rs`

- [ ] 开发模式下记录 Hook 调用
  - 记录每次渲染的 Hook 数量
  - 记录 Hook 类型序列
  - 检测不一致
- [ ] 实现错误报告
  - 清晰的错误消息
  - 指出错误位置
  - 建议修复方法
- [ ] 集成到 HookContext
- [ ] 编写测试
  - 测试正常情况
  - 测试条件调用检测
  - 测试循环调用检测

**测试覆盖率目标**: 95%+

**验收标准**:
```rust
// 这段代码会在 debug mode 下 panic，并给出清晰错误
fn bad_component(show: bool) -> Element {
    if show {
        let x = use_signal(|| 0);  // ❌ 条件调用
    }
    // Error: Hook call count mismatch!
    // Expected: 1, Got: 0
    // Hint: Hooks must be called unconditionally
}
```

---

#### 3.2 错误边界 (1 天)

**文件**: `src/components/error_boundary.rs`

- [ ] 实现 `error_boundary` 函数
  - catch_unwind 集成
  - 错误信息提取
  - Fallback UI
- [ ] 实现 `Error` 类型
  - Panic 错误
  - Render 错误
  - Layout 错误
- [ ] 编写测试
  - 测试 panic 捕获
  - 测试 fallback 渲染
  - 测试嵌套边界

**测试覆盖率目标**: 90%+

**验收标准**:
```rust
error_boundary(
    |err| Text::new(format!("Error: {}", err)).color(Color::Red),
    || risky_component(),
)
```

---

#### 3.3 Hook 性能优化 (1 天)

**文件**: `src/hooks/context.rs`

- [ ] 优化 HookStorage
  - 使用 `SmallVec` 减少分配
  - 预分配常见大小
- [ ] 优化依赖比较
  - 缓存 hash 值
  - 快速路径优化
- [ ] 性能基准测试
  - Hook 创建开销
  - 依赖检查开销
  - 内存占用

**性能目标**:
- Hook 创建 < 100ns
- 依赖检查 < 50ns
- 内存开销 < 100 bytes/hook

---

#### 3.4 Phase 3 测试

**文件**: `tests/hooks_validation_test.rs`

- [ ] 验证测试
  - 条件调用检测
  - 循环调用检测
  - 错误消息质量
- [ ] 错误边界测试
  - Panic 恢复
  - 嵌套边界
  - 多次错误
- [ ] 性能测试
  - 大量 Hook 场景
  - 复杂依赖图

---

## Phase 4: 文档和示例 (3 天)

### 目标

完整的文档、迁移指南、示例代码。

### 任务清单

#### 4.1 API 文档 (1 天)

**文件**: `docs/ar/1-24/api-reference.md`

- [ ] Command 系统 API
  - Cmd 枚举
  - CmdExecutor
  - 预定义任务
- [ ] Hook API
  - use_cmd
  - use_signal (更新)
  - use_effect (更新)
- [ ] App API
  - App 构建器
  - 配置选项
  - 运行方法

**文件**: 各模块的 rustdoc
- [ ] 为所有公共 API 添加文档注释
- [ ] 添加示例代码
- [ ] 添加链接引用

---

#### 4.2 架构文档 (1 天)

**文件**: `docs/ar/1-24/architecture.md`

- [ ] 整体架构图
- [ ] 模块依赖关系
- [ ] 数据流向
- [ ] 事件处理流程
- [ ] 命令执行流程
- [ ] 设计决策记录

**文件**: `docs/ar/1-24/migration-guide.md`

- [ ] 从旧版本迁移
  - 破坏性变更列表
  - 迁移步骤
  - 常见问题
- [ ] 最佳实践
  - 何时使用 Cmd
  - 何时使用 Hooks
  - 性能优化建议

---

#### 4.3 示例代码 (1 天)

**迁移现有示例**:

- [ ] `examples/counter.rs` → 使用 use_cmd
- [ ] `examples/async_data.rs` (新) → HTTP 请求
- [ ] `examples/timer.rs` (新) → 定时器
- [ ] `examples/multi_task.rs` (新) → 并发任务
- [ ] `examples/error_handling.rs` (新) → 错误边界

**每个示例要求**:
- 完整的代码
- 详细的注释
- README 说明
- 运行说明

---

#### 4.4 教程 (1 天)

**文件**: `docs/ar/1-24/tutorial.md`

- [ ] 第一章: Hello World
- [ ] 第二章: 状态管理
- [ ] 第三章: 副作用和命令
- [ ] 第四章: 异步数据加载
- [ ] 第五章: 事件处理
- [ ] 第六章: 错误处理
- [ ] 第七章: 性能优化

---

## 测试策略

### 测试金字塔

```
         E2E Tests (5%)
        ┌─────────────┐
       │  集成测试  │
      │  (20%)    │
     │           │
    │  单元测试  │
   │   (75%)   │
  └───────────┘
```

### 测试覆盖率目标

| 模块 | 单元测试 | 集成测试 | 总覆盖率 |
|------|---------|---------|---------|
| cmd/ | 95% | 90% | 92%+ |
| hooks/ | 95% | 85% | 90%+ |
| renderer/ | 85% | 90% | 87%+ |
| components/ | 90% | 80% | 85%+ |
| layout/ | 85% | 75% | 80%+ |
| **总体** | **90%** | **85%** | **87%+** |

### 测试工具

- **单元测试**: `cargo test --lib`
- **集成测试**: `cargo test --test '*'`
- **覆盖率**: `cargo tarpaulin --out Html`
- **基准测试**: `cargo bench`
- **属性测试**: `proptest`

---

## 质量检查清单

每个 Phase 完成后必须通过：

### 代码质量

- [ ] `cargo fmt` - 格式化检查
- [ ] `cargo clippy -- -D warnings` - Linting
- [ ] `cargo test --lib` - 单元测试通过
- [ ] `cargo test --all-targets` - 所有测试通过
- [ ] `cargo doc --no-deps` - 文档生成成功

### 测试覆盖率

- [ ] 单元测试覆盖率 ≥ 90%
- [ ] 集成测试覆盖率 ≥ 85%
- [ ] 关键路径 100% 覆盖

### 性能基准

- [ ] 渲染性能未退化
- [ ] 内存占用未增加 > 10%
- [ ] 命令执行延迟 < 1ms

### 文档

- [ ] 所有公共 API 有文档
- [ ] 至少 2 个使用示例
- [ ] 架构文档更新

---

## 风险和缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 破坏现有 API | 高 | 高 | 保持向后兼容，提供迁移工具 |
| 性能退化 | 中 | 高 | 持续基准测试，优化热路径 |
| 测试不足 | 中 | 高 | 严格的覆盖率要求 |
| 文档滞后 | 高 | 中 | 代码和文档同步开发 |
| Tokio 集成问题 | 低 | 高 | 早期原型验证 |

---

## 验收标准

### Phase 1 完成标准

- [ ] Command 系统完整实现
- [ ] 所有测试通过，覆盖率 ≥ 90%
- [ ] 至少 2 个工作示例
- [ ] API 文档完整

### Phase 2 完成标准

- [ ] app.rs 拆分完成，< 400 行
- [ ] 全局状态移除
- [ ] 所有测试通过，覆盖率 ≥ 85%
- [ ] 现有示例迁移成功
- [ ] 性能无退化

### Phase 3 完成标准

- [ ] Hook 验证工作正常
- [ ] 错误边界可用
- [ ] 所有测试通过，覆盖率 ≥ 90%
- [ ] 性能达标

### Phase 4 完成标准

- [ ] 完整的 API 文档
- [ ] 架构文档完整
- [ ] 至少 5 个示例
- [ ] 迁移指南完整
- [ ] 教程可用

### 总体完成标准

- [ ] 所有 Phase 通过验收
- [ ] 总体测试覆盖率 ≥ 87%
- [ ] 所有示例运行正常
- [ ] 文档完整且准确
- [ ] 性能基准达标
- [ ] 代码审查通过

---

## 进度追踪

使用 GitHub Issues/Projects 追踪：

- 每个任务创建一个 Issue
- 使用 Labels 标记优先级
- 使用 Milestones 标记 Phase
- 每日更新进度

---

## 下一步

1. **立即开始**: Phase 1.1 - Core Command Types
2. **创建分支**: `feature/hybrid-architecture`
3. **设置 CI**: 自动运行测试和覆盖率
4. **每日同步**: 更新进度文档

---

## 附录

### A. 文件清单

新增文件：
```
src/
├── cmd/
│   ├── mod.rs
│   ├── executor.rs
│   └── tasks.rs
├── hooks/
│   ├── use_cmd.rs
│   └── validation.rs
├── renderer/
│   ├── runtime.rs
│   ├── scheduler.rs
│   ├── registry.rs
│   ├── static_content.rs
│   └── mode_switch.rs
├── events/
│   └── dispatcher.rs
└── components/
    └── error_boundary.rs

tests/
├── cmd_integration_test.rs
├── app_integration_test.rs
└── hooks_validation_test.rs

examples/
├── async_data.rs
├── timer.rs
├── multi_task.rs
└── error_handling.rs

docs/ar/1-24/
├── api-reference.md
├── architecture.md
├── migration-guide.md
└── tutorial.md
```

### B. 依赖项

新增 Cargo.toml 依赖：
```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
futures = "0.3"
async-trait = "0.1"

[dev-dependencies]
criterion = "0.5"
proptest = "1.5"
tarpaulin = "0.31"
```

### C. 参考资料

- [Tokio 异步运行时](https://tokio.rs)
- [Elm Command 系统](https://guide.elm-lang.org/effects/)
- [React Error Boundaries](https://react.dev/reference/react/Component#catching-rendering-errors-with-an-error-boundary)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)
