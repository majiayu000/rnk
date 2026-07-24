# Tech Spec：事务式增量布局与类型化错误

## Linked Issue

GH-60: https://github.com/majiayu000/rnk/issues/60

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-60 的 post-preflight transaction、一次 fresh rebuild、target-exact
postcondition 与 missing-layout renderer propagation。GH-58 的 TextFlow/error，GH-59 的
identity/order plan 与成功 map cleanup 保持独立；GH-61 的 snapshot/state-machine/
benchmark 不提前并入。

## Codebase Context

以下锚点均在 stacked base `spec/GH59-keyed-identity-order`
`a985d4f7003bf0b04107b183a74675942e9715f8` 上通过 Read/grep 核实。该 branch 的生产代码
仍等于 `origin/main` `e4a89ae128533270d28d768d49977a05a389a582`；GH-58/GH-59
implementation 合入后，GH-60 implementer 必须在真实 merged head 上重新定位。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Destructive full build | `src/layout/engine.rs:64`, `src/layout/engine.rs:112`, `src/layout/engine.rs:179`, `src/layout/engine.rs:287` | `build_tree` / `build_vnode_tree` 先 clear 当前 engine；build 返回 `Option`，compute 忽略 Taffy result | final recovery 不能在 committed engine 上原地 clear/build |
| Incremental caller | `src/layout/engine.rs:133`, `src/layout/engine.rs:150`, `src/layout/engine.rs:163` | previous tree 存在时 diff/apply；bool false 后直接在同一 engine full rebuild | 需要区分 invalid plan、commit failure、recovered success 与 final failure |
| Batch success rule | `src/layout/engine.rs:309`, `src/layout/engine.rs:314`, `src/layout/engine.rs:316` | 一个共享 `needs_recompute`；任一 patch 成功即可让整个 batch 返回 true | 这是 partial commit 的直接根因 |
| Ignored patch failures | `src/layout/engine.rs:355`, `src/layout/engine.rs:369`, `src/layout/engine.rs:380`, `src/layout/engine.rs:391`, `src/layout/engine.rs:421` | create/remove/replace/reorder 吞 Taffy result，update 只返回 bool | 必须转换为有 patch/stage/key locator 的 typed failure |
| Recompute failure | `src/layout/engine.rs:448` | `compute_layout_with_measure` result 被忽略 | mutation 成功但 compute 失败仍会被报告成功 |
| Silent map omission | `src/layout/engine.rs:275`, `src/layout/engine.rs:464`, `src/layout/engine.rs:490`, `src/layout/engine.rs:509` | sync/collection 使用 `if let` / `filter_map` 丢弃 missing NodeId/layout | target-exact postcondition 和 required lookup 必须 fail closed |
| Dynamic publication order | `src/renderer/pipeline.rs:26`, `src/renderer/pipeline.rs:32`, `src/renderer/pipeline.rs:39` | compute 后立即更新 previous VNode 与 RuntimeContext measurements | render/output/terminal 失败会留下跨帧状态不一致 |
| Mouse publication order | `src/renderer/app.rs:261`, `src/renderer/app.rs:263`, `src/renderer/app.rs:265`, `src/renderer/terminal.rs:597`, `src/renderer/terminal.rs:606` | layout/static/dynamic prepare 前即执行会写控制序列的 mouse enable/disable | mouse transition 必须进入 prepared terminal commit，prepare failure 零 terminal side effect |
| Dynamic default layout | `src/renderer/pipeline.rs:48` | 缺失 root layout 使用 `unwrap_or_default()` | 不能把 transaction/rebuild failure显示成零尺寸 frame |
| Recursive default layout | `src/renderer/tree_renderer.rs:40`, `src/renderer/tree_renderer.rs:51` | 每个 visible Element 缺失 layout 都用 default 后继续递归 | shared renderer 必须返回 typed missing-layout cause |
| Static/string fallback | `src/renderer/static_content.rs:48`, `src/renderer/render_to_string.rs:123`, `src/renderer/render_to_string.rs:152` | static 用 default layout；string probe 对 missing root 使用 initial guess并继续 | checked static/string entrypoints 必须无 partial/blank success |
| Dependency version | `Cargo.toml:45` | `taffy = "0.7"`；当前 lock 解析的 0.7.7 `TaffyTree` 可 clone | clone-staging 可保留 candidate 内 NodeId；实现时必须在 merged lock 上复核 |
| GH-58 boundary | `specs/GH58/tech.md:178`, `specs/GH58/tech.md:228` | `TextRenderError` / `TextFlowError` 明确保持 TextFlow-only | GH-60 必须增加独立 generalized render/layout errors |
| GH-59 boundary | `specs/GH59/tech.md:123`, `specs/GH59/tech.md:231`, `specs/GH59/tech.md:298` | GH-59 提供 checked plan/preflight、`IncrementalLayoutError` / `LayoutLookupError` / `DynamicFrameError`；commit rollback/rebuild/missing-layout 留给 GH-60 | GH-60 扩展现有 checked chain，不复制 identity/order planner |
| No-op alias refresh | `src/layout/engine.rs:133`, `src/layout/engine.rs:159`, `src/layout/engine.rs:164`, `src/layout/engine.rs:275` | VNode diff 可为空，但当前 Element tree 的 fresh `ElementId` 仍需重新映射到已有 NodeId | B-001 no-op 不能保留 stale per-frame aliases |

## 设计方案

### 1. Gate 与依赖边界

GH-60 spec 可以 stacked 在 GH-59 accepted spec head 上；implementation 不可以。实现必须
基于 GH-58 与 GH-59 的真实 merge SHA 重新定位 Codebase Context 中的锚点。

GH-59 的 `ReconcilePlanError` 与 final-order preflight 仍是 mutation 前 fail-closed 边界：
metadata/duplicate/collision/missing/duplicate/extra final identity 直接返回，不进入 GH-60
recovery。GH-60 只处理已得到完整 `ResolvedParentPlan` 之后发生的 commit、compute、
read-back 与 postcondition failure。

### 2. 类型化 transaction / recovery contract

在 GH-59 的 `src/layout/incremental_error.rs` 中新增 GH-60 concrete types，不使用 public
alias，也不向 GH-59 已发布、可穷举匹配的 `IncrementalLayoutError` 或
`DynamicFrameError` 追加 variant：

```text
IncrementalPatchKind =
  Create | Update | Remove | Replace | Reorder | Recompute

PatchStage =
  ResolveTarget | CreateNode | SetContext | SetStyle | RemoveNode |
  SetChildren | ReadBack | ComputeLayout | VerifyPostcondition

IncrementalInvariantError =
  MissingRoot | InvalidRoot | ReachableNodeCycle | ReachableNodeSetMismatch |
  NodeCountMismatch | ScopedMapMismatch | ElementMapMismatch |
  CompatibilityMapMismatch | InvalidMappedNode | ChildOrderMismatch |
  MissingComputedLayout | CurrentFrameContextMismatch

PatchFailure =
  MissingParent | MissingTarget | Taffy(TaffyError) |
  TextFlow(TextFlowError) | Invariant(IncrementalInvariantError)

DirectPatchPreflightCause =
  MissingTarget |
  AmbiguousTarget { match_count } |
  MissingParent |
  AmbiguousParent { match_count } |
  AlreadyExists |
  SubtreeCollision { conflicting_key } |
  InvalidReorderMove { from, to, child_count } |
  DependencyRemoved { prior_patch_index } |
  DependencyReplaced { prior_patch_index }

DirectPatchPreflightError {
  patch_index: usize,
  kind: IncrementalPatchKind,
  key: Option<NodeKey>,
  parent: Option<NodeKey>,
  source: DirectPatchPreflightCause
}

DirectPatchError =
  Preflight(DirectPatchPreflightError) |
  Transaction(PatchTransactionError)

#[non_exhaustive]
DirectPatchApplyReport =
  NoChange |
  Applied { patch_count }

PatchTransactionError {
  patch_index: Option<usize>,
  kind: IncrementalPatchKind,
  key: Option<NodeKey>,
  parent: Option<NodeKey>,
  stage: PatchStage,
  source: PatchFailure
}

RebuildStage =
  BuildTarget | SetContext | ComputeLayout | VerifyPostcondition

RebuildFailure =
  InvalidTargetRoot |
  Taffy(TaffyError) |
  TextFlow(TextFlowError) |
  Invariant(IncrementalInvariantError)

FullRebuildError {
  stage: RebuildStage,
  key: Option<NodeKey>,
  source: RebuildFailure
}

#[non_exhaustive]
TransactionalLayoutError =
  Upstream(IncrementalLayoutError) |
  DirectPatch(DirectPatchError) |
  InitialBuild(FullRebuildError) |
  RecoveryFailed {
    incremental: Box<PatchTransactionError>,
    rebuild: Box<FullRebuildError>
  }

#[non_exhaustive]
TransactionalFrameError =
  Upstream(DynamicFrameError) |
  Transaction(TransactionalLayoutError) |
  Render(CheckedRenderError)

CheckedIncrementalLayoutReport =
  InitialFullBuild |
  NoChange |
  Incremental { patch_count } |
  RecomputedViewport |
  RecoveredFullRebuild {
    patch_count,
    incremental_failure: PatchTransactionError
  }
```

- `patch_index=None` 只用于没有单一 public patch ordinal 的 viewport recompute/批次后
  compute/postcondition；不以 `usize::MAX` 冒充缺失值。
- `Error::source` 对 `TaffyError`、`TextFlowError` 与 nested rebuild/transaction cause
  保留可遍历 source。`RecoveryFailed` 同时提供显式 accessors 读取两层 cause；标准
  `source()` 指向 final rebuild error，不能删除 incremental field。
- `RebuildFailure` 是 closed concrete cause 集合；`InvalidTargetRoot`、Taffy、TextFlow 与
  postcondition invariant 均由 `FullRebuildError.stage/key/source` 精确定位，规格中不允许
  未定义的 “rebuild failure” 占位符。
- key/parent 使用 GH-59 的 collision-checked compatibility `NodeKey`；display 时只输出
  safe debug token/type/index，不回显 raw user string、ESC/C0/C1。
- `TransactionalLayoutError` / `TransactionalFrameError` 从首版即 `#[non_exhaustive]`；
  rustdoc 明确外部 match 需要 wildcard。external compile fixture 必须证明 GH-59 两个 enum
  仍可按旧 variant 集合穷举匹配，并证明新 wrapper 可用。
- targetless direct checked apply 返回
  `Result<DirectPatchApplyReport, TransactionalLayoutError>` 且永不 rebuild；空 batch 映射
  `DirectPatchApplyReport::NoChange`，完整非空 batch commit 映射
  `Applied { patch_count }`。只有携带 target VNode 的
  target-aware checked compute/apply 才执行一次 rebuild。

### 3. Clone-staging transaction

当前 Taffy 0.7.7 与 `NodeContext`/maps 可 clone。`LayoutEngine` 增加内部 clone 能力，但不
把 clone candidate 暴露为第二个 public mutable engine。

`src/layout/engine/transaction.rs` 实现：

```text
prepare_element_incremental(
  committed: &LayoutEngine,
  target_element,
  previous_vnode,
  viewport
) -> Result<PreparedLayoutFrame, TransactionalLayoutError>

PreparedLayoutFrame {
  candidate_engine,
  current_vnode,
  element_alias_overlay,
  report
}
```

流程：

1. 使用 GH-59 checked core 生成 current VNode、`ReconcilePlan` 与
   `ResolvedParentPlan`；任何 plan/preflight error 原样返回，committed 只读。
2. target、previous 与 viewport 均不变且 plan 为空时，tree/root/scoped maps/layout 不
   clone、不重算；但必须从 current Element tree 建立 target-exact
   `element_alias_overlay: ElementId -> existing scoped identity/NodeId`。overlay lookup 任一
   missing/ambiguity 立即 typed 失败；只有 prepared frame publication 成功才替换 committed
   per-frame aliases。若 overlay 与 committed aliases 也完全相同，才是零状态变化。
   viewport 改变即使 plan 为空也进入 clone candidate 的 checked recompute。
3. 对非空 plan clone committed engine。Taffy SlotMap keys 在 clone 中保持一致，因此
   surviving identity 的 NodeId 在正常 incremental success 上不改变。
4. 只在 candidate 上按 addressed plan 执行 create/update/remove/replace/reorder；
   每个 Taffy result 立即转换为 `PatchTransactionError`，停止当前 candidate。
5. 全部 mutation 完成后，在 candidate 上执行一次 checked layout compute，再运行
   target-exact postcondition。
6. 只有 caller 完成 element render/output/terminal I/O 后，`PreparedLayoutFrame::commit`
   通过不可失败的 move/swap 一次性替换 App engine（若有 candidate）、previous VNode、
   `ElementId` alias overlay 与 measurement maps；放弃/drop candidate/overlay 对 committed
   state 零影响。

公开 `try_compute_element_incremental_checked(&mut self, ...)` 可调用同一 prepare core并在
布局/renderer 外立即 commit，用于非 App caller。App/pipeline 必须使用 delayed prepared
frame，避免 terminal write failure 后出现“engine 已新、previous VNode 仍旧”。

`Patch` 的 public `NodeKey` 是 unscoped compatibility token；GH-59 允许相同
type/index/raw key 分别存在于不同 parent scope。`preflight_direct_patch_batch` 必须在任何
clone/Taffy/map mutation 前，以 committed scoped identities 建立只读虚拟状态，并按 public
batch 原始顺序模拟 identity/parent/order 变化，生成内部
`ResolvedDirectPatchBatch`。cardinality 与 batch dependency 决策表如下：

| Patch kind | Target rule at this ordinal | Parent/order rule | Virtual-state transition |
| --- | --- | --- | --- |
| `Create` | `key` 在 resolved parent scope 下必须为 0 matches；`node` root/key 必须与 `Create.key` 一致 | `parent` 在当前 virtual state 必须恰好 1 match；new subtree 每层 scoped identity 无内部 duplicate，且与当前 virtual state 0 collision | 验证完整 subtree 后才把其 identities/order加入 virtual state，后续 patch 可引用 |
| `Update` | unscoped `key` 在当前 virtual state 必须恰好 1 match | N/A | identity/order 不变；若目标由先前 create/replace 产生，resolved address 记录 prior ordinal |
| `Remove` | unscoped `key` 在当前 virtual state 必须恰好 1 match | root remove 按 GH-59 illegal-transition error拒绝 | 从 virtual state 删除目标完整 subtree；后续引用返回带 prior ordinal 的 `DependencyRemoved` |
| `Replace` | old `key` 在当前 virtual state 必须恰好 1 match | 先虚拟移除 old subtree，再按原 parent/slot 对 `node` 完整 subtree 做 create-equivalent collision check | 原子替换 virtual subtree；后续引用新 identities，旧引用返回 `DependencyReplaced` |
| `Reorder` | N/A | `parent` 必须恰好 1 match；每个 `(from,to)` 均在 current child cardinality 内，最终位置是完整 permutation | 只更新该 parent virtual child order |

0 match、2+ matches、already-exists、subtree collision、invalid reorder 与 batch-local
removed/replaced dependency 分别返回 concrete `DirectPatchPreflightCause`。每个
`DirectPatchPreflightError` 必须填充原始 `patch_index`、`kind`、可用的 `key`/`parent`；
不得用最终 batch index、`None` 或 generic string 掩盖 locator。只有整批 preflight
完成后才 clone/apply；任一失败 committed fingerprint、candidate count 与 rebuild count
均不变。

`apply_patches()` 不能自行恢复，因为它没有 target VNode。新增 targetless
`try_apply_patches_transactional()` 只在 clone 上提供 scoped-preflight 后的 atomic
apply + compute + structural postcondition，返回 concrete `DirectPatchApplyReport`，失败不
rebuild；旧 bool API 委托它，`NoChange` 返回 false、`Applied` 返回 true，
missing/ambiguity/collision/dependency/commit failure 均携带 cause fail loudly。
另提供 `try_apply_patches_to_target_transactional(patches, target_vnode, viewport)`（或由
GH-59 resolved plan 驱动的等价 public checked API），其 target-aware 路径才享有一次 fresh
rebuild。dynamic correctness path 不调用 legacy bool wrapper。

手写 inverse journal 不作为 correctness source。Taffy `set_children` 会同时修改多个
parent relation，`remove` 会 detach direct children，measure context/cache 也会变化；
试图逐个反向操作难以证明恢复完整。clone candidate drop 是唯一 rollback primitive。

### 4. Exactly-once fresh full rebuild

`src/layout/engine/rebuild.rs` 定义只在新 engine 上工作的 checked builder：

```text
try_rebuild_target_fresh(target_vnode, element_key_map, viewport)
  -> Result<LayoutEngine, FullRebuildError>
```

- 每次 recovery 创建 `LayoutEngine::new()`，不 clone partial candidate，不 clear committed。
- recursive build 的每个 node/context/Taffy result 都返回带 key/stage 的 typed error；
  禁止 `.ok()?`、`filter_map` 或 `let _`。
- 完成 build 后执行 checked TextFlow/layout compute 与相同 target-exact postcondition。
- incremental candidate failure 只能调用它一次。success 将 report 标记为
  `RecoveredFullRebuild` 并保留原 error；failure 组装 `RecoveryFailed` 后 drop candidate
  与 failed rebuild，committed engine 不变。
- initial frame 没有 previous VNode 时也使用 fresh checked builder；完整 build、compute、
  target-exact postcondition 与 prepared publication 成功后才返回 `InitialFullBuild`。
  build/compute/postcondition 任一步失败只返回
  `TransactionalLayoutError::InitialBuild(FullRebuildError)`，不伪造不存在的 incremental
  cause，也不发布 engine、previous、measurements 或 output。
- recovery 内禁止调用 `compute_element_incremental`，因此不会递归 fallback。

一次调用的 `rebuild_attempts` 由内部局部状态计算并在 test report/fault harness 中断言为
0 或 1；不在 `LayoutEngine` 保存跨帧 mutable counter，避免历史无限增长。

### 5. Target-exact postcondition

`src/layout/engine/postcondition.rs` 在 candidate/fresh engine 发布前只读验证：

1. target layout-node set 排除 `VirtualText`，从 current VNode/scoped identity 生成唯一集合。
2. `root_node` 存在、在 Taffy 中有效并对应 target root。
3. 从 root 遍历实际 Taffy children，拒绝 cycle/重复可达 node；actual reachable NodeId set
   与 target set 数量和映射逐项相等。
4. `TaffyTree::total_node_count()` 等于 target layout-node count，拒绝 remove/replace
   留下的 detached orphan descendants。
5. GH-59 scoped map、ElementId map 与 composite/legacy projection 的 key set 与 target
   exact set 一致；每个映射 NodeId 有效、可达且只出现一次。
6. 每个 parent 的实际 child NodeId vector 逐项等于 resolved final order；不只比较集合。
7. 每个 target layout node 的 current layout 可读；TextFlow/frame context 与当前
   source/style/viewport 匹配，不接受上一 frame cache。
8. 任一差异返回 concrete `IncrementalInvariantError`，不修补、不删除 stale entry 后继续。

GH-59 仍负责“正常成功 remove/replace/move 时构造 target-exact maps”；GH-60 的职责是
在 publish 前强制验证并在 commit failure/rebuild path 保证原子性。若 GH-59 实现遗漏
detached Taffy descendants，GH-60 transaction lane需在 addressed mutation 中递归删除它们，
但不得复制 identity planner。

### 6. Renderer missing-layout 与 publication order

保持 GH-58 `TextRenderError` 为 TextFlow-only，在 `src/renderer/error.rs` 新增：

```text
LayoutRenderError =
  MissingElementLayout { element_id } |
  MissingRootLayout { element_id } |
  LayoutLookup(LayoutLookupError)

CheckedRenderError =
  LayoutBuild(TransactionalLayoutError) |
  Text(TextRenderError) |
  Layout(LayoutRenderError)
```

GH-59 `DynamicFrameError` variant 集合保持原样；新的
`TransactionalFrameError::Upstream/Transaction/Render` 组合旧 error 与 GH-60 layout
failure。不得向 GH-59 exhaustive enum 追加 variant，也不得把 layout failure 塞入
`TextRenderError` 或字符串。

- `try_render_element_tree_checked` / `try_render_element_checked` 对每个非
  `Display::None` 且非 `ElementType::VirtualText` 的 Element 调 required layout lookup；
  `VirtualText` 在 lookup 前过滤，其他 missing 立即 Err，Output candidate 被丢弃。
- `RenderPipeline::prepare_dynamic_frame` 消费 `PreparedLayoutFrame` 的 candidate engine，
  在局部值中构建 `ElementId` alias overlay、measurement maps、Output String 和 current
  VNode，过程中不修改 RuntimeContext/previous VNode；initial build/compute/postcondition
  error 进入 `TransactionalFrameError::Transaction`，不能 panic/stringify。
- `StaticRenderer` 增加 generalized checked extraction，递归任一步失败即返回 Err且不追加
  committed lines；产物只进入局部 `PreparedStaticOutput`。旧 GH-58 TextFlow-only try 与
  非-try入口保留，遇不可表达 layout error fail loudly。
- `App::prepare_frame` 必须先完成 static extraction、dynamic layout/render、required
  lookups 与 output 构造，再组合成 `PreparedAppFrame { terminal_output,
  mouse_mode_transition, static_commit, dynamic_commit, element_alias_overlay }`；desired
  mouse mode 只记录为 value，在全部可失败 prepare 完成前不得调用
  `enable_mouse`/`disable_mouse`/frame writer，也不得增加 static committed-lines。
- `App::render_frame` 只对完整 `PreparedAppFrame` 执行 terminal commit：先应用 prepared
  mouse transition（仅当 mode 改变），再提交组合 output；只有全部 I/O 成功后才以不可失败
  move/swap 同时提交 terminal mouse-state mirror、static committed-lines、candidate engine、
  previous VNode、ElementId aliases 与 RuntimeContext measurements。initial/layout/static/
  dynamic prepare 失败时 writer spy 必须观察到零 mouse/control/frame bytes；I/O 阶段失败时
  in-memory state 保持旧值。
- render-to-string 与 `TestRenderer` 增加返回 `CheckedRenderError` 的 generalized checked
  entrypoints；checked layout build/compute/postcondition 原 cause 进入 `LayoutBuild`，
  probe/final root 和所有 child required lookups fail closed，partial/blank String 不返回。

本合同原子化的是 terminal 调用之前的业务准备与 terminal 成功后的内存 publication。
`Terminal::render`/writer 已经向操作系统或设备写出的 partial bytes（包括成功 mouse
transition 后后续 frame write 失败留下的 control state）无法回滚，属于 product non-goal；
该 I/O error 必须原样传播，且不能提交任何 prepared in-memory state。此例外不允许把
mouse transition 移回 prepare 阶段。

### 7. 兼容表面

- 不给 `IncrementalLayoutOutcome` 增 required field；新
  `CheckedIncrementalLayoutReport` 承载 recovered cause。旧 outcome 只在兼容 wrapper
  successful/recovered 映射现有三个 bool/count field。
- GH-59 `IncrementalLayoutError` / `DynamicFrameError` 的公开 variant 集合、签名与外部
  exhaustive match 保持可编译；GH-60 checked entrypoints 返回新的
  `#[non_exhaustive] TransactionalLayoutError` / `TransactionalFrameError`。不得通过
  “顺手给旧 enum 加 variant”取得组合能力。
- `Element`、`VNode`、`NodeKey`、`Patch` 公开字段、constructors 与 patterns 不变。
- `get_layout(ElementId) -> Option<Layout>` 保留任意查询的 zero/one 语义；renderer 对
  target-required Element 使用独立 `try_get_required_layout`，不能把 Option None 当 blank。
- GH-59 `try_get_vnode_layout` / `try_get_all_vnode_layouts` 与 legacy ambiguity behavior
  不改变。
- 旧 `compute*` / `apply_patches` / render wrappers 对正常 input 保持结果；final typed
  error fail loudly。App 和 recoverable内部路径只调用 checked API。
- `src/runtime/context.rs` 不修改；pipeline 只在 successful prepared-frame commit 调已有
  setter。若 implementation 需要改 runtime contract，必须先更新本 spec。

### 8. Public rustdoc 与 doctest contract

- GH-60 public declarations 只定义在 `src/layout/incremental_error.rs`、
  `src/layout/engine/transaction.rs`、`src/renderer/error.rs` 与新
  `src/renderer/checked.rs`；四个 dedicated modules 从文件首行使用
  `#![forbid(missing_docs)]`。既有 `mod.rs`/`lib.rs`/`prelude.rs` 只做 exact
  re-export，不在其他 existing module 增加未受 lint scope 覆盖的 public item。
- 每个 checked entrypoint 与 compatibility wrapper 必须带可运行 doctest，禁止
  `ignore` / `no_run` / `compile_fail` 或空 code block；至少覆盖 target-aware success、
  targetless ambiguity、initial-build cause、`#[non_exhaustive]` wildcard match 与
  generalized checked render layout-build cause。
- `tests/gh60_public_docs.rs` 是 external-crate compile fixture，证明 documented exports
  可导入、GH-59 exhaustive enum match 仍编译、新 wrapper 必须 wildcard match；它不替代
  rustdoc doctest。
- 由既有 CI Documentation 与 Doc Tests job 执行
  `RUSTDOCFLAGS='-D warnings' cargo doc` 与 `cargo test --doc`；`forbid(missing_docs)`
  是缺失文档的 fail-closed 边界，不另建 public-API manifest 或自定义 token parser。

### 9. Deterministic fault seam 与覆盖率

真实 Taffy 对合法预检输入的部分 error 很难稳定触发，伪造无效 `NodeId` 又可能在第三方
内部 index 时 panic。`transaction.rs` 定义 crate-private mutation backend：

- production `TaffyMutationBackend` 是唯一启动 wiring，直接委托真实 Taffy并传播 Result；
- `#[cfg(test)] FaultingMutationBackend` 只接受 closed `FaultPoint` enum
  `{Create, SetContext, SetStyle, Remove, SetChildren, ReadBack, Compute, Postcondition,
  RebuildBuild, RebuildCompute, RebuildPostcondition}` 与 occurrence；
- 不公开 trait/object/closure/`Any`，不从 env、user key 或 runtime input 激活；
- exact tests 证明 production constructor 总是 real backend，release surface 无 injector。

fault tests 为 create/update/remove/replace/reorder 各建立“前一个 mutation 已成功、当前点
失败”的 candidate，才能证明 drop candidate 而非“首步失败所以偶然没污染”。state
fingerprint 只在 crate-private tests 比较 root、Taffy total/children/layout、所有 maps、
viewport/frame context，不成为 public snapshot API；随机状态机与性能 benchmark 留给 GH-61。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | prepare/no-op/viewport branch | `cargo test --workspace --lib --locked layout::engine::tests::unchanged_target_and_viewport_is_noop -- --exact`; `cargo test --workspace --lib --locked layout::engine::tests::unchanged_vnode_refreshes_current_element_id_aliases -- --exact`; `cargo test --workspace --lib --locked layout::engine::tests::viewport_only_recompute_is_transactional -- --exact` |
| B-002 | GH-59 checked plan handoff | `cargo test --test incremental_transaction --locked invalid_plan_returns_without_rebuild_or_mutation -- --exact` |
| B-003 | clone candidate apply | `cargo test --test incremental_transaction --locked mixed_batch_failure_commits_no_partial_state -- --exact` |
| B-004 | transaction error locator | `cargo test --workspace --lib --locked layout::engine::tests::each_patch_failure_has_exact_locator_and_cause -- --exact` |
| B-005 | postcondition/root/tree/order | `cargo test --workspace --lib --locked layout::engine::tests::incremental_success_has_target_exact_tree_root_and_order -- --exact` |
| B-006 | subtree/address maps | `cargo test --workspace --lib --locked layout::engine::tests::remove_replace_success_has_no_descendant_or_orphan_state -- --exact` |
| B-007 | candidate drop/fingerprint | `cargo test --workspace --lib --locked layout::engine::tests::failed_or_dropped_candidate_preserves_committed_fingerprint -- --exact` |
| B-008 | recovery dispatcher | `cargo test --workspace --lib --locked layout::engine::tests::commit_failure_attempts_exactly_one_fresh_rebuild -- --exact` |
| B-009 | recovered report | `cargo test --test incremental_transaction --locked recovered_rebuild_preserves_incremental_cause -- --exact` |
| B-010 | double cause/old state | `cargo test --test incremental_transaction --locked rebuild_failure_returns_both_causes_and_preserves_committed_state -- --exact` |
| B-011 | rebuild postcondition | `cargo test --workspace --lib --locked layout::engine::tests::rebuild_success_must_pass_target_exact_postcondition -- --exact` |
| B-012 | deterministic retry | `cargo test --workspace --lib --locked layout::engine::tests::repeated_fault_has_stable_result_and_rebuild_count -- --exact` |
| B-013 | independent wrappers/error families/source | `cargo test --test layout_error_paths --locked text_identity_transaction_and_rebuild_causes_stay_distinct -- --exact`；`cargo test --test layout_error_paths --locked gh59_exhaustive_error_matches_still_compile_with_gh60_wrappers -- --exact` |
| B-014 | required layout lookup | `cargo test --test layout_error_paths --locked missing_layout_reaches_all_checked_render_entrypoints -- --exact`；`cargo test --test layout_error_paths --locked virtual_text_is_filtered_before_required_layout_lookup -- --exact` |
| B-015 | prepared dynamic commit | `cargo test --workspace --lib --locked renderer::pipeline::tests::failure_commits_no_engine_previous_measurement_or_frame -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::terminal_error_drops_prepared_layout_frame -- --exact` |
| B-016 | static/string atomic error | `cargo test --test layout_error_paths --locked static_and_string_layout_failure_returns_no_partial_output -- --exact`；`cargo test --test layout_error_paths --locked checked_renderers_preserve_initial_layout_build_cause -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::mixed_static_and_dynamic_failure_writes_no_terminal_or_static_state -- --exact` |
| B-017 | checked/legacy engine/render API | `cargo test --test layout_error_paths --locked legacy_wrappers_compile_and_fail_loudly_on_final_error -- --exact` |
| B-018 | public struct compatibility | `cargo test --test layout_error_paths --locked public_layout_vnode_patch_outcome_literals_compile -- --exact` |
| B-019 | no ignored result/default | 运行下方 changed-hunk multiline scan；`cargo test --workspace --lib --locked layout::engine::tests::all_backend_failures_are_observed -- --exact`；`cargo test --test layout_error_paths --locked every_required_layout_failure_is_observed_without_fallback -- --exact`；`cargo test --test incremental_transaction --locked legacy_wrappers_delegate_to_checked_core -- --exact` |
| B-020 | private fault seam/safe display | `cargo test --workspace --lib --locked layout::engine::tests::fault_backend_is_test_only_and_diagnostics_are_terminal_safe -- --exact`; `cargo check --workspace --all-targets --all-features --locked` |
| B-021 | serialized prepared commit | `cargo test --workspace --lib --locked renderer::pipeline::tests::cancelled_candidate_cannot_interleave_with_next_batch -- --exact` |
| B-022 | exact-head quality gates | 运行下方 coverage + full Rust/CI/review/SpecRail gates |
| B-023 | merged dependency gate | implementation head 必须包含已合入的 GH-59 |
| B-024 | candidate resource release | `cargo test --workspace --lib --locked layout::engine::tests::candidate_and_recovery_resources_drop_on_every_exit -- --exact`；GH-61 handoff review |
| B-025 | initial frame checked build | `cargo test --workspace --lib --locked layout::engine::tests::initial_frame_success_commits_target_exact_state -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::initial_build_failure_has_no_incremental_cause_or_commit -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::initial_compute_failure_has_no_incremental_cause_or_commit -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::initial_postcondition_failure_has_no_incremental_cause_or_commit -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::initial_prepared_app_frame_success_commits_once -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::initial_build_compute_and_postcondition_failures_write_and_publish_nothing -- --exact` |
| B-026 | scoped raw Patch preflight | `cargo test --test incremental_transaction --locked direct_patch_per_kind_cardinality_is_checked_before_mutation -- --exact`；`cargo test --test incremental_transaction --locked direct_create_and_subtree_collisions_report_exact_ordinal_and_kind -- --exact`；`cargo test --test incremental_transaction --locked direct_batch_dependencies_are_preflighted_in_order -- --exact`；`cargo test --test incremental_transaction --locked direct_patch_ambiguous_target_fails_before_mutation -- --exact`；`cargo test --test incremental_transaction --locked direct_patch_ambiguous_parent_fails_before_mutation -- --exact`；`cargo test --test incremental_transaction --locked direct_patch_apply_report_is_concrete_and_exact -- --exact`；`cargo test --test incremental_transaction --locked legacy_apply_patches_ambiguity_fails_loudly_without_mutation -- --exact`；`cargo test --test incremental_transaction --locked target_aware_patch_failure_rebuilds_once -- --exact` |
| B-027 | whole App frame prepare/commit | `cargo test --workspace --lib --locked renderer::app::tests::mixed_static_and_dynamic_failure_writes_no_terminal_or_static_state -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::mixed_static_and_dynamic_success_commits_once -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::layout_or_render_prepare_failure_emits_no_mouse_or_frame_bytes -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::mouse_mode_change_is_emitted_only_during_prepared_frame_terminal_commit -- --exact` |
| B-028 | public docs/doctests | `cargo test --test gh60_public_docs --locked gh60_public_checked_surface_is_documented_and_compiles -- --exact`；`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked`；`cargo test --workspace --doc --all-features --locked` |
| B-029 | PR base/head coverage binding | 下方 coverage artifact 的 `pr_base_oid`、`coverage_merge_base_sha`、`head_sha` exact assertions |
| B-030 | no-op ElementId alias overlay | `cargo test --workspace --lib --locked layout::engine::tests::unchanged_vnode_refreshes_current_element_id_aliases -- --exact`；`cargo test --workspace --lib --locked renderer::pipeline::tests::unchanged_frame_new_element_ids_render_and_commit_aliases -- --exact`；`cargo test --workspace --lib --locked renderer::pipeline::tests::failed_unchanged_frame_keeps_previous_aliases -- --exact` |

## 数据流

```text
Element + previous VNode + committed LayoutEngine + viewport
  -> GH-58 exact TextFlow inputs + GH-59 checked identity/order plan
  -> plan/preflight error ---------------------------------> typed Err, no rebuild
  -> clone committed engine as candidate
  -> addressed create/update/remove/replace/reorder or viewport recompute
  -> checked TextFlow/Taffy layout compute
  -> target-exact root/reachable/order/maps/layout postcondition
       success -> PreparedLayoutFrame(candidate, VNode, measurements, report)
       failure -> drop candidate
                -> exactly one new LayoutEngine full build from target VNode
                -> checked compute + same postcondition
                   success -> PreparedLayoutFrame(rebuilt, recovered report + old cause)
                   failure -> RecoveryFailed(incremental cause, rebuild cause)

PreparedLayoutFrame
  + prepared static extraction
  -> required-layout tree render into local dynamic Output
  + desired mouse-mode transition
  -> combine as PreparedAppFrame, with no terminal/static/alias publication yet
  -> terminal commit(mouse transition + frame output)
  -> infallible commit static lines + candidate engine + previous VNode
     + ElementId aliases + measurements

raw Patch batch
  -> simulate original-order per-kind cardinality and subtree/dependency changes
     against a read-only virtual scoped-identity state
     missing/ambiguous/collision/dependency/order -> exact ordinal/kind typed Err
     complete ResolvedDirectPatchBatch -> clone/apply/compute/postcondition
       -> DirectPatchApplyReport atomic commit or typed Err, no rebuild
```

没有持久化、网络或外部服务。candidate/error history 只存在一次调用生命周期；terminal I/O
failure 保留旧 in-memory frame。static terminal history 的 exactly-once 属于 GH-66。

## 备选方案

- **直接在 committed engine 上改，失败后 full rebuild**：拒绝。rebuild 也失败时旧状态已
  丢失，无法满足 B-007/B-010。
- **手写 inverse mutation journal**：拒绝作为首个实现。Taffy parent/children/context/cache
  联动，remove/set_children 的逆操作容易遗漏；clone candidate 更容易证明完整隔离。
- **每帧总是 full rebuild**：拒绝。掩盖 patch failure 并破坏 GH-59 的增量身份目标。
- **catch panic 恢复无效 NodeId**：拒绝。preflight 阻止无效 NodeId，确定性 test 使用
  crate-private Result fault seam；panic 不是 typed control flow。
- **把所有新 error 塞进 `TextRenderError`**：拒绝。违反 GH-58 TextFlow-only合同。
- **给 GH-59 exhaustive public enum 追加 GH-60 variant**：拒绝。会使现有外部 exhaustive
  match 产生 source break；使用从首版即 `#[non_exhaustive]` 的独立 wrapper。
- **在 renderer checked error 中只表示 lookup，不表示 layout build**：拒绝。static/string/
  testing initial checked build failure 会被迫 panic/stringify；`CheckedRenderError::LayoutBuild`
  保留 `TransactionalLayoutError` cause。
- **prepare layout 前直接切换 mouse mode**：拒绝。enable/disable 会写 terminal control
  sequence，必须与完整 prepared output 一起延迟到 terminal commit。
- **缺 layout 时跳过 node或返回 default**：拒绝。会把 data/layout corruption 显示成成功。
- **为恢复报告给 `IncrementalLayoutOutcome` 加字段**：拒绝。破坏外部完整 literal。

## 风险

- **Security**：raw user key/control bytes 若进入 error display 可注入终端。只显示 safe
  compatibility token/type/index；fault backend crate-private、cfg(test)，不接受运行时输入。
- **Compatibility**：GH-58/GH-59 将先修改相同 engine/renderer 文件。implementation 必须
  在 merged SHA 重定位；通过新 `#[non_exhaustive]` checked wrapper、旧 exhaustive enum
  compile fixture 与 legacy fail-loud wrapper避免 variant/required-field/signature break。
- **Performance**：非空 batch clone 整个 Taffy tree/maps，时间和内存为 O(n)。本 issue
  优先正确性；GH-61 必须量化 chat streaming/large-tree成本。任何 future optimization
  必须先复用同一 failure/postcondition suite。
- **Maintenance**：prepared frame 同时协调 engine、VNode、measurements 与 terminal I/O。
  commit 必须是小型不可失败 move/swap，禁止在 commit 阶段新增可能失败的逻辑。
- **Dependency**：GH-59 尚是 spec PR，实际 error variants/module split 可能变化。本 spec
  的文件拆分是 planned upper bound，不授权在旧 spec head 实现。
- **Test integrity**：真实 Taffy failure 难触发。fault seam 只模拟 backend Result，不修改
  production assertions；至少一个 public root-cause fixture 必须在旧实现 red。
- **Resource**：candidate clone/rebuild 必须在所有 early return/unwind drop，不保留全局
  history；leak test与 Taffy exact node count postcondition共同锁定。

## 测试计划

- [ ] Root cause：public mixed batch 在旧实现中先成功 update、再失败 target，证明当前
      partial mutation；最终 checked API 返回原子 recovery/error。
- [ ] Unit：no-op/viewport-only/fresh ElementId alias overlay、initial
      success/build/compute/postcondition failure、五类 patch
      fault、compute/read-back/postcondition fault、exact locator、candidate fingerprint、
      fresh rebuild 0/1 次、双 cause、resource drop。
- [ ] Postcondition：missing/extra/orphan/cycle、map mismatch、invalid NodeId、child order、
      missing layout、stale frame context 分别返回 exact invariant variant。
- [ ] Integration：recovered rebuild 与 fresh target等价；rebuild failure保持旧 engine；
      invalid GH-59 plan零 rebuild；raw Patch per-kind cardinality、create/subtree collision、
      batch dependency与 target/parent ambiguity mutation前失败；concrete direct report；
      连续 retry确定。
- [ ] Renderer：dynamic/static/string/testing missing-layout、VirtualText lookup exemption、
      initial layout-build cause、mixed static+dynamic prepare failure、mouse transition延迟、
      terminal I/O failure、partial output/static-lines/aliases/previous/measurement/engine
      non-commit。
- [ ] Compatibility：旧 engine/render signatures、公开 struct literals/patterns、new checked
      exports、GH-59 exhaustive enum matches、legacy final-error fail-loud。
- [ ] Docs：新增 public item 全部 documented，dedicated modules 均受
      `forbid(missing_docs)` 覆盖；rustdoc `-D warnings`、每个 required exact runnable
      doctest、source rejection fixtures 与 external compile fixture 全部通过。
- [ ] Coverage：changed executable >=80%；transaction/rebuild/postcondition line+branch 100%。
- [ ] Full gates：`cargo fmt --all -- --check`；
      `cargo check --workspace --all-targets --all-features --locked`；
      `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
      `cargo test --workspace --all-targets --all-features --locked`。
- [ ] GitHub gates：current exact head CI、independent reviewer artifact、reviewThreads、
      SpecRail `pr_gate` 与 merge authorization evidence。

## 回滚方案

GH-60 implementation 使用独立 PR。若 transaction/recovery/renderer error 引入回归，整体
回滚该 implementation PR，恢复 GH-58/GH-59 merged behavior；不得只关闭 postcondition、
双 cause、missing-layout error 或保留“clone 但提前 commit”的半套路径，否则 silent partial
success 会重新出现。

回滚后 GH-60 保持打开并恢复 `ready_to_implement`，保存失败 exact head、coverage、CI、
review 与 reproduction evidence。GH-61、GH-64、GH-65 等依赖方继续 blocked。若 clone 成本
超出 GH-61 后续阈值，先保留本合同，再单独 spec mutation journal/copy-on-write
optimization；性能问题不授权削弱原子性或 typed errors。
