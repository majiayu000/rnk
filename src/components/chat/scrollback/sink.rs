//! The commit boundary between the live region and the terminal's scrollback.
//!
//! A sink is the only thing in this crate allowed to move a message across that
//! boundary, and [`ScrollbackSink::commit`] is deliberately shaped so that a
//! caller cannot report success it did not observe: the return type is the
//! three-state [`ScrollbackCommitOutcome`], not a `Result<(), _>` whose `Ok`
//! would quietly absorb a partial write.
//!
//! # Ordering
//!
//! [`NativeTerminalSink`] resolves a commit in a fixed order, and the order is
//! the correctness argument:
//!
//! 1. **Closed?** A shut-down sink accepts nothing.
//! 2. **Does the identity match the bytes?** A commit whose declared content
//!    disagrees with what it carries is refused before any byte is written.
//! 3. **Already confirmed?** Return the *original* receipt with an
//!    [`AlreadyCommitted`] disposition. This is the dedup that makes a repeated
//!    completion event, a repeated render, or a burst of deltas harmless.
//! 4. **Conflicting?** Refuse. The earlier line is already in the terminal and
//!    cannot be rewritten, so a disagreement is surfaced rather than papered
//!    over.
//! 5. **Is there ledger room?** Checked *before* writing. Writing first and
//!    discovering afterwards that the commit cannot be recorded would leave the
//!    terminal ahead of the ledger — a permanently [`Unknown`] commit, from a
//!    condition that was knowable in advance.
//! 6. **Write, flush, record.** Only when all three succeed is the outcome
//!    [`Committed`].
//!
//! [`AlreadyCommitted`]: AttemptDisposition::AlreadyCommitted
//! [`Committed`]: ScrollbackCommitOutcome::Committed
//! [`Unknown`]: ScrollbackCommitOutcome::Unknown

use std::io::{self, Write};
use std::num::NonZeroUsize;

use super::content::{ScrollbackContent, TransportStage};
use super::identity::ScrollbackCommitId;
use super::ledger::{ConfirmedLedger, LedgerLookup, LedgerRecordError};
use super::outcome::{
    AttemptDisposition, NotCommittedCause, ScrollbackCommitOutcome, ScrollbackGuarantee,
    UnknownEvidence, UnknownReason,
};

/// The default confirmed-ledger capacity for [`NativeTerminalSink::new`].
///
/// Bounded because the ledger never evicts. A session that commits more than
/// this many distinct messages gets a visible [`LedgerAtCapacity`] refusal
/// rather than a silently forgotten commit; callers who expect longer sessions
/// pass their own capacity to [`NativeTerminalSink::with_capacity`].
///
/// [`LedgerAtCapacity`]: NotCommittedCause::LedgerAtCapacity
pub const DEFAULT_LEDGER_CAPACITY: usize = 4096;

/// A destination that fixes finished transcript into scrollback exactly once.
///
/// Implementors carry the whole burden of the guarantee they advertise through
/// [`guarantee`]. A sink that returns [`DurableAtomicIdempotency`] is promising
/// that its query-and-record step is one atomic durable transaction; if it is
/// two steps, a crash between them produces the duplicate line the type system
/// was asked to prevent.
///
/// [`guarantee`]: Self::guarantee
/// [`DurableAtomicIdempotency`]: ScrollbackGuarantee::DurableAtomicIdempotency
pub trait ScrollbackSink {
    /// Returns what this sink promises about commits that outlive the process.
    fn guarantee(&self) -> ScrollbackGuarantee;

    /// Commits `content` under `commit_id`, or explains why it did not.
    ///
    /// Committing the same `commit_id` twice must produce the same receipt and
    /// write nothing the second time. Callers may only remove a message from the
    /// live region when the outcome [permits it].
    ///
    /// [permits it]: ScrollbackCommitOutcome::permits_live_removal
    fn commit(
        &mut self,
        commit_id: &ScrollbackCommitId,
        content: &ScrollbackContent,
    ) -> ScrollbackCommitOutcome;
}

/// Writes commits straight to the terminal, deduplicating within this process.
///
/// This is the default sink, and its guarantee is the honest ceiling for a plain
/// terminal write: [`ProcessLocalConfirmed`]. The write reaching the terminal
/// and the ledger recording it are two separate events with no transaction
/// around them, so a crash landing between them leaves a line on screen that no
/// restarted process can know about. Callers who need more supply a sink backed
/// by a store that can do both atomically.
///
/// [`ProcessLocalConfirmed`]: ScrollbackGuarantee::ProcessLocalConfirmed
#[derive(Debug)]
pub struct NativeTerminalSink<W: Write> {
    writer: W,
    ledger: ConfirmedLedger,
    closed: bool,
}

impl<W: Write> NativeTerminalSink<W> {
    /// Creates a sink over `writer` with [`DEFAULT_LEDGER_CAPACITY`].
    pub fn new(writer: W) -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_LEDGER_CAPACITY)
            .expect("DEFAULT_LEDGER_CAPACITY is a nonzero constant");
        Self::with_capacity(writer, capacity)
    }

    /// Creates a sink over `writer` with an explicit ledger capacity.
    pub fn with_capacity(writer: W, capacity: NonZeroUsize) -> Self {
        Self {
            writer,
            ledger: ConfirmedLedger::new(capacity, ScrollbackGuarantee::ProcessLocalConfirmed),
            closed: false,
        }
    }

    /// Returns the confirmed-commit ledger.
    pub const fn ledger(&self) -> &ConfirmedLedger {
        &self.ledger
    }

    /// Reports whether the sink has been shut down.
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Shuts the sink down; every later commit is refused as [`SinkClosed`].
    ///
    /// Flushing is attempted and its error returned, because a dropped buffered
    /// write is a transcript line the caller believes it committed.
    ///
    /// [`SinkClosed`]: NotCommittedCause::SinkClosed
    pub fn close(&mut self) -> io::Result<()> {
        self.closed = true;
        self.writer.flush()
    }

    /// Writes one stage, reporting how many of its bytes the terminal accepted.
    ///
    /// `already_accepted` is the running total across earlier stages, and it is
    /// what decides whether a stop is [`NotCommitted`] or [`Unknown`]: only a
    /// stop at a running total of zero can be retried.
    ///
    /// [`NotCommitted`]: ScrollbackCommitOutcome::NotCommitted
    /// [`Unknown`]: ScrollbackCommitOutcome::Unknown
    fn write_stage(
        &mut self,
        stage: TransportStage,
        bytes: &[u8],
        already_accepted: usize,
    ) -> Result<usize, ScrollbackCommitOutcome> {
        let mut written = 0;
        while written < bytes.len() {
            match self.writer.write(&bytes[written..]) {
                Ok(0) => {
                    let accepted = already_accepted + written;
                    return Err(stopped(
                        stage,
                        accepted,
                        UnknownReason::WriteStalledAfterAccept,
                        io::Error::new(
                            io::ErrorKind::WriteZero,
                            "the terminal accepted no further bytes",
                        ),
                    ));
                }
                Ok(count) => written += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    let accepted = already_accepted + written;
                    return Err(stopped(
                        stage,
                        accepted,
                        UnknownReason::WriteFailedAfterAccept,
                        error,
                    ));
                }
            }
        }
        Ok(written)
    }
}

/// Classifies a stop by how many bytes the terminal is known to have taken.
///
/// Zero accepted bytes is the only provably clean state, and the only one a
/// caller may retry. Anything else is undecidable from inside this process.
fn stopped(
    stage: TransportStage,
    accepted: usize,
    reason: UnknownReason,
    error: io::Error,
) -> ScrollbackCommitOutcome {
    if accepted == 0 {
        ScrollbackCommitOutcome::NotCommitted {
            cause: NotCommittedCause::FirstWriteRejected {
                stage,
                source: error,
            },
        }
    } else {
        ScrollbackCommitOutcome::Unknown {
            evidence: UnknownEvidence::new(stage, accepted, reason, Some(error)),
        }
    }
}

impl<W: Write> ScrollbackSink for NativeTerminalSink<W> {
    fn guarantee(&self) -> ScrollbackGuarantee {
        ScrollbackGuarantee::ProcessLocalConfirmed
    }

    fn commit(
        &mut self,
        commit_id: &ScrollbackCommitId,
        content: &ScrollbackContent,
    ) -> ScrollbackCommitOutcome {
        if self.closed {
            return ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::SinkClosed,
            };
        }
        if let Some(mismatch) = declaration_mismatch(commit_id, content) {
            return mismatch;
        }
        match self.ledger.lookup(commit_id) {
            LedgerLookup::Confirmed(receipt) => {
                return ScrollbackCommitOutcome::Committed {
                    receipt: receipt.clone(),
                    disposition: AttemptDisposition::AlreadyCommitted,
                };
            }
            LedgerLookup::Conflict { confirmed } => {
                return ScrollbackCommitOutcome::NotCommitted {
                    cause: NotCommittedCause::IdentityConflict {
                        confirmed,
                        presented: content.identity(),
                    },
                };
            }
            LedgerLookup::Absent => {}
        }
        // Checked before writing: a commit written but unrecordable is stuck as
        // Unknown forever, and this particular cause is knowable in advance.
        if !self.ledger.has_room() {
            return ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::LedgerAtCapacity {
                    capacity: self.ledger.capacity().get(),
                },
            };
        }

        let encoding = content.encode();
        let mut accepted = 0;
        for (stage, bytes) in encoding.stages() {
            match self.write_stage(stage, bytes, accepted) {
                Ok(written) => accepted += written,
                Err(outcome) => return outcome,
            }
        }
        if let Err(error) = self.writer.flush() {
            return ScrollbackCommitOutcome::Unknown {
                evidence: UnknownEvidence::new(
                    TransportStage::Delimiter,
                    accepted,
                    UnknownReason::FlushFailed,
                    Some(error),
                ),
            };
        }
        match self.ledger.record(commit_id.clone()) {
            Ok(receipt) => ScrollbackCommitOutcome::Committed {
                receipt,
                disposition: AttemptDisposition::Written,
            },
            // The bytes are on screen and the process cannot prove it. Reported
            // as Unknown rather than Committed: a receipt implies a record.
            Err(error) => ScrollbackCommitOutcome::Unknown {
                evidence: UnknownEvidence::new(
                    TransportStage::Delimiter,
                    accepted,
                    UnknownReason::LedgerNotRecorded,
                    Some(io::Error::other(LedgerFailure(error))),
                ),
            },
        }
    }
}

/// Refuses a commit whose identity does not describe the bytes it carries.
///
/// Shared by every sink, because the check protects the ledger rather than the
/// terminal: dedup compares identities, so an identity that lies about its
/// content lets two different lines occupy one entry.
pub(super) fn declaration_mismatch(
    commit_id: &ScrollbackCommitId,
    content: &ScrollbackContent,
) -> Option<ScrollbackCommitOutcome> {
    if commit_id.content() == content.identity() && commit_id.context() == content.context() {
        return None;
    }
    Some(ScrollbackCommitOutcome::NotCommitted {
        cause: NotCommittedCause::DeclaredContentMismatch {
            declared: commit_id.content(),
            presented: content.identity(),
        },
    })
}

/// Carries a [`LedgerRecordError`] through [`io::Error::other`].
#[derive(Debug)]
struct LedgerFailure(LedgerRecordError);

impl std::fmt::Display for LedgerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for LedgerFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[cfg(test)]
pub(super) mod harness {
    //! Writers that stop where a real terminal would, on demand.

    use std::io::{self, Write};

    use super::super::content::ScrollbackContent;
    use super::super::identity::{
        ProjectionContext, ScrollbackCommitId, ScrollbackCommitKey, ScrollbackNamespace,
        ThemeIdentity,
    };
    use crate::components::chat::{MessageId, MessageRevision};

    /// A writer that accepts a fixed number of bytes and then fails.
    ///
    /// The byte budget is what makes the three-state outcome testable: a budget
    /// of zero is a provably clean refusal, and any nonzero budget short of the
    /// full encoding is the partial write that must never be retried.
    #[derive(Debug)]
    pub(in super::super) struct BudgetedWriter {
        accepted: Vec<u8>,
        budget: usize,
        stall_instead_of_failing: bool,
        flush_fails: bool,
    }

    impl BudgetedWriter {
        pub(in super::super) const fn new(budget: usize) -> Self {
            Self {
                accepted: Vec::new(),
                budget,
                stall_instead_of_failing: false,
                flush_fails: false,
            }
        }

        pub(in super::super) const fn unlimited() -> Self {
            Self::new(usize::MAX)
        }

        pub(in super::super) const fn stalling(budget: usize) -> Self {
            let mut writer = Self::new(budget);
            writer.stall_instead_of_failing = true;
            writer
        }

        pub(in super::super) const fn failing_flush() -> Self {
            let mut writer = Self::unlimited();
            writer.flush_fails = true;
            writer
        }

        pub(in super::super) fn accepted(&self) -> &[u8] {
            &self.accepted
        }

        pub(in super::super) fn transcript(&self) -> String {
            String::from_utf8_lossy(&self.accepted).into_owned()
        }
    }

    impl Write for BudgetedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let room = self.budget.saturating_sub(self.accepted.len());
            if room == 0 {
                if self.stall_instead_of_failing {
                    return Ok(0);
                }
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
            if self.flush_fails {
                return Err(io::Error::other("flush refused"));
            }
            Ok(())
        }
    }

    pub(in super::super) fn context() -> ProjectionContext {
        ProjectionContext::new(80, ThemeIdentity::new(1)).expect("non-zero width")
    }

    pub(in super::super) fn content(text: &str) -> ScrollbackContent {
        ScrollbackContent::try_new(text, context()).expect("printable content")
    }

    /// Builds the identity a caller would derive for `text` at `message`.
    pub(in super::super) fn commit_id(message: u64, text: &str) -> ScrollbackCommitId {
        let key = ScrollbackCommitKey::new(
            ScrollbackNamespace::new("test").expect("non-empty"),
            MessageId::new(message),
            MessageRevision::INITIAL,
        );
        let content = content(text);
        ScrollbackCommitId::new(key, content.identity(), context())
    }
}

#[cfg(test)]
mod tests {
    use super::harness::{BudgetedWriter, commit_id, content, context};
    use super::*;
    use crate::components::chat::scrollback::identity::{
        ScrollbackCommitKey, ScrollbackContentIdentity, ScrollbackNamespace,
    };
    use crate::components::chat::scrollback::outcome::ScrollbackReceipt;
    use crate::components::chat::{MessageId, MessageRevision};

    fn sink(writer: BudgetedWriter) -> NativeTerminalSink<BudgetedWriter> {
        NativeTerminalSink::new(writer)
    }

    #[test]
    fn a_completed_message_is_written_once_with_a_reset_and_a_crlf() {
        let mut sink = sink(BudgetedWriter::unlimited());
        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::Committed {
                disposition: AttemptDisposition::Written,
                ..
            }
        ));
        assert!(outcome.permits_live_removal());
        assert_eq!(sink.get_ref_transcript(), "hello\u{1b}[0m\r\n");
    }

    #[test]
    fn a_canonical_newline_reaches_the_terminal_as_crlf() {
        let mut sink = sink(BudgetedWriter::unlimited());
        sink.commit(&commit_id(1, "a\nb"), &content("a\nb"));

        assert_eq!(sink.get_ref_transcript(), "a\r\nb\u{1b}[0m\r\n");
    }

    #[test]
    fn a_repeated_terminal_event_does_not_write_a_second_line() {
        let mut sink = sink(BudgetedWriter::unlimited());
        let id = commit_id(1, "hello");
        let first = sink.commit(&id, &content("hello"));
        let after_first = sink.get_ref_transcript();

        for _ in 0..16 {
            let repeat = sink.commit(&id, &content("hello"));
            assert!(matches!(
                repeat,
                ScrollbackCommitOutcome::Committed {
                    disposition: AttemptDisposition::AlreadyCommitted,
                    ..
                }
            ));
            // The original receipt, not a new one: a second receipt would imply
            // a second transcript line.
            assert_eq!(
                repeat.receipt().map(ScrollbackReceipt::sequence),
                first.receipt().map(ScrollbackReceipt::sequence)
            );
        }
        assert_eq!(sink.get_ref_transcript(), after_first);
        assert_eq!(sink.ledger().len(), 1);
    }

    #[test]
    fn a_burst_of_deltas_before_completion_commits_exactly_one_line() {
        let mut sink = sink(BudgetedWriter::unlimited());
        // A streaming message is only ever committed at its terminal revision,
        // so every delta presents the same identity once it settles.
        let id = commit_id(7, "streamed answer");
        let mut written = 0;
        for _ in 0..256 {
            let outcome = sink.commit(&id, &content("streamed answer"));
            if matches!(
                outcome,
                ScrollbackCommitOutcome::Committed {
                    disposition: AttemptDisposition::Written,
                    ..
                }
            ) {
                written += 1;
            }
        }
        assert_eq!(written, 1);
        assert_eq!(
            sink.get_ref_transcript().matches("streamed answer").count(),
            1
        );
    }

    #[test]
    fn a_later_revision_of_the_same_message_is_a_separate_line() {
        let mut sink = sink(BudgetedWriter::unlimited());
        let namespace = ScrollbackNamespace::new("test").expect("non-empty");
        let edited = content("edited");
        let second = ScrollbackCommitId::new(
            ScrollbackCommitKey::new(
                namespace,
                MessageId::new(1),
                MessageRevision::new(2).expect("non-zero"),
            ),
            edited.identity(),
            context(),
        );

        sink.commit(&commit_id(1, "first"), &content("first"));
        let outcome = sink.commit(&second, &edited);

        assert!(outcome.permits_live_removal());
        assert_eq!(sink.ledger().len(), 2);
    }

    #[test]
    fn the_same_key_with_different_content_fails_closed() {
        let mut sink = sink(BudgetedWriter::unlimited());
        sink.commit(&commit_id(1, "original"), &content("original"));

        let replacement = content("rewritten");
        let conflicting = ScrollbackCommitId::new(
            ScrollbackCommitKey::new(
                ScrollbackNamespace::new("test").expect("non-empty"),
                MessageId::new(1),
                MessageRevision::INITIAL,
            ),
            replacement.identity(),
            context(),
        );
        let outcome = sink.commit(&conflicting, &replacement);

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::IdentityConflict { .. }
            }
        ));
        assert!(!sink.get_ref_transcript().contains("rewritten"));
    }

    #[test]
    fn an_identity_that_disagrees_with_its_bytes_is_refused_before_any_write() {
        let mut sink = sink(BudgetedWriter::unlimited());
        let lying = ScrollbackCommitId::new(
            ScrollbackCommitKey::new(
                ScrollbackNamespace::new("test").expect("non-empty"),
                MessageId::new(1),
                MessageRevision::INITIAL,
            ),
            ScrollbackContentIdentity::derive("something else", context()),
            context(),
        );
        let outcome = sink.commit(&lying, &content("actual bytes"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::DeclaredContentMismatch { .. }
            }
        ));
        assert!(sink.get_ref_transcript().is_empty());
    }

    #[test]
    fn a_closed_sink_refuses_without_writing_and_the_refusal_is_retryable() {
        let mut sink = sink(BudgetedWriter::unlimited());
        sink.close().expect("flush succeeds");
        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::SinkClosed
            }
        ));
        assert!(outcome.permits_retry());
        assert!(!outcome.permits_live_removal());
        assert!(sink.get_ref_transcript().is_empty());
    }

    #[test]
    fn a_rejected_first_byte_is_not_committed_rather_than_unknown() {
        let mut sink = sink(BudgetedWriter::new(0));
        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::FirstWriteRejected { .. }
            }
        ));
        // Provably zero bytes accepted, so a retry cannot duplicate anything.
        assert!(outcome.permits_retry());
        assert_eq!(sink.ledger().len(), 0);
    }

    #[test]
    fn a_partial_write_is_unknown_and_must_not_be_retried() {
        let mut sink = sink(BudgetedWriter::new(3));
        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        let ScrollbackCommitOutcome::Unknown { evidence } = &outcome else {
            panic!("expected Unknown, got {outcome:?}");
        };
        assert_eq!(evidence.accepted_transport_bytes(), 3);
        assert_eq!(evidence.reason(), UnknownReason::WriteFailedAfterAccept);
        assert!(!outcome.permits_retry());
        assert!(!outcome.permits_live_removal());
        // Never recorded, so the commit stays visible as unresolved.
        assert_eq!(sink.ledger().len(), 0);
    }

    #[test]
    fn a_stalled_write_after_progress_is_distinguished_from_a_failure() {
        let mut sink = sink(BudgetedWriter::stalling(2));
        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        let ScrollbackCommitOutcome::Unknown { evidence } = &outcome else {
            panic!("expected Unknown, got {outcome:?}");
        };
        assert_eq!(evidence.reason(), UnknownReason::WriteStalledAfterAccept);
        assert_eq!(evidence.accepted_transport_bytes(), 2);
    }

    #[test]
    fn a_failed_flush_leaves_the_commit_unknown() {
        let mut sink = sink(BudgetedWriter::failing_flush());
        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        let ScrollbackCommitOutcome::Unknown { evidence } = &outcome else {
            panic!("expected Unknown, got {outcome:?}");
        };
        assert_eq!(evidence.reason(), UnknownReason::FlushFailed);
        assert_eq!(evidence.stage(), TransportStage::Delimiter);
        assert_eq!(sink.ledger().len(), 0);
    }

    #[test]
    fn a_full_ledger_refuses_before_writing_rather_than_after() {
        let capacity = NonZeroUsize::new(1).expect("non-zero");
        let mut sink = NativeTerminalSink::with_capacity(BudgetedWriter::unlimited(), capacity);
        sink.commit(&commit_id(1, "first"), &content("first"));

        let outcome = sink.commit(&commit_id(2, "second"), &content("second"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::LedgerAtCapacity { capacity: 1 }
            }
        ));
        // The decisive assertion: nothing reached the terminal, so this refusal
        // is retryable instead of becoming a permanently Unknown commit.
        assert!(!sink.get_ref_transcript().contains("second"));
        assert!(outcome.permits_retry());
    }

    #[test]
    fn the_native_sink_never_claims_more_than_process_local_dedup() {
        let sink = sink(BudgetedWriter::unlimited());
        assert_eq!(sink.guarantee(), ScrollbackGuarantee::ProcessLocalConfirmed);
    }

    impl NativeTerminalSink<BudgetedWriter> {
        fn get_ref_transcript(&self) -> String {
            self.writer.transcript()
        }
    }

    #[test]
    fn accepted_bytes_are_exactly_the_encoded_stages() {
        let mut sink = sink(BudgetedWriter::unlimited());
        let body = content("hi");
        sink.commit(&commit_id(1, "hi"), &body);
        assert_eq!(sink.writer.accepted().len(), body.encode().total_len());
    }
}
