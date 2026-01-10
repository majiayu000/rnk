# Tink Quality Assurance Architecture

> 目标：将 Tink 打造成 Rust 届的 Ink，保证零 bug 的高质量终端 UI 框架

## 核心理念

参考 Ink 和 Bubble Tea 的最佳实践，结合 Rust 的强类型系统，设计一套多层次的质量保证体系：

```
┌─────────────────────────────────────────────────────────────────┐
│                    Quality Assurance Pyramid                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌───────────────────────────────────────────────────────┐     │
│   │              Visual Regression Tests                   │     │
│   │         (Golden file / Snapshot testing)               │     │
│   └───────────────────────────────────────────────────────┘     │
│                           ▲                                      │
│   ┌───────────────────────────────────────────────────────┐     │
│   │              Integration Tests                         │     │
│   │    (Full render pipeline, interactive scenarios)       │     │
│   └───────────────────────────────────────────────────────┘     │
│                           ▲                                      │
│   ┌───────────────────────────────────────────────────────┐     │
│   │              Property-Based Tests                      │     │
│   │        (QuickCheck / Proptest for edge cases)          │     │
│   └───────────────────────────────────────────────────────┘     │
│                           ▲                                      │
│   ┌───────────────────────────────────────────────────────┐     │
│   │                  Unit Tests                            │     │
│   │     (Each module: layout, render, hooks, components)   │     │
│   └───────────────────────────────────────────────────────┘     │
│                           ▲                                      │
│   ┌───────────────────────────────────────────────────────┐     │
│   │              Static Analysis                           │     │
│   │      (Clippy, rustfmt, custom lints, MIRI)             │     │
│   └───────────────────────────────────────────────────────┘     │
│                           ▲                                      │
│   ┌───────────────────────────────────────────────────────┐     │
│   │              Type System Contracts                     │     │
│   │      (Marker traits, newtype patterns, const generics) │     │
│   └───────────────────────────────────────────────────────┘     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Skill 体系设计

### 1. tink-layout-validator
**目的：** 验证布局计算的正确性

**检测的 Bug 类型：**
- Unicode 字符宽度计算错误（如你刚遇到的问题）
- Flexbox 布局计算错误
- 边框/Padding 尺寸计算错误
- 溢出处理错误
- 嵌套布局坐标累加错误

**验证方法：**
```rust
// 1. 属性测试：任意 Element 树的布局必须满足约束
// 2. 边界测试：极端尺寸（0, 1, MAX）
// 3. Unicode 测试：CJK、Emoji、组合字符
// 4. Golden file：标准布局场景的快照
```

### 2. tink-render-validator
**目的：** 验证渲染输出的正确性

**检测的 Bug 类型：**
- ANSI 转义序列错误
- 样式重叠/泄漏
- 边框字符绘制错误
- 背景色填充错误
- 宽字符渲染占位错误

**验证方法：**
```rust
// 1. render_to_string() 测试：对比期望输出
// 2. ANSI 解析验证：输出可被正确解析
// 3. 视觉快照：保存标准场景的渲染结果
// 4. 差分渲染：增量更新的正确性
```

### 3. tink-hooks-validator
**目的：** 验证 Hooks 系统的正确性

**检测的 Bug 类型：**
- 状态丢失/泄漏
- Effect 重复执行
- 依赖追踪错误
- 生命周期错误
- 多次渲染状态一致性

**验证方法：**
```rust
// 1. 状态持久化测试
// 2. Effect 触发次数验证
// 3. 多组件 Hook 隔离测试
// 4. 并发安全测试
```

### 4. tink-component-validator
**目的：** 验证组件行为的正确性

**检测的 Bug 类型：**
- Builder 模式链式调用问题
- 默认值错误
- 样式继承错误
- 子元素处理错误

**验证方法：**
```rust
// 1. 每个组件的完整 API 测试
// 2. 组合测试：组件嵌套场景
// 3. 边界值测试
```

### 5. tink-terminal-mode-validator
**目的：** 验证不同终端模式下的输出正确性

**检测的 Bug 类型：**
- Raw mode 换行符问题（`\n` vs `\r\n`）
- Alternate screen 模式清理问题
- 光标定位错误
- ANSI 转义序列在不同模式下的行为差异
- 终端尺寸变化处理

**验证方法：**
```rust
// 1. 换行符测试：确保使用 CRLF
// 2. 模式切换测试：进入/退出 raw mode
// 3. 光标位置测试：验证 MoveTo 后的输出位置
// 4. 尺寸变化测试：resize 事件处理
```

### 6. tink-integration-validator
**目的：** 验证完整渲染管线

**检测的 Bug 类型：**
- 端到端渲染错误
- 交互场景问题
- 真实终端兼容性

**验证方法：**
```rust
// 1. 完整应用场景测试
// 2. 键盘输入模拟测试
// 3. 终端尺寸变化测试
```

### 6. tink-visual-regression
**目的：** 防止视觉回归

**方法：**
```rust
// 1. Golden file 快照对比
// 2. 自动化截图对比
// 3. 跨终端一致性验证
```

## 测试基础设施

### TestRenderer - 核心测试工具

```rust
/// 无副作用的渲染器，用于单元测试
pub struct TestRenderer {
    width: u16,
    height: u16,
}

impl TestRenderer {
    /// 渲染 Element 为纯文本（无 ANSI）
    pub fn render_to_plain(&self, element: &Element) -> String;

    /// 渲染 Element 为带 ANSI 的字符串
    pub fn render_to_ansi(&self, element: &Element) -> String;

    /// 获取布局信息用于验证
    pub fn get_layouts(&self, element: &Element) -> HashMap<ElementId, Layout>;

    /// 验证布局约束
    pub fn validate_layout(&self, element: &Element) -> Result<(), LayoutError>;
}
```

### GoldenTest - 快照测试框架

```rust
/// 快照测试宏
macro_rules! golden_test {
    ($name:ident, $element:expr) => {
        #[test]
        fn $name() {
            let renderer = TestRenderer::new(80, 24);
            let output = renderer.render_to_plain($element);
            goldenfile::assert_eq!(&output);
        }
    };
}
```

### PropertyTest - 属性测试

```rust
/// 使用 proptest 进行属性测试
proptest! {
    #[test]
    fn layout_bounds_valid(element in arb_element()) {
        let renderer = TestRenderer::new(80, 24);
        let layouts = renderer.get_layouts(&element);

        for (_, layout) in layouts {
            prop_assert!(layout.x >= 0.0);
            prop_assert!(layout.y >= 0.0);
            prop_assert!(layout.width >= 0.0);
            prop_assert!(layout.height >= 0.0);
        }
    }
}
```

## 持续集成流程

```yaml
# .github/workflows/quality.yml
name: Quality Gate

on: [push, pull_request]

jobs:
  # 第一层：静态分析
  static-analysis:
    - rustfmt --check
    - clippy -- -D warnings
    - cargo deny check
    - cargo audit

  # 第二层：单元测试
  unit-tests:
    - cargo test --lib
    - cargo test --doc

  # 第三层：属性测试
  property-tests:
    - cargo test --test property_tests -- --ignored

  # 第四层：集成测试
  integration-tests:
    - cargo test --test integration_tests

  # 第五层：视觉回归
  visual-regression:
    - cargo test --test golden_tests

  # 第六层：覆盖率
  coverage:
    - cargo tarpaulin --out Xml
    - 要求 ≥ 90% 覆盖率

  # 第七层：性能基准
  benchmarks:
    - cargo bench --bench render_bench
    - 对比历史基准，防止性能回归
```

## 开发者工作流 Skills

### /tink-check - 提交前检查
```
运行所有验证器，确保代码可以安全提交
```

### /tink-test-module <module> - 模块测试
```
针对特定模块运行完整测试套件
```

### /tink-golden-update - 更新快照
```
更新 golden file 快照（需要人工确认）
```

### /tink-coverage - 覆盖率报告
```
生成覆盖率报告，标识未测试代码
```

### /tink-bench - 性能基准
```
运行性能基准测试
```

## 模块级别的约束和不变量

### Layout 模块不变量

```rust
// 1. 所有坐标必须非负
assert!(layout.x >= 0.0 && layout.y >= 0.0);

// 2. 子元素必须在父元素边界内（除非 overflow）
assert!(child.x >= parent.padding.left);
assert!(child.y >= parent.padding.top);

// 3. 布局尺寸必须与渲染尺寸一致
assert_eq!(computed_width, rendered_width);

// 4. Unicode 字符宽度必须正确
assert_eq!(measure_text_width("你好"), 4);  // CJK = 2 each
assert_eq!(measure_text_width("hello"), 5); // ASCII = 1 each
```

### Render 模块不变量

```rust
// 1. 每行渲染宽度不超过终端宽度
assert!(line_width <= terminal_width);

// 2. ANSI 序列必须正确闭合
assert!(output.ends_with("\x1b[0m") || !has_style);

// 3. 宽字符占位符必须正确处理
// 宽字符后紧跟 \0 占位符
```

### Hooks 模块不变量

```rust
// 1. Signal 更新必须触发重渲染
let old_render_count = get_render_count();
signal.set(new_value);
assert!(get_render_count() > old_render_count);

// 2. Effect 依赖变化时必须重新执行
// 3. Hook 调用顺序必须稳定
```

## 错误分类和优先级

| 优先级 | 类型 | 示例 | 检测方法 |
|-------|------|------|---------|
| P0 | 崩溃 | panic, 内存不安全 | 单元测试 + MIRI |
| P1 | 渲染错误 | 布局错位, 样式错误 | 视觉回归测试 |
| P2 | 功能缺陷 | Hook 状态丢失 | 集成测试 |
| P3 | 性能问题 | 渲染卡顿 | 基准测试 |
| P4 | 边缘情况 | 极端尺寸处理 | 属性测试 |

## 下一步行动

1. **创建测试基础设施** (`src/testing/mod.rs`)
2. **为每个核心模块编写属性测试**
3. **创建 Golden file 测试套件**
4. **设置 CI 质量门禁**
5. **编写 Skill 配置文件**
