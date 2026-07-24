# Tech Spec：keyed 增量身份与子节点顺序

## Linked Issue

GH-59: https://github.com/majiayu000/rnk/issues/59

## Product Spec

见 [`product.md`](product.md)。

本文件只定义 GH-59 的 identity/order 实现边界。它以 GH-58 TextFlow packet 为 stacked
dependency，不修改 TextFlow 算法；通用 patch 事务、失败回滚、失败后的 map 恢复与最终
rebuild typed error 仍属于 GH-60；成功 subtree map 清理属于 GH-59。
`LayoutSnapshot` parity/benchmark 属于 GH-61。

## Codebase Context

以下锚点均在 stacked base `spec/GH58-text-flow`
`6e6e58932a009ab5c205a9227f996b1d4f604b35` 上通过 Read/grep 核实。该 base 的生产代码仍
等于 `origin/main` `e4a89ae128533270d28d768d49977a05a389a582`；GH-58 implementation
合入后，GH-59 implementer 必须重新核对行号和 typed TextFlow boundary。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Public node identity | `src/core/vnode.rs:16`, `src/core/vnode.rs:26`, `src/core/vnode.rs:61` | `NodeKey` 的 fieldwise `Eq/Hash` 包含 index；`matches()` 对 keyed key 忽略 index | 不能再让 fieldwise map identity 与 reconciliation match 互相冲突 |
| Public VNode key builders | `src/core/vnode.rs:157`, `src/core/vnode.rs:205`, `src/core/vnode.rs:212` | `VNode`/`NodeKey` 字段公开，`with_key` 接受通用 `Hash`，`with_index` 写当前位置 | 新设计必须保持源码兼容，并把 index 限定为 keyed position metadata |
| Public props-only key path | `src/core/vnode.rs:102`, `src/core/vnode.rs:128`, `src/core/vnode.rs:170`, `src/core/vnode.rs:219` | `Props::key` 与 `VNode::with_props` 可产生 `props.key: Some`、`NodeKey.user_key: None` 的合法 public VNode | canonical source 表必须接受该组合，不能要求 caller 手工同步两个公开字段 |
| Child diff | `src/reconciler/diff.rs:128`, `src/reconciler/diff.rs:135`, `src/reconciler/diff.rs:180` | old child map 的 `key_identity()` 对 keyed 节点仍包含 index | keyed reorder 会 miss 并产生 create/remove |
| Reorder heuristic | `src/reconciler/diff.rs:173`, `src/reconciler/diff.rs:188` | 仅出现 `to < from` 才发 `Reorder`，只记录局部 moves | insert/delete/forward shifts 没有完整 final-order contract |
| Element identity ingress | `src/layout/engine.rs:223`, `src/layout/engine.rs:252`, `src/layout/engine.rs:262` | synthetic key 混入 ancestor numeric path；child path总是追加 index | keyed ancestor reorder 会改变后代 synthetic identity |
| Engine identity maps | `src/layout/engine.rs:38`, `src/layout/engine.rs:41`, `src/layout/engine.rs:275` | `ElementId -> NodeKey` 与全局 `NodeKey -> NodeId` map 直接使用公开 fieldwise key | same key/different parent 或 moved keyed child 可错失/覆盖 node |
| Create application | `src/layout/engine.rs:318`, `src/layout/engine.rs:355`, `src/layout/engine.rs:363` | `Create` 调 `add_child()`，始终追加到 parent 尾部 | 前插/中插位置依赖后续启发式修正 |
| Reorder application | `src/layout/engine.rs:420`, `src/layout/engine.rs:427`, `src/layout/engine.rs:432` | 从旧 child vector 按局部 moves 覆盖槽位，再 `set_children` | 局部赋值可能重复 NodeId、遗漏 child，不能证明 exact order |
| Existing regression | `src/layout/engine.rs:718`, `src/layout/engine.rs:744` | 只验证跨分支有 layout、reorder 不 fallback；不验证 NodeId 保留或最终顺序 | 测试会放过 map alias、remove/create 和错误排列 |
| Runtime compatibility consumer | `src/renderer/pipeline.rs:34`, `src/renderer/pipeline.rs:60`, `src/runtime/context.rs:428` | renderer 从 engine 取 current NodeKey/layout，再建立 string alias | engine 必须继续提供兼容 current-key view，不要求 consumer 改 API |
| Renderer module table | `src/renderer/mod.rs:37`, `src/renderer/mod.rs:53` | 每个 renderer 子模块必须在此声明，public caller-visible 类型也从此 re-export | 新 `error.rs` 若不接入该文件将无法编译或对 caller 不可见 |

## 设计方案

### 1. 分离公开位置 key 与内部语义身份

保留 `NodeKey` 的公开字段、构造器、`Copy`、fieldwise `PartialEq/Eq/Hash` 和现有
`matches()` 签名。外部 HashMap、struct literal 与 builder 因而继续编译。新增仅 crate
内部使用的 identity：

```text
SiblingIdentity =
  Keyed { token: ExactString | OpaqueHash(u64), compatible_type }
  | Positional { compatible_type, index }

ScopedNodeIdentity =
  Root
  | Child { parent: ScopedNodeIdentity, sibling: SiblingIdentity }
```

- keyed sibling identity 不包含 index；unkeyed identity 必须包含 index。
- `compatible_type` 使用现有 `VNodeType::type_id()` 合同。same key 换不兼容 type 不能 match。
- scoped identity 按逻辑 parent 递归构造。keyed ancestor sibling reorder 不改变其 scope；
  unkeyed ancestor 的 position 变化按既有 positional contract 得到新 scope。
- 每次 `plan_diff(old, new)` / checked layout traversal 都把传入 tree 的 top-level VNode
  直接绑定为 `ScopedNodeIdentity::Root`；该 scope 来自 traversal context，不读取或要求
  top-level `VNode.key == NodeKey::root()`。`VNode::root()` 只是可选显式 container。

Element ingress 保留 `Element.key` 的 exact String 到 `VNode.props.key`，不增加 `VNode` field。
planner 只对非 root children 使用以下 sibling metadata 判定表；child vector 的枚举 index
是 current position truth，`NodeKey.index` 只是输入兼容 metadata，不参与判定并由
compatibility projection 归一为 actual index：

| `VNode.props.key` | `VNode.key.user_key` | 其他校验 | 唯一结果 |
| --- | --- | --- | --- |
| `Some(exact)` | `Some(token)` | `token == hash(exact)`，type 一致 | `Keyed::ExactString(exact)` |
| `Some(exact)` | `None` | type 一致 | `Keyed::ExactString(exact)`；内部派生 compatibility token，不修改 VNode |
| `None` | `Some(token)` | type 一致 | `Keyed::OpaqueHash(token)` |
| `None` | `None` | type 一致 | `Positional { type, actual_index }` |
| `Some(exact)` | `Some(token)` | `token != hash(exact)` | `ReconcilePlanError::KeyMetadataMismatch` |
| 任意 | 任意 | `key.type_id != node_type.type_id()` | `ReconcilePlanError::KeyTypeMismatch` |

top-level root 不进入本表，也不产生 `RootKeyMetadataMismatch`：公开 box/text/component
builders 的默认 `NodeKey`、可选 `VNode::root()` container 都合法。root identity 固定并不
吞掉节点语义：old/new root 的 `VNodeType` 不兼容时生成 root replace；类型兼容时正常比较
text content、props 和 children。顶层 public key/props 不参与 sibling identity 或 duplicate
validation，patch addressing 继续使用现有 public key。对非 root child，
`VNode.key` 与 `props.key` 不存在“实现任选其一”的状态：
`props.key: Some(exact)` 是 canonical exact source；公开 props-only VNode 合法，内部按 exact
string 派生 compatibility token 但不修改输入；同时存在 keyed `NodeKey` 时它是可校验的
compatibility metadata，只有 token mismatch 才 fail closed。`props.key: None` 时才把 keyed
`NodeKey` 当 opaque source，两者都缺失才是 positional。相关 error 只记录 escaped
token/位置/type，不回显未转义 user string。非 root 冲突判定优先级固定为：
`KeyTypeMismatch` -> simultaneous exact/NodeKey token mismatch -> exact/opaque/positional；
同一输入永远只得到首个确定 variant。`hash(exact)` 明确调用与
`NodeKey::with_key(exact.as_str(), type_id, actual_index)` 相同的 compatibility-token 函数。

公开 `NodeKey` 不再作为 engine 内部全局唯一地址。`NodeKey::matches()` 继续提供公开 sibling
match 语义；所有新 engine/diff correctness 路径必须显式使用 `SiblingIdentity` 或
`ScopedNodeIdentity`，禁止直接拿 `NodeKey` fieldwise equality 当 reconciliation identity。

### 2. Search-first duplicate validation

对 old 与 new VNode tree 分别递归做 sibling validation，再生成任何 plan：

- 每个 parent 建立 exact keyed-token set；`None` 不进入 set，`Some("")` 正常进入。
- 同一 exact key/token 第二次出现即返回窄域 `ReconcilePlanError::DuplicateSiblingKey`，
  包含 parent scoped identity、display-safe token/type、first/second index。
- exact string 相等才是 Element key duplicate；不同 exact strings 即使兼容 `u64` view
  碰撞也不得 alias。若需要投影到 `NodeKey` compatibility view 时发生碰撞，返回
  `KeyTokenCollision`，不覆盖 map。
- direct VNode 的 opaque token 没有原值可做二次 equality；同 parent 相同 token 按
  `OpaqueTokenCollision` 拒绝。
- exact-string collision 使用 crate-private token derivation seam：production
  `plan_diff` 注入与 `NodeKey::with_key` 相同的 `default_compatibility_token(&str) -> u64`；
  `#[cfg(test)]` 的 `plan_diff_with_token_source` 接受受控 `Fn(&str) -> u64`，collision test
  让两个不同 props-only exact strings 确定返回同一 token。matcher 仍按 exact string 区分
  两者，只有 compatibility projection 返回 `KeyTokenCollision`；测试不暴力搜索
  `DefaultHasher` collision，也不伪造会先触发 metadata mismatch 的 `NodeKey`。
- metadata/duplicate validation 是只读纯函数；错误发生时还没有 Taffy、`vnode_map`、
  `node_map`、root 或 previous VNode 变更。

这是 GH-59 必须提供的输入诊断，不扩张成 GH-60 的通用 `PatchError`。错误传播使用独立
边界，不修改 GH-58 的 `TextFlowError` / `TextRenderError` variants：

```text
ReconcilePlanError
  -> IncrementalLayoutError::{Identity(ReconcilePlanError), TextFlow(TextFlowError)}
  -> LayoutEngine::try_compute_element_incremental_checked
  -> DynamicFrameError::{
       Incremental(IncrementalLayoutError),
       Text(TextRenderError),
       LegacyLookup(LayoutLookupError)
     }
  -> RenderPipeline::try_render_dynamic_frame_checked
  -> App::render_frame -> io::Error(source = DynamicFrameError)
```

- `src/layout/incremental_error.rs` 定义 `IncrementalLayoutError` 和
  `LayoutLookupError`，都实现 `Error::source`；`src/layout/mod.rs` 导出 public checked
  caller 所需类型。
- GH-58 的 `try_compute_element_incremental` 保留 TextFlow-only 返回类型；它与现有
  `compute_element_incremental` 均委托 checked core。遇 identity/lookup error 时，因为旧
  签名无法表达该 error，必须携带 cause fail loudly，不能 fallback；TextFlow error 仍按
  GH-58 原合同返回。
- `src/renderer/error.rs` 在不改变 `TextRenderError` 的前提下新增独立
  `DynamicFrameError` composite；`src/renderer/mod.rs` 声明 `mod error` 并
  `pub use error::DynamicFrameError`，`pipeline.rs` 的 checked variant 返回它。
- `src/renderer/app.rs` 只调用 checked pipeline，只有成功后才更新 previous VNode、
  measurement aliases 和 terminal frame；失败映射为保留 source chain 的 `io::Error`。
- 旧 `render_dynamic_frame` / GH-58 TextFlow-only try wrapper 保持签名，identity error
  时 fail loudly。GH-60 后续只接管 Taffy apply/rebuild failure variants 与恢复策略。

### 3. 生成 addressed reconciliation plan

保留公开 `diff(old, new) -> Vec<Patch>`、`diff_children(..., &mut Vec<Patch>)` 与现有
`Patch` variants 作为兼容表面，同时新增 public checked adapters 和 crate-private
checked planner；不增加 type alias：

```text
pub try_diff(old: &VNode, new: &VNode)
  -> Result<Vec<Patch>, ReconcilePlanError>

pub try_diff_children(
  old_children: &[VNode],
  new_children: &[VNode],
  parent_key: NodeKey
) -> Result<Vec<Patch>, ReconcilePlanError>

plan_diff(old, new)
  -> Result<ReconcilePlan, ReconcilePlanError>

ParentPlan {
  parent: ScopedNodeIdentity,
  removals: [ScopedNodeIdentity],
  creates: [{ identity, target_index, vnode }],
  updates_or_replaces: [...],
  final_children: [ScopedNodeIdentity]
}
```

`try_diff` 在转换任何公开 patch 前递归校验完整 old/new tree 并生成完整 plan；
`try_diff_children` 对两个 sibling slices 及其后代执行同一 validation/planning，并只返回
新建的 owned vector。metadata mismatch、duplicate/collision 或 invalid plan 返回原始
`ReconcilePlanError`，私有临时 plan/patch buffer 全部丢弃，不存在可观察的空或部分成功。
`try_diff` 把两个函数参数都作为各自 top-level root；root scope 不调用 sibling metadata
validator。public box/text/component roots 因而可进入同一 checked core，旧 `diff` wrapper
对这些合法 roots 正常返回，不触发 fail-loud error path。
每个 new child 通过 `SiblingIdentity` 在同 parent 的 old children 中至多匹配一次。planner
不使用包含 keyed index 的 map key；它为所有受影响 parent 保存完整 `final_children`，
而不是只保存“向前移动”的局部 moves。相同 old/new tree 返回空 plan；同一输入的输出顺序
由 new tree 顺序唯一决定。

`src/reconciler/mod.rs` 公开导出 `try_diff`、`try_diff_children` 和
`ReconcilePlanError`。公开 `diff()` 只调用 `try_diff()`：成功时返回完整 vector，失败时
携带 typed cause panic。公开 `diff_children()` 只调用 `try_diff_children()`：checked
调用成功后才把完整结果一次性 `extend` 到 caller destination；失败时携带 typed cause
panic，destination 连原有内容都保持不变。禁止 catch 后返回 `Vec::new()`、忽略 error、
继续递归或暴露私有 partial buffer。

`Patch::Create.node.key.index` 继续表示目标 index，现有 `Patch::Reorder.moves` 由兼容
adapter 产生，不成为 LayoutEngine 的 correctness source。public checked adapters 与
LayoutEngine 使用同一个 `plan_diff` checked core；LayoutEngine 直接消费
`ReconcilePlan` 并传播 `ReconcilePlanError`，不得调用 legacy panic wrappers。

### 4. 精确 apply identity 与 child order

`LayoutEngine` 把全局 `HashMap<NodeKey, NodeId>` correctness source 改为
`HashMap<ScopedNodeIdentity, NodeId>`，另保留由当前 VNode 投影出的 compatibility
`NodeKey` view，供 `get_vnode_layout`、measurement aliases 与现有 renderer consumer 使用。
compatibility projection 的 keyed `user_key` 是对完整 scoped identity 的 collision-checked
token，所以 same key/type/index 位于不同 parent 时得到不同 composite NodeKey；
`get_all_vnode_layouts()` 必须保留两项。ElementId 查询始终绑定 exact scope。

direct VNode caller 仍可能拿未带 scope 的 raw NodeKey 查询。新增
`try_get_vnode_layout(NodeKey) -> Result<Option<Layout>, LayoutLookupError>`：

- zero semantic match -> `Ok(None)`；
- exactly one -> `Ok(Some(layout))`；
- more than one -> `Err(AmbiguousLegacyNodeKey { key, scoped_match_count })`。

旧 `get_vnode_layout(NodeKey) -> Option<Layout>` 委托 try variant；ambiguity 时 fail loudly，
绝不任取一个或伪装成 `None`。新增 `try_get_all_vnode_layouts()` 在 composite projection
collision 时返回 `CompositeKeyCollision`；旧 all-layouts wrapper fail loudly，正常 same-key/
different-parent 则返回包含两个 composite keys 的 map。pipeline 使用 try-all variant并通过
`DynamicFrameError::LegacyLookup` 传播。Element tree 的精确 caller 继续优先使用
`get_layout(ElementId)`。

每个 `ParentPlan` 先构造不触碰 Taffy 的 `ResolvedParentPlan`：

1. 校验 `final_children` identity 唯一；重复项返回
   `ReconcilePlanError::DuplicateFinalIdentity { parent, identity, first_index, second_index }`。
2. 每个 final identity 必须恰好来自 existing survivor 或 planned create；两边都没有返回
   `MissingFinalIdentity`，两边同时提供返回 `DuplicateFinalIdentitySource`。
3. 每个 survivor/create 必须在 final list 恰好出现一次；未出现返回
   `ExtraPlannedIdentity`。
4. existing survivor 必须在 scoped NodeId map 中可解析；缺失返回
   `MissingExistingNodeId`。以上任一 error 都发生在 create/remove/set_children 前，并断言
   Taffy node count/children、所有 maps、root 和 previous VNode 逐字段不变。
5. preflight 成功后才进入 commit：创建 planned subtree、移除 direct child、把 surviving/
   created NodeId 按 resolved final slots 组装为 exact vector，一次调用
   `set_children(parent_id, exact_node_ids)`。
6. 立即读取 `taffy.children(parent_id)` 比较 exact vector；创建/remove/set_children/read-back
   失败是 GH-60 的 commit/rollback 范围，不伪装成 GH-59 preflight error。
7. 成功 read-back 后，以 target VNode/ResolvedParentPlan 和已解析 NodeId 构造新的 scoped
   map、composite/legacy projection 与 ElementId aliases，再一次性替换 compatibility
   views；所有 remove/replace subtree descendant 和 cross-parent old-scope entries 都必须
   缺失，新 subtree/new scope 与无关 siblings 保留。成功 map key set 与 target tree
   identity set 做 exact postcondition。commit 任一步失败时如何恢复旧 Taffy/maps 仍由
   GH-60 定义，但成功路径不得把 stale cleanup 推迟给 GH-60。

因此前插、中插、尾插、remove + move、swap、reverse 和 multi-position reorder 共用同一
final-order primitive，不再需要 `needs_reorder()`。surviving compatible keyed identity
继续指向原 NodeId；same key/different type 使用 replacement NodeId；cross-parent move
出现在两个不同 ParentPlan 中，按 remove + create 处理。

### 5. 文件拆分与 GH-58 协作

当前 `src/layout/engine.rs` 已超过 800 行。GH-59 不继续向该文件堆叠：

- `src/layout/engine/incremental.rs` 独占 scoped Element/VNode traversal、checked plan apply
  与 order/map-set postcondition；其 line/branch coverage 必须为 100%。
- `src/layout/engine/incremental_order.rs` 独占 final-order preflight/resolve；该新 critical
  module 的 line/branch coverage 必须为 100%。
- `src/layout/engine/tests.rs` 承载能访问 private Taffy/NodeId 的 identity/order unit tests。
- `src/reconciler/identity.rs`、`src/reconciler/plan.rs` 分别承载 identity source table 与
  pure checked plan，便于对 critical modules 做 100% line/branch gate。
- `src/reconciler/diff/tests.rs` 承载 pure planner/duplicate/checked public adapter/
  fail-loud compatibility diff tests，并通过 crate-private controlled token source 构造
  exact-string projection collision。
- `tests/keyed_incremental_identity.rs` 只用现有 public Element/LayoutEngine surface 建立
  root-cause 与 full-rebuild layout parity。
- `tests/keyed_incremental_error_paths.rs` 锁定 public checked layout error variant/source；
  public `rnk::renderer::DynamicFrameError` re-export 与 pipeline/App exact unit negative
  锁定 end-to-end caller propagation。

GH-58 也计划修改 `src/layout/engine.rs`。GH-59 implementation 开始前必须基于 GH-58 merged
head rebase，保留其 TextFlow cache、frame context 与 `try_compute*` cause chain，再移动
incremental 代码；不得恢复 GH-58 删除的旧 measurement 算法或新增平行 engine。

### 6. 兼容与范围边界

- `NodeKey`、`VNode`、`Patch` 公开字段/构造函数及 unkeyed positional behavior 不移除。
- `diff()` / `diff_children()` 保留函数签名；新增 public `try_diff` /
  `try_diff_children` 直接返回 `ReconcilePlanError`，不通过 type alias 隐藏错误。
- legacy wrappers 只在 checked 结果完整成功后返回/追加 patches；invalid input 时 fail
  loudly。LayoutEngine 与 renderer 路径只调用 checked core，不捕获 panic wrapper。
- `Props::key` + `VNode::with_props` 的 props-only exact key 保持有效；不要求 caller 额外
  调用 `VNode::with_key`，也不修改 public builder 行为。
- `VNode::root()` 保持可选；public `box_node` / `text` / `component` 可直接作为根。
  `ScopedNodeIdentity::Root` 由 traversal context 建立，不把 root 当 sibling 校验。
- `runtime/context.rs` 不修改；`renderer/pipeline.rs` / `app.rs` 只为 checked identity/
  lookup error propagation 改动，TextFlow-only error 类型不加 identity variant。
- 不修改 GH-58 `text_flow.rs`、renderer output/projection、chat component 或 examples。
- 不在本 issue 实现 apply rollback、一次 full rebuild 或 missing-layout renderer error；
  这些在 GH-60。GH-59 保证输入 validation 在 mutation 前完成，并且只有 exact order 与
  target-exact map-set postcondition 都成立的批次可报告成功。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `SiblingIdentity`, checked matcher | `cargo test --workspace --lib --locked reconciler::diff::tests::keyed_match_ignores_position_within_parent -- --exact` |
| B-002 | recursive `ScopedNodeIdentity` | `cargo test --workspace --lib --locked layout::engine::tests::keyed_ancestor_reorder_preserves_descendant_identity -- --exact` |
| B-003 | scoped engine map | `cargo test --workspace --lib --locked layout::engine::tests::same_key_in_distinct_parents_has_distinct_nodes -- --exact` |
| B-004 | planner + engine NodeId reuse | `cargo test --workspace --lib --locked layout::engine::tests::keyed_insert_delete_and_moves_reuse_survivor_nodes -- --exact` |
| B-005 | compatible-type matcher / replace | `cargo test --workspace --lib --locked reconciler::diff::tests::same_key_incompatible_type_is_replace -- --exact` |
| B-006 | positional identity domain | `cargo test --workspace --lib --locked reconciler::diff::tests::mixed_keyed_unkeyed_keeps_positional_contract -- --exact` |
| B-007 | final-order postcondition | `cargo test --workspace --lib --locked layout::engine::tests::taffy_child_order_equals_target_vnode_order -- --exact` |
| B-008 | complete `ParentPlan::final_children` | `cargo test --workspace --lib --locked reconciler::diff::tests::plan_contains_total_hole_free_final_order -- --exact` |
| B-009 | pre-mutation duplicate validation | `cargo test --workspace --lib --locked layout::engine::tests::duplicate_sibling_key_fails_before_mutation -- --exact` |
| B-010 | exact/opaque token handling | `cargo test --workspace --lib --locked reconciler::diff::tests::raw_hash_collision_never_aliases_exact_keys -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::opaque_token_collision_is_error -- --exact` |
| B-011 | empty/missing key boundaries | `cargo test --workspace --lib --locked reconciler::diff::tests::empty_key_is_keyed_and_duplicate_is_error -- --exact` |
| B-012 | deterministic plan + multi-frame state | `cargo test --workspace --lib --locked reconciler::diff::tests::identical_tree_has_empty_deterministic_plan -- --exact`；`cargo test --test keyed_incremental_identity --locked consecutive_frames_match_full_rebuild -- --exact` |
| B-013 | cross-parent semantics | `cargo test --workspace --lib --locked layout::engine::tests::cross_parent_move_is_remove_and_create -- --exact` |
| B-014 | public compatibility | `cargo check --workspace --all-targets --all-features --locked`；`cargo test --test keyed_incremental_identity --locked public_node_key_and_patch_surface_compiles -- --exact` |
| B-015 | exact-head quality gates | full fmt/check/clippy/test；执行下方 exact-head Cobertura checker；fresh CI/reviewThreads/PR gate |
| B-016 | GH-58 dependency handoff | 人工/queue gate 核对 GH-58 implementation merged SHA 早于 GH-59 implementation base，且 GH-59 exact head 包含该 SHA |
| B-017 | pure planning / cancellation boundary | `cargo test --workspace --lib --locked reconciler::diff::tests::discarded_plan_mutates_no_engine_state -- --exact`；GH-60 scope review |
| B-018 | VNode identity source table | `cargo test --workspace --lib --locked reconciler::diff::tests::vnode_key_metadata_decision_table -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::mismatched_key_metadata_and_type_are_typed_errors -- --exact` |
| B-019 | independent checked error chain | `cargo test --test keyed_incremental_error_paths --locked duplicate_key_reaches_checked_layout_boundary -- --exact`；`cargo test --workspace --lib --locked renderer::app::tests::duplicate_key_reaches_app_io_error_without_frame_commit -- --exact` |
| B-020 | composite legacy lookup | `cargo test --workspace --lib --locked layout::engine::tests::same_raw_key_across_parents_has_two_composite_layouts -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::raw_legacy_lookup_reports_typed_ambiguity -- --exact` |
| B-021 | final-order preflight atomicity | `cargo test --workspace --lib --locked layout::engine::tests::invalid_final_order_variants_fail_before_mutation -- --exact` |
| B-022 | coverage/property evidence | `cargo test --workspace --lib --locked reconciler::diff::tests::property_mixed_key_permutations_are_bijective_or_typed_error -- --exact`；运行下方 exact-head Cobertura 命令并断言 checker `decision=allowed` |
| B-023 | checked public diff 与 fail-loud compatibility adapters | `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_invalid_nested_metadata_returns_error_without_partial_patches -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_children_duplicate_returns_error_without_patches -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_fails_loudly_on_invalid_input -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_children_fails_loudly_without_mutating_destination -- --exact` |
| B-024 | successful subtree map cleanup | `cargo test --workspace --lib --locked layout::engine::tests::successful_remove_cleans_descendant_identity_maps -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::successful_replace_cleans_old_descendant_identity_maps -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::cross_parent_move_cleans_old_scope_without_deleting_new_scope -- --exact` |
| B-025 | traversal-derived root scope / public root compatibility | `cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_box_root -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_text_root -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::try_diff_accepts_public_component_root -- --exact`；`cargo test --workspace --lib --locked reconciler::diff::tests::legacy_diff_accepts_public_non_container_roots -- --exact`；`cargo test --workspace --lib --locked layout::engine::tests::checked_layout_accepts_public_box_text_component_roots -- --exact` |

coverage job 的 `continue-on-error` 与 Codecov upload 的 `fail_ci_if_error: false` 不能满足本
packet。GH-59 verification 必须在 implementation PR exact head 的 clean worktree 运行：

## 数据流

```text
old/new VNode trees
  -> bind each top-level argument to traversal-derived ScopedNodeIdentity::Root
  -> checked plan core (sibling metadata table applies only below root)
  -> validate complete VNode key/props/type/index decision table
  -> recursively derive exact/opaque SiblingIdentity under parent scope
  -> validate duplicate/collision for every sibling list
  -> deterministic ReconcilePlan { addressed ops, complete final_children }
       |-> public try_diff / try_diff_children
       |     -> atomically convert valid plan to owned patch vector, or return typed error
       |     -> legacy wrappers commit complete result or fail loudly
       |-> LayoutEngine checked caller
             -> preflight final identity sources + existing NodeIds (no mutation)
  -> checked IncrementalLayoutError boundary
  -> set_children(exact target vector)
  -> read-back exact order postcondition
  -> atomically refresh target-exact scoped/ElementId/composite compatibility maps
  -> assert removed subtree/old scope absent and map identity set equals target tree
  -> checked DynamicFrameError -> App io::Error
```

没有持久化、网络或进程外状态。`Element.key` 只作为本地 exact token；错误文本不得包含用户
内容，只包含 display-safe key/token 与结构位置。公开 hash token 不能作为权限或安全身份。

## 备选方案

- 直接让 `NodeKey` 的 `Eq/Hash` 在 keyed 情况忽略 index：拒绝。它仍缺少 parent scope，
  会让不同 parent 的 same key 在全局 map 中 alias，并破坏依赖 fieldwise Eq/Hash 的外部代码。
- 继续用 ancestor numeric path 生成 synthetic hash：拒绝。keyed ancestor reorder 仍会改变
  后代 identity，且 hash collision 无法做 exact-string validation。
- 只修 `key_identity()` 去掉 index：拒绝。diff 可能 match 正确，但 engine 的全局 map、
  append create 与局部 reorder 仍不正确。
- 总是 full rebuild：拒绝。它能偶然给出最终布局，却丢失 keyed identity，违背本 issue 核心目标。
- 保留仅返回 `Vec<Patch>` 的唯一 public diff 入口，并在 validation error 时返回空 vector
  或已有 partial patches：拒绝。它把非法输入伪装成 no-op/成功，且
  `diff_children(..., &mut patches)` 会污染 caller destination；checked public API 必须
  先得到完整结果，legacy wrapper 只能 success 后提交或 fail loudly。
- 在 GH-59 同时实现通用 transaction/rollback：拒绝。该范围属于依赖本 issue 的 GH-60。

## 风险

- Security：user key 只参与本地身份；诊断必须转义控制字符且不得执行/解释 key。exact string
  避免攻击者用已知 hash collision 静默覆盖另一分支。
- Compatibility：`NodeKey`/`Patch` 已公开。通过内部 plan 而非改公开字段布局，保留源码
  兼容；compat compile test 锁定 constructors、struct patterns、props-only key、
  box/text/component roots、checked/legacy diff signatures 与 public `DynamicFrameError`
  re-export。
  旧 unscoped lookup 在新增合法 same-key/different-parent 状态下 fail loudly；recoverable
  caller 使用 try variant，不能继续接受旧 HashMap 覆盖。
- Performance：递归 scoped identity 若反复 clone 完整路径会退化。实现应使用 arena/index
  或共享不可变 parent handle，使 plan/validation 对节点数和 sibling 数为 O(n)，不得为每个
  node 重建 O(depth) String path。性能阈值与 benchmark 在 GH-61。
- Maintenance：公开 `Patch` 与内部 `ReconcilePlan` 可能漂移。`diff()` 必须复用同一
  checked core，测试断言 checked/legacy adapters 对 keyed create/remove/replace 的语义
  一致，并用 negative exact tests 锁定 typed error/no-partial/fail-loud；engine
  correctness 只依赖 checked plan。
- Dependency：GH-58 会改 `LayoutEngine`。实现必须在 GH-58 merged head 上重新锚定、编译，
  不能按本 spec 写作时的行号机械套 patch。

## 测试计划

- [ ] Unit：VNode metadata decision table、keyed/unkeyed match、duplicate/collision、
      public checked diff typed errors、legacy diff fail-loud/no destination mutation、
      traversal-derived root scope 与 box/text/component public roots、
      missing/duplicate/extra final order、same-key type replace、nested scope、NodeId reuse、
      successful remove/replace/cross-parent subtree map cleanup、composite lookup、exact
      Taffy order/map-set postcondition。
- [ ] Integration：公开 API 下的前/中/尾插、删除、swap/reverse、多位置移动、混合列表、
      same key/different parents、checked typed error、连续多帧与 full rebuild layout parity。
- [ ] Property：bounded proptest 生成 unique/duplicate keyed/unkeyed sibling permutations；
      unique input 的 plan/final order 为 bijection 且 deterministic，duplicate input 只得到
     对应 typed error并无 mutation。无界 fuzz/benchmark/stress 留给 GH-61。
- [ ] Compatibility：`NodeKey`/`VNode`/`Patch` 公开构造与现有 runtime measurement alias
      callers 编译、原 reconciliation/layout tests 全绿。
- [ ] Coverage：执行上方 exact-head Cobertura + fail-closed checker；changed executable
      lines >=80%，四个新 critical modules line/branch 均为 100%，JSON evidence head
      必须等于 implementation PR head。
- [ ] Full gates：`cargo fmt --all -- --check`；
      `cargo check --workspace --all-targets --all-features --locked`；
      `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`；
      `cargo test --workspace --all-targets --all-features --locked`。

## 回滚方案

GH-59 implementation 使用独立 PR。若新 planner 出现回归，整体回滚该 implementation PR，
恢复原 public `diff`/engine 行为；不得只关闭 duplicate validation 或 order postcondition，
否则会让错误重新静默成功。回滚不修改已合入的 GH-58 TextFlow，也不关闭 GH-59；issue
恢复 `ready_to_implement` 并保留失败 exact head、CI、review 与复现证据。若问题属于 apply
原子性而 identity/order plan 正确，转交 GH-60，不在 GH-59 隐藏失败。
