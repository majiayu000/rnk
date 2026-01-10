# Tink 框架对比分析

本文档对比 Tink 与两个主流终端 UI 框架：**Ink (JavaScript)** 和 **Bubbletea (Go)**。

## 概览

| 框架 | 语言 | 架构模式 | 布局引擎 | 成熟度 |
|------|------|----------|----------|--------|
| **Tink** | Rust | React/Hooks | Taffy (Flexbox) | 新项目 |
| **Ink** | JavaScript | React | Yoga (Flexbox) | 成熟 |
| **Bubbletea** | Go | Elm Architecture | 无 (手动) | 成熟 |

---

## Tink vs Ink (JavaScript)

### 组件对比

| 组件 | Tink | Ink | 状态 |
|------|------|-----|------|
| Box | `Box::new()` | `<Box>` | ✅ 一致 |
| Text | `Text::new()` | `<Text>` | ✅ 一致 |
| Newline | `Newline::new()` | `<Newline>` | ✅ 一致 |
| Spacer | `Spacer::new()` | `<Spacer>` | ✅ 一致 |
| Transform | `Transform::new()` | `<Transform>` | ✅ 一致 |
| Static | `Static::new()` | `<Static>` | ✅ 一致 |

### Hooks 对比

| Hook | Tink | Ink | 状态 |
|------|------|-----|------|
| useState | `use_signal` | `useState` | ✅ 一致 |
| useEffect | `use_effect` | `useEffect` | ✅ 一致 |
| useInput | `use_input` | `useInput` | ✅ 一致 |
| useApp | `use_app` | `useApp` | ✅ 一致 |
| useFocus | `use_focus` | `useFocus` | ✅ 一致 |
| useFocusManager | `use_focus_manager` | `useFocusManager` | ✅ 一致 |
| useStdin | `use_stdin` | `useStdin` | ✅ 一致 |
| useStdout | `use_stdout` | `useStdout` | ✅ 一致 |
| useStderr | `use_stderr` | `useStderr` | ✅ 一致 |
| useMeasure | `use_measure` | - | Tink 独有 |
| useIsScreenReaderEnabled | `use_is_screen_reader_enabled` | `useIsScreenReaderEnabled` | ✅ 一致 |

### 布局系统

| 特性 | Tink | Ink |
|------|------|-----|
| 布局引擎 | Taffy | Yoga |
| flex-direction | ✅ | ✅ |
| justify-content | ✅ | ✅ |
| align-items | ✅ | ✅ |
| position: absolute | ✅ | ✅ |
| top/right/bottom/left | ✅ | ❌ |
| max-width/height | ✅ | ❌ |
| display: none | ✅ | ✅ |

### 样式系统

| 特性 | Tink | Ink |
|------|------|-----|
| 16 色 | ✅ | ✅ |
| 256 色 | ✅ | ✅ |
| RGB/Hex | ✅ | ✅ |
| bold | ✅ | ✅ |
| italic | ✅ | ✅ |
| underline | ✅ | ✅ |
| strikethrough | ✅ | ✅ |
| dim | ✅ | ✅ |
| 边框样式 | 8 种内置 | cli-boxes |
| 边框四边颜色 | ✅ | ✅ |

### 完成度

```
组件系统:     ████████████████████ 100%
Hooks系统:    ████████████████████ 100%
布局系统:     ████████████████████ 100%+
样式系统:     ████████████████████ 100%
渲染系统:     ████████████████████ 100%
────────────────────────────────────────
总体完成度:   ████████████████████ 100%
```

---

## Tink vs Bubbletea (Go)

### 架构对比

| 维度 | Tink | Bubbletea |
|------|------|-----------|
| 设计范式 | React/Hooks | Elm Architecture |
| 状态管理 | Hooks (use_signal) | Model-Update-View |
| 组件模式 | 声明式 Builder | 命令式字符串 |
| 布局系统 | Taffy Flexbox | 无 (手动/LipGloss) |

### 组件对比

| 组件 | Tink | Bubbletea |
|------|------|-----------|
| 容器 | `Box::new()` | 无内置 |
| 文本 | `Text::new()` | 字符串 |
| 布局 | Flexbox 自动 | 手动计算 |
| 样式 | Style 结构体 | LipGloss 库 |

### 状态管理

| 特性 | Tink | Bubbletea |
|------|------|-----------|
| 模式 | 响应式 Hooks | 消息驱动 |
| 更新触发 | 自动 | 手动返回新 Model |
| 副作用 | use_effect | Cmd 函数 |
| 类型安全 | Rust 强类型 | interface{} |

### 输入处理

| 特性 | Tink | Bubbletea |
|------|------|-----------|
| 键盘 | 15+ 键 | 35+ 键 |
| 鼠标 | ❌ | ✅ 完整 |
| 粘贴检测 | ❌ | ✅ |
| 焦点事件 | ❌ | ✅ |

### 布局能力

| 特性 | Tink | Bubbletea |
|------|------|-----------|
| Flexbox | ✅ 完整 | ❌ |
| flex-direction | ✅ | ❌ |
| justify-content | ✅ | ❌ |
| align-items | ✅ | ❌ |
| position: absolute | ✅ | ❌ |
| 百分比尺寸 | ✅ | ❌ |

### 对比总结

```
┌─────────────────────────────────────────────────────────┐
│              TINK vs BUBBLETEA 功能矩阵                 │
├─────────────────────────────────────────────────────────┤
│  Flexbox布局:    TINK ████████████  BT ░░░░░░░░░░░░    │
│  内置组件:       TINK ████████████  BT ░░░░░░░░░░░░    │
│  状态管理:       TINK ████████████  BT ████████████    │
│  键盘输入:       TINK ████████░░░░  BT ████████████    │
│  鼠标支持:       TINK ░░░░░░░░░░░░  BT ████████████    │
│  生态成熟度:     TINK ████░░░░░░░░  BT ████████████    │
│  类型安全:       TINK ████████████  BT ████████░░░░    │
└─────────────────────────────────────────────────────────┘
```

---

## 选型建议

### 选择 Tink 当需要

- 复杂的 Flexbox 布局
- React/Hooks 开发体验
- Rust 类型安全和性能
- 精确的样式控制

### 选择 Ink 当需要

- JavaScript/TypeScript 生态
- React 组件复用
- 成熟的社区支持
- 快速原型开发

### 选择 Bubbletea 当需要

- Go 语言生态集成
- 函数式 Elm 架构
- 完整的鼠标支持
- 简单的命令行工具

---

## 功能路线图

### 已完成

- [x] 核心组件 (Box, Text, Newline, Spacer, Transform, Static)
- [x] Hooks 系统 (use_signal, use_effect, use_input, use_app)
- [x] 焦点管理 (use_focus, use_focus_manager)
- [x] I/O Hooks (use_stdin, use_stdout, use_stderr)
- [x] 元素测量 (use_measure, measure_element)
- [x] 无障碍支持 (use_is_screen_reader_enabled)
- [x] Flexbox 布局 (Taffy)
- [x] 绝对定位 (position: absolute)
- [x] 完整样式系统 (颜色, 文本样式, 边框)
- [x] 增量渲染

### 待开发

- [ ] 鼠标支持
- [ ] 更多特殊键 (F1-F20, Insert 等)
- [ ] 粘贴检测 (Bracketed paste)
- [ ] 焦点事件检测
- [ ] 高级组件库 (TextInput, List, Table 等)

---

## 代码示例对比

### 计数器应用

**Tink (Rust)**
```rust
fn counter() -> Element {
    let count = use_signal(|| 0i32);

    use_input(move |_, key| {
        if key.up_arrow { count.update(|c| *c += 1); }
        if key.down_arrow { count.update(|c| *c -= 1); }
    });

    Box::new()
        .padding(1)
        .border_style(BorderStyle::Round)
        .child(Text::new(format!("Count: {}", count.get())).bold())
        .into_element()
}
```

**Ink (JavaScript)**
```jsx
function Counter() {
    const [count, setCount] = useState(0);

    useInput((_, key) => {
        if (key.upArrow) setCount(c => c + 1);
        if (key.downArrow) setCount(c => c - 1);
    });

    return (
        <Box padding={1} borderStyle="round">
            <Text bold>Count: {count}</Text>
        </Box>
    );
}
```

**Bubbletea (Go)**
```go
type model int

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    if key, ok := msg.(tea.KeyMsg); ok {
        switch key.String() {
        case "up": m++
        case "down": m--
        }
    }
    return m, nil
}

func (m model) View() string {
    return fmt.Sprintf("Count: %d", m)
}
```

---

## 性能对比

| 指标 | Tink | Ink | Bubbletea |
|------|------|-----|-----------|
| 启动时间 | 快 (原生) | 中 (Node.js) | 快 (原生) |
| 内存占用 | 低 | 中 | 低 |
| 渲染性能 | 高 | 中 | 高 |
| 布局计算 | Taffy (快) | Yoga (快) | 无 |

---

## 结论

Tink 已实现与 Ink 的 **100% 功能对等**，并在某些方面超越：

1. **布局能力更强** - 支持 top/right/bottom/left 定位和 max-width/height
2. **Rust 类型安全** - 编译时错误检查
3. **更好的性能** - 原生编译，无运行时开销

相比 Bubbletea，Tink 提供了：

1. **完整的 Flexbox 布局** - 无需手动计算
2. **React 式开发体验** - Hooks 状态管理
3. **声明式组件** - Builder 模式

未来重点是补充**鼠标支持**和**高级组件库**。
