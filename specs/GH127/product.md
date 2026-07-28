# Product Spec：线性 styled boundary 归一化

## Linked Issue

GH-127: https://github.com/majiayu000/rnk/issues/127

## 用户问题

当前 TextFlow 在每个 source grapheme 上都会重新扫描全部 styled ranges，并再次扫描每个
range 的 start/end boundary。一个 grapheme 对应一个 styled range 时，range 数量翻倍会让
这段归一化工作接近四倍增长；长对话或高密度富文本因此可能在真正换行前就消耗大量 CPU。

本 issue 只收敛 styled range 已通过 typed validation 后的 style 选择与 boundary
diagnostic 归一化。优化必须保持 GH-58 已发布的完整 TextFlow 行为，包括 grapheme 首 source
style、`StyleBoundaryNormalized` 的内容/顺序/重数、typed range errors、source map、cache
identity 与 interruption 原子性。

## 目标

- 把 validation 后的 styled normalization work 限制为 source graphemes 与 styled
  ranges/boundaries 数量的线性函数，消除逐 grapheme 全量重扫。
- 保持 combining sequence、emoji ZWJ sequence、adjacent/empty ranges、未排序合法 ranges
  及默认 style 的当前可观察语义。
- 用 2k/4k/8k deterministic operation counter 同时覆盖普通 ASCII ranges、跨
  combining/ZWJ EGC 的内部 boundary，以及大量 event 集中在单个 EGC 的 skew；debug/release
  使用同一逻辑上界，wall-clock 只能作为非门控补充。
- 保持 invalid/overlapping range 的 closed typed errors、exact source/style/cache
  identity、cancellation/interruption 与上一已发布 cache 的原子性。
- 与当前 main 已合入的 #126、#128、#129、#130 及现有 TextFlow property/engine
  split-style contracts 兼容；不修改 #126 的 wrap interruption paths/assertions。

## 非目标

- 不改变 public `TextFlowInput`、`StyledTextRange`、`TextFlowDiagnostic`、
  `TextFlowError`、`TextFlowCacheIdentity` 或 `TextFlow` 的类型形状。
- 不改变 wrap、truncate、renderer projection、LayoutEngine flow publication、
  VNode cleanup 或 clean-context invalidation 算法。
- 不把合法 range 自动排序、合并、去重或写回 caller input；原始输入顺序继续属于 cache
  identity。
- 不用 wall-clock benchmark、release-only threshold 或特定机器耗时替代 deterministic
  operation count。
- 不在本 issue 中完成 #126；它只建立 implementation ordering 与 regression boundary。

## Behavior Invariants

1. **B-001** 对已通过现有 typed range validation 的输入，styled normalization phase 的
   operation count 必须满足 `O(G + R)`，其中 `G` 是 source extended grapheme cluster
   数，`R` 是 caller 提供的 styled range 数；实现不得在任一 grapheme 内重新遍历全部
   ranges 或全部 boundary endpoints。
2. **B-002** 独立 deterministic performance gate 必须对 2,000、4,000、8,000 三档规模
   分别运行：(a) 一 non-empty range/一 ASCII grapheme；(b) combining/ZWJ EGC 内部的
   adjacent、empty 与 shared endpoint events；(c) 大量 strict-interior events 集中于单个
   combining EGC 的 skew。每个 family/size 必须满足
   `operations <= 12 * (G + R) + 64`，且相邻两组满足
   `next_operations <= 2 * previous_operations + 128`。计数项必须至少包含 grapheme merge
   step、plan endpoint visit、style-range cursor advance、boundary endpoint visit 与 ordered
   diagnostic projection；内部 boundary families 必须断言 projection count 非零并等于
   exact expected event count。结果不得依赖时钟、CPU、optimizer 或采样噪声。
3. **B-003** B-002 gate 必须在 debug 与 release 使用相同输入、计数定义和整数上界；
   任一模式失败都必须显示 fixture family/size、`G`、`R`、internal/projected event counts、
   每类 operation breakdown、observed total、absolute bound 及前后两组 density，不得只
   输出“too slow”或以另一模式成功替代。
4. **B-004** 每个 source grapheme 的 style 必须继续取其第一个 source byte 所落入的唯一
   non-empty styled range；没有 range 覆盖该 byte 时使用 exact caller
   `default_style`。range 在 grapheme 后续 byte 才开始时不得覆盖首 source style。
5. **B-005** styled range 的 start 或 end 严格落在 grapheme source range 内部时，系统
   必须继续产生 `StyleBoundaryNormalized { boundary, grapheme_range }`；boundary 等于
   grapheme start/end 时不产生该 diagnostic，且不得把 split combining/ZWJ boundary
   变成 error。
6. **B-006** diagnostics 的可观察顺序和重数必须与当前合同完全相同：先按 source grapheme
   顺序，再按 caller 原始 range 顺序和每个 range 的 start/end 顺序；adjacent ranges
   共享同一内部 byte boundary 时保留两个 endpoint 事件，内部 empty range 的
   `start == end` 也保留两个事件，不得排序后去重或合并。
7. **B-007** 合法但未按 start 排列的 non-overlapping ranges 必须继续被接受；style、
   diagnostics、tokens、runs 与 source map 必须和相同 caller input 在优化前的语义一致，
   normalization 不得修改 caller vector 或把排序后的副本写入结果 identity。
8. **B-008** source 为空且 styled ranges 为空时仍产生当前确定的单个空 logical row；
   合法 empty range 不应用 style、不覆盖 default style，也不被当作 overlap。位于某个
   grapheme 内部的 empty range 仅按 B-006 贡献两个 ordered diagnostics。
9. **B-009** `start > end`、`end > source.len()`、任一 endpoint 不是 UTF-8 char
   boundary，以及包含 `usize::MAX` 的越界 endpoint，必须继续返回
   `TextFlowError::InvalidStyleRange { range }`，不得 panic、截断、饱和、忽略或回退到
   default style；多个 invalid ranges 时仍报告 caller 顺序中的第一个 invalid range。
10. **B-010** 两个 non-empty ranges 真正重叠时必须继续返回
    `TextFlowError::OverlappingStyleRanges { first, second }`，pair 选择保持现有按 start
    验证后的确定顺序；相邻 `end == start` 与 empty ranges 不属于 overlap。
11. **B-011** 任一 validation、normalization、arithmetic 或 interruption failure 都必须
    fail loudly 为现有 closed `TextFlowError`，不得返回空/部分 flow、删除 diagnostics、
    静默降级为 default style、重建 source 或使用旧 flow 冒充当前输入成功。
12. **B-012** 成功结果的 source bytes、`TextFlowSourceKind`、token source ranges、
    safe text、style、display width、placement、logical rows/runs、position map 与
    diagnostics 必须逐字段等于相同输入在当前合同下的结果；优化不得改变 hard break、
    tab、wrap、truncate 或 synthetic token 语义。
13. **B-013** `TextFlowCacheIdentity` 必须继续精确包含 caller 原始
    `TextFlowInput`（包括 styled range 顺序、empty ranges、range endpoints、每个完整
    `Style`）和全部 `TextFlowOptions`；语义等价但顺序不同的 range vectors 仍是不同
    identity，不能因内部排序而误命中 cache。
14. **B-014** 完全相同的 input/options 允许复用同一已发布 `Arc<TextFlow>`；任一 source、
    source kind、default style、range 顺序/endpoint/style 或 option 改变时必须 cache
    miss。cache hit 与 cold build 的完整 flow 必须逐字段相等。
15. **B-015** `try_build_interruptible` 的 immediate interruption 仍优先于 empty input、
    cache hit 和 range validation；initial poll 返回 false 后，invalid/overlapping input
    仍返回其 typed validation error，不能被后续 normalization polling 改写。
16. **B-016** range preprocessing 与 monotonic normalization 必须在有界工作间隔继续轮询
    interruption，使大量合法 empty/adjacent ranges 或大量 boundary events 不能形成新的
    无界不可取消区间。收到 cancellation 后返回 `TextFlowError::Interrupted`，不再处理
    后续 range/grapheme，也不产出部分结果。
17. **B-017** cache miss 的 normalization 被取消或失败时，`TextFlowCache` 必须保留上一
    completed `Arc<TextFlow>` 与原 build count；private build count 由 crate unit test 精确
    断言，crate 外 integration 只经 public API 断言 Arc/cache identity/完整 flow 语义，不得
    为测试新增 accessor。相同失败输入随后在不取消时必须得到与 direct cold build 完全相同
    的结果。重复取消不得累积部分 diagnostics、tokens 或 cache identity。
18. **B-018** 现有 public TextFlow API、closed error/diagnostic variants、GH-58
    first-source style、split combining/ZWJ tests、4096-case logical source round-trip
    property 与 engine cache identity tests 必须源码兼容且保持绿色；不得新增 public
    `Any`、无类型 side channel 或 caller-visible performance counter。
19. **B-019** 当前 main 中 #128 的四种 truncation mode/tab/linearity contracts、#129 的
    detached VNode flow cleanup 与 #130 的 unchanged context/Arc reuse contracts必须保持
    绿色；GH-127 不得修改这些 issue 的 owned source/test paths来“适配”优化。
20. **B-020** #126 已由 PR #136 合并为
    `50f6a203c1861814d288d4bdeae0e28d877af34c`，继续独占
    `src/layout/text_flow/wrap.rs` 与 `tests/text_flow_wrap_interruption.rs` 的行为合同。
    GH-127 implementation base/head 必须包含该 merge commit，并运行其 exact interruption
    target；不得复制、覆盖或按旧 callback count 固化 wrap-owned 断言。
21. **B-021** GH-127 新 public-behavior fixtures 必须覆盖 combining、ZWJ、adjacent、
    empty、合法未排序、invalid、overlapping、`usize::MAX`、exact cache identity、
    cancellation 与 retry；内部 counter gate 必须实际调用 production normalization，并
    覆盖 ordered projection 的非零 strict-interior 与 one-EGC skew，不得测试一份复制算法、
    预制计数或仅覆盖零 diagnostic 的 ASCII fast path。
22. **B-022** 完成证据必须绑定同一 implementation PR exact head：B-002/B-003 counter、
    B-004 至 B-021 exact tests、4096-case property、#126/#128/#129/#130 regression、
    fmt/check/clippy/all-target tests、至少 80% changed production line coverage、私有
    styled-normalization module executable branch/line 各 100%、固定 revision/checker
    SHA-256 的外部 SpecRail mirror、独立 review、零 unresolved non-outdated actionable
    threads 与 required CI 全部 fresh 通过后，才能宣称完成。新增 unit tests必须进入自然
    拆分文件，并让被触碰的 `src/layout/text_flow/tests.rs` 回到 800 行以内。
    closure 开始和结束都必须 fresh fetch remote `main`，并证明 caller `BASE_SHA`、
    fetched main、PR `baseRefOid` 与 `merge-base(baseRefOid, headRefOid)` 四者相同，
    PR `headRefOid` 等于 local exact `HEAD`；任一 ref 漂移使整份证据失效。实现 manifest
    只从该 trusted base 的 exact regular `tech.md` Git blob读取；NUL-delimited raw diff
    只接受其五路径非空子集、`A/M` status 和 `100644/100755` target mode，拒绝
    rename/copy/delete/type-change/unmerged、重复或 non-canonical path。
    coverage source 必须从 exact head tree 的 regular Git blobs/OID 安全物化到只读隔离树，
    不从可变 checkout pathname 读取 source evidence；descriptor-relative
    `O_NOFOLLOW|O_DIRECTORY` 读取父目录、`O_CREAT|O_EXCL|O_NOFOLLOW` 创建目标，所有
    coverage/target/provenance 输出均位于源码树外。raw LCOV SHA-256 与 exact blob manifest
    一起进入 provenance；`SF:` 必须精确映射物化 root 下的 tracked Rust blob，record 不重复、
    不靠 suffix 命中，每条 `DA/BRDA` line 必须在对应 blob 的 1-based `1..=EOF`，且
    `LF/LH` 与 `DA`、`BRF/BRH` 与 `BRDA` 一致。ledger selector 每项
    `matched=1/passed=1/ignored=0`；Cargo.toml raw-diff、ignored selector、line 0/超 EOF
    `DA/BRDA`、empty/deleted DA、summary mismatch、bad hash、symlink/existing-target 与
    shell early-failure negative fixtures必须全部 fail closed。所有 Python 从预先解析的
    absolute interpreter、清空启动注入环境并以 `-I -S` 执行；SpecRail 只显式导入已按
    exact tree/OID 验证的只读 mirror path。

## 验收标准

- [ ] ASCII、combining/ZWJ internal-boundary 与 one-EGC skew 的 2k/4k/8k deterministic
      counter 同时满足 absolute bound 与 doubling slope；debug/release 共用同一 integer
      contract，projection action 在内部 boundary fixtures 中非零且失败信息完整。
- [ ] 一 range/一 grapheme、合法未排序、adjacent、内部 empty、默认 style、combining 与
      ZWJ fixtures 的完整 tokens/runs/diagnostics/source map 与当前语义逐字段一致。
- [ ] invalid、overlapping、char-boundary、reverse 与 `usize::MAX` fixtures 返回精确
      `TextFlowError`，零 panic、零 fallback、零 partial cache publication。
- [ ] cache identity、Arc reuse/miss、interruption/retry 与 engine split-style/property
      regressions全部通过。
- [ ] implementation head 包含 #126 merge
      `50f6a203c1861814d288d4bdeae0e28d877af34c`；#126/#128/#129/#130 与 full
      workspace gates 未被弱化。
- [ ] current exact head coverage raw artifact/provenance、CI、immutable SpecRail mirror、
      独立 review 与 reviewThreads 证据完整；验证脚本在 Unix Bash 与 Windows
      Git Bash/MSYS2 runner 使用 `${TMPDIR:-/tmp}` + `mktemp`，不得绑定机器专用路径。

## 边界情况清单

| 类别 | 判定 |
| --- | --- |
| 1. 空/缺失输入 | covered: B-008、B-015；空 source/range vector 保持当前空 flow，empty range 单独定义 |
| 2. 错误与失败路径 | covered: B-009、B-010、B-011、B-017；每个 range/cancellation/cache failure 都是 typed 且原子 |
| 3. 授权/权限 | N/A：TextFlow 是纯本地数据变换，不读取权限、网络、凭据或执行工具 |
| 4. 并发/竞态/顺序 | covered: B-006、B-007、B-013、B-020；输入/diagnostic 顺序与已合并 #126 no-write ownership 均固定 |
| 5. 重试/重复/幂等 | covered: B-014、B-017；cache hit/cold build/retry 必须逐字段等价 |
| 6. 非法状态转换 | N/A：normalization 无业务状态机；未完成结果不得发布由 B-011、B-017 约束 |
| 7. 兼容/迁移 | covered: B-012、B-018、B-019、B-020；public API、GH-58 及已合入 follow-up contracts 保持 |
| 8. 降级/回退 | covered: B-011；任何 error/cancellation 都不得伪装成 default-style 或旧-flow success |
| 9. 证据与审计完整性 | covered: B-002、B-003、B-021、B-022；counter 不能由时钟或复制算法冒充 |
| 10. 取消/中断/部分完成 | covered: B-015、B-016、B-017；定义优先级、polling 与 cache 原子性 |

## 发布说明

这是内部性能与正确性兼容修复，不新增 public API，也不要求调用方迁移。
`TextFlowInput::styled_ranges` 的原始顺序、diagnostics 与 cache identity 保持不变。
发布说明必须注明高密度 styled spans 的 normalization 由 quadratic scan 收敛为
validation 后的线性 merge，并明确 wall-clock 不属于 correctness gate。
