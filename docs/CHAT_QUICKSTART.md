# Building a Chat With rnk

Two shapes, one set of parts. Pick the shape first, because it decides what you
can undo.

| | Inline | Fullscreen |
|---|---|---|
| Finished transcript lives in | the terminal's own scrollback | your application's viewport |
| Can re-flow on resize | **no** — committed lines belong to the terminal | yes |
| Can scroll back through history | with the terminal's own scroll gesture | with the transcript's scroll API |
| Survives your process exiting | yes, the lines stay on screen | no, the alternate screen is discarded |
| Shell | `InlineChatShell` | `FullscreenChatShell` |

The asymmetry is not a missing feature. Inline hands finished lines to the
terminal and can no longer address them; that is exactly why they persist.

This minimal typed-state example is compiled as a doctest:

```rust
use rnk::components::chat::{
    BlockId, ChatMessage, ChatRole, ConversationEvent, ConversationGuard,
    ConversationState, ConversationUpdate, MessageBlock, MessageBlockEntry,
    MessageId, UpdateId,
};
use std::num::NonZeroUsize;

let mut state = ConversationState::new(0, NonZeroUsize::MIN);
let message = ChatMessage::new(
    MessageId::new(1),
    ChatRole::User,
    vec![MessageBlockEntry::new(
        BlockId::new(1),
        MessageBlock::Text("hello".to_owned()),
    )],
)?;
let update = ConversationUpdate::push(ConversationGuard::new(state.revision()), message);
state.apply_event(ConversationEvent::new(UpdateId::new("push")?, 0, update))?;
assert_eq!(state.messages().len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Inline quickstart

```rust,compile
use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    InlineChatShell, InlineCommitReport, MessageId, MessageRevision,
    ProjectionContext, ScrollbackNamespace, ThemeIdentity,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut shell = InlineChatShell::new(
    // Stable within this process. NativeTerminalSink does not persist its
    // ledger across restart; durable cross-process dedup needs a durable sink.
    ScrollbackNamespace::new("my-app.session")?,
    NativeTerminalSink::new(std::io::sink()),
);
let context = ProjectionContext::new(80, ThemeIdentity::new(1))?;

// A message that is still streaming stays in the live region. Deltas need no
// call at all: a live message has no final content, so nothing can be committed.
shell.stream(MessageId::new(1))?;

// When it finishes, commit it. Safe to call again for a duplicate terminal
// event — the identity is derived from the content, so the sink recognises it.
match shell.finish(MessageId::new(1), MessageRevision::INITIAL, "done", context)? {
    InlineCommitReport::Fixed { .. } => {
        // Confirmed. The message has left the live region.
    }
    InlineCommitReport::Retained { cause: _ } => {
        // Provably nothing was written. Retryable, when you decide to.
    }
    InlineCommitReport::Latched { evidence: _ } => {
        // Undecidable. See "Error handling" below — do not retry this.
    }
}
Ok(())
}
```

Run `cargo run --example inline_chat_scrollback` to see all three outcomes.

## Fullscreen quickstart

```rust,compile
use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, FullscreenChatShell,
    FullscreenKeyOutcome,
};
use rnk::components::chat::message_list::{
    MessageListState, MessageMeasureOutcome, ViewportRows,
};
use rnk::hooks::Key;

fn main() -> Result<(), Box<dyn std::error::Error>> {
let transcript = MessageListState::try_new::<(), (), _>(
    &[], 80, ViewportRows::new(20), 8,
    |_| MessageMeasureOutcome::Missing,
).expect("an empty list does not invoke measurement");
let mut shell = FullscreenChatShell::try_new(
    transcript,
    ChatComposerState::new(),
    80, 24,
    1,                        // status bar rows
)?;

// Regions, top to bottom. They tile the terminal exactly.
let layout = shell.layout();
assert_eq!(layout.width(), 80);

match shell.handle_key(&ChatComposerKeyMap::new(), "hello", &Key::default())? {
    FullscreenKeyOutcome::Submitted(_) | FullscreenKeyOutcome::Overlay
    | FullscreenKeyOutcome::Cancelled | FullscreenKeyOutcome::Changed(_)
    | FullscreenKeyOutcome::Consumed(_) | FullscreenKeyOutcome::Unconsumed(_) => {}
}

shell.try_resize(100, 30)?;
Ok(())
}
```

Run `cargo run --example fullscreen_chat_shell` to see region assignment across
resizes, including one that is refused.

## Updating a conversation

`ConversationState` is provider-independent data. It parses nothing, requests
nothing, and writes to no terminal. Every mutation is an event carrying a guard,
so a stale update is rejected rather than applied to a conversation that moved
underneath it.

```rust,ignore
use rnk::components::chat::{
    ConversationEvent, ConversationGuard, ConversationUpdate,
    MessageMutationGuard, UpdateId,
};

let push = ConversationUpdate::push(
    ConversationGuard::new(state.revision()),
    message,
);
state.apply_event(ConversationEvent::new(UpdateId::new("push")?, 0, push))?;

// Appending a delta needs the message's own revision as well, so two concurrent
// writers cannot both append to what they each think is the current text.
let message = state.message(MessageId::new(1)).expect("pushed above");
let guard = MessageMutationGuard::new(
    ConversationGuard::new(state.revision()),
    message.id(),
    message.revision(),
);
let append = ConversationUpdate::append_text(guard, BlockId::new(1), "hello")?;
state.apply_event(ConversationEvent::new(UpdateId::new("delta")?, 1, append))?;
```

## Custom block renderers

`ChatBlockRenderer` decides how one block becomes an `Element`. Implement it when
you have a block kind the built-in variants do not cover — a diff view, a chart,
a domain-specific tool result.

```rust,ignore
use rnk::components::chat::{
    ChatBlockRef, ChatBlockRenderer, ChatRenderContext, ChatRenderOverride,
};

struct MyRenderer;

impl ChatBlockRenderer for MyRenderer {
    fn render(
        &self,
        block: ChatBlockRef<'_>,
        context: ChatRenderContext<'_>,
    ) -> ChatRenderOverride {
        match block {
            ChatBlockRef::Code(code) => ChatRenderOverride::element(my_code_view(code)),
            // Everything else falls through to the library's typed renderer.
            _ => ChatRenderOverride::UseDefault,
        }
    }
}
```

Any `Fn(ChatBlockRef, ChatRenderContext) -> ChatRenderOverride` implements the
trait, so a closure works where a named type is overkill.

`UseDefault` rather than an empty element matters: a renderer that returns a
blank body for blocks it does not understand silently hides content, and the
fall-through path is the one that shows it.

## Keymaps

`ChatComposerKeyMap::new()` is the default binding set. Rebind by category:

```rust,ignore
let keymap = ChatComposerKeyMap::new()
    .submit(vec![/* bindings */])
    .newline(vec![/* bindings */])
    .cancel(vec![/* bindings */])
    .clear(vec![/* bindings */]);
```

Two default behaviours are deliberate and surprise people:

- **Enter stages a submission; it does not clear the draft.** The draft is
  cleared by `acknowledge_success`, once the send is known to have worked.
  Clearing on Enter destroys the user's text at exactly the moment a failed send
  makes it hardest to reproduce.
- **Escape cancels the interaction, not the draft.** It reports `Cancelled` and
  leaves the text alone, so a stray key cannot discard a long message.

## Error handling

Every fallible operation returns a typed outcome rather than an `Option` or a
bare `bool`. The one worth understanding before you write any of it is the
three-state commit result.

| Outcome | Bytes the terminal accepted | What you may do |
|---|---|---|
| `Committed` | all of them, flushed, recorded | remove the message from the live region |
| `NotCommitted` | provably zero | retry, when you choose to |
| `Unknown` | somewhere in between, or unknowable | **neither** |

`Unknown` is the state a `Result<(), io::Error>` cannot express, and omitting it
is what duplicates transcript lines: a partial write that gets retried writes its
accepted prefix twice. There is no automatic recovery, because nothing inside
your process can observe what the terminal already showed.

`InlineChatShell` therefore *latches* a message on `Unknown` and refuses to touch
it again. Clearing the latch is an explicit human decision:

```rust,ignore
use rnk::components::chat::UnknownResolution;

// After a human has looked at the terminal:
shell.resolve(id, UnknownResolution::AlreadyVisible)?; // drop it, do not rewrite
shell.resolve(id, UnknownResolution::NotVisible)?;     // allow one more attempt
```

Error enums and the scrollback outcome enums marked `#[non_exhaustive]` require
a wildcard arm. `InlineCommitReport`, `InlineKeyOutcome`,
`FullscreenKeyOutcome`, and `MessageMeasureOutcome` are currently exhaustive;
match every variant so an additive change produces a compile-time review point.

## Non-goals

These are out of scope by design, not yet-to-do:

- **Model requests, retries, tool execution and session persistence in the chat
  module.** Core chat remains provider-independent. `glm_chat` is an explicit
  adapter example: its network and default-deny workspace policy stay in the
  example and never become capabilities of `MessageBlock::ToolCall`.
- **Simulating a terminal scrollback buffer.** Inline hands lines to the terminal
  and stops tracking them.
- **Rewriting history already committed to a terminal.** Once bytes are in the
  terminal's scrollback nothing can address them, which is why identity and
  conflict detection fail closed instead of overwriting.
- **Cross-process exactly-once over a plain terminal write.** The write and its
  record are two events with no transaction around them. `NativeTerminalSink`
  claims `ProcessLocalConfirmed` and nothing more; anything stronger needs a
  `DurableCommitStore`.
- **Assuming every message is one row.** Nothing in the transcript path uses item
  counts as row offsets.
- **Promising untested terminals, platforms or input capabilities.** See
  [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md), which marks what is
  implemented, what is best effort, and what is not automatically tested.

## Where to look next

| Topic | File |
|---|---|
| Stability grade of each chat type | [API_STABILITY.md](API_STABILITY.md) |
| Terminal, multiplexer and OS support | [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md) |
| Component architecture and phasing | [CHAT_UI_COMPONENT_ARCHITECTURE.md](CHAT_UI_COMPONENT_ARCHITECTURE.md) |
| Runnable examples and their intent | [../examples/README.md](../examples/README.md) |
