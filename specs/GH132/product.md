# Product Spec：保留有符号坐标与失败元素身份

## Linked Issue

GH-132: https://github.com/majiayu000/rnk/issues/132

complexity: large

## 用户问题

renderer 当前把浮点屏幕坐标直接转换为整数。Rust 的浮点到整数转换会向零截断，因此
`-0.5` 会变成 `0`：本应位于 viewport 左侧或上方并被裁剪的 grapheme、背景或边框，会错误
绘制在第 0 列或第 0 行。

同一渲染路径在 nested child 的坐标无效时先产生 projection error，随后却用整棵树的 root
ID 填充公共 `TextRenderError::Coordinate`。字符串渲染、dynamic App 和 `TestRenderer`
因此会把真实失败 child 误报为 root，降低诊断可信度。

本规格闭合 PR #119 `discussion_r3651788166` 与 PR #120
`discussion_r3651889684` 的后续行为。它是 GH-58 TextFlow renderer 与 GH-101 closure
evidence 的独立修复合同，不以 umbrella issue 的宽泛描述替代验收。

## 目标

- 为所有参与 frame projection 的有限有符号坐标定义一致、向负无穷方向取整的转换。
- 保证 `(-1.0, 0.0)` 内的坐标保持负向处置并在 viewport/clip 判定时被裁剪，而不是绘制到
  零坐标。
- 保持 `-0.0`、非负小数和整数坐标的既有可见结果。
- 对 NaN、正负无穷和有限范围溢出给出稳定的 typed 分类。
- 让每个坐标错误携带当时正在访问的 exact `ElementId`，并通过字符串、dynamic App 和
  `TestRenderer` caller 保留 typed source chain。
- 让失败 frame 对 Output、projection、VNode、runtime layout evidence 与 flow/layout cache
  保持原子，不发布部分候选状态。

## 非目标

- 不改变 Taffy 布局算法、TextFlow segmentation/wrap/cache key、VNode reconciliation 或
  public `Element` 字段。
- 不接管 GH-124 的 zero-width owner/predecessor 算法；GH-132 只消费其稳定合入后的
  projection ownership 行为。
- 不实现 GH-131 的 VirtualText/span-only canonicalization。
- 不把所有 layout dimension 量化规则重写为新的 snapshot architecture。
- 不新增 terminal protocol、ANSI、授权、网络或持久化能力。
- 不把 malformed flow、writer mismatch 或 injected test failure 伪装成 coordinate error。

## Behavior Invariants

1. **B-001** 每个参与 render projection 的有限有符号浮点坐标，在进入整数写入、可见性或
   clip 判定前，必须按数学 `floor` 转换；同一 frame 不得混用向零截断、饱和和 floor。
2. **B-002** 对任意 `-1.0 < value < 0.0`，转换结果必须为 `-1`。因此 origin 为
   `x=-0.5` 的首个单元必须保持在负列并被裁剪，origin 为 `y=-0.5` 的首行必须保持在负行并
   被裁剪；两者都不得绘制到 `(0, *)` 或 `(*, 0)`。
3. **B-003** `-0.0` 必须与 `0.0` 等价；非负有限小数继续保持现有向下/向零一致的可见
   结果，例如 `0.5 -> 0`、`1.5 -> 1`，整数坐标逐值不变。修复不得使正向内容整体右移或
   下移。
4. **B-004** B-001 的单一规则必须覆盖 x、y、root render offset、当前 layout origin、
   nested ancestor 累积 offset、padding、scroll subtraction、content/border origin、文本
   run offset 和 clip edge；组合计算必须使用 checked arithmetic，x/y 两轴独立判定。
5. **B-005** negative fractional x、y、scroll 组合、ancestor offset 和 own/ancestor clip
   必须在 terminal bounds 与完整 active clip stack 中保持 half-open、signed 处置。任何
   中间步骤不得先 clamp 到 0；只有最终 viewport/clip 交集可以把不可见 cell 分类为
   clipped。
6. **B-006** 每个 `f32` 坐标贡献项必须在组合前单独验证为有限值，并在 `f64` 或更宽的
   domain 按原运算顺序组合；任一中间结果超出 `f32` 可表示范围、floor 后超出内部 signed
   coordinate domain，或任一 checked integer add/sub/extent-edge 计算溢出，都必须返回
   `TextCoordinateError::Overflow`。例如两个有限的 `f32::MAX` 相加必须是 Overflow，
   不得因先在 `f32` 中变成 `+inf` 而误报 NonFinite，也不得 wrap、panic、饱和为成功或
   产生 partial frame。
7. **B-007** 任一坐标贡献项或需要检查的 extent 为 NaN、`+inf` 或 `-inf` 时必须返回
   `TextCoordinateError::NonFinite`；NonFinite 与 Overflow 是互斥、稳定的 typed 分类，
   不得依赖平台 cast 结果。
8. **B-008** coordinate failure 必须记录发生转换或 checked 组合时正在遍历的 exact
   `ElementId`。nested child 的 padding、layout origin、ancestor/scroll 组合、text origin、
   background、border 或该 child 的 TextFlow footprint validation overflow 均归属该 child，
   不得归属 root、parent、sibling 或上一 token。
9. **B-009** root fallback 只允许用于确实没有 element owner 的 tree-level
   malformed/finish failure；`MissingLayout`、`MissingCurrentFlow`、child-known flow
   validation 和所有 coordinate failure 都必须在产生点携带 owner。后续 error conversion
   不得覆盖已有 owner。
10. **B-010** `try_render_to_string*` 失败时必须返回
    `TextRenderError::Coordinate { element_id, source }`，其中 `element_id` 是 B-008 的 child，
    `source()` 可下转为相同 `TextCoordinateError`，且不返回 partial `String`。
11. **B-011** `TestRenderer::try_render_to_ansi` 与
    `TestRenderer::try_render_to_plain` 必须保留与 B-010 相同的 exact child ID 和 typed
    source；对应 fail-loud compatibility wrappers 只能在无副作用后 panic。
12. **B-012** public `App::run` 的 dynamic frame 路径必须把同一
    `TextRenderError::Coordinate` 保留为 `io::Error` source。StaticRenderer 去除 static
    subtree 后，所有保留的 dynamic nodes 必须继续使用 caller 原树的 canonical
    `ElementId`；内部 filter 与 `RenderPipeline::try_render_dynamic_frame` 均不得生成隐藏
    fresh ID 或把 child ID 改为 dynamic root ID。
13. **B-013** 公共错误文本和 source chain 只能包含 element identity、closed error
    classification 与固定安全上下文；不得泄漏 source text、styled spans、frame cells、
    cache contents、terminal bytes 或其他 element 的内容。
14. **B-014** coordinate failure 前即使已在 staged Output 写过 background、border、
    sibling 或 text，caller Output 的 dimensions、cells、grapheme metadata、dirty flags、
    active clips 以及 forward/reverse projection 都必须保持调用前状态；失败 projection
    不得被返回或缓存。
15. **B-015** dynamic candidate 失败时不得提交 candidate VNode、runtime layout/key alias、
    frame string 或候选 flow/layout cache。最后成功的 `previous_vnode` 和 runtime evidence
    保持不变；LayoutEngine 必须保留最后完整可复用状态或显式清空为 clean retry 状态，不能
    暴露半完成 candidate。
16. **B-016** nested traversal 中，parent 或 earlier sibling 已完成 staged paint 后发生的
    child NaN/overflow，也必须遵守 B-014/B-015；错误归属与原子性不得依赖 child 的遍历
    顺序或是否已有可见输出。
17. **B-017** 对同一无效输入重复调用必须稳定返回相同 child ID 与分类，且每次均零提交；
    修正该 child 后重试必须从 clean state 生成完整 frame，不重复节点、cell、projection
    record 或 runtime alias。重复成功 render 不得积累 rounding drift。
18. **B-018** **Reserved / N/A：**GH-132 的 synchronous tree/dynamic renderer 没有
    candidate 创建后、commit 前可达的 cancellation/interruption checkpoint，issue #132
    也不新增该能力。不得用 injected failure 或函数返回后 drop 冒充 cancellation 证据；
    typed render failure 的 candidate discard 与 clean retry 仅由 B-014 至 B-017 验收。
19. **B-019** 独立的字符串/TestRenderer 调用不得共享可变 staged frame 或错误 owner
    context；并发或任意交错执行必须各自得到与串行执行相同的 output/error。App 仍按既有
    单帧发布顺序提交，later frame 不得观察 earlier failed candidate。
20. **B-020** 现有 public 函数签名、`TextRenderError`/`TextCoordinateError` variant、
    `Element` struct shape、正向 fractional snapshots、空文本、缺失 optional scroll
    （等价于 0）以及既有 `MissingLayout`/`MissingCurrentFlow` 分类必须兼容；非 fallible
    wrappers 继续 fail loudly，不允许 blank/old-frame fallback。
21. **B-021** 完成声明必须绑定 implementation PR exact head：negative fractional
    x/y/scroll/ancestor/clip fixtures、nested child NaN/overflow 在 string/dynamic/
    TestRenderer 三类 caller 的 exact fixtures、atomic failure/retry/repetition
    fixtures、全部既有 signed-coordinate/typed-error tests、full Rust gates、coverage、
    CI 与 review-thread evidence都通过。零匹配 filter、旧 SHA 或只看 green rollup 不算
    证据。

Revision note：B-018 在本轮 review 后由“外层 cancellation fixture”收窄为显式
Reserved/N/A。原因是 issue #132 的真实 scope只有signed conversion、owner propagation与
failure atomicity，当前同步pipeline也没有可达的pre-commit cancellation checkpoint；该ID
不重用于其他行为，B-014至B-017继续承担全部typed failure原子性与重试验收。

## 验收标准

- [ ] `-0.5` 的 x/y origin、`0.5 - scroll(1)`、负 fractional ancestor 累积和负
      fractional clip 组合均保留负 disposition；首 cell 被 clipped 而不是绘制到零坐标。
- [ ] `-0.0`、`0.5`、`1.5`、整数和正向 clip snapshots 与当前兼容；x/y、Hidden/Scroll、
      terminal bounds 与 nested active clips 的组合互不串轴。
- [ ] NaN、`+inf`、`-inf` 和有限 out-of-range/checked arithmetic overflow 分别返回
      `NonFinite` 或 `Overflow`，不 panic、不饱和成功。
- [ ] root→parent→child fixture 中，由 child 触发的 NaN 和 overflow 在
      `try_render_to_string*`、dynamic App pipeline、`TestRenderer::try_render_to_ansi/plain`
      均报告 exact child ID，并保留 typed `Error::source` chain。
- [ ] 失败发生在已有 staged background/sibling/text 之后时，Output、projection、VNode、
      runtime evidence 与 cache 均无 partial commit；重复失败、修正后重试和独立交错
      render 均确定。
- [ ] implementation exact head 的 focused tests、fmt/check/clippy/full tests、coverage、
      CI、独立 review 和 unresolved current reviewThreads gate 全部满足 B-021。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-020；空文本与缺失 optional scroll 保持兼容，缺失 layout/flow 保持既有 typed 分类 |
| 错误与失败路径 | covered: B-006、B-007、B-008、B-009、B-010、B-011、B-012、B-013、B-014、B-015、B-016 |
| 授权/权限 | N/A：renderer 是本地纯计算/terminal projection，不读取权限或执行外部动作；错误上下文安全由 B-013 约束 |
| 并发/竞态 | covered: B-014、B-015、B-019 |
| 重试/重复/幂等 | covered: B-017 |
| 非法状态转换 | covered: B-009、B-014、B-015、B-016；unowned/scoped error 与 staged/committed 状态不得非法转换 |
| 兼容/迁移 | covered: B-003、B-020 |
| 降级/回退 | covered: B-002、B-006、B-007、B-009、B-020；禁止 clamp、blank、old-frame 或 root-ID silent fallback |
| 证据与审计完整性 | covered: B-021 |
| 取消/中断/部分完成 | cancellation/interruption N/A：同步 renderer 无 pre-commit checkpoint，GH-132 不新增 API；typed failure 的部分完成由 B-014、B-015、B-016、B-017 覆盖，B-018 保留为显式 scope revision |

## 发布说明

这是 correctness-only renderer 修复。用户不需要迁移 API：负 fractional 内容会从错误的
viewport-edge paint 改为正确 clipped，nested coordinate diagnostics 会显示真实 child
`ElementId`。现有正坐标、公共签名和 typed variants 保持不变。

issue 当前误挂 `ready_to_implement`，但本 packet 在起草前不存在；SpecRail route artifact
已将真实路线判为 `ready_to_spec -> write_spec`。本 spec PR 只使用 `Refs #132`。任何实现
必须等待人工 spec review/approval、canonical implementation gate，以及 Tech Spec 声明的
PR #137/#131 ownership refresh；本规格不修改 label，也不自动授权实现。
