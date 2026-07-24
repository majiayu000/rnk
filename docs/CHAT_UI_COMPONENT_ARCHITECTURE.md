# Terminal AI Chat UI Component Architecture

## 1. Document Status

This document records the chat UI component design, maturity criteria, and
layout-engine audit discussed for `rnk`. It is an architecture and delivery
reference, not an implementation claim.

Audit baseline:

- Repository: `majiayu000/rnk`
- Latest inspected `origin/main`: `a7c05a6`
- Inspection date: 2026-07-23
- SpecRail tracking issue: `GH-57` with child issues `GH-58` through `GH-68`
- Target environment: terminal-first Rust applications
- Target product shape: a backend-independent AI chat component library with
  an experience comparable in scope to an Element-style AI component suite

All types and APIs marked **proposed** do not exist yet. Existing public APIs
are identified separately.

## 2. Executive Conclusion

`rnk` already contains much of the general terminal UI foundation needed for
AI chat applications, but it does not yet provide a coherent, product-grade
chat component system.

The correct goal is not a single large `Chat` widget. The correct goal is a
layered component family with:

- backend-independent conversation data
- deterministic state and input contracts
- reusable message content blocks
- a multiline chat composer
- streaming and tool-call presentation
- separate inline and fullscreen shells
- correct variable-height scrolling
- consistent Unicode measurement and rendering
- stable theming, testing, and extension boundaries

Completing those features would make the library product-capable. It would not
make it "perfect." A mature UI library is defined by stable contracts,
correctness, compatibility, performance, documentation, and upgrade safety as
well as by its visible feature list.

Current qualitative assessment:

| Area | Approximate maturity | Notes |
| --- | ---: | --- |
| General TUI foundation | 70% | Components, hooks, Flexbox, themes, rendering modes, and input primitives exist. |
| Chat-specific product capability | 30-40% | Examples demonstrate the experience, but behavior is duplicated and not packaged. |
| After the proposed component extraction | 75-85% | Reusable chat flows would exist, but production hardening would still remain. |
| Product-grade target | 100% of documented gates | Requires correctness, performance, compatibility, tests, and API stability. |

These percentages are planning estimates, not measured quality scores.

## 3. Product Definition

The target is:

> A backend-independent terminal AI UI component library supporting inline and
> fullscreen chat, streaming messages, tool calls, rich content, robust input,
> correct scrolling, and stable behavior across supported terminals, without
> requiring application authors to implement cursor movement, Unicode wrapping,
> message-height calculation, streaming delta assembly, or scroll anchoring.

### 3.1 Design principles

1. Keep UI independent of model providers and network clients.
2. Prefer explicit state plus pure handlers for testable interaction behavior.
3. Keep visual components composable instead of creating one monolithic chat
   component.
4. Treat inline and fullscreen rendering as distinct shells with shared
   primitives.
5. Use the same text-flow result for both measurement and rendering.
6. Make state transitions explicit; do not silently degrade after failed layout
   or rendering operations.
7. Preserve existing public APIs through wrappers or staged deprecation.
8. Support custom renderers through typed contracts rather than untyped values.

### 3.2 Non-goals for the UI package

The chat component layer should not own:

- model-provider authentication
- HTTP or WebSocket clients
- API keys or secret storage
- model-specific request schemas
- tool execution or shell commands
- conversation database persistence
- retry policy for external services
- provider-specific token accounting

Applications or adapter crates may translate provider events into the typed UI
updates defined by the chat component layer.

## 4. Existing Foundation That Should Be Reused

Search-first review shows that substantial primitives already exist. They
should be extended or composed rather than re-created.

| Existing capability | Current location or API | Proposed chat use |
| --- | --- | --- |
| Layout containers | `Box`, `ScrollableBox`, `fixed_bottom_layout` | Shells, message rows, composer region |
| Scrolling | `Viewport`, `ViewportState`, `use_scroll` | Transcript navigation and scroll state |
| Text input | `TextInputState`, `handle_text_input` | Single-line fields and command surfaces |
| Multiline editing | `TextArea`, `TextAreaState`, textarea handlers | Foundation for `ChatComposer` |
| Chat display | `Message`, `MessageRole` | Compatibility wrapper or simple-message preset |
| AI content blocks | `ToolCall`, `ThinkingBlock` | Starting point for structured block views |
| Rich content | `Markdown`, `Text`, `Span`, `CodeEditor` | Message body renderers |
| Feedback | `Spinner`, `StatusBar`, `Alert`, `Notification` | Streaming, connection, and error states |
| Help and shortcuts | `KeyHint`, `Help`, keymaps | Composer and transcript commands |
| Empty states | `EmptyState` | New conversation and no-results states |
| Persistent inline output | `AppContext::println`, static output | Completed transcript in native scrollback |
| Render modes | Inline and fullscreen app builders | Two chat shells |
| Styling | `Theme`, `DesignTokens`, variants | Chat visual presets and semantic states |
| Interaction contracts | `InteractionMode`, `InteractionOutcome<T>` | Consistent controlled component behavior |

### 4.1 Current duplication that motivates extraction

- `examples/chat.rs` manually manages input editing and a vector of strings.
- `examples/rnk_chat.rs` defines its own message model, roles, scrolling,
  rendering, input area, and footer.
- `examples/claude_input_box.rs` contains roughly 595 lines and implements a
  separate cursor, wrapping, viewport, submission, and inline transcript flow.
- `examples/glm_chat/prompt_box.rs` implements another prompt box plus direct
  ANSI cursor positioning.
- `Message` accepts only a string plus a fixed role presentation. It cannot
  represent a message containing multiple typed blocks or a complete streaming
  and tool-call lifecycle.

The examples are useful product references, but repeated state and rendering
logic should move into tested components.

## 5. Proposed Component Architecture

```text
Chat shell
├── Transcript
│   ├── MessageList
│   │   └── ChatMessageView
│   │       ├── MessageHeader
│   │       ├── RoleMarker
│   │       └── MessageBlockView[]
│   │           ├── TextBlock
│   │           ├── MarkdownBlock
│   │           ├── CodeBlock
│   │           ├── ThinkingBlock
│   │           ├── ToolCallBlock
│   │           ├── ToolResultBlock
│   │           └── ErrorBlock
│   ├── StreamingIndicator
│   └── TranscriptEmptyState
└── ComposerRegion
    ├── ChatComposer
    ├── ContextLine
    └── ChatStatusBar
```

Two shells share these primitives:

```text
InlineChatShell
├── committed transcript -> terminal native scrollback
├── active streaming message -> live region
└── ChatComposer -> live region

FullscreenChatShell
├── TranscriptViewport -> flexible height
├── ChatComposer -> fixed bottom
└── ChatStatusBar -> fixed bottom
```

### 5.1 Layer boundaries

| Layer | Responsibility | Must not own |
| --- | --- | --- |
| Conversation model | Typed messages, blocks, status, identity | Rendering or network requests |
| Headless state | Updates, selection, streaming transitions, scroll intent | ANSI output or provider clients |
| Interaction | Composer and transcript key handling | Global hidden side effects |
| View primitives | Render one message, block, indicator, or status line | Conversation persistence |
| Shells | Inline/fullscreen composition and focus routing | Provider-specific logic |
| Adapter | Translate external events into conversation updates | Core component rendering |

## 6. Proposed Conversation Model

The following API is proposed and does not exist yet.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MessageId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageRevision(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Tool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: MessageId,
    pub revision: MessageRevision,
    pub role: ChatRole,
    pub blocks: Vec<MessageBlockEntry>,
    pub status: MessageStatus,
    pub author: Option<String>,
    pub timestamp: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageBlockEntry { pub id: BlockId, pub block: MessageBlock }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageBlock {
    Text(String),
    Markdown(String),
    Code {
        language: Option<String>,
        content: String,
    },
    Diff(DiffContent),
    Quote(QuoteContent),
    Link(LinkContent),
    Thinking(ThinkingContent),
    ToolCall(ToolCallContent),
    ToolResult(ToolResultContent),
    TerminalAttachmentSummary(TerminalAttachmentSummary),
    Error(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageStatus {
    Pending,
    Streaming,
    Complete,
    Failed { message: String },
    Cancelled,
}
```
`MessageRevision::INITIAL` is zero and required in serialized `ChatMessage`.
Missing, negative, or overflowing values are typed errors; the legacy wrapper
uses `INITIAL`; edits increment once; thinking/tool types remain explicit.

### 6.1 Streaming update contract

Applications should not mutate internal vectors or concatenate arbitrary output
through a rendering component. A proposed explicit update protocol is:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub struct UpdateId(String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct InvalidUpdateId;
impl UpdateId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidUpdateId> {
        let value = value.into(); if value.trim().is_empty() { Err(InvalidUpdateId) } else { Ok(Self(value)) }
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
impl TryFrom<String> for UpdateId { type Error = InvalidUpdateId; fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(value) } }
impl std::fmt::Display for UpdateId { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) } }
impl std::fmt::Display for InvalidUpdateId { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("update ID must not be empty or whitespace") } }
impl std::error::Error for InvalidUpdateId {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationEvent {
    pub event_id: UpdateId,
    pub sequence: u64,
    pub update: ConversationUpdate,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationUpdate {
    Push(ChatMessage),
    AppendText { message_id: MessageId, expected_revision: MessageRevision, block_id: BlockId, delta: String },
    ReplaceBlock { message_id: MessageId, expected_revision: MessageRevision, block_id: BlockId, block: MessageBlock },
    AppendMessageBlock { message_id: MessageId, expected_revision: MessageRevision, block: MessageBlockEntry },
    InsertMessageBlock { message_id: MessageId, expected_revision: MessageRevision, index: usize, block: MessageBlockEntry },
    EditMessage { message_id: MessageId, expected_revision: MessageRevision, blocks: Vec<MessageBlockEntry> },
    DeleteMessage { message_id: MessageId, expected_revision: MessageRevision },
    Resend { source_message_id: MessageId, expected_revision: MessageRevision, new_message: ChatMessage },
    Complete { message_id: MessageId, expected_revision: MessageRevision },
    Cancel { message_id: MessageId, expected_revision: MessageRevision },
    Fail { message_id: MessageId, expected_revision: MessageRevision, message: String },
}
```
`UpdateId::new`/`TryFrom<String>` reject empty or whitespace-only IDs with the typed
`InvalidUpdateId`; `Display`/`as_str` expose validated values without opening the
private field. `sequence` is conversation-wide and adapter-supplied. The reducer retains
event IDs and outcomes for a documented sequence/time window. Replay inside that
window returns the original outcome; replay after eviction returns typed
`ReplayOutsideRetention`. ID/content conflict, stale sequence, or a gap is a
distinct explicit error, keeping the boundary provider-independent.

Every event returns an explicit result; missing IDs, invalid indices, stale target
revisions, invalid transitions, and ordering failures are errors, not warnings
plus fallback. Every mutation, including Complete/Cancel/Fail, compares
`expected_revision`; stale terminal events leave state/revision unchanged. A
successful mutation increments once. Resend preserves its terminal source.

### 6.2 Required state transitions

```text
Pending -> Streaming -> Complete
Pending -> Cancelled
Streaming -> Cancelled
Pending -> Failed
Streaming -> Failed
```

Completed, cancelled, and failed messages are terminal states unless an explicit
retry creates a new message identity.

## 7. Component Contracts

### 7.1 `ChatMessageView`

Responsibilities:

- render role, author, timestamp, blocks, and message status
- provide compact, bordered, and bubble variants
- wrap continuation lines consistently
- allow a typed block renderer registry
- expose semantic accessibility information where supported

The existing `Message` public API should remain as a simple compatibility
wrapper. A new `ChatMessageView` avoids breaking its string-only constructor.

### 7.2 Message block views

| Component | Required behavior |
| --- | --- |
| `TextBlock` | Plain text, wrapping, selection-safe output |
| `MarkdownBlock` | Headings, lists, quotes, links, emphasis, fenced code |
| `CodeBlock` | Language label, horizontal strategy, copy hint, optional line numbers |
| `DiffBlock` | Typed file/hunk metadata, added/removed/context lines, copy-safe output |
| `QuoteBlock` | Typed quoted content and optional source label without flattening into Markdown |
| `LinkBlock` | Typed label/target pair, safe terminal fallback, selectable target |
| `ThinkingBlock` | Streaming state, collapsed/expanded view, maximum preview lines |
| `ToolCallBlock` | Tool name, typed arguments presentation, pending/running/success/failure state |
| `ToolResultBlock` | Output preview, truncation indicator, expansion, error state |
| `TerminalAttachmentSummaryBlock` | Typed attachment identity, display name, media/size metadata, availability state; summary only, never implicit file access |
| `ErrorBlock` | Visible failure message and optional retry hint |
| `StreamingIndicator` | Pending or active generation without changing message semantics |

### 7.3 `MessageList`

Responsibilities:

- render variable-height messages
- maintain a line-height cache keyed by stable message identity, width, content
  revision, visual variant, and block expansion revision
- preserve the user's scroll position when older messages are prepended
- follow the bottom only while the user has not intentionally scrolled upward
- show a "new output below" indicator when bottom-following is paused
- expose page, line, top, bottom, and message navigation outcomes
- keep selection and focused message stable across updates

The current `virtual_scroll_view` is item-count based. It is not sufficient for
chat messages because one message may occupy one line or hundreds of lines.

### 7.4 `ChatComposer`

`ChatComposer` should build on `TextAreaState`, not on another example-local
input implementation.

Required behavior:

- multiline editing with explicit LF/CRLF normalization
- configurable submit and newline shortcuts
- insertion, deletion, cursor movement, word movement, and selection using shared TextFlow grapheme/cell boundaries
- bracketed paste
- CJK, emoji, combining sequence, and wide-character correctness
- auto-growing height capped by `max_visible_lines`
- placeholder and context hints
- enabled, read-only, disabled, and submitting modes
- typed clear/cancel/submission errors that leave the controlled draft atomically unchanged

Proposed controlled contract:

```rust
pub struct ComposerSubmission {
    pub text: String,
}

pub fn handle_chat_composer_input(
    state: &mut ChatComposerState,
    input: &str,
    key: &Key,
    keymap: &ChatComposerKeyMap,
    mode: InteractionMode,
) -> InteractionOutcome<ComposerSubmission>;
```

Suggested default keys:

| Action | Default | Notes |
| --- | --- | --- |
| Submit | Enter | Configurable |
| Insert newline | Shift+Enter or configured fallback | Terminal support varies, so a fallback such as Alt+Enter or Ctrl+J is needed. |
| Cancel generation | Escape | Shell decides whether a request is active. |
| Clear input | Ctrl+U | Must not affect transcript. |
| Move through history | Configurable Up/Down behavior | Only at document boundaries. |

### 7.5 `ChatStatusBar`

It should compose existing status and key-hint primitives and may display:

- active model label
- connection state
- generation status
- elapsed time
- token or context usage supplied by the application
- active keyboard shortcuts

No-data fields render blank. The component must not invent token counts,
connection state, or model metadata.

## 8. Inline and Fullscreen Semantics

### 8.1 Inline chat

Inline mode is the preferred Claude/Codex-style terminal experience.

Rules:

1. Completed transcript content is submitted through a typed scrollback sink
   with a stable `commit_id`.
2. The composer remains in the live dynamic region.
3. A streaming assistant message remains live until it reaches a terminal state.
4. After a confirmed commit, the message is removed from the live region.
5. Terminal-native scrolling is preserved; the app does not simulate an entire
   scrollback buffer.
6. Exiting or panicking must restore raw mode, cursor visibility, and terminal
   state.

Already committed lines cannot be reliably edited in place across terminals.
Therefore streaming content must not be committed before it is stable.

The default native-terminal sink cannot make a terminal write and an in-memory
ledger one atomic transaction. It guarantees process-local deduplication for a
confirmed commit, but an interrupted or partially observed write must return an
explicit `Unknown` outcome and must not be retried automatically. Cross-retry or
cross-process exactly-once delivery is available only when an injected sink
persists `commit_id` and implements an atomic idempotency contract. The public
API must not claim a stronger guarantee than the selected sink can provide.

### 8.2 Fullscreen chat

Fullscreen mode owns the visible transcript and uses the alternate screen.

Rules:

1. Transcript viewport takes remaining vertical space.
2. Composer and status areas stay fixed at the bottom.
3. Resize triggers layout and message-height recalculation.
4. Streaming follows the bottom only when bottom anchoring is active.
5. Page navigation and message navigation remain available while generation
   continues.
6. Leaving fullscreen restores the original terminal screen.

### 8.3 Why these should be separate shells

Inline mode commits immutable content to native scrollback. Fullscreen mode owns
and redraws the complete visible transcript. Combining both behind one complex
conditional component would hide distinct lifecycle, scrolling, and failure
semantics. Shared primitives should be reused, but shell behavior should remain
explicit.

## 9. Complete Capability Matrix

A product-grade chat component suite ultimately needs all of the following.
Every listed capability has a child owner, umbrella task owner, and acceptance
invariant; the child packet must refine that assignment rather than silently
drop it.

- **Conversation:** streaming -> GH-62 / SP57-T4 / B-008+B-009; stop/cancel ->
  GH-62+GH-66 / SP57-T4+SP57-T5 / B-021; retry/regenerate -> GH-62 / SP57-T4 /
  B-008; edit/delete/resend -> GH-62+GH-63 / SP57-T4 / B-006+B-018; explicit
  recovery -> GH-62+GH-66+GH-68 / SP57-T4+SP57-T5 / B-017+B-021+B-025;
  complete tool-call lifecycle -> GH-62+GH-63 / SP57-T4 / B-007+B-010+B-022;
  collapsible thinking -> GH-63+GH-65 / SP57-T4 / B-010+B-015;
  Markdown/code/diff/quote/link blocks and terminal attachment representation ->
  GH-63 / SP57-T4 / B-010; copy/selection/search -> GH-63+GH-64+GH-65 /
  SP57-T4 / B-014+B-015+B-019; empty/loading/disconnected/rate-limited/failed
  states -> GH-62+GH-63 / SP57-T4 / B-005+B-007+B-010+B-019+B-021.
- **Terminal correctness:** CJK/emoji/combining/grapheme/long-token correctness ->
  GH-58+GH-64 / SP57-T3+SP57-T4 / B-014+B-016; resize/reflow ->
  GH-58+GH-65+GH-67 / SP57-T3+SP57-T4+SP57-T5 / B-013+B-015+B-016; large
  histories and high-frequency streaming -> GH-61+GH-68 / SP57-T3+SP57-T5 /
  B-020; flicker-free output -> GH-60+GH-61+GH-66+GH-67 /
  SP57-T3+SP57-T5 / B-017+B-020+B-025; paused bottom-follow ->
  GH-65+GH-66+GH-67 / SP57-T4+SP57-T5 / B-015; macOS/Linux/Windows and
  SSH/tmux compatibility -> GH-68 / SP57-T5 / B-020+B-025; safe raw-mode and
  cursor restoration -> GH-66+GH-67+GH-68 / SP57-T5 / B-012+B-013+B-025.
- **Component quality:** controlled contracts and documented uncontrolled usage
  -> GH-62+GH-64+GH-65 / SP57-T4 / B-004+B-014+B-015+B-018; explicit
  state/handler/outcome/keyboard behavior and consistent interaction modes ->
  GH-62+GH-64 / SP57-T4 / B-007+B-014+B-019; configurable keymaps -> GH-64 /
  SP57-T4 / B-014; semantic themes -> GH-63+GH-68 / SP57-T4+SP57-T5 /
  B-010+B-019; typed render extensions -> GH-63 / SP57-T4 / B-010+B-022;
  provider-independent APIs -> GH-62+GH-68 / SP57-T4+SP57-T5 / B-004+B-022;
  pre-1.0 deprecation policy -> GH-63+GH-68 / SP57-T4+SP57-T5 / B-018.
- **Developer experience:** application-owned Unicode wrapping -> GH-58 /
  SP57-T3 / B-016; visual cursors -> GH-64 / SP57-T4 / B-014; message heights
  -> GH-58+GH-65 / SP57-T3+SP57-T4 / B-015+B-016; bottom-following -> GH-65 /
  SP57-T4 / B-015; delta concatenation -> GH-62 / SP57-T4 /
  B-006+B-008+B-009; tool transitions -> GH-62+GH-63 / SP57-T4 /
  B-007+B-010+B-022; inline transcript commitment -> GH-66 / SP57-T5 / B-012;
  focus competition -> GH-64+GH-66+GH-67 / SP57-T4+SP57-T5 /
  B-011+B-013+B-019. These assignments mean applications do not reimplement
  those mechanics.

A target application surface should be small and declarative. The exact API
must be validated before it is declared stable.

## 10. Maturity Ladder and Definition of Done

| Stage | Definition |
| --- | --- |
| MVP | Input, submission, messages, and basic streaming work in one mode. |
| Reusable component library | Shared composer, message blocks, scrolling, themes, inline shell, and fullscreen shell exist. |
| Production-capable | Unicode, resize, failure recovery, performance, and supported-platform tests pass. |
| Element AI-level maturity | Stable APIs, extension contracts, complete docs, variants, examples, compatibility policy, and upgrade safety exist. |
| Perfect | Not a practical terminal condition; quality must be continuously measured and improved. |

### 10.1 Verification gates

The library is product-capable only when fresh evidence covers:

- state-machine unit tests
- input, paste, scroll, and resize integration tests
- ANSI output snapshots
- CJK, emoji, grapheme, and width property tests
- inline and fullscreen end-to-end tests
- long-session performance benchmarks
- high-frequency streaming stress tests
- terminal compatibility checks
- zero ignored mapped, critical, integration, PTY, or example acceptance tests
- current-head artifact proving at least 80% changed-executable-line coverage
- 100% artifact coverage for every declared critical transition and recovery path

## 11. Current Layout Pipeline Audit

### 11.1 Current pipeline

The inspected dynamic-frame path is approximately:

```text
Component function
    -> Element tree
    -> Element-to-VNode conversion
    -> VNode diff
    -> patches applied to TaffyTree
    -> Taffy layout computation
    -> Element tree rendering into Output cells
    -> ANSI line generation
    -> terminal line comparison and repaint
```

The main app pipeline currently calls `compute_element_incremental()`. An older
statement in `DESIGN_ISSUES.md` says reconciliation is not connected to the main
render loop; that statement is no longer accurate for the inspected baseline.

Other paths, including direct layout computation, static content, and
`render_to_string`, still use full-tree computation in relevant call sites.

### 11.2 What is architecturally sound

- Taffy is a strong Flexbox foundation and avoids creating a custom general
  layout algorithm.
- `Element`, layout, cell composition, and terminal output are conceptually
  separated.
- Text measurement uses Unicode grapheme segmentation and display-width logic.
- The dynamic pipeline retains a previous VNode snapshot and attempts
  incremental patching.
- Layout can be queried using stable node-oriented mappings as well as current
  frame element IDs.
- Terminal output has clipping and a persistent line-level comparison layer.
- Inline and fullscreen rendering are explicit modes.

The choice of Taffy and the high-level separation are correct. The gaps are in
cross-layer contracts and failure handling.

### 11.3 Findings

| ID | Severity | Evidence and impact | Required direction |
| --- | --- | --- | --- |
| LAY-01 | Critical | Measurement counts wrapped rows, but the tree renderer sends original text to `Output::write()` once; writing stops at the row edge. Rich spans also lack a shared automatic text-flow result. Computed height and drawn content can disagree, truncate chat messages, and misplace later content. | Produce measured dimensions and positioned styled runs through one text-flow operation consumed by both layout and composition. |
| LAY-02 | Critical | `NodeKey::matches()` ignores index for keyed nodes, while child diff lookup includes index; synthetic identities also include ancestor paths. These competing contracts create a stable-identity risk during reorder; the audit does not claim that every reorder demonstrably loses state. | Use one parent-scoped canonical identity: user key plus compatible type for keyed children, with sibling index stored only as position. |
| LAY-03 | Critical | `Create` carries no explicit insertion index and appends a Taffy child; correct final order therefore depends on later reorder inference, which ignores some forward shifts. Middle insertion can make Taffy order disagree with Element/VNode traversal. | Derive the intended position from the canonical target tree and validate every parent's final Taffy order against the current VNode order. |
| LAY-04 | Critical | Patch helpers report independent success, successful patches set one shared flag, and many Taffy results are ignored. A partially successful sequence can be accepted and silently corrupt layout. | Make patching transactional: any failure rejects incremental state and triggers one deterministic full rebuild; surface an error if rebuild fails. |
| LAY-05 | High | Removal and replacement delete the requested root mapping without explicit recursive descendant cleanup. Stale NodeKey-to-NodeId entries may survive and later target invalid nodes. | Remove complete subtree mappings before mutation and verify map/tree consistency afterward. |
| LAY-06 | High | Several Taffy results are assigned to `_`, while missing layouts can become `Layout::default()`. Invalid layout can appear as blank or misplaced output without the root cause. | Propagate typed layout errors; allow one explicit full-rebuild recovery but never silently substitute a default layout after rebuild failure. |
| LAY-07 | High | Dynamic apps use incremental Element-to-VNode layout, while direct `compute()`, static rendering, and render-to-string use related but different paths. No parity contract proves that they stay equivalent; this is an architecture and verification gap, not evidence that every current snapshot differs. | Define one canonical layout/text-flow service whose full and incremental strategies produce the same immutable snapshot. |
| LAY-08 | High for chat | `virtual_scroll_view` intentionally treats offsets and viewport height as item counts. That contract remains valid for fixed-height lists, but it cannot provide the row-based semantics required by variable-height Markdown, code, tools, and thinking blocks. | Add a chat-specific row-offset message list with a width-dependent height cache invalidated by width, content, variant, and expansion changes; preserve the fixed-height API. |
| LAY-09 | Medium | Taffy produces floating-point coordinates and the terminal consumes integer cells; rounding ownership and invariants are not documented. This is a quantization-contract gap, not proof of universal overlap or gaps in current output. | Define cell-rounding rules and assert against sibling overlap, rounding gaps, and out-of-bounds borders. |
| LAY-10 | Medium | Existing benchmarks emphasize full layout of generic trees, not unchanged frames, streaming deltas, appended messages, middle insertion, variable-height scrolling, or resize invalidation. | Add realistic chat benchmarks and compare validated incremental updates with full rebuilds. |

### 11.4 Layout verdict

The layout foundation is not the most elegant or fully correct version yet.

The precise conclusion is:

> Taffy is the right general layout engine, and the Element-to-layout-to-output
> layering is directionally sound. The current implementation still has
> correctness risks in text flow, identity, ordering, transactional patching,
> error propagation, and variable-height scrolling. Those contracts must be
> repaired before the layout system can be considered a product-grade chat UI
> foundation.

## 12. Recommended Target Layout Architecture

```text
Element tree
    -> canonical stable semantic tree
    -> validated tree diff
    -> transactional Taffy adapter
    -> immutable LayoutSnapshot
    -> shared TextFlow cache
    -> cell compositor with clipping and scroll transforms
    -> frame diff
    -> terminal writer
```

### 12.1 Canonical stable tree

- one identity rule for keyed and unkeyed nodes
- parent-scoped user keys
- sibling index stored separately from identity
- no frame-generated Element ID used for cross-frame identity

### 12.2 Transactional layout adapter

- validate all patch targets before mutation
- apply structural changes at exact indices
- clean subtree mappings recursively
- fail the whole incremental attempt on any error
- full rebuild as the only recovery path
- surface an error when full rebuild also fails

### 12.3 Immutable layout snapshot

Rendering should consume a read-only snapshot containing:

- stable node identity
- integer terminal-cell bounds
- content bounds after border and padding
- clipping region
- scroll transform
- measured text-flow identity

The renderer should not query a partially mutated Taffy tree.

### 12.4 Shared text-flow cache

Cache keys should include at least:

- content or content revision
- styled-span structure
- available width
- wrap policy
- overflow policy
- relevant Unicode-width policy

The result should include:

- row count
- maximum row width
- positioned styled runs for every row
- source-to-cell mapping needed for cursor and selection

Measurement and drawing must use this same result.

### 12.5 Chat transcript height index

For variable-height messages, maintain:

- `message_id -> measured_height`
- prefix sums or a Fenwick-style height index for row lookup
- viewport row offset
- bottom anchor state
- width revision and expansion revision

This enables efficient lookup of the first visible message and stable scrolling
while content changes.

## 13. Recommended Delivery Order

### Phase 0: Contract and reproduction tests

- document stable identity and cell-rounding rules
- reproduce long-text truncation
- reproduce middle insertion and keyed reorder behavior
- reproduce partial patch failure handling
- add inline/fullscreen parity fixtures

Done when failures are deterministic and no implementation change is based on
an unproven hypothesis.

### Phase 1: Layout correctness

- unify text measurement and rendering
- make patch application transactional
- correct keyed identity and insertion order
- recursively clean subtree mappings
- propagate layout errors

Done when incremental and full layout produce equivalent snapshots for the same
trees across structural property tests.

### Phase 2: Chat model and block primitives

- add explicit conversation types and update transitions
- add `ChatMessageView` and typed block views
- keep existing `Message` compatibility
- add semantic chat theme tokens

Done when all message and tool states have render and transition coverage.

### Phase 3: Composer extraction

- implement `ChatComposer` on `TextAreaState`
- add configurable submit/newline keymap
- support paste, wide characters, selection, and auto-grow
- migrate one existing example manually

Done when the migrated example contains no separate cursor or wrapping state.

### Phase 4: Inline shell

- manage committed transcript, active stream, and composer regions
- commit completed output exactly once
- preserve native scrollback
- verify terminal restoration

Done when streaming, cancel, failure, completion, and resize paths pass end-to-end
tests.

### Phase 5: Fullscreen shell and variable-height transcript

- implement row-based message virtualization
- preserve scroll anchor across append, prepend, expand, and resize
- pause and resume bottom-follow behavior
- integrate fixed-bottom composer and status areas

Done when long sessions remain correct and responsive under benchmarked loads.

### Phase 6: Example convergence and API hardening

- migrate `chat.rs`, `rnk_chat.rs`, `claude_input_box.rs`, and `glm_chat.rs`
  one at a time
- remove duplicated behavior only after parity is verified
- publish complete usage and extension documentation
- stabilize the supported public surface

Done when examples demonstrate different compositions of shared components rather
than separate input, wrapping, scrolling, and message systems.

### Phase 7: Optional crate extraction

Start inside `rnk::components::chat` so layout and runtime contracts can mature
together. Consider a separate `rnk-chat` crate only after:

- public component contracts are stable
- dependencies on runtime internals are minimal
- examples have converged
- cross-crate versioning cost is justified

## 14. Proposed MVP Public Surface

The first public chat release should stay focused:

1. `ChatComposer`
2. `ChatMessageView`
3. `MessageList`
4. `StreamingIndicator`
5. `InlineChatShell`
6. `FullscreenChatShell`

Supporting public model and handler types are required, but provider adapters,
tool execution, and persistence remain outside the core UI package.

## 15. Final Decision Summary

- Build a component family, not one monolithic chat widget.
- Reuse existing rnk primitives before creating new ones.
- Keep the chat model typed and backend independent.
- Implement inline and fullscreen as separate shells.
- Build `ChatComposer` on the existing textarea state model.
- Treat streaming and tool calls as explicit state machines.
- Do not use item-count virtualization for variable-height messages.
- Keep the current `Message` API compatible and introduce a richer message view.
- Keep the initial implementation in `rnk::components::chat`; extract a crate
  only after the API stabilizes.
- Keep Taffy as the general layout engine.
- Repair text-flow equivalence, stable identity, child ordering, transactional
  patching, subtree cleanup, and error propagation before claiming a fully
  correct layout foundation.
- Define completion through fresh tests, compatibility evidence, and benchmarks,
  not through visual demos alone.
