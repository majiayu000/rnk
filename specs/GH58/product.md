# Product Spec：统一终端文本测量与绘制流

## Linked Issue

GH-58: https://github.com/majiayu000/rnk/issues/58

complexity: large

## 用户问题

当前 `rnk` 会在布局阶段按可用宽度计算文本换行高度，但绘制阶段仍把原始文本或原始
styled spans 直接写到单行 `Output`。一旦遇到行末或显式换行，写入会停止，因而可能出现
“布局已经为多行预留高度、实际内容却被截断”的可见错误，并让后续组件错位。

终端 AI Chat 会持续显示长文本、Markdown 派生的富文本、CJK、emoji、组合序列和窄宽
resize。用户需要一个唯一、确定、grapheme-safe 的 TextFlow 结果同时驱动测量与绘制；
调用方还需要可靠的源文本到终端 cell 映射，以便后续 Composer 光标、选择和消息复制不再
各自重写 Unicode 换行算法。

本规格是 GH-58 的独立产品合同。它完整定义 TextFlow 的用户可见行为和实现门，不以
GH-57 umbrella 文档替代自身验收。

## 目标

- 让普通文本和 styled spans 的测量、逐行定位与绘制消费同一个不可变 TextFlow 结果。
- 明确定义 grapheme、Unicode display width、显式换行、tab、wrap、truncate、ellipsis
  和 overflow 的终端 cell 语义。
- 对 width=0/1、宽字符边界、clip、resize 和缓存重用给出确定且无越界的行为。
- 在 `Text::new` / `Element::text` 的原始字符串被 `str::lines()` 或 rich-text 拼接归一化前
  保留 source bytes，再输出 source UTF-8 byte range / grapheme 与可见 cell 的双向映射。
- 保留现有 `Text`、`Line`、`Span`、`TextWrap`、`Overflow` 与低层 `Output::write` 使用方式，
  通过兼容适配逐步收敛，不要求应用一次性迁移。
- 用回归、property、渲染 parity 与覆盖率证据证明内容不再静默丢失。

## 非目标

- 不改写 Taffy 的 Flexbox 算法、keyed identity、child order 或事务式 patch；它们分别属于
  GH-59 至 GH-61。
- 不实现 `ChatComposer`、`MessageList`、消息模型、shell 或 provider adapter。
- 不解析或透传嵌入字符串的 ANSI escape，不允许字符串内控制码替代结构化 `Style` / `Span`；
  renderer 自身基于结构化 Style 生成的 allowlisted ANSI 不属于原始文本。
- 不定义终端数据库、跨进程缓存或全局持久化。
- 不在 GH-58 中承诺所有终端对 ambiguous-width 字符具有相同物理显示；本次只保证同一
  已选 Unicode width policy 被测量与绘制一致使用。
- 不以词典断词、连字符或自然语言排版替代确定的 grapheme 边界换行。

## Behavior Invariants

1. **B-001** 当一个 Text element 参与布局和绘制时，系统必须只生成一个不可变的
   TextFlow 结果；测量读取它的 row count / maximum row width，绘制读取同一结果的逐行
   positioned styled runs。不得在布局与 renderer 中保留两套独立换行算法。
2. **B-002** 普通 `text_content` 与 `Line` / `Span` 富文本必须经过同一 flow 规则；跨
   span 换行后每个 grapheme 保留其结构化样式，样式边界本身不得改变文本宽度、丢字或重复字。
   若样式边界落在一个 grapheme 内，flow 必须按文档化规则归一到该 grapheme 的首个 source
   style，并记录可诊断的 normalization，而不是拆开 grapheme 或返回 error；combining
   sequence 与 emoji ZWJ 两类 split-style 输入都属于受支持输入。
3. **B-003** 缺失文本和空字符串均显示为空且不捏造内容；一个存在但为空的 Text element
   产生一个高度可确定的空 logical row。空 span、空 line 与相邻空 style run 不得导致额外
   cell、无限循环或崩溃。
4. **B-004** 在 B-020 保留的 exact source 上，`\n`、`\r\n` 与独立 `\r` 均作为 hard
   line break，CRLF 只能计为一次；连续 hard breaks 保留中间空行。为兼容现有可见行合同，
   末尾 hard break 默认不额外生成最终空行；source map 仍必须以原始 byte range 记录该
   break 已被消费，不能从归一化后的 `\n` 猜测原字节。
5. **B-005** 所有分段、换行、截断和 cell 定位必须以 extended grapheme cluster 为最小
   单位，并使用同一 Unicode width policy；CJK、emoji ZWJ、variation selector、combining
   sequence 和零宽 grapheme 不得被拆分、重复或使 measure/render 使用不同宽度。
6. **B-006** tab 不得作为原始控制字符写入 Output；每个 `\t` 必须从当前 logical row 的
   cell column 扩展到下一个固定 tab stop。本版本默认 tab stop 为 4，tab 配置必须大于 0，
   且展开后的合成空格映射回原 tab source range。
7. **B-007** 当 `TextWrap::Wrap` 且可用宽度大于 0 时，flow 按 grapheme 贪心填充每行；
   超长单词不依赖空格即可续写，原空格不折叠，styled run 可跨行拆分但 source 顺序不变。
8. **B-008** `Truncate` 与 `TruncateEnd` 均在 logical line 尾部截断，
   `TruncateStart` 从开头截断，`TruncateMiddle` 从中部截断；只有确实省略 source grapheme
   时才显示默认 `…`。ellipsis 本身也按 grapheme/cell 宽度裁到可用宽度，synthetic
   ellipsis cell 不得伪装成 source byte range。
9. **B-009** width=0 时 flow 必须终止、产生确定的 logical row 数且不定位任何可写 cell；
   width=1 时宽度为 2 的 grapheme 不得拆分或越界。`Overflow::Visible` 可保留完整的
   overwide positioned run 供外层终端边界裁剪；`Hidden` / `Scroll` 必须把它标记为 clipped
   并消费 source，不能卡住后续 flow。
10. **B-010** TextFlow 始终先产出完整 logical rows/runs；实际 `overflow_x/y`、scroll offset、
    content rect、祖先 clip stack 与 terminal bounds 由当前 frame 的 render projection 决定
    visible/clipped cells。projection 只能隐藏 cell，不得改变测量 row count、source order
    或把被裁内容报告为已绘制。
11. **B-011** cached logical map 中每个 source grapheme 必须能查询其在 B-020 source domain
    中的 UTF-8 byte range、logical row 和
    `{positioned | truncated | hard-break | zero-width | sanitized-control}` disposition；当前
    frame 的 render projection 再给出 `{visible cells | clipped}`，每个可见 cell 也必须能
    反查 source grapheme 或 synthetic ellipsis。两层映射均不得指向 grapheme 中间字节。
12. **B-012** logical flow cache key 必须精确包含 source bytes、line/span 结构及全部
    structured run style 值、`overflow_x/y`、可用内容宽度、`TextWrap`、tab stop、ellipsis
    与 Unicode width policy/revision。
    任一项改变必须重算；不得只依赖可能碰撞的 hash 或 Element frame ID 判定相等。viewport
    height、scroll offsets、content rect、clip stack 与 terminal bounds 不进入 logical cache，
    而必须每 frame 参与 B-023 render projection。`overflow_x/y` 虽不改变 logical row
    geometry，仍按 linked issue 的验收语义使完整 TextFlow 结果失效并重算，随后以新结果重建
    projection；不得只复用旧 logical flow 并改 projection。
13. **B-013** 当 B-012 的所有输入完全相同时允许复用缓存，复用结果必须与重新计算逐字段
    等价且不可被调用方修改。重复 measure/render 不得追加 run、改变 dirty cells 或产生
    第二份不同 source map。
14. **B-014** terminal resize 改变任一 Text element 的可用 content width，或
    `overflow_x/y` 单独改变时，该 element 的 logical flow 必须在同一 frame 失效并重算；
    仅高度、scroll 或 clip 改变时允许复用 logical flow，但必须在同一 frame 重建
    projection。layout row count 必须等于 logical rows，实际输出必须等于 projection 的当前
    visible rows；renderer 不得使用上一 viewport 的旧 visibility。
15. **B-015** flow 计算或缓存发布失败时，纯 TextFlow 层必须返回 `TextFlowError`，不得发布
    部分结果；renderer 消费缺失/错误 flow 时不得回退到当前“只写第一行”的旧路径并显示为
    成功，上一帧缓存也不得被错标为当前输入的有效结果。非 grapheme-boundary error 仅适用于
    normalization 后由 engine 生成的 finalized token/map range 违反内部不变量，不适用于
    B-002 可归一化的输入 style boundary。end-to-end caller 传播由 B-021 定义。
16. **B-016** 多个只读消费者可共享一个已完成的不可变 flow；计算被取消、中断或失败时，
    cache 只能保留中断前已完整发布且 key 仍匹配的结果，不能暴露正在构建的 rows/source map。
17. **B-017** 现有 `Text` / `Line` / `Span` 构造器、`Text::wrap`、布局测量 helper、
    `Output::write` 调用方及外部完整 `Element { ... }` struct literal 必须继续编译；helper
    与 renderer 通过 TextFlow 或兼容 wrapper 收敛。若确需改变已测试的 trailing-newline
    或 truncate 行为，必须另行走有期限的弃用与迁移流程，本 issue 不静默破坏。
18. **B-018** 字符串中的 ANSI escape 或其他控制序列不得被 TextFlow 解释为颜色、光标移动
    或授权，也不得由 Output 原样进入 terminal byte stream；只有结构化 `Style` / `Span`
    控制样式。具体 replacement 与 allowlist 合同由 B-022 定义。
19. **B-019** GH-58 只有在当前 implementation head 的正例、边界、失败、property 与渲染
    parity 测试通过，新代码 patch coverage 至少 80%，TextFlow 核心 segmentation/wrap/
    truncate/cache 分支达到 100% 时才可声明完成；零匹配 filter、旧 SHA 或视觉演示不算证据。
20. **B-020** `Text::new(String)` 必须在任何 `str::lines()`、CRLF 归一化或 `Line` 重建前
    把 exact input 保存在 Text 私有状态，并在 `into_element` 时原样写入既有
    `text_content`；`Element::text(String)` 已直接保存 exact input。`Text::spans` / `line` /
    `from_lines` 则生成一次 canonical source 写入 `text_content`，`spans` 仅提供 exact style
    ranges。Element clone 依靠既有字段保留 source；字段不一致时以当前 `text_content` 为
    source truth 并标记 `Reconstructed`，不得声称恢复已丢失的 CRLF 或 trailing break。
21. **B-021** TextFlow failure 必须沿一条具体 typed boundary 传播：LayoutEngine 的新增
    `try_compute*` 返回 flow error；tree/element renderer、static/dynamic pipeline 和新增
    public `try_render_to_string*` 返回包含 source `TextFlowError` 的 `TextRenderError`；
    `App::run` 路径把同一 error/source 链映射为失败的 `io::Result`。现有返回 `()` / tuple /
    `String` 的兼容 wrapper 可以保留签名，但遇到 flow error 必须 fail loudly，不能返回空白、
    第一行、部分 output 或旧 frame。T5 必须先新增 `try_*` Result variants 并让所有现有 caller
    继续调用签名不变的 fail-loud wrapper，使 T5 自身可编译测试；T8 再把 App/static/terminal/
    test callers 切到 `try_*` 完成 recoverable 传播。GH-58 不接管 GH-60 通用 Taffy/patch error。
22. **B-022** hard-break tokenizer 必须先消费 LF/CRLF/CR，tab expander 必须先消费 `\t`；
    到达 compositor 的 source scalar 若属于 ESC、其余 C0（U+0000–U+001F）、DEL 或 C1
    （U+0080–U+009F），必须被 terminal-safe replacement 代替且仍映射回原 source range：
    C0/ESC 用对应 control picture，DEL 用 `␡`，C1 用 `�`。`Output::write` 的低层入口也必须
    执行相同防线，绝不能把 source 的 ESC/C0/C1 bytes 写入 terminal；只有 renderer 根据
    结构化 Style/terminal protocol 生成的 allowlisted ANSI 序列可进入最终输出。
23. **B-023** render projection 是 frame-local、不可跨 viewport 缓存的结果；它必须以
    当前 logical flow、`overflow_x/y`、scroll offsets、content rect、完整 clip stack 与
    terminal bounds 为输入，原子地产生 visible/clipped source dispositions 和 cells。
    `overflow_x/y` 改变必须先按 B-012 重算 flow 再重投影；height-only resize、scroll
    或任一祖先 clip 改变可以复用 logical flow，但仍必须重投影，不能复用上一 frame 的
    visibility。
24. **B-024** GH-58 不给 public `Element` 增加任何 required field，也不把它改为
    `#[non_exhaustive]`。`Text` 的私有 source state 在 `into_element` 时写入既有
    `text_content`：plain input 保留 exact bytes，structured input 写 canonical bytes；
    `spans` 只提供 style structure。现有外部完整 struct literal、`Element::text` 和 clone
    继续编译；spans 与 text_content 不一致时以当前 text_content 为 source truth并标记
    `Reconstructed`，不得增加全局 sidecar 或隐藏 required field。

## 验收标准

- [ ] 普通文本和跨 style spans 的长文本在多个宽度下，TextFlow row count、布局高度和
      Output 实际非空行逐项一致；style boundary 分别切入 combining sequence 与 emoji ZWJ
      时归一到首 source style、产生 diagnostic 且不返回 error，覆盖 B-001、B-002、B-007。
- [ ] fixtures 覆盖空/缺失、LF/CRLF/CR、连续与末尾换行、tab、长单词、CJK、emoji ZWJ、
      variation selector、组合序列与零宽 grapheme，证明 B-003 至 B-006。
- [ ] wrap、四种 truncate 语义、ellipsis、width=0/1、宽字符边界和
      `Visible` / `Hidden` / `Scroll` snapshots 证明 B-007 至 B-010。
- [ ] source-to-cell / cell-to-source round-trip property test 从不返回 grapheme 中间字节，
      并对 truncated、clipped、hard-break、zero-width 和 synthetic ellipsis 给出明确结果，
      证明 B-011；`Text::new("a\r\nb\r\n")` 的 source map 必须指向原始 CRLF byte ranges，
      trailing break 在不生成最终空行时仍可查询，证明 B-004、B-020。
- [ ] 内容、span/style 结构、宽度、wrap、`overflow_x/y`、tab、ellipsis 或 Unicode
      policy 分别改变时 cache miss；overflow-only 变更同时证明 flow 重算与 projection
      重建，height/scroll/clip 单独改变时 logical cache 可 hit 但 projection 必须改变，
      证明 B-012、B-013、B-014、B-023。
- [ ] 连续窄/宽 resize、计算失败注入、取消/中断和重复 render 测试无旧 flow、部分 flow、
      越界写入或第一行 fallback；negative fixtures 分别从 LayoutEngine、dynamic/static App
      路径和 `try_render_to_string*` 观察同一 typed cause，证明 B-014 至 B-016、B-021。
- [ ] 现有 public surface 与外部完整 `Element` struct literal 编译测试、原有
      text/output/layout 回归全绿，证明 B-017、B-024；ESC screen-clear/cursor/OSC、C0/C1
      payload 被可见替换且 source map 保留原 range，最终 terminal stream 不含 payload
      控制序列，证明 B-018、B-022。
- [ ] 当前 head 的全量 CI、独立 review、review threads、SpecRail PR gate 和覆盖率证据
      满足 B-019。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-003、B-009 |
| 错误与失败路径 | covered: B-002、B-015、B-019、B-021 |
| 授权/权限 | N/A：TextFlow 是纯本地布局计算，不读取权限、不执行工具；source controls 被替换且只有结构化 ANSI 可输出（B-018、B-022） |
| 并发/竞态 | covered: B-013、B-014、B-016、B-023 |
| 重试/幂等 | covered: B-013、B-015、B-016 |
| 非法状态转换 | N/A：TextFlow 无业务状态机；未完成结果不得发布由 B-015、B-016 约束 |
| 兼容/迁移 | covered: B-004、B-017、B-020、B-021、B-024 |
| 降级/回退 | covered: B-009、B-010、B-015 |
| 证据与审计完整性 | covered: B-019 |
| 取消/中断/部分完成 | covered: B-015、B-016 |

## 发布说明

GH-58 首先在现有 `rnk::layout` 与 renderer 内引入 TextFlow；不创建独立 crate，也不移除
现有文本 helper。用户可继续通过 `Text` / `Line` / `Span` 构造内容，测量与绘制会自动共享
新结果；需要恢复的调用方可使用 typed `try_render_to_string*`，App 运行路径会返回保留
TextFlow cause 的 I/O error。发布说明必须明确 exact/canonical/reconstructed source domain、
terminal-safe control replacement、logical cache 与 frame projection 边界、默认 tab stop、
trailing hard-break、truncate/ellipsis、width=0/1 及 Unicode width policy 边界。`Element`
struct literal 无迁移要求。后续 GH-64 / GH-65 可消费 source map 与 row count，但不得复制
TextFlow 算法。
