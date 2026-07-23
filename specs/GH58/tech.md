# Tech Spec：统一终端文本测量与绘制流

## Linked Issue

GH-58: https://github.com/majiayu000/rnk/issues/58

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":58,"complete":true,"paths":["specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md","src/components/display/text.rs","src/layout/text_flow.rs","src/layout/mod.rs","src/layout/measure.rs","src/layout/engine.rs","src/layout/engine/text_flow_bridge.rs","src/layout/engine/tests.rs","src/renderer/error.rs","src/renderer/mod.rs","src/renderer/tree_renderer.rs","src/renderer/tree_renderer/projection.rs","src/renderer/tree_renderer/projection/staged.rs","src/renderer/tree_renderer/projection/tests.rs","src/renderer/output.rs","src/renderer/output/tests.rs","src/renderer/element_renderer.rs","src/renderer/pipeline.rs","src/renderer/app.rs","src/renderer/render_to_string.rs","src/renderer/static_content.rs","src/renderer/terminal_controller.rs","src/lib.rs","src/prelude.rs","src/testing/renderer.rs","tests/text_flow_root_cause.rs","tests/text_source_compat.rs","tests/text_flow_parity.rs","tests/text_flow_renderer_error_paths.rs","tests/property_tests.rs","tests/prelude_surfaces.rs","tests/text_flow_error_paths.rs"],"spec_refs":["specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-58 的实现边界。它不修改 GH-59 的 keyed identity/order，不把 GH-60 的
事务式 patch/error propagation 或 GH-61 的 `LayoutSnapshot`/benchmark 提前并入本 issue。

## Codebase Context

以下锚点均在 `origin/main` 基线 `54617335e9ec16825232685e94433acdd1fd7cb4`
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
- `Text::into_element` 对 `Exact` 把保存的原 String 原样写入既有 `text_content`，并在 T7
  checkpoint 保留当前 normalized multiline spans 作为 legacy renderer compatibility view，
  避免 TextFlow renderer 在 T5 接管前让 `Text::new("a\nb")` 退化为只显示第一行。该 view
  不声明独立 source domain；T2 按可见 grapheme 与 exact hard-break 顺序对齐其样式，不能从
  normalized spans 反推 CRLF/trailing bytes。对 `Structured` 则把 canonical String 写入
  `text_content` 并把 Line/Span 留在既有 `spans` 提供 style structure。
- `Element::text` 已把传入 String 原样写入 `text_content`，clone 已 clone 该字段，无需改
  `Element` layout。外部 struct literal 也以其当前 `text_content` 为 source truth。
- LayoutEngine 以 `text_content` 为 source domain；当 spans canonical content 与它相同，
  生成 exact style ranges。T2 对 `Text::new` 的 legacy compatibility view 按可见 grapheme
  和 hard-break 顺序对齐 exact source；只有该 view 或其他 spans 无法完整、无歧义对齐时，
  才保留 `text_content` bytes、使用 element-level Style 并由 T3 标记 `Reconstructed`
  diagnostic，不猜测已经丢失的 CRLF/trailing bytes。

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
- `RenderProjection`：private、frame-local、不可变的双向 map，不写入 `TextFlowCache`。
  正向以 source byte range、完整 grapheme 与 logical disposition 为 identity，记录其
  `Visible` / `Clipped` cell range；反向让每个占用的 `(x, y)` cell 指回 source range，或
  明确指向 synthetic ellipsis，不能伪造 source byte range。
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

1. `src/layout/engine/text_flow_bridge.rs` 是从 `engine.rs` 拆出的专用桥接模块，负责从当前
   Element/VNode 的既有 `text_content` / `spans` 与 element-level Style 构造 flow 输入、
   对齐 source domain，并在最终 layout width 已知后构造待发布结果；不得以
   `#[rustfmt::skip]`、压缩既有 patch/reorder 逻辑或把 `engine.rs` 卡在 800 行来代替拆分。
   普通无 spans 的 Element/VNode 也必须把 element-level Style 带入每个 TextFlow run。
   `src/layout/engine/tests.rs` 只机械承载从 `engine.rs` 迁出的既有 engine unit tests 与
   T3 新 exact gates；不得改变既有测试语义、弱化断言或引入 GH-59/GH-60 行为。
2. legacy normalized spans 以可见 grapheme 与 hard-break 序列对齐 exact CRLF、lone CR 和
   trailing-break source ranges；只有序列确实无法完整、无歧义对齐时才发布
   `Reconstructed` diagnostic，并继续以 `text_content` 为 source truth。
3. dynamic incremental 路径在 diff 前同步本帧 source；即使 diff 产生空 patch 集合，或
   VNode pure text 未改变但 span/style/options 改变，也会更新 NodeContext 并触发 text
   measure recompute。T3 的 exact no-patch fixture 必须分别改变 source 与 span style，
   并证明当前 frame flow/cache identity 都已更新。
4. Taffy measure callback 用 `TextFlowCache::get_or_compute`，从结果读取 width/row count；
   不再调用独立 wrapped-line counter。
5. 完成 layout 后，engine 必须用最终 content width 取得或重算与当前 exact key 匹配的
   immutable flow，再与 layout 原子发布；即使 known dimensions 让 Taffy 不调用 measure
   callback，也不得缺失 current flow、保留错误 width 的 flow 或把缺失静默当成功。失败或
   中断不更新 published map。
6. `tree_renderer` 只能通过 engine 的只读查询取得当前 element flow；缺失/错误必须进入
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

- `src/renderer/output/tests.rs` 只机械承载从 `output.rs` 迁出的既有 unit tests 与 T4 新
  exact gates；不得改变或弱化既有测试语义，不得用 `#[rustfmt::skip]` 或压缩旧实现把
  `output.rs` 卡在 800 行，也不得在该文件引入 integration fixtures 或 T5 renderer 行为。
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
- `Output` 提供 crate-private、只读的 whole-EGC active-clip visibility query，供 projection
  在写入前判断一个 grapheme 的全部 display cells 是否同时位于 terminal bounds、所有当前
  active clips 内。该 query 不移动 cursor、不修改 cell/clip state；wide grapheme 只有所有
  cell 都可见时才返回 visible，不能让 projection 先宣称 visible 再由 Output 丢弃一半。
  `renderer::output::tests::active_clips_report_grapheme_visibility` 必须覆盖嵌套 clip、边界和
  wide grapheme 的全宽原子结果。
- `Output` 另提供 crate-private isolated staged snapshot；snapshot 完整复制 terminal
  width/height dimensions、grid、grapheme lead/suffix/continuation metadata、dirty flags
  与 active clip stack，后续对
  snapshot 的 clip、cell、metadata 或 dirty mutation 均不得反向影响 source `Output`。
  不为 `Output` 扩展 public `Clone` 或其他 public snapshot API。
- staged snapshot 提供只读、signed-coordinate、checked-arithmetic 的 prospective
  grapheme write-footprint query。query 必须按 snapshot 当前 staged 写入顺序的状态返回
  本次目标 EGC cells，以及会被替换的既有完整 EGC lead/suffix/continuation cells；从 wide
  continuation 写入时也必须追溯并报告旧 wide EGC 的完整 footprint。target footprint 与
  old footprint 必须同时通过 terminal bounds 和当前完整 active clip stack，不能只验证新
  lead cell。query 本身不得写 cell、移动 cursor 或改变 clip/dirty 状态。
- exact gate
  `renderer::output::tests::staged_snapshot_and_write_footprint_are_isolated` 必须锁定 snapshot
  的 terminal width/height、grid、grapheme metadata、dirty flags、active clips 均完整
  复制，snapshot mutation 不影响 source；并覆盖从 wide continuation 跨 clip 覆写时 old
  EGC 完整 footprint 与 target footprint 的原子可见性。

`Output::write` 作为低层单起点兼容 wrapper 改用相同 grapheme writer，但仍不负责自动多行
布局；只有 TextFlow renderer 负责逐行位置。这样既避免破坏直接调用者，也防止 renderer
重新创造第二套 wrap。

### 6. Renderer 与 render-to-string 收敛

`tree_renderer` 删除 `render_spans` 的自有位置算法，统一遍历 TextFlow rows/runs。每个 run
只携带结构化 Style。renderer 必须先解析并验证整棵树所需的所有 current TextFlow；缺失或
malformed flow 在创建任何可提交绘制结果前返回 typed error。随后从调用方 `Output` 创建
T4 isolated staged snapshot，并在该 private snapshot 上严格按最终 paint order 预写
background、border、sibling 与 text。原 caller `Output` 在此期间不得发生任何 cell、
grapheme metadata、dirty flag 或 clip stack mutation。

每个 text write 在当前 staged 顺序状态下，以当前 Text 自身的 content rect、
`overflow_x/y` 与 scroll、全部祖先 clip、terminal bounds，以及调用
`try_render_element_tree` 前 `Output` 已有的 active clip stack 生成并验证
`RenderProjection`。投影必须使用 T4 prospective footprint query，同时验证 target EGC
和其将替换的 old EGC 完整 footprint；两者都受本次 logical axis viewport 与当前 staged
active clips 约束。projection 的 visible/clipped 与 reverse map 必须根据实际 writer
outcome 构建，不能根据写前猜测宣称 visible。预期 visible 的 write 若意外返回 `Clipped`
或 outcome/footprint 与投影不一致，必须返回 typed error 并丢弃整个 staged snapshot，
不得 silent skip。只有所有 paint、projection 与 map 验证成功后，才以一次内部 replace
把完整 staged state 提交给 caller `Output`，且提交后的 terminal width/height、cell/
grapheme metadata、dirty flags、active clips 与最终 staged state 完全一致。这是 GH-58
单次 renderer 调用的私有 Output 原子性，不扩展为 GH-60 的 layout/patch transaction，
也不得重新引入或公开 `PushClip`。

projection 的正向 map 对每个 source grapheme/disposition 恰好有一条记录：tab 展开的所有
spaces 共享原 tab 的同一 source range；hidden、hard break 与无前导 cell 的 zero-width
grapheme 保留明确的无-cell disposition；被 clip 的 grapheme 保留 `Clipped` cell range。
truncate policy 即使生成多 EGC ellipsis，也必须逐 EGC 标记 synthetic，并让每个可见 cell
反向指向对应 synthetic identity。display width=2 的 grapheme 要么完整 visible、要么完整
clipped，不得产生半个 wide cell。所有 occupied visible/clipped cells 都必须能反查唯一的
source 或 synthetic identity；source/grapheme/disposition 缺项、cell gap、重叠或反向
identity 不一致均为 malformed map，并在任何 Output mutation 前返回 typed failure。

projection 的 x/y overflow 必须独立执行：`overflow_x` 的 Hidden/Scroll 只决定水平
位移与裁剪，`overflow_y` 的 Hidden/Scroll 只决定垂直位移与裁剪；任一轴为 Hidden/Scroll
都不得隐式裁掉另一轴仍为 Visible 的 cells。content rect origin、ancestor offset、scroll
offset、run position 与 terminal position 在整个投影计算中使用 signed coordinates；左移或
上移后的负坐标保持为负值直到 terminal/active-clip visibility 判定，禁止 saturating cast 或
提前 clamp 到 0 后把本应 clipped 的 cells 错投到首列/首行。

T5 必须把上述 private projection 拆为
`src/renderer/tree_renderer/projection.rs` 的 core、
`src/renderer/tree_renderer/projection/staged.rs` 的 staged compositor，以及
`src/renderer/tree_renderer/projection/tests.rs` 的 unit tests；三个文件自然拆分且各自
低于 800 行，不得通过压缩、`#[rustfmt::skip]` 或混入无关职责满足行数。该模块仍属于 T5
ownership，不扩展 public API。T5 的 crate-private exact gate
`renderer::tree_renderer::projection::tests::projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells`
必须同时锁定 visible、clipped、tab same-range、synthetic multi-EGC ellipsis、wide 原子、
hard-break/zero-width/hidden 无-cell 记录、malformed map gap fail-loud、x-visible +
y-hidden、x-hidden + y-visible、scroll 后落到 terminal 左侧/上侧的负坐标不能 clamp，
以及 old wide EGC、active clip、background/border/sibling/text later-paint overwrite 的
最终 writer outcome 与 reverse map 一致，并证明成功路径只 single replace 一次、caller
最终 terminal width/height 与全部 staged state 一致。tree atomic exact gate
`renderer::tree_renderer::tests::text_flow_error_preserves_source_and_commits_no_partial_output`
必须在 staged paint 中途注入 typed failure，并逐字段证明 caller `Output` 的 terminal
width/height、grid、grapheme metadata、dirty flags 与 active clips 完全未变。

projection round-trip validation 每帧必须是 O(cells)：用 cell identity `HashMap` 在单次
遍历中拒绝重复 reverse cell，再断言 forward occupied-cell count 等于 reverse map length；
禁止对每个 forward cell 线性扫描 reverse map 或其他 O(cells²) 实现。crate-private
`renderer::tree_renderer::projection::tests::projection_round_trip_validation_is_linear`
用 2,000 与 10,000 cells 的可诊断规模和计数器/操作次数上界锁定线性逻辑，不新增 benchmark
文件，也不把 GH-61 的整体性能基准并入本 issue。
T5 顺序依赖 T4 先提供并通过 whole-EGC active-clip visibility、isolated staged snapshot
与 prospective target+old-EGC footprint query 的 exact gates。

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
- dynamic incremental 的 layout 或 render 返回 Err 时，T5 必须 invalidate/reset
  `LayoutEngine` 已可能推进的 incremental tree/current publication，使下一次调用强制从
  当前 Element/VNode 做 full rebuild；`previous_vnode` 与 runtime context 继续保持最后一个
  成功帧，不能与失败后残留的 engine tree 混用。该恢复只处理 GH-58 typed text failure 后的
  clean retry，不承诺也不实现 GH-60 的通用 patch transaction、rollback 或任意 layout error。
  `renderer::pipeline::tests::incremental_failure_retries_from_clean_layout_tree` 必须在同一
  exact gate 分别覆盖 layout Err 与 layout 已成功后的 render Err；允许 pipeline 内部共享
  实现接受 private、`#[cfg(test)]` renderer closure/seam，在不扩 public API 的前提下注入
  `MissingCurrentFlow`。两条路径都要证明失败后 engine 已 reset，修正同一 child 后从 clean
  tree 重建，且节点不重复、layout/measure/alias/VNode 与最后成功 runtime state 一致。
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
- 负例按写入所有权和 Rust 可见性分阶段落地：T3 在 LayoutEngine 注入 error；T5 在
  `tree_renderer.rs` 的 `#[cfg(test)]` unit tests 断言 exact variant/source、无 partial Output
  与无 legacy first-line fallback，在 `pipeline.rs` 的 `#[cfg(test)]` unit tests 断言 dynamic
  failure 不更新 previous VNode；crate 外的 `tests/text_flow_renderer_error_paths.rs` 只通过
  public `try_render_to_string*` 断言 source chain 与无 partial String，不调用 crate-private
  renderer/pipeline API。T8 只验证其独占的 App/static/TerminalController/TestRenderer caller
  传播和未提交 static/terminal output。

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
| B-002 | styled source normalization / positioned runs | `verify_lib_exact layout::text_flow::tests::text_flow_styled_runs`；`verify_lib_exact layout::text_flow::tests::split_combining_and_zwj_style_boundary_normalizes`；`verify_lib_exact layout::engine::tests::plain_text_style_is_published` |
| B-003 | empty input / empty line normalization | `verify_lib_exact layout::text_flow::tests::text_flow_empty_inputs` |
| B-004 | exact-source hard-break tokenizer | `verify_integration_exact text_source_compat exact_crlf_and_trailing_break_ranges` |
| B-005 | grapheme tokenizer / Output grapheme cell | `verify_lib_exact layout::text_flow::tests::text_flow_graphemes`；`verify_integration_exact text_flow_parity unicode_graphemes_render_intact` |
| B-006 | tab expansion | `verify_lib_exact layout::text_flow::tests::text_flow_tabs`，断言 stops 1/4/8、source range 与零值失败 |
| B-007 | wrap row builder | `verify_lib_exact layout::text_flow::tests::text_flow_wrap`；长 ASCII/CJK/emoji token parity |
| B-008 | truncate/ellipsis builder | `verify_lib_exact layout::text_flow::tests::text_flow_truncate`，覆盖五种 enum、无截断/窄 ellipsis 与 synthetic mapping |
| B-009 | zero/narrow width + overwide disposition | `verify_lib_exact layout::text_flow::tests::text_flow_narrow_width`；Output bounds 断言 |
| B-010 | frame-local renderer projection | `verify_lib_exact renderer::output::tests::active_clips_report_grapheme_visibility`；`verify_lib_exact renderer::output::tests::staged_snapshot_and_write_footprint_are_isolated`；`verify_lib_exact renderer::tree_renderer::projection::tests::projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells`；`verify_integration_exact text_flow_parity viewport_projection_tracks_overflow_scroll_and_clip` |
| B-011 | logical source map + render projection | T2：`verify_integration_exact property_tests text_flow_logical_source_round_trip`；T5：`verify_lib_exact renderer::tree_renderer::projection::tests::projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells`；`verify_lib_exact renderer::tree_renderer::projection::tests::projection_round_trip_validation_is_linear`；`verify_integration_exact text_flow_parity projection_source_cell_round_trip` |
| B-012 | exact logical cache key | `verify_lib_exact layout::text_flow::tests::text_flow_cache_invalidation`，逐项变更 source/style/width/wrap/overflow/tab/ellipsis/policy；`verify_lib_exact layout::engine::tests::incremental_no_patch_refreshes_source_and_style` |
| B-013 | immutable cache reuse | `verify_lib_exact layout::text_flow::tests::text_flow_cache_reuse`，比较复用与冷算完整 logical result |
| B-014 | resize/overflow invalidation and reprojection | `verify_lib_exact layout::engine::tests::known_dimensions_publish_final_width_flow`；`verify_integration_exact text_flow_parity resize_reflows_or_reprojects_before_render`；`verify_integration_exact text_flow_parity overflow_change_recomputes_flow_and_projection` |
| B-015 | finalized-range `TextFlowError` + atomic publish | `verify_lib_exact layout::text_flow::tests::finalized_non_grapheme_range_is_error`；`verify_lib_exact layout::engine::tests::text_flow_failure_is_atomic` |
| B-016 | immutable readers / interrupted compute | `verify_lib_exact layout::text_flow::tests::text_flow_interruption` |
| B-017 | compatibility wrappers/public surface | `verify_integration_exact text_source_compat plain_multiline_compatibility`；`verify_integration_exact text_source_compat external_element_struct_literal_compiles`；`cargo check --workspace --all-targets --all-features --locked` |
| B-018 | structured style only / no raw controls | `verify_integration_exact text_flow_parity source_controls_are_not_terminal_sequences` |
| B-019 | current-head evidence/coverage | `cargo fmt --all -- --check`; exact CI clippy command；`cargo test --workspace --all-targets --all-features --locked`；CodeCov patch coverage >=80%，TextFlow core tarpaulin report =100% |
| B-020 | Text private source -> existing Element fields | T7：`verify_integration_exact text_source_compat exact_crlf_and_trailing_break_ranges`；`verify_integration_exact text_source_compat structured_source_domain`；T3：`verify_lib_exact layout::engine::tests::alignable_crlf_spans_keep_exact_source_domain`；`verify_lib_exact layout::engine::tests::reconstructed_source_domain_uses_text_content_truth` |
| B-021 | engine/render/App/string typed failure chain | T3 engine gate：`verify_lib_exact layout::engine::tests::try_compute_entrypoints_return_text_flow_error`；T5 crate-private unit gates：`verify_lib_exact renderer::tree_renderer::tests::text_flow_error_preserves_source_and_commits_no_partial_output`、`verify_lib_exact renderer::pipeline::tests::text_flow_error_keeps_previous_vnode`、`verify_lib_exact renderer::pipeline::tests::incremental_failure_retries_from_clean_layout_tree`；T5 public integration gates：`verify_integration_exact text_flow_renderer_error_paths try_render_to_string_preserves_source_and_returns_no_partial_string`、`verify_integration_exact prelude_surfaces try_render_to_string_surface`；T5/T8 从 T3 typed entrypoints 向 renderer、string、App 与其余 callers 传播同一 cause；T8：`verify_integration_exact text_flow_error_paths typed_error_reaches_remaining_callers`、`verify_integration_exact text_flow_error_paths caller_failure_commits_no_partial_output` |
| B-022 | compositor control sanitization | `verify_lib_exact renderer::output::tests::source_controls_are_replaced`；`verify_integration_exact text_flow_parity source_controls_are_not_terminal_sequences` |
| B-023 | uncached viewport projection | `verify_lib_exact renderer::output::tests::active_clips_report_grapheme_visibility`；`verify_lib_exact renderer::output::tests::staged_snapshot_and_write_footprint_are_isolated`；`verify_lib_exact renderer::tree_renderer::projection::tests::projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells`；`verify_lib_exact renderer::tree_renderer::projection::tests::projection_round_trip_validation_is_linear`；`verify_integration_exact text_flow_parity viewport_projection_tracks_overflow_scroll_and_clip`；`verify_integration_exact text_flow_parity overflow_change_recomputes_flow_and_projection` |
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
   `TextFlowInput`；先按 B-020 对齐 legacy multiline compatibility view，只有无法完整、
   无歧义对齐的 mismatch 才标记 Reconstructed。
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
- Performance：完整 source map、staged Output snapshot 与 exact cache key 增加内存/比较
  成本。缓解：immutable logical result 共享、engine-local 有界 cache；projection 每 frame
  只遍历当前 positioned runs/cells，`HashMap` uniqueness + count equality 保持 O(cells)，
  2k/10k diagnostic unit gate 防止退化为 quadratic。GH-61 再建立整体 benchmark 门槛。
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
      atomic publish、interruption、Output trust boundary，以及 tree renderer/pipeline
      crate-private typed error、partial Output 与 previous VNode 状态；T4 exact gates
      `renderer::output::tests::active_clips_report_grapheme_visibility` 验证 nested active
      clips 与 whole-EGC wide 原子可见性，
      `renderer::output::tests::staged_snapshot_and_write_footprint_are_isolated` 验证完整
      staged state 隔离及 target/old wide EGC footprint；T5 exact gate
      `renderer::tree_renderer::projection::tests::projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells`
      验证双向 map、tab same-range、multi-EGC synthetic ellipsis、hidden/hard-break/
      zero-width disposition、malformed map gap、old wide+clip 与 later paint order；
      `renderer::tree_renderer::projection::tests::projection_round_trip_validation_is_linear`
      用 2k/10k cells 与操作计数锁定 O(cells)；tree atomic gate 在 staged failure 后逐字段
      锁定原 Output 不变；T5 pipeline exact gate
      `renderer::pipeline::tests::incremental_failure_retries_from_clean_layout_tree` 验证成功
      旧帧后分别经历新增 NaN child 的 layout failure，以及通过 private test-only renderer
      seam 注入 `MissingCurrentFlow` 的 post-layout render failure；两者修正同 child 后都
      clean full rebuild，且没有重复节点并恢复正确 layout/measure/alias/VNode/runtime。
- [ ] integration：`tests/text_flow_parity.rs` 比较 TextFlow rows、Taffy layout height 与
      Output ANSI/cells，覆盖 plain/rich、width=0/1、width/height resize、scroll/clip
      reprojection、projection 双向 source map 与 source control payload；
      `tests/text_flow_renderer_error_paths.rs` 只验证 public `try_render_to_string*` source
      chain 与无 partial String，剩余 T8 caller failures 由 `tests/text_flow_error_paths.rs`
      覆盖。
- [ ] property：T2-owned `tests/property_tests.rs` 生成合法 Unicode/width/style boundaries，
      断言 logical map 不落在 grapheme 中间且所有 source 恰好一个 logical disposition；
      T5-owned `tests/text_flow_parity.rs::projection_source_cell_round_trip` 生成 viewport/clip
      cases，断言 visible/clipped cell reverse map 不落在 grapheme 中间且 synthetic ellipsis
      不伪装成 source；crate-private projection exact gate 额外证明 current Text 自身
      content rect/overflow/scroll、祖先 clip、terminal bounds 与调用前已有 Output clips
      一起参与 map，任何 occupied-cell gap 都显式失败，并覆盖 x-visible+y-hidden、
      x-hidden+y-visible 与向左/向上滚出 terminal 的 signed negative coordinates。
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
