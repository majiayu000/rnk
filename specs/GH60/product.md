# Product Spec：事务式增量布局与类型化错误

## Linked Issue

GH-60: https://github.com/majiayu000/rnk/issues/60

complexity: large

## 用户问题

当前 `LayoutEngine::apply_patches()` 会逐个修改 Taffy tree 与布局映射，只要批次中任意一个
patch 成功就把整批视为成功。后续 patch 即使因为目标、parent、Taffy 操作或重算失败，
之前的修改也可能已经留下；调用方随后还会更新 previous VNode 与 measurement aliases，
于是一个逻辑帧可能同时包含旧状态和新状态。

full rebuild 也不是可靠的失败边界：现有构建入口会先清空当前 engine，多个 Taffy
`Result` 被忽略。若恢复失败，renderer 又可能把缺失 layout 替换成 `Layout::default()`，
最终呈现空白、零尺寸或旧内容，而不是告诉调用方布局没有成功。

终端聊天会高频执行 create、update、remove、replace 与 reorder，并在 resize、流式文本和
可变高度内容之间连续切换。用户需要的是一帧完整成功或明确失败，不能接受部分 patch、
无限 fallback、静默空白或无法定位的布尔失败。

本规格是 GH-60 的独立产品合同。GH-58 提供 TextFlow 失败边界，GH-59 提供 checked
identity/order plan 与 mutation 前 preflight；本 issue 只接管 preflight 之后的 commit
事务、一次完整重建恢复、最终错误与缺失 layout 的显式传播。

## 目标

- 让每个非空增量批次对 tree、root、布局映射和已计算 layout 具有 all-or-nothing 语义。
- 对 create、update、remove、replace、reorder 及重算/后置校验失败提供可定位的 typed error。
- 增量 commit 失败后只允许一次、基于目标 VNode 的 fresh full rebuild。
- rebuild 成功时暴露已恢复状态及原增量失败原因；rebuild 失败时同时保留两层原因。
- 让成功增量与成功 rebuild 都满足 target-exact tree/map/root/order/layout 不变量。
- 让 dynamic、static、render-to-string 与 testing renderer 不再用默认布局冒充成功。
- 保留现有 `LayoutEngine`、render helper 与公开数据结构的源码兼容入口。
- 以确定性 fault injection、失败恢复测试和 exact-head coverage 证明关键失败路径。

## 非目标

- 不实现跨进程事务、持久化日志或通用数据库式 ACID。
- 不修改 Taffy 自身，也不承诺对任意第三方 panic 做进程级恢复。
- 不重新设计 GH-58 的 TextFlow、Unicode flow cache 或 `TextFlowError` / `TextRenderError`。
- 不重新设计 GH-59 的 identity source、duplicate validation、final-order planner 或成功
  subtree map cleanup。
- 不引入 GH-61 的 `LayoutSnapshot`、随机状态机、benchmark/stress 阈值或 cell 量化。
- 不实现 ChatComposer、MessageList、chat shell 或 provider adapter。
- 不承诺操作系统 terminal write 的字节级回滚；`Terminal::render` 返回错误时必须传播错误并
  保留旧的 engine/previous/measurement state，但底层设备已经接受的部分字节无法撤回。
- 不保证 full rebuild 后复用增量路径原有的内部 Taffy `NodeId`；对外正确性由目标树、布局
  与公开映射合同决定。
- 不把“任何无效输入都尝试 rebuild”当作恢复；GH-59 的 plan/metadata 错误仍直接失败。

## Behavior Invariants

1. **B-001** 当 patch batch 为空、target 与当前 committed VNode 等价且 viewport 未改变时，
   调用必须返回确定的 no-op 结果，不修改 tree、root、scoped/compatibility identity map、
   layout、缓存或 previous VNode，也不得启动 full rebuild；当前 frame 的 `ElementId`
   aliases 按 B-030 处理。若 viewport 改变，则必须把 recompute 作为受同一事务/恢复合同
   约束的候选计算，不能因 patch 为空而复用旧尺寸 layout。
2. **B-002** 当 GH-59 checked plan、identity metadata、duplicate/collision 或 final-order
   preflight 失败时，错误必须在任何 Taffy/map mutation 前返回；这类非法输入不得触发
   full rebuild，也不得被降级为空 patch、成功 frame 或旧 output。
3. **B-003** 当一个非空且已通过 preflight 的批次包含 create、update、remove、replace、
   reorder 中任意组合时，只有全部 mutation、布局计算与批次后置校验成功才能发布增量状态；
   任一步失败都不得让部分新 tree、map、root 或 layout 对 caller 可见。
4. **B-004** 每个增量失败必须以 closed typed contract 定位 patch ordinal、patch kind、
   node key、可用时的 parent key、失败阶段与原因；缺失字段使用明确的 `None` 语义，不能用
   空字符串、布尔值或通用文本代替，诊断中的 user-derived key 必须 terminal-safe。
5. **B-005** 成功增量 commit 后，root 必须存在且可布局；每个 target VNode identity 必须
   精确映射到一个有效、从 root 可达的 Taffy node；每个可达 node 必须恰好对应 target tree
   中一个 identity，且每个 parent 的 child order 逐项等于 checked target order。
6. **B-006** 成功 remove 或 replace 多层 subtree 后，被删除 subtree 的所有 descendant
   scoped identity、compatibility key、ElementId alias 与 NodeId 映射必须消失；新 subtree、
   无关 sibling 和其他 parent scope 保持可查询，成功 map set 精确等于 target tree。
7. **B-007** 当增量 batch 在 mutation、layout compute、read-back 或 postcondition 期间
   失败、取消或中断时，未完成候选状态不得被发布；上一 committed engine 在恢复结果确定前
   保持可用且逐项不变。
8. **B-008** 当且仅当 target-aware checked compute 已通过 preflight、随后增量 commit
   失败时，系统必须基于目标 VNode 与当前 viewport 启动一次 fresh full rebuild；不得在
   已部分修改的候选状态上继续构建，不得尝试第二次 rebuild，也不得递归回到 incremental
   路径。没有 target VNode 的 raw `Patch` 批次不能重建，必须按 B-026 原子失败。
9. **B-009** 当 B-008 的 full rebuild 成功时，调用返回成功的
   `RecoveredFullRebuild` 语义，并保留原 patch failure 的全部 typed locator/cause；
   recovered frame 不得伪装成未发生失败的普通 incremental success。
10. **B-010** 当 B-008 的 full rebuild 也失败时，调用必须返回一个同时携带原 incremental
    failure 与 rebuild failure 的 typed error；不得丢弃任一 cause、返回 blank/default
    layout、发布旧 frame 为新成功，且上一 committed engine 必须保持不变。
11. **B-011** 成功 full rebuild 必须通过与成功 incremental 相同的 target-exact
    tree/root/map/order/layout 后置条件；“Taffy build 返回 node”或“root 有一个数值 layout”
    本身不能视为恢复成功。
12. **B-012** 对相同 committed engine、相同 target VNode、相同 viewport 与相同确定性
    fault，重复执行必须得到同一种 success/error 分类、相同失败阶段与一次 rebuild 计数；
    不得因残留候选状态而在重试时改变结果。
13. **B-013** GH-58 `TextFlowError` / `TextRenderError` 保持 TextFlow-only，GH-59
    `ReconcilePlanError` 保持 plan/identity-only；GH-59 已公开且可穷举匹配的
    `IncrementalLayoutError` / `DynamicFrameError` variant 集合保持不变。GH-60 的 patch、
    Taffy、postcondition 与 rebuild errors 由新的独立、从首版即 `#[non_exhaustive]` 的
    checked wrapper 组合并保留原 `Error::source`，不能追加到 GH-59 枚举或改写成 generic
    I/O 文本。
14. **B-014** renderer 必须在 required layout lookup 前过滤 `Display::None` 和
    `ElementType::VirtualText`；对其余 target tree Element，若没有有效 current layout，
    dynamic、static、render-to-string 与 testing checked entrypoint 都必须返回 typed
    missing-layout error；不得使用 `Layout::default()`、零尺寸、空字符串或上一帧 layout
    冒充成功。
15. **B-015** dynamic frame 只有在 checked layout、所有 required layout lookup、完整
    element rendering、output 构造与整帧 `Terminal::render` 返回成功后，才可一次性更新
    engine、previous VNode 与 runtime measurement aliases；任一失败时三者均不得提交当前
    候选值。terminal I/O 已接受的部分字节按非目标处理，但错误不能被伪装成 frame success。
16. **B-016** static extraction 与 render-to-string 在布局构建、root lookup、递归 child
    lookup 或 TextFlow/render 失败时必须通过可恢复 checked API 返回原 typed cause，且不得
    提交 partial static lines 或返回 partial/blank String；static lines 必须先成为整帧
    prepared candidate，不能在同帧 dynamic 阶段仍可能失败时提前写出或记为 committed；
    checked render error 必须能表达 initial/layout build、compute 与 postcondition cause；
    不能表达新错误的旧入口只允许 fail loudly。
17. **B-017** 现有 `compute`、`compute_vnode`、`compute_element_incremental`、
    `apply_patches`、`get_layout` 与非-try render helper 的公开签名和正常成功行为必须保持
    源码兼容；新增 checked entrypoints 承载 recoverable errors，旧入口遇最终失败或 raw
    key ambiguity 时 fail loudly，不能静默返回 `false`、`None`、空 output 或部分结果。
18. **B-018** 现有公开 `IncrementalLayoutOutcome`、`Element`、`VNode`、`NodeKey` 和
    `Patch` 的外部完整 struct literal/pattern 必须继续编译；recovery 诊断使用独立 checked
    report/error 类型，不给现有公开 struct 增加 required field、不改为
    `#[non_exhaustive]`，也不以 public type alias 隐藏真实类型。
19. **B-019** 所有进入 transaction/rebuild correctness path 的 Taffy `Result`、required
    layout lookup 与 map conversion 必须被完整检查；禁止 `let _ = ...`、`.ok()?`、
    `unwrap_or_default()`、`filter_map` 丢项或 warning + fallback 把失败降格成成功。
20. **B-020** fault injection 必须只能来自 crate-private test seam，production caller
    不能注入任意 closure、`Any`、伪造 NodeId 或绕过 preflight；错误显示不得解释控制字符、
    ANSI、路径或 key 为命令、权限或终端协议。
21. **B-021** 同一个 `LayoutEngine` 的 mutable transaction 仍由单一 caller 独占；已完成
    的 committed/candidate 值可以只读检查，但系统不得通过共享可变全局状态让两个 batch
    交错发布。若一个 batch 被取消，后续 batch 只能从最后 committed state 开始。
22. **B-022** GH-60 只有在 implementation PR 当前 exact head 的正常、五类 patch 失败、
    compute/read-back/postcondition 失败、rebuild success/failure、missing-layout 与
    compatibility tests 全部通过，新代码 changed-line coverage 至少 80%，transaction、
    rebuild、postcondition 的 line/branch coverage 均为 100%，且 CI、independent review、
    reviewThreads 与 SpecRail PR gate 全绿时才可声明完成。
23. **B-023** GH-60 implementation 必须基于已合入 GH-59 implementation 的 exact merge
    commit，并包含其 GH-58 dependency；spec-only PR、未合入 branch 或旧 evidence 不能满足
    依赖。实现开始前必须重新 search duplicate work、核对实际 error/engine 模块与 planned
    paths，任何漂移都需更新本 packet 后再实现。
24. **B-024** transaction 使用的候选资源在 success、recovered success、error、取消或
    panic unwind 后都不得被保留为第二个可观察 engine 或无限增长的 recovery history；
    本 issue 不设性能阈值，但不得以省略原子性、postcondition 或 typed cause 换取速度，
    clone/rebuild 成本由 GH-61 的 benchmark 再量化。
25. **B-025** initial frame 没有 previous committed VNode 时必须走 fresh checked build：
    成功只在完整 build、compute、postcondition 与 prepared-frame publication 后返回
    `InitialFullBuild`；任一 build/compute/postcondition 失败只返回 typed initial-build
    cause，不伪造 incremental cause，不发布 engine、previous、measurements 或 output。
26. **B-026** public raw `Patch` / `apply_patches` 边界必须在 clone 或 mutation 前，把每个
    unscoped `NodeKey` target/parent 按 patch kind 与原始 batch 顺序对 GH-59 scoped identity
    的只读虚拟状态做 checked resolution：create target scoped identity 必须不存在且整个
    subtree 无 collision，update/remove/replace target 与 create/reorder parent 必须恰好
    唯一；先前 patch 创建、替换或删除的 batch-local dependency 必须纳入后续 cardinality。
    任一 missing/ambiguous/already-exists/subtree-collision/order/dependency failure 都必须携带
    B-004 的 patch ordinal/kind/key/parent locator，在 clone 或 mutation 前返回并保持
    committed state。targetless checked API 只提供原子 apply，不能执行 rebuild；需要恢复
    的 caller 必须使用携带 target VNode 与 resolved plan 的 target-aware checked API。
    相同 raw key/type/index 位于不同 parent scope 是合法树状态，不能任选一个节点。
27. **B-027** 同一 App frame 的 static 与 dynamic 内容必须先共同准备为一个
    `PreparedAppFrame`/等价不可见 candidate；任一 static/dynamic layout、lookup、render 或
    output 构造失败时，terminal 零写入且 static committed-lines、engine、previous 与
    measurements 零提交。mouse enable/disable 等 terminal control transition 也必须成为
    prepared terminal operation，不能在后续 layout/render 仍可能失败时提前写出。只有所有
    可失败准备完成后才允许 terminal commit 与内存提交；terminal commit 内实际 I/O 调用
    已经产生的 partial bytes/control state 仍按非目标边界处理。
28. **B-028** 每个 GH-60 新 public checked entrypoint、report、error 与 re-export 都必须有
    可执行且准确的 rustdoc；所有新增 public declaration 必须位于
    `#![forbid(missing_docs)]` 覆盖的 dedicated module，或被 machine-readable public-item
    每个新增 public item 都处于 module-level `forbid(missing_docs)` scope。
    `RUSTDOCFLAGS='-D warnings' cargo doc` 与 `cargo test --doc`
    指定的每个 exact runnable doctest 都必须执行成功；`ignore`、`no_run`、compile-only、
    零匹配或额外未登记 public item 均不得通过。
29. **B-029** changed-line coverage 必须比较 implementation PR 的已验证 base OID 与当前
    exact head 的 merge-base，不得以依赖分支的 merged SHA 充当 diff base；报告必须同时
    绑定 PR base OID、coverage merge-base SHA 与 current head SHA。GH-59 merged ancestry
    是独立 dependency assertion，不替代 coverage base 证明。
30. **B-030** VNode diff 为空不代表当前 `Element` 实例的 `ElementId` 相同。unchanged
    target/viewport frame 必须从当前 Element tree 生成 target-exact alias overlay，并与
    prepared frame 一起在成功 publication 时替换 committed `ElementId` aliases；layout、
    scoped identity 与 Taffy state 保持 B-001 no-op。alias 构造、required lookup、render
    或 terminal 失败时不得发布 overlay；旧 `compute_element_incremental(...);
    get_layout(current.id)` 与 checked renderer 对 fresh current IDs 必须继续成功。

## 验收标准

- [ ] 空 batch、GH-59 preflight error 和每一种合法 success batch 都满足 B-001 至 B-006，
      并以内部 state fingerprint 证明失败前后 committed tree/root/maps/layout 完全相等。
- [ ] create、update、remove、replace、reorder 分别在 mutation 中途被确定性注入失败；
      每例都返回 exact typed locator/cause、发布零部分状态并只启动一次 fresh rebuild，
      覆盖 B-003、B-004、B-007、B-008。
- [ ] rebuild 成功返回含原 patch cause 的 recovered report，最终 tree/map/root/order/layout
      与 target 完全一致；rebuild 失败返回双 cause 且保留旧 committed engine，覆盖
      B-009 至 B-012。
- [ ] TextFlow、identity、patch/Taffy、postcondition、rebuild 与 layout lookup cause 在
      public checked boundary 和 App I/O source chain 中保持可区分，覆盖 B-013。
- [ ] dynamic、static、render-to-string、testing renderer 的 missing-layout negative
      fixtures 均返回 typed error、无 partial output/measurement/previous VNode/frame，
      覆盖 B-014 至 B-016。
- [ ] public compatibility compile tests 覆盖旧 engine/render signatures、完整公开 struct
      literal/pattern、GH-59 exhaustive enum matches、新 GH-60 wrapper 与 fail-loud wrapper，
      覆盖 B-013、B-017、B-018。
- [ ] initial frame success 精确提交 target state；build、compute、postcondition 三类失败
      分别返回无虚构 incremental cause 的 typed initial-build error 且零发布，覆盖 B-025。
- [ ] raw Patch 相同 key 位于不同 parent scope 的 fixture 证明 target/parent ambiguity 在
      mutation 前 typed 失败；create/subtree collision、per-kind cardinality、batch-local
      create→update/remove→update dependency 与 ordinal/kind locator 都有 exact negative
      fixture；targetless report 与 target-aware recovery 边界清晰，覆盖 B-026。
- [ ] initial App/PreparedAppFrame 的 build、compute、postcondition failure 由 writer spy
      证明 terminal/mouse/static/runtime/frame 零发布；initial success 只提交一次，覆盖 B-025。
- [ ] mixed static+dynamic 成功只提交一次；任一准备阶段失败时 mouse/terminal/static/runtime
      均零提交，`VirtualText` 在 required lookup 前过滤，覆盖 B-014、B-027。
- [ ] unchanged VNode 使用 fresh ElementIds 时，成功帧发布 target-exact alias overlay；
      required lookup/terminal failure 保持旧 alias state，覆盖 B-001、B-030。
- [ ] production transaction path 不存在 ignored Taffy Result/default layout/test injector
      暴露，取消和串行重试只从 committed state 开始，覆盖 B-019 至 B-021、B-024。
- [ ] 新增 public surface 全部 documented，每个 required rustdoc exact runnable，
      `ignore/no_run` negative fixture 被拒绝；exact-head coverage、全量 Rust gates、CI、独立
      review、reviewThreads、SpecRail gate、PR base/head artifact 与 GH-59 merged ancestry
      证据满足 B-022、B-023、B-028、B-029。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-001、B-004、B-014、B-025、B-026 |
| 错误与失败路径 | covered: B-002、B-003、B-004、B-007 至 B-011、B-013、B-014、B-016、B-019、B-025 至 B-027 |
| 授权/权限 | N/A：本地布局不读取权限、不执行工具；test seam 不公开且 user-derived 诊断不得被解释为命令或授权（B-020） |
| 并发/竞态 | covered: B-007、B-015、B-021、B-027、B-030 |
| 重试/幂等 | covered: B-008、B-012、B-024、B-026 |
| 非法状态转换 | covered: B-002、B-003、B-008、B-010、B-015、B-025 至 B-027 |
| 兼容/迁移 | covered: B-013、B-017、B-018、B-023、B-028、B-030 |
| 降级/回退 | covered: B-008 至 B-011、B-014、B-016、B-019、B-024 至 B-027 |
| 证据与审计完整性 | covered: B-022、B-023、B-028、B-029 |
| 取消/中断 | covered: B-007、B-021、B-024、B-027、B-030 |

## 发布说明

GH-60 将为可恢复 caller 增加独立且从首版 `#[non_exhaustive]` 的 checked
transaction/rebuild/report 与 generalized render error 表面，不扩展 GH-59 已公开的可穷举
error enum。现有 engine 与非-try render helper 保留正常成功行为；只有过去会静默返回
`false`、`None`、默认布局、空白或部分 output 的最终失败现在会 fail loudly。发布说明必须
列出五类 patch、一次 fresh rebuild、recovered report、双 cause、missing-layout 与旧入口
行为，并明确 GH-58 TextFlow、GH-59 identity/order、GH-61 benchmark 的边界。

本 spec PR 只建立合同，不授权实现。GH-60 implementation 必须等待 GH-59 implementation
合入并重新核对实际路径；最终实现 PR 的 current-head CI、独立 review、reviewThreads、
SpecRail gate 与 merge 证据分别保留。
