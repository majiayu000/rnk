# Product Spec：keyed 增量身份与子节点顺序

## Linked Issue

GH-59: https://github.com/majiayu000/rnk/issues/59

complexity: large

## 用户问题

当前 `rnk` 对 keyed 节点同时存在互相冲突的身份规则：`NodeKey::matches()` 在 user key
存在时忽略 sibling index，但 child diff 的 map identity 又包含 index；Element 转 VNode
时还把祖先的数字路径写入 synthetic key。一个 keyed child 只要换位置，就可能被当作新节点，
而 keyed ancestor 的 sibling index 变化也可能连带改变整棵后代树的身份。

顺序执行同样没有完整合同：`Create` 只向 Taffy parent 尾部追加，`Reorder` 仅在启发式认为
出现“向前移动”时生成。前插、中插、删除与多节点移动组合后，增量树的 child order 可能不再
等于当前 VNode，即使最终布局仍能返回数值也不能算成功。

终端聊天会频繁在消息列表中插入 streaming block、状态行与 tool result。应用需要 keyed
节点在同一逻辑父级内跨帧保留身份，同时每批成功增量更新都严格实现声明的最终顺序。

本规格是 GH-59 的独立产品合同。GH-57 只提供 umbrella 范围，GH-58 只提供共享 TextFlow
前置；两者都不能替代本 packet 的验收。

## 目标

- 为 keyed 与 unkeyed sibling 定义唯一、父级作用域内的跨帧身份规则。
- 让 sibling index 只描述当前顺序，不再参与 keyed 节点的跨帧匹配。
- 让每个成功增量批次的 Taffy child order 与当前 VNode child order 完全一致。
- 对重复 sibling user key、token collision 和不可能的目标顺序给出确定、可定位的失败。
- 保留现有 unkeyed 按位置匹配和公开 `NodeKey` / `VNode` / `Patch` 构造方式。
- 用连续多帧、混合 keyed/unkeyed 和 full-rebuild parity 证据证明身份与顺序合同。

## 非目标

- 不实现 GH-60 的通用 patch 事务、Taffy 失败回滚、失败后的 map 恢复或最终 rebuild typed
  error；不重构 raw `Patch` topology，也不实现 whole-frame terminal transaction、rollback、
  rebuild 或 fallback。成功 remove/replace/move 的完整 subtree map 清理仍是 GH-59 正确性范围。
- 不实现 GH-61 的 `LayoutSnapshot`、随机状态机 parity harness 或 benchmark 阈值。
- 不改变 Taffy 的 Flexbox 算法、GH-58 TextFlow 或 renderer cell composition。
- 不要求 user key 在整棵树、不同 parent、不同应用或不同进程中全局唯一。
- 不支持在两个 parent 之间移动节点并保留原身份；parent scope 改变即是 remove + create。
- 不把未提供 key 的节点自动猜成 keyed，也不按内容、样式或 `ElementId` 猜测身份。

## Behavior Invariants

1. **B-001** 当 sibling 同时具有 user key 时，跨帧匹配身份必须是
   `{stable parent scope, exact user-key token, compatible VNode type}`；其旧/new sibling
   index 只表示位置，不得进入 keyed match。
2. **B-002** parent scope 必须递归来自逻辑父节点身份：root scope 固定；keyed parent 在
   同一 parent 内重排后 scope 不变；unkeyed parent 继续由 compatible type + sibling
   position 定义。祖先的显示路径字符串或旧 sibling index 不得混入已稳定 keyed ancestor
   的后代身份。
3. **B-003** 相同 user key 可以分别出现在不同 parent 下，且各自独立匹配、更新和删除；
   任一 parent 的操作不得覆盖另一 parent 的 node/layout 映射，也不得要求应用改成全局 key。
4. **B-004** 当 keyed child 在同一 stable parent 内发生前插、中插、尾插、删除、交换或
   跨多位置移动时，所有仍存在且 type compatible 的 keyed child 必须复用原 Taffy node
   identity；不得退化为对应 child 的 remove + create，也不得把另一 child 的状态或布局
   别名转移给它。
5. **B-005** 当同一 user key 的节点类型变为不兼容的 `VNodeType` 时，该节点必须执行
   replace 语义，不得复用旧类型的节点或状态；type compatible 的文本内容/props 更新继续
   使用既有 update/replace 内容合同，不得改变 sibling identity。
6. **B-006** 未提供 user key 的 child 必须继续按
   `{stable parent scope, compatible VNode type, sibling index}` 匹配。keyed 与 unkeyed
   identity domain 永不相等；混合列表中 keyed child 的移动不得误消费相邻 unkeyed child。
7. **B-007** 每个成功增量批次完成后，对每个受影响 parent，从 Taffy 读取的 child NodeId
   序列必须逐项等于当前 VNode child identity 序列对应的 NodeId；只拥有相同集合、相同数量
   或“看起来布局相同”都不满足顺序合同。
8. **B-008** `Create` 必须落实 child 的目标 index；前插和中插不能先永久追加到尾部。
   reorder 计划必须表达完整、无重复、无空洞的最终排列，不能依赖“只有向前 move 才需要
   reorder”的启发式，也不能用会互相覆盖槽位的局部 move assignment 冒充最终顺序。
9. **B-009** 同一 parent 的新 sibling list 出现重复 user key 时，必须在任何 Taffy 或
   identity map 修改前确定性拒绝；诊断至少包含 parent scope、冲突 key/token 与两个
   sibling index。节点类型不同不豁免重复 key，错误不得降级为成功 full rebuild。
10. **B-010** 对由 `Element.key: String` 产生的 key，identity 必须比较 exact string，
    不得仅凭可能碰撞的 `u64` hash 判定相等。直接通过通用 `VNode::with_key(Hash)` 产生的
    opaque token 若发生相同 token 冲突，必须按 B-009 拒绝并标明 opaque-token collision，
    不得静默别名。
11. **B-011** `key: None` 是 unkeyed；显式 `key: Some("")` 是有效 keyed token。空字符串
    可在不同 parent 各出现一次，但同一 parent 重复空 key 仍按 B-009 失败；缺失 children
    与空 children list 都产生确定的空最终顺序。
12. **B-012** 相同 old/new tree 重复 diff 必须得到空计划；同一有效目标 tree 从同一 old
    tree 重算必须得到逐项等价的确定计划。连续多帧 reorder 不得累积旧 index、重复 child、
    stale identity 或额外 create/remove。
13. **B-013** keyed child 改变 parent scope 时必须按旧 parent remove、新 parent create
    处理；即使 user key 和类型相同也不能跨 parent 偷渡复用。该规则与 B-003 共同避免状态
    在分支之间泄漏。
14. **B-014** 现有公开 `NodeKey` 字段、构造器、`matches()`、`VNode` builders、
    `diff()` / `Patch` 表面和未 keyed 行为必须保持源码兼容。内部 engine 可以使用新的
    scoped identity/plan，但不得要求应用为已有 Element/VNode 增加字段或迁移 key 格式。
    runtime 现有 ElementId、raw node identity 与 string-key measurement helpers 保持签名；
    无歧义调用继续返回该唯一 scoped measurement，只有新增合法多 parent 候选时才由 checked
    API 返回 typed ambiguity、legacy helper fail loudly。
15. **B-015** GH-59 只有在当前 implementation head 的 unit、integration、连续多帧和
    full-rebuild parity 测试通过，新代码 patch coverage 至少 80%，identity match、
    duplicate validation 与 final-order planning/apply 分支达到 100%，且 exact-head CI、
    independent review、reviewThreads 和 SpecRail PR gate 全绿时才可声明完成。零匹配
    test filter、旧 SHA 或只检查 patch count 的测试不算证据。
16. **B-016** GH-59 implementation 必须在 GH-58 implementation 已完成并合入其 TextFlow
    合同后开始；若 rebase 后 GH-58 改变 `LayoutEngine` 路径或 typed `try_compute*` 边界，
    GH-59 必须重新核对 planned paths 和错误包装，不得复制或绕过 TextFlow 状态。
17. **B-017** identity/order plan 生成必须是无共享可变状态的纯计算；计划生成被取消或
    中断时不得修改 Taffy 或映射。计划已生成但尚未 apply 时可安全丢弃；apply 中途失败、
    rollback 与一次 full rebuild 恢复仍由 GH-60 定义，GH-59 不把未完成 apply 报告为成功。
18. **B-018** 对每个非 root child 上可独立修改的 `VNode.key` 与 `VNode.props.key`，系统
    必须只有一个确定 sibling identity source 判定表：`props.key: Some(exact)` 是
    canonical exact source；
    `NodeKey.user_key: None` 的 props-only public VNode 必须合法并由 exact string 派生
    compatibility token；若同时存在 keyed `NodeKey`，其 token 必须与 exact string 一致；
    只有 keyed `NodeKey` 是 opaque token；两者都缺失是 positional。exact metadata 与已有
    `NodeKey.user_key` 或节点类型不一致时必须返回 typed metadata error；实际 child vector
    position 是唯一 current index truth，公开 `NodeKey.index` 不得反向覆盖它。系统不得任选
    一边、修改输入字段或给 `VNode` 增加 required/private field。
19. **B-019** GH-58 的 `TextFlowError` / `TextRenderError` 保持 TextFlow-only。GH-59 必须
    另设 caller-visible checked incremental boundary，以独立 composite error 保留
    `ReconcilePlanError` 或 GH-58 text cause；既有签名无法表达 identity error 的 wrapper
    必须 fail loudly。dynamic App 路径不得把 duplicate/collision/mismatch 变成成功 frame、
    full rebuild、空 output 或被更新的 previous VNode。
20. **B-020** scoped identity 投影为 legacy `NodeKey` 时必须包含稳定 parent scope 并检测
    投影 collision。同 key/type/index 位于两个 parent 时，`get_all_vnode_layouts()` 必须
    同时保留两个不同 composite key；以无 scope 的 raw legacy key 查询多个候选时，新
    `try_get_vnode_layout` 返回 typed ambiguity，旧 `get_vnode_layout` fail loudly，不得
    任取一个、返回误导性 `None` 或覆盖另一项。engine 的 composite projection 必须继续
    完整保留；renderer 只在 layout/render 全部成功后把 validated scoped layouts、raw
    candidates 与 string aliases 发布给 `RuntimeContext`。runtime raw/string measurement
    lookup 的 0/1/N 合同为：无候选返回 `None`；全局唯一候选返回该 exact scoped value；
    多个 parent-scoped candidates 时 checked API 返回 typed ambiguity，legacy wrapper fail
    loudly。每个成功 frame 必须原子替换 ElementId、scoped、composite/raw candidate 与 string
    alias views；unique -> ambiguous -> unique 转换不得遗留任一旧 ElementId、raw candidate
    或 alias，identity validation 失败则不得发布任何新 measurement state。
21. **B-021** `final_children` 在任何 Taffy/map mutation 前必须完成集合和现有映射 preflight：
    missing final identity、duplicate final identity、planned survivor/create 未出现在 final
    order（extra）分别返回 typed error，并证明 tree/map/root/previous VNode 未变。新节点
    创建、remove、`set_children` 或 read-back 的 Taffy commit 失败及其 rollback 仍由 GH-60。
22. **B-022** B-015 的覆盖率证据必须由绑定 implementation PR exact head 的 Cobertura
    artifact 和 fail-closed checker 机器判定：compiler-observed production changed executable
    lines >=80%，`#[cfg(test)]` code 不进入分母/分子，coverage-only suppression 必须被拒绝；
    未展开的 macro invocation 内任一非 literal coverage/suppression token，以及
    production-active macro token stream 内任一 `path` 或 production-excluded module
    identifier，必须保守拒绝；tracked declarative
    macro 通过动态 attribute name（`#[$meta ...]`）合成控制属性时同样必须拒绝，
    普通同名 declaration/call 仅在 macro 外允许；任一非 literal/comment `include` source
    token（包括 import/re-export alias 与 `include!`）必须拒绝；默认 `mod name;`、
    inline-module context 或 `#[path]` 解析到非 `.rs`、absolute、parent-traversing 或
    production-excluded source target 时必须拒绝；production-excluded `.rs` target
    仅在 enclosing item 被证明为 test-only 时允许；POSIX/Windows absolute target
    与 tracked HEAD symlink/file redirect 均必须 fail closed；production source roots
    下被 ignore 的 `.rs` 必须拒绝，防止 clean-status 隐藏 HEAD 外编译源码；
    新 identity/plan/incremental-order/incremental-apply critical modules 的 production
    line/branch rate 均为 100%。collector 必须直接执行并绑定已哈希的 pinned coverage plugin，
    不能经 Cargo alias 解析到其他命令。resolved coverage plugin、nightly Cargo/rustc 及其
    sysroot LLVM tools、Git/Python/OS/runner 是受信基础设施；receipt 只绑定其中已记录的
    plugin/Cargo/rustc/Git entrypoints 与 coverage artifacts，不承担在无外部签名或
    known-good digest 下自证 vendor authenticity 或证明未记录的基础设施。
    同一 head 还必须通过 bounded property test，覆盖 unique 与 duplicate 的 keyed/unkeyed
    permutation；无界 fuzz、性能随机状态机与长期 stress 留给 GH-61。
23. **B-023** reconciler 必须公开 `try_diff` 与 `try_diff_children`（或等价、无 type alias
    的 checked API），以 `Result<Vec<Patch>, ReconcilePlanError>` 表达 metadata mismatch、
    duplicate/collision 和无效计划；任一错误都不得返回空或部分 patch vector。既有
    `diff()` / `diff_children()` 保持源码兼容，但只能委托 checked API 并在错误时 fail
    loudly；尤其旧 `diff_children(..., &mut patches)` 只能在 checked 结果完整成功后一次性
    追加，失败时不得修改调用者已有的 patch vector。LayoutEngine 与其他可恢复 caller
    必须使用 checked boundary，不得依赖 panic wrapper。
24. **B-024** 每个成功 remove、replace 或 cross-parent move 必须清除被移除 subtree 的全部
    scoped identity、NodeId、legacy/composite layout projection 与 ElementId alias；成功批次
    的可查询 map identity 集合必须精确等于 target VNode tree，不得留下已移除后代或旧 parent
    scope。replace 后只保留新 subtree，cross-parent move 只清除旧 scope 且不得误删新 scope
    或无关 sibling。Taffy commit 中途失败后的 transactional map rollback 才由 GH-60 负责。
25. **B-025** `ScopedNodeIdentity::Root` 必须由每次 old/new tree 的 top-level traversal
    context 隐式派生，不得要求顶层 VNode 的 public `NodeKey` 等于 `NodeKey::root()`。
    `VNode::root()` 只是可选的显式 container builder；现有 `VNode::box_node()`、
    `VNode::text()`、`VNode::component()` 均可直接作为合法 tree root。顶层节点不进入
    B-018 的 sibling metadata table；root type 相同按正常 props/content/children diff，
    type 不兼容按 root replace。checked diff/layout 与旧 wrapper 均不得因合法 public root
    返回 `RootKeyMetadataMismatch`、panic 或 fallback。

## 验收标准

- [ ] keyed 前插、中插、尾插、删除、交换、reverse 和跨多位置移动均保留 surviving
      compatible child 的 NodeId，且没有对应 remove/create，覆盖 B-001、B-004、B-005。
- [ ] nested keyed ancestor reorder 后后代 identity 保持；同 key 分处两个 parent 时映射互不
      覆盖；跨 parent 移动则明确 remove/create，覆盖 B-002、B-003、B-013。
- [ ] 混合 keyed/unkeyed 与纯 unkeyed fixtures 锁定位置语义，keyed move 不消费 unkeyed
      child，覆盖 B-006、B-011、B-014。
- [ ] insert/delete/reorder 组合及连续多帧更新后，逐 parent 的 Taffy NodeId order 精确等于
      VNode target order，并与 full rebuild 的布局和顺序一致，覆盖 B-007、B-008、B-012。
- [ ] 重复普通 key、重复空 key、不同类型重复 key、raw-string hash collision fixture 和
      opaque-token collision fixture 均在 mutation 前失败，诊断包含 parent/key/indices，
      覆盖 B-009、B-010、B-011。
- [ ] `VNode.key` / `props.key` 的全部判定组合和 type mismatch 均得到唯一 exact/opaque/
      positional 结果或 typed error；其中 public props-only keyed VNode 保持合法，stale
      public index 由 child vector position 确定归一；checked layout -> dynamic frame -> App caller
      保留 identity cause，兼容 wrapper fail loudly；既有六分支 `PatchFailure` 仍可被旧 caller
      无 wildcard 穷举匹配，新 `DirectPatchError` 保持 non-exhaustive，覆盖 B-014、B-018、B-019。
- [ ] 两个 parent 下相同 key/type/index 通过 public all-layouts 查询同时可见；raw legacy
      single lookup 返回 typed ambiguity，旧 wrapper fail loudly；runtime raw/string
      measurement lookup 对 0/1/N 分别返回 None/exact scoped value/typed ambiguity，legacy
      measurement wrapper 在 N 时 fail loudly；unique -> ambiguous -> unique 成功发布会原子
      清除旧 ElementId、scoped/composite/raw candidate 与 alias，identity 失败不发布，
      覆盖 B-003、B-014、B-019、B-020。
- [ ] missing/duplicate/extra final identity negative injection 在 mutation 前失败并保持
      tree/map/root/previous VNode 逐项不变，覆盖 B-017、B-021。
- [ ] invalid nested VNode metadata 与 duplicate sibling 分别令 `try_diff` /
      `try_diff_children` 返回精确 typed error 且没有 patch vector；旧 `diff` /
      `diff_children` fail loudly，后者即使失败也不改变预置 destination，LayoutEngine
      直接传播 checked cause，覆盖 B-018、B-019、B-023。
- [ ] successful subtree remove/replace/cross-parent move 后，removed descendant 的 scoped/
      legacy/ElementId 查询全部消失，目标/无关 scope 仍可查询且 map key set 精确等于 target
      tree，覆盖 B-003、B-012、B-013、B-020、B-024。
- [ ] public box/text/component VNode 分别作为 top-level root 时，checked diff/layout 正常
      完成且 old wrapper 不 panic；`VNode::root()` 仍可选，只有非 root children 执行 sibling
      metadata validation，覆盖 B-014、B-018、B-023、B-025。
- [ ] 当前 head 的覆盖率、全量 Rust gates、CI、独立 review、review threads 与 SpecRail
      gate 满足 B-015、B-022；bounded permutation property 与 same-key type replace exact
      test 通过，handoff 证明 GH-58 dependency 满足 B-016。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-011、B-012 |
| 错误与失败路径 | covered: B-009、B-010、B-015、B-017、B-018、B-019、B-021、B-022、B-023、B-024 |
| 授权/权限 | N/A：本地 reconciliation 不读取权限、不执行工具或外部请求 |
| 并发/竞态 | covered: B-012、B-017、B-021、B-024 |
| 重试/幂等 | covered: B-012、B-017、B-021、B-024 |
| 非法状态转换 | covered: B-005、B-009、B-013、B-018、B-021、B-024 |
| 兼容/迁移 | covered: B-006、B-014、B-016、B-019、B-020、B-023、B-025 |
| 降级/回退 | covered: B-009、B-017、B-019、B-020、B-021、B-023、B-024 |
| 证据与审计完整性 | covered: B-015、B-016、B-022 |
| 取消/中断/部分完成 | covered: B-017、B-021、B-023、B-024 |

## 发布说明

本变更不要求应用迁移现有 key API。已有 keyed Element/VNode 在同一 parent 内移动时将真正
保留增量身份，same-key/different-parent 也不再发生内部 map 覆盖；unkeyed 节点继续按位置
匹配。发布说明必须指出：重复 sibling key（包括空 key）现在会显式失败；跨 parent move
仍是 remove + create；公开 `NodeKey` 保持兼容，但 engine 的正确性由内部 scoped identity
和完整 final-order plan 保证。所有 patch/Taffy 失败的事务恢复合同仍由 GH-60 ownership
覆盖，本 corrective 不重构该 topology。
metadata 不一致、legacy lookup 歧义和无效 final-order plan 会返回新的 typed error；GH-58
TextFlow error 类型不因此扩张。reconciler 同时提供返回 `ReconcilePlanError` 的 checked
public diff API；旧 diff API 只保留为 fail-loud compatibility wrapper，不会把错误伪装成
空或部分 patches。成功 remove/replace/move 会同步清除完整旧 subtree 映射；GH-60 只接管
失败提交的 rollback。`VNode::root()` 不是必需 sentinel，公开 box/text/component builders
仍可直接作为 root。runtime measurement 的 ElementId、raw identity 与 string helper 签名
保持兼容：0/1/N 候选分别表现为 None、唯一 exact scoped value、checked typed ambiguity；
legacy helper 在歧义时 fail loudly。成功 renderer publication 原子替换全部 measurement
views，unique -> ambiguous -> unique 不保留 stale ElementId/scoped/composite/raw/alias；
identity 失败不发布。engine 的
composite projection 仍完整保留。raw `Patch` topology、whole-frame terminal transaction、
rollback/rebuild/fallback 继续由 GH-60 定义。coverage 结论只接受 exact-head checker artifact。
