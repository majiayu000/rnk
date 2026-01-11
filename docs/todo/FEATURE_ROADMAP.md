# Tink Feature Roadmap

> 基于 Ratatui 和 Bubbletea 的功能对比分析，规划 tink 的功能增强路线图。

## 实施状态

| 阶段 | 功能 | 状态 | 文件 |
|------|------|------|------|
| 1.1 | Span/富文本 | ✅ 完成 | `src/components/text.rs` |
| 1.2 | 鼠标事件 | ✅ 完成 | `src/hooks/use_mouse.rs` |
| 1.3 | 滚动状态 | ✅ 完成 | `src/hooks/use_scroll.rs` |
| 1.4 | 窗口标题 | ✅ 完成 | `src/hooks/use_window_title.rs` |
| 2.1 | List 组件 | ✅ 完成 | `src/components/list.rs` |
| 2.2 | Table 组件 | ✅ 完成 | `src/components/table.rs` |
| 2.3 | Scrollbar | ✅ 完成 | `src/components/scrollbar.rs` |
| 2.4 | Tabs 组件 | ✅ 完成 | `src/components/tabs.rs` |
| 2.5 | Progress | ✅ 完成 | `src/components/progress.rs` |
| 3.1 | Sparkline | ✅ 完成 | `src/components/sparkline.rs` |
| 3.2 | BarChart | ✅ 完成 | `src/components/barchart.rs` |

---

## 设计原则

Tink 是 **React-like** 的声明式 TUI 框架（类似 Ink），而非 Ratatui 的即时模式。因此：

1. **组件组合优先** - 高级组件由基础组件组合而成
2. **Hooks 驱动状态** - 使用 use_signal, use_effect 等管理状态
3. **保持核心精简** - 核心库只提供必要的 primitives
4. **渐进式增强** - 高级组件可选引入

---

## Phase 1: 核心能力增强 ✅ 已完成

### 1.1 富文本组合 (Span/Text) ✅

支持类似 Ratatui 的 Span/Line 组合：
```rust
// API
Text::spans(vec![
    Span::new("Hello ").color(Color::White),
    Span::new("World").color(Color::Green).bold(),
    Span::new("!").color(Color::Yellow),
])

// 或使用 Line builder
Text::line(
    Line::new()
        .span(Span::new("Part 1").color(Color::Red))
        .span(Span::new(" - "))
        .span(Span::new("Part 2").color(Color::Blue))
)
```

### 1.2 鼠标事件支持 ✅

```rust
use_mouse(|mouse| {
    match mouse.action {
        MouseAction::Press(MouseButton::Left) => { /* 处理点击 */ }
        MouseAction::ScrollUp | MouseAction::ScrollDown => { /* 处理滚轮 */ }
        MouseAction::Move => { /* 处理移动 */ }
        _ => {}
    }
});
```

### 1.3 滚动状态管理 ✅

```rust
let scroll = use_scroll();

// 设置内容和视口大小
scroll.set_content_size(100, 500);
scroll.set_viewport_size(80, 20);

// 处理滚动
scroll.scroll_up(1);
scroll.scroll_down(1);
scroll.page_up();
scroll.page_down();
scroll.scroll_to_item(index);

// 获取可见范围
let (start, end) = scroll.visible_range();
```

### 1.4 窗口标题设置 ✅

```rust
use_window_title("My App - Running");

// 或动态标题
use_window_title_fn(|| format!("Count: {}", count.get()));
```

---

## Phase 2: 高级组件 ✅ 已完成

### 2.1 List 组件 ✅

```rust
let list_state = ListState::with_selected(Some(0));

List::from_items(vec!["Item 1", "Item 2", "Item 3"])
    .highlight_style(Style::new().bg(Color::Blue))
    .highlight_symbol("> ")
    .render(&list_state)
```

### 2.2 Table 组件 ✅

```rust
let table_state = TableState::new();

Table::new()
    .header(Row::new(vec!["ID", "Name", "Email"]))
    .rows(vec![
        Row::new(vec!["1", "Alice", "alice@example.com"]),
        Row::new(vec!["2", "Bob", "bob@example.com"]),
    ])
    .highlight_style(Style::new().inverse())
    .render(&table_state)
```

### 2.3 Scrollbar 组件 ✅

```rust
Scrollbar::new()
    .position(0.5)  // 50% 位置
    .viewport_ratio(0.2)  // 显示 20% 的内容
    .length(20)
    .thumb_color(Color::Cyan)
    .into_element()
```

### 2.4 Tabs 组件 ✅

```rust
Tabs::from_items(vec!["Home", "Settings", "About"])
    .selected(active_tab)
    .highlight_color(Color::Cyan)
    .divider(" | ")
    .into_element()
```

### 2.5 Progress/Gauge 组件 ✅

```rust
// Progress bar
Progress::new()
    .progress(0.75)
    .width(30)
    .filled_color(Color::Green)
    .show_percent(true)
    .into_element()

// Gauge
Gauge::new()
    .progress(0.75)
    .label("CPU")
    .color(Color::Cyan)
    .into_element()
```

---

## Phase 3: 数据可视化 ✅ 已完成

### 3.1 Sparkline 组件 ✅

```rust
Sparkline::from_data(vec![1.0, 4.0, 2.0, 8.0, 5.0, 7.0, 3.0])
    .color(Color::Cyan)
    .width(20)
    .into_element()
```

### 3.2 BarChart 组件 ✅

```rust
BarChart::from_bars(vec![
    Bar::new("A", 10.0).color(Color::Red),
    Bar::new("B", 25.0).color(Color::Green),
    Bar::new("C", 15.0).color(Color::Blue),
])
.bar_max_size(30)
.show_values(true)
.into_element()
```

---

## 文件结构

```
src/
├── components/
│   ├── mod.rs
│   ├── box_component.rs
│   ├── text.rs          # Span/Line 支持
│   ├── list.rs          # List 组件
│   ├── table.rs         # Table 组件
│   ├── scrollbar.rs     # Scrollbar 组件
│   ├── tabs.rs          # Tabs 组件
│   ├── progress.rs      # Progress/Gauge 组件
│   ├── sparkline.rs     # Sparkline 组件
│   └── barchart.rs      # BarChart 组件
├── hooks/
│   ├── mod.rs
│   ├── use_mouse.rs     # 鼠标事件
│   ├── use_scroll.rs    # 滚动状态
│   └── use_window_title.rs  # 窗口标题
└── ...
```

---

## 不添加的功能

以下功能经过评估，决定不添加：

### Backend 抽象
- **原因**: crossterm 已足够成熟且跨平台
- **替代**: 如需其他 backend，可通过 trait 抽象后期添加

### Canvas 组件
- **原因**: 终端字符限制，效果有限，实现复杂
- **替代**: 使用 Sparkline/BarChart 展示数据

### Chart (折线图/散点图)
- **原因**: 终端字符限制，效果有限
- **替代**: 使用 Sparkline 展示趋势
