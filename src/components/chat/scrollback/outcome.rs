//! The closed three-state result of one scrollback commit.
//!
//! The states are not a success/failure pair with a third case bolted on. They
//! partition the only thing that matters after a terminal write: **how many of
//! this commit's bytes the terminal accepted.**
//!
//! | State | Accepted bytes | What the caller may do |
//! |---|---|---|
//! | [`Committed`] | all of them, flushed, ledger recorded | remove the message from the live region |
//! | [`NotCommitted`] | provably zero | retry, once the caller asks |
//! | [`Unknown`] | somewhere in between, or unknowable | neither, until a human resolves it |
//!
//! [`Unknown`] is the state a naive design omits, and omitting it is what
//! produces duplicated transcript lines: a partial write that is retried writes
//! its accepted prefix twice. There is no automatic recovery from it, because
//! nothing inside this process can observe what the terminal already showed.
//!
//! [`Committed`]: ScrollbackCommitOutcome::Committed
//! [`NotCommitted`]: ScrollbackCommitOutcome::NotCommitted
//! [`Unknown`]: ScrollbackCommitOutcome::Unknown

use std::fmt;
use std::io;
use std::num::NonZeroU64;

use super::content::TransportStage;
use super::identity::{ScrollbackCommitId, ScrollbackContentIdentity};

/// What a sink promises about commits that outlive the current process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScrollbackGuarantee {
    /// Duplicate commits are suppressed only while this process lives.
    ///
    /// The native terminal sink can promise no more: a write reaching the
    /// terminal and the in-process ledger recording it are two separate events,
    /// and a crash can land between them.
    ProcessLocalConfirmed,
    /// The sink queries and records a commit in one atomic durable transaction.
    ///
    /// Only a sink that can do this may return an already-committed result
    /// across restarts.
    DurableAtomicIdempotency,
}

impl fmt::Display for ScrollbackGuarantee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ProcessLocalConfirmed => "process-local confirmed dedup",
            Self::DurableAtomicIdempotency => "durable atomic idempotency",
        })
    }
}

/// Proof that a specific commit reached the terminal.
///
/// A receipt is minted once, by the attempt that actually wrote. Every later
/// observation of the same commit returns that same original receipt — a
/// duplicate attempt never mints a second one, because a second receipt would
/// imply a second line in the transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackReceipt {
    sequence: NonZeroU64,
    commit_id: ScrollbackCommitId,
    guarantee: ScrollbackGuarantee,
}

impl ScrollbackReceipt {
    pub(super) const fn new(
        sequence: NonZeroU64,
        commit_id: ScrollbackCommitId,
        guarantee: ScrollbackGuarantee,
    ) -> Self {
        Self {
            sequence,
            commit_id,
            guarantee,
        }
    }

    /// Returns this commit's position in the sink's confirmed order.
    pub const fn sequence(&self) -> NonZeroU64 {
        self.sequence
    }

    /// Returns the identity that was committed.
    pub const fn commit_id(&self) -> &ScrollbackCommitId {
        &self.commit_id
    }

    /// Returns the guarantee the issuing sink operates under.
    pub const fn guarantee(&self) -> ScrollbackGuarantee {
        self.guarantee
    }
}

impl fmt::Display for ScrollbackReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "commit #{} of {} under {}",
            self.sequence,
            self.commit_id.key(),
            self.guarantee
        )
    }
}

/// What *this particular attempt* did, as distinct from what the commit is.
///
/// A confirmed commit observed five times yields one receipt and five
/// dispositions. Collapsing the two is how duplicate-suppression bugs hide:
/// the caller cannot tell "already done" from "just done" and re-runs side
/// effects that should have happened once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttemptDisposition {
    /// This attempt performed the terminal write.
    Written,
    /// The commit was already confirmed; this attempt wrote nothing.
    AlreadyCommitted,
}

impl fmt::Display for AttemptDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Written => "written by this attempt",
            Self::AlreadyCommitted => "already committed by an earlier attempt",
        })
    }
}

/// The closed set of commit results.
///
/// Deliberately neither `Clone` nor `PartialEq`: the failure states carry the
/// originating [`io::Error`], and discarding it to gain those impls would throw
/// away the only evidence of what the terminal actually did.
#[derive(Debug)]
#[non_exhaustive]
pub enum ScrollbackCommitOutcome {
    /// Every transport byte was accepted, flushed, and recorded.
    Committed {
        /// The original receipt for this commit.
        receipt: ScrollbackReceipt,
        /// What this attempt contributed.
        disposition: AttemptDisposition,
    },
    /// Provably zero of this commit's transport bytes were accepted.
    NotCommitted {
        /// Why nothing was written.
        cause: NotCommittedCause,
    },
    /// The accepted byte count is nonzero or unknowable.
    Unknown {
        /// What was observed before the outcome became undecidable.
        evidence: UnknownEvidence,
    },
}

impl ScrollbackCommitOutcome {
    /// Returns the receipt when the commit is confirmed.
    pub const fn receipt(&self) -> Option<&ScrollbackReceipt> {
        match self {
            Self::Committed { receipt, .. } => Some(receipt),
            _ => None,
        }
    }

    /// Reports whether the message may leave the live region.
    ///
    /// Only [`Committed`] permits it. `Unknown` in particular does not: the
    /// message must stay visible precisely because nobody knows whether the
    /// terminal already has it.
    ///
    /// [`Committed`]: Self::Committed
    pub const fn permits_live_removal(&self) -> bool {
        matches!(self, Self::Committed { .. })
    }

    /// Reports whether a plain retry of the same identity is allowed.
    ///
    /// True only for [`NotCommitted`], where the byte count is provably zero.
    ///
    /// [`NotCommitted`]: Self::NotCommitted
    pub const fn permits_retry(&self) -> bool {
        matches!(self, Self::NotCommitted { .. })
    }
}

/// Every reason a commit can end with zero bytes accepted.
#[derive(Debug)]
#[non_exhaustive]
pub enum NotCommittedCause {
    /// The sink was shut down before the attempt.
    SinkClosed,
    /// The same key was already committed with different content or context.
    ///
    /// This fails closed: the earlier line is already in the terminal and
    /// nothing can rewrite it, so the disagreement is surfaced rather than
    /// resolved by overwriting.
    IdentityConflict {
        /// The content identity already confirmed under this key.
        confirmed: ScrollbackContentIdentity,
        /// The content identity this attempt presented.
        presented: ScrollbackContentIdentity,
    },
    /// The confirmed ledger is full.
    ///
    /// Reported rather than resolved by eviction: evicting a confirmed entry
    /// would let its commit be written to the terminal a second time.
    LedgerAtCapacity {
        /// The ledger's configured capacity.
        capacity: usize,
    },
    /// The commit identity declares content the presented bytes do not match.
    ///
    /// The identity is what dedup and conflict detection compare, so accepting a
    /// commit whose declared content disagrees with its actual bytes would let
    /// two different transcript lines share one ledger entry. Refused before any
    /// byte reaches the terminal.
    DeclaredContentMismatch {
        /// The content identity carried by the commit identity.
        declared: ScrollbackContentIdentity,
        /// The content identity derived from the bytes presented alongside it.
        presented: ScrollbackContentIdentity,
    },
    /// The very first write of the commit failed without accepting a byte.
    FirstWriteRejected {
        /// The stage that failed — always [`TransportStage::Body`] in practice,
        /// since a later stage implies earlier bytes were accepted.
        stage: TransportStage,
        /// The underlying I/O error.
        source: io::Error,
    },
}

impl fmt::Display for NotCommittedCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SinkClosed => f.write_str("the scrollback sink was already shut down"),
            Self::IdentityConflict {
                confirmed,
                presented,
            } => write!(
                f,
                "this key is already committed with content {confirmed}, but {presented} was presented"
            ),
            Self::LedgerAtCapacity { capacity } => write!(
                f,
                "the confirmed ledger is full at its capacity of {capacity} entries"
            ),
            Self::DeclaredContentMismatch {
                declared,
                presented,
            } => write!(
                f,
                "the commit identity declares content {declared}, but the presented bytes are {presented}"
            ),
            Self::FirstWriteRejected { stage, source } => {
                write!(
                    f,
                    "the terminal rejected the {stage} before any byte: {source}"
                )
            }
        }
    }
}

impl std::error::Error for NotCommittedCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FirstWriteRejected { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// What was observed before a commit's outcome became undecidable.
///
/// The accepted byte count is the important field: it is nonzero (or unknown),
/// which is exactly why a retry cannot be safe.
#[derive(Debug)]
pub struct UnknownEvidence {
    stage: TransportStage,
    accepted_transport_bytes: usize,
    reason: UnknownReason,
    source: Option<io::Error>,
}

impl UnknownEvidence {
    pub(super) const fn new(
        stage: TransportStage,
        accepted_transport_bytes: usize,
        reason: UnknownReason,
        source: Option<io::Error>,
    ) -> Self {
        Self {
            stage,
            accepted_transport_bytes,
            reason,
            source,
        }
    }

    /// Returns the stage the commit stopped in.
    pub const fn stage(&self) -> TransportStage {
        self.stage
    }

    /// Returns how many transport bytes the terminal is known to have accepted.
    pub const fn accepted_transport_bytes(&self) -> usize {
        self.accepted_transport_bytes
    }

    /// Returns why the outcome is undecidable.
    pub const fn reason(&self) -> UnknownReason {
        self.reason
    }

    /// Returns the underlying I/O error, when the stop had one.
    pub const fn source_error(&self) -> Option<&io::Error> {
        self.source.as_ref()
    }
}

impl fmt::Display for UnknownEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} after {} transport byte(s) accepted in the {}",
            self.reason, self.accepted_transport_bytes, self.stage
        )?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl std::error::Error for UnknownEvidence {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

/// Every way a commit's outcome can become undecidable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum UnknownReason {
    /// A write failed after earlier bytes of this commit were accepted.
    WriteFailedAfterAccept,
    /// A write reported zero progress after earlier bytes were accepted.
    ///
    /// Distinct from a failure: the stream neither advanced nor errored, so the
    /// commit is stuck with a partial prefix visible.
    WriteStalledAfterAccept,
    /// The flush failed, so accepted bytes may or may not have been displayed.
    FlushFailed,
    /// Every byte was written and flushed, but the ledger could not record it.
    ///
    /// The transcript line exists and the process cannot prove it does, which is
    /// the one case where the terminal is ahead of the ledger.
    LedgerNotRecorded,
}

impl fmt::Display for UnknownReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WriteFailedAfterAccept => "the write failed",
            Self::WriteStalledAfterAccept => "the write stalled without progress",
            Self::FlushFailed => "the flush failed",
            Self::LedgerNotRecorded => "the commit was flushed but could not be recorded",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_committed_outcome_permits_leaving_the_live_region() {
        let unknown = ScrollbackCommitOutcome::Unknown {
            evidence: UnknownEvidence::new(
                TransportStage::Body,
                3,
                UnknownReason::WriteFailedAfterAccept,
                None,
            ),
        };
        assert!(!unknown.permits_live_removal());

        let not_committed = ScrollbackCommitOutcome::NotCommitted {
            cause: NotCommittedCause::SinkClosed,
        };
        assert!(!not_committed.permits_live_removal());
    }

    #[test]
    fn only_a_not_committed_outcome_permits_a_plain_retry() {
        let unknown = ScrollbackCommitOutcome::Unknown {
            evidence: UnknownEvidence::new(
                TransportStage::Reset,
                7,
                UnknownReason::FlushFailed,
                None,
            ),
        };
        assert!(!unknown.permits_retry());

        let not_committed = ScrollbackCommitOutcome::NotCommitted {
            cause: NotCommittedCause::SinkClosed,
        };
        assert!(not_committed.permits_retry());
    }

    #[test]
    fn unknown_evidence_preserves_the_underlying_io_error() {
        let evidence = UnknownEvidence::new(
            TransportStage::Body,
            5,
            UnknownReason::WriteFailedAfterAccept,
            Some(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed")),
        );
        assert_eq!(
            evidence.source_error().map(io::Error::kind),
            Some(io::ErrorKind::BrokenPipe)
        );
        assert_eq!(evidence.accepted_transport_bytes(), 5);
        let rendered = evidence.to_string();
        assert!(rendered.contains("5 transport byte(s)"), "{rendered}");
        assert!(rendered.contains("pipe closed"), "{rendered}");
    }

    #[test]
    fn a_ledger_capacity_refusal_names_the_capacity() {
        let cause = NotCommittedCause::LedgerAtCapacity { capacity: 64 };
        assert!(cause.to_string().contains("64"));
    }
}
