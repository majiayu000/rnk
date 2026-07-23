# Tech Spec：统一终端文本测量与绘制流

## Linked Issue

GH-58: https://github.com/majiayu000/rnk/issues/58

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":58,"complete":true,"paths":["specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md","src/components/display/text.rs","src/core/element.rs","src/layout/text_flow.rs","src/layout/mod.rs","src/layout/measure.rs","src/layout/engine.rs","src/renderer/error.rs","src/renderer/mod.rs","src/renderer/tree_renderer.rs","src/renderer/output.rs","src/renderer/element_renderer.rs","src/renderer/pipeline.rs","src/renderer/app.rs","src/renderer/render_to_string.rs","src/renderer/static_content.rs","src/renderer/terminal_controller.rs","src/lib.rs","src/prelude.rs","src/testing/renderer.rs","tests/text_flow_parity.rs","tests/property_tests.rs","tests/prelude_surfaces.rs"],"spec_refs":["specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-58 的实现边界。它不修改 GH-59 的 keyed identity/order，不把 GH-60 的
事务式 patch/error propagation 或 GH-61 的 `LayoutSnapshot`/benchmark 提前并入本 issue。

## Codebase Context

以下锚点均在 `origin/main` 基线 `e4a89ae128533270d28d768d49977a05a389a582`
上通过 Read/grep 核实。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Text node context | `src/layout/engine.rs:29`, `src/layout/engine.rs:88` | `NodeContext` 只保存拼接后的 `text_content`；rich span/style 结构不进入测量上下文 | TextFlow 输入与缓存必须保留 exact span/style 结构，不能只看纯文本 |
| Text measurement | `src/layout/engine.rs:549`, `src/layout/engine.rs:570`, `src/layout/measure.rs:309` | Taffy measure 调用 `count_wrapped_lines_by_width` 只算行数；width=0 固定返回一行 | 要用同一 flow 结果提供尺寸、rows、runs 与 source map |
| Unicode helpers | `src/layout/measure.rs:3`, `src/layout/measure.rs:48`, `src/layout/measure.rs:89` | helper 使用 `unicode-segmentation` / `unicode-width`，wrap 以 grapheme 处理，但只返回重写后的字符串 | 可复用依赖与兼容 helper，但不能继续让 helper 成为 renderer 之外的第二套算法 |
| Text source loss | `src/components/display/text.rs:210`, `src/components/display/text.rs:221`, `src/components/display/text.rs:223`, `src/components/display/text.rs:332` | `Text::new` 先调用 `str::lines()`；trailing break 被丢弃，CRLF 被移除，`into_element` 再用 LF 拼接 | 必须在 constructor ingress 保存 exact bytes，不能让 TextFlow 从归一化 Element 猜 source |
| Element text boundary | `src/core/element.rs:241`, `src/core/element.rs:252`, `src/core/element.rs:275`, `src/core/element.rs:330` | Element 只保存 `text_content` / `spans`，clone 与 `Element::text` 没有独立 source metadata | exact/canonical/reconstructed source domain 需要一个可 clone、可校验的内部 metadata 边界 |
| Tree rendering | `src/renderer/tree_renderer.rs:69`, `src/renderer/tree_renderer.rs:74`, `src/renderer/tree_renderer.rs:175` | simple text 单次 `Output::write`；rich spans 按原 line/span 直接横向写 | renderer 必须改为消费 engine 发布的 positioned runs |
| Output buffer | `src/renderer/output.rs:9`, `src/renderer/output.rs:179`, `src/renderer/output.rs:228`, `src/renderer/output.rs:309` | cell 只存一个 `char`；write 遇到 newline/行末就停止，宽字符用占位 cell | 需要 grapheme-safe 写入与 continuation/suffix 表达，且保留现有 char-facing 兼容 |
| Render-to-string sizing | `src/renderer/render_to_string.rs:185`, `src/renderer/render_to_string.rs:196`, `src/renderer/render_to_string.rs:209` | 辅助路径再次独立调用 wrapped-line counter 估算高度 | 必须删除该重复语义，改读 layout/TextFlow 的实际 rows |
| Existing policy surface | `src/core/style.rs:110`, `src/core/style.rs:121`, `src/core/style.rs:332` | 已存在 `Overflow::{Visible,Hidden,Scroll}` 与五种 `TextWrap`，但当前 renderer 未完整执行其文本流语义 | GH-58 应实现已有 enum 合同，不新增同义 alias |
| Incremental input | `src/layout/engine.rs:132`, `src/layout/engine.rs:237` | dynamic frame 由 Element 生成 VNode；VNode text 只含拼接字符串 | engine 需在每帧独立同步 exact flow source/context，不能要求 GH-59 先改 NodeKey |
| Untyped layout entrypoints | `src/layout/engine.rs:112`, `src/layout/engine.rs:133`, `src/layout/engine.rs:287` | direct/VNode compute 返回 `()` 或 tuple，measure/taffy 错误没有 typed caller boundary | 增加仅覆盖 TextFlow 的 `try_compute*`，旧签名只做 fail-loud compatibility wrapper |
| Untyped renderer chain | `src/renderer/tree_renderer.rs:40`, `src/renderer/element_renderer.rs:11`, `src/renderer/pipeline.rs:17`, `src/renderer/app.rs:248` | tree/element renderer 返回 `()`，dynamic pipeline 返回 `String`，App 只收到该 String | flow failure 必须经 `TextRenderError` 进入 App 的失败 `io::Result`，不能提交 partial frame |
| Other render callers | `src/renderer/render_to_string.rs:31`, `src/renderer/static_content.rs:32`, `src/renderer/terminal_controller.rs:127`, `src/testing/renderer.rs:42` | public string helpers、static content、println element 与 TestRenderer 都走 infallible direct render | 新增 typed try surface；已有无 Result wrapper 只能 fail loudly，不能返回空白/第一行 |

## 设计方案

### 1. 在归一化前保存 source domain

`Text` 增加内部 source origin，`Element` 增加可 clone 的内部 `TextSourceMetadata`。metadata
同时保存 stored source、origin、style byte ranges，以及构建当时 `text_content` / `spans`
兼容投影的 snapshot：

- `Text::new` 在调用现有 `str::lines()` 构造兼容 `Line` 视图前保存输入 String 的 exact bytes。
- `Element::text` 同时把原 String 写入 `text_content` 与 exact source metadata。
- `Text::spans` / `line` / `from_lines` 没有外部 line-separator 真值；`into_element` 按当前
  Line/Span 顺序一次性生成 canonical source、style byte ranges 和 `Canonical` origin。
- `Text::new` 的 `into_element` 把保存的 exact source 与 style range 写入 Element；现有
  `lines`、`text_content`、`spans` 继续作为兼容视图，不再承担 source 真值。
- Element clone 原样 clone metadata。LayoutEngine 每帧把当前公开 `text_content` / `spans`
  与 metadata 保存的兼容投影 snapshot 做值比较；未变时继续使用 exact/canonical stored
  source，若调用方直接改公开字段导致不匹配，则从当前字段重建 `Reconstructed` source，
  明确不声称恢复已经丢失的 CRLF/trailing bytes。

source map 的 byte range 永远相对于 metadata 标识的 exact/canonical/reconstructed String。
`Text::new("a\r\nb\r\n")` 即使兼容 Line 视图没有末尾行，TextFlow 仍能看到原始 6 bytes，
把两个 CRLF 分别映射为两个 hard-break ranges；默认不显示最终空行只影响 rows，不删除 map。

### 2. 单一纯函数与不可变结果

在 `src/layout/text_flow.rs` 增加唯一 flow engine。计划中的类型名是实现指引，review 时可做
不改变合同的命名调整：

- `TextFlowInput`：按 source 顺序保存 logical lines 与 styled source runs。
- `TextFlowOptions`：保存 content width、`TextWrap`、`overflow_x/y`、非零 tab stop、
  ellipsis 和明确的 Unicode width policy/revision。
- `TextFlow`：不可变结果，包含 `rows`、`row_count`、`max_row_width`、positioned
  styled runs、双向 source/cell map、normalization diagnostics 与 cache identity。
- `TextFlowDisposition`：闭集表示 `VisibleCells`、`Truncated`、`Clipped`、
  `HardBreak`、`ZeroWidth`；synthetic ellipsis 只存在于 cell-to-source 的 synthetic 分支。
- `TextFlowError`：非法 tab stop、内部 byte range 非 grapheme boundary、算术溢出或不完整
  source 覆盖必须显式失败。

flow 先把 LF/CRLF/CR 归一为 hard-break token，再把 tab 展开为带 source range 的 synthetic
spaces，随后用 `UnicodeSegmentation::graphemes(true)` 与同一个 `unicode-width` policy
生成 grapheme tokens。styled span 边界如果落在 grapheme 内，以第一个 source style 为准并
记录 diagnostic；绝不把 combining/ZWJ 序列拆开。

### 3. Wrap、truncate 与 overflow 顺序

每个 logical line 的处理顺序固定为：

```text
styled source bytes
  -> hard-break / tab normalization
  -> grapheme tokens + display width
  -> TextWrap row construction
  -> ellipsis synthesis (truncate modes only)
  -> positioned styled runs + source map
  -> content-rect overflow visibility classification
```

- `Wrap` 贪心放置 grapheme；长 token 只在 grapheme 边界续行，不折叠空格。
- `Truncate` 是 `TruncateEnd` 的兼容语义；Start/Middle/End 都只在确有省略时加 `…`。
- width=0 消费输入并返回 logical rows，但 positioned runs 为空。
- overwide grapheme 在 width=1 不拆分：Visible 保留完整 run，Hidden/Scroll 标记 clipped。
- overflow 不反向改变 row count；renderer 在 element content rect 与 Output terminal bounds
  上执行实际 cell 裁剪。

### 4. LayoutEngine 所有 flow 生命周期

`LayoutEngine` 增加 engine-local `TextFlowCache` 与当前 frame 的
`ElementId -> flow result` 映射：

1. `build_node` / `element_to_vnode` 从当前 Element 的 `TextSourceMetadata` 读取
   exact/canonical/reconstructed source 和 style byte ranges。
2. dynamic incremental 路径在 diff 前同步本帧 source；即使 VNode pure text 未改变，
   span/style/options 改变也会更新 NodeContext 并触发 text measure recompute。
3. Taffy measure callback 用 `TextFlowCache::get_or_compute`，从结果读取 width/row count；
   不再调用独立 wrapped-line counter。
4. 完成 layout 后，engine 只发布与当前 exact key 匹配的不可变 flow。失败或中断不更新
   published map。
5. `tree_renderer` 只能通过 engine 的只读查询取得当前 element flow；缺失/错误必须进入
   B-021 typed boundary，禁止回退到单行 `Output::write`。

缓存 key 保存并比较完整值；hash 只能用于索引，命中后仍做 equality。它不依赖 frame-local
Element ID，resize/options/source 改变即 miss；完全相同输入复用同一 immutable result。
缓存为 `LayoutEngine` 实例所有，不增加进程全局锁或跨应用状态。

Taffy measure closure 本身不能返回 `Result`，所以失败处理固定为：NodeContext 记录首个
`TextFlowError`，该次 closure 返回只供 Taffy 结束遍历的零尺寸 sentinel，但不发布 flow、
layout 或 cache；`try_compute` / `try_compute_vnode` / `try_compute_element_incremental`
在调用栈返回前读取该 error 并返回 `Err`。现有 `compute*` wrapper 调用对应 try 方法并在
Err 时携带 error 文本 fail loudly，不把 sentinel 当成功 layout。这个合同只涵盖 TextFlow
构建/发布错误；Taffy patch transaction 与通用 LayoutError 仍由 GH-60 负责。

### 5. Grapheme-safe Output compositor

`Output` 保留现有 `StyledChar::ch` 与 `Output::write` 兼容表面，同时为 cell 增加内部
grapheme continuation/suffix 表达和 grapheme 写入原语：

- 一个 grapheme 的首个 scalar 继续保存在 lead cell 的 `ch`，剩余 scalars 作为同 cell
  suffix 在 ANSI render 时紧随输出。
- display width=2 的 lead cell 继续占一个 placeholder cell；overwrite 时 lead、suffix 和
  placeholder 作为整体清理。
- display width=0 的 source grapheme 附着到前一个可见 grapheme；无前导 cell 时保留
  `ZeroWidth` disposition，不做负索引或覆盖下一字符。
- write flow 时以 positioned run 的 row/column 为准；任何 terminal/clip 越界都安全跳过，
  但 source map 保持 `Clipped`，不能宣称可见。

`Output::write` 作为低层单起点兼容 wrapper 改用相同 grapheme writer，但仍不负责自动多行
布局；只有 TextFlow renderer 负责逐行位置。这样既避免破坏直接调用者，也防止 renderer
重新创造第二套 wrap。

### 6. Renderer 与 render-to-string 收敛

`tree_renderer` 删除 `render_spans` 的自有位置算法，统一遍历 TextFlow rows/runs。每个 run
只携带结构化 Style，嵌入字符串的 ANSI 不被解析为样式或命令。

`render_to_string` 的 probe 可继续用于 Taffy 高度求解，但 text height 不再通过
`count_wrapped_lines_by_width` 二次估算；最终高度取当前 layout/TextFlow 结果。direct
`compute`、dynamic incremental 与 render-to-string 对同一 Element/width 必须产出相同
TextFlow rows 和可见 Output。

### 7. Typed failure 传播边界

在 `src/renderer/error.rs` 定义 `TextRenderError`，至少包含
`Flow { element_id, source: TextFlowError }`、`MissingCurrentFlow { element_id }` 和
`IncompleteSourceMap { element_id }`；实现 `std::error::Error::source`，不得擦除原
`TextFlowError`。

调用链固定为：

```text
TextFlow::try_build -> Result<TextFlow, TextFlowError>
  -> LayoutEngine::try_compute*
  -> render_element_tree -> render_element
  -> RenderPipeline::render_dynamic_frame / StaticRenderer::extract_static_content
  -> App::render_frame -> io::Result (io::Error source = TextRenderError)

LayoutEngine::try_compute + render_element_tree
  -> try_render_to_string_with_options / try_render_to_string*
  -> Result<String, TextRenderError>
```

- `tree_renderer`、`element_renderer`、dynamic pipeline 与 static renderer 改为返回
  `Result`；任一 child 失败立即停止当前临时 Output，调用方不得提交 partial frame/static lines。
- `App::render_frame` 把 `TextRenderError` 包装进 `io::Error::other` 并保留 source chain；
  `App::run()` 因而以已有 `io::Result` 向应用返回失败。
- `render_to_string*` 增加同名 `try_` variants 并从 renderer module、crate root 与 prelude
  导出。现有 String-returning wrapper 保留签名，但仅调用 try variant；Err 时带完整 cause
  fail loudly，绝不返回空 String/partial String。
- `TerminalController::handle_println_messages` 使用 try variant 并映射到 I/O error；
  `TestRenderer` 增加 `try_render_to_ansi/plain`，旧 test wrapper 同样 fail loudly。
- 负例分别在 LayoutEngine、tree renderer、dynamic App、static content、
  `try_render_to_string*` 注入同一 error，断言 exact variant/source、未更新 previous VNode、
  未提交 static lines/terminal output、无 stale cache 与无 legacy first-line output。

### 8. 兼容与后续边界

- `src/layout/measure.rs` 的现有 public helper 本 issue 不删除；其行为由 TextFlow core
  复用或作为薄 wrapper 保持现有测试。
- `Text` / `Line` / `Span` 与 `Style::{text_wrap,overflow_x,overflow_y}` 不新增同义字段。
- `Text::get_lines`、现有 `render_to_string*` 与 `LayoutEngine::compute*` 签名继续可用；
  source metadata 和 typed try surface 是增量能力，旧 wrapper 只在错误时 fail loudly。
- 不修改 `src/core/vnode.rs`、`src/reconciler/*` 的 identity/order 规则；exact flow source
  由 LayoutEngine 的 frame context 维护，避免与 GH-59 重叠。
- `TextFlowError` / `TextRenderError` 不包含通用 patch、Taffy tree 或 full-rebuild variants；
  那些 typed errors/transaction semantics 仍属于 GH-60。
- 不加入全局 cache、provider 依赖、ANSI parser 或 chat 类型。

## Product-to-Test Mapping

所有 filter 验证必须先调用下面函数，零匹配立即失败：

```sh
verify_cargo_filter() {
  filter="$1"
  matched="$(
    cargo test --workspace --lib --locked "$filter" -- --list |
      awk '/: test$/{count++} END{print count+0}'
  )"
  test "$matched" -gt 0 || {
    printf 'no tests matched filter: %s\n' "$filter" >&2
    return 1
  }
  cargo test --workspace --lib --locked "$filter"
}
```

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `layout::text_flow`, `LayoutEngine`, `tree_renderer` | `verify_cargo_filter text_flow_shared_result`；`cargo test --test text_flow_parity --locked measure_rows_equal_rendered_rows` |
| B-002 | styled source normalization / positioned runs | `verify_cargo_filter text_flow_styled_runs`；parity fixture 在 span 边界换行并核对 Style |
| B-003 | empty input / empty line normalization | `verify_cargo_filter text_flow_empty_inputs` |
| B-004 | exact-source hard-break tokenizer | `verify_cargo_filter text_flow_hard_breaks`，覆盖 LF/CRLF/CR、连续与末尾 break，并断言原 byte ranges |
| B-005 | grapheme tokenizer / Output grapheme cell | `verify_cargo_filter text_flow_graphemes`；`cargo test --test text_flow_parity --locked unicode_graphemes_render_intact` |
| B-006 | tab expansion | `verify_cargo_filter text_flow_tabs`，断言 stops 1/4/8、source range 与零值失败 |
| B-007 | wrap row builder | `verify_cargo_filter text_flow_wrap`；长 ASCII/CJK/emoji token parity |
| B-008 | truncate/ellipsis builder | `verify_cargo_filter text_flow_truncate`，覆盖五种 enum、无截断/窄 ellipsis 与 synthetic mapping |
| B-009 | zero/narrow width + overwide disposition | `verify_cargo_filter text_flow_narrow_width`；Output bounds 断言 |
| B-010 | overflow classification / renderer clip | `verify_cargo_filter text_flow_overflow`；Visible/Hidden/Scroll ANSI snapshots |
| B-011 | exact/canonical/reconstructed source/cell map | `cargo test --test property_tests --locked text_flow_source_cell_round_trip`，并先用 `-- --list` 断言非零匹配 |
| B-012 | exact cache key | `verify_cargo_filter text_flow_cache_invalidation`，逐项变更 content/span/style/width/wrap/overflow/tab/ellipsis/policy |
| B-013 | immutable cache reuse | `verify_cargo_filter text_flow_cache_reuse`，比较复用与冷算完整结果及重复 render cells |
| B-014 | resize invalidation | `cargo test --test text_flow_parity --locked resize_reflows_before_render` |
| B-015 | `TextFlowError` + atomic compute/publish | `verify_cargo_filter text_flow_failure_is_atomic`，断言无 partial/stale/legacy-first-line fallback |
| B-016 | immutable readers / interrupted compute | `verify_cargo_filter text_flow_interruption`，断言未完成 result 不可见 |
| B-017 | compatibility wrappers/public surface | `verify_cargo_filter layout::measure::tests`；`verify_cargo_filter renderer::output::tests`；`cargo check --workspace --all-targets --all-features --locked` |
| B-018 | structured style only | `cargo test --test text_flow_parity --locked embedded_ansi_is_not_style` |
| B-019 | current-head evidence/coverage | `cargo fmt --all -- --check`; exact CI clippy command；`cargo test --workspace --all-targets --all-features --locked`；CodeCov patch coverage >=80%，TextFlow core tarpaulin report =100% |
| B-020 | Text/Element source metadata ingress | `verify_cargo_filter text_source_preservation`，覆盖 `Text::new("a\r\nb\r\n")`、Element clone、structured canonical source 与 direct-field mismatch -> Reconstructed |
| B-021 | engine/render/App/string typed failure chain | `verify_cargo_filter text_flow_typed_error_chain`；`cargo test --test text_flow_parity --locked typed_error_reaches_all_render_entrypoints`；`cargo test --test prelude_surfaces --locked try_render_to_string_surface` |

## 数据流

### 输入

- Element 的 exact/canonical/reconstructed `TextSourceMetadata`、兼容 `text_content` 或
  `Line` / `Span` 结构化内容与 Style。
- Taffy 提供的 known dimensions / available content width。
- `TextWrap`、`Overflow`、默认 tab stop、ellipsis 与 Unicode width policy。
- renderer 提供的 element content origin、clip stack 与 terminal Output bounds。

### 处理

1. `Text` / `Element` ingress 在 `str::lines()` 或拼接前保存 exact source；structured
   constructors 生成 canonical source，metadata mismatch 才明确重建 Reconstructed source。
2. 当前 frame 从 metadata 把 Element 归一为带 origin/style byte ranges 的
   `TextFlowInput`。
3. cache 用完整 key 查找；未命中时纯函数构建完整临时 flow。
4. 所有 rows/runs/source map 成功后原子发布 immutable result。
5. Taffy measure 读取同一结果的 dimensions。
6. renderer 读取同一结果的 positioned styled runs 并写 grapheme cells。
7. 任一 failure 通过 `TextFlowError -> TextRenderError -> try*/App io::Result` 退出，当前
   临时 output/static lines/previous VNode 均不提交。
8. Output 按 clip/terminal bounds 标记可见或裁剪，最终生成 ANSI 行。

### 输出与持久化

输出只存在于 Element/LayoutEngine 生命周期内：owned source metadata、immutable TextFlow
与终端 cell buffer。无磁盘、网络、provider、数据库或跨进程持久化。source map 使用
metadata 声明的 source domain UTF-8 byte range，不持有应用外部可变引用。

## 备选方案

- 只让 renderer 调用 `wrap_text`：拒绝。布局仍只拿行数，rich style/source map/cache 继续
  分裂，无法证明消费同一结果。
- 预先把内容改写成带 `\n` 的 String：拒绝。会丢 styled run 边界、synthetic ellipsis 与
  source-to-cell 映射。
- 把 source map 定义为当前归一化 `text_content`：拒绝。它无法满足 CRLF/trailing-break
  映射，也会让后续 cursor/selection 相对用户输入发生 byte offset 漂移。
- 每帧在 layout 和 renderer 各算一次相同算法：拒绝。即使函数相同也不是“同一结果”，且
  cache、失败和 policy revision 仍可能漂移。
- 把 flow 放入全局 singleton：拒绝。会引入跨 app 竞态、无界生命周期和测试污染。
- 在 GH-58 同时改 VNode identity/transactional patch：拒绝。属于 GH-59/GH-60，扩大范围
  也会让根因验证失焦。
- 只在 LayoutEngine 保存 `last_error` 但 renderer 仍返回 String：拒绝。调用方会把 blank/
  partial output 当成功；typed `try_*` 必须贯穿真实 dynamic/static/string entrypoints。
- 把每个 grapheme 直接改成独立 `String` cell 并破坏 `StyledChar::ch`：拒绝。优先使用内部
  suffix/continuation 兼容表示，保留现有测试和调用面。

## 风险

- Security：raw text 可能包含 ANSI/control bytes。缓解：TextFlow 不解释 ANSI 或执行终端
  操作，Style 只来自结构化字段；renderer 测试锁定 escape 不变成样式状态。
- Compatibility：trailing newline、`Text::get_lines`、truncate、infallible render wrapper
  与 direct `Output::write` 已有依赖。缓解：metadata 在 constructor 分行前旁路保存，不改变
  get_lines 视图；默认可见行仍不增加 trailing empty row，`Truncate` 等价 End，旧 wrapper/
  write 保留签名并在错误时 fail loudly。
- Correctness：span 边界可能切入 combining/ZWJ grapheme。缓解：先拼 source 再分 grapheme，
  首 source style + diagnostic，property test 锁定 byte boundaries。
- Performance：完整 source map 与 exact cache key 增加内存/比较成本。缓解：immutable
  result 共享、engine-local 有界 cache；GH-61 再建立整体 benchmark 门槛，本 issue 不用
  未验证的 hash-only key 换正确性。
- Terminal compatibility：ambiguous width 与 width=1 的宽字符无法在所有终端相同显示。
  缓解：policy/revision 进入 key，Visible/Hidden/Scroll 有显式 overwide disposition，
  不宣称超出支持矩阵。
- Maintenance：legacy measure helpers 容易再次形成第二套算法。缓解：helper 复用 core，
  parity tests 同时覆盖 direct/dynamic/render-to-string 三路径。
- Scope：typed error 容易扩成 GH-60 的通用 layout error。缓解：GH-58 variants 只覆盖
  TextFlow/source-map/missing-current-flow；Taffy patch/rebuild/transaction error 明确不纳入。

## 测试计划

- [ ] unit：exact/canonical/reconstructed source ingress、tokenization、hard break、tab、
      grapheme、wrap、truncate、ellipsis、overflow、source map、exact cache key、atomic
      publish、interruption 与 typed error source chain。
- [ ] integration：`tests/text_flow_parity.rs` 比较 TextFlow rows、Taffy layout height 与
      Output ANSI/cells，覆盖 exact CRLF/trailing source、plain/rich、width=0/1、resize、
      embedded ANSI，以及 LayoutEngine/dynamic/static/string entrypoint failure injection。
- [ ] property：`tests/property_tests.rs` 生成合法 Unicode/width/style boundaries，断言
      source/cell map 不落在 grapheme 中间、所有 source 恰好一个 disposition、无越界。
- [ ] compatibility：原 layout measure、output、render-to-string、public surface 与全部
      workspace tests 通过；`tests/prelude_surfaces.rs` 同时证明 typed try API 可从稳定 surface
      导入。
- [ ] coverage：CodeCov 当前 head patch coverage >=80%；TextFlow 核心 segmentation、
      wrap、truncate、cache 和失败分支 100%，报告绑定 implementation PR exact head。
- [ ] fresh commands：

  ```sh
  cargo fmt --all -- --check
  cargo check --workspace --all-targets --all-features --locked
  cargo clippy --workspace --all-targets --all-features --locked -- \
    -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
  cargo test --workspace --all-targets --all-features --locked
  ```

## 回滚方案

实现 PR 必须保持现有 public 构造器和 helper。若新 flow 出现回归，回滚整个 GH-58
implementation commit/PR，恢复此前 renderer 与 measure 路径；不得在运行时静默切回只写
第一行的 legacy fallback。若仅 cache 引发问题，可在后续修复中禁用复用但仍每次运行同一个
TextFlow core；不能恢复两套换行语义。
