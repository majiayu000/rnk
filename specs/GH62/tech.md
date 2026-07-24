# Tech Spec：后端无关的会话模型与状态机

## Linked Issue

GH-62: https://github.com/majiayu000/rnk/issues/62

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":62,"complete":true,"paths":["src/components/chat/mod.rs","src/components/chat/model.rs","src/components/chat/error.rs","src/components/chat/state.rs","src/components/chat/reducer.rs","src/components/mod.rs","src/prelude.rs","tests/chat_conversation_contracts.rs"],"spec_refs":["specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本 packet 只规划 GH-62。GH-57 是 umbrella tracking，GH-63、GH-65、GH-66、GH-67 是
下游；这些 issue 不属于本 implementation diff。

## Codebase Context

以下锚点已在写作基线 `e4a89ae128533270d28d768d49977a05a389a582` 上通过 Read/grep 核实。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 简单消息展示 | `src/components/display/message.rs:10`, `src/components/display/message.rs:40` | `MessageRole` 与 `Message` 只表达角色、字符串和可选前缀，没有稳定消息 ID、内容块或生命周期 | 新模型必须独立新增并保持旧 API，不应把 reducer 塞进展示组件 |
| 现有 Tool/Thinking 展示 | `src/components/display/message.rs:162`, `src/components/display/message.rs:206` | `ToolCall` 只有 name/args，`ThinkingBlock` 只有 content/max_lines；均不是会话状态 | GH-62 定义 typed model，GH-63 才负责新 block views |
| 组件公共表面 | `src/components/mod.rs:3`, `src/components/mod.rs:23`, `src/components/mod.rs:57` | components 以模块加 re-export 组织，interaction contracts 已有独立模块 | `components::chat` 应沿用该结构，不创建平行 crate |
| 推荐应用表面 | `src/prelude.rs:26`, `src/prelude.rs:55` | `rnk::prelude::*` 是推荐稳定入口，并导出当前 Message 类型 | 新 chat model 要进入推荐 surface，同时保留旧导出 |
| 本地聊天模型重复 | `examples/rnk_chat.rs:13`, `examples/rnk_chat.rs:20`, `examples/rnk_chat.rs:63` | example 自定义 `ChatMessage`/`Role` 并直接 push vector | 证明核心模型不存在；本 issue 不迁移 example |
| provider wire 类型 | `examples/glm_chat.rs:41`, `examples/glm_chat.rs:54`, `examples/glm_chat.rs:79` | example 使用 serde、`Value` 和 provider tagged enums | wire 类型必须停留在 adapter/example，不能进入核心 public API |
| 依赖边界 | `Cargo.toml:43`, `Cargo.toml:66`, `Cargo.toml:76` | reqwest 为可选生产依赖；serde/serde_json 仅为 dev dependencies | GH-62 无需修改 Cargo.toml，也不新增 provider/JSON/runtime 依赖 |
| 公共表面测试 | `tests/prelude_surfaces.rs:1` | integration test 已验证推荐 prelude 可从外部 crate 使用 | 新 `chat_conversation_contracts` 应从 public surface 测试，不依赖 private internals |

## 设计方案

### 1. 模块和所有权

在 `src/components/chat/` 新增五个小模块：

- `model.rs`：所有 public owned data types、validated identifiers、消息/block/nested status、
  event/update/outcome。
- `error.rs`：闭集 `ConversationError` 及必要的 validation error context；每个失败 variant
  携带 message/event/index/expected 等可诊断字段，不使用字符串总括所有错误。
- `state.rs`：chat-internal transition/correlation/cross-level helpers、`ConversationState`、
  只读访问器、revision、expected sequence 和有界 processed-event ledger；内部字段不允许
  调用方绕过 reducer 直接修改。
- `reducer.rs`：`ConversationState::apply_event`、完整 preflight、staged mutation 和一次 commit。
- `mod.rs`：模块文档、受控 public re-export、可编译 end-to-end rustdoc 示例，以及只作用于
  `components::chat` 及其子模块、不可由 child 降级的 `#![forbid(missing_docs)]`。

`src/components/mod.rs` 暴露 `pub mod chat` 并 re-export GH-62 的推荐类型；`src/prelude.rs`
导出同一组类型。`src/lib.rs` 已公开 `components`，无需修改。现有
`display::Message`、`MessageRole`、`ToolCall`、`ThinkingBlock` 不修改。

T1 创建五个可编译文件，只发布完整 data model；`error.rs`/`state.rs`/`reducer.rs` 是 private
skeleton，因此 T1 不运行或认领完整 B-001 public-surface gate。T2 串行接管 `error.rs`/
`state.rs`，发布 typed error、state constructor/read-only accessors，并以 `pub(super)`
实现/测试 pure helpers。T3 接管 `state.rs`/`reducer.rs` 与 `model.rs` test module，
实现/导出 apply API 后才运行完整 B-001 与 GH-57 child bridge tests；每文件始终单 writer。

### 2. 公共类型

所有新增 data/guard/outcome 类型使用 private fields、fallible constructor 和只读 accessor；
`MessageBlock`、`ConversationUpdate`、`ConversationError` 等扩展性 enum 标记
`#[non_exhaustive]`，闭集 `ChatRole` 除外。公共类型只含标准库 owned values：

- `MessageId(u64)`、`BlockId(u64)`；`ThinkingId`、`ToolCallId` 是 trim 后非空字符串；
  `UpdateId` 另公开 `new`、`TryFrom<String>`、`Display`、`as_str`。
- `ConversationRevision(u64)` 初态 0；`MessageRevision(NonZeroU64)` 提供
  `pub const INITIAL: Self = Self(NonZeroU64::MIN)`、`get()`、fallible `new(u64)` 与内部
  `checked_next()`。
- `ChatRole::{User, Assistant, System, Tool}` 精确闭合；`From<ChatRole> for MessageRole` 是
  four-way total mapping，`TryFrom<MessageRole>` 对同名四项成功，对旧 ToolResult/Error 返回
  `LegacyRoleConversionError`，不降级或猜测。
- `ChatMessage` 含 id/role/status/revision、非空 `Vec<MessageBlockEntry>` 与
  `ChatMessageMetadata { author: Option<MessageAuthor>, timestamp: Option<MessageTimestamp> }`。
  metadata 及两个 string newtype 均为 private fields + constructor/accessor；author 是显示名，
  timestamp 是应用格式化的显示文本；Some trim 后非空，None 不造值。GH-63 只能显式借用
  两者 `as_str()` 到 view metadata。
  `MessageBlockEntry` 含稳定 `BlockId` 与 `MessageBlock`；静态 block 也必须有 ID。
- `MessageBlock` 覆盖 Text、Markdown、Code、Thinking、ToolCall、ToolResult、Error、Diff、
  Quote、Link、TerminalAttachmentSummary；lifecycle/error payload 分别为 private-field
  `ThinkingContent { id, content, status }`、`ToolCallContent { call_id, name, arguments, status }`、
  `ToolResultContent { call_id, output, status }`、`ErrorContent { message, source }`，每个字段均有
  constructor/accessor；`ErrorSource` 是 trim-nonempty 应用来源标签。其他复杂 payload 精确为：
  `CodeContent { language: Option<String>, content: String }`、
  `DiffContent { language: Option<String>, content: String }`、
  `QuoteContent { content: String, attribution: Option<String> }`、
  `LinkContent { label: String, target: String }`、
  `TerminalAttachmentSummary { name: String, media_type: Option<String>, summary: String }`。
- `ToolArgument { name: String, value: TypedValue }` 与
  `TypedField { name: String, value: TypedValue }` 使用 private fields/constructor/accessor。
  `TypedValue` 是 `Null | Bool(bool) | Integer(i64) | Decimal(DecimalValue) |
  String(String) | List(Vec<TypedValue>) | Object(Vec<TypedField>)` 的 closed owned enum；
  Decimal 规范文本为 `0`，或可选 `-` + 无前导零非零整数（或 `0`）+ 可选且末位非零的小数；
  禁止 `-0`、正号、指数、空白、NaN/Infinity 和 `1.0`/`1.00` 等同值异形，从而保持 `Eq`。
  argument/object 同层名字 trim 后非空且唯一、顺序保留；list/object/字符串值允许空。
- `FailureCause` 是 trim 后非空 string 的 private-field value。`MessageStatus`、
  `ThinkingStatus`、`ToolCallStatus`、`ToolResultStatus` 的 Failed variant 均为
  `Failed(FailureCause)`，并通过 accessor 返回 cause；Fail payload 直接携带该类型。
- `ConversationGuard` 封装 expected conversation revision；`MessageMutationGuard` 再包含
  message ID 与 expected message revision。guard/payload 都通过 constructor 创建。
- `ApplyOutcome` 含 applied `ConversationRevision` 与确定性 `Vec<AffectedMessage>`；
  affected entry 含 message ID、`Option<MessageRevision>` previous、applied revision、
  `AffectedMessageDisposition::{Present, Deleted}`；previous 仅对新 Push/Resend 为 `None`，
  其余为 `Some`，全部只通过 accessor 读取。

`ConversationUpdate` 使用带 private-field payload 的 typed variants：
`Push`、`AppendText`、`AppendMessageBlock`、`InsertMessageBlock`、`ReplaceBlock`、
`Complete`、`Cancel`、`Fail`、`EditMessage`、`DeleteMessage`、`Resend`。Push payload
携带 conversation guard；所有已有 message mutation 携带 `MessageMutationGuard`；Resend
携带 source guard 与新 message。public rustdoc 使用 `ConversationUpdate::{push,
append_text,complete}` constructor，不要求 variant struct literal。

`BlockId` 是 conversation-state-lifetime mutation/UI identity，所有 messages 共用一个
namespace；`ThinkingId` 是 per-MessageId lifetime identity，`ToolCallId` 是 conversation-wide
correlation identity。state-wide BlockId 与 per-message ThinkingId seen/retired sets 均独立于
processed-event ledger；edit 移除 Thinking 后同 message 不得重建该 ID，不同 message 可复用。
每个 ToolCallId 另有 result slot `Vacant | Occupied(ToolResultLocation) | Retired`，location
包含 MessageId/BlockId；显式 restore 必须恢复并验证全部 histories。
### 3. 状态机和 reducer

`ConversationState::new(initial_sequence: u64, ledger_capacity: NonZeroUsize)` 显式设置
顺序起点、有界 ledger 和 revision 0。只读 API 暴露 messages、typed revisions、expected
sequence 与 retention boundary，不返回可变内部引用。公共
`fn snapshot(&self) -> ConversationStateSnapshot` 与
`fn try_restore(snapshot: ConversationStateSnapshot) -> Result<Self, ConversationError>` 闭合恢复入口。
snapshot 精确含 messages、conversation revision、expected sequence、`RetentionHistory` 与
`ConversationIdentityHistory`；前者为 `{ capacity, records: Vec<ProcessedEventRecord>,
evicted_through }`，record 闭合 `{ event, outcome }`；后者含 seen/retired MessageId/BlockId、
per-message Thinking histories、ToolCall/result-slot histories。所有 public
snapshot/history/record 类型仅有 fallible constructors/accessors；restore 验证 record 顺序、
容量、event ID/outcome 唯一与 revisions、active/history/tombstone disjointness、result locations，
任一缺失或矛盾返回 typed error 且不产生 state。

`apply_event` 固定执行顺序：

```text
validate event_id
-> retained exact replay / event-ID conflict
-> stale / gap / ReplayOutsideRetention
-> checked next sequence -> checked next ConversationRevision
-> expected conversation revision -> target lookup -> expected message revision
-> resolve affected set -> checked next MessageRevision for each existing affected message
-> stage and validate structure, lifecycle, tombstones, global correlation matrix
-> one commit of messages/tombstones/revisions/sequence/ledger -> typed ApplyOutcome
```

exact replay 返回 ledger 保存的原 outcome（含 affected list），不推进任何 counter。
同 ID 不同 event 返回 `EventIdConflict`。expected 为 `u64::MAX` 的同序新事件即使 update
malformed 也返回 `SequenceExhausted`；conversation revision exhaustion 再先于 guard/target
validation。affected messages 先按 mutation 前 message 顺序确定，逐一 checked revision；
新 Push/Resend message revision 为 `INITIAL` 且排在 outcome 末尾。任一错误使完整 state
（含 tombstones/ledger）保持 `Eq`；禁止 wrapping、saturating、panic 或 fallback。

### 4. 更新和嵌套状态

- Push：conversation guard 必须匹配；message/entry 非空，MessageId/BlockId/lifecycle
  identities 从未在 state-wide namespaces 使用，message 与 lifecycle blocks 均为 Pending。
- AppendText/ReplaceBlock 以 BlockId 定位。前者只向 Text/Markdown/Code/Thinking 追加非空
  delta；后者只保留同 entry ID、同 kind、同 lifecycle identity 的合法状态迁移。
- AppendMessageBlock 在尾部、InsertMessageBlock 在 checked position 加入全新 entry；
  均只作用于 active message，不重排已有 entry。新 lifecycle block 从 Pending 开始，静态
  late-discovered block payload 非空；成功时 Pending message 进入 Streaming。
- payload validation：language/media type/attribution/Error source 的 `Some` 必须 trim 后非空；
  Diff/Quote/Error message、Link label/opaque target、attachment name/summary 必须非空；
  DecimalValue 使用第 2 节唯一规范 grammar。ToolArgument/
  TypedField 同层名字唯一；missing value 与 explicit Null 区分。
- Complete 只接受 Streaming + lifecycle 全终态，或 Pending + 至少一个 Text/Markdown/
  Code/Error/Diff/Quote/Link/TerminalAttachmentSummary 非空静态 payload 且无 lifecycle
  block；不制造 dummy 内容。Cancel/Fail 原子终结目标 active nested 与跨
  message active call/result counterpart，其他 message top-level status/无关内容不变。
- EditMessage 以完整非空 entries 替换内容，保留 message id/role/status/metadata。保留 BlockId 必须
  同 kind/identity，removed IDs 退役；active message 可加入 Pending lifecycle entry，terminal
  message 只可加入静态 entry，且 retained lifecycle status/identity 不得改变。removed
  ThinkingId 先 stage 到该 message retired set；candidate 内或后续重用原子失败。完整
  candidate（含 tombstones）通过 global matrix 后才提交。
- DeleteMessage 先 checked 目标 message revision，再 stage 删除与 tombstones。删除 result
  而保留 call 时其 result slot 原子转 `Retired`，同 call 后续结果即使 ledger eviction 后也拒绝；
  删除 call 但其他 message 仍有 result 返回 typed orphan/correlation error；同删 call/result
  时 call 与 slot 均退役。restore 单独验证 result-slot history，不从 live call 猜测。MessageId 永不复用。
- Resend 只接受 terminal source 与匹配 guard；source 逐值/revision 不变且不在 affected
  outcome。新 message 必须 same role、Pending、state-wide 全新 MessageId/BlockId 与合法
  identities，revision 从 1 开始；因 Thinking namespace 随 MessageId，新 message 可复用
  source ThinkingId，ToolCallId 仍须 conversation-wide fresh。

Thinking、Tool Call 和 Tool Result 的 transition table 由纯 helper 返回
`Result<(), ConversationError>`，table-driven tests 枚举状态笛卡尔积，避免 if/else 漏边。
若 ToolResult 不存在，任一合法 ToolCall status 均允许；若存在，额外的纯 correlation helper
必须枚举并只接受下表：

| ToolCall status | Allowed ToolResult status |
| --- | --- |
| Pending | absent only |
| Running | absent, Pending, Streaming |
| Succeeded | absent, Pending, Streaming, Complete, Cancelled, Failed |
| Cancelled | absent, Cancelled |
| Failed | absent, Failed |

Succeeded+Cancelled/Failed(_) 表达“call 成功但 result 传输/消费取消或失败”。矩阵在所有可能
改变 blocks 的 updates 上验证；Complete 还要求 pair 双方终态。Cancel/Fail 的 affected set
包含实际改变的 counterpart message，每条只推进一次 MessageRevision。Fail 把 payload 的
同一 `FailureCause` clone 到目标 MessageStatus 和每个被终结的 Thinking/ToolCall/ToolResult
Failed variant；equality/accessor tests 证明 reason 未丢失。任何 terminal message都不得含
active nested status。

### 5. Ledger、counter 与幂等边界

ledger 保存最多 `ledger_capacity` 个已接受 record：
`event_id -> exact event fingerprint/data + original ApplyOutcome`，并保留 sequence 顺序以逐出
最老记录。容量必须非零；original affected list 同 event 一起保留。旧 sequence 未命中但落在
该 state 已逐出范围时，先返回 `ReplayOutsideRetention`，不能被后续 UnknownMessage/Block
覆盖；fresh state 没有恢复 boundary 时不得伪造该结论。sequence、conversation revision 与
message revision 都只 checked increment；private max fixtures 分别证明 exhaustion 原子性。

### 6. Adapter、兼容与错误策略

两个结构不同的 mock provider adapter 产生相同 typed events/outcomes/state。核心不解析 JSON
且不实现 serde wire schema；adapter 必须显式读取 revision/BlockId，经 fallible constructor
拒绝 revision 的 missing/zero/negative/overflow 与 BlockId 的 missing/negative/overflow，
不能 default/truncate。adapter 还必须把 wire scalars/arrays/objects 完整转换为 closed
`TypedValue`，unknown value kind/field 返回 typed error，不塞入未声明字段。

现有 `display::Message` 等 API 保持源码和行为不变；`components::chat::<Type>` 是新权威路径。
GH-63 的 `ChatBlockRef` 必须分别借用 `&ThinkingContent`、`&ToolCallContent`、
`&ToolResultContent`、`&ErrorContent`，并只经 GH-62 public role/status/payload accessors 投影；
metadata 仍显式借用 author/timestamp `as_str()`，不允许 private-field hack。
private-field constructors 避免 required-field struct-literal break；`non_exhaustive` enum
要求外部 match 保留 wildcard。`ConversationError` 实现 `Display`/`Error` 并为 stale
conversation/message revision、unknown/retired identity、message revision exhaustion、
orphan-on-delete 与 resend precondition 提供 typed variants，不 warning+fallback。
`chat/mod.rs` 的 scoped `#![forbid(missing_docs)]` 必须由普通 `cargo check` 执行，覆盖所有
public type、variant、field、constructor、accessor 和 apply API。`forbid` 使 child
`allow(missing_docs)` / `expect(missing_docs)` 成为编译错误；额外 source audit 扫描全部五个
chat source files 并拒绝 lint lowering 与 `doc(hidden)` 逃逸。doc test 只负责示例执行，
不能替代 missing-docs gate。

## Product-to-Test Mapping

所有过滤验证必须通过下方 helper；helper 使用 `--include-ignored`，先用 `--list` 证明精确
测试名唯一，再执行并严格断言 libtest 汇总为 `1 passed; 0 failed; 0 ignored`。因此零匹配、
多匹配、仅列出 ignored test、执行后 ignored 或没有实际通过都失败：

```sh
assert_exact_one_test_passed() {
  test_output="$1"
  printf '%s\n' "$test_output"
  result_count="$(printf '%s\n' "$test_output" | awk '/^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out; finished in /{count++} END{print count+0}')"
  test "$result_count" -eq 1 || {
    printf 'expected exactly one passed test result, matched %s\n' "$result_count" >&2
    return 1
  }
}
verify_chat_test() {
  test_name="$1"
  list_output="$(cargo test --test chat_conversation_contracts "$test_name" -- --exact --include-ignored --list 2>&1)" ||
    { printf '%s\n' "$list_output" >&2; return 1; }
  matched="$(printf '%s\n' "$list_output" | awk '/: test$/{count++} END{print count+0}')"
  test "$matched" -eq 1 || {
    printf 'expected exactly one test for %s, matched %s\n' "$test_name" "$matched" >&2
    return 1
  }
  run_output="$(cargo test --test chat_conversation_contracts "$test_name" -- --exact --include-ignored 2>&1)" ||
    { printf '%s\n' "$run_output" >&2; return 1; }
  assert_exact_one_test_passed "$run_output"
}
verify_chat_lib_test() {
  test_name="$1"
  list_output="$(cargo test --lib "$test_name" -- --exact --include-ignored --list 2>&1)" ||
    { printf '%s\n' "$list_output" >&2; return 1; }
  matched="$(printf '%s\n' "$list_output" | awk '/: test$/{count++} END{print count+0}')"
  test "$matched" -eq 1 || {
    printf 'expected exactly one lib test for %s, matched %s\n' "$test_name" "$matched" >&2
    return 1
  }
  run_output="$(cargo test --lib "$test_name" -- --exact --include-ignored 2>&1)" ||
    { printf '%s\n' "$run_output" >&2; return 1; }
  assert_exact_one_test_passed "$run_output"
}
verify_chat_missing_docs_gate() {
  python3 <<'PY'
from pathlib import Path
import re

paths = [
    Path("src/components/chat/mod.rs"),
    Path("src/components/chat/model.rs"),
    Path("src/components/chat/error.rs"),
    Path("src/components/chat/state.rs"),
    Path("src/components/chat/reducer.rs"),
]
missing = [str(path) for path in paths if not path.is_file()]
if missing:
    raise SystemExit(f"missing planned chat sources: {missing}")

mod_lines = paths[0].read_text(encoding="utf-8").splitlines()
guard_count = sum(line == "#![forbid(missing_docs)]" for line in mod_lines)
if guard_count != 1:
    raise SystemExit(
        f"expected exactly one chat-root #![forbid(missing_docs)], found {guard_count}"
    )

lowering = re.compile(r"(?:allow|expect)\([^)]*missing_docs[^)]*\)|doc\(hidden\)")
findings = []
for path in paths:
    source = path.read_text(encoding="utf-8")
    compact = re.sub(r"\s+", "", source)
    for match in lowering.finditer(compact):
        findings.append(f"{path}: forbidden missing-docs downgrade {match.group(0)}")
if findings:
    raise SystemExit("\n".join(findings))
PY
  cargo check --workspace --all-targets --all-features --locked
}
verify_chat_rustdoc_example() {
  source_path="src/components/chat/mod.rs"
  test -f "$source_path" || {
    printf 'missing chat module rustdoc source: %s\n' "$source_path" >&2
    return 1
  }
  normal_fences="$(rg -c '^//! ```rust$' "$source_path")" || return 1
  test "$normal_fences" -eq 1 || {
    printf 'expected exactly one ordinary rust doctest fence, matched %s\n' \
      "$normal_fences" >&2
    return 1
  }
  if rg -n '^//! ```[^`]*(ignore|no_run)' "$source_path"; then
    printf 'chat module doctest must not be ignored or compile-only\n' >&2
    return 1
  else
    rg_status="$?"
    if test "$rg_status" -ne 1; then
      printf 'chat doctest fence audit failed: %s\n' "$rg_status" >&2
      return "$rg_status"
    fi
  fi
  doctest_code="$(
    python3 - "$source_path" <<'PY'
from pathlib import Path
import re
import sys

source_path = Path(sys.argv[1])
lines = source_path.read_text(encoding="utf-8").splitlines()
starts = [index for index, line in enumerate(lines) if line == "//! ```rust"]
if len(starts) != 1:
    raise SystemExit(f"expected one ordinary rust fence, found {len(starts)}")
start = starts[0]
try:
    end = next(
        index
        for index in range(start + 1, len(lines))
        if lines[index] == "//! ```"
    )
except StopIteration:
    raise SystemExit("ordinary rust fence has no closing delimiter")

body = []
for line in lines[start + 1:end]:
    if not line.startswith("//!"):
        raise SystemExit("selected rust fence contains a non-module-doc source line")
    code_line = line[3:]
    if code_line.startswith(" "):
        code_line = code_line[1:]
    body.append(code_line)

tokens = [
    "ConversationUpdate::push",
    "ConversationUpdate::append_text",
    "ConversationUpdate::complete",
]
code = "\n".join(body)
code = re.sub(r'r(?P<hashes>#{0,16})".*?"(?P=hashes)', '""', code, flags=re.DOTALL)
code = re.sub(r'"(?:\\.|[^"\\])*"', '""', code, flags=re.DOTALL)
code = re.sub(r'/\*.*?\*/', '', code, flags=re.DOTALL)
code = re.sub(r'//.*$', '', code, flags=re.MULTILINE)
positions = []
for token in tokens:
    match = re.search(rf"{re.escape(token)}\s*\(", code)
    if match is None:
        raise SystemExit(f"selected rust fence is missing executable token: {token}")
    positions.append(match.start())
if not positions[0] < positions[1] < positions[2]:
    raise SystemExit("selected rust fence must order Push before AppendText before Complete")
print("\n".join(body))
PY
  )" || return 1
  test -n "$doctest_code" || {
    printf 'selected chat doctest fence is empty\n' >&2
    return 1
  }
  list_output="$(
    cargo test --doc -p rnk --all-features --locked \
      -- --include-ignored --list 2>&1
  )" || {
    printf '%s\n' "$list_output" >&2
    return 1
  }
  doc_names="$(
    printf '%s\n' "$list_output" |
      sed -nE 's/^(src\/components\/chat\/mod\.rs - components::chat \(line [0-9]+\)): test$/\1/p'
  )"
  matched="$(
    printf '%s\n' "$doc_names" |
      awk 'NF{count++} END{print count+0}'
  )"
  test "$matched" -eq 1 || {
    printf 'expected exactly one components::chat doctest, matched %s\n' \
      "$matched" >&2
    return 1
  }
  chat_filter="components::chat"
  filter_matches="$(
    printf '%s\n' "$list_output" |
      awk -v filter="$chat_filter" 'index($0, filter) && /: test$/{count++} END{print count+0}'
  )"
  test "$filter_matches" -eq 1 || {
    printf 'expected chat doctest filter to select one test, matched %s\n' \
      "$filter_matches" >&2
    return 1
  }
  run_output="$(
    cargo test --doc -p rnk --all-features --locked "$chat_filter" \
      -- --include-ignored 2>&1
  )" || {
    printf '%s\n' "$run_output" >&2
    return 1
  }
  assert_exact_one_test_passed "$run_output"
}
audit_forbidden_package_aliases() {
  metadata_file="$1"
  root_manifest="$2"
  source_dir="$3"
  python3 - "$metadata_file" "$root_manifest" "$source_dir" <<'PY'
import json, re, sys
from pathlib import Path
metadata_path, root_manifest = Path(sys.argv[1]), Path(sys.argv[2]).resolve()
source_dir = Path(sys.argv[3])
forbidden_packages = {
    "reqwest", "serde_json", "tokio", "anthropic", "openai", "crossterm", "ctrlc",
    "dirs_next", "libc", "secrecy", "keyring",
}
def rust_code_only(source):
    output, length = list(source), len(source)
    def blank(start, end):
        for index in range(start, end):
            if output[index] != "\n":
                output[index] = " "
    index = 0
    while index < length:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = length if end < 0 else end; blank(index, end); index = end
            continue
        if source.startswith("/*", index):
            start, index, depth = index, index + 2, 1
            while index < length and depth:
                if source.startswith("/*", index):
                    depth += 1; index += 2
                elif source.startswith("*/", index):
                    depth -= 1; index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("unterminated Rust block comment")
            blank(start, index); continue
        raw = re.match(r'(?:b|c)?r(?P<hashes>#{0,255})"', source[index:])
        if raw:
            start, delimiter = index, '"' + raw.group("hashes")
            end = source.find(delimiter, index + raw.end())
            if end < 0:
                raise ValueError("unterminated Rust raw string")
            index = end + len(delimiter); blank(start, index)
            continue
        prefix = 1 if source.startswith(('b"', 'c"'), index) else 0
        if source[index + prefix:index + prefix + 1] == '"':
            start, index = index, index + prefix + 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("unterminated Rust string")
            blank(start, min(index, length)); continue
        char_match = re.match(r"(?:b)?'(?:\\.|[^\\'\n])+'", source[index:])
        if char_match:
            end = index + char_match.end(); blank(index, end); index = end
            continue
        index += 1
    return "".join(output)
try:
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    packages = {package["id"]: package for package in metadata["packages"]}
    roots = [p for p in packages.values() if Path(p["manifest_path"]).resolve() == root_manifest]
    if len(roots) != 1:
        raise ValueError(f"expected one root package for {root_manifest}, found {len(roots)}")
    root_id = roots[0]["id"]
    nodes = [node for node in metadata["resolve"]["nodes"] if node["id"] == root_id]
    if len(nodes) != 1:
        raise ValueError(f"expected one resolve node for {root_id}, found {len(nodes)}")
    import_names = {name.replace("-", "_") for name in forbidden_packages}
    for dependency in nodes[0]["deps"]:
        package = packages[dependency["pkg"]]
        if package["name"].replace("-", "_") in forbidden_packages:
            import_names.add(dependency["name"].replace("-", "_"))
    source_paths = sorted(source_dir.glob("*.rs"))
    if not source_paths:
        raise ValueError(f"no Rust sources found in {source_dir}")
    findings = []
    for source_path in source_paths:
        source = source_path.read_text(encoding="utf-8")
        code = rust_code_only(source)
        for use_match in re.finditer(r"\buse\b(?P<body>[^;]*);", code, re.DOTALL):
            skip_alias = False
            source_words = []
            for token in re.finditer(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", use_match.group("body")):
                word = token.group(0).removeprefix("r#")
                if word == "as":
                    skip_alias = True
                    continue
                if skip_alias:
                    skip_alias = False
                    continue
                source_words.append(word)
                if word in import_names:
                    position = use_match.start("body") + token.start()
                    line = code.count("\n", 0, position) + 1
                    findings.append(f"{source_path}:{line}: forbidden use-tree source {word}")
            source_set = set(source_words)
            if "std" in source_set and (
                source_set & {"process", "env", "fs", "net"}
                or "io" in source_set and source_set & {"stdin", "stdout", "stderr"}
            ):
                line = code.count("\n", 0, use_match.start()) + 1
                findings.append(f"{source_path}:{line}: forbidden std capability use tree")
            if "crate" in source_set and source_set & {"cmd", "renderer", "runtime"}:
                findings.append(f"{source_path}:{code.count(chr(10), 0, use_match.start()) + 1}: forbidden crate capability use tree")
        for import_name in sorted(import_names):
            escaped = re.escape(import_name)
            patterns = [
                re.compile(rf"\bextern\s+crate\s+{escaped}\b"),
                re.compile(rf"(?<![A-Za-z0-9_])(?:::)?{escaped}\s*::"),
                re.compile(rf"(?<![A-Za-z0-9_]){escaped}\s*!"),
            ]
            positions = {m.start() for pattern in patterns for m in pattern.finditer(code)}
            for position in sorted(positions):
                line = code.count("\n", 0, position) + 1
                findings.append(f"{source_path}:{line}: forbidden crate token {import_name}")
        capability_patterns = [
            r"(?<![A-Za-z0-9_])(?:::)?std\s*::\s*(?:process|env|fs|net)\b",
            r"(?<![A-Za-z0-9_])(?:::)?std\s*::\s*io\s*::\s*(?:stdin|stdout|stderr)\b",
            r"(?<![A-Za-z0-9_])(?:env|option_env)\s*!",
            r"(?<![A-Za-z0-9_])crate\s*::\s*(?:cmd|renderer|runtime)\b",
        ]
        for pattern in capability_patterns:
            for match in re.finditer(pattern, code):
                line = code.count("\n", 0, match.start()) + 1
                findings.append(f"{source_path}:{line}: forbidden execution capability")
    if findings:
        print("\n".join(findings), file=sys.stderr)
        raise SystemExit(1)
except SystemExit:
    raise
except Exception as error:
    print(f"forbidden dependency metadata audit failed: {error}", file=sys.stderr)
    raise SystemExit(2)
PY
}
verify_forbidden_dependency_alias_detection() {
  fixture_dir="$(mktemp -d "${TMPDIR:-/tmp}/gh62-alias-audit.XXXXXX")" || return 1
  fixture_manifest="$fixture_dir/Cargo.toml"
  fixture_source="$fixture_dir/chat"
  fixture_metadata="$fixture_dir/metadata.json"
  mkdir -p "$fixture_source" || return 1
  printf '[package]\nname = "fixture"\nversion = "0.0.0"\n' >"$fixture_manifest"
  printf '{"packages":[{"id":"fixture","name":"fixture","manifest_path":"%s"},{"id":"serde","name":"serde_json","manifest_path":"%s/serde-json/Cargo.toml"}],"resolve":{"nodes":[{"id":"fixture","deps":[{"name":"json_alias","pkg":"serde"}]}]}}\n' \
    "$fixture_manifest" "$fixture_dir" >"$fixture_metadata"
  check_alias_fixture() {
    expected="$1"; label="$2"; fixture="$3"
    printf '%b\n' "$fixture" >"$fixture_source/model.rs"
    actual=0
    audit_forbidden_package_aliases "$fixture_metadata" "$fixture_manifest" "$fixture_source" \
      >/dev/null 2>&1 || actual="$?"
    test "$actual" -eq "$expected" || {
      printf 'dependency fixture %s: expected %s, got %s\n' "$label" "$expected" "$actual" >&2
      return 1
    }
  }
  check_alias_fixture 0 non-code '//! serde_json::Value\n// json_alias::Value\n/* serde_json::Value */\nconst NOTE: &str = "json_alias::Value";\nconst RAW: &str = r#"serde_json::Value"#;' &&
  check_alias_fixture 0 safe-group 'use crate::{safe as json_alias, other::{leaf as json_alias_2}};' &&
  check_alias_fixture 1 direct 'use json_alias as json;\nfn denied() { json::Value; }' &&
  check_alias_fixture 1 grouped-root 'use {json_alias as json};\nfn denied() { json::Value; }' &&
  check_alias_fixture 1 nested-group 'use crate::{json_alias as json};\nfn denied() { json::Value; }' &&
  check_alias_fixture 1 grouped-std-process 'use {std::{fmt, process::Command}};' &&
  check_alias_fixture 1 std-terminal-io 'fn denied() { std :: io :: stdout(); }' &&
  check_alias_fixture 1 env-macro 'const SECRET: &str = option_env!("TOKEN").unwrap_or("");' &&
  check_alias_fixture 1 crate-execution-module 'use crate::{runtime::Executor};'
  fixture_status="$?"
  rm -rf -- "$fixture_dir"
  return "$fixture_status"
}
verify_no_forbidden_chat_dependencies() {
  base_ref="${1:-origin/main}"
  test -d src/components/chat || {
    printf 'missing planned chat source directory\n' >&2
    return 1
  }
  base_sha="$(git merge-base "$base_ref" HEAD)" || return 1
  if ! git diff --quiet "$base_sha"...HEAD -- Cargo.toml Cargo.lock; then
    printf 'GH62 must not change Cargo manifests or lockfile\n' >&2
    git diff --name-only "$base_sha"...HEAD -- Cargo.toml Cargo.lock >&2
    return 1
  fi
  metadata_file="$(mktemp "${TMPDIR:-/tmp}/gh62-cargo-metadata.XXXXXX")" || return 1
  cargo metadata --format-version 1 --locked >"$metadata_file" || {
    metadata_status="$?"
    rm -f -- "$metadata_file"
    return "$metadata_status"
  }
  audit_status=0
  audit_forbidden_package_aliases \
    "$metadata_file" "$(pwd)/Cargo.toml" src/components/chat ||
    audit_status="$?"
  rm -f -- "$metadata_file"
  return "$audit_status"
}
verify_chat_new_code_coverage() {
  head_sha="$(git rev-parse HEAD)" || return 1
  coverage_dir="target/specrail/GH62/coverage-$head_sha"
  mkdir -p "$coverage_dir" || return 1
  cargo tarpaulin --workspace --all-features --locked \
    --include-files 'src/components/chat/*.rs' \
    --out Xml --output-dir "$coverage_dir" --fail-under 80 || return 1
  coverage_report="$coverage_dir/cobertura.xml"
  test -s "$coverage_report" || return 1
  python3 - "$coverage_report" <<'PY'
import sys
import xml.etree.ElementTree as ET

report = sys.argv[1]
root = ET.parse(report).getroot()
line_rate = float(root.attrib["line-rate"])
required = {
    "src/components/chat/mod.rs",
    "src/components/chat/model.rs",
    "src/components/chat/error.rs",
    "src/components/chat/state.rs",
    "src/components/chat/reducer.rs",
}
reported = {
    node.attrib["filename"].replace("\\", "/")
    for node in root.findall(".//class")
}
missing = sorted(
    path
    for path in required
    if not any(name == path or name.endswith(f"/{path}") for name in reported)
)
if missing:
    raise SystemExit(f"coverage artifact omitted planned chat files: {missing}")
if line_rate < 0.80:
    raise SystemExit(f"chat new-code line rate {line_rate:.4f} is below 0.80")
print(f"chat new-code line rate: {line_rate:.4f}")
PY
  test "${coverage_dir##*/coverage-}" = "$(git rev-parse HEAD)"
}
```

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `chat/model.rs`, `chat/error.rs`, public exports | `verify_chat_test public_model_is_typed_and_constructible`；`verify_chat_test chat_message_metadata_is_closed_and_optional`；`verify_chat_test chat_roles_and_legacy_mapping_are_closed`；`verify_chat_test error_content_is_typed_and_source_aware`；`verify_chat_lib_test components::chat::model::tests::gh62_provider_independent_model_contract` |
| B-002 | closed typed payloads and identity namespaces | `verify_chat_test every_block_variant_preserves_typed_data`；`verify_chat_test closed_typed_values_reject_invalid_payloads`；`verify_chat_test lifecycle_payloads_are_closed_and_projectable`；`verify_chat_lib_test components::chat::state::tests::thinking_replacement_requires_same_identity`；`verify_chat_lib_test components::chat::state::tests::thinking_id_message_lifetime_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::identity_and_correlation_helpers_cover_all_namespaces`；`verify_chat_test lifecycle_identity_namespaces_are_scoped_and_correlated` |
| B-003 | message/metadata/block/typed-value validation | `verify_chat_test empty_and_missing_inputs_have_explicit_results`；`verify_chat_test chat_message_metadata_is_closed_and_optional`；`verify_chat_test closed_typed_values_reject_invalid_payloads`；`verify_chat_test decimal_values_have_one_canonical_representation`；`verify_chat_lib_test components::chat::model::tests::gh62_update_id_public_construction`；`verify_chat_lib_test components::chat::model::tests::gh62_empty_and_missing_contract`；`verify_chat_lib_test components::chat::state::tests::identity_and_correlation_helpers_cover_all_namespaces`；`verify_chat_test duplicate_lifecycle_identities_are_rejected_atomically` |
| B-004 | Push preflight / unique ID and lifecycle identities | `verify_chat_test push_is_unique_and_atomic`；`verify_chat_test duplicate_lifecycle_identities_are_rejected_atomically`；`verify_chat_test lifecycle_identity_namespaces_are_scoped_and_correlated` |
| B-005 | message transition helper | `verify_chat_lib_test components::chat::model::tests::gh62_message_transition_matrix`；`verify_chat_lib_test components::chat::state::tests::message_transition_matrix_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::static_completion_readiness_matrix_is_exhaustive`；`verify_chat_test static_message_completes_without_dummy_append`；`verify_chat_test empty_static_message_requires_content_before_complete`；`verify_chat_test pending_message_with_active_nested_block_cannot_complete` |
| B-006 | BlockId-targeted AppendText/AppendMessageBlock/InsertMessageBlock branches | `verify_chat_lib_test components::chat::state::tests::append_block_cross_level_rules_are_exhaustive`；`verify_chat_test streaming_deltas_are_ordered_lossless_and_typed`；`verify_chat_test append_block_supports_late_discovered_typed_blocks`；`verify_chat_test append_block_rejects_invalid_blocks_atomically`；`verify_chat_test edit_and_insert_are_revisioned_and_identity_safe` |
| B-007 | same-kind ReplaceBlock helper and reducer branch | `verify_chat_lib_test components::chat::state::tests::replace_block_kind_rules_are_exhaustive`；`verify_chat_test replace_block_validates_before_commit`；`verify_chat_test replace_block_requires_same_variant_and_identity` |
| B-008 | nested/correlated transition helpers and typed failure payloads | `verify_chat_lib_test components::chat::state::tests::nested_status_transition_matrices_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::tool_call_result_correlation_matrix_is_exhaustive`；`verify_chat_test correlated_lifecycle_updates_are_atomic`；`verify_chat_test failure_causes_are_typed_and_propagated` |
| B-009 | complete/cancel/fail reducer branches | `verify_chat_lib_test components::chat::model::tests::gh62_terminal_revision_race_contract`；`verify_chat_lib_test components::chat::model::tests::gh62_cancellation_contract`；`verify_chat_lib_test components::chat::state::tests::terminal_updates_are_single_effect_and_race_safe`；`verify_chat_lib_test components::chat::state::tests::tool_call_result_correlation_matrix_is_exhaustive`；`verify_chat_test correlated_lifecycle_updates_are_atomic`；`verify_chat_test cancel_cascades_across_correlated_messages_atomically`；`verify_chat_test fail_cascades_across_correlated_messages_atomically`；`verify_chat_test failure_causes_are_typed_and_propagated`；`verify_chat_test message_complete_rejects_inconsistent_tool_pairs`；`verify_chat_test static_message_completes_without_dummy_append`；`verify_chat_test empty_static_message_requires_content_before_complete`；`verify_chat_test pending_message_with_active_nested_block_cannot_complete` |
| B-010 | expected sequence initialization/advance | `verify_chat_lib_test components::chat::model::tests::gh62_ordered_update_contract`；`verify_chat_test sequence_is_conversation_wide_and_contiguous` |
| B-011 | exact replay lookup | `verify_chat_lib_test components::chat::model::tests::gh62_event_idempotency_contract`；`verify_chat_test exact_replay_returns_original_outcome_without_mutation` |
| B-012 | event ID conflict lookup | `verify_chat_test reused_event_id_with_different_content_conflicts` |
| B-013 | stale/gap/retention errors | `verify_chat_test stale_gap_and_retention_errors_do_not_advance_state` |
| B-014 | staged commit and all typed errors | `verify_chat_test every_failure_is_atomic_for_full_state`；`verify_chat_test duplicate_lifecycle_identities_are_rejected_atomically`；`verify_chat_test append_block_rejects_invalid_blocks_atomically`；`verify_chat_test replace_block_requires_same_variant_and_identity`；`verify_chat_test correlated_lifecycle_updates_are_atomic`；`verify_chat_test cancel_cascades_across_correlated_messages_atomically`；`verify_chat_test fail_cascades_across_correlated_messages_atomically`；`verify_chat_test message_complete_rejects_inconsistent_tool_pairs` |
| B-015 | current/restored-state bounded ledger and restart boundary | `verify_chat_lib_test components::chat::model::tests::gh62_replay_retention_boundary`；`verify_chat_test bounded_ledger_exposes_honest_replay_boundary`；`verify_chat_test fresh_restart_state_has_no_replay_or_eviction_evidence`；`verify_chat_test restore_snapshot_roundtrip_preserves_histories`；`verify_chat_lib_test components::chat::state::tests::restore_history_validation_is_exhaustive` |
| B-016 | deterministic fold / interleaving | `verify_chat_test identical_sequences_produce_identical_state_and_outcomes` |
| B-017 | two mock adapter conversions | `verify_chat_test distinct_mock_adapters_produce_equal_core_events` |
| B-018 | no wire/provider types or dependency drift in core | `verify_chat_test core_model_requires_adapter_owned_typed_values`；`verify_forbidden_dependency_alias_detection` 证明 non-code/safe-group 成功且 direct/grouped-root/nested-group renamed imports 失败；刷新 base 后运行 `verify_no_forbidden_chat_dependencies origin/main`，按 metadata identity 解析 Rust use-tree sources/extern/paths/macros |
| B-019 | old/new public surfaces、constructor compatibility、scoped docs 与 rustdoc | `verify_chat_test legacy_message_and_new_chat_surface_coexist`；`verify_chat_test constructor_based_public_api_remains_compatible`；`verify_chat_missing_docs_gate`；`verify_chat_rustdoc_example`；`cargo test --doc --workspace --all-features --locked` |
| B-020 | partial stream cancellation/failure and typed resend identity | `verify_chat_test cancellation_preserves_partial_content_and_rejects_late_events`；`verify_chat_test cancel_cascades_across_correlated_messages_atomically`；`verify_chat_test fail_cascades_across_correlated_messages_atomically`；`verify_chat_test resend_preserves_source_and_creates_fresh_identity` |
| B-021 | tool data-only boundary | `verify_chat_test tool_and_thinking_models_have_no_execution_surface`；上述两个 dependency helpers 对全部 planned chat files 执行 metadata-aware non-code-stripped lexical audit，并由 std process、terminal I/O、env macro、crate runtime adversarial fixtures 证明拒绝 process/env/fs/net/terminal/secret 与等价依赖 surface |
| B-022 | current-head exact-test、scoped docs 与 new-code coverage evidence | 运行本表全部 exact tests；所有 `verify_chat_test` / `verify_chat_lib_test` 均须以 `--include-ignored --exact` 实际得到 `1 passed; 0 failed; 0 ignored`；再运行 `cargo test --workspace --all-targets --all-features --locked`、绑定唯一 fence 的 `verify_chat_rustdoc_example`、metadata alias self-test、`verify_chat_missing_docs_gate` 和 `verify_chat_new_code_coverage`；保留 `target/specrail/GH62/coverage-<full-head-sha>/cobertura.xml` 作为当前 head artifact；table-driven matrices 必须枚举每个 reducer/nested/cross-level/call-result transition、identity namespace 和 counter exhaustion path |
| B-023 | Push/AppendText/block insert-replace/terminal cross-level validation | `verify_chat_lib_test components::chat::state::tests::append_block_cross_level_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::static_completion_readiness_matrix_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::tool_call_result_correlation_matrix_is_exhaustive`；`verify_chat_lib_test components::chat::state::tests::cross_level_terminality_never_freezes_active_nested_blocks`；`verify_chat_test append_block_supports_late_discovered_typed_blocks`；`verify_chat_test append_block_rejects_invalid_blocks_atomically`；`verify_chat_test replace_block_requires_same_variant_and_identity`；`verify_chat_test edit_and_insert_are_revisioned_and_identity_safe`；`verify_chat_test correlated_lifecycle_updates_are_atomic`；`verify_chat_test cancel_cascades_across_correlated_messages_atomically`；`verify_chat_test fail_cascades_across_correlated_messages_atomically`；`verify_chat_test message_complete_rejects_inconsistent_tool_pairs`；`verify_chat_test static_message_completes_without_dummy_append`；`verify_chat_test empty_static_message_requires_content_before_complete`；`verify_chat_test pending_message_with_active_nested_block_cannot_complete` |
| B-024 | checked sequence/revision advancement and deterministic precedence | `verify_chat_test sequence_exhaustion_is_checked_and_atomic_at_u64_max`；`verify_chat_test sequence_exhaustion_precedes_malformed_update_at_u64_max`；`verify_chat_test replay_conflict_stale_and_gap_precede_exhaustion`；`verify_chat_lib_test components::chat::state::tests::revision_exhaustion_is_checked_and_atomic_at_u64_max`；`verify_chat_test exact_replay_does_not_advance_exhausted_counters` |
| B-025 | nonzero MessageRevision、per-message checked increment、typed affected outcome | `verify_chat_lib_test components::chat::model::tests::gh62_revisioned_atomic_mutations`；`verify_chat_test message_revision_and_affected_outcome_are_typed`；`verify_chat_lib_test components::chat::state::tests::message_revision_checked_increment_is_exhaustive`；`verify_chat_test revision_guards_and_mutation_failures_are_atomic` |
| B-026 | BlockId/ThinkingId lifetime tombstones and correlation identity | `verify_chat_lib_test components::chat::state::tests::block_id_state_lifetime_rules_are_exhaustive`；`verify_chat_lib_test components::chat::state::tests::thinking_id_message_lifetime_rules_are_exhaustive`；`verify_chat_test block_ids_are_conversation_unique_and_retained`；`verify_chat_test edit_retires_thinking_ids_atomically`；`verify_chat_test every_block_variant_preserves_typed_data` |
| B-027 | edit/delete/resend and tombstone/correlation semantics | `verify_chat_test edit_and_insert_are_revisioned_and_identity_safe`；`verify_chat_test edit_retires_thinking_ids_atomically`；`verify_chat_test delete_preserves_global_correlation_atomically`；`verify_chat_test deleted_tool_result_retires_result_slot_atomically`；`verify_chat_lib_test components::chat::state::tests::tool_result_slot_history_rules_are_exhaustive`；`verify_chat_test resend_preserves_source_and_creates_fresh_identity` |
| B-028 | typed guards、atomic precedence、retention、serialization/API compatibility | `verify_chat_test revision_guards_and_mutation_failures_are_atomic`；`verify_chat_test mutation_replay_retention_is_consistent`；`verify_chat_test block_ids_are_conversation_unique_and_retained`；`verify_chat_test edit_retires_thinking_ids_atomically`；`verify_chat_test message_revision_and_affected_outcome_are_typed`；`verify_chat_test constructor_based_public_api_remains_compatible` |

## 数据流

### 输入

应用 adapter 提交 owned `ConversationEvent`。event 已含稳定 event ID、conversation-wide
sequence 和 typed `ConversationUpdate`；核心不接收 provider payload、反序列化 context、
network client 或执行 callback。

### 处理

reducer 先判 replay/conflict/retention，再 checked-compute sequence/conversation revision，
验证 expected revision guards 并 checked-compute affected message revisions，最后验证
blocks、nested/correlation 状态。成功时一次提交 staged state 并将 exact event 与 outcome
放入 ledger；失败时返回 `ConversationError`，全部 state 保持相等。

### 输出

调用方得到 typed `ApplyOutcome`（含 affected message revisions）或 `ConversationError`，
并可通过只读访问器读取消息、revisions、next expected sequence 和 retention boundary。
下游消费者只能提交 typed update 并读取 snapshot，不能绕过 reducer 修改内部 messages。

### 持久化与外部调用

无持久化、网络、provider、工具执行或终端输出。ledger 生命周期等于 `ConversationState`
实例；需要持久幂等时由应用在 adapter/state-store 边界实现。

## 备选方案

- 扩展现有 `display::Message`：拒绝。它是兼容展示组件，加入 reducer 会混合模型与视图并破坏
  简单 API。
- 直接复用 `serde_json::Value` 表达 blocks/tool args：拒绝。公共边界会变成未声明字段和
  provider wire schema。
- 让调用方拿到 `Vec<ChatMessage>` 可变引用：拒绝。可绕过 sequence、ledger 和状态机。
- 要求 Push 预声明全部未来 blocks：拒绝。provider 可在流中用 AppendMessageBlock/InsertMessageBlock。
- 用 ReplaceBlock 表达 block kind 变化：拒绝。它会混淆生命周期并允许
  ToolCall↔ToolResult；新 kind 必须使用全新 BlockId 插入。
- 无界 HashMap 保存 event：拒绝。长会话内存不可控，且仍无法提供跨进程保证。
- 只按 sequence 去重：拒绝。无法发现同 event ID 内容冲突，也无法返回原始 outcome。
- 失败后修复半成品 state：拒绝。reducer 必须 staged apply 后一次 commit。

## 风险

- Security：Tool Call 数据可能被误作执行授权。缓解：模型无 callback/process/network/secret
  handle，公共 rustdoc 明确 data-only。
- Compatibility：新类型名可能与现有 `MessageRole` 等冲突。缓解：保留旧 API，权威路径放在
  `components::chat`，integration test 同时编译新旧表面。
- Correctness：replay 顺序、nested/top-level 终态或 counter exhaustion 若处理错误，会重复
  delta、遗漏 message revision invalidation、复用 retired identity、留下 orphan result 或
  wrap/panic。缓解：stable/tombstoned identities、typed guards/affected outcome、全局 matrix、
  fixed precedence、checked increments、全状态原子性与 max-boundary tests。
- Performance：每次完整 clone conversation 会随历史增长。缓解：只 stage 目标 message、
  cross-message active counterpart 所在消息与 ledger entry；不得改成全历史 clone 常态。
- Maintenance：多套嵌套状态表容易漂移。缓解：纯 transition helpers 和笛卡尔积 table tests。
- Serialization：adapter 可能默认 revision/BlockId、截断数值或丢弃 typed-value kind。缓解：
  显式 fallible construction；missing/zero/negative/overflow revision、missing/negative/overflow
  BlockId 与 unknown value kind 均失败；restore 保留 BlockId 与 per-message ThinkingId history。

## 测试计划

- [ ] `verify_chat_test` 与 `verify_chat_lib_test` 使用 libtest
      `--exact --include-ignored`，对 Product-to-Test Mapping 的每个名字均精确匹配一个 test，
      并实际得到 `1 passed; 0 failed; 0 ignored`；T2 的 pure helpers 和 private revision
      boundary test 均使用完整 module-qualified libtest name。
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets --all-features --locked`
- [ ] `cargo test --workspace --all-targets --all-features --locked`
- [ ] `verify_chat_rustdoc_example`；它必须唯一解析
      `src/components/chat/mod.rs - components::chat (line N)`，并证明整个
      `components::chat` 过滤域只有该测试；从唯一 ordinary rust fence 抽取非注释代码并验证
      Push/AppendText/Complete 的顺序后，再以 `--include-ignored` 执行并得到
      `1 passed; 0 failed; 0 ignored`。fence 外 token 不是证据；也不得依赖 merged rustdoc
      harness 对完整展示名称的 `--exact` 行为，因为当前工具链会零匹配但 exit 0。
- [ ] `cargo test --doc --workspace --all-features --locked`
- [ ] `verify_chat_missing_docs_gate`；五个 planned chat source files 全部存在，chat root 恰有
      一个 `#![forbid(missing_docs)]`，且没有 `allow/expect(missing_docs)` 或
      `doc(hidden)` 降级；随后 full-scope `cargo check` 通过。
- [ ] 刷新 `origin/main` 后运行 `verify_no_forbidden_chat_dependencies origin/main`；零 forbidden
      package identity/alias matches 必须以 exit 0 通过，manifest/lockfile diff、匹配项、
      cargo metadata 或扫描错误必须非零失败；`verify_forbidden_dependency_alias_detection`
      必须证明 non-code/safe source 成功，renamed direct/grouped/nested、std process/terminal、
      env macro 与 crate runtime source 失败。
- [ ] Product-to-Test Mapping 中的 exact tests 覆盖 duplicate lifecycle identities、
      stable/retired BlockId 与 per-message ThinkingId、late-discovered append/insert、typed
      revision guards/affected outcome、edit/delete/resend、retention replay、empty/non-empty
      Complete、完整 ToolCall-status × ToolResult matrix，以及跨 message Cancel/Fail
      一次 conversation revision + 每 affected message 一次 revision 的原子传播。
- [ ] `verify_chat_new_code_coverage`；artifact 必须包含全部五个 planned chat source files、
      path 中的 full head SHA 必须等于当前 `git rev-parse HEAD`，line-rate 必须 ≥ 0.80。
- [ ] `cargo tree -e normal --prefix none` 与 base diff 证明未新增 provider/JSON/network direct dependency。
- [ ] 当前 implementation head 的 CI、独立 review、reviewThreads 和 SpecRail PR gate 全部通过。

## 回滚方案

该实现是新增 chat module 与 re-export，没有数据迁移。若需回滚，回滚实现 PR 即可恢复原公共
表面；现有 `Message` 未修改。任何下游若已开始依赖新 API，必须先回滚下游或保留临时兼容
wrapper，不能留下导出但静默禁用 reducer。ledger 数据只在内存中，回滚不宣称保留幂等历史。
