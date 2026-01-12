# glm_chat 版本对比

## 代码行数对比

- **glm_chat_old.rs**: 979 行（包含完整的自定义渲染实现）
- **glm_chat.rs**: 721 行（使用 rnk v0.6.0 的 render API）
- **减少**: 258 行（26.4% 的代码减少）

## 主要区别

### glm_chat_old.rs（自定义渲染）

包含以下自定义实现：

1. **文本换行** (`wrap_text`)
   - 处理 Unicode 字符宽度
   - 处理显式换行符
   - 处理行边界

2. **高度计算** (`calculate_element_height`)
   - 考虑文本换行
   - 考虑边框和 padding
   - 递归处理子元素

3. **元素渲染** (`render_element_recursive`)
   - 背景渲染
   - 边框渲染
   - 文本内容渲染（支持 spans）
   - 递归渲染子元素

4. **主渲染函数** (`render_to_string`)
   - 创建 LayoutEngine
   - 计算布局
   - 创建 Output 缓冲区
   - 调用递归渲染

**关键代码片段（250+ 行）：**

```rust
// 文本换行实现
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    // ... 50+ 行实现
}

// 高度计算
fn calculate_element_height(element: &Element, max_width: u16) -> u16 {
    // ... 40+ 行实现
}

// 元素渲染
fn render_element_recursive(
    element: &Element,
    engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
    container_width: u16,
) {
    // ... 100+ 行实现
}

// 主渲染函数
fn render_to_string(element: &Element, width: u16) -> String {
    // ... 10+ 行实现
}
```

### glm_chat.rs（使用 rnk API）

简化为：

```rust
// 打印元素（带换行）
fn print_element(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    println!("{}", output);
}

// 打印元素（不带换行）
fn print_element_inline(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    print!("{}", output);
}
```

**仅需 10 行代码！**

## 功能对比

| 功能 | glm_chat_old.rs | glm_chat.rs |
|------|----------------|-------------|
| 文本换行 | ✅ 自定义实现 | ✅ rnk 内置 |
| Unicode 支持 | ✅ 手动处理 | ✅ rnk 内置 |
| 边框渲染 | ✅ 手动实现 | ✅ rnk 内置 |
| 布局计算 | ✅ 手动调用 Taffy | ✅ rnk 内置 |
| 内容宽度计算 | ❌ 使用全宽 | ✅ 智能计算 |
| 尾部空格处理 | ❌ 未处理 | ✅ 自动 trim |
| 代码行数 | 979 行 | 721 行 |
| 维护成本 | 高 | 低 |

## 教育价值

### glm_chat_old.rs 适合学习：

1. **TUI 渲染原理**
   - 如何从零实现文本换行
   - 如何计算元素高度
   - 如何递归渲染元素树
   - 如何处理 Unicode 字符宽度

2. **布局引擎集成**
   - 如何使用 Taffy 布局引擎
   - 如何将布局结果转换为渲染输出
   - 如何处理边框和 padding

3. **底层实现细节**
   - Output 缓冲区的使用
   - ANSI 转义序列的生成
   - 坐标系统和偏移计算

### glm_chat.rs 适合学习：

1. **框架使用**
   - 如何使用 rnk 的高级 API
   - 如何构建实际应用
   - 如何快速原型开发

2. **代码简洁性**
   - 框架抽象的价值
   - API 设计的重要性
   - 关注点分离

3. **最佳实践**
   - 使用成熟的库而不是重复造轮子
   - 代码可维护性
   - 开发效率

## v0.6.0 的价值体现

通过对比可以看出，rnk v0.6.0 的 render API 带来的价值：

1. **大幅减少代码量**：从 979 行减少到 721 行（-26.4%）
2. **消除重复代码**：250+ 行的自定义渲染逻辑被 2 个函数调用替代
3. **提高可维护性**：不需要维护复杂的渲染逻辑
4. **更好的对齐**：自动 trim 和智能宽度计算
5. **统一的 API**：所有项目都可以使用相同的渲染方式

## 运行示例

```bash
# 运行新版本（推荐）
GLM_API_KEY=your_key cargo run --example glm_chat

# 运行旧版本（教育目的）
GLM_API_KEY=your_key cargo run --example glm_chat_old
```

两个版本的功能完全相同，只是实现方式不同。

## 迁移指南

如果你的项目中有类似 glm_chat_old.rs 的自定义渲染代码，可以这样迁移：

### 之前（自定义渲染）

```rust
fn print_element(element: &Element) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let output = render_to_string(element, width);
    println!("{}", output);
}

// 需要实现：
// - wrap_text()
// - calculate_element_height()
// - render_element_recursive()
// - render_to_string()
// 总共 250+ 行代码
```

### 之后（使用 rnk API）

```rust
fn print_element(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    println!("{}", output);
}

// 仅需 3 行代码！
```

## 总结

- **glm_chat_old.rs**：保留作为教育资源，展示底层实现原理
- **glm_chat.rs**：推荐使用，展示现代框架的最佳实践

两个版本都有其价值，选择取决于你的学习目标：
- 想学习 TUI 渲染原理？看 glm_chat_old.rs
- 想快速构建应用？用 glm_chat.rs
