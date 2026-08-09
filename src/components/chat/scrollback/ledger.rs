//! The confirmed-commit ledger every sink deduplicates against.
//!
//! The ledger is bounded and **never evicts**. That is the whole design: an
//! eviction policy on a confirmed-commit record is a policy for writing the same
//! transcript line to the terminal twice. When the bound is reached the ledger
//! refuses new commits, which is visible and recoverable; silently forgetting an
//! old one is neither.

use std::collections::HashMap;
use std::num::{NonZeroU64, NonZeroUsize};

use super::identity::{ScrollbackCommitId, ScrollbackCommitKey, ScrollbackContentIdentity};
use super::outcome::{ScrollbackGuarantee, ScrollbackReceipt};

/// A bounded, eviction-free record of confirmed commits.
#[derive(Debug)]
pub struct ConfirmedLedger {
    capacity: NonZeroUsize,
    guarantee: ScrollbackGuarantee,
    entries: HashMap<ScrollbackCommitKey, ConfirmedEntry>,
    next_sequence: NonZeroU64,
}

#[derive(Debug)]
struct ConfirmedEntry {
    commit_id: ScrollbackCommitId,
    receipt: ScrollbackReceipt,
}

impl ConfirmedLedger {
    /// Creates an empty ledger with a fixed capacity.
    pub fn new(capacity: NonZeroUsize, guarantee: ScrollbackGuarantee) -> Self {
        Self {
            capacity,
            guarantee,
            entries: HashMap::new(),
            next_sequence: NonZeroU64::MIN,
        }
    }

    /// Returns the configured capacity.
    pub const fn capacity(&self) -> NonZeroUsize {
        self.capacity
    }

    /// Returns how many commits are confirmed.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether no commit is confirmed yet.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Classifies a commit against what is already confirmed.
    pub fn lookup(&self, commit_id: &ScrollbackCommitId) -> LedgerLookup<'_> {
        match self.entries.get(commit_id.key()) {
            None => LedgerLookup::Absent,
            Some(entry) if entry.commit_id.is_same_commit(commit_id) => {
                LedgerLookup::Confirmed(&entry.receipt)
            }
            Some(entry) => LedgerLookup::Conflict {
                confirmed: entry.commit_id.content(),
            },
        }
    }

    /// Records a commit and mints its original receipt.
    ///
    /// Only ever called once per key: [`lookup`] must return [`Absent`] first,
    /// which the sinks in this module enforce by construction.
    ///
    /// [`lookup`]: Self::lookup
    /// [`Absent`]: LedgerLookup::Absent
    pub fn record(
        &mut self,
        commit_id: ScrollbackCommitId,
    ) -> Result<ScrollbackReceipt, LedgerRecordError> {
        if self.entries.contains_key(commit_id.key()) {
            return Err(LedgerRecordError::AlreadyRecorded);
        }
        if self.entries.len() >= self.capacity.get() {
            return Err(LedgerRecordError::AtCapacity {
                capacity: self.capacity.get(),
            });
        }
        let receipt = ScrollbackReceipt::new(self.next_sequence, commit_id.clone(), self.guarantee);
        // Checked, because a wrapped sequence would let two distinct commits
        // claim the same position in the confirmed order.
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or(LedgerRecordError::SequenceOverflow)?;
        self.entries.insert(
            commit_id.key().clone(),
            ConfirmedEntry {
                commit_id,
                receipt: receipt.clone(),
            },
        );
        self.next_sequence = next;
        Ok(receipt)
    }

    /// Reports whether the ledger can accept one more commit.
    pub fn has_room(&self) -> bool {
        self.entries.len() < self.capacity.get()
    }
}

/// How a commit relates to the ledger's confirmed contents.
#[derive(Debug)]
pub enum LedgerLookup<'a> {
    /// This exact commit is already confirmed; here is its original receipt.
    Confirmed(&'a ScrollbackReceipt),
    /// The key is confirmed with different content or context.
    Conflict {
        /// The content identity already confirmed under the key.
        confirmed: ScrollbackContentIdentity,
    },
    /// Nothing is confirmed under this key.
    Absent,
}

/// Every way recording a confirmed commit can fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LedgerRecordError {
    /// The ledger is full and will not evict a confirmed entry.
    AtCapacity {
        /// The ledger's configured capacity.
        capacity: usize,
    },
    /// The key was already recorded, so a caller skipped its lookup.
    AlreadyRecorded,
    /// The confirmed sequence counter would overflow.
    SequenceOverflow,
}

impl std::fmt::Display for LedgerRecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtCapacity { capacity } => write!(
                f,
                "the confirmed ledger is full at its capacity of {capacity} entries"
            ),
            Self::AlreadyRecorded => {
                f.write_str("this commit key is already recorded in the confirmed ledger")
            }
            Self::SequenceOverflow => {
                f.write_str("the confirmed commit sequence counter would overflow")
            }
        }
    }
}

impl std::error::Error for LedgerRecordError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::chat::scrollback::identity::{
        ProjectionContext, ScrollbackContentIdentity, ScrollbackNamespace, ThemeIdentity,
    };
    use crate::components::chat::{MessageId, MessageRevision};

    fn commit(id: u64, text: &str) -> ScrollbackCommitId {
        let context = ProjectionContext::new(80, ThemeIdentity::new(0)).expect("valid");
        ScrollbackCommitId::new(
            ScrollbackCommitKey::new(
                ScrollbackNamespace::new("store").expect("non-empty"),
                MessageId::new(id),
                MessageRevision::INITIAL,
            ),
            ScrollbackContentIdentity::derive(text, context),
            context,
        )
    }

    fn ledger(capacity: usize) -> ConfirmedLedger {
        ConfirmedLedger::new(
            NonZeroUsize::new(capacity).expect("non-zero"),
            ScrollbackGuarantee::ProcessLocalConfirmed,
        )
    }

    #[test]
    fn an_unseen_commit_is_absent() {
        let ledger = ledger(4);
        assert!(matches!(
            ledger.lookup(&commit(1, "a")),
            LedgerLookup::Absent
        ));
        assert!(ledger.is_empty());
    }

    #[test]
    fn a_recorded_commit_is_found_with_its_original_receipt() {
        let mut ledger = ledger(4);
        let receipt = ledger.record(commit(1, "a")).expect("recorded");
        match ledger.lookup(&commit(1, "a")) {
            LedgerLookup::Confirmed(found) => assert_eq!(found, &receipt),
            other => panic!("expected confirmed, got {other:?}"),
        }
    }

    #[test]
    fn the_same_key_with_different_content_is_a_conflict() {
        let mut ledger = ledger(4);
        ledger.record(commit(1, "first")).expect("recorded");
        match ledger.lookup(&commit(1, "second")) {
            LedgerLookup::Conflict { confirmed } => {
                assert_eq!(confirmed, commit(1, "first").content());
            }
            other => panic!("expected conflict, got {other:?}"),
        }
    }

    #[test]
    fn sequences_are_assigned_in_recording_order() {
        let mut ledger = ledger(4);
        let first = ledger.record(commit(1, "a")).expect("recorded");
        let second = ledger.record(commit(2, "b")).expect("recorded");
        assert_eq!(first.sequence().get(), 1);
        assert_eq!(second.sequence().get(), 2);
    }

    #[test]
    fn recording_the_same_key_twice_is_refused_rather_than_resequenced() {
        let mut ledger = ledger(4);
        ledger.record(commit(1, "a")).expect("recorded");
        assert_eq!(
            ledger.record(commit(1, "a")),
            Err(LedgerRecordError::AlreadyRecorded)
        );
    }

    #[test]
    fn a_full_ledger_refuses_rather_than_evicting_a_confirmed_commit() {
        let mut ledger = ledger(2);
        ledger.record(commit(1, "a")).expect("recorded");
        ledger.record(commit(2, "b")).expect("recorded");
        assert!(!ledger.has_room());
        assert_eq!(
            ledger.record(commit(3, "c")),
            Err(LedgerRecordError::AtCapacity { capacity: 2 })
        );
        // The refusal must not have cost an earlier confirmed entry.
        assert!(matches!(
            ledger.lookup(&commit(1, "a")),
            LedgerLookup::Confirmed(_)
        ));
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn a_refused_record_does_not_consume_a_sequence_number() {
        let mut ledger = ledger(1);
        ledger.record(commit(1, "a")).expect("recorded");
        let _ = ledger.record(commit(2, "b"));
        // Draining and re-checking is impossible without eviction, so assert on
        // the invariant directly: one entry, one consumed sequence.
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger.next_sequence.get(), 2);
    }
}
