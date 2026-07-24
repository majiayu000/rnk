# Product Spec：统一 LayoutSnapshot、终端 Cell 量化与聊天布局基准

## Linked Issue

GH-61: https://github.com/majiayu000/rnk/issues/61

complexity: large

## 用户问题

当前 `rnk` 的 full-tree、incremental、static、testing 与 `render_to_string()` 路径虽然都使用
Taffy，但它们仍直接读取可变 `LayoutEngine`，并在 renderer 中各自把浮点坐标和尺寸转换为
终端整数 cell。相同目标树可能因此因入口、更新策略、父级偏移、scroll 或 resize 不同而得到
不同的可见边界；缺失 layout、负坐标和溢出还可能被默认值或截断掩盖。

终端 AI Chat 会持续执行 streaming delta、消息追加、中部插入、可变高度 transcript、
CJK/emoji 重排和 resize。用户需要一个唯一、不可变、后端无关的 `LayoutSnapshot` 作为布局
与绘制边界，并需要可复现的正确性、工作量、分配与耗时证据来判断 incremental 路径是否真的
保持语义和成本，而不是只看通用整树 microbenchmark。

本规格是 GH-61 的独立产品合同。GH-58 提供 TextFlow 与 frame-local projection，GH-59
提供 scoped identity/final order，GH-60 提供 prepared candidate、transaction/recovery 与
整帧提交；GH-61 只把这些已完成结果投影为统一 cell snapshot、验证 parity 并建立 benchmark
门，不重新实现三个上游合同。

## 目标

- 让 dynamic、static、testing 与 string renderer 只消费同一类不可变 snapshot，不再各自
  读取、量化或补全浮点 layout。
- 用稳定 identity、确定 child order、signed half-open cell bounds、content bounds、
  effective clip、scroll transform 与 TextFlow semantic revision 完整描述一帧布局。
- 证明 full、incremental 与 recovered-full 对同一 target/viewport 产生语义等价 snapshot。
- 对负坐标、非有限值、溢出、裁剪、边框、scroll 与 resize 给出 fail-closed 的 cell 语义。
- 用确定性 work counters、allocation evidence 和抗噪声的 paired benchmark 建立聊天负载
  baseline 与后续回归阈值。
- 保持现有 `Layout`、engine getters、render wrappers、testing helpers 与公开 struct
  literal 的源码兼容。

## 非目标

- 不改变 Taffy 的 Flexbox/Grid 算法，也不实现新的通用约束布局器。
- 不重新设计 GH-58 TextFlow segmentation、source map、cache key 或 Unicode width policy。
- 不重新设计 GH-59 identity/order plan，或削弱 GH-60 clone-staging、一次 rebuild 和
  prepared App frame 原子提交。
- 不实现 MessageList 高度索引、虚拟化、ChatComposer、shell 或模型 provider。
- 不承诺 incremental 在所有规模和场景都快于 full；本 issue 先建立可解释 baseline 和
  回归门，性能优化不能牺牲正确性。
- 不用 benchmark 结果替代 snapshot parity、typed error 或 transaction 测试。
- 不保证不同终端字体的像素级一致，只保证选定 terminal cell 模型中的一致结果。

## Behavior Invariants

1. **B-001** 当任一 checked layout 路径完成一棵可渲染 target tree 时，必须产出一个完整、
   不可变的 `LayoutSnapshot`；renderer、measurement 与 TextFlow projection 只能读取该
   snapshot，不得再次从 live `LayoutEngine` 解释浮点 geometry。
2. **B-002** snapshot 必须按 GH-59 target final child order 保存每个 layout node，并使用
   GH-59 scoped semantic identity 作为跨 full/incremental 路径的主身份；Taffy `NodeId`、
   frame-local `ElementId` 或遍历地址不得成为 semantic equality 的唯一依据。
3. **B-003** GH-61不得假设GH-59/GH-60存在未声明的“committed visible plan”。engine侧只依赖
   GH-59 `ReconcilePlan` / `ResolvedParentPlan::final_children`与GH-60
   `PreparedLayoutFrame`、target-exact postcondition和required-layout lookup；不得改变其
   tree/map/layout集合。render-required snapshot target由GH-61新增且唯一拥有的crate-private
   adapter从target Element tree按GH-60 B-014既有规则派生：`Display::None`在required-layout
   lookup前停止renderer traversal，`VirtualText`免lookup，其余节点按target child order查询
   GH-60 prepared candidate。full、incremental、recovered与所有renderer必须共用该adapter；
   missing layout走GH-60 typed lookup error，不能补删engine entry或发明上游visibility状态。
4. **B-004** 每个 snapshot node 必须提供 signed、整数、半开区间的 border bounds 与 content
   bounds；width/height 必须由同一对已量化 edges 相减，禁止分别转换 position 和 extent。
5. **B-005** 量化必须先累计 parent-relative geometry、父级 transform 与 scroll 得到绝对
   浮点 edges，再对 start/end 使用同一单调规则；相同 edge 在同一 frame 必须得到相同 cell，
   且量化不得给原本不重叠的 half-open sibling rectangles 引入重叠。
6. **B-006** 对原始 gap、border、padding 与 content geometry，量化误差必须局部有界且
   确定；content bounds 必须包含于对应 border bounds，空 content 可表示为空半开区间，
   不能通过负 width/height 或 wraparound 表示。
7. **B-007** 负坐标和 scroll 后位于 viewport 外的坐标必须在 snapshot 中保持 signed；
   只有与 terminal/effective clip 求交后才允许转换为 output cell。负值不得提前 clamp 到 0，
   正溢出不得 saturate 到 `u16::MAX` 后冒充可见。
8. **B-008** 每个 node 的 effective clip 必须按 x/y 轴独立计算为 terminal bounds、祖先
   axis clip stack 与本节点 axis overflow clip 的确定交集。某轴 `Overflow::Visible`
   不在该轴添加本节点 content span，`Hidden` / `Scroll` 只在该轴添加 content span；
   例如 `overflow_x=Hidden, overflow_y=Visible` 只裁剪 x，不能顺带裁剪 y。嵌套 mixed-axis
   overflow 必须逐轴组合，任一轴空交集表示不可见而不是错误。
9. **B-009** scroll transform 必须作为 signed cell vector 记录，并只影响 descendant
   projection；它不能改变自身 border/content bounds、TextFlow logical row count 或 stable
   identity。scroll、祖先 clip 或 terminal bounds 变化必须在当前 frame 产生新 projection。
10. **B-010** snapshot node 的 TextFlow 引用必须绑定当前 source/style/content-width 与
    Unicode policy 的 semantic revision；engine-local allocation address、cache slot 或单调
    generation counter 只能进入非语义诊断，不能让 cold full 与 cache-hit incremental
    snapshot 产生伪不等。
11. **B-011** frame producer strategy、patch count、incremental cause、rebuild count、
    cache-hit 与 timing counters 必须位于独立 build/report evidence 中；它们不得进入
    `LayoutSnapshot` semantic equality。
12. **B-012** 对同一 target tree、viewport 和 TextFlow policy，initial/full、
    successful incremental 与 GH-60 `RecoveredFullRebuild` snapshot 必须逐节点语义等价：
    identity、order、bounds、clip、scroll 与 TextFlow semantic revision 全部一致。
13. **B-013** parity 必须覆盖 unchanged、text/style update、streaming delta、append、
    front/middle insert、remove、replace、keyed reorder、mixed keyed/unkeyed、CJK、emoji ZWJ、
    combining sequence 与 variable-height transcript；只验证单一静态树不算完成。
14. **B-014** 状态机必须使用tech spec公开的exact seed列表、SplitMix64算法、每步固定draw
    数与operation权重；每个seed执行64步，并在每次合法操作后分别从committed
    incremental 与 fresh full 路径生成 snapshot 并比较；失败时必须报告 seed、step、
    operation 与首个 differing identity/field，禁止只输出“snapshot 不相等”。
15. **B-015** resize 改变 width 时必须在同一 frame 反映 TextFlow reflow、cell bounds 与
    clip；只改变 height/scroll/clip 时 logical TextFlow 可复用，但 snapshot projection 必须
    更新。resize 往返后，相同 target/viewport 必须恢复相同 semantic snapshot。
16. **B-016** dynamic、static、testing、`render_to_string()` 与 public checked render helper
    必须从同一 snapshot contract 得到 bounds；任何入口都不得保留第二套
    `as u16` / default-layout / recursive float-offset 语义。
17. **B-017** dynamic App 的 snapshot 必须作为 GH-60 `PreparedAppFrame` 的不可见 candidate
    构建；只有 layout、snapshot、render、static/dynamic output 与 terminal commit 全部成功
    后，才可与 engine、previous VNode、aliases 和 measurements 一次发布。
18. **B-018** snapshot 构建、quantization、required lookup、TextFlow revision 或 renderer
    失败时必须返回保留原 source chain 的 typed error；candidate snapshot、output、
    measurements、aliases、static lines 与 previous VNode 均不得部分发布。
19. **B-019** 没有previous committed state的initial checked build只能执行一次
    `InitialFullBuild`。GH-60已有initial layout failure必须保持
    `TransactionalFrameError::Transaction(TransactionalLayoutError::InitialBuild(...))`；
    GH-61 snapshot failure走同一`Transaction` wrapper的新增non-exhaustive composition，
    renderer failure走既有`Render(CheckedRenderError)`，不得虚构
    `TransactionalFrameError::Initial`。所有initial failure均`rebuild_count=0`且不得进入
    recovery。只有已有committed state且incremental transaction在GH-60 recovery seam失败时
    才允许exactly-once recovered full；成功后renderer只消费recovered candidate snapshot，
    不得消费failed incremental snapshot或再运行未记录布局，失败则零发布。
20. **B-020** snapshot 计算被取消、panic unwind 或提前 drop 时，未完成 builder state 不得
    可见；已发布 snapshot 可被多个只读消费者安全共享。公开 `LayoutSnapshot` /
    `SnapshotNode` storage 必须 private，只能由 crate-private checked builder 构造；public
    API 仅提供 read-only accessor/iterator，禁止 public field、任意-state constructor、
    setter、`DerefMut`/`AsMut`/`IndexMut`。frame alias overlay 必须结构上独立于 semantic
    snapshot；重复读取不得修改 order、maps、clip、TextFlow projection 或 counters。
21. **B-021** NaN/Infinity、负extent、edge算术溢出、cell coordinate overflow、反向
    content bounds、missing/duplicate identity、missing layout/TextFlow revision、snapshot
    target/tree contract、frame alias与renderer cell conversion必须分别返回可穷举的
    closed semantic variant，并携带identity、field/axis/operation、frame/alias与原始值。
    `LayoutSnapshotError`、`LayoutAliasError`与`SnapshotRenderError`均须定义完整
    variant/payload；跨snapshot、renderer和GH-60实际
    `Upstream`/`Transaction`/`Render` wrapper的`From` / `Error::source()`必须保留concrete
    nested cause，禁止虚构`Initial` variant或使用string/`Other` catch-all；错误不得被
    `None`、空snapshot、default rect、warning或旧snapshot降级。
22. **B-022** 现有 `Layout`、`LayoutEngine::get_layout/get_all_layouts`、
    `compute*`、非-try renderer、`TestRenderer` 正常成功行为及公开完整 struct
    literal/pattern 必须继续编译；新增 checked snapshot API 承载可恢复错误，旧入口遇最终
    错误必须 fail loudly。
23. **B-023** unchanged target/viewport 可复用已发布 immutable snapshot，但必须采用 GH-60
    当前 frame 的 target-exact `ElementId` alias overlay；复用不能保留 stale frame aliases，
    也不能把 report counters 伪装为新计算。
24. **B-024** benchmark scenario 集必须严格等于
    `unchanged_frame`、`streaming_delta`、`append_message`、`middle_insert`、
    `variable_height_transcript`、`resize_invalidation`。除 `unchanged_frame` 明确禁止
    recovery 外，每个 scenario 都必须记录 full、incremental、recovered；每个允许组合的
    operation/sample必须非零且达到tech spec固定最小次数，`median_ns`、allocation count/bytes、
    visited/mutated nodes、TextFlow recomputes、snapshot nodes与rebuild count必须完整且满足
    各strategy的非负/非零约束（unchanged incremental允许`mutated_nodes=0`）。artifact row
    按scenario/strategy/batch聚合，因此每个recovered row必须
    `rebuild_count == operation_count`，所有非recovered row必须为0。缺组合、额外名称、
    零operation/sample或counter缺字段时artifact无效。
25. **B-025** benchmark 必须使用确定的 seed、target size、viewport 序列、message corpus、
    toolchain、Cargo.lock、profile、runner fingerprint、warmup、sample/batch 数与 exact
    head/base SHA；setup/tree construction 不得混入被计时 operation，full/incremental 必须
    对同一 target 与等价起点测量。
26. **B-026** PR required gate必须先执行无时钟依赖的parity、work-counter与allocation
    checks；存在trusted canonical baseline时，timing比较必须在同一runner内对base/head采用
    交错paired batches，只有回归同时超过20%与50µs、并在3个batch中至少2个复现时才失败，
    单次outlier不得阻断。首次implementation严格走B-028 bootstrap route，不伪造compare。
27. **B-027** allocation count/bytes 相对 trusted base 同时超过 10% 与绝对
    8 allocations / 4096 bytes时必须失败。compare 的 base 只能来自 PR exact base tree 中
    repo-owned baseline，且其 source SHA 必须是 PR base 的 ancestor、不得等于 current head；
    runner/schema/corpus/scenario/toolchain fingerprint 不一致、stale/self/untrusted baseline、
    缺失/负 counter 都必须 blocked 或明确 `needs_rebaseline`，不得判 green。
28. **B-028** GH-61 implementation PR首次引入benchmark时只运行bootstrap并生成绑定exact
    head的non-authoritative candidate artifact，验证固定scenario matrix、schema、nonzero
    operations/samples与counter completeness；它不得新增/修改canonical baseline，也不得
    伪称“无性能回归”。`.github/benchmarks/gh61-baseline.json`的唯一writer是implementation
    合入后的独立baseline-promotion PR；该PR须在exact merged implementation SHA的隔离
    checkout重新运行bootstrap（不得仅改写candidate SHA），并通过独立review、current CI、
    SpecRail gate与merge authorization后合入default branch。首次implementation required job
    只允许`bootstrap_valid`/`comparison_status=not_available`/`promotion_required=true`，
    明确不是performance green。checker不得自行提升，feature/promotion head不得信任自身baseline。后续compare
    只能读取PR base tree已存在且通过ancestry/content-hash验证的canonical baseline；文件尚未
    promotion时必须`needs_rebaseline`。
29. **B-029** GH-61 只有在当前 implementation head 的 unit、property、seeded state-machine、
    all-entrypoint parity、error、compile-immutability、compatibility 与 benchmark-contract
    测试通过，新代码 executable changed-line coverage 至少80%，且 snapshot、quantizer、
    parity、error 核心文件 line/branch 都为100%且 denominator非零时才能声明完成。public API
    manifest必须双向匹配，所有指定 doctest须 exact执行、非`ignore`、非`no_run`且非零；
    coverage/docs artifact必须绑定GitHub PR exact base/head。所有required exact Rust helper
    必须汇总证明`1 passed; 0 ignored`；零匹配、仅listed、ignored、旧SHA、人工截图、
    compile-only docs或只运行benchmark不算证据。
30. **B-030** GH-61 implementation 必须基于 GH-58、GH-59、GH-60 三个 implementation
    PR 的已合入 exact SHA，并重新核对 merged public types、module paths 与 prepared-frame
    boundary；任一依赖只是 spec head、open PR 或未合入 branch 时 implementation 保持 blocked。

## 验收标准

- [ ] full/incremental/recovered parity fixtures 与 seeded state machine 在每一步比较完整
      semantic snapshot，覆盖 B-001 至 B-015。
- [ ] cell fixtures 覆盖 fraction、shared edge、nested offset、border/padding、negative
      scroll、mixed-axis 与 nested mixed-axis overflow、empty clip、u16 terminal boundary、
      NaN/Infinity 与 overflow，覆盖 B-004 至 B-009、B-021。
- [ ] dynamic、mixed static/dynamic、testing 与 string checked renderer 都只消费 snapshot；
      failure spy 证明 terminal/runtime/previous/static 零部分提交，覆盖 B-016 至 B-020。
- [ ] compatibility/compile fixtures 编译旧 engine/layout/renderer/testing surface，证明
      snapshot/node字段与任意-state constructors无法从crate外访问；compile-fail必须由
      独立exact trybuild测试实际执行并匹配checked-in stderr；旧wrapper最终失败fail loudly，
      覆盖 B-020、B-022、B-023。
- [ ] benchmark artifact 完整覆盖固定六类chat workload及其明确strategy matrix（unchanged
      不含recovery）、minimum operations、时间/allocation/work counters、trusted
      baseline、bootstrap/promotion与regression negative fixtures，覆盖 B-024 至 B-028。
- [ ] exact merged dependency SHA、current-head coverage、全量 Rust、CI、independent review、
      reviewThreads 与 SpecRail `pr_gate` 证据满足 B-029、B-030。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-003、B-008、B-021、B-024 |
| 错误与失败路径 | covered: B-003、B-007、B-018、B-019、B-021、B-024 至 B-028 |
| 授权/权限 | N/A：本地布局与 benchmark 不读取权限、不执行用户工具；exact-head/runner evidence 完整性由 B-025 至 B-030 约束 |
| 并发/竞态 | covered: B-017、B-018、B-020、B-023 |
| 重试/幂等 | covered: B-012、B-014、B-015、B-019、B-023、B-026 |
| 非法状态转换 | covered: B-003、B-017 至 B-021、B-028、B-030 |
| 兼容/迁移 | covered: B-010、B-016、B-022、B-023、B-028、B-030 |
| 降级/回退 | covered: B-018、B-019、B-021、B-026 至 B-028 |
| 证据与审计完整性 | covered: B-011、B-014、B-024 至 B-030 |
| 取消/中断 | covered: B-017、B-018、B-020 |

## 发布说明

GH-61 将新增公开但不可变的 snapshot/cell/checked error surface，并让所有 renderer 入口收敛
到这一边界。现有 float `Layout` 与旧 render/testing helper 保留；它们成为 snapshot 或
checked producer 的 compatibility projection/wrapper。发布说明必须列出 signed half-open
cell 语义、semantic parity、typed overflow behavior、benchmark bootstrap 与回归阈值，并明确
它不交付聊天组件、虚拟列表或新的 Taffy 算法。
