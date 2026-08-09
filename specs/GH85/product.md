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
- 让base-owned workflow先完成route classification，再执行确定性正确性
  检查，并以抗噪声paired batches判断timing/allocation回归。
- 只信任 PR base tree 中可验证祖先来源的 canonical baseline，并对不可比较证据 fail closed。
- 为首次引入benchmark的PR提供non-authoritative candidate bootstrap，并把canonical baseline promotion
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
2. **B-002** 每个允许的 scenario/strategy/batch row 必须从同一起点执行恰好10个隔离sample；
   每个sample在warmup后重置target与instrumentation，再执行固定`operation_count`。timing以10个
   observation排序后第5/第6个值的checked整数平均数作为`median_ns`；allocation count/bytes与
   visited/mutated nodes、TextFlow recomputes、snapshot nodes、rebuild count在10个sample间必须
   逐字段完全相同，不能求和或平均掩盖差异。每个recovered sample必须满足
   `rebuild_count == operation_count`，非recovered sample必须为0。缺字段、负counter、reset
   失败、sample不一致、算术溢出、operation/sample未达合同或schema未知字段都使artifact无效。
   每个sample还必须保留exact `operation_count`项per-operation counters，并checked求和为sample
   totals；ABBA聚合必须保留原leg/sequence/role/source/binary identity，每个same-role leg恰贡献5项，
   不得跨leg复用、丢弃或重新编号。
3. **B-003** benchmark 必须固定 seed、target size、viewport 序列、message corpus、
   exact Rust toolchain、`Cargo.lock`、target triple、完整release profile、warmup与sample/batch
   数。runner必须把跨run比较使用的稳定
   compatibility class与仅用于本次ABBA诊断的volatile observation分开；canonical不绑定volatile
   CPU/hosted-image build identity，本次base/head observation则必须逐字段完全相同。稳定class固定
   hosted image label；toolchain/target/profile/class合同改变只能走授权的contract-update route并在
   合入后重新authority measurement/promotion。canonical只记录历史source/
   authority provenance，current-run artifact只记录当前exact base/head/run refs，二者不得要求
   相等或复用字段。每个artifact必须记录闭合role、内容/config/corpus hash、paired order与实际
   executable hash/source以及pinned sandbox image/policy digest；authority/current compare必须使用
   同一sandbox contract。setup/tree construction不得计入operation，full/incremental必须从
   等价起点测量。
4. **B-004** protected default-ref拥有的单一workflow只监听`pull_request_target`
   opened/synchronize/reopened/ready_for_review（另有与PR jobs权限隔离的`workflow_dispatch` authority），
   从payload规范化PR/base/head，并以per-PR `cancel-in-progress` concurrency及每job timeout保证只有
   newest exact head/run/attempt能发出当前status request。workflow job check本身不配置为required，
   repository/org/environment Actions secrets中不得存放reporter private key/token，Actions内也不得mint
   reporter installation token或拥有`statuses:write`/`checks:write`。外部托管的专用GitHub App
   `gh85-benchmark-status-reporter`只安装在base repo，权限最小为metadata/pull-request read与commit-status
   write；private key只由service server-side持有。trusted reporter-request jobs只申请`id-token:write`，
   以exact audience `gh85-benchmark-status-reporter`把closed pending/final bundle交给受保护的external
   endpoint。service验证OIDC issuer/
   audience/repository_id、reviewed protected-default workflow path/ref/SHA、event、run/attempt、PR/base/head/
   artifact binding，拒绝PR-head workflow/OIDC与repo token spoof，然后才以专用App写status。

   App每次fresh GET PR，解析current exact head与current test-merge SHA，验证test-merge object current/
   mergeable且parents精确为base+head；没有有效test merge即保持pending或写failure并blocked。context
   `gh85/layout-benchmark`必须同时写到head与GitHub UI/ruleset实际评估的test-merge SHA；final re-query要求
   pair未变，stale/superseded run绝不success。ruleset将该context绑定专用App integration_id并要求
   latest evaluated SHA success，任何repo workflow/GITHUB_TOKEN都不能伪造。same-repo/fork任一路径若
   不支持双status，实施blocked，无workflow-check、单SHA或其他source fallback。

   同一workflow/run使用互不共享host filesystem/process/env的fresh hosted VM：trusted `phase_zero`
   只读base policy并产生route/checker artifact；零GITHUB_TOKEN权限的`sandbox_collect`由base-owned
   host unauthenticated fetch exact public refs，并只在固定digest的ephemeral Docker containers内
   build/run PR-controlled Cargo code；`trusted_validate`只执行phase-zero pinned checker并把raw ZIP
   bytes经bounded archive preflight后解析，绝不执行PR binary；最终trusted reporter request只信validated
   artifact。hostile container无network、capability、Docker socket、host workspace/Actions runtime/
   token/env挂载，base/head及每leg输出隔离且退出即销毁；host在容器结束后hash固定输出，再由host-side
   pinned upload action上传始终untrusted的raw artifact，并由host-side controller输出exact artifact
   name/id/digest/run/attempt/PR/head binding；PR container不能写Actions command files或job outputs。
   PR代码不能观察或持久化到trusted jobs。
   missing/duplicate、archive/digest/identity mismatch、stale head、replay、cancel或timeout均blocked。
   route通过后才执行不依赖wall clock的parity、work-counter与
   allocation-correctness checks。prerequisite category必须严格按
   `{parity, work_counter, allocation_correctness}`各一次执行并绑定闭合`spec_ref`；命令只能在
   对应exact checkout root以base-owned workflow/checker共享的closed Cargo exact-test argv allowlist
   执行；执行产生的raw result仍不可信，必须由后续trusted checker重验。
   absolute/traversal路径、symlink escape、额外Cargo子命令/flag、cwd fallback、缺失/重复/未知/
   失败/错序证据都必须blocked，不得开始benchmark或给出performance-green结论。
5. **B-005** normal trusted compare必须在同一runner上从PR exact base checkout与current head
   分别build/run，并按每个pair/batch的ABBA顺序采集current-run artifacts；跨run、错序、pair
   identity不一致或base/head volatile observation不一致都无效。exact refs只能来自PR event，
   merge ref、`GITHUB_SHA`或未验证shallow object不得使用。每个row按B-002聚合；只有
   `head/base > 1.20`且`head-base > 50_000ns`并在3个batch至少2个复现时判timing回归；任一
   median为0或checked arithmetic失败都blocked。
6. **B-006** allocation count相对本次current-run base同时超过10%与8 allocations，或
   allocated bytes同时超过10%与4096 bytes时判回归；任一paired batch对任一metric同时超过
   两个阈值即失败，不适用timing的2-of-3规则。base为0且head为0表示无回归；base为0且head
   大于0时相对条件视为满足，但仍须严格超过绝对阈值。
7. **B-007** compare的baseline只能从PR exact base tree读取。canonical的历史source必须是
   current PR base的可验证祖先，但不需要也通常不应等于current head/base；current invocation
   refs只绑定本次run。baseline missing、closed-schema/content-hash无效、历史source不在future
   base ancestry、authority attestation无效或由current head自写时必须blocked；已验证来源但
   schema/checker/config/corpus/toolchain/stable compatibility class不兼容时必须明确为
   `needs_rebaseline`。volatile CPU observation变化只诊断，不使canonical stale。
8. **B-008** base-owned classifier必须按repository topology对每个regular file进行闭合分类，再从
   raw diff与base-tree canonical状态选择且只选择一个route：
   `initial_implementation_bootstrap -> bootstrap_valid`、
   `contract_update_bootstrap -> contract_update_valid`、
   `canonical_only_promotion -> promotion_valid`、
   `normal_trusted_compare -> comparison_valid`、
   `non_benchmark_change -> not_applicable_valid`。benchmark runtime class覆盖`src/**/*.rs`、
   `crates/*/src/**/*.rs`与exact chat bench；build/contract class覆盖root/crate Cargo manifests/lock及
   GH85 checker/workflow/schema/config/corpus/test paths；non-benchmark class闭合覆盖examples、普通
   tests/golden、其他benches、docs/specs/video、`.claude/**`、非GH85 `.github` paths、
   `crates/*/README.md`与包括`DESIGN_ISSUES.md`在内的exact standard root metadata。contract test必须
   对`git ls-files -z`的每个current tracked path证明精确命中一个class；未知或新增topology仍blocked。
   non-benchmark-only成功但`performance_status=not_available`；runtime加non-benchmark仍compare，
   initial/contract加non-benchmark仍走各自route。canonical promotion raw diff必须精确只有canonical
   path，连docs都不得混入。unknown top-level/path、symlink/submodule、mode/type、rename/copy、path
   collision或跨不兼容class混合一律blocked，直到authorized classifier contract update明确纳入。

   route classification、external authorization与performance必须分开。classifier不查询或解释
   reviews、markers、reviewer role，也不授予merge权限；initial/contract/promotion仅输出
   `authorization_status=external_required`，normal/non-benchmark输出`not_required`。required commit
   status可在受限route的route/artifact均有效且performance unavailable时success，但branch protection
   必须由T3实际配置并验证required approving review与new-commit stale approval dismissal；
   CONTRIBUTING所要求的maintainer final merge
   authorization必须绑定同一exact head。benchmark status永远不能授权merge。
   `route_status`、`authorization_status`与`performance_status`均为closed enum；只有normal route满足
   `route_status=comparison_valid`与`performance_status=passed`时最终decision为
   `comparison_passed`并表示“无回归”。`bootstrap_valid`、
   `contract_update_valid`、`promotion_valid`、`not_applicable_valid`在route/artifact均有效时可让
   required commit status success，但`performance_status=not_available`，不得称performance green；
   regression、needs_rebaseline、blocked均令check失败。contract update合入后必须重新authority
   measurement并走canonical-only promotion。bootstrap CLI必须显式接收
   repo/base/head/run/target/artifact参数，验证exact checkout、objects、ancestry、merge-base与
   diff后才生成non-authoritative candidate；candidate不可复用、不可promotion、不可声称性能通过。
9. **B-009** canonical authority只能由implementation或authorized contract update合入后、
   default-ref-owned受信workflow在exact merged source SHA隔离checkout重新测量。pipeline顺序必须
   为：`generate-authority-subject`只产生canonical subject与unsigned metadata；随后
   pinned attest action签subject；最后`finalize-authority`读取action输出的exact bundle path与
   attestation id，验证subject/workflow/run/source后产生final `authority.json`。authority job
   permissions精确为`contents:read`、`id-token:write`、`attestations:write`、
   `artifact-metadata:write`，其他全部none。subject、action bundle与final envelope必须由
   pinned upload-artifact action以闭合唯一name、禁止overwrite上传并记录artifact id/digest/run；整个
   authority workflow run的event必须是`workflow_dispatch`且conclusion必须为`success`，不能只看
   authority job或artifact存在；
   缺失、过期、wrong run/id/digest/bundle均blocked。attestation必须绑定repository/workflow/
   default-ref/run/source与subject digest并验证平台签名，不能由bundle自签。implementation
   candidate不参与。promotion PR只能提交与该authority
   evidence逐byte/digest相同的canonical blob。promotion CI必须read-only验证committed blob与
   attestation，任何阶段都不得先生成、创建或覆盖repo canonical path再验证。只有current
   exact-head repository CI、独立review、resolved threads与maintainer对同一promotion head的
   明确merge authorization全部满足后才能合入。implementation与promotion是两个独立PR，各自
   必须在执行前取得绑定各自exact head的maintainer明确授权，不能跨PR复用；未来PR只能信任已
   出现在base tree中的blob。

   trust-root workflow内所有third-party actions必须固定为reviewed full SHA：checkout v4
   `11d5960a326750d5838078e36cf38b85af677262`、upload-artifact v4
   `ea165f8d65b6e75b540449e92b4886f43607fa02`、download-artifact v4
   `d3f86a106a0bac45b974a628896c90dbdf5c8093`、attest v4
   `1e69f48acb82d1966a394da916b4c1698aa569d6`；任一升级只能走authorized contract-update route。

## 验收标准

- [ ] 固定六 scenario、strategy matrix、minimum operations、closed schema、artifact role、
      source/hash/paired-order 字段均由 schema/checker 的正负 fixture 验证，覆盖
      B-001 至 B-003。
- [ ] base-owned workflow证明guarded PR-target/dispatch隔离、OIDC requester identity、dedicated App service、
      current head+test-merge pending/final status、real Statuses API/combined schema、same-repo/fork、
      sandbox/validator/requester权限与文件系统隔离、newest-pair concurrency、timeout、前置确定性门、
      container/archive/raw-controller containment、exact ancestry/merge-base、
      same-runner ABBA、10-sample aggregation、timing 2-of-3与allocation any-batch双阈值，覆盖
      B-004 至 B-006。
- [ ] self/stale/untrusted/stable-class-mismatch/missing baseline 与同run observation mismatch均
      产生 non-green 结果，volatile canonical observation变化只诊断，
      覆盖 B-007。
- [ ] 五route互斥；route/auth/performance三种status互不冒充，只有comparison passed表示无回归；
      full-tree topology path classes、同一base-owned run隔离handoff与required status identity闭合；三阶段
      default-ref authority重新测量，所有actions full-SHA pinned，
      promotion CI只读验证immutable handoff，覆盖B-008、B-009。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-001、B-002、B-007 |
| 错误与失败路径 | covered: B-002、B-004、B-007、B-008 |
| 授权/权限 | covered: B-004、B-008、B-009 |
| 并发/竞态/顺序 | covered: B-004、B-005、B-009 |
| 重试/重复/幂等 | covered: B-003、B-005、B-008、B-009 |
| 非法状态转换 | covered: B-007、B-008、B-009 |
| 兼容/迁移 | covered: B-007、B-008；旧 baseline 不兼容时进入 `needs_rebaseline`，不默认放行 |
| 降级/回退 | covered: B-004、B-007、B-008 |
| 证据与审计完整性 | covered: B-002、B-003、B-007、B-008、B-009 |
| 取消/中断/部分完成 | covered: B-002、B-008、B-009 |

## 发布说明

GH-85 新增独立base-owned required benchmark合同，不改变既有`ci.yml`或用户运行时API。首次implementation的
`bootstrap_valid` 仅表示 candidate 结构有效，不代表性能无回归；default-ref authority必须
独立重新测量，promotion PR只能提交并只读验证匹配的canonical bytes。只有baseline合入future
base tree后，普通PR才能进入trusted compare并以`comparison_passed`表示无回归。
