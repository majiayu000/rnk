# Tech Spec：LayoutSnapshot、绝对 Cell 边界量化与 producer parity

## Linked Issue

GH-61: https://github.com/majiayu000/rnk/issues/61

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-61 的 immutable snapshot、terminal-cell quantization、跨 producer parity、
state-machine 与 parity evidence。GH-58 TextFlow、GH-59 scoped identity/order 和 GH-60
transaction/recovery/prepared commit 是强依赖，不在本 issue 内复制或弱化。

## Codebase Context

以下锚点在 stacked base `spec/GH60-transactional-patching`
`f67f973ed6903edb0cb76b5cb45c977ce92be851` 上通过 Read/grep 核实。该 base 尚未包含
GH-58 至 GH-60 的生产实现；GH-61 implementation 必须在三个真实 merge SHA 上重新定位。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Float layout surface | `src/layout/engine.rs:9`, `src/layout/engine.rs:11`, `src/layout/engine.rs:464` | public `Layout` 以四个 `f32` 字段暴露；每次 getter 直接读取 Taffy current layout | snapshot 必须新增整数语义且保留旧 surface |
| Full/incremental split | `src/layout/engine.rs:112`, `src/layout/engine.rs:133`, `src/layout/engine.rs:287` | direct、VNode 与 Element incremental 分别 compute，并由 caller 后续读取 engine | producer strategy 尚未收敛到共同 snapshot |
| Silent collection | `src/layout/engine.rs:490`, `src/layout/engine.rs:509` | `get_all_*` 用 `filter_map` 丢弃 missing layout | snapshot builder 必须按 target exact set fail closed |
| Local float conversion | `src/renderer/tree_renderer.rs:12`, `src/renderer/tree_renderer.rs:27`, `src/renderer/tree_renderer.rs:51` | renderer 独立把坐标/extent 转为 `u16`，missing layout 使用 default | position/extent 分开转换无法证明共享 edge |
| Clip/scroll interpretation | `src/renderer/tree_renderer.rs:81`, `src/renderer/tree_renderer.rs:89`, `src/renderer/tree_renderer.rs:107` | renderer 现场计算 content clip、父 offset 与 scroll | 所有入口必须共享同一 effective clip/transform |
| Dynamic engine reads | `src/renderer/pipeline.rs:26`, `src/renderer/pipeline.rs:41`, `src/renderer/pipeline.rs:48`, `src/renderer/pipeline.rs:56` | pipeline 从 engine 分别取 layout maps/root，再把 engine 交给 renderer | snapshot 应成为 output 与 measurement 的唯一 frame source |
| String path | `src/renderer/render_to_string.rs:123`, `src/renderer/render_to_string.rs:151`, `src/renderer/render_to_string.rs:175` | string renderer反复 compute并从 engine读root height，最后仍把 engine交给 renderer | height probe/final render 必须使用同一 snapshot contract |
| Static path | `src/renderer/static_content.rs:45`, `src/renderer/static_content.rs:48`, `src/renderer/static_content.rs:54` | static 独立 full compute、default root layout、engine renderer | 需要 checked full snapshot，且接入 GH-60 whole-frame prepare |
| Testing path | `src/testing/renderer.rs:49`, `src/testing/renderer.rs:54`, `src/testing/renderer.rs:64` | TestRenderer 计算新 engine并分别读 layout/render | 测试不能保留另一套量化解释 |
| GH-58 projection | `specs/GH58/tech.md:70`, `specs/GH58/tech.md:77`, `specs/GH58/tech.md:127`, `specs/GH58/tech.md:168` | logical TextFlow immutable；visible/clipped projection由每帧 overflow/scroll/content/clip/terminal 输入生成 | snapshot 提供 projection geometry，不复制 logical flow |
| GH-59 identity/order | `specs/GH59/tech.md:170`, `specs/GH59/tech.md:178`, `specs/GH59/tech.md:190`, `specs/GH59/tech.md:203` | checked core产出`ReconcilePlan`，每个parent保存`final_children`，LayoutEngine直接消费该plan | snapshot semantic node/order直接复用该结果 |
| GH-60 error/report algebra | `specs/GH60/tech.md:141`, `specs/GH60/tech.md:151`, `specs/GH60/tech.md:157`, `specs/GH60/tech.md:285` | layout wrapper为`Upstream`/`DirectPatch`/`InitialBuild`/`RecoveryFailed`，frame wrapper只有`Upstream`/`Transaction`/`Render`，initial build失败为`Transaction(InitialBuild(...))` | GH-61只能在现有non-exhaustive wrapper中组合snapshot cause，不能发明initial frame variant |
| GH-60 prepare/postcondition | `specs/GH60/tech.md:210`, `specs/GH60/tech.md:226`, `specs/GH60/tech.md:295`, `specs/GH60/tech.md:317`; `specs/GH60/product.md:101` | candidate经target-exact tree/root/map/order/layout检查；renderer在required lookup前过滤`Display::None`/`VirtualText`；App延迟到terminal success后提交 | snapshot target adapter必须只组合这些真实输入，并在candidate内构建后随同一commit发布 |

## 设计方案

### 1. Gate 与依赖边界

GH-61 spec 可以 stacked 在 PR #77 accepted exact head 上；implementation 不可以。实现必须
基于 GH-58、GH-59、GH-60 三者的真实 merge SHA 重新定位 Codebase Context 中的锚点。

GH-61 不修改 GH-58/GH-59 的既有语义错误。GH-61 自己的 semantic failure algebra 是
closed、无 `Other(String)`/opaque catch-all；若 GH-60 的公开 frame wrapper 已
`#[non_exhaustive]`，只新增带 concrete `source` 的组合 variant，并保留该外层兼容边界。

### 2. Immutable snapshot 模型

在 `src/layout/snapshot.rs` 定义只读、documented public surface。以下字段是规格说明，
实现中 `LayoutSnapshot` 与 `SnapshotNode` 的所有 storage 必须 private：

```text
CellPoint { x: i32, y: i32 }
CellVector { dx: i32, dy: i32 }
CellRect { left: i32, top: i32, right: i32, bottom: i32 } // half-open

pub struct SnapshotIdentity(ScopedNodeIdentity); // public opaque type, private storage/constructors

pub struct SnapshotNode { // all fields private
  identity: SnapshotIdentity,
  parent: Option<SnapshotNodeIndex>,
  children: Arc<[SnapshotNodeIndex]>,
  border_bounds: CellRect,
  content_bounds: CellRect,
  text_origin: CellPoint,
  effective_clip: AxisClip { x: CellSpan, y: CellSpan },
  scroll_transform: CellVector,
  text_flow: Option<TextFlowSemanticStamp>,
}

pub struct LayoutSnapshot { // all fields private
  viewport: CellRect,
  nodes: Arc<[SnapshotNode]>,
  root: SnapshotNodeIndex,
  semantic_index: Arc<...>,
}

pub(crate) struct LayoutSnapshotBuilder { ... }
pub struct PreparedSnapshotFrame {
  snapshot: Arc<LayoutSnapshot>, // private
  frame_aliases: FrameAliasOverlay, // private, semantic equality之外
}

SnapshotBuildReport {
  strategy, patch_count, recovery_cause, rebuild_count,
  cache_hits, work_counters
}
```

`CellSpan { start: i32, end: i32 }` 与 `AxisClip { x, y }` 同样只允许 checked construction。
`LayoutSnapshot` 公开 `viewport()`、`root()`、`nodes()`（read-only exact-size iterator）与
`get(&SnapshotIdentity)`；`SnapshotNode` 仅公开 identity/parent/children/bounds/clip/
scroll/TextFlow 的借用或 copy accessor。`SnapshotIdentity` 只公开 exact `Eq`/`Hash` 与
diagnostic accessor，不公开内部 `ScopedNodeIdentity`、构造器或 segment。不得公开 field、
setter、任意-state `new`/`Default`、
`DerefMut`、`AsMut`、`IndexMut`，也不得从调用方接收未经验证的 node slice。`Clone` 只共享
已验证 `Arc`。

`LayoutSnapshot` 的 semantic equality 只比较 viewport 与 semantic nodes。producer strategy、
Taffy NodeId、ElementId alias、cache slot、generation counter 与独立 build/report work
counters 不进入 equality。frame-local alias overlay 结构上只存在于 `PreparedSnapshotFrame`，允许
同一 semantic snapshot配合 GH-60 fresh ElementId；不得把 aliases放回snapshot再靠自定义
`PartialEq`排除。

GH-59/GH-60没有`committed visible plan`，GH-61不得发明该依赖。真实边界是：

- GH-59 `ReconcilePlan` / `ResolvedParentPlan::final_children`提供target child order；
- GH-60 `PreparedLayoutFrame`携带未发布candidate，target-exact postcondition验证
  tree/root/maps/order/layout；其B-014要求renderer在required-layout lookup前过滤
  `Display::None` / `VirtualText`；
- GH-61在`src/layout/engine/snapshot.rs`新增crate-private `SnapshotTargetPlan`及
  `prepare_snapshot_target(&PreparedLayoutFrame, &Element)`，由T3唯一拥有。它只把GH-59
  final order与GH-60 B-014 renderer traversal规则组合为snapshot-required identities，
  通过GH-60 checked required-layout lookup读取geometry；不得删除/重写engine map/layout、
  改GH-60 postcondition集合或暴露第二个mutable engine。

nodes按`SnapshotTargetPlan` root preorder/final children保存。`Display::None`停止该
render-required traversal，`VirtualText`不产生snapshot node；其余plan entry必须恰好得到一个
GH-60 checked layout，missing/ambiguity原样进入GH-60 typed lookup/layout wrapper。T4 renderer
只消费同一plan产生的snapshot，不再单独过滤第二次。
`snapshot_target_adapter_uses_gh59_order_and_gh60_lookup_contract`锁定上述真实接口；
`display_none_prunes_only_snapshot_render_traversal`证明hidden descendants仍可存在于GH-60
engine target/map集合但不进入snapshot required lookup/output；三strategy parity比较
adapter identities、snapshot与renderer output。

`tests/layout_snapshot_immutability.rs`包含两个独立exact测试：

```text
public_snapshot_read_only_accessors_compile
public_snapshot_mutation_surface_is_compile_fail
```

第二个test body必须无条件执行
`trybuild::TestCases::new().compile_fail("tests/ui/gh61_snapshot_private_fields.rs")`，并与
checked-in `tests/ui/gh61_snapshot_private_fields.stderr`逐字匹配字段、builder与任意-state
constructor的privacy diagnostics；不得用feature/env/平台条件skip。任务须实际执行
`cargo test --test layout_snapshot_immutability --locked public_snapshot_mutation_surface_is_compile_fail -- --exact`，因此仅存在
fixture、仅编译positive test或只跑`cargo check`不算证据。public API manifest同时拒绝上述
mutation surface。

`TextFlowSemanticStamp` 必须由 GH-58 immutable flow 提供并代表 exact source/style/width/
wrap/tab/ellipsis/Unicode policy revision 的语义。若 GH-58 merged实现只有 engine-local
generation，则 GH-61 新增由完整 cache identity 与 logical result equality支持的 semantic
stamp；hash只能作为查找加速，碰撞时仍逐字段比较，不能单独判等。

#### Closed typed failure algebra

首版 semantic variants 固定如下；实现可将 field/axis/operation拆成同样closed的辅助enum，
但不得用字符串替代variant或source：

```text
LayoutSnapshotError =
  NonFiniteGeometry { identity, field, value_bits }
  NegativeExtent { identity, axis, value_bits }
  EdgeArithmeticOverflow { identity, operation, lhs_bits, rhs_bits }
  CellCoordinateOverflow { identity, edge, rounded_bits }
  ReversedContentBounds { identity, border_bounds, attempted_content_bounds }
  MissingIdentity { element_id }
  DuplicateIdentity { identity }
  MissingLayout { identity }
  MissingTextFlowRevision { identity }
  TextFlowRevision { identity, source: TextFlowError }
  InvalidTree { identity: Option<SnapshotIdentity>, source: SnapshotInvariantError }

SnapshotInvariantError =
  MissingParent { child, expected_parent }
  ChildOrderMismatch { parent, child, expected_index, actual_index }
  OrphanNode { identity }
  SnapshotTargetMismatch { identity, reason: SnapshotTargetMismatchReason }

LayoutAliasError =
  MissingFrameAlias { element_id, frame_revision }
  DuplicateFrameAlias { element_id, first_identity, second_identity }
  AliasTargetMissing { element_id, identity }
  StaleFrameAlias { element_id, expected_frame_revision, actual_frame_revision }
  AliasIdentityMismatch { element_id, expected_identity, actual_identity }

CellOutputError =
  NegativeAfterClip { axis, value }
  CoordinateOutOfRange { axis, value }
  ExtentOutOfRange { axis, start, end }

SnapshotRenderError =
  Snapshot { source: LayoutSnapshotError }
  Alias { source: LayoutAliasError }
  Output { identity, source: CellOutputError }
  Text { identity, source: TextRenderError }

RecoveredSnapshotError {
  incremental: Box<PatchTransactionError>,
  snapshot: Box<LayoutSnapshotError>
}

RecoveredSnapshotRenderError {
  incremental: Box<PatchTransactionError>,
  render: Box<SnapshotRenderError>
}

// GH-61 additions to GH-60 non-exhaustive wrappers:
TransactionalLayoutError +=
  Snapshot(LayoutSnapshotError)
  RecoveredSnapshot(RecoveredSnapshotError)

CheckedRenderError +=
  Snapshot(SnapshotRenderError)
  RecoveredSnapshot(RecoveredSnapshotRenderError)
```

其中`GeometryField`、`Axis`、`Edge`、`ArithmeticOperation`、
`SnapshotTargetMismatchReason`与`FrameRevision`也必须closed；`PatchTransactionError`、
`TextRenderError`、`TransactionalLayoutError`、`CheckedRenderError`与
`TransactionalFrameError`绑定GH-60 tech §2/§6的真实type，不得降为字符串。每个包含
`source`的outer variant必须返回concrete nested error；alias leaf的`source()`为`None`但保留
全部payload。两个recovered aggregate的标准`source()`指向最终snapshot/render failure，
并提供`incremental_failure()` accessor保留原GH-60 patch cause。

GH-60 frame wrapper保持且仅保持既有
`Upstream(DynamicFrameError) | Transaction(TransactionalLayoutError) |
Render(CheckedRenderError)`三路；GH-61不新增`Initial` variant：

```text
initial GH-60 build/postcondition
  -> Transaction(InitialBuild(FullRebuildError))
initial/ordinary snapshot build
  -> Transaction(Snapshot(LayoutSnapshotError))
recovered snapshot build failure
  -> Transaction(RecoveredSnapshot(RecoveredSnapshotError))
snapshot/alias/output/text render
  -> Render(Snapshot(SnapshotRenderError))
recovered render failure
  -> Render(RecoveredSnapshot(RecoveredSnapshotRenderError))
GH-59 existing frame failure
  -> Upstream(DynamicFrameError)
```

所有`From`只做上述typed composition，不得加入`Other`或把initial伪装成incremental cause。

`every_snapshot_failure_variant_preserves_payload_and_source_chain`、
`every_layout_alias_variant_preserves_payload_and_source`与
`gh60_frame_wrapper_routes_snapshot_failures_without_fictitious_initial_variant`逐variant匹配
payload，并分别证明`SnapshotRenderError::Alias -> LayoutAliasError`、
`Transaction -> Snapshot/RecoveredSnapshot -> leaf`与
`Render -> Snapshot/RecoveredSnapshot -> leaf`的source traversal；compile fixture必须按
GH-60真实三路match且不存在`TransactionalFrameError::Initial`。

### 3. Absolute half-open edge quantization

`src/layout/snapshot/quantize.rs` 是唯一 terminal-cell quantizer。它从 Taffy unrounded
geometry 或 GH-60 等价 raw layout读取 `f32`，提升为 `f64` 后累计：

```text
absolute_left   = parent_absolute_left + local_x - inherited_scroll_x
absolute_top    = parent_absolute_top  + local_y - inherited_scroll_y
absolute_right  = absolute_left + raw_width
absolute_bottom = absolute_top  + raw_height

cell_left   = checked_floor(absolute_left)
cell_top    = checked_floor(absolute_top)
cell_right  = checked_floor(absolute_right)
cell_bottom = checked_floor(absolute_bottom)
```

- `checked_floor` 沿用已合入 PR #160 的绝对 half-open containing-cell 合同：正负坐标都取
  checked floor，`(-1.0, 0.0)` 不得被折叠到 cell 0；start/end 共用同一函数。
- 量化前验证所有 input/intermediate finite、extent非负、加法无超出 `i32` representable
  rounded range；失败返回 exact identity/field/value 的 `LayoutSnapshotError`。
- width/height只能由 right-left / bottom-top 得出。不能量化 raw width，不能 `as u16`。
- content edges从 raw border/padding/content geometry以相同 absolute rule量化，再与
  border rect求交；正常空 content合法，反向 raw geometry或算术错误不合法。
- `text_origin`保留量化后的raw content start，允许负padding等兼容输入在clip前保持signed；
  `content_bounds`仍与border求交并保持内含。若producer TextFlow width与量化后的raw content
  width不同，snapshot在不可见candidate内用同一input/policy重绑定TextFlow，失败保留
  `TextFlowError` source。
- monotone Q 保证 raw `right <= next.left` 时 cell `right <= next.left`；合法 CSS overlap
  仍保留，测试只证明 quantizer 不制造新 overlap。
- scroll offset累积为 signed transform。clip使用`AxisClip`逐轴继承：x轴只与terminal x、
  祖先x clip和当前`overflow_x`为Hidden/Scroll时的content x span求交，y轴完全对称；
  Visible轴保持继承span。`Hidden/Visible`与`Visible/Hidden`都不能构造完整content rect clip。
- renderer只把 x/y均位于`effective_clip ∩ viewport`内的非负cell checked-convert为`u16`。
  `mixed_axis_overflow_clips_only_selected_axis`与
  `nested_mixed_axis_overflow_matches_all_strategies`覆盖两种方向、两层嵌套、空单轴span，
  并比较full/incremental/recovered及renderer最终cells。

Taffy 0.7 自带 pixel rounding，但 GH-61 不在 renderer 二次猜测其局部 float。merged lock
确认 `layout()` 默认返回 final rounded relative layout，而 `unrounded_layout()` 返回 canonical
raw relative geometry；snapshot builder统一读取后者、累计绝对 edges，再执行上述 checked
floor。parity fixture锁定 nested cumulative edges、content/border 与 scroll；禁止同时保留
“Taffy rounded getter”和“renderer cast”两套 correctness 路径。

### 4. Producer、recovery 与 publication

在 `src/layout/engine/snapshot.rs` 提供：

```text
try_build_snapshot(candidate, target, viewport, aliases)
  -> Result<(LayoutSnapshot, SnapshotBuildReport), LayoutSnapshotError>
```

- 没有previous committed state时走一次`InitialFullBuild`：GH-60 layout/postcondition failure
  保持`Transaction(InitialBuild(FullRebuildError))`；snapshot failure为
  `Transaction(Snapshot(LayoutSnapshotError))`；render failure为
  `Render(Snapshot(SnapshotRenderError))`。三者`rebuild_count=0`，都不构造incremental
  cause、不进入GH-60 recovery、不发布任何state。
- 已有committed state时，normal incremental candidate运行同一个builder；只有GH-60既定
  incremental transaction seam失败才允许一次recovered full。
- GH-60 target-exact postcondition先确认 tree/map/order/layout/TextFlow context；snapshot
  builder再确认 terminal cell projection。
- snapshot失败视为prepared-frame失败，不触发第二次layout/rebuild；initial snapshot失败
  直接返回`Transaction(Snapshot(...))`，incremental snapshot/quantization失败也不能递归
  触发recovery。
  GH-60 recovery只由其既定transaction failure触发。
- recovered candidate只构建一次snapshot。snapshot成功时report保留GH-60
  `incremental_failure`；若snapshot失败，返回
  `Transaction(RecoveredSnapshot { incremental, snapshot })`，若后续render失败则返回
  `Render(RecoveredSnapshot { incremental, render })`。两种aggregate均保留原增量cause，
  不把rebuild success误报为frame success。
- `PreparedAppFrame` 持有 candidate engine、snapshot、rendered static/dynamic output、
  aliases/measurements 与 terminal operations；terminal commit success后不可失败 move/swap。
- cancellation/drop释放 candidate和snapshot builder；已发布 snapshot通过 `Arc` 只读共享。

`initial_snapshot_failure_never_enters_incremental_recovery`必须用recovery spy断言
incremental/recovered调用数均为0、`rebuild_count=0`、source chain为
`Transaction -> Snapshot -> LayoutSnapshotError`且所有
published slots未变；`recovered_frame_uses_only_recovered_candidate_snapshot`只覆盖已有
committed state的incremental fault；`recovered_snapshot_or_render_failure_preserves_both_causes`
逐项断言aggregate accessors/source chain和零发布。

non-App checked engine API可以在成功 builder后立即提交并返回 snapshot/report。旧 `compute*`
保留签名，委托 checked core后保存 current snapshot供 compatibility getter；最终失败
fail loudly。

### 5. Renderer 与 measurement 收敛

`tree_renderer` / `element_renderer` 的 checked core改为接收 `&LayoutSnapshot`，通过当前
frame aliases解析 Element到 semantic node：

- background/border/content/TextFlow projection只读 snapshot bounds/clip/transform；
- renderer源文件不再出现 float screen conversion、required-layout default或递归 engine
  lookup；
- GH-58 `RenderProjection`接收 snapshot content rect、effective clip和terminal bounds；
- dynamic output尺寸来自snapshot root bounds与viewport交集；
- RuntimeContext measurements从snapshot border bounds生成 compatibility `u16/f32`视图，
  与 snapshot一起在GH-60 commit后发布；
- static、testing、render-to-string调用同一 checked full producer和snapshot renderer；
  string height probe如需多次布局，每次 probe也产生完整 checked snapshot，最终 output只消费
  最后一次与resolved viewport匹配的snapshot。

### 6. Parity 与 seeded state machine

`tests/layout_snapshot_parity.rs` 对每个 fixture建立两个独立 engine：

1. full engine每帧从target fresh checked build；
2. incremental engine从同一initial target顺序执行updates；
3. fault fixture使增量在GH-60 seam失败并成功rebuild；
4. 每帧比较 semantic snapshot，并单独断言 build report strategy/cause。

fixtures包含：

- unchanged frame与fresh ElementId alias；
- streaming delta（ASCII、CJK、emoji ZWJ、combining）；
- front/middle/tail insert、remove、replace、keyed reorder、mixed keyed/unkeyed；
- 1000-message transcript，消息高度在1至12 logical rows；
- viewport `120x40 -> 80x24 -> 120x40`；
- nested border/padding/overflow/scroll和负screen position。

`tests/layout_snapshot_state_machine.rs`必须对下列五个u64 seed各运行exact 64步：

```text
0x0000000000000001
0x243f6a8885a308d3
0x9e3779b97f4a7c15
0xd1b54a32d192ed03
0xffffffffffffffff
```

PRNG固定为SplitMix64 wrapping算法，不能使用`StdRng`或随crate版本变化的generator：

```text
state = state + 0x9e3779b97f4a7c15
z = state
z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
z = (z ^ (z >> 27)) * 0x94d049bb133111eb
draw = z ^ (z >> 31)
```

每步无条件消费8个draw，依次作为operation、target、parent、index、payload、width、height、
scroll selector；branch不得多取随机数。`operation_draw % 100`的closed权重为：
`0..=9 unchanged`、`10..=24 update_text`、`25..=34 update_style`、
`35..=49 append`、`50..=59 insert`、`60..=69 remove`、`70..=79 replace`、
`80..=89 reorder`、`90..=94 resize`、`95..=99 scroll`。payload从固定表
`["ascii","中","👩‍💻","e\u{301}"]`按selector取模；viewport从
`[(120,40),(80,24),(120,40),(1,1)]`取模。target/parent/index均按当前stable identity/order
排序后的长度取模；remove/reorder在无合法target时确定映射为unchanged，不补draw。非法
identity/order输入仍由GH-59/GH-60测试负责。failure输出seed、PRNG state、step、8个raw
draw、normalized operation、identity/field/full/incremental value；replay必须从seed重建
完整前缀和首个difference，不允许实现后挑选更容易的seed/权重。

### 7. Work counters

per-frame `SnapshotWorkCounters`是closed、read-only report，不写全局历史：

```text
visited_nodes
mutated_nodes
text_flow_recomputes
snapshot_nodes
rebuild_count
```

这些 counters 是 `SnapshotBuildReport` 的一部分。GH-61 只负责在 full、incremental 与
recovered producer 上以同一 closed 字段集生成 counters；以它们为输入的 benchmark harness、
artifact、baseline、compare 与 promotion 生命周期全部属于 #85。

### 8. Verification、public docs 与 coverage

所有 filtered Rust test 以 `-- --exact` 运行并逐个列在 Product-to-Test Mapping 中。
新增 public item 必须 documented，指定 doctest 须真实执行且非 `ignore` / `no_run`。
新代码 changed-line coverage >=80%，snapshot / quantizer / parity / error 核心文件
line 与 branch 均 100%，由既有 CI Coverage job 报告。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | snapshot producer/consumers | `cargo test --test layout_snapshot_parity --locked all_render_consumers_use_one_snapshot -- --exact` |
| B-002 | semantic node/index | `cargo test --workspace --lib --locked layout::snapshot::tests::semantic_identity_and_final_order -- --exact` |
| B-003 | GH61 adapter over actual GH59/GH60 contracts | `cargo test --test layout_snapshot_parity --locked snapshot_target_adapter_uses_gh59_order_and_gh60_lookup_contract -- --exact`; `cargo test --test layout_snapshot_parity --locked display_none_prunes_only_snapshot_render_traversal -- --exact` |
| B-004 | `CellRect` | `cargo test --workspace --lib --locked layout::snapshot::quantize::tests::half_open_bounds_derive_extent_from_edges -- --exact` |
| B-005 | absolute edge quantizer | `cargo test --test layout_snapshot_parity --locked nested_shared_edges_do_not_gain_overlap -- --exact` |
| B-006 | content/border quantization | `cargo test --workspace --lib --locked layout::snapshot::quantize::tests::content_border_and_gap_error_are_bounded -- --exact` |
| B-007 | signed coordinates/output conversion | `cargo test --test layout_snapshot_error_paths --locked negative_and_overflow_cells_are_not_clamped_to_success -- --exact` |
| B-008 | axis-independent effective clip | `cargo test --workspace --lib --locked layout::snapshot::tests::mixed_axis_overflow_clips_only_selected_axis -- --exact`; `cargo test --test layout_snapshot_parity --locked nested_mixed_axis_overflow_matches_all_strategies -- --exact` |
| B-009 | scroll projection | `cargo test --test layout_snapshot_parity --locked scroll_changes_descendant_projection_only -- --exact` |
| B-010 | TextFlow semantic stamp | `cargo test --test layout_snapshot_parity --locked cold_and_cached_text_flow_revisions_are_semantically_equal -- --exact` |
| B-011 | build report separation | `cargo test --workspace --lib --locked layout::snapshot::tests::producer_report_does_not_change_semantic_equality -- --exact` |
| B-012 | three-strategy parity | `cargo test --test layout_snapshot_parity --locked full_incremental_and_recovered_are_semantically_equal -- --exact` |
| B-013 | mutation matrix | `cargo test --test layout_snapshot_parity --locked chat_mutation_matrix_matches_full -- --exact` |
| B-014 | seeded state machine | `cargo test --test layout_snapshot_state_machine --locked seeded_operations_match_after_every_step -- --exact` |
| B-015 | resize round trip | `cargo test --test layout_snapshot_parity --locked resize_round_trip_restores_semantic_snapshot -- --exact` |
| B-016 | renderer convergence | `cargo test --test layout_snapshot_parity --locked dynamic_static_testing_and_string_share_cell_contract -- --exact` |
| B-017 | prepared App frame | `cargo test --workspace --lib --locked renderer::app::tests::snapshot_commits_only_with_prepared_app_frame -- --exact` |
| B-018 | snapshot failure atomicity | `cargo test --test layout_snapshot_error_paths --locked snapshot_failure_publishes_nothing -- --exact` |
| B-019 | actual GH60 initial/recovery wrapper source | `cargo test --test layout_snapshot_error_paths --locked initial_snapshot_failure_never_enters_incremental_recovery -- --exact`; `cargo test --test layout_snapshot_error_paths --locked recovered_snapshot_or_render_failure_preserves_both_causes -- --exact`; `cargo test --test layout_snapshot_parity --locked recovered_frame_uses_only_recovered_candidate_snapshot -- --exact` |
| B-020 | cancellation / enforced immutable share | `cargo test --workspace --lib --locked layout::snapshot::tests::cancelled_builder_is_hidden_and_published_snapshot_is_immutable -- --exact`; `cargo test --test layout_snapshot_immutability --locked public_snapshot_read_only_accessors_compile -- --exact`; `cargo test --test layout_snapshot_immutability --locked public_snapshot_mutation_surface_is_compile_fail -- --exact` |
| B-021 | closed typed failures/GH60 three-route source chain | `cargo test --test layout_snapshot_error_paths --locked every_snapshot_failure_variant_preserves_payload_and_source_chain -- --exact`; `cargo test --test layout_snapshot_error_paths --locked every_layout_alias_variant_preserves_payload_and_source -- --exact`; `cargo test --test layout_snapshot_error_paths --locked gh60_frame_wrapper_routes_snapshot_failures_without_fictitious_initial_variant -- --exact` |
| B-022 | compatibility | `cargo test --test layout_snapshot_compat --locked existing_layout_engine_renderer_and_testing_surface_compiles -- --exact` |
| B-023 | no-op alias overlay | `cargo test --test layout_snapshot_parity --locked reused_snapshot_accepts_target_exact_frame_aliases -- --exact` |
| B-029 | exact head/coverage/docs gates | direct execution of the two exact command blocks、full Rust gates、current exact-head repository CI、independent review、resolved review threads与maintainer明确merge authorization |
| B-030 | merged dependencies | 三次 `git merge-base --is-ancestor "$GH*_MERGED_SHA" HEAD` 与 GitHub merged evidence |

## 数据流

```text
Element target + viewport
  -> GH-58 exact TextFlow inputs
  -> GH-59 scoped identity/final order
  -> GH-60 prepared candidate + target-exact postcondition
  -> GH-61 SnapshotTargetPlan(GH-59 final order + GH-60 B-014 traversal/checked lookup)
  -> GH-61 absolute edge quantization
  -> immutable LayoutSnapshot + separate SnapshotBuildReport
  -> GH-58 frame-local RenderProjection(snapshot content/clip/viewport)
  -> renderer Output + measurements
  -> GH-60 PreparedAppFrame terminal commit
  -> atomic publication of engine/snapshot/previous/aliases/measurements/static state
```

full/incremental/recovered 只改变 producer report，不改变 snapshot semantic data。report
随 checked producer 返回但不进入 snapshot equality 或运行时持久化；生产 snapshot 只保存在
当前 App/checked caller 内，无磁盘或全局跨 App cache。

## 备选方案

- **renderer继续读取 `LayoutEngine` 并共享一个 cast helper**：拒绝。共享函数不能阻止不同
  入口读取不同frame、missing/default或重复clip解释。
- **分别 `round(x)` 与 `round(width)`**：拒绝。相邻rect无法共享同一quantized edge，
  width总和可漂移。
- **所有负值先clamp为0**：拒绝。scroll-out内容会被错误绘制到top/left edge。
- **直接把Taffy NodeId作为snapshot identity**：拒绝。fresh full/rebuild可分配不同NodeId，
  semantic parity会产生伪差异。
- **TextFlow generation counter直接判等**：拒绝。cold full与cache hit可能语义相同但
  generation不同。
- **snapshot失败再触发一次full rebuild**：拒绝。违反GH-60 exactly-once recovery并可能循环。
- **从 work counters 直接宣称 incremental 更快**：拒绝。counters 只描述确定性工作量；
  benchmark 与 performance decision 属于 #85。

## 风险

- **Security**：identity 或 error payload 可能含 terminal controls。沿用 GH-58
  sanitization；诊断不暴露 public `Any`、arbitrary closure 或不受控执行 seam。
- **Compatibility**：GH-58至GH-60尚未实现，真实module/public enum可能变化。implementation
  只在merged SHA重定位后开始；GH-61 semantic error set首版closed，只有GH-60既有公开outer
  wrapper可保持`#[non_exhaustive]`；旧`Layout`/wrappers保留。
- **Correctness**：Taffy 0.7默认rounding与unrounded layout细节可能与写作时假设不同。只选
  一个canonical source，并用nested cumulative edge fixtures锁定；禁止double rounding。
- **Identity**：ElementId alias与semantic identity混放会破坏no-op reuse/parity。aliases放在
  prepared frame或明确排除semantic equality。
- **TextFlow**：opaque generation若被当semantic revision会让full/cache-hit伪不等；必须
  绑定完整logical identity并对hash碰撞逐字段确认。
- **Performance**：snapshot node/index/Arc 与 clone-staging 可能增加工作量。closed work
  counters 暴露变化；不能以删除 postcondition、typed errors 或 atomicity 优化。
- **Maintenance**：布局与 renderer 文件多。按模块拆分，保持每文件低于800行；
  parity/state-machine fixtures 与 production writer 保持明确 ownership。

## 测试计划

- [ ] Unit：cell point/vector/rect、absolute edge round、nested/shared edges、border/content、
      x/y独立clip intersection、scroll、semantic index/equality、`SnapshotTargetPlan`对
      GH-59 final order与GH-60 checked lookup的exact consumption（不改engine maps）、
      TextFlow stamp、每个closed typed error payload/source。
- [ ] Integration：initial无recovery、full/incremental/recovered parity、mixed-axis/nested clip、
      `SnapshotTargetPlan`按GH-60 B-014处理`Display::None`/`VirtualText`并完成
      snapshot/output parity、missing required layout typed failure、所有renderer入口、
      prepared App publication、compatibility、exact trybuild immutable boundary与完整error
      source chain。
- [ ] Property/state-machine：上述五个fixed seeds、每个exact 64步、SplitMix64每步固定8 draws
      和closed operation权重；逐步snapshot equality并输出可重放诊断。
- [ ] Public docs：直接执行checker；manifest双向、`forbid(missing_docs)`、nonzero exact
      runnable doctests、拒绝`ignore`/`no_run`、`RUSTDOCFLAGS="-D warnings"`。
- [ ] Coverage：直接执行checker；changed executable >=80%且denominator非零；五个critical
      paths line+branch 100%且各denominator非零。
- [ ] Full gates：
      `cargo fmt --all -- --check`；
      `cargo check --workspace --all-targets --all-features --locked`；
      `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
      `cargo test --workspace --all-targets --all-features --locked`；
      `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`。
- [ ] GitHub：current exact-head repository CI、independent review artifact、resolved review
      threads、maintainer对同一head的明确merge authorization与三个dependency merged SHA；
      labels仅描述状态，不授权实施或合并。

## 回滚方案

GH-61 implementation 使用独立PR。若snapshot migration引入正确性回归，整体回滚该PR，
恢复GH-60已合入的prepared renderer/transaction行为；不得只恢复某个renderer的live-engine
lookup、default layout或独立float cast，否则重新形成多套布局语义。

若 snapshot/work-counter 成本确实过高，保留 correctness、typed error 与 atomic publication
合同，另开 spec 优化 storage、incremental snapshot reuse 或 GH-60 candidate strategy。
回滚后 GH-61 保持打开并保存 exact failed head、CI 与 review evidence；#85 不得在缺少
merged GH-61 measurement seam 时开始 implementation。
