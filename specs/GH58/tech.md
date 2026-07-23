# Tech Spec：统一终端文本测量与绘制流

## Linked Issue

GH-58: https://github.com/majiayu000/rnk/issues/58

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":58,"complete":true,"paths":["specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md","src/components/display/text.rs","src/layout/text_flow.rs","src/layout/mod.rs","src/layout/measure.rs","src/layout/engine.rs","src/renderer/error.rs","src/renderer/mod.rs","src/renderer/tree_renderer.rs","src/renderer/output.rs","src/renderer/element_renderer.rs","src/renderer/pipeline.rs","src/renderer/app.rs","src/renderer/render_to_string.rs","src/renderer/static_content.rs","src/renderer/terminal_controller.rs","src/lib.rs","src/prelude.rs","src/testing/renderer.rs","tests/text_flow_root_cause.rs","tests/text_source_compat.rs","tests/text_flow_parity.rs","tests/property_tests.rs","tests/prelude_surfaces.rs","tests/text_flow_error_paths.rs"],"spec_refs":["specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md"]}
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
| Element compatibility boundary | `src/core/element.rs:241`, `src/core/element.rs:275`, `src/core/element.rs:330` | `Element` 是 public field-addressable struct；增加 private/public required field 都会破坏外部完整 literal | source 必须走现有 `text_content` / `spans`，不得给 Element 增加字段或改成 `#[non_exhaustive]` |
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

只给字段私有的 `Text` 增加 `TextSourceState::{Exact(String), Structured}`；public
field-addressable `Element` 不增加字段、不加 `#[non_exhaustive]`，也不使用进程全局 sidecar：

- `Text::new` 在调用现有 `str::lines()` 构造兼容 `Line` 视图前保存输入 String 的 exact bytes。
- `Text::spans` / `line` / `from_lines` 没有外部 line-separator 真值；`into_element` 按当前
  Line/Span 顺序生成 canonical source 和 style byte ranges。
- `Text::into_element` 对 `Exact` 把保存的原 String 原样写入既有 `text_content`，不设置会与
  CRLF source 冲突的 normalized spans；对 `Structured` 把 canonical String 写入
  `text_content` 并把 Line/Span 留在既有 `spans` 提供 style structure。
- `Element::text` 已把传入 String 原样写入 `text_content`，clone 已 clone 该字段，无需改
  `Element` layout。外部 struct literal 也以其当前 `text_content` 为 source truth。
- LayoutEngine 以 `text_content` 为 source domain；当 spans canonical content 与它相同，
  生成 exact style ranges；不一致时保留 `text_content` bytes、使用 element-level Style 并
  标记 `Reconstructed` diagnostic，不猜测已经丢失的 CRLF/trailing bytes。

source map 的 byte range 永远相对于 Element 当前 `text_content`（或 text_content 缺失时由
spans 生成的 canonical String）。`Text::new("a\r\nb\r\n")` 的 `into_element` 因而仍含
原始 6 bytes，把两个 CRLF 映射为两个 hard-break ranges；不显示最终空行只影响 rows。
`tests/text_source_compat.rs` 作为 crate 外集成测试同时锁定 exact source 和完整
`Element { ... }` literal 继续编译。

### 2. 单一纯函数与不可变结果

在 `src/layout/text_flow.rs` 增加唯一 flow engine。计划中的类型名是实现指引，review 时可做
不改变合同的命名调整：

- `TextFlowInput`：按 source 顺序保存 logical lines 与 styled source runs。
- `TextFlowOptions`：只保存 logical-flow 输入：content width、`TextWrap`、非零 tab stop、
  ellipsis 和明确的 Unicode width policy/revision。
- `TextFlow`：不可变结果，包含 `rows`、`row_count`、`max_row_width`、positioned
  styled runs、logical source/position map、normalization diagnostics 与 cache identity。
- `TextFlowDisposition`：闭集表示 `Positioned`、`Truncated`、`HardBreak`、`ZeroWidth`、
  `SanitizedControl`；synthetic ellipsis 只存在于 position-to-source 的 synthetic 分支。
- `RenderProjection`：当前 frame 的不可变 visible/clipped cells 与反向 cell map，不写入
  `TextFlowCache`。
- `TextFlowError`：非法 tab stop、算术溢出、不完整 source 覆盖，或 normalization 完成后
  engine 生成的 finalized token/source-map range 仍非 grapheme boundary 时显式失败。调用方
  输入的 style range 在 grapheme 内不属于 error，必须先按 B-002 归一化并记录 diagnostic。

flow 先把 LF/CRLF/CR 归一为 hard-break token，再把 tab 展开为带 source range 的 synthetic
spaces；其余 ESC/C0/DEL/C1 scalar 标记 `SanitizedControl` 并生成 B-022 replacement；
随后用 `UnicodeSegmentation::graphemes(true)` 与同一个 `unicode-width` policy 生成
grapheme tokens。styled span 边界如果落在 grapheme 内，以第一个 source style 为准并记录
diagnostic；绝不把 combining/ZWJ 序列拆开，也不把受支持的 split-style 输入误报为
`TextFlowError`。只有归一化结束后构造出的 finalized token/map range 违反边界才是内部错误。

### 3. Wrap、truncate 与 overflow 顺序

每个 logical line 的处理顺序固定为：

```text
styled source bytes
  -> hard-break / tab normalization
  -> grapheme tokens + display width
  -> TextWrap row construction
  -> ellipsis synthesis (truncate modes only)
  -> positioned safe styled runs + logical source map
  -> per-frame render projection (overflow/scroll/content rect/clip/terminal bounds)
```

- `Wrap` 贪心放置 grapheme；长 token 只在 grapheme 边界续行，不折叠空格。
- `Truncate` 是 `TruncateEnd` 的兼容语义；Start/Middle/End 都只在确有省略时加 `…`。
- width=0 消费输入并返回 logical rows，但 positioned runs 为空。
- overwide grapheme 在 width=1 不拆分；logical flow 保留完整 run，projection 再按当前
  overflow/clip 将它标为 visible 或 clipped。
- overflow/scroll/clip 不反向改变 row count；但为满足 GH-58 的明确验收合同，
  `overflow_x/y` 是 logical cache key 的失效维度，变化时重算等价的 logical rows 后再
  重建 projection。scroll/clip 仍只触发 projection。

### 4. LayoutEngine 所有 flow 生命周期

`LayoutEngine` 增加 engine-local `TextFlowCache` 与当前 frame 的
`ElementId -> flow result` 映射：

1. `build_node` / `element_to_vnode` 从当前 Element 的既有 `text_content` / `spans` 归一化
   exact/canonical/reconstructed source 和 style byte ranges，不需要 Element 新字段。
2. dynamic incremental 路径在 diff 前同步本帧 source；即使 VNode pure text 未改变，
   span/style/options 改变也会更新 NodeContext 并触发 text measure recompute。
3. Taffy measure callback 用 `TextFlowCache::get_or_compute`，从结果读取 width/row count；
   不再调用独立 wrapped-line counter。
4. 完成 layout 后，engine 只发布与当前 exact key 匹配的不可变 flow。失败或中断不更新
   published map。
5. `tree_renderer` 只能通过 engine 的只读查询取得当前 element flow；缺失/错误必须进入
   B-021 typed boundary，禁止回退到单行 `Output::write`。

logical cache key 保存并比较 source、structured run style、`overflow_x/y`、content width、
TextWrap、tab、ellipsis、Unicode policy 完整值；hash 只能用于索引，命中后仍做 equality。
它不依赖 frame-local Element ID；width、overflow 或其他 logical input 改变即 miss。
viewport height、scroll、content rect、clip stack 与 terminal bounds 不属于这个 key，
也不允许缓存其 visible/clipped 结果。每次 render 都从当前完整 viewport context 新建
`RenderProjection`；overflow-only 变化先产生新的等价 logical flow，再以它重建
projection。缓存为 LayoutEngine 实例所有，不增加进程全局锁或跨应用状态。

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
  `RenderProjection` 标记 `Clipped`，不能宣称可见。
- compositor 只接受 safe grapheme。作为独立 trust boundary，grapheme writer 对任何漏入
  的 ESC/C0/DEL/C1 再执行 B-022 replacement；LF/CR/tab 已在 flow 结构化消费，低层
  `Output::write` 则在存 cell 前处理它们，最终 cell suffix 绝不保存 source control scalar。
- terminal encoder 只能输出由结构化 Style、cursor/terminal protocol 生成的固定 allowlist
  ANSI；source replacement 不得经过 raw escape passthrough。

`Output::write` 作为低层单起点兼容 wrapper 改用相同 grapheme writer，但仍不负责自动多行
布局；只有 TextFlow renderer 负责逐行位置。这样既避免破坏直接调用者，也防止 renderer
重新创造第二套 wrap。

### 6. Renderer 与 render-to-string 收敛

`tree_renderer` 删除 `render_spans` 的自有位置算法，统一遍历 TextFlow rows/runs。每个 run
只携带结构化 Style。renderer 每次以当前 overflow、scroll、content rect、祖先 clip stack
和 terminal bounds 生成 `RenderProjection`，再把 safe runs 写入 Output；不缓存 projection。

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
  -> try_render_element_tree -> try_render_element
  -> RenderPipeline::try_render_dynamic_frame / StaticRenderer::try_extract_static_content
  -> App::render_frame -> io::Result (io::Error source = TextRenderError)

LayoutEngine::try_compute + try_render_element_tree
  -> try_render_to_string_with_options / try_render_to_string*
  -> Result<String, TextRenderError>
```

- T5 在 tree/element renderer 与 dynamic pipeline 新增 Result-returning
  `try_render_element_tree`、`try_render_element`、`try_render_dynamic_frame`。现有
  `render_element_tree` / `render_element` / `render_dynamic_frame` 保留原返回类型并只委托
  try variant，Err 时 fail loudly；因此 T5 完成时 App/static/TestRenderer 的现有调用点仍
  编译，且不会把 error 变成 blank/partial output。
- T8 再为 static renderer 增加 `try_extract_static_content`；现有
  `extract_static_content` 同样保留为 fail-loud wrapper。App、static 内部、
  TerminalController 与 TestRenderer callers 切到 T5/T8 的 try variants；任一 child 失败
  立即停止临时 Output，调用方不得提交 partial frame/static lines。
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
- `Element` 不增加 field、不加 `#[non_exhaustive]`；外部 struct literal 是明确受支持的
  compatibility surface，由 `tests/text_source_compat.rs` 在 crate 外编译。
- `Text::get_lines`、现有 `render_to_string*` 与 `LayoutEngine::compute*` 签名继续可用；
  private Text source state 和 typed try surface 是增量能力，旧 wrapper 只在错误时 fail loudly。
- 不修改 `src/core/vnode.rs`、`src/reconciler/*` 的 identity/order 规则；exact flow source
  由 LayoutEngine 的 frame context 维护，避免与 GH-59 重叠。
- `TextFlowError` / `TextRenderError` 不包含通用 patch、Taffy tree 或 full-rebuild variants；
  那些 typed errors/transaction semantics 仍属于 GH-60。
- 不加入全局 cache/source sidecar、provider 依赖、raw ANSI parser 或 chat 类型。

## Product-to-Test Mapping

所有 filtered 验证只能调用下面两个 exact helper。两者先 `--list --exact`，且匹配数必须
恰好为 1；不得在表格、任务或 handoff 中直接运行未守卫的 cargo filter：

```sh
verify_lib_exact() {
  test_name="$1"
  matched="$(
    cargo test --workspace --lib --locked "$test_name" -- --list --exact |
      awk '/: test$/{count++} END{print count+0}'
  )"
  test "$matched" -eq 1 || {
    printf 'expected one exact lib test, matched %s: %s\n' "$matched" "$test_name" >&2
    return 1
  }
  cargo test --workspace --lib --locked "$test_name" -- --exact
}

verify_integration_exact() {
  target="$1"
  test_name="$2"
  matched="$(
    cargo test --test "$target" --locked "$test_name" -- --list --exact |
      awk '/: test$/{count++} END{print count+0}'
  )"
  test "$matched" -eq 1 || {
    printf 'expected one exact integration test, matched %s: %s::%s\n' \
      "$matched" "$target" "$test_name" >&2
    return 1
  }
  cargo test --test "$target" --locked "$test_name" -- --exact
}
```

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `layout::text_flow`, `LayoutEngine`, `tree_renderer` | `verify_lib_exact layout::text_flow::tests::text_flow_shared_result`；`verify_integration_exact text_flow_parity measure_rows_equal_rendered_rows` |
| B-002 | styled source normalization / positioned runs | `verify_lib_exact layout::text_flow::tests::text_flow_styled_runs`；`verify_lib_exact layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes` |
| B-003 | empty input / empty line normalization | `verify_lib_exact layout::text_flow::tests::text_flow_empty_inputs` |
| B-004 | exact-source hard-break tokenizer | `verify_integration_exact text_source_compat exact_crlf_and_trailing_break_ranges` |
| B-005 | grapheme tokenizer / Output grapheme cell | `verify_lib_exact layout::text_flow::tests::text_flow_graphemes`；`verify_integration_exact text_flow_parity unicode_graphemes_render_intact` |
| B-006 | tab expansion | `verify_lib_exact layout::text_flow::tests::text_flow_tabs`，断言 stops 1/4/8、source range 与零值失败 |
| B-007 | wrap row builder | `verify_lib_exact layout::text_flow::tests::text_flow_wrap`；长 ASCII/CJK/emoji token parity |
| B-008 | truncate/ellipsis builder | `verify_lib_exact layout::text_flow::tests::text_flow_truncate`，覆盖五种 enum、无截断/窄 ellipsis 与 synthetic mapping |
| B-009 | zero/narrow width + overwide disposition | `verify_lib_exact layout::text_flow::tests::text_flow_narrow_width`；Output bounds 断言 |
| B-010 | frame-local renderer projection | `verify_integration_exact text_flow_parity viewport_projection_tracks_overflow_scroll_and_clip` |
| B-011 | logical source map + render projection | `verify_integration_exact property_tests text_flow_source_cell_round_trip` |
| B-012 | exact logical cache key | `verify_lib_exact layout::text_flow::tests::text_flow_cache_invalidation`，逐项变更 source/style/width/wrap/overflow/tab/ellipsis/policy |
| B-013 | immutable cache reuse | `verify_lib_exact layout::text_flow::tests::text_flow_cache_reuse`，比较复用与冷算完整 logical result |
| B-014 | resize/overflow invalidation and reprojection | `verify_integration_exact text_flow_parity resize_reflows_or_reprojects_before_render`；`verify_integration_exact text_flow_parity overflow_change_recomputes_flow_and_projection` |
| B-015 | finalized-range `TextFlowError` + atomic publish | `verify_lib_exact layout::text_flow::tests::finalized_non_grapheme_range_is_error`；`verify_lib_exact layout::engine::tests::text_flow_failure_is_atomic` |
| B-016 | immutable readers / interrupted compute | `verify_lib_exact layout::text_flow::tests::text_flow_interruption` |
| B-017 | compatibility wrappers/public surface | `verify_integration_exact text_source_compat external_element_struct_literal_compiles`；`cargo check --workspace --all-targets --all-features --locked` |
| B-018 | structured style only / no raw controls | `verify_integration_exact text_flow_parity source_controls_are_not_terminal_sequences` |
| B-019 | current-head evidence/coverage | `cargo fmt --all -- --check`; exact CI clippy command；`cargo test --workspace --all-targets --all-features --locked`；CodeCov patch coverage >=80%，TextFlow core tarpaulin report =100% |
| B-020 | Text private source -> existing Element fields | `verify_integration_exact text_source_compat exact_crlf_and_trailing_break_ranges`；`verify_integration_exact text_source_compat structured_and_reconstructed_domains` |
| B-021 | engine/render/App/string typed failure chain | `verify_integration_exact text_flow_error_paths typed_error_reaches_all_render_entrypoints`；`verify_integration_exact prelude_surfaces try_render_to_string_surface` |
| B-022 | compositor control sanitization | `verify_lib_exact renderer::output::tests::source_controls_are_replaced`；`verify_integration_exact text_flow_parity source_controls_are_not_terminal_sequences` |
| B-023 | uncached viewport projection | `verify_integration_exact text_flow_parity viewport_projection_tracks_overflow_scroll_and_clip`；`verify_integration_exact text_flow_parity overflow_change_recomputes_flow_and_projection` |
| B-024 | Element literal compatibility | `verify_integration_exact text_source_compat external_element_struct_literal_compiles` |

## 数据流

### 输入

- Element 既有 `text_content` source truth、可选 `Line` / `Span` style structure 与 Style。
- Taffy 提供的 known dimensions / available content width。
- `TextWrap`、`Overflow`、默认 tab stop、ellipsis 与 Unicode width policy。
- renderer 提供的 overflow、scroll offsets、element content rect、完整 clip stack 与
  terminal Output bounds。

### 处理

1. `Text` 私有 source state 在 `str::lines()` 前保存 exact source；`into_element` 把 exact
   或 canonical source 写入现有 `text_content`，不改变 Element public layout。
2. 当前 frame 从 `text_content` / `spans` 归一为带 origin/style byte ranges 的
   `TextFlowInput`；mismatch 标记 Reconstructed。
3. hard-break/tab 先结构化消费，其余 ESC/C0/DEL/C1 转成带原 source range 的 safe replacement。
4. logical cache 用完整 key 查找；未命中时纯函数构建完整临时 flow。
5. 所有 logical rows/runs/source map 成功后原子发布 immutable result。
6. Taffy measure 读取同一结果的 dimensions。
7. renderer 以当前 viewport/clip 输入生成不缓存的 RenderProjection，再写 safe grapheme cells。
8. 任一 failure 通过 `TextFlowError -> TextRenderError -> try*/App io::Result` 退出，当前
   临时 output/static lines/previous VNode 均不提交。
9. terminal encoder 只把 safe cells 与结构化 allowlisted ANSI 生成最终输出。

### 输出与持久化

输出只存在于 Text/Element/LayoutEngine 生命周期内：既有 owned `text_content`、
immutable logical TextFlow、frame-local RenderProjection 与终端 cell buffer。无磁盘、
网络、provider、数据库、全局 sidecar 或跨进程持久化。source map 使用当前 source domain
的 UTF-8 byte range，不持有应用外部可变引用。

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
- 给 public Element 增加 metadata field 或 `#[non_exhaustive]`：拒绝。两者都会破坏现有
  外部完整 struct literal；exact/canonical source 可由 Text 私有状态写入既有字段。
- 把 visible/clipped disposition 放入 logical cache：拒绝。height/scroll/ancestor clip
  改变会复用 stale visibility；这些状态只存在于 frame-local projection。
- 仅“不解析”原始 ESC：拒绝。终端仍会执行被透传的 bytes；Output 必须替换 source controls。
- 在 GH-58 同时改 VNode identity/transactional patch：拒绝。属于 GH-59/GH-60，扩大范围
  也会让根因验证失焦。
- 只在 LayoutEngine 保存 `last_error` 但 renderer 仍返回 String：拒绝。调用方会把 blank/
  partial output 当成功；typed `try_*` 必须贯穿真实 dynamic/static/string entrypoints。
- 把每个 grapheme 直接改成独立 `String` cell 并破坏 `StyledChar::ch`：拒绝。优先使用内部
  suffix/continuation 兼容表示，保留现有测试和调用面。

## 风险

- Security：raw text 可能包含 ANSI/control bytes。缓解：flow 标记并替换 controls，
  Output trust boundary 二次替换，terminal encoder 只生成结构化 allowlisted ANSI；负例
  锁定 screen-clear、cursor move 与 OSC payload 不能进入 terminal stream。
- Compatibility：trailing newline、`Text::get_lines`、truncate、infallible render wrapper
  、direct `Output::write` 与 Element literals 已有依赖。缓解：只给 private Text 加 source
  state 并写回既有 text_content，不改 Element fields/get_lines；默认可见行不增加 trailing
  empty row，`Truncate` 等价 End，旧 wrapper/write 保留签名并在错误时 fail loudly。
- Correctness：span 边界可能切入 combining/ZWJ grapheme。缓解：先拼 source 再分 grapheme，
  首 source style + diagnostic，property test 锁定 byte boundaries。
- Performance：完整 source map 与 exact cache key 增加内存/比较成本。缓解：immutable
  logical result 共享、engine-local 有界 cache；projection 每 frame 重建但只遍历当前
  positioned runs。GH-61 再建立整体 benchmark 门槛。
- Terminal compatibility：ambiguous width 与 width=1 的宽字符无法在所有终端相同显示。
  缓解：policy/revision 进入 key，Visible/Hidden/Scroll 有显式 overwide disposition，
  不宣称超出支持矩阵。
- Maintenance：legacy measure helpers 容易再次形成第二套算法。缓解：helper 复用 core，
  parity tests 同时覆盖 direct/dynamic/render-to-string 三路径。
- Scope：typed error 容易扩成 GH-60 的通用 layout error。缓解：GH-58 variants 只覆盖
  TextFlow/source-map/missing-current-flow；Taffy patch/rebuild/transaction error 明确不纳入。

## 测试计划

- [ ] unit：exact/canonical/reconstructed source ingress、tokenization、hard break、tab、
      control replacement、grapheme、wrap、truncate、ellipsis、logical source map/cache、
      atomic publish、interruption、Output trust boundary 与 typed error source chain。
- [ ] integration：`tests/text_flow_parity.rs` 比较 TextFlow rows、Taffy layout height 与
      Output ANSI/cells，覆盖 plain/rich、width=0/1、width/height resize、scroll/clip
      reprojection 与 source control payload；typed entrypoint failures 单独由
      `tests/text_flow_error_paths.rs` 覆盖。
- [ ] property：`tests/property_tests.rs` 生成合法 Unicode/width/style boundaries，断言
      logical/projection map 不落在 grapheme 中间、所有 source 恰好一个 disposition、无越界。
- [ ] compatibility：原 layout measure、output、render-to-string、public surface 与全部
      workspace tests 通过；`tests/text_source_compat.rs` 证明外部完整 Element literal 与
      exact source，`tests/prelude_surfaces.rs` 证明 typed try API 可从稳定 surface 导入。
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
