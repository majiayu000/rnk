//! Exactly-once commits for sinks whose store can transact.
//!
//! [`NativeTerminalSink`] cannot promise anything across a restart, and the
//! reason is structural rather than a missing feature: writing to the terminal
//! and recording the write are two events, and a crash can land between them.
//! Nothing added on top of a plain `Write` closes that gap.
//!
//! So the gap is moved instead of papered over. A [`DurableCommitStore`] performs
//! **the visible effect and its record in one transaction**, and
//! [`DurableScrollbackSink`] does nothing but translate that store's answers into
//! typed outcomes. The store is where the guarantee actually lives; the sink is
//! only allowed to advertise [`DurableAtomicIdempotency`] because it never writes
//! anything itself.
//!
//! # Reporting failure honestly
//!
//! A store that fails must say whether its transaction landed. Some failures are
//! provably clean — a refused connection, a rejected precondition — and those are
//! retryable. Others, notably a timeout after the write was dispatched, are not
//! knowable from the caller's side. [`DurableCertainty`] is how a store states
//! which it hit, and a store that reports [`NotApplied`] when it does not know is
//! how duplicate lines get written.
//!
//! [`NativeTerminalSink`]: super::sink::NativeTerminalSink
//! [`DurableAtomicIdempotency`]: ScrollbackGuarantee::DurableAtomicIdempotency
//! [`NotApplied`]: DurableCertainty::NotApplied

use std::fmt;
use std::io;
use std::num::NonZeroU64;

use super::content::{ScrollbackContent, TransportStage};
use super::identity::{ScrollbackCommitId, ScrollbackContentIdentity};
use super::outcome::{
    AttemptDisposition, NotCommittedCause, ScrollbackCommitOutcome, ScrollbackGuarantee,
    ScrollbackReceipt, UnknownEvidence, UnknownReason,
};
use super::sink::{ScrollbackSink, declaration_mismatch};

/// A store that applies a commit's visible effect and its record atomically.
///
/// The single method is the whole contract, and it is a *transaction*, not two
/// calls a caller could interleave. An implementation that queries first and
/// writes second has the same crash window as a plain terminal write and must
/// not be used behind [`DurableScrollbackSink`].
pub trait DurableCommitStore {
    /// The store's own failure type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Applies `content` under `commit_id` if it has not been applied before.
    ///
    /// Returning [`AlreadyApplied`] must be based on a durable record, since it
    /// is the answer that suppresses a retry's visible effect. Returning it from
    /// process-local state would make a restarted process write the line twice.
    ///
    /// [`AlreadyApplied`]: DurableApply::AlreadyApplied
    fn apply(
        &mut self,
        commit_id: &ScrollbackCommitId,
        content: &ScrollbackContent,
    ) -> Result<DurableApply, DurableFailure<Self::Error>>;
}

/// What a store's transaction did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableApply {
    /// This call applied the commit and recorded it, in one transaction.
    Applied {
        /// The commit's durable position in the confirmed order.
        sequence: NonZeroU64,
    },
    /// A durable record already existed; this call changed nothing.
    AlreadyApplied {
        /// The sequence recorded when the commit was originally applied.
        sequence: NonZeroU64,
    },
    /// The key is recorded with different content, so it is refused.
    Conflict {
        /// The content identity durably recorded under this key.
        confirmed: ScrollbackContentIdentity,
    },
}

/// A store failure, paired with what the store knows about its transaction.
#[derive(Debug)]
pub struct DurableFailure<E> {
    certainty: DurableCertainty,
    source: E,
}

impl<E> DurableFailure<E> {
    /// Reports a failure whose transaction provably did not land.
    pub const fn not_applied(source: E) -> Self {
        Self {
            certainty: DurableCertainty::NotApplied,
            source,
        }
    }

    /// Reports a failure whose transaction may or may not have landed.
    ///
    /// The correct choice whenever the store cannot prove otherwise. It costs a
    /// commit that a human must resolve; the alternative costs a duplicated
    /// transcript line that nobody notices.
    pub const fn unknown(source: E) -> Self {
        Self {
            certainty: DurableCertainty::Unknown,
            source,
        }
    }

    /// Returns what the store knows about whether its transaction landed.
    pub const fn certainty(&self) -> DurableCertainty {
        self.certainty
    }

    /// Returns the underlying store error.
    pub const fn source_error(&self) -> &E {
        &self.source
    }
}

/// Whether a failed store transaction is known not to have landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DurableCertainty {
    /// The transaction provably did not apply; a retry is safe.
    NotApplied,
    /// The transaction may have applied; no retry can be proven safe.
    Unknown,
}

impl fmt::Display for DurableCertainty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotApplied => "the durable transaction provably did not apply",
            Self::Unknown => "the durable transaction may or may not have applied",
        })
    }
}

/// A sink whose exactly-once guarantee is delegated to its store.
///
/// It performs no I/O of its own — deliberately. Every byte reaches its
/// destination inside the store's transaction, which is the only reason this
/// sink may advertise [`DurableAtomicIdempotency`].
///
/// [`DurableAtomicIdempotency`]: ScrollbackGuarantee::DurableAtomicIdempotency
#[derive(Debug)]
pub struct DurableScrollbackSink<S> {
    store: S,
    closed: bool,
}

impl<S: DurableCommitStore> DurableScrollbackSink<S> {
    /// Wraps a store that satisfies the [`DurableCommitStore`] contract.
    pub const fn new(store: S) -> Self {
        Self {
            store,
            closed: false,
        }
    }

    /// Returns the underlying store.
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Unwraps the sink, returning its store.
    ///
    /// Useful across a simulated restart: the store outlives the sink, which is
    /// exactly the asymmetry the durable guarantee rests on.
    pub fn into_store(self) -> S {
        self.store
    }

    /// Shuts the sink down; every later commit is refused as [`SinkClosed`].
    ///
    /// [`SinkClosed`]: NotCommittedCause::SinkClosed
    pub const fn close(&mut self) {
        self.closed = true;
    }

    /// Reports whether the sink has been shut down.
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
}

impl<S: DurableCommitStore> ScrollbackSink for DurableScrollbackSink<S> {
    fn guarantee(&self) -> ScrollbackGuarantee {
        ScrollbackGuarantee::DurableAtomicIdempotency
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
        let receipt = |sequence| {
            ScrollbackReceipt::new(
                sequence,
                commit_id.clone(),
                ScrollbackGuarantee::DurableAtomicIdempotency,
            )
        };
        match self.store.apply(commit_id, content) {
            Ok(DurableApply::Applied { sequence }) => ScrollbackCommitOutcome::Committed {
                receipt: receipt(sequence),
                disposition: AttemptDisposition::Written,
            },
            // The original receipt, reconstructed from the durable record rather
            // than minted anew: a second receipt would imply a second line.
            Ok(DurableApply::AlreadyApplied { sequence }) => ScrollbackCommitOutcome::Committed {
                receipt: receipt(sequence),
                disposition: AttemptDisposition::AlreadyCommitted,
            },
            Ok(DurableApply::Conflict { confirmed }) => ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::IdentityConflict {
                    confirmed,
                    presented: content.identity(),
                },
            },
            Err(failure) => {
                let error = io::Error::other(failure.source);
                match failure.certainty {
                    DurableCertainty::NotApplied => ScrollbackCommitOutcome::NotCommitted {
                        cause: NotCommittedCause::FirstWriteRejected {
                            stage: TransportStage::Body,
                            source: error,
                        },
                    },
                    DurableCertainty::Unknown => ScrollbackCommitOutcome::Unknown {
                        evidence: UnknownEvidence::new(
                            TransportStage::Body,
                            content.encode().total_len(),
                            UnknownReason::LedgerNotRecorded,
                            Some(error),
                        ),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use super::super::identity::ScrollbackCommitKey;
    use super::super::sink::harness::{commit_id, content, context};
    use super::*;
    use crate::components::chat::{MessageId, MessageRevision};

    /// A store whose records and visible transcript outlive any one sink.
    ///
    /// Sharing both through one `Rc` is what makes a restart testable: dropping
    /// the sink and building a new one over the same handle is exactly the
    /// scenario `ProcessLocalConfirmed` cannot survive.
    #[derive(Debug, Clone, Default)]
    struct SharedStore {
        inner: Rc<RefCell<StoreState>>,
    }

    #[derive(Debug, Default)]
    struct StoreState {
        applied: HashMap<ScrollbackCommitKey, (NonZeroU64, ScrollbackContentIdentity)>,
        transcript: Vec<String>,
        next_sequence: u64,
        fail_with: Option<DurableCertainty>,
    }

    #[derive(Debug)]
    struct StoreError(&'static str);

    impl fmt::Display for StoreError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for StoreError {}

    impl SharedStore {
        fn transcript(&self) -> Vec<String> {
            self.inner.borrow().transcript.clone()
        }

        fn fail_next(&self, certainty: DurableCertainty) {
            self.inner.borrow_mut().fail_with = Some(certainty);
        }
    }

    impl DurableCommitStore for SharedStore {
        type Error = StoreError;

        fn apply(
            &mut self,
            commit_id: &ScrollbackCommitId,
            content: &ScrollbackContent,
        ) -> Result<DurableApply, DurableFailure<Self::Error>> {
            let mut state = self.inner.borrow_mut();
            if let Some(certainty) = state.fail_with.take() {
                let error = StoreError("the store rejected the transaction");
                return Err(match certainty {
                    DurableCertainty::NotApplied => DurableFailure::not_applied(error),
                    DurableCertainty::Unknown => DurableFailure::unknown(error),
                });
            }
            if let Some((sequence, recorded)) = state.applied.get(commit_id.key()).copied() {
                if recorded != commit_id.content() {
                    return Ok(DurableApply::Conflict {
                        confirmed: recorded,
                    });
                }
                return Ok(DurableApply::AlreadyApplied { sequence });
            }
            // One transaction: the visible effect and its record together.
            state.next_sequence += 1;
            let sequence = NonZeroU64::new(state.next_sequence).expect("incremented from zero");
            let line = content.canonical().to_owned();
            state.transcript.push(line);
            state
                .applied
                .insert(commit_id.key().clone(), (sequence, commit_id.content()));
            Ok(DurableApply::Applied { sequence })
        }
    }

    #[test]
    fn a_first_commit_applies_and_is_written_by_this_attempt() {
        let store = SharedStore::default();
        let mut sink = DurableScrollbackSink::new(store.clone());

        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::Committed {
                disposition: AttemptDisposition::Written,
                ..
            }
        ));
        assert_eq!(store.transcript(), vec!["hello".to_owned()]);
    }

    #[test]
    fn a_retry_across_a_restart_produces_no_second_visible_effect() {
        let store = SharedStore::default();
        let id = commit_id(1, "hello");

        let mut first_run = DurableScrollbackSink::new(store.clone());
        let first = first_run.commit(&id, &content("hello"));
        let original_sequence = first
            .receipt()
            .expect("the first attempt commits")
            .sequence();
        drop(first_run);

        // A new process would rebuild the sink from scratch; only the store
        // survives, which is the whole point of the durable guarantee.
        let mut second_run = DurableScrollbackSink::new(store.clone());
        let replay = second_run.commit(&id, &content("hello"));

        assert!(matches!(
            replay,
            ScrollbackCommitOutcome::Committed {
                disposition: AttemptDisposition::AlreadyCommitted,
                ..
            }
        ));
        assert_eq!(
            replay.receipt().expect("already committed").sequence(),
            original_sequence,
            "the durable record must return the original receipt, not a new one"
        );
        assert_eq!(store.transcript(), vec!["hello".to_owned()]);
    }

    #[test]
    fn repeated_completion_events_within_one_run_apply_once() {
        let store = SharedStore::default();
        let mut sink = DurableScrollbackSink::new(store.clone());
        let id = commit_id(3, "answer");

        for _ in 0..32 {
            sink.commit(&id, &content("answer"));
        }

        assert_eq!(store.transcript().len(), 1);
    }

    #[test]
    fn a_provably_clean_store_failure_is_retryable_and_then_succeeds() {
        let store = SharedStore::default();
        let mut sink = DurableScrollbackSink::new(store.clone());
        store.fail_next(DurableCertainty::NotApplied);

        let refused = sink.commit(&commit_id(1, "hello"), &content("hello"));
        assert!(refused.permits_retry());
        assert!(store.transcript().is_empty());

        let retried = sink.commit(&commit_id(1, "hello"), &content("hello"));
        assert!(retried.permits_live_removal());
        assert_eq!(store.transcript(), vec!["hello".to_owned()]);
    }

    #[test]
    fn an_ambiguous_store_failure_is_unknown_and_blocks_both_retry_and_removal() {
        let store = SharedStore::default();
        let mut sink = DurableScrollbackSink::new(store.clone());
        store.fail_next(DurableCertainty::Unknown);

        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        assert!(matches!(outcome, ScrollbackCommitOutcome::Unknown { .. }));
        assert!(!outcome.permits_retry());
        assert!(!outcome.permits_live_removal());
    }

    #[test]
    fn a_durably_recorded_key_with_different_content_fails_closed() {
        let store = SharedStore::default();
        let mut sink = DurableScrollbackSink::new(store.clone());
        sink.commit(&commit_id(1, "original"), &content("original"));

        let replacement = content("rewritten");
        let conflicting = ScrollbackCommitId::new(
            ScrollbackCommitKey::new(
                super::super::identity::ScrollbackNamespace::new("test").expect("non-empty"),
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
        assert_eq!(store.transcript(), vec!["original".to_owned()]);
    }

    #[test]
    fn a_closed_durable_sink_refuses_without_touching_the_store() {
        let store = SharedStore::default();
        let mut sink = DurableScrollbackSink::new(store.clone());
        sink.close();

        let outcome = sink.commit(&commit_id(1, "hello"), &content("hello"));

        assert!(matches!(
            outcome,
            ScrollbackCommitOutcome::NotCommitted {
                cause: NotCommittedCause::SinkClosed
            }
        ));
        assert!(store.transcript().is_empty());
    }

    #[test]
    fn the_durable_sink_advertises_atomic_idempotency() {
        let sink = DurableScrollbackSink::new(SharedStore::default());
        assert_eq!(
            sink.guarantee(),
            ScrollbackGuarantee::DurableAtomicIdempotency
        );
    }
}
