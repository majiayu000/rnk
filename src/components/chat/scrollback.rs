//! Typed, idempotent commits of finished transcript into native scrollback.
//!
//! A chat that streams into the terminal has two regions with opposite rules.
//! The *live region* is redrawn freely: the active message and the composer are
//! repainted on every frame. The *scrollback* is append-only and, once written,
//! belongs to the terminal rather than to this library — nothing here can go
//! back and re-flow a line the terminal already owns.
//!
//! Moving a message across that boundary is therefore a one-way, unrepeatable
//! act, and the types in this module exist to make it one:
//!
//! - [`identity`] answers *is this the same commit I already made?* without ever
//!   carrying the message's bytes, so an audit trail cannot leak a transcript.
//! - [`content`] holds the bytes, and rejects control sequences that would let a
//!   committed line escape its own region.
//! - [`outcome`] partitions the result of a write into [`Committed`],
//!   [`NotCommitted`] and [`Unknown`] — the third state being the one a naive
//!   design omits, and the omission that duplicates transcript lines.
//! - [`ledger`] records confirmed commits so a repeated terminal event, a
//!   repeated render, or a burst of deltas cannot produce a second write.
//!
//! # Guarantees
//!
//! The default native-terminal path promises [`ProcessLocalConfirmed`]: within
//! one process, a confirmed commit is written exactly once. It cannot promise
//! more, because the write reaching the terminal and the ledger recording it are
//! two separate events and a crash can land between them. A sink that queries
//! and records in a single durable transaction may declare
//! [`DurableAtomicIdempotency`] and answer for commits that outlive the process.
//!
//! [`Committed`]: ScrollbackCommitOutcome::Committed
//! [`NotCommitted`]: ScrollbackCommitOutcome::NotCommitted
//! [`Unknown`]: ScrollbackCommitOutcome::Unknown
//! [`ProcessLocalConfirmed`]: ScrollbackGuarantee::ProcessLocalConfirmed
//! [`DurableAtomicIdempotency`]: ScrollbackGuarantee::DurableAtomicIdempotency

pub mod content;
pub mod digest;
pub mod durable;
pub mod identity;
pub mod ledger;
pub mod outcome;
pub mod sink;

pub use content::{
    ForbiddenControlKind, ScrollbackContent, ScrollbackContentError, TransportEncoding,
    TransportStage,
};
pub use digest::{ContentDigest, DigestBuilder};
pub use durable::{
    DurableApply, DurableCertainty, DurableCommitStore, DurableFailure, DurableScrollbackSink,
};
pub use identity::{
    ProjectionContext, ScrollbackCommitId, ScrollbackCommitKey, ScrollbackContentIdentity,
    ScrollbackIdentityError, ScrollbackNamespace, ThemeIdentity,
};
pub use ledger::{ConfirmedLedger, LedgerLookup, LedgerRecordError};
pub use sink::{DEFAULT_LEDGER_CAPACITY, NativeTerminalSink, ScrollbackSink};

pub use outcome::{
    AttemptDisposition, NotCommittedCause, ScrollbackCommitOutcome, ScrollbackGuarantee,
    ScrollbackReceipt, UnknownEvidence, UnknownReason,
};
