# Product Spec：grapheme-safe 多行 ChatComposer

## Linked Issue

GH-64: https://github.com/majiayu000/rnk/issues/64

## 用户问题

当前 Claude/GLM 示例分别维护输入字符串、光标、换行、裁剪和 ANSI 定位。通用
`TextAreaState` 虽已提供多行、selection 与 viewport 状态，但编辑坐标仍按 Unicode
scalar `char` 计数，handler 只接受 `input.len() == 1` 的单字节输入。中文、emoji、
combining sequence、多字符 committed input 与 bracketed paste 因此可能被忽略、拆分或
部分写入；wrapped visual rows、组件高度和光标可见性也没有共同合同。

用户需要一个后端无关、受控、可测试的 `ChatComposer`：它复用现有交互合同，以 grapheme
为编辑单位，以终端 cell 为显示单位，明确区分 submit/newline，失败时不丢草稿，并在 resize
后保持正确的 auto-grow 与光标位置。

## 目标

- 在现有 `TextAreaState` 上建立 grapheme-safe 的编辑、selection 与 cursor 基础。
- 提供 `ChatComposerState`、可验证的 `ChatComposerKeyMap`、纯 key/paste handlers 与
  `ChatComposer` 视图。
- 让多字符输入、CRLF 和 bracketed paste 经过同一原子、类型化 ingress。
- 让 GH-58 的唯一 TextFlow rows/source map 驱动 visual cursor、selection、auto-grow 与
  resize，不复制换行或 Unicode width 算法。
- 提供 submit acknowledgement 合同：提交后保留草稿，调用方确认成功才清空。
- 手工迁移一个现有 Claude 输入示例，证明应用不再维护独立 cursor/wrapping state。

## 非目标

- 不发送模型请求，不管理 transcript、conversation history 或 provider adapter。
- 不执行工具，不读取密钥，不提供系统剪贴板实现。
- 不实现 undo/redo、输入历史浏览或 shell 级焦点仲裁。
- 不声称支持 crossterm 未提供的 native IME preedit/candidate UI；只支持终端已提交的文本。
- 不自动拥有 bracketed-paste terminal mode 的启停与退出恢复；该生命周期由应用或后续
  Inline/Fullscreen shell 管理。
- 不在本 issue 迁移全部聊天示例，也不直接打印或拼接 ANSI。

## Behavior Invariants

1. **B-001** 当应用创建聊天输入区时，系统必须提供公开的 `ChatComposerState`、
   `ChatComposerKeyMap`、`ChatComposer` 与纯 handler；handler 必须直接返回现有
   `InteractionOutcome<T>` 的语义，不得新增 `InteractionMode` /
   `InteractionOutcome<T>` alias，也不得要求 provider、网络或 transcript 对象。
2. **B-002** 当 cursor 移动、Backspace、Delete、word edit、selection 或 selection
   replacement 作用于文本时，最小可编辑单位必须是 extended grapheme cluster。`e` 加
   combining mark、家庭 ZWJ emoji、旗帜 emoji、variation selector、零宽 grapheme 与 CJK
   不得被拆成无效半段、重复或跳过。
3. **B-003** 当 grapheme 被定位或显示时，source 位置必须以 UTF-8 byte range 和 grapheme
   ordinal 表达，visual 位置必须使用 GH-58 的同一 Unicode width policy 与 terminal cell
   mapping。cursor 与 selection 不得落在 grapheme 中间字节或宽字符 continuation cell。
4. **B-004** 当 key handler 收到任意非空、无 Ctrl/Alt shortcut 语义的 committed UTF-8
   input 时，必须一次接受完整字符串，而不是要求 `input.len() == 1`。LF、CRLF 与独立 CR
   统一为内部 LF，CRLF 只产生一个 hard break；tab 按已配置 tab policy 处理。整批输入必须
   先验证再提交，任一失败时 selection、content、cursor、revision 与 viewport 全部不变。
5. **B-005** 当 paste handler 收到 `PasteEvent` 时，必须把其完整内容送入与 B-004 相同的
   原子 ingress；多字符、多行、CRLF、CJK、emoji 与 combining sequence 的顺序和归一化结果
   必须可验证。空 paste 返回 `Ignored`，同一事件不得同时作为 key input 再分发。
6. **B-006** 当 committed input 或 paste 包含 ESC、除结构化 LF/CR/tab 外的 C0、DEL 或
   C1 control 时，系统必须返回包含原 source range 的 typed rejection，且零 mutation；
   不得把 payload 当作 ANSI、快捷键、命令或授权，也不得静默删除或部分插入。
7. **B-007** 当 keymap 使用默认配置时，未修饰 Enter 必须提交，Shift+Enter 必须插入
   newline，并必须提供一个文档化、可配置的 terminal fallback（默认 Alt+Enter）。自定义
   binding 的 action 解析顺序必须确定；同一 binding 同时匹配 submit、newline 或其他冲突
   action 时，keymap 构造必须 typed 失败，不能依赖字段遍历顺序。
8. **B-008** 当 submit action 发生在空字符串或仅包含 Unicode whitespace 的草稿上时，
   handler 必须返回 `Handled` 而不是 `Submitted`，草稿和 revision 不变。当草稿有效时，
   `Submitted` payload 必须包含完整规范化文本与可比较的 submission token，不得裁剪、
   trim、隐式换行或清空。
9. **B-009** 当有效提交产生 token 后，composer 必须进入 submitting 状态并保留原草稿；
   相同 token 的显式 success acknowledgement 才能清空 content、cursor 与 selection。
   failure/rejection acknowledgement 必须保留草稿并退出 submitting；缺失、过期或不匹配
   token 必须 typed 失败且不清空较新的草稿。首次 success 必须把 token 放入容量固定为 16、
   FIFO 淘汰且只保存 token 的 bounded success tombstone ring；ring 内相同 token 的重复
   success 必须返回成功 no-op，即使已有更新草稿或另一个 pending submission 也不得清除它们。
   从未提交或已淘汰 token 返回 typed stale/unknown error 且零 mutation。
10. **B-010** 当 `InteractionMode::Enabled` 且不在 submitting 时，允许 navigation、
    selection、编辑、paste、submit 与 cancel；`ReadOnly` 允许 navigation、selection、
    selected-text 读取与 cancel，但禁止编辑、paste 与 submit；`Disabled` 忽略全部输入；
    submitting 允许 navigation、selection 与 cancel，但阻止内容修改、paste 和重复 submit。
    submitting 不得通过给 `InteractionMode` 增 variant 或 alias 表达。
11. **B-011** 当 Escape 在 Enabled、ReadOnly 或 submitting composer 中触发时，handler
    必须返回 `Cancelled` 且不清空、不回退 revision；Disabled 中 Escape 返回 `Ignored`。
    cancel 只表达交互结果，是否取消模型请求由 shell 决定。
12. **B-012** 当用户用 Shift+movement 扩展或反向收缩 selection 时，系统必须保留稳定的
    private anchor/focus 方向，同时公开 normalized `Selection { start, end }`。复制出的
    selected text、删除、Backspace、Delete 和 committed input replacement 必须精确覆盖完整
    grapheme 范围；跨 logical line selection 保留单个 LF 分隔。
13. **B-013** 当 Up/Down 在 wrapped 内容中移动 cursor 或扩展 selection 时，系统必须基于
    当前 immutable TextFlow source-to-cell map 跨 visual row 移动，并尽量保持 preferred
    terminal cell column；无法命中同列时选择该 row 最接近的合法 grapheme boundary。
    Left/Right 仍按 source grapheme 顺序移动，Home/End 的 logical-line 语义必须保持兼容并
    文档化。
14. **B-014** 当 composer 获得有效 content width 时，可见高度必须等于
    `clamp(visual_row_count, 1, max_visible_lines)`。内容超过上限时只移动 visual-row viewport
    以保持 cursor 可见，不得删除 source、把 item count 当 row count或维护第二套 wrapping。
    `max_visible_lines=0` 必须在配置阶段 typed 拒绝。
15. **B-015** 当 width、prompt、border/padding 或其他影响 content width 的输入改变时，
    composer 必须在同一新 frame 使用 GH-58 TextFlow 重新 flow/reproject；cursor 仍指向相同
    source grapheme，selection source ranges 不变，visible height 与 scroll row 随新 mapping
    更新。旧 viewport projection 不得作为当前 resize 的成功结果。
16. **B-016** 空草稿必须显示一个可编辑 visual row。若草稿以 LF 结尾，GH-58 logical source
    map 仍按其 trailing-break 合同消费原 bytes，而 composer 另外显示一个映射到 end
    insertion position 的可编辑 caret row；该 synthetic caret row 不得伪造 source byte
    range。content width=0 时高度仍为 1、无越界 cell，cursor 明确标为 clipped。
17. **B-017** 配置冲突、非法 control、limit overflow、无效 position/selection、算术溢出、
    stale submission token 与 GH-58 flow failure 必须由 closed typed error family 暴露，
    `Error::source` 保留底层 `TextFlowError`。任一失败不得返回看似成功的 `Changed` /
    `Submitted`、部分 state 或旧 projection；GH-60 layout/render errors 保持其独立 checked
    boundary，不得压成 composer 字符串错误。公开 error enums 必须保持可穷举的 closed
    category set，不得用 `#[non_exhaustive]` 或 catch-all/string variant 破坏 exhaustive
    matching；可扩展的 action/event enums 与 error enums 的策略必须明确区分。
18. **B-018** 现有 `TextAreaState`、`Position { row, col }`、`Selection { start, end }`、
    `TextAreaKeyMap`、`TextAreaAction`、`handle_textarea_input*` 与正常 ASCII 行为必须保持
    source compatible。`Position.col` 明确定义为 logical-line grapheme ordinal；该 Unicode
    correctness 修正、`char_count` 与既有 max/limit 的保留语义必须形成迁移说明和 crate 外
    compile/behavior fixture。不得给 public field-addressable `Position` / `Selection` 增
    required field，也不得给现有 exhaustive `InteractionMode` / `TextAreaAction` 增 variant。
19. **B-019** 当终端把 IME 结果作为 committed text 交付时，连续 `e` 后 combining mark、
    连续 CJK scalar 与一次多-scalar committed input 必须得到相同 grapheme-safe 最终状态。
    native preedit、候选窗口与 composition update 在缺少 terminal event contract 时必须明确
    标为未支持，不能把 committed-text 测试宣传成完整 IME 支持。
20. **B-020** 当 runtime 收到 `crossterm::Event::Paste(content)` 时，必须 exactly once
    dispatch 到当前 paste handlers、记录 activity 并请求 render；不得落入 wildcard 丢弃，
    不得调用 key handlers，也不得把 paste 的换行解释成 submit shortcut。无 handler 时仍
    安全完成，不产生隐藏 global composer state。
21. **B-021** 同一个 `ChatComposerState` 的 mutation 由单一 caller 串行拥有；immutable
    projection 可只读共享；content、cursor、selection、preferred visual column、pending/
    acknowledgement state 或其他 projection-observable state 每次成功改变都必须让 checked
    state revision 恰好递增一次，overflow 必须在 commit 前 typed 失败且零 mutation。
    printable text、paste、Left/Right、logical Home/End、delete、submit/ack 与其他只依赖
    source state 的 action 不得因尚未 render 新 projection 而被拒绝；只有必须读取 visual
    row/cell geometry 的 Up/Down 及其 selection variants 要求 exact-current projection。
    stale geometry action typed 失败且零 mutation。重复 key/paste 事件各执行一次，重复 render
    不重复 mutation，Ignored/Handled-no-change/Cancelled/flow failure 不递增 revision，下一
    action 从最后 committed state 开始。
22. **B-022** `examples/claude_input_box.rs` 必须手工迁移到公开 composer state、handler、
    keymap 与 view；迁移后不得继续定义私有 input chars/cursor/wrapping/visible-row state，
    也不得直接输出 ANSI 来定位 live composer。其 transcript 演示可保持原有职责；本 issue
    明确不迁移 `glm_chat`。
23. **B-023** GH-64 只有在 implementation PR 当前 exact head 的正例、Unicode 边界、typed
    failure、固定 seed 的 property、paste routing、resize/render 与 compatibility tests
    全部以 `-- --exact` 实际运行且非 `#[ignore]`；changed executable line coverage 至少 80%，grapheme
    edit/selection、composer ingress/keymap/submit、runtime paste 与 projection 的全部声明
    critical paths changed line+branch coverage 100%。chat scope 必须使用不可由 child
    `allow`/`expect` 降级的 missing-docs policy；固定 public API inventory 中每个新增
    error/action/config/token/payload/state/projection/handler/view 都必须绑定各自唯一、
    nonignored、实际执行的 crate 外 exact compile-example test；测试体必须直接 type-check、
    构造或调用该 symbol 并断言可观察结果，`cfg(any())`、静态死分支或未展开 macro 中的名字
    不算证据。`ChatComposer` 另有 selector list exact-one、实际 1 passed/0 ignored 的 runnable
    doctest。Claude adoption 必须由剥离 comment/string/char/raw-string、disabled cfg item、
    静态死代码和未展开 macro 后的 production Rust token evidence，加上调用与 `main` 相同
    production composer path、断言真实 state/projection/submit payload 的 exact example test
    共同证明；伪造 token 负例必须失败。全部
    examples、CI、independent review、reviewThreads 与 SpecRail PR gate 全绿时才可声明完成；
    零匹配 filter、ignored/零执行、空或可伪造 source evidence、旧 SHA 或手工视觉演示不算证据。
24. **B-024** GH-64 implementation 必须基于已合入 GH-58 与 GH-60 implementation 的 exact
    merge commits，并证明当前 head 同时包含二者。spec-only PR、open branch、stacked spec
    ancestry 或预计 merge 不满足依赖；实现前 fresh duplicate search 与真实 API/path 核对
    发现漂移时必须先更新并重新 review 本 packet。

## 验收标准

- [ ] grapheme fixtures 覆盖 combining、ZWJ family、flags、variation selector、零宽
      grapheme、CJK 与跨行 selection，证明 B-002、B-003、B-012、B-013。
- [ ] key 与 paste fixtures 覆盖 multi-scalar、LF/CRLF/CR、tab、非法 controls、limits 与
      empty paste，逐项证明原子 ingress 和 typed failure，覆盖 B-004 至 B-006、B-017。
- [ ] keymap fixtures 覆盖 Enter、Shift+Enter、Alt+Enter fallback、custom binding conflict、
      blank submit、有效 submit、容量 16 tombstone、首次/立即重复/新草稿或新 pending 后重复/
      未知/淘汰 token 与四种 mode 状态，证明 B-007 至 B-011。
- [ ] TextFlow/render fixtures 覆盖 width=0/1、长 token、prompt width、1 到上限高度、超过
      上限 scroll、trailing LF 与连续 resize，证明同一 projection 驱动 cursor、selection、
      auto-grow 和 visible output，覆盖 B-014 至 B-016、B-021。
- [ ] crate 外 compatibility fixtures 证明既有 TextArea public surface 编译、ASCII 结果
      不变且 Unicode position 迁移有文档；committed IME-like tests 不声称 preedit 支持，
      覆盖 B-018、B-019。
- [ ] runtime fixture 证明一个 `Event::Paste` 只触发一次 paste handler、零次 key handler并
      请求 render；无 handler 路径安全，覆盖 B-020。
- [ ] `claude_input_box` 完成手工迁移且不再拥有私有 cursor/wrap state，`glm_chat` 无 diff；
      production-only lexical audit 的 comment/string、`cfg(any())`、dead branch、unexpanded
      macro负例和断言真实 production composer state/output 的 exact example test均通过，
      覆盖 B-022、B-023。
- [ ] chat-scoped `forbid(missing_docs)` 不可由 child 降级；public API inventory 的
      逐 symbol crate 外 compile-example exact tests 与 `ChatComposer` doctest 都实际运行，
      覆盖 B-001、B-023。
- [ ] 当前 implementation head 的 full Rust/docs/example/coverage、独立 review、
      reviewThreads、CI、SpecRail gate、duplicate evidence 与 GH-58/GH-60 merged ancestry
      证据满足 B-023、B-024。

## 边界情况清单

| 类别 | 判定（covered: B-xxx / N/A + 原因） |
| --- | --- |
| 空/缺失输入 | covered: B-005、B-008、B-016 |
| 错误与失败路径 | covered: B-004、B-005、B-006、B-007、B-009、B-015、B-017、B-023 |
| 授权/权限 | N/A：composer 不读取权限、不执行工具；控制 payload 不得被解释为命令或授权（B-006） |
| 并发/竞态 | covered: B-009、B-015、B-020、B-021 |
| 重试/幂等 | covered: B-009、B-020、B-021 |
| 非法状态转换 | covered: B-007、B-009、B-010、B-017、B-021 |
| 兼容/迁移 | covered: B-018、B-019、B-022、B-024 |
| 降级/回退 | covered: B-006、B-015、B-016、B-017、B-019 |
| 证据与审计完整性 | covered: B-023、B-024 |
| 取消/中断 | covered: B-009、B-011、B-017、B-021 |

## 发布说明

GH-64 首次发布 `rnk::components::chat::composer`，不拆独立 crate。发布说明必须明确：

- `Position.col` 的 Unicode 语义为 grapheme ordinal，既有字段形状和 ASCII 行为不变；
- `char_count`、max-line 与 total-limit 的既有计数语义，以及新增 checked API 的迁移方式；
- `InteractionMode` 未增加 submitting variant，提交确认通过 token 显式完成；
- success acknowledgement token 只在容量16 FIFO tombstone ring内提供重复成功幂等性；
- source-only action 不依赖 render cadence，只有 visual geometry action要求 fresh projection，
  且所有 projection-observable mutation 使用 checked revision；
- composer error enums 是 closed/exhaustive，`#[non_exhaustive]` 只用于可扩展 behavior/action；
- bracketed paste 的 event handling 与 terminal mode lifecycle 是两个独立责任；
- 只支持 committed IME text，不支持 native preedit/candidate UI；
- 本 issue 只迁移 `claude_input_box`，不迁移 `glm_chat`；
- GH-58 TextFlow 与 GH-60 checked renderer/layout errors 是依赖，不是 composer 内复制的算法
  或 generic error。
