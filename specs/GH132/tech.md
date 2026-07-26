# Tech Spec：有符号坐标量化与元素级错误归属

## Linked Issue

GH-132: https://github.com/majiayu000/rnk/issues/132

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-spec-packet-changes
{"version":1,"issue":132,"complete":true,"paths":["specs/GH132/product.md","specs/GH132/tech.md","specs/GH132/tasks.md"]}
-->
<!-- specrail-planned-changes
{"version":1,"issue":132,"complete":true,"paths":["src/renderer/app.rs","src/renderer/error.rs","src/renderer/pipeline.rs","src/renderer/static_content.rs","src/renderer/tree_renderer.rs","src/renderer/tree_renderer/projection.rs","src/renderer/tree_renderer/projection/staged.rs","src/renderer/tree_renderer/projection/tests.rs","src/renderer/tree_renderer/projection/tests/coordinates.rs","src/renderer/tree_renderer/tests.rs","src/renderer/tree_renderer/tests/coordinates.rs","tests/text_flow_renderer_error_paths.rs"],"spec_refs":["specs/GH132/product.md","specs/GH132/tech.md","specs/GH132/tasks.md","specs/GH58/product.md","specs/GH58/tech.md","specs/GH58/tasks.md"]}
-->

`specrail-spec-packet-changes` 只约束本 spec PR 的三文件 diff；
`specrail-planned-changes` 是后续 implementation PR 的独立 12 路径 manifest。实现 closure
只比较后者，不要求或允许重改已经合入 main 的 packet。

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-132 的 signed coordinate conversion、coordinate error owner、三类 caller
传播和原子性验证。GH-124 继续拥有 zero-width prospective/actual footprint 与 predecessor
selection；GH-131 继续拥有 VirtualText/span-only compatibility。GH-132 不复制或顺带修复
两者。

issue #132 当前 label 是 `ready_to_implement`，但起草前缺少本 packet。implx auto 的
creation-time route artifact `gh132-route-gate.json` 明确给出
`current_state=ready_to_spec`、`route=write_spec`、`decision=allowed`。该 artifact 只授权
写 spec；它不是后续实现的可执行 gate 依赖。本 PR 不改 label，implementation 仍等待人工
spec approval，并按 Tasks 中文档化的 `SPEC_RAIL_ROOT` fail-closed 运行 fresh
implementation gate。

## Codebase Context

以下锚点均在 `main@b4f39ed53506a42b7c06d0b0222bf3ac2c3e5ad8` 通过 Read/grep
核实。实现若在 dependency 合入后开始，必须在 refreshed base 上重新定位行号和签名。

| Area | Current anchor | Current behavior | GH-132 decision |
| --- | --- | --- | --- |
| Clip bounds | `src/renderer/tree_renderer.rs:41`, `src/renderer/tree_renderer.rs:49`, `src/renderer/tree_renderer.rs:101` | `ClipBounds` 过早使用 `u16`；`clip_bound` 对负数 clamp 0，对正小数直接 cast | clip edge 先保留 signed/floor 结果，与 viewport/active clips 求交后才转换为 Output clip |
| Tree coordinate conversion | `src/renderer/tree_renderer.rs:153`, `src/renderer/tree_renderer.rs:169`, `src/renderer/tree_renderer.rs:200`, `src/renderer/tree_renderer.rs:218`, `src/renderer/tree_renderer.rs:342` | `signed_coord` 对有限值使用 `value as i64`，`-0.5` 向零变 0；helper 不接收 current element | 改为 element-scoped、range-checked floor helper；所有 recursive call site 传当前 `element.id` |
| Extent handling | `src/renderer/tree_renderer.rs:113` | non-finite extent 返回 unscoped NonFinite；finite negative/oversized extent 按既有规则 clamp 0/u16::MAX | 保持 finite extent compatibility，但 non-finite 与随后 checked edge overflow携带当前 element ID |
| Text/scroll composition | `src/renderer/tree_renderer.rs:189`, `src/renderer/tree_renderer.rs:202`, `src/renderer/tree_renderer.rs:218` | text origin 用 signed x/y、content rect、padding 与 integer scroll checked add/sub；child offset 继续以 f32 累积 | 在现有 semantic boundaries 使用同一 scoped floor helper，保持 positive behavior；每个 checked failure归属当前 element |
| Border/background paint | `src/renderer/tree_renderer.rs:177`, `src/renderer/tree_renderer.rs:247`, `src/renderer/tree_renderer.rs:331` | staged fill/paint 可在 checked arithmetic 中返回无 owner Overflow | background、border 与 paint helper 显式携带 owner，staged failure不回落到 root |
| Projection error/flow validation | `src/renderer/tree_renderer/projection.rs:127`, `src/renderer/tree_renderer/projection.rs:174`, `src/renderer/tree_renderer/projection.rs:256`, `src/renderer/tree_renderer/projection.rs:311` | `NonFiniteCoordinate`/`CoordinateOverflow` 没有 ID；`validate_tree_flows` 已知 child ID，但 `validate_flow`/`validate_row_footprints` 丢失它，overflow最终使用 root fallback | coordinate variants在产生点携带 `ElementId`；flow validation签名逐层传当前 child ID；root fallback只服务真正无 owner 的 non-coordinate malformed/finish failure |
| Projection transaction | `src/renderer/tree_renderer/projection.rs:228`, `src/renderer/tree_renderer/projection.rs:241`, `src/renderer/tree_renderer/projection.rs:250` | 先 validate，复制 staged Output，finish/round-trip成功后一次 `commit_staged` | 保持一次 publish boundary；coordinate failure前后的 projection builder永不逃逸 |
| Staged coordinate arithmetic | `src/renderer/tree_renderer/projection/staged.rs:46`, `src/renderer/tree_renderer/projection/staged.rs:71`, `src/renderer/tree_renderer/projection/staged.rs:187`, `src/renderer/tree_renderer/projection/staged.rs:212`, `src/renderer/tree_renderer/projection/staged.rs:266` | flow token有 `ProjectionId`，但 base add、paint/fill/checkpoint overflow仍生成无 owner variant | flow使用 `id.element_id`；background/border入口显式传 owner；所有 checked failures构造 scoped coordinate error |
| Public error surface | `src/renderer/error.rs:30`, `src/renderer/error.rs:48`, `src/renderer/error.rs:97`, `src/renderer/error.rs:137` | `TextCoordinateError::{NonFinite,Overflow}` 与 `TextRenderError::Coordinate` 已存在，`source()` 保留 typed cause；Display只含 ID/classification | 不增 public variant/字段；增加 safe display/source-chain regression，证明不泄漏 frame/source contents |
| String caller | `src/renderer/render_to_string.rs:42`, `src/renderer/render_to_string.rs:119`, `src/renderer/render_to_string.rs:195` | public `try_render_to_string*` 已返回 `TextRenderError`；Output仅在成功后转 String | 无生产改动；在既有 integration test 文件验证 nested child ID、typed cause与无 partial String |
| TestRenderer caller | `src/testing/renderer.rs:48`, `src/testing/renderer.rs:59`, `src/testing/renderer.rs:89` | public fallible plain/ANSI APIs共享 tree renderer；compat wrappers fail loudly | 无生产改动；通过 public integration fixture验证 child NaN/overflow ID与clean retry |
| Dynamic candidate | `src/renderer/pipeline.rs:37`, `src/renderer/pipeline.rs:71`, `src/renderer/pipeline.rs:98`, `src/renderer/pipeline.rs:105` | render/layout失败时重置 LayoutEngine；runtime evidence与 `previous_vnode` 只在 render成功后发布 | 增 nested child exact-ID、candidate state/cache、repeat/corrected retry tests；保持既有 commit顺序 |
| App/static identity | `src/renderer/app.rs:248`, `src/renderer/app.rs:269`, `src/renderer/app.rs:281`, `src/renderer/static_content.rs:140`, `src/core/element.rs:266` | public `App::run` 保留 typed source，但 dynamic filter递归 `Element::clone`，而 Clone为每个保留节点生成 caller不可知的 fresh ID | `static_content.rs` 构造identity-preserving filtered tree：保留节点的 filtered ID逐字等于caller original ID；App exact test用过滤前child ID断言I/O source |
| Existing projection tests | `src/renderer/tree_renderer/projection/tests.rs:343`, `src/renderer/tree_renderer/projection/tests.rs:488` | 已覆盖整数 negative scroll、axis clips、nested active clip和 injected failure atomicity | 扩展 exact fractional x/y/scroll/ancestor/clip、range边界和 scoped failure fixtures |
| Existing caller tests | `tests/text_flow_renderer_error_paths.rs:1`, `tests/text_flow_error_paths.rs:1` | string测试只覆盖 invalid tab stop；TestRenderer测试只检查 root padding NaN且未断言 exact ID | GH-132只修改前者并集中三类 public-facing fixture；后者保持 regression，不创建重复测试文件 |
| Test file size | `src/renderer/tree_renderer/tests.rs:1`, `src/renderer/tree_renderer/projection/tests.rs:1` | 起草时分别为 664/774 行；继续内联全部 GH132 fixtures会触及 800 行 hard ceiling | 两个既有文件只增加 `mod coordinates;`，fixture分别放入新 `tests/coordinates.rs` 子模块；不得压缩旧测试或使用 rustfmt skip |

## 设计方案

### 1. Implementation 与 ownership gate

所有 SpecRail 命令固定到 upstream
`https://github.com/majiayu000/specrail.git` revision
`bfc60f26164af5df1ebd3b5cb79d07379fc416b7`。执行环境提供的 `SPEC_RAIL_ROOT` 必须是该
exact detached checkout；T1/T5分别校验 `route_gate.py` SHA-256
`d77cad0763713ca589be1c4278edcec7c90c017bc383fd6a7976402be22a7433` 和
`pr_gate.py` SHA-256
`10cb7412ff504291d136a2c1486bc96e6b5e811c8040d1f61a8d222994e87873`；
T1另外校验 `check_workflow.py` SHA-256
`c5bd73060037b0e8febace0e5ee8473e17973e1ca17257ea1517a94e05fa7549` 和
`github_duplicate_evidence.py` SHA-256
`eab228a33d84a43cde1ba3587d5edde50993ae11c5c5a522ee8d01b64b284d55`。
路径、revision或hash不符均fail closed，不从机器特定绝对路径或mutable branch运行。
由于route gate会把artifact路径约束在`--repo`内，T1从固定revision导出临时workflow
mirror，再将当前merged GH132 packet复制到mirror的规范路径；不得把目标仓库绝对路径
传给以SpecRail checkout为`--repo`的route gate。T5的PR gate还需要读取implementation
git history中的approved spec revision，因此以exact PR head创建临时只读验证clone，并只
overlay同一固定revision的`workflow.yaml`、`states.yaml`、`labels.yaml`和`schemas/`；
`--repo`指向该clone，不能指向没有目标历史的SpecRail checkout。

开始任何实现 edit 前必须同时满足：

1. GH-132 spec PR 已 merged，存在 human approval，issue 有唯一 canonical
   `ready_to_implement`，fresh route gate 对 `implement` 返回 `allowed`。在首次 source/test
   edit前必须fetch `origin/main`，把解析出的 exact SHA记录为
   `GH132_IMPLEMENTATION_BASE_SHA`，并同时证明worktree porcelain为空且`HEAD`逐字等于该
   SHA；只证明某个旧SHA是ancestor不满足此 gate。
2. PR #137（GH-124）在本 packet 最初的 `b4f39ed...` anchor之后，于
   `2026-07-26T08:36:49Z` 合入 main；final head
   `4d135668943e06aaefb8ffffe7f8267337fc9d19`、merge commit
   `84a7492ecff9a5ae560cf7627438909282558f2a` 直接修改
   `src/renderer/tree_renderer/projection/staged.rs` 与
   `src/renderer/tree_renderer/projection/tests.rs`，并新增zero-width子模块。fresh expected
   main必须包含该merge；实现前重新定位zero-width owner contract，并逐条重跑两项已命名
   exact regression且证明`matched=1`。
3. issue #131 当前无 spec、branch 或 PR，但其 VirtualText traversal/flow validation scope
   预计与 `tree_renderer.rs`、`projection.rs` 和 caller tests 冲突。coordinator 必须先取得
   #131 的 frozen manifest：若与本文件12路径implementation manifest相交，则两者串行
   实现；不得以“尚无分支”视为无 owner。GH-132 不修改 VirtualText/span source behavior。

上述 gate 是 implementation dependency/refresh gate，不阻止本 spec-only PR。记录的
PR #137 merge证据必须在实现时fresh查询并证明属于上述exact expected main，不能只复用
本规格中的SHA文本。closure时再次fetch current main：PR `baseRefOid`必须与其逐字相等，
该SHA必须是PR head的exact merge-base，implementation diff再与12路径manifest精确比较。

### 2. Scoped floor conversion

在 `tree_renderer.rs` 保留一个权威、element-scoped coordinate accumulator。概念接口为：

```text
CheckedCoordinate::from_f32(element_id, operand)
  .add_f32(operand)
  .sub_integer(operand)
  .floor_i64()
  -> Result<i64, ProjectionError>
```

这表示一个私有的单一实现入口，不要求逐字采用上述名称：

- 每个来自 root offset、layout、ancestor、padding 或其他坐标源的 `f32` operand 都必须在
  任何算术前单独用 `is_finite` 验证。原始 NaN/`+inf`/`-inf` 分类为 scoped NonFinite。
- 通过验证的 operand 转为 `f64`（或更宽的精确 domain）后才按源表达式的原顺序 add/sub；
  禁止保留 `offset_x + layout.x - scroll` 这类先在 `f32` 中运算再交给 helper 的路径。
- 每一步 wider-domain 组合后都检查 `[-f32::MAX, f32::MAX]`。有限 operands 的结果一旦
  超出该范围，或 wider arithmetic 自身产生非有限值，分类为 scoped Overflow，而不是
  NonFinite；即使后续相反项可能抵消也不得继续。
- 最终值执行 `floor`，再用精确 half-open signed bound `[-2^63, 2^63)` 检查。下界可接受，
  上界 `2^63` 必须拒绝；不得依赖 saturating cast。通过后才转 `i64`。
- `-0.0` floor/cast 为 0；非负小数结果与现状相同。整数 content/scroll/edge 组合继续使用
  `checked_*`，并通过同一 accumulator 的 scoped Overflow constructor 报错。
- finite extent 的既有 clamp policy不变；只有 non-finite extent与计算 edge overflow进入
  scoped coordinate error，避免把 GH-132 扩成 layout dimension迁移。

递归 offset 必须以这个 wider accumulator/值传递，不能降回 `f32` 后再供下一层组合。不得
创建第二个 alias helper。所有 x/y/root offset/layout/ancestor/padding/scroll/text/
background/border call site经同一权威 accumulator或其 scoped checked integer操作。

### 3. Signed clip 与组合数据流

`ClipBounds` 在 renderer 内部改为 signed half-open edges。own clip 的 origin使用 B-001
floor 后的 signed origin，content/border integer inset和extent用 checked add得到 `[x1,x2)`
与 `[y1,y2)`。ancestor intersection继续取 max lower/min upper，但允许整个 rect在负坐标：

```text
each f32 layout/ancestor operand
  -> element-scoped finite validation
  -> ordered f64/wider composition + per-step f32-range Overflow check
  -> floor + signed-domain check
  -> checked signed origin/content/scroll composition
  -> signed own clip ∩ signed ancestor clip ∩ Output active clip ∩ terminal viewport
  -> checked ClipRegion only at Output boundary
  -> StagedFrame prospective write
  -> visible | clipped projection disposition
```

单轴 `Overflow::Visible` 继续使用 viewport对应轴而不是捏造 clip；Hidden/Scroll只收紧自己
轴。负 clip edge不得提前变 0，否则会丢失“首 token在 -1”证据。`StagedFrame` 和 Output
继续使用 signed prospective coordinates决定 cell是否 clipped。

### 4. Element owner 与 public error flow

内部 coordinate variants携带 `{ element_id, source: TextCoordinateError }`，或等价的两个
scoped variants。owner在产生点确定：

- tree recursion 的 origin、extent、padding、scroll、background、border使用当前
  `element.id`；
- TextFlow token add使用 `ProjectionId.element_id`；
- checked child offset在访问 child 时若失败，使用该 child ID，而不是 parent/root；
- child-known TextFlow validation 不得丢失 owner，具体调用链固定为
  `validate_tree_flows(element, engine) -> validate_flow(element.id, flow) ->
  validate_row_footprints(element.id, rows)`；最后一个 `checked_add` 失败必须直接构造
  `CoordinateOverflow(element.id)`；同一flow validation内的`MalformedFlow`也映射为该child
  的`IncompleteSourceMap`，不得回退root；
- test-only forged validation rows也必须显式携带目标 child ID，例如
  `ProjectionOptions::validation_rows: Option<(ElementId, Vec<TextFlowRow>)>`，不得靠
  conversion时的root参数补 ID；exact test必须把该错误转换到public
  `TextRenderError::Coordinate`并断言注入child而非root；
- `MissingLayout`/`MissingCurrentFlow` 继续携带自身 ID；
- `DuplicateForwardRecord(ProjectionId)`使用`ProjectionId.element_id`；只有
  `DuplicateReverseCell`、无ID的malformed projection、writer/clip finish failure与test-only
  injected failure可使用调用入口fallback；
- round-trip/malformed/finish等真正无 current owner 的 **non-coordinate** 错误才接受
  `try_render_element_tree` 提供的 root fallback；所有 coordinate variants已经scoped，
  `into_text_render_error` 不得为其读取或覆盖 fallback。

错误数据流保持现有 public shape：

```text
ProjectionError(scoped child)
  -> TextRenderError::Coordinate { child_id, TextCoordinateError }
  -> try_render_to_string* / TestRenderer::try_render_* (direct Result)
  -> RenderPipeline (same TextRenderError)
  -> App::render_frame -> io::Error(source = same TextRenderError)
```

`Display` 只输出固定分类与 debug-form ElementId。不得加入 raw `f32`、source text、style、
frame/projection dump或cache内容。`Error::source` 必须仍返回公开
`TextCoordinateError`；App 的 `io::Error` 必须先保留 `TextRenderError`，再保留其 cause。

### 5. Static filter 的 canonical identity

App 的 dynamic path在 layout/projection前调用 `StaticRenderer::filter_static_elements`。
GH-132把 caller原树 ID定义为整个 public App call的canonical identity：

```text
original retained node ElementId
  -> identity-preserving private filtered node (same ElementId)
  -> LayoutEngine / ProjectionError / TextRenderError (same ElementId)
  -> App io::Error source chain (same ElementId)
```

`static_content.rs` 不再对保留节点调用会分配 fresh ID 的递归 `Element::clone`。private
filter按字段构造节点，复制style/text/spans/key/accessibility/scroll，递归只收集non-static
children，并逐节点写入original `element.id`；被删除的static subtree不进入映射。由于
retained关系是逐节点identity，禁止另建index/path/hash推断ID，也不改变public `Element`
shape或Clone的一般语义。

两个exact seam锁定这条边界：

- `renderer::static_content::tests::filter_static_elements_preserves_original_ids_for_retained_dynamic_nodes`
  构造root→dynamic parent→dynamic failing child并夹入static sibling，逐层断言filtered ID
  等于original且static node缺失；
- `renderer::app::tests::nested_child_coordinate_error_reaches_app_io_source_chain` 在filter前保存
  failing child ID，经`try_prepare_frame`与既有`into_io`转换后断言
  `TextRenderError::Coordinate.element_id`和typed cause均为该original ID。

### 6. Transaction、retry、failure scope 与 concurrency

- tree renderer先验证 flows，再复制 caller Output到 private staged state；包括 early
  background/sibling writes在内，任一 coordinate failure都 drop staged Output和
  ProjectionBuilder。只有 `finish`、round-trip和clip-depth检查全部通过后才能一次
  `commit_staged`。
- dynamic pipeline在 layout/render失败时不调用 runtime evidence setter、不更新
  `previous_vnode`、不返回 frame string；当前既有 reset-to-new LayoutEngine行为作为 clean
  retry边界保留并用 exact test锁定。
- `App::try_prepare_frame` 先收集但不提交 static lines；dynamic失败时整个 candidate丢弃，
  `render_frame` 不触发 terminal/static commit。
- renderer本身是同步计算，无candidate创建后、commit前可达的 cancellation yield；issue
  #132也不新增callback/token/checkpoint。因此B-018显式保留为N/A scope revision：测试不得
  把injected renderer failure或函数返回后drop表述为cancellation。failure fixture只证明
  typed error时不发布candidate；corrected retry由B-014至B-017覆盖。
- string和TestRenderer每次创建独立 LayoutEngine/Output；交错或 scoped-thread fixture使用
  不同 element/tree实例，断言 owner/output互不串扰。App保持既有单线程 frame发布顺序。

### 7. Compatibility 与 rollback boundary

不改变 public enum variant、函数签名、panic wrapper、Element字段或 TextFlow data。
positive fractional/integer snapshots、empty text、missing scroll=0、MissingLayout/
MissingCurrentFlow和 PR #137 zero-width tests都是 mandatory regressions。

实现只需普通 revert即可回滚；没有持久化/schema迁移。若实现需要新增 public variant、
修改 `src/layout/**`、`src/renderer/output*`、`src/testing/renderer.rs` 或其他 manifest外文件，
先更新并重新审批 spec，不得静默扩 scope。

## Product-to-Test Mapping

下表中的新 test name 是 implementation 必须创建的 exact fixture；标为“existing”的命令在
当前 base已存在，必须继续通过。

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `tree_renderer.rs` scoped floor helper | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::signed_coordinates_use_one_floor_conversion -- --exact` |
| B-002 | tree/projection negative visibility | `cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::negative_fractional_x_and_y_clip_instead_of_painting_at_zero -- --exact` |
| B-003 | conversion compatibility matrix | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::negative_zero_positive_fractional_and_integral_coordinates_are_compatible -- --exact` |
| B-004 | recursive coordinate composition | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::signed_coordinate_composition_is_checked_and_axis_independent -- --exact` |
| B-005 | signed clip/scroll/ancestor projection | `cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::negative_fractional_scroll_ancestor_and_clip_preserve_signed_disposition -- --exact` |
| B-006 | finite operands / f32-range / signed bound overflow | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::finite_operands_that_overflow_f32_composition_and_i64_bounds_classify_overflow -- --exact` |
| B-007 | non-finite classification | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::nan_and_infinities_classify_as_non_finite_for_each_coordinate_source -- --exact` |
| B-008 | scoped current owner, including flow validation | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::nested_coordinate_failures_report_exact_current_child -- --exact`; `cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::nested_flow_validation_overflow_reaches_public_error_with_exact_child -- --exact` |
| B-009 | owner/fallback boundary | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::coordinate_owner_survives_conversion_and_only_unscoped_failures_use_root_fallback -- --exact`; the nested validation test must also prove conversion ignores the root fallback |
| B-010 | public string caller | `cargo test --test text_flow_renderer_error_paths --locked nested_child_coordinate_errors_reach_string_api_with_exact_id -- --exact` |
| B-011 | public TestRenderer callers | `cargo test --test text_flow_renderer_error_paths --locked nested_child_coordinate_errors_reach_test_renderer_with_exact_id -- --exact` |
| B-012 | dynamic/App typed chain + canonical filtered identity | `cargo test --workspace --lib --locked renderer::static_content::tests::filter_static_elements_preserves_original_ids_for_retained_dynamic_nodes -- --exact`; `cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::nested_child_coordinate_errors_keep_id_and_candidate_state -- --exact`; `cargo test --workspace --lib --locked renderer::app::tests::nested_child_coordinate_error_reaches_app_io_source_chain -- --exact` |
| B-013 | safe Display/source | `cargo test --workspace --lib --locked renderer::error::tests::coordinate_error_context_is_typed_and_does_not_leak_content -- --exact` |
| B-014 | staged Output/projection atomicity | `cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::coordinates::coordinate_failure_commits_neither_output_nor_projection -- --exact` |
| B-015 | VNode/runtime/cache atomicity | `cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::nested_child_coordinate_errors_keep_id_and_candidate_state -- --exact`; `cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::failed_coordinate_candidate_is_never_published -- --exact` |
| B-016 | late nested traversal failure | `cargo test --workspace --lib --locked renderer::tree_renderer::tests::coordinates::late_nested_coordinate_failure_discards_earlier_staged_paint -- --exact` |
| B-017 | repeat/corrected retry | `cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::repeated_coordinate_failure_then_correction_retries_cleanly -- --exact` |
| B-018 | Reserved / N/A synchronous cancellation scope | source review proves no pre-commit cancellation checkpoint/API is added; no runtime test may claim failure/drop is cancellation |
| B-019 | independent caller contexts | `cargo test --test text_flow_renderer_error_paths --locked independent_coordinate_failures_do_not_share_owner_or_frame_state -- --exact` |
| B-020 | public/behavior compatibility | existing: `cargo test --test prelude_surfaces --locked try_render_to_string_surface -- --exact`; `cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::incremental_failure_retries_from_clean_layout_tree -- --exact`; `cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::projection_zero_width_only_attaches_to_the_same_flow_sequence -- --exact`; `cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection -- --exact` |
| B-021 | exact-head evidence ledger | run every command below plus full fmt/check/clippy/test; `GH132_COVERAGE_MODE=fixture cargo test --test text_flow_renderer_error_paths --locked gh132_current_head_coverage_contract -- --exact`; produce/validate coverage, CI, independent review and fresh reviewThreads against one head SHA |

## Critical Test Ledger

Implementation tasks必须逐项创建并运行上表新 tests；以下现有 regressions也必须逐项运行：

```sh
cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::projection_signed_coordinates_axis_clips_and_nested_active_clips_are_exact -- --exact
cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::projection_failure_commits_neither_cells_nor_projection -- --exact
cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::projection_zero_width_only_attaches_to_the_same_flow_sequence -- --exact
cargo test --workspace --lib --locked renderer::tree_renderer::projection::tests::zero_width::synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection -- --exact
cargo test --workspace --lib --locked renderer::tree_renderer::tests::scrolled_out_negative_rows_do_not_paint_at_top -- --exact
cargo test --workspace --lib --locked renderer::pipeline::typed_error_tests::incremental_failure_retries_from_clean_layout_tree -- --exact
cargo test --workspace --lib --locked renderer::app::tests::app_render_candidate_preserves_typed_error_source -- --exact
cargo test --test text_flow_error_paths --locked typed_error_reaches_remaining_callers -- --exact
cargo test --test text_flow_error_paths --locked caller_failure_commits_no_partial_output -- --exact
cargo test --test text_flow_renderer_error_paths --locked try_render_to_string_preserves_source_and_returns_no_partial_string -- --exact
cargo test --test prelude_surfaces --locked try_render_to_string_surface -- --exact
```

每个 filtered command 的 implementation evidence必须证明 `matched=1`、`passed=1`、
`ignored=0`；仅凭 exit 0 或 substring filter 不足。完成后运行：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of
cargo test --workspace --all-targets --all-features --locked
```

coverage必须绑定同一 exact head，新代码 line coverage至少 80%，signed conversion、
owner attribution与transaction failure关键分支 100%。`cargo-llvm-cov` 固定为
`0.8.7`；raw JSON、base/head/merge-base、diff SHA-256、tool version、command和threshold
必须进入可重算provenance artifact，并由
`tests/text_flow_renderer_error_paths.rs::gh132_current_head_coverage_contract` 的
fixture/produce/validate modes fail-closed验证。CI、review与 reviewThreads也必须fresh query
同一 head。

## 风险

- **Security / privacy:** 错误若包含 source/frame dump会泄漏用户文本；B-013只允许
  ID和closed classification，并要求Display/source regression。
- **Compatibility:** floor改变负 fractional可见结果是目标；正 fractional、整数、extent
  clamp与 public enum必须由 B-003/B-020锁定。
- **Correctness:** clip若先转u16会重新引入 clamp；owner若存在线程式 mutable “current ID”
  容易跨递归串扰。设计要求显式参数/variant携带，不使用全局或 thread-local owner。
- **Atomicity:** error可能发生在 staged earlier writes之后；single `commit_staged` 与
  pipeline publish-order必须由 failure injection验证。
- **Identity:** `Element::clone` 分配fresh ID，不能用于App filtered dynamic tree；source
  review与static/App exact tests必须证明每个retained node沿整条caller链保持original ID。
- **Performance:** floor/range check每 element执行，复杂度仍 O(elements + projected cells)；
  不增加全 viewport scan或二次 traversal。
- **Maintenance:** PR #137/#131有直接或预期文件交集；未执行 refresh/serialization会造成
  owner contract漂移。

## 测试计划

- [ ] Unit：floor、`-0.0`、positive compatibility、i64 half-open bounds、NaN/±inf、
      x/y/padding/scroll/ancestor/background/border owner。
- [ ] Projection：negative fractional x/y/scroll/ancestor/clip、terminal/active clip、
      early-write failure、forward/reverse零发布。
- [ ] Caller integration：nested child NaN与overflow分别通过 string和TestRenderer；dynamic
      pipeline与App I/O chain由 source-module tests覆盖；static filter逐节点保留original ID。
- [ ] State：previous VNode/runtime evidence/layout cache不发布，repeat failure、corrected
      retry与interleaving确定；不把failure/drop称为cancellation。
- [ ] Compatibility：全部 ledger regressions、public prelude、PR #137 zero-width focused
      suite与 full Rust gates。
- [ ] Evidence：coverage、CI、独立 review、current unresolved reviewThreads和SpecRail PR
      gate全部绑定 implementation exact head。

## 回滚方案

没有数据迁移或 feature flag。若出现兼容回归，普通 revert GH-132 implementation commit，
恢复此前转换与错误映射，同时重开 issue；不得只删除 negative fixtures或把 errors改成
warning/fallback。spec packet保留为问题和验收证据。若 PR #137/#131 后续改变 shared
contract，先暂停实现、刷新本 spec与 manifest并重新审批。
