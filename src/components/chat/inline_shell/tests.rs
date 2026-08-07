//! Lifecycle tests: what stays in the live region, and what leaves it.

use std::io::{self, Write};

use super::*;
use crate::components::chat::scrollback::{AttemptDisposition, NativeTerminalSink, ThemeIdentity};
use crate::hooks::Key;

/// A writer that accepts a fixed byte budget and then fails.
#[derive(Debug, Default)]
struct BudgetedWriter {
    accepted: Vec<u8>,
    budget: Option<usize>,
}

impl BudgetedWriter {
    const fn unlimited() -> Self {
        Self {
            accepted: Vec::new(),
            budget: None,
        }
    }

    const fn accepting(budget: usize) -> Self {
        Self {
            accepted: Vec::new(),
            budget: Some(budget),
        }
    }
}

impl Write for BudgetedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(budget) = self.budget else {
            self.accepted.extend_from_slice(buf);
            return Ok(buf.len());
        };
        let room = budget.saturating_sub(self.accepted.len());
        if room == 0 {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "budget exhausted",
            ));
        }
        let take = room.min(buf.len());
        self.accepted.extend_from_slice(&buf[..take]);
        Ok(take)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

type TestShell = InlineChatShell<NativeTerminalSink<BudgetedWriter>>;

fn shell(writer: BudgetedWriter) -> TestShell {
    InlineChatShell::new(
        ScrollbackNamespace::new("inline-test").expect("non-empty"),
        NativeTerminalSink::new(writer),
    )
}

fn context() -> ProjectionContext {
    ProjectionContext::new(80, ThemeIdentity::new(1)).expect("non-zero width")
}

fn finish(shell: &mut TestShell, id: u64, text: &str) -> InlineCommitReport {
    shell
        .finish(
            MessageId::new(id),
            MessageRevision::INITIAL,
            text,
            context(),
        )
        .expect("the message is not latched")
}

#[test]
fn a_streaming_message_stays_in_the_live_region() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");

    assert_eq!(
        shell.live_state(MessageId::new(1)),
        Some(LiveState::Streaming)
    );
    // Nothing has been committed: a streaming message has no final content.
    assert_eq!(shell.sink().ledger().len(), 0);
}

#[test]
fn streaming_the_same_message_twice_is_refused_rather_than_double_tracked() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");

    assert_eq!(
        shell.stream(MessageId::new(1)),
        Err(InlineShellError::AlreadyLive {
            id: MessageId::new(1)
        })
    );
    assert_eq!(shell.live_messages().len(), 1);
}

#[test]
fn a_confirmed_commit_is_what_removes_a_message_from_the_live_region() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");

    let report = finish(&mut shell, 1, "the answer");

    assert!(report.left_live_region());
    assert!(shell.live_messages().is_empty());
    assert_eq!(shell.sink().ledger().len(), 1);
}

#[test]
fn a_repeated_terminal_event_does_not_commit_a_second_line() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");
    finish(&mut shell, 1, "the answer");

    for _ in 0..8 {
        let repeat = finish(&mut shell, 1, "the answer");
        let InlineCommitReport::Fixed { disposition, .. } = repeat else {
            panic!("a repeated terminal event must still report a confirmed commit");
        };
        assert_eq!(disposition, AttemptDisposition::AlreadyCommitted);
    }
    assert_eq!(shell.sink().ledger().len(), 1);
}

#[test]
fn a_burst_of_deltas_commits_nothing_at_all() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");

    // Deltas take no shell call by design: a streaming message has no final
    // content, so there is nothing a delta could commit.
    for _ in 0..1024 {
        assert_eq!(
            shell.live_state(MessageId::new(1)),
            Some(LiveState::Streaming)
        );
    }
    assert_eq!(shell.sink().ledger().len(), 0);
}

#[test]
fn a_clean_refusal_keeps_the_message_live_and_retryable() {
    let mut shell = shell(BudgetedWriter::accepting(0));
    shell.stream(MessageId::new(1)).expect("not yet live");

    let report = finish(&mut shell, 1, "the answer");

    assert!(matches!(report, InlineCommitReport::Retained { .. }));
    assert!(!report.left_live_region());
    assert_eq!(
        shell.live_state(MessageId::new(1)),
        Some(LiveState::AwaitingRetry)
    );
}

#[test]
fn an_undecidable_commit_latches_the_message_against_further_attempts() {
    let mut shell = shell(BudgetedWriter::accepting(3));
    shell.stream(MessageId::new(1)).expect("not yet live");

    let report = finish(&mut shell, 1, "the answer");
    assert!(matches!(report, InlineCommitReport::Latched { .. }));
    assert_eq!(
        shell.live_state(MessageId::new(1)),
        Some(LiveState::AwaitingResolution)
    );

    // The decisive assertion: the shell refuses to try again on its own, because
    // some of the bytes may already be on screen.
    let refused = shell.finish(
        MessageId::new(1),
        MessageRevision::INITIAL,
        "the answer",
        context(),
    );
    assert!(matches!(
        refused,
        Err(InlineShellError::AwaitingResolution { id }) if id == MessageId::new(1)
    ));
}

#[test]
fn a_message_that_never_streamed_is_still_latched_when_its_commit_is_undecidable() {
    let mut shell = shell(BudgetedWriter::accepting(3));

    let report = finish(&mut shell, 9, "one-shot");

    assert!(matches!(report, InlineCommitReport::Latched { .. }));
    assert_eq!(
        shell.live_state(MessageId::new(9)),
        Some(LiveState::AwaitingResolution)
    );
}

#[test]
fn resolving_a_latch_as_visible_drops_it_without_rewriting() {
    let mut shell = shell(BudgetedWriter::accepting(3));
    shell.stream(MessageId::new(1)).expect("not yet live");
    finish(&mut shell, 1, "the answer");
    let bytes_before = shell.sink().ledger().len();

    let state = shell
        .resolve(MessageId::new(1), UnknownResolution::AlreadyVisible)
        .expect("the message is latched");

    assert_eq!(state, None);
    assert!(shell.live_messages().is_empty());
    assert_eq!(shell.sink().ledger().len(), bytes_before);
}

#[test]
fn resolving_a_latch_as_not_visible_permits_exactly_one_more_attempt() {
    let mut shell = shell(BudgetedWriter::accepting(3));
    shell.stream(MessageId::new(1)).expect("not yet live");
    finish(&mut shell, 1, "the answer");

    let state = shell
        .resolve(MessageId::new(1), UnknownResolution::NotVisible)
        .expect("the message is latched");

    assert_eq!(state, Some(LiveState::AwaitingRetry));
    // The attempt is now permitted rather than refused.
    let retried = shell.finish(
        MessageId::new(1),
        MessageRevision::INITIAL,
        "the answer",
        context(),
    );
    assert!(retried.is_ok());
}

#[test]
fn resolving_a_message_that_is_not_latched_is_refused() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");

    assert_eq!(
        shell.resolve(MessageId::new(1), UnknownResolution::AlreadyVisible),
        Err(InlineShellError::NotLive {
            id: MessageId::new(1)
        })
    );
}

#[test]
fn content_that_cannot_be_committed_is_reported_rather_than_sanitised() {
    let mut shell = shell(BudgetedWriter::unlimited());
    shell.stream(MessageId::new(1)).expect("not yet live");

    let refused = shell.finish(
        MessageId::new(1),
        MessageRevision::INITIAL,
        "before\u{1b}[2Jafter",
        context(),
    );

    assert!(matches!(refused, Err(InlineShellError::Content(_))));
    // Still live: nothing was committed, so nothing may be removed.
    assert_eq!(
        shell.live_state(MessageId::new(1)),
        Some(LiveState::Streaming)
    );
}

#[test]
fn the_composer_keeps_its_draft_across_a_commit() {
    let mut shell = shell(BudgetedWriter::unlimited());
    let keymap = ChatComposerKeyMap::new();
    shell.handle_key(&keymap, "draft", &Key::default());
    shell.stream(MessageId::new(1)).expect("not yet live");

    finish(&mut shell, 1, "the answer");

    // The composer never leaves the live region, and a commit does not touch it.
    assert_eq!(shell.composer().text(), "draft");
}

#[test]
fn submitting_reports_the_text_without_clearing_the_draft() {
    let mut shell = shell(BudgetedWriter::unlimited());
    let keymap = ChatComposerKeyMap::new();
    shell.handle_key(&keymap, "hello", &Key::default());

    let enter = Key {
        return_key: true,
        ..Key::default()
    };
    let outcome = shell.handle_key(&keymap, "", &enter);

    assert_eq!(outcome, InlineKeyOutcome::Submitted("hello".to_owned()));
    assert_eq!(shell.composer().text(), "hello");
}

#[test]
fn cancelling_is_reported_and_leaves_the_draft_intact() {
    let mut shell = shell(BudgetedWriter::unlimited());
    let keymap = ChatComposerKeyMap::new();
    shell.handle_key(&keymap, "a long message", &Key::default());

    let escape = Key {
        escape: true,
        ..Key::default()
    };
    let outcome = shell.handle_key(&keymap, "", &escape);

    assert_eq!(outcome, InlineKeyOutcome::Cancelled);
    assert_eq!(shell.composer().text(), "a long message");
}

#[test]
fn keys_do_not_reach_the_composer_while_the_transcript_holds_focus() {
    let mut shell = shell(BudgetedWriter::unlimited());
    let keymap = ChatComposerKeyMap::new();
    assert_eq!(
        shell.set_focus(InlineFocus::Transcript),
        InlineFocusOutcome::Moved(InlineFocus::Transcript)
    );

    let outcome = shell.handle_key(&keymap, "x", &Key::default());

    assert_eq!(outcome, InlineKeyOutcome::NotFocused);
    assert_eq!(shell.composer().text(), "");
}

#[test]
fn moving_focus_where_it_already_rests_reports_no_change() {
    let mut shell = shell(BudgetedWriter::unlimited());
    assert_eq!(
        shell.set_focus(InlineFocus::Composer),
        InlineFocusOutcome::Unchanged(InlineFocus::Composer)
    );
    assert_eq!(shell.focus(), InlineFocus::Composer);
}

#[test]
fn a_resized_terminal_makes_a_finished_message_a_distinct_commit() {
    let mut shell = shell(BudgetedWriter::unlimited());
    let wide = ProjectionContext::new(120, ThemeIdentity::new(1)).expect("non-zero width");
    shell.stream(MessageId::new(1)).expect("not yet live");
    finish(&mut shell, 1, "the answer");

    // A committed line cannot be re-flowed, so the same text at a new width is a
    // different commit rather than a silent reuse of the earlier one.
    let second = shell
        .finish(
            MessageId::new(1),
            MessageRevision::new(2).expect("non-zero"),
            "the answer",
            wide,
        )
        .expect("not latched");

    assert!(second.left_live_region());
    assert_eq!(shell.sink().ledger().len(), 2);
}
