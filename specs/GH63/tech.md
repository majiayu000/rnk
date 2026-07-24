# Tech Spec：类型化消息与 AI 内容块视图

## Linked Issue

GH-63: https://github.com/majiayu000/rnk/issues/63

<!-- specrail-requires-planned-changes-v1 -->
<!-- specrail-planned-changes
{"version":1,"issue":63,"complete":true,"paths":["src/components/chat/view/mod.rs","src/components/chat/view/message.rs","src/components/chat/view/block.rs","src/components/chat/view/custom.rs","src/components/chat/view/cache.rs","src/components/chat/mod.rs","src/components/mod.rs","src/prelude.rs","docs/API_STABILITY.md","tests/chat_message_views.rs","tests/golden/chat_message_views.txt","tests/golden/chat_message_views.ansi.txt"],"spec_refs":["specs/GH63/product.md","specs/GH63/tech.md","specs/GH63/tasks.md","specs/GH62/product.md","specs/GH62/tech.md","specs/GH62/tasks.md"]}
-->

## Product Spec

见 [`product.md`](product.md)。

本 packet 只规划 GH-63。它基于 GH-62 typed conversation contract，并以外部
dependency/retarget gate 验证 GH-58 TextFlow；GH-58 files 不在当前 stacked head/base，
因此不伪造为本 manifest 的 `spec_refs`。GH-57 是 umbrella，GH-66/GH-67 是下游 shell，
均不属于本 implementation diff。

## Codebase Context

以下锚点已在 stacked spec 写作基线
`8eab00ea6bd8bc90ec38c00447f752149ba0efb7` 上通过 Read/grep 核实；这是 GH-62
PR #74 round 12 的 provisional-current contract，不是最终独立 review/merge 结论，
且仍没有 GH-62 生产实现。GH-62 final contract 若改变，GH-63 必须重新比较、同步并 review，
不能把本 SHA 写成永久 API 承诺。

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| legacy message | `src/components/display/message.rs:40`, `src/components/display/message.rs:162`, `src/components/display/message.rs:206` | `Message` 是 role/string/prefix；`ToolCall` 是 name/args；`ThinkingBlock` 是 content/max_lines | GH-63 必须新增 typed views，不修改或冒充这些兼容入口 |
| legacy exports | `src/components/display/mod.rs:53`, `src/components/mod.rs:23`, `src/prelude.rs:55` | legacy 四个类型经 components/prelude 导出 | 新 exports 需增量加入且由 compile test 防止遮蔽旧名称 |
| GH-62 planned model | `specs/GH62/product.md`, `specs/GH62/tech.md`, `specs/GH62/tasks.md` | provisional-current GH-62 规划 stable `MessageBlockEntry/BlockId`、11-variant `MessageBlock`、typed metadata/payload、message revision、affected-message outcome 与 Edit/Delete/Resend | view 只依赖 public read API；不得写 model/error/state/reducer files |
| Markdown | `src/components/display/markdown.rs:27`, `src/components/display/markdown.rs:77` | `Markdown::new(...).into_element()` 已提供 structured Element path | MarkdownBlockView 组合它，不复制 parser |
| layout primitives | `src/components/layout/box_component.rs:54`, `src/components/layout/box_component.rs:185`, `src/components/layout/box_component.rs:201` | `Box` 支持 column/row、width、padding、border 和 background | 三种 variant 用现有 primitives 组合 |
| theme tokens | `src/components/theme/tokens.rs:9`, `src/components/theme/tokens.rs:150`, `src/components/theme/tokens.rs:253` | 现有 closed component variants、design tokens 和 theme resolver | chat defaults 从当前 theme/tokens 派生，不新增平行全局 theme |
| test renderer | `src/testing/renderer.rs:16`, `src/testing/renderer.rs:43` | `TestRenderer` 支持固定宽高 plain/ANSI output | exact narrow/multiline snapshots 使用同一 renderer |
| golden harness | `src/testing/golden.rs:44`, `tests/golden_real_apps.rs:112` | plain/ANSI golden 位于 `tests/golden`，缺失时只允许显式 update | GH-63 新增 checked-in plain/ANSI fixtures，CI 禁止更新模式 |
| examples duplication | `examples/rnk_chat.rs:13`, `examples/rnk_chat.rs:187`, `examples/glm_chat.rs:238`, `examples/glm_chat.rs:247`, `examples/glm_chat.rs:270` | examples 各自定义消息和 AI block render functions | 证明 view 缺口；examples 迁移属于 GH-68，不在本 diff |
| API policy | `docs/API_STABILITY.md:7`, `docs/API_STABILITY.md:166` | prelude 是推荐 surface；新 API 要说明稳定级别、文档和 tests | 记录 legacy/typed migration 与 chat view 稳定性 |

## 设计方案

### 1. 模块和文件所有权

在 GH-62 的 `src/components/chat/` 下新增 `view/`：

- `view/mod.rs`：启用模块文档，定义并导出 `MessageViewVariant`、
  `ThinkingDisclosure`、`ChatMessageViewOptions`、typed block views 与 renderer contract。
- `view/message.rs`：`ChatMessageView<'a>`、role/status header、metadata、variant container、
  stable key 组合和 ordered block dispatch。
- `view/block.rs`：`TextBlockView`、`MarkdownBlockView`、`CodeBlockView`、
  `ThinkingBlockView`、`ToolCallBlockView`、`ToolResultBlockView`、`ErrorBlockView`、
  `DiffBlockView`、`QuoteBlockView`、`LinkBlockView`、
  `TerminalAttachmentSummaryBlockView`、`StreamingIndicatorView` 及 pure line-preview helper。
- `view/custom.rs`：闭集 `ChatBlockRef<'a>`、`ChatRenderContext<'a>`、
  `ChatRenderOverride`、`ChatBlockRenderer` trait 与 typed closure blanket impl。
- `view/cache.rs`：caller-owned `ChatMessageViewCache`、按
  `ApplyOutcome.affected_messages` public accessor 精确失效，以及 restore 后只从 live typed snapshot
  重建的 presentation-only cache bookkeeping。

`src/components/chat/mod.rs` 仅增加 `pub mod view` 和受控 re-export；它已有 GH-62 scoped
`#![forbid(missing_docs)]`，因此 view 及 child modules 不能降低 public-doc gate。
`src/components/mod.rs` 与 `src/prelude.rs` 只 re-export app-facing view types。
不修改 `model.rs`、`error.rs`、`state.rs`、`reducer.rs` 或 legacy
`display/message.rs`。

### 2. 公共 view contract

`ChatMessageView<'a>` 借用 `&'a ChatMessage`。全部字段私有，通过 builders 设置：

- `variant(MessageViewVariant::{Compact, Bordered, Bubble})`；
- `thinking_disclosure(BlockId, ThinkingDisclosure)`，内部使用有序 typed entries，
  不暴露动态 map；
- `thinking_preview_lines(NonZeroUsize)` 与
  `tool_result_preview_lines(NonZeroUsize)`；
- `indicator_frame(StreamingIndicatorFrame)`；
- `renderer(&dyn ChatBlockRenderer)`；
- `theme(Theme)`，用于显式提供一个 owned resolved snapshot；
- `style(ChatMessageViewStyle)`，只覆盖当前 view 的具名颜色/符号/spacing options。

author/timestamp 只从 `ChatMessage` 的 public metadata accessor 返回的 GH-62
`ChatMessageMetadata` 借用，并通过 `MessageAuthor::as_str()` /
`MessageTimestamp::as_str()` 投影；不存在
`MessageViewMetadata`、重复 builder、provider fallback 或 clone whole metadata。
`None` 不创建 header child。options 有确定默认值：Compact、Thinking collapsed、preview 5、
ToolResult preview 12、indicator frame 0；所有 line limits 使用 `NonZeroUsize`，不存在
silent clamp 或 zero 的歧义。`ChatMessageView::new` 在构造时恰好调用一次 `get_theme()`
并保存 owned snapshot；`.theme(...)` 替换该 snapshot。`into_element` 只读保存值，
不再次访问 ambient theme。

`into_element(self)` 执行：

```text
borrowed ChatMessage + explicit options + one resolved Theme snapshot
  -> resolve role/status semantics without ambient theme reread
  -> build message shell for selected variant
  -> enumerate MessageBlockEntry values in current source order
  -> derive stable key from conversation-lifetime BlockId (position is observational only)
  -> call typed custom renderer when present
  -> UseDefault: exhaustive default dispatch
     Element: wrap returned block with library-owned key/status boundary
  -> append message lifecycle indicator/terminal marker
  -> return one owned Element
```

`MessageId`、`MessageRevision`、`BlockId`、metadata 和 lifecycle/correlation identifiers
必须通过 GH-62 public accessors 读取；若 GH-62
最终没有足够的只读 accessor，GH-63 implementation 必须停止并回到 GH-62 spec/implementation，
不得使用 unsafe、debug-string parsing 或复制 private model。

### 3. Typed renderer contract

`ChatBlockRef<'a>` 是对 GH-62 十一种 closed variants 的借用投影，每个 variant 携带具体
typed reference；`ThinkingContent`、`ToolCallContent`、`ToolResultContent`、
`ErrorContent`、`DiffContent`、`QuoteContent`、`LinkContent`、
`TerminalAttachmentSummary`、`ToolArgument` 与递归 `TypedValue` 均只经 borrowed
accessors 读取，不把 block 擦除成 `dyn Any`，不 clone whole payload。
`ChatRenderContext<'a>` 至少含：message ID、当前 `MessageRevision`、role、message status、
`BlockId`、当前 position、variant、stable key 和 resolved `ChatMessageViewStyle`
read-only reference；position 不参与 identity。

```rust
pub trait ChatBlockRenderer {
    fn render(
        &self,
        block: ChatBlockRef<'_>,
        context: ChatRenderContext<'_>,
    ) -> ChatRenderOverride;
}

pub enum ChatRenderOverride {
    UseDefault,
    Element(Element),
}
```

为满足 typed closure，提供只接受同一签名的 blanket impl；不增加 string registry。
`ChatRenderOverride::UseDefault` 进入唯一 default dispatch；`Element` 只替换目标 block
body，library 仍添加 key/status shell。trait/closure panic 不 catch；吞掉 panic 再
默认渲染会制造假成功。

### 4. Block view 细节

- Text：`Text::new(content)`；不 trim、不填 placeholder。
- Markdown：`Markdown::new(content)`；不复制 parser。
- Code：结构化 header + `Text` body；language empty/None 不创建 header，content 不按 byte
  截断。
- Thinking：library wrapper 与 disclosure entry 都使用其 `MessageBlockEntry` public
  accessor 返回的 `BlockId` key，`ThinkingId` 仅作为 borrowed lifecycle 内容；
  Collapsed 把原始 `&str` 原样交给已合并的 GH-58
  exact-source ingress，通过 TextFlow logical hard-break row/source ranges 选择前 N 行，
  不调用 `str::lines()`、不重建 source、不自行切 byte。LF、CRLF、standalone CR、连续与
  尾随 hard break 的 terminator/source map 保持；只有 TextFlow 证明存在隐藏 logical row
  时添加 marker。Expanded 把完整 source 原样交给同一 ingress。若 GH-58 public surface
  无法提供该 projection，停止 GH-63 implementation 并回到 GH-58，不实现平行 tokenizer。
- ToolCall：name、ordered `ToolArgument` 和 typed status 分开构造；参数值只交给 `Text`，
  递归穷尽 closed `TypedValue`，不串成 JSON/命令、不读取未声明字段、不执行。
  Failed 同时显示 GH-62 nested typed cause。
- ToolResult：以与 Thinking 相同的 GH-58 exact-source row projection 显示 call ID、status、
  完整行预览和真实 truncation marker；Failed 同时显示 nested typed cause，Cancelled 使用
  cancel semantic style。
- Error：通过 `ErrorContent` 的 message 与 optional source public accessors 展示完整 typed payload；
  始终 error semantic，不 fallback、不丢 source。
- Diff：保留 optional language 与完整 diff content，只进入 structured Text/Box path。
- Quote：保留完整 content 与 optional attribution；缺失 attribution 不填值。
- Link：保留 label/target，以 inert structured content 展示，不导航、不发网络请求。
- TerminalAttachmentSummary：保留 name、optional media type 与 summary，不读取附件。
- StreamingIndicator：frame 是 closed、copy typed value；render 不读时间。Pending 与
  Streaming 使用不同 label/status，Complete 不调用该 view。

top-level `MessageStatus::Failed` 和 Thinking/ToolCall/ToolResult nested Failed variant
携带的 GH-62 typed reason 都属于默认输出必需内容；Error block 是独立内容，不能作为
reason 的替代品。reason 由结构化 `Text` 显示，不填 generic fallback。

所有 default styles 只从构造时捕获或显式提供的一个 `Theme` snapshot，经
`Theme::design_tokens()`、`Theme::semantic_color()` 或现有 typed resolver 计算为当前 view
局部值；不得 `set_theme`，`into_element` 不得 `get_theme`。变体只包裹 presentation，
不分叉 block semantics。golden 和 deterministic tests 显式传入 `Theme::dark()` /
`Theme::light()`；ambient-theme capture test 用 `with_theme` 并验证 scope 后恢复。

### 5. 身份、顺序和兼容

key 采用库内部明确分段的 canonical encoding：
`chat-block/<BlockId>`；message shell 可另用 `chat-message/<MessageId>`。字符串内容、status、
revision、position、variant、metadata、indicator frame、`ThinkingId` 与 `ToolCallId` 均不进入
block key。当前 position 只决定 source order；同一 `BlockId` 经 append/replace/edit 后仍指向
同一 wrapper，插入或重排 sibling 不换 key。`ThinkingId` 是 message-local lifecycle identity，
`ToolCallId` 是 conversation-wide call/result correlation identity，均不能替代
conversation-state-lifetime `BlockId`。

### 6. Revision、changefeed 与 caller-owned cache

`ChatMessageView` 本身保持纯且无共享可变状态。可选 `ChatMessageViewCache` 是显式、
caller-owned presentation helper；它不写 `ConversationState`，也不序列化 `Element` 到
GH-62 snapshot。cache 至少按 `(MessageId, MessageRevision)` 识别 message projection，并按
`BlockId` 管理 block body/preview/disclosure；key identity 与 cache value freshness 分开。

成功 apply 后调用方把同一 `ApplyOutcome` 与新 immutable state snapshot 交给 cache：

- 遍历 affected-messages accessor 的确定顺序；`Present` 校验 snapshot 中 message revision
  等于 affected entry 的 applied revision accessor，只逐出该 message 的旧 revision
  projection，并比较 live BlockId 集合：
  保留 ID 的 wrapper identity 不变，但 payload、status、metadata、theme/options 或 revision
  改变时重算 value；移除 ID 的 body/preview/disclosure 全部逐出；新增 ID 不得命中旧 entry。
- `Deleted` 要求 snapshot 中不再存在 message，逐出该 MessageId 的 shell 与所有已知
  BlockId/preview/disclosure。GH-62 state 保有 tombstone；view 不自行释放或复用 ID。
- outcome 未列出的 message 不失效。尤其 Resend source 保持逐值/revision/cache 不变，
  新 message 的 previous revision accessor 返回 `None`、applied revision 为
  `MessageRevision::INITIAL == 1`，并使用 fresh MessageId/BlockIds。
- exact replay 返回相同 outcome 时 cache update 幂等；不能全局 flush、重复推进 revision
  或根据 conversation revision 猜 affected set。

Edit/Insert/Append/Replace 的展示顺序来自新 `MessageBlockEntry` list；删除或 edit 移除的
BlockId/ThinkingId/ToolCall result-slot history 由 GH-62 state 保持。processed-event ledger
eviction 不释放任何 cache identity。显式 restore 后丢弃旧 presentation values，只从 restored
live `MessageBlockEntry`、message revision 和已由 GH-62 验证的 tombstone/history snapshot
重建；若 restore 缺失/矛盾，GH-62 必须先拒绝，GH-63 不从旧 position/Element 猜补。

legacy `Message`/`ToolCall`/`ThinkingBlock` 源文件和 exports 保持。`docs/API_STABILITY.md`
增加迁移段：简单字符串继续用 legacy；需要 typed/lifecycle 时先构造 GH-62 model，再
借用给 view。compile test 同时导入 legacy 与新 surface，防止名称遮蔽。

GH-62 的 chat module 只允许一个普通 rust doctest 的既有 gate；GH-63 不新增普通 rustdoc
fence，view 示例使用 `text`/`ignore`，可执行迁移路径由 integration exact test 覆盖，
避免破坏 GH-62 的唯一 doctest 证据。

### 7. 外部依赖与 retarget gate

本 packet 写作时只消费 GH-62 base
`8eab00ea6bd8bc90ec38c00447f752149ba0efb7` 的 provisional-current contract；
PR #74 round 12 尚未完成最终独立 review。T1 开始前必须确认 GH-62 implementation 已合并，
并把 final merged spec/API 与本 packet 逐项比较；若 `MessageBlockEntry/BlockId`、
11 variants、typed metadata/payload accessors、`MessageRevision::INITIAL`、
`ApplyOutcome::affected_messages`、Edit/Delete/Resend 或 tombstone/restore contract 改变，
先同步并重新 review GH-63，不能沿用旧通过结论。
T2 的 preview source ingress 以及后续任务必须等待 GH-58 implementation。由于当前 stacked
spec head 不含 GH-58 packet/files，依赖不通过 `spec_refs` 冒充：implementation coordinator
必须刷新 `origin/main`、保存 issue #58/#62 的 fresh closed evidence，把 implementation
branch retarget/rebase/merge 到包含两项 merged implementation 的 main（禁止 force push），
并运行 `verify_gh63_upstream_gate origin/main`。

该 gate 必须证明传入 ref 是当前 `origin/main`、是 implementation HEAD 的 ancestor，且
ref 中实际存在 `specs/GH58/{product,tech,tasks}.md`、GH-58 TextFlow source/public tests、
`specs/GH62/{product,tech,tasks}.md` 与 GH-62 chat model source；任一 issue 未关闭、路径
缺失、ref stale、不是 ancestor 或 GitHub 查询失败均非零退出。spec-only PR #75 不运行
此 gate；它只声明未来 implementation 的可验证外部前置。

所有 view source 禁止写 ESC/CSI/OSC 序列或调用 terminal encoder。Unicode/control
sanitization、exact-source hard breaks、visual-cell wrap 和 clip parity 由 retargeted
GH-58 TextFlow 提供。GH-63 只使用当前 dependencies；`Cargo.toml`/`Cargo.lock` 不在
planned paths。

## Exact Verification Helpers

所有 filtered test 必须经以下 helper，先用 libtest `--list --exact` 证明恰好一个匹配，
再用 `--include-ignored --exact` 执行并解析为 one passed、zero failed、zero ignored：

```sh
assert_gh63_exact_summary() {
  summary="$1"
  printf '%s\n' "$summary" |
    grep -Eq 'test result: ok\. 1 passed; 0 failed; 0 ignored;' || {
      printf '%s\n' "$summary" >&2
      return 1
    }
}

verify_gh63_exact_helper_self_test() {
  assert_gh63_exact_summary \
    'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out'
  if assert_gh63_exact_summary \
    'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out' \
    >/dev/null 2>&1; then
    printf 'exact summary parser accepted two passed tests\n' >&2
    return 1
  fi
}

verify_gh63_exact() {
  test_name="$1"
  listed="$(
    cargo test --test chat_message_views --locked "$test_name" -- --list --exact |
      awk '/: test$/{count++} END{print count+0}'
  )"
  test "$listed" -eq 1 || {
    printf 'expected one GH63 test, matched %s: %s\n' "$listed" "$test_name" >&2
    return 1
  }
  output="$(
    cargo test --test chat_message_views --locked "$test_name" \
      -- --include-ignored --exact 2>&1
  )" || {
    printf '%s\n' "$output" >&2
    return 1
  }
  assert_gh63_exact_summary "$output"
}

verify_gh63_upstream_gate() {
  base_ref="${1:-origin/main}"
  git fetch origin main || return 1
  base_sha="$(git rev-parse "$base_ref")" || return 1
  test "$base_sha" = "$(git rev-parse origin/main)" || {
    printf 'GH63 dependency ref is stale: %s != origin/main\n' "$base_sha" >&2
    return 1
  }
  git merge-base --is-ancestor "$base_sha" HEAD || {
    printf 'GH63 implementation head must include %s\n' "$base_sha" >&2
    return 1
  }

  for issue_number in 58 62; do
    issue_state="$(
      gh issue view "$issue_number" --repo majiayu000/rnk \
        --json state --jq '.state'
    )" || return 1
    test "$issue_state" = "CLOSED" || {
      printf 'required issue #%s is not closed\n' "$issue_number" >&2
      return 1
    }
    closing_prs="$(
      gh issue view "$issue_number" --repo majiayu000/rnk \
        --json closedByPullRequestsReferences \
        --jq '.closedByPullRequestsReferences[].number'
    )" || return 1
    test -n "$closing_prs" || {
      printf 'required issue #%s has no closing PR evidence\n' "$issue_number" >&2
      return 1
    }
    printf '%s\n' "$closing_prs" |
      while IFS= read -r pr_number; do
        test -n "$pr_number" || exit 1
        pr_state="$(
          gh pr view "$pr_number" --repo majiayu000/rnk --json state --jq '.state'
        )" || exit 1
        merge_sha="$(
          gh pr view "$pr_number" --repo majiayu000/rnk \
            --json mergeCommit --jq '.mergeCommit.oid'
        )" || exit 1
        test "$pr_state" = "MERGED" && test -n "$merge_sha" || exit 1
        git merge-base --is-ancestor "$merge_sha" "$base_sha" || {
          printf 'closing PR #%s merge %s is absent from %s\n' \
            "$pr_number" "$merge_sha" "$base_sha" >&2
          exit 1
        }
      done || return 1
  done

  for required_path in \
    specs/GH58/product.md \
    specs/GH58/tech.md \
    specs/GH58/tasks.md \
    src/layout/text_flow.rs \
    tests/text_flow_parity.rs \
    specs/GH62/product.md \
    specs/GH62/tech.md \
    specs/GH62/tasks.md \
    src/components/chat/model.rs \
    src/components/chat/state.rs \
    src/components/chat/reducer.rs \
    tests/chat_conversation_contracts.rs
  do
    git cat-file -e "$base_sha:$required_path" || {
      printf 'upstream dependency path missing at %s: %s\n' \
        "$base_sha" "$required_path" >&2
      return 1
    }
  done
}

verify_gh63_docs_gate() {
  command -v verify_chat_rustdoc_example >/dev/null 2>&1 || {
    printf 'define the merged GH62 verify_chat_rustdoc_example helper first\n' >&2
    return 1
  }
  verify_chat_rustdoc_example || return 1
  python3 <<'PY'
from pathlib import Path
import re

paths = [
    Path("src/components/chat/mod.rs"),
    Path("src/components/chat/model.rs"),
    Path("src/components/chat/error.rs"),
    Path("src/components/chat/state.rs"),
    Path("src/components/chat/reducer.rs"),
    Path("src/components/chat/view/mod.rs"),
    Path("src/components/chat/view/message.rs"),
    Path("src/components/chat/view/block.rs"),
    Path("src/components/chat/view/custom.rs"),
    Path("src/components/chat/view/cache.rs"),
]
missing = [str(path) for path in paths if not path.is_file()]
if missing:
    raise SystemExit(f"missing planned chat/view sources: {missing}")

root_lines = paths[0].read_text(encoding="utf-8").splitlines()
guard_count = sum(line == "#![forbid(missing_docs)]" for line in root_lines)
if guard_count != 1:
    raise SystemExit(
        f"expected one chat-root forbid(missing_docs), found {guard_count}"
    )

lowering = re.compile(r"(?:allow|expect)\([^)]*missing_docs[^)]*\)|doc\(hidden\)")
findings = []
ordinary_view_fences = []
for path in paths:
    source = path.read_text(encoding="utf-8")
    compact = re.sub(r"\s+", "", source)
    for match in lowering.finditer(compact):
        findings.append(f"{path}: forbidden docs downgrade {match.group(0)}")
    if "/view/" in path.as_posix():
        ordinary_view_fences.extend(
            f"{path}:{line_number}"
            for line_number, line in enumerate(source.splitlines(), start=1)
            if line == "//! ```rust"
        )
if ordinary_view_fences:
    findings.append(f"GH63 added ordinary chat doctest fences: {ordinary_view_fences}")
if findings:
    raise SystemExit("\n".join(findings))
PY
  cargo check --workspace --all-targets --all-features --locked
}

verify_gh63_branch_matrices() {
  verify_gh63_exact every_block_variant_dispatches_once_in_order &&
  verify_gh63_exact every_message_and_nested_failed_status_preserves_typed_reason &&
  verify_gh63_exact typed_trait_and_closure_override_or_explicitly_default &&
  verify_gh63_exact preview_source_ranges_cover_every_hard_break_and_truncation_branch &&
  verify_gh63_exact affected_messages_drive_exact_cache_invalidation
}

verify_gh63_new_code_coverage() {
  required_sources="
src/components/chat/view/mod.rs
src/components/chat/view/message.rs
src/components/chat/view/block.rs
src/components/chat/view/custom.rs
src/components/chat/view/cache.rs
"
  printf '%s\n' "$required_sources" |
    while IFS= read -r source_file; do
      test -z "$source_file" || test -f "$source_file" || exit 1
    done || {
      printf 'one or more planned GH63 view sources are missing\n' >&2
      return 1
    }
  verify_gh63_branch_matrices || return 1

  head_sha="$(git rev-parse HEAD)" || return 1
  coverage_dir="target/specrail/GH63/coverage-$head_sha"
  coverage_report="$coverage_dir/cobertura.xml"
  mkdir -p "$coverage_dir" || return 1
  rm -f -- "$coverage_report" "$coverage_dir/head.sha"
  cargo tarpaulin --workspace --all-features --locked \
    --include-files 'src/components/chat/view/*.rs' \
    --out Xml --output-dir "$coverage_dir" --fail-under 80 || return 1
  test -s "$coverage_report" || return 1
  printf '%s\n' "$head_sha" >"$coverage_dir/head.sha" || return 1

  python3 - "$coverage_report" "$head_sha" "$coverage_dir/head.sha" <<'PY'
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

report_path = Path(sys.argv[1])
head_sha = sys.argv[2]
head_path = Path(sys.argv[3])
if head_path.read_text(encoding="utf-8").strip() != head_sha:
    raise SystemExit("coverage head artifact does not match current full SHA")
if report_path.parent.name != f"coverage-{head_sha}":
    raise SystemExit("coverage directory is not bound to the current full SHA")

root = ET.parse(report_path).getroot()
line_rate = float(root.attrib["line-rate"])
required = {
    "src/components/chat/view/mod.rs",
    "src/components/chat/view/message.rs",
    "src/components/chat/view/block.rs",
    "src/components/chat/view/custom.rs",
    "src/components/chat/view/cache.rs",
}
reported = {
    node.attrib["filename"].replace("\\", "/")
    for node in root.findall(".//class")
}
normalized = {
    required_path
    for required_path in required
    if any(
        name == required_path or name.endswith(f"/{required_path}")
        for name in reported
    )
}
missing = sorted(required - normalized)
if missing:
    raise SystemExit(f"coverage omitted planned GH63 sources: {missing}")
if line_rate < 0.80:
    raise SystemExit(f"GH63 aggregate line rate {line_rate:.4f} is below 0.80")
print(f"GH63 aggregate line rate: {line_rate:.4f}; head: {head_sha}")
PY
}
```

`verify_chat_rustdoc_example` 复用 GH-62 tech 中已执行 `--list`、唯一解析完整 rustdoc
名称并以 `--exact --include-ignored` 得到 one passed/zero ignored 的 helper；GH-63 wrapper
在它缺失时 fail closed，并额外审计全部十个 chat/view sources。实现/验证 shell必须先定义
该 GH-62 helper，再定义本节 helpers。`verify_gh63_exact_helper_self_test` 必须先实际通过，
证明正常 Cargo `ok.` summary 被接受且 two-passed negative 被拒绝。

Golden tests additionally unset `UPDATE_GOLDEN` and assert both checked-in paths are unchanged
before/after execution; missing fixture、environment update mode 或 write 均失败。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 | `view/message.rs` immutable dispatch | `verify_gh63_exact message_view_is_pure_and_preserves_block_order` |
| B-002 | role header + borrowed `ChatMessageMetadata` | `verify_gh63_exact roles_and_missing_metadata_are_explicit`；`verify_gh63_exact typed_metadata_is_borrowed_without_placeholders` |
| B-003 | message status shell/indicator + typed failure reason | `verify_gh63_exact every_message_status_has_distinct_semantics`；`verify_gh63_exact every_message_and_nested_failed_status_preserves_typed_reason` |
| B-004 | exhaustive eleven-variant `MessageBlock` dispatch | `verify_gh63_exact every_block_variant_dispatches_once_in_order`；`verify_gh63_exact all_extended_block_variants_render_typed_payloads` |
| B-005 | `TextBlockView` | `verify_gh63_exact text_view_preserves_empty_multiline_and_unicode_content` |
| B-006 | `MarkdownBlockView` | `verify_gh63_exact markdown_view_uses_structured_component_without_fallback` |
| B-007 | `CodeBlockView` | `verify_gh63_exact code_view_preserves_language_absence_and_multiline_content` |
| B-008 | `ThinkingBlockView` + disclosure + TextFlow range projection | `verify_gh63_exact thinking_disclosure_is_controlled_identity_stable_and_exact`；`verify_gh63_exact preview_source_ranges_cover_every_hard_break_and_truncation_branch`；`verify_gh63_exact every_nested_failed_status_preserves_typed_reason`；`verify_gh63_exact every_message_and_nested_failed_status_preserves_typed_reason` |
| B-009 | `ToolCallBlockView` | `verify_gh63_exact tool_call_status_matrix_and_argument_order_are_typed`；`verify_gh63_exact typed_value_tool_arguments_render_without_json`；`verify_gh63_exact every_nested_failed_status_preserves_typed_reason`；`verify_gh63_exact every_message_and_nested_failed_status_preserves_typed_reason` |
| B-010 | `ToolResultBlockView` + TextFlow range projection | `verify_gh63_exact tool_result_status_and_true_truncation_are_explicit`；`verify_gh63_exact preview_source_ranges_cover_every_hard_break_and_truncation_branch`；`verify_gh63_exact every_nested_failed_status_preserves_typed_reason`；`verify_gh63_exact every_message_and_nested_failed_status_preserves_typed_reason` |
| B-011 | `ErrorBlockView` borrowed `ErrorContent` | `verify_gh63_exact error_block_never_degrades_to_normal_text`；`verify_gh63_exact error_content_message_and_source_are_projected` |
| B-012 | `StreamingIndicatorView` | `verify_gh63_exact streaming_indicator_is_frame_controlled_and_deterministic` |
| B-013 | three message containers | `verify_gh63_exact variants_change_presentation_not_semantics` |
| B-014 | custom trait/closure + default dispatch | `verify_gh63_exact typed_trait_and_closure_override_or_explicitly_default`；`verify_gh63_exact typed_renderer_contract_contains_no_dynamic_erasure` |
| B-015 | revision/BlockId/position context + library-owned wrapper | `verify_gh63_exact custom_renderer_receives_typed_context_without_reordering` |
| B-016 | BlockId-only key builder | `verify_gh63_exact keys_survive_content_status_and_disclosure_updates`；`verify_gh63_exact block_id_not_position_or_lifecycle_identity_keys_views` |
| B-017 | retargeted GH-58 exact-source TextFlow | `verify_gh63_upstream_gate origin/main`；`verify_gh63_exact preview_source_ranges_cover_every_hard_break_and_truncation_branch`（Thinking/ToolResult × LF/CRLF/CR/consecutive/trailing）；`verify_gh63_exact narrow_unicode_and_control_fixtures_use_textflow_safely` |
| B-018 | legacy exports + migration | `verify_gh63_exact legacy_and_typed_message_surfaces_coexist`；复核 `docs/API_STABILITY.md` |
| B-019 | one captured/explicit theme snapshot + local style | `verify_gh63_exact theme_snapshot_is_captured_once_and_explicitly_deterministic`；`verify_gh63_exact theme_and_style_overrides_are_local_to_one_view` |
| B-020 | deterministic repeat including theme snapshot | `verify_gh63_exact identical_inputs_render_identically`；`verify_gh63_exact theme_snapshot_is_captured_once_and_explicitly_deterministic`；`verify_gh63_exact theme_scope_restores_after_dark_and_light_snapshots` |
| B-021 | retry/panic boundary | `verify_gh63_exact interrupted_retry_has_no_library_side_effect`；`verify_gh63_exact custom_renderer_panic_is_not_silently_swallowed` |
| B-022 | snapshot-only concurrency boundary | `verify_gh63_exact independent_snapshots_do_not_share_view_state` |
| B-023 | exact current-head tests/golden/docs/coverage | `verify_gh63_exact_helper_self_test`；本表全部 exact tests；`verify_gh63_exact plain_and_ansi_golden_cover_full_matrix`；`verify_gh63_docs_gate`；`verify_gh63_branch_matrices`；`verify_gh63_new_code_coverage`；fresh full suite、CI/review/gate |
| B-024 | no dependency/model/reducer drift | `git diff --exit-code <implementation-base> -- Cargo.toml Cargo.lock src/components/chat/model.rs src/components/chat/error.rs src/components/chat/state.rs src/components/chat/reducer.rs`；planned-path audit |
| B-025 | `MessageBlockEntry/BlockId` identity namespaces | `verify_gh63_exact block_id_not_position_or_lifecycle_identity_keys_views`；`verify_gh63_exact edit_insert_reorder_preserve_or_retire_block_identity` |
| B-026 | borrowed typed metadata/payload projections | `verify_gh63_exact typed_metadata_is_borrowed_without_placeholders`；`verify_gh63_exact error_content_message_and_source_are_projected`；`verify_gh63_exact typed_value_tool_arguments_render_without_json` |
| B-027 | Diff/Quote/Link/TerminalAttachmentSummary views | `verify_gh63_exact all_extended_block_variants_render_typed_payloads` |
| B-028 | message revision + affected-message changefeed | `verify_gh63_exact message_revision_initial_and_checked_updates_are_observed`；`verify_gh63_exact affected_messages_drive_exact_cache_invalidation` |
| B-029 | exact caller-owned cache invalidation | `verify_gh63_exact affected_messages_drive_exact_cache_invalidation`；`verify_gh63_exact delete_evicts_view_cache_and_preserves_tombstones`；`verify_gh63_exact resend_keeps_source_cache_and_starts_fresh_message_revision` |
| B-030 | edit/delete/resend presentation behavior | `verify_gh63_exact edit_insert_reorder_preserve_or_retire_block_identity`；`verify_gh63_exact delete_evicts_view_cache_and_preserves_tombstones`；`verify_gh63_exact resend_keeps_source_cache_and_starts_fresh_message_revision` |
| B-031 | restore/ledger/tombstone boundary | `verify_gh63_exact restore_rebuilds_cache_without_resurrecting_retired_ids`；`verify_gh63_exact delete_evicts_view_cache_and_preserves_tombstones` |

## 数据流

### 输入

- GH-62 `&ChatMessage` 与 closed block/lifecycle types。
- GH-62 message 自带的 typed metadata，以及 variant、preview limits、indicator frame、
  style options。
- 可选 `&dyn ChatBlockRenderer`。
- 可选 caller-owned `ChatMessageViewCache` 与 reducer `ApplyOutcome` changefeed。
- 构造时恰好捕获一次、或调用方显式传入的 owned Theme snapshot。
- retargeted GH-58 TextFlow exact-source ingress/projection。

### 处理

1. 从保存的 Theme snapshot 解析局部 style 和 role/status semantics，不再读 ambient theme。
2. Thinking/ToolResult 把完整 raw source 交给 TextFlow，并从保留 terminator/source map 的
   logical projection 选择 preview；其他 content 同样只走 structured element ingress。
3. 按当前 source position 穷尽遍历 `MessageBlockEntry`，只由 `BlockId` 构造 stable key，
   并把 position、message revision 和 borrowed typed payload 放入 context。
4. 调用 custom renderer；`UseDefault` 进入唯一 default renderer，`Element` 进入相同
   library-owned keyed/status wrapper。
5. 对 top-level/nested Failed 显示 typed reason，再构造 selected variant shell、
   metadata 和 message status indicator。
6. 返回完整 owned `Element`；无外部写、无 conversation mutation。若调用方启用 cache，
   只按 `affected_messages` 的 Present/Deleted 项更新 presentation entries。

### 输出

- 一个按输入顺序组合的 `Element` tree。
- snapshot/golden only evidence；不写 terminal、conversation 或 provider state。

### 持久化与外部调用

无 conversation/provider 持久化。theme 只读，工具不执行，renderer 不发网络请求；
可选 cache 由调用方持有且不替代 GH-62 revision、ledger 或 tombstone state。

## 备选方案

- 把 legacy `Message` 扩展成 typed mega-component：拒绝；会破坏 simple API 并把 GH-62
  model 与展示耦合。
- 接受 `serde_json::Value` 或 `HashMap<String, renderer>`：拒绝；失去 closed typed
  exhaustiveness，违反 issue contract。
- custom renderer 返回 `Option<Element>`：拒绝；`None` 是忽略还是 default 不明确，
  容易静默丢 block。
- view 内部持有 Thinking toggle state：拒绝；跨 shell 生命周期和焦点属于 GH-66/GH-67，
  GH-63 采用 controlled presentation state。
- 复制 Markdown parser 或实现 syntax highlighter：拒绝；超出 GH-63 且引入重复行为。

## 风险

- Security: tool arguments/results 和 Markdown 可能含 control payload；view 只走 structured
  elements、无 raw ANSI、无执行边界，最终 sanitization 由 GH-58 验证。
- Compatibility: 新类型名可能遮蔽 legacy `Message`；独立 compile test 同时导入两组 API，
  legacy 文件不在 planned paths。
- Performance: 无 cache 时每次 render 线性遍历 blocks/visible lines；preview helper 不复制
  被隐藏尾部。可选 cache 只精确失效 affected messages，不引入 global cache 或全量 flush；
  极大消息的列表虚拟化属于 GH-65。
- Maintenance: GH-62 public accessor 或 enum 若改变，exhaustive compile 会阻断；不得用
  wildcard match 掩盖 drift。
- Evidence: golden update env 会隐藏回归；CI 明确拒绝 `UPDATE_GOLDEN` 并检查 fixture diff。

## 测试计划

- [ ] Unit/integration exact：所有 role、message/nested status 与 typed failure reason、
      十一种 block、variant、override/default、typed metadata absence、
      `ErrorContent`/`TypedValue` borrowed projection、empty/multiline/Unicode、
      LF/CRLF/CR/consecutive/trailing hard breaks、BlockId keys、revision/changefeed、
      Edit/Delete/Resend/cache/tombstone/restore、captured/explicit theme 和 compatibility。
- [ ] Golden：同一 full matrix 的 checked-in plain/ANSI 输出；CI 中禁止 update。
- [ ] Dependency gates：T1 前运行 GH-62 completion evidence；T2 前 retarget 到包含
      GH-58/GH-62 merged implementations 的 current `origin/main`，并使
      `verify_gh63_upstream_gate origin/main` 通过。
- [ ] Docs：先定义并执行 GH-62 exact-one `verify_chat_rustdoc_example`，再运行
      `verify_gh63_docs_gate`；唯一普通 chat doctest 必须 one passed/zero ignored，
      全部十个 chat/view files 无 missing-docs/doc-hidden downgrade。
- [ ] Coverage：`verify_gh63_new_code_coverage` 重新生成只统计
      `src/components/chat/view/*.rs` 的 current-head artifact，合计 line-rate >=80%，
      并运行 dispatch/status-reason/override/TextFlow-preview/changefeed-cache 五个 exhaustive
      matrices。
- [ ] Full verification：fmt、check、clippy、all-target tests、doc tests、diff/planned paths、
      fresh CI、独立 review、reviewThreads、SpecRail PR gate。

## 回滚方案

GH-63 是增量 view layer。若发布前失败，回滚 `src/components/chat/view/`（包括 caller-owned
cache）、三处 exports、
API 文档、integration test 与两份 golden；GH-62 model/reducer 和 legacy message 不受影响。
若仅 custom renderer 合同有缺陷，在合并前回退整个 GH-63 implementation 并修订 spec，
不得静默改成 `Any`/string registry。若 GH-58 或 GH-62 dependency 尚未完成，则保持
implementation PR 未合并，不能以禁用窄宽测试或 fallback 旧 string component 作为发布。
