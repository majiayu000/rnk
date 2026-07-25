# Product Spec：聊天布局 benchmark baseline 与回归门

## Linked Issue

GH-85: https://github.com/majiayu000/rnk/issues/85

complexity: large

## 用户问题

通用 layout microbenchmark 不能回答终端聊天负载中的 full、incremental 与 recovered
策略是否保持可解释的工作量、分配与耗时。若基准输入、运行环境、候选 artifact、可信
baseline 与 promotion 权限没有闭合合同，CI 可能把噪声、过期数据或当前分支自行生成的
baseline 误报为“无回归”。

本规格从 GH-61 拆出其原 B-024 至 B-028。GH-61 负责 `LayoutSnapshot`、跨 producer
parity 与每帧确定性 work counters；GH-85 只消费这些测量面，定义 chat workload
benchmark、artifact、required gate 与独立 baseline-promotion 生命周期。

## 目标

- 固定能代表聊天布局变化的 workload/strategy matrix 与最小 operation 数。
- 生成绑定 exact base/head、环境 fingerprint、work、allocation 与 timing 的版本化 artifact。
- 让 required gate 先执行确定性正确性检查，再以抗噪声 paired batches 判断 timing/allocation
  回归。
- 只信任 PR base tree 中可验证祖先来源的 canonical baseline，并对不可比较证据 fail closed。
- 为首次引入 benchmark 的 PR 提供非授权 bootstrap，并把 canonical baseline promotion
  保留为独立、人工授权的后续 PR。

## 非目标

- 不实现或修改 GH-61 的 snapshot、量化、producer parity、recovery 或 work-counter 语义。
- 不优化 layout 算法，也不承诺 incremental 在每个 workload 中都快于 full。
- 不用 benchmark 替代 parity、work-counter 或 allocation correctness checks。
- 不实现 MessageList、ChatComposer、chat shell、模型 provider 或终端字体像素一致性。
- 不允许 checker、feature PR 或 promotion PR 自行授权、合并或信任自身 baseline。

## Behavior Invariants

1. **B-001** benchmark scenario 集必须严格等于
   `unchanged_frame`、`streaming_delta`、`append_message`、`middle_insert`、
   `variable_height_transcript`、`resize_invalidation`。`unchanged_frame` 只允许
   full、incremental；其余 scenario 必须同时记录 full、incremental、recovered。缺失组合或
   额外名称均为无效 artifact。
2. **B-002** 每个允许的 scenario/strategy/batch row 必须达到固定最小 operation 数，
   `sample_count >= 10`，并完整记录 `median_ns`、allocation count/bytes、visited/mutated nodes、
   TextFlow recomputes、snapshot nodes 与 rebuild count。row 按
   scenario/strategy/batch 聚合；recovered row 必须满足
   `rebuild_count == operation_count`，非 recovered row 必须为 0。缺字段、负 counter、
   operation/sample 未达下限、不满足各 strategy 约束或出现 schema 未声明字段时 artifact
   无效；candidate、current-run compare artifact 与 canonical baseline 均适用 closed schema。
3. **B-003** benchmark 必须固定 seed、target size、viewport 序列、message corpus、
   toolchain、`Cargo.lock`、profile、runner fingerprint、warmup、sample/batch 数及 exact
   head/base SHA。每个 artifact 必须记录闭合 role、`source_sha`、内容/config/corpus hash 与
   `paired_order`；hash 不匹配时不得消费。setup/tree construction 不得计入 operation；
   full/incremental 必须从等价起点对同一 target 测量。
4. **B-004** PR required gate 必须先通过不依赖 wall clock 的 parity、work-counter 与
   allocation contract checks；任一前置检查失败、缺失或证据不完整时，不得继续给出
   performance-green 结论。
5. **B-005** 存在 trusted canonical baseline 时，timing 比较必须在同一 runner 上从 PR
   exact base checkout 与 current head 分别 build/run，并按每个 pair/batch 的 ABBA 顺序
   交错采集 current-run base/head artifacts；跨 run、错序或 pair identity 不一致的 row 无效。
   只有 `head/base > 1.20` 且
   `head-base > 50_000ns`，并在 3 个 batch 中至少 2 个复现时才判回归；单次 outlier
   不得阻断。base 或 head 的 `median_ns == 0` 时 timing denominator 无效，结果必须 blocked，
   不得以无穷比率或零差值继续比较。
6. **B-006** allocation count 相对本次 trusted compare 的 current-run base 同时超过
   10% 与 8 allocations，或
   allocated bytes 同时超过 10% 与 4096 bytes 时，required gate 必须判回归；相对或绝对
   阈值仅单独超过时不得误报。base allocation denominator 为 0 时，head 也为 0 表示该 metric
   无回归；head 大于 0 时相对条件视为满足，但仍须严格超过对应绝对阈值才判回归。
7. **B-007** compare 的 base 只能来自 PR exact base tree 中 repo-owned canonical
   baseline，且 source SHA 必须是 PR base 的祖先、不得等于 current head。baseline
   missing、closed-schema/content-hash 无效、source 不在 ancestry 中或由 current head
   self-authored 时必须 blocked；已验证来源但 schema/config/corpus/toolchain/runner
   fingerprint 与 current compare 不兼容时必须明确为 `needs_rebaseline`。stale 与 trust
   predicate 必须输出具体失败字段，二者均不得判 green。
8. **B-008** 首次引入 benchmark 的 implementation PR 只能运行 bootstrap，并生成绑定 exact
   head 的 non-authoritative candidate artifact。成功状态只能是
   `bootstrap_valid`、`comparison_status=not_available`、
   `promotion_required=true`；该 PR 不得新增/修改 canonical baseline、复用旧 candidate
   或宣称“无性能回归”。中断或部分生成的 candidate 不具备任何授权效力。
9. **B-009** canonical baseline 的唯一 writer 是 implementation 合入后的独立
   baseline-promotion PR。promotion 必须在 exact merged implementation SHA 的隔离 checkout
   重新测量，不得只改写 candidate SHA；只有独立 review、current CI、SpecRail gate 与明确
   merge authorization 全部满足后才能合入。checker、feature head 与 promotion head 均不得
   信任自身写入的 baseline；只有 baseline 成为未来 PR base-tree 内容后才可用于 compare。

## 验收标准

- [ ] 固定六 scenario、strategy matrix、minimum operations、closed schema、artifact role、
      source/hash/paired-order 字段均由 schema/checker 的正负 fixture 验证，覆盖
      B-001 至 B-003。
- [ ] required job 证明前置确定性门、exact-checkout same-runner ABBA、timing 2-of-3
      双阈值、allocation 双阈值与两个 zero-denominator 分支，覆盖 B-004 至 B-006。
- [ ] self/stale/untrusted/fingerprint-mismatch/missing baseline 均产生 non-green 结果，
      覆盖 B-007。
- [ ] 首次 implementation 只产生 exact-head candidate；独立 promotion 重新测量并保留所有
      人工 gate，覆盖 B-008、B-009。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-001、B-002、B-007 |
| 错误与失败路径 | covered: B-002、B-004、B-007、B-008 |
| 授权/权限 | covered: B-008、B-009 |
| 并发/竞态/顺序 | covered: B-005、B-009 |
| 重试/重复/幂等 | covered: B-003、B-005、B-008、B-009 |
| 非法状态转换 | covered: B-007、B-008、B-009 |
| 兼容/迁移 | covered: B-007、B-008；旧 baseline 不兼容时进入 `needs_rebaseline`，不默认放行 |
| 降级/回退 | covered: B-004、B-007、B-008 |
| 证据与审计完整性 | covered: B-002、B-003、B-007、B-008、B-009 |
| 取消/中断/部分完成 | covered: B-002、B-008、B-009 |

## 发布说明

GH-85 新增 CI/benchmark 合同，不改变用户运行时 API。首次 implementation 的
`bootstrap_valid` 仅表示 candidate 结构有效，不代表性能无回归；canonical baseline 必须由
后续独立 promotion PR 建立，之后的 PR 才能进入 trusted compare。
