# Product Spec：聊天 Examples 收敛与产品级 Hardening 证据

## Linked Issue

GH-68: https://github.com/majiayu000/rnk/issues/68

complexity: large

## 背景

当前 `chat.rs`、`rnk_chat.rs`、`claude_input_box.rs` 与 `glm_chat` 各自承担了聊天体验验证，
但也各自实现了输入、消息、滚动、视觉光标、终端输出或恢复逻辑。仅让这些 examples
“可以运行”不能证明公共 Chat UI 组件可复用：用户仍可能复制 example 私有状态机，
Inline/Fullscreen 的兼容性、错误语义和性能边界也没有形成可审计证据。

GH-68 是 GH-57 的最终 hardening child。它不重新设计 GH-61 的布局快照、GH-66 的
Inline 生命周期或 GH-67 的 Fullscreen 生命周期，而是在这些依赖完成后，把 examples、
兼容入口、文档、测试、benchmark、兼容矩阵和 CI 收敛为同一套产品证据。

PR #69（head `2c4720152d43f9507fe1fb43e331a866c683c585`）提供了 GH-57
架构与验证草案，可作为本规格的搜索证据；该 PR 当前为 parked/draft，不能被描述为已批准
或已合并的产品合同。

## 目标

- 让四个聊天 examples 分别展示公共组件的不同组合，而不是四套私有聊天实现。
- 保留既有简单 `Message` 使用方式，并清楚标注新增 Chat API 的成熟度和迁移路径。
- 以 deterministic tests、plain/ANSI golden、PTY 恢复证据、stress/benchmark、兼容矩阵
  与 CI gate 证明 Inline/Fullscreen 可用于真实终端应用。
- 让所有完成声明绑定依赖 merge、当前提交和可复现证据，禁止以视觉演示或旧结果替代。

## 非目标

- 不删除仍有独立教学价值且能够说明公共 API 组合方式的 example。
- 不在缺少兼容包装和迁移说明时破坏现有 `Message` 或 pre-1.0 推荐入口。
- 不承诺未经测试的 OS、终端、tmux/SSH 或输入能力。
- 不把模型供应商 SDK、供应商 JSON、网络请求、密钥、工具执行或权限决策放入核心 Chat UI
  合同；应用级 `glm_chat` 可保留 provider adapter，但核心 example 依赖必须保持后端无关。
- 不复制或弱化 GH-61、GH-66、GH-67 已批准的正确性、幂等性和恢复合同。
- 不把 benchmark 结果当作正确性测试，也不以单次计时噪声决定回归。

## Behavior Invariants

1. **B-001** `chat.rs`、`rnk_chat.rs`、`claude_input_box.rs`、`glm_chat.rs` 及其
   `glm_chat/` 辅助代码必须逐个完成迁移审计；每个保留的顶层 example 都必须在统一索引中记录
   唯一、可观察的教学目的、运行模式和目标读者。缺少其中任一项、把多个 examples 写成同一
   功能的换皮版本，或未审查辅助模块时，收敛不得判定完成。
2. **B-002** 每个保留 example 必须保存迁移前需要继续支持的用户可见能力，并用迁移后
   snapshot/interaction evidence 证明等价或记录有期限的行为变化。只有在无独立教学目的、
   索引与文档不再引用、且替代路径已验证时才可删除；“公共组件已经存在”本身不是删除依据。
3. **B-003** 所有公开聊天 examples 必须被标记为闭集
   `{tutorial, showcase, debug, internal}` 中的一类并出现在唯一索引中；缺失、重复、越界分类，
   或索引引用不存在文件时，CI 必须失败。`internal` 项不得被 README 推荐为公共入门入口。
4. **B-004** 迁移后的 examples 只能组合公共 Conversation、message/block view、
   `ChatComposer`、`MessageList`、`InlineChatShell` 或 `FullscreenChatShell` 合同；
   不得自行实现 Unicode wrapping、视觉光标、streaming delta 拼接、消息行高索引、
   bottom-follow、scroll anchor 或 scrollback commit ledger。示例特有的主题、fixture、
   provider adapter 和应用业务动作可以保留，但不能复制核心状态机。
5. **B-005** 至少两个相互独立的应用 adapter 必须把等价外部事件转换为同一组公开
   conversation updates，并产生相同的最终 conversation state 与语义 view snapshot；
   核心 crate 与后端无关 examples 不得依赖供应商 SDK、传输协议、供应商 JSON 或密钥。
   adapter 输入为空、缺字段或失败时必须产生公开 typed failure/empty outcome，不能伪造成功
   消息或供应商元数据。
6. **B-006** 现有 `Message::{system,user,assistant}` 等简单使用方式必须继续编译并保持
   role/text 的用户可见语义；若由兼容包装投影到 richer chat model，缺失 revision、非法 role
   或无法表达的状态必须按已批准的兼容合同处理，不得静默猜测或改变已有消息内容。
7. **B-007** 每个新增 Chat API、compatibility wrapper、extension hook 和测试 helper
   必须在公共文档中标记 `stable`、`advanced` 或 `experimental`，并说明推荐 import、
   pre-1.0 变更承诺、迁移步骤和弃用窗口。未获得明确稳定性证据的 API 不得因出现在 example
   中被宣称为 stable。
8. **B-008** 使用文档必须分别提供最小可运行的 Inline 与 Fullscreen quickstart，并明确
   Inline native scrollback/live region 与 Fullscreen alternate-screen/owned transcript 的
   生命周期差异；任一 quickstart 不得要求用户复制 example 私有输入、滚动或提交代码。
9. **B-009** 使用文档必须提供 conversation update、custom block renderer、keymap、
   error handling 与非目标说明；每段示例必须使用公开入口，并明确应用负责 provider 请求、
   tool authorization/execution、retry policy 与持久化，核心 Chat UI 不负责这些副作用。
10. **B-010** 空会话、空文本、空 block 列表、缺失可选 author/model/token metadata 与
    provider adapter 无数据必须呈现明确空状态或空白字段；不得虚构模型名、token 数、连接状态、
    消息内容或成功结果。空列表与缺失可选列表按公开上游合同处理，不得由 example 自设别名语义。
11. **B-011** focused interaction evidence 必须覆盖 CJK、emoji、combining sequence、
    ZWJ grapheme、CRLF/LF、单次多字符输入与 bracketed paste；光标、删除、换行、选择、
    wrapping 和提交不得拆分 grapheme，也不得因宽字符导致视觉光标或内容越界。
12. **B-012** focused interaction evidence 必须覆盖 focus 进入/离开、keymap 冲突、
    supported minimum size、窄宽 resize、连续 resize 与 resize 期间输入；事件只能被目标
    component 消费一次，focus 或尺寸变化不得丢失 draft、改变已提交 transcript 或制造错误
    bottom jump。
13. **B-013** 同一 deterministic conversation fixture 必须同时生成 plain 与 ANSI golden；
    去除 ANSI 后的语义文本、状态、消息顺序和可见错误必须等价。颜色只能增强表现，不能成为
    empty/loading/streaming/failed/cancelled/focus 等状态的唯一信息来源。
14. **B-014** Inline 与 Fullscreen 都必须有非 ignored 的 terminal restoration evidence，
    覆盖正常退出、取消、typed failure 和 panic/unwind；退出后 raw mode、cursor visibility、
    alternate screen 与输入模式必须恢复。Fullscreen 必须由自身 focused evidence 覆盖全部四种
    退出路径和四项 restoration，不能用 Inline 或 provider example 代替。恢复失败必须作为失败
    暴露，不能只记录 warning 后成功。
15. **B-015** stress/benchmark 必须覆盖长会话、高频 streaming delta、可变高度消息、
    prepend history、active stream 增长、block 展开/折叠和连续 resize；每个 workload 必须同时
    验证最终状态/anchor 正确性，不能只记录耗时。
16. **B-016** 每份 benchmark baseline 必须以 durable JSON artifact 绑定 exact
    implementation head、fresh base-main SHA、Rust/toolchain、OS/arch、terminal-independent
    fixture 参数、消息/字符数量、宽高序列、warm-up/sample 配置、实际样本与结果单位；缺字段、
    fixture 不同、非当前 head 或仅嵌入预制 fixture 的结果不可比较，也不得用于产品级完成声明。
17. **B-017** 性能回归判定必须从当前 head 实际生成的 benchmark artifact 读取样本，使用批准的
    相对阈值与绝对噪声下限，并以多样本统计量比较同环境 baseline；单次慢样本、不同机器结果、
    只编译未执行的 benchmark 或只校验内嵌正/负 fixture 不得判定通过或失败。benchmark smoke
    只证明 workload 可运行，不等于满足性能门槛。
18. **B-018** 兼容矩阵必须分别记录支持的 OS、terminal emulator、Inline、Fullscreen、
    paste、resize、raw-mode restoration、tmux 与 SSH 状态，并使用闭集
    `{verified, best_effort, terminal_dependent, unsupported, unverified}`。每个 `verified`
    单元必须链接或标识当前证据；无证据、证据过期或只由通用 CI 推断时必须标为 `unverified`。
19. **B-019** CI 必须发现并构建全部公开 examples，而不是维护可遗漏的手写子集；任何公开
    example 编译失败、索引漂移或引入核心不允许的 provider 依赖时，required gate 必须失败。
    hosted CI 必须验证 implementation PR exact head 而非 merge-ref，并按本地相同顺序先生成
    runner-local benchmark/coverage artifacts 再运行 full suite；baseline coordinate/digest
    缺失或不匹配时 fail closed。
20. **B-020** CI 必须逐名运行 task-owned focused example、全局 convergence、docs、
    benchmark/coverage evidence 与 CI contract tests，并证明每个 exact test
    `matched=1`、`passed=1`、`ignored=0`；没有匹配、被 ignore、只运行宽泛 workspace tests
    或复用其他 task 的绿色结果均不能替代。benchmark/coverage evidence-dependent exact test
    缺少批准 mode、artifact path、head/base binding 或 digest 时必须 fail closed。
21. **B-021** pre-edit authorization 使用 `phase=initial`，允许 clean
    `IMPLEMENTATION_HEAD == BASE_MAIN_SHA` 或 base 为其祖先；任何 current implementation
    evidence/final verification 必须使用 `phase=finalorigin/main`。
22. **B-022** 高频 delta、append、prepend、可变高度和连续 resize 交错时，conversation update
    顺序、message identity、visible anchor 与 bottom-follow 必须保持上游合同：用户位于底部时
    自动跟随，主动滚离后保持锚点并显示新内容提示。example 不得通过重排、丢弃或全量重建状态
    掩盖错误。
23. **B-023** 取消、provider failure、`NotCommitted`、`Unknown`、重复终态、重试和部分完成
    必须保持 typed outcome；未稳定内容不得被标记为成功或产生第二次 confirmed scrollback
    commit，`Unknownfile + name` 集合必须与 task plan
    完全相等且逐项 100%。producer/validator 必须在当前 head 实际执行；CI 中
    `continue-on-error` coverage、视觉录屏、旧 artifact、内嵌样本或其他 task coverage
    不构成证明。
26. **B-026** GH-68 实现必须通过可执行、fresh、fail-closed 的 preflight evidence adapter：
    #61/#66/#67 issue 必须 CLOSED、无 parked，且各自唯一 closing final implementation PR
    merged、非 draft/parked、所有分页完整并含明确 executable Rust source diff；三个 merge SHA
    两两不同且都是 implementation head 的祖先；`phase=final` 时必须为严格祖先。GH-68 spec PR 必须以 `main` 为 base、
    body 仅用非 closing `Refs #68` linkage、changed files exact 等于本 packet 三文件、merged、
    非 draft/parked，并有绑定 exact head/scope 的 human `APPROVED` review；#68 必须 fresh 带
    canonical `ready_to_implement` 且无 parked/冲突 readiness labels。adapter/validator 必须
    全量重验 decisive fields、sets 与 digests；环境变量声明、spec-only dependency、绿色 CI
    或被 pipeline status 覆盖的失败都不满足门禁。
27. **B-027** 终端能力不可用时只能进入文档化的显式降级路径并向用户说明能力差异；数据丢失、
    消息顺序错误、布局/anchor 错误、重复提交、终端恢复失败或无法判定的副作用不得降级为
    看似成功的输出。
28. **B-028** 四个 examples、文档、golden、benchmark、兼容矩阵与 CI gate 是一个原子完成
    集合；任一 example 未审查、任一 mapped test/benchmark/矩阵证据缺失、依赖未满足或验证
    中断时，GH-68 必须保持未完成。重跑只能生成新鲜证据，不能把部分旧证据拼接成一次完整通过。

## 验收标准追踪

| Issue AC | 覆盖不变量 |
| --- | --- |
| 1. 四个聊天 examples 逐个审查、迁移并记录独立目的 | B-001、B-002、B-004、B-028 |
| 2. 分类与统一索引 | B-001、B-003、B-019 |
| 3. 不再私有实现公共聊天行为 | B-004、B-022、B-023 |
| 4. `Message` 兼容与 API 成熟度/迁移策略 | B-006、B-007 |
| 5. Inline/Fullscreen 与扩展/错误文档 | B-008、B-009、B-010 |
| 6. Unicode、paste、focus、resize、golden、恢复 | B-011、B-012、B-013、B-014、B-020 |
| 7. 长会话、streaming、可变高度、prepend、resize benchmark | B-015、B-016、B-017、B-022 |
| 8. OS/terminal/tmux/SSH 兼容矩阵 | B-018、B-027 |
| 9. 全部公开 examples 与 chat test/golden CI gate | B-019、B-020 |
| 10. fresh check/tests/examples/benchmark smoke | B-021、B-025、B-026、B-028 |

## Boundary Checklist

| 边界类别 | 结论 | 覆盖 |
| --- | --- | --- |
| 1. Empty / missing input | covered；空会话、空字段、空 adapter 与缺失凭证都不得虚构数据 | B-005、B-010、B-024 |
| 2. Error and failure paths | covered；adapter、commit、恢复、benchmark 与验证失败均 fail closed | B-005、B-014、B-017、B-023、B-027、B-028 |
| 3. Authorization / permission | covered；Tool Call 仅展示，provider/tool 权限留在应用边界，实现还需人工授权 | B-009、B-024、B-026 |
| 4. Concurrency / race / ordering | covered；delta/prepend/resize/focus 交错仍保持顺序、identity 与 anchor | B-012、B-022、B-023 |
| 5. Retry / repetition / idempotency | covered；重复终态、`Unknown`、重试与旧 evidence 不得重复副作用或冒充新证据 | B-020、B-023、B-025、B-028 |
| 6. Illegal state transitions | covered；examples 必须消费上游 typed outcomes，不能改写取消/失败/未知为成功 | B-004、B-022、B-023、B-027 |
| 7. Compatibility / migration | covered；逐 example parity、`Message` wrapper、API 成熟度和弃用窗口均明确 | B-001、B-002、B-006、B-007 |
| 8. Degradation / fallback | covered；只有终端可选能力可显式降级，关键正确性与恢复失败不得伪装成功 | B-018、B-027 |
| 9. Evidence and audit integrity | covered；baseline、exact tests、当前 SHA、coverage、依赖和原子完成集合均可审计 | B-016、B-017、B-019、B-020、B-021、B-025、B-026、B-028 |
| 10. Cancellation / interruption / partial completion | covered；取消/中断保持 typed 状态，部分 packet 不得宣称完成 | B-014、B-023、B-028 |

## Human Gates

- 本 packet 只处于 `write_spec`；人工 spec approval 与 `ready_to_implement` 缺一不可。
- PR #69 与任何 parked/draft spec 只能作为草案证据，不能替代人工批准或 dependency merge。
- 最终 implementation PR approval、merge、release 与 GH-57 closure 仍由人类决定。
