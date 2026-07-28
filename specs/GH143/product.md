# Product Spec：让定向流式更新与 transcript 大小无关

## Linked Issue

GH-143

## 用户问题

PR #117 已让普通 `AppendText` / `Complete` 跳过全量 mutation backup 与全局
correlation validation，但 reducer 仍会为每个流式 chunk 多次遍历完整 `messages`。
当一个长会话包含大量无关消息时，更新单个正在流式输出的消息仍会随 transcript
增长而变慢，累计成本为 `O(M × C)`（`M` 为消息数，`C` 为 chunk 数）。

## 目标

- 定向解析消息一次，并在 guard 校验、mutation、revision 推进和 outcome 构造中复用。
- 普通单目标 `AppendText` 与 `Complete` 不访问无关消息。
- 保持现有公开 API、更新顺序、typed error、回放、snapshot 与失败原子性。
- 用确定性计数器证明成本边界，而不是用易受机器噪声影响的 wall-clock benchmark。

## 非目标

- 不重设计公开 chat model、snapshot 格式或 provider adapter。
- 不把 `Cancel` / `Fail` 等真正跨消息的 correlation 语义强行改成单目标。
- 不改变消息、block、revision、ledger 或 affected-message 的可观察顺序。
- 不以放宽校验、吞掉错误或静默 fallback 换取性能。

## Behavior Invariants

1. B-001 当合法 `AppendText` 指向一个已存在消息时，每次 event 只解析目标消息
   一次；添加 10,000 个无关消息不得增加无关消息访问次数，目标 lookup 次数保持
   一次。
2. B-002 当 `Complete` 的目标不需要跨消息 correlation 判定时，event 只访问目标
   消息和它自己的 blocks；添加 10,000 个无关消息不得增加无关消息访问次数。
3. B-003 当 `AppendText` 指向 `Text`、`Markdown`、`Code` 或 `Thinking` block 时，
   既有内容、language / thinking identity 与 lifecycle 语义必须保持，且只追加给定
   delta。
4. B-004 当目标分别位于 transcript 首部和尾部时，B-001 与 B-002 的计数上界相同；
   目标位置不得重新引入线性扫描。
5. B-005 当单目标更新成功时，只推进目标 message revision 与 conversation
   revision，并直接构造一个按现有顺序排列的 `AffectedMessage`；无关消息的值、
   revision 与 storage 不得变化。
6. B-006 当单目标更新因 unknown message / block、stale guard、terminal lifecycle、
   illegal transition 或 revision exhaustion 被拒绝时，messages、identity state、
   revisions、sequence、ledger 与 outcome history 必须逐项保持不变。
7. B-007 当 `Push`、`DeleteMessage` 或 `Resend` 成功时，后续按 `MessageId` 查找必须
   返回与稳定 transcript 顺序一致的消息；当这些更新的后置全局校验失败并 rollback
   时，查找结果与更新前完全一致。
8. B-008 当从合法 snapshot 恢复会话时，所有 active `MessageId` 都必须立即解析到
   snapshot 中的正确位置；缺失、重复或矛盾 identity 继续 fail closed，不得以扫描
   fallback 掩盖索引不一致。
9. B-009 当 `Complete` 的目标含跨消息 tool correlation，或操作为 `Cancel` /
   `Fail` 时，reducer 必须保留必要的 correlation 检查与确定性 affected ordering；
   只有语义确实跨消息的路径才允许访问相关或完整 transcript。
10. B-010 当重复提交已接受的同一 event 时，返回既有 outcome 且不再次 mutation；
    相同 event id 携带不同 payload 时继续返回 typed conflict，优化不得改变 replay
    与 proof-integrity 合同。
11. B-011 成本证据必须分别记录 message visits、target lookups、block visits、
    global validation calls 与 backup captures；计数器只用于 deterministic tests，
    不新增公开 API，也不得把一次全量工作伪装成一个计数。
12. B-012 现有 exhaustive update classification、snapshot/restart、proof integrity、
    correlation、failure atomicity 与 public constructor tests 必须原样继续通过。
13. B-013 普通 `AppendText` 与非 correlation `Complete` 继续跳过 full mutation
    backup 和 global conversation validation；任何 degraded fallback 必须可见且仅限
    B-009 的跨消息语义。

## 验收标准

- [ ] 同一 `AppendText` workload 在 1 条和至少 10,001 条消息下均为一次 target
  lookup、零 unrelated-message visits；目标在首部与尾部都满足。
- [ ] 非 correlation `Complete` 满足相同 transcript-size independence。
- [ ] Text、Markdown、Code、Thinking 的成功与拒绝路径均有确定性测试。
- [ ] Push、Delete、Resend、rollback 与 snapshot restore 后的 message identity
  解析和 transcript 顺序均有回归测试。
- [ ] accepted / rejected deltas 保持 revision、ledger、affected ordering 与失败原子性。
- [ ] 新代码行覆盖率至少 80%，单目标 fast path 与私有索引关键分支 100%。
- [ ] fmt、workspace check、严格 Clippy、完整 workspace tests、doc tests 与 exact-head
  CI 全部通过。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-003 B-006（空 delta 仍由现有 constructor 拒绝；unknown target/block fail closed） |
| 错误与失败路径 | covered: B-006 B-008（所有拒绝路径无 partial mutation；snapshot/index 矛盾不可降级） |
| 授权/权限 | N/A：纯本地数据 reducer，无用户权限或外部授权面 |
| 并发/竞态 | N/A：`apply_event(&mut self)` 串行独占状态；测试计数器为线程局部，不新增共享并发状态 |
| 重试/幂等 | covered: B-010（exact replay 幂等，event-id payload 冲突保持 typed error） |
| 非法状态转换 | covered: B-003 B-006 B-009（terminal/相关生命周期仍执行原状态机） |
| 兼容/迁移 | covered: B-010 B-012（公开 API、snapshot 与历史 proof 合同不变，无持久化迁移） |
| 降级/回退 | covered: B-008 B-009 B-013（禁止索引扫描 fallback；跨消息路径显式保留） |
| 证据与审计完整性 | covered: B-011 B-012（分项计数且完整回归，不以聚合计数隐藏全量扫描） |
| 取消/中断 | covered: B-006 B-009（拒绝无 partial state；Cancel/Fail 保留跨消息原子语义） |

## 发布说明

这是内部性能与实现结构修复，不改变公开构造器、返回类型或 snapshot wire-neutral
parts。发布说明应指出大 transcript 下定向流式更新不再随无关消息数增长，并明确
`Cancel` / `Fail` 与 correlation-bearing `Complete` 仍按既有跨消息语义校验。
