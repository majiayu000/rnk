//! Inline chat that commits finished transcript into the terminal's scrollback.
//!
//! Run it, then scroll up: the committed lines are in your terminal's own
//! history, not in a buffer this program owns. That is the point — and it is
//! also why committing is irreversible, which is what the whole lifecycle is
//! built around.
//!
//! The example composes public components only. There is no private "did I
//! already print this" flag anywhere in it; every decision comes from a typed
//! outcome the library returned.
//!
//! ```text
//! cargo run --example inline_chat_scrollback
//! ```

use std::io::{self, Write};

use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    AttemptDisposition, ChatComposerKeyMap, InlineChatShell, InlineCommitReport, InlineKeyOutcome,
    MessageId, MessageRevision, ProjectionContext, ScrollbackNamespace, ThemeIdentity,
};
use rnk::hooks::Key;

/// One scripted assistant reply, delivered the way a provider delivers them.
struct ScriptedReply {
    id: u64,
    deltas: &'static [&'static str],
    /// How many times the provider announces this message as finished.
    ///
    /// Two is not a contrived case: retries, reconnects and at-least-once
    /// delivery all produce it, and it is exactly what must not print twice.
    terminal_events: usize,
}

const SCRIPT: &[ScriptedReply] = &[
    ScriptedReply {
        id: 1,
        deltas: &["Committing ", "is ", "one-way."],
        terminal_events: 1,
    },
    ScriptedReply {
        id: 2,
        deltas: &["A duplicate ", "terminal event ", "must not print twice."],
        terminal_events: 3,
    },
];

fn main() -> io::Result<()> {
    let namespace = ScrollbackNamespace::new("example.inline-chat")
        .expect("a non-empty, restart-stable namespace");
    let context = ProjectionContext::new(80, ThemeIdentity::new(1)).expect("a non-zero width");

    let mut shell = InlineChatShell::new(namespace, NativeTerminalSink::new(io::stdout()));

    // The composer is in the live region from the start and never leaves it.
    let keymap = ChatComposerKeyMap::new();
    match shell.handle_key(&keymap, "explain the commit boundary", &Key::default()) {
        InlineKeyOutcome::Changed(draft) => println!("[composer] draft: {draft}\r"),
        other => println!("[composer] unexpected: {other:?}\r"),
    }

    for reply in SCRIPT {
        let id = MessageId::new(reply.id);
        shell.stream(id).expect("this message is not live yet");

        // Deltas repaint the live region and commit nothing. A streaming message
        // has no final content, so there is nothing here to fix into history.
        let mut streamed = String::new();
        for delta in reply.deltas {
            streamed.push_str(delta);
            print!("\r\x1b[2K[live] {streamed}");
            io::stdout().flush()?;
        }
        print!("\r\x1b[2K");

        for event in 1..=reply.terminal_events {
            let report = shell
                .finish(id, MessageRevision::INITIAL, &streamed, context)
                .expect("this message is not latched");

            match report {
                InlineCommitReport::Fixed {
                    receipt,
                    disposition: AttemptDisposition::Written,
                } => println!("[commit {event}] wrote {receipt}\r"),
                // The line is already in the terminal's scrollback. The shell
                // reports the *original* receipt and writes nothing.
                InlineCommitReport::Fixed {
                    receipt,
                    disposition: AttemptDisposition::AlreadyCommitted,
                } => println!("[commit {event}] already committed as {receipt}\r"),
                // Nothing reached the terminal, so the message stays visible in
                // the live region and the caller may retry it.
                InlineCommitReport::Retained { cause } => {
                    println!("[commit {event}] not committed: {cause}\r");
                }
                // Some bytes may be on screen and nothing in this process can
                // find out which. Retrying would risk a duplicated line, so the
                // shell latches the message until a human resolves it.
                InlineCommitReport::Latched { evidence } => {
                    println!("[commit {event}] undecidable: {evidence}\r");
                    println!("[commit {event}] resolve it with InlineChatShell::resolve\r");
                }
            }
        }

        assert!(
            shell.live_state(id).is_none(),
            "a confirmed commit is what removes a message from the live region"
        );
    }

    println!("\r");
    println!(
        "live region now holds: the composer, and {} message(s)\r",
        shell.live_messages().len()
    );
    println!(
        "composer draft survived every commit: {:?}\r",
        shell.composer().text()
    );
    Ok(())
}
