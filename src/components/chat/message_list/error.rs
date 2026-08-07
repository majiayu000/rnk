//! Closed error families for the message list.
//!
//! Every failure the list can report names one cause. Nothing here is a
//! catch-all, a string, or `Box<dyn Error>`: a caller matching exhaustively on
//! these must keep compiling against a distinct set of reasons, because
//! "measurement missing" and "measurement failed" call for different recovery.

use core::fmt;
use std::error::Error;

use super::key::{MessageMeasureKey, MessageMeasureKeyHandle};
use super::types::{MessageRows, MessageRowsError, RowOffset};
use crate::components::chat::MessageId;

/// Why a row count could not be produced or a state transition could not run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageListStateError {
    /// Two entries claimed the same message identity.
    DuplicateMessageId {
        /// The repeated identity.
        message_id: MessageId,
    },
    /// The operation named a message the list does not hold.
    UnknownMessageId {
        /// The identity that was not found.
        message_id: MessageId,
    },
    /// An insert position was past the end of the list.
    InvalidInsertIndex {
        /// The requested position.
        index: usize,
        /// The number of entries currently held.
        len: usize,
    },
    /// A measurement callback reported no result for a key it was asked about.
    MissingMeasurement {
        /// The key whose measurement is missing.
        key: Box<MessageMeasureKey>,
    },
    /// A committed entry had no active row count, which breaks slot parity.
    MissingActiveMeasurement {
        /// The message whose active slot was empty.
        message_id: MessageId,
    },
    /// The caller's expected revision did not match the committed one.
    StaleStateRevision {
        /// The revision the caller expected.
        expected: u64,
        /// The revision the list actually holds.
        actual: u64,
    },
    /// The revision counter cannot advance any further.
    StateRevisionOverflow {
        /// The revision that could not be incremented.
        revision: u64,
    },
    /// A key no longer describes the entry it was built from.
    MeasurementIdentityMismatch {
        /// The message whose entry and key disagreed.
        message_id: MessageId,
    },
    /// Summing row counts exceeded the range of the row counter.
    RowArithmeticOverflow,
    /// A row coordinate does not fit the renderer type it must convert to.
    CoordinateOverflow {
        /// The value that could not be represented.
        value: u64,
        /// The type it was being converted to.
        target: &'static str,
    },
    /// An anchor named a row the message does not have.
    InvalidAnchorRow {
        /// The anchored message.
        message_id: MessageId,
        /// The row the caller asked for.
        requested: RowOffset,
        /// The number of rows the message actually measures.
        measured_rows: MessageRows,
    },
    /// A rebuilt config did not describe the entry it was rebuilt for.
    InvalidResizeConfig {
        /// Position of the entry in the committed order.
        message_index: usize,
        /// The entry's identity.
        message_id: MessageId,
        /// The width the rebuild was asked to target.
        new_width: u16,
    },
    /// A viewport width of zero cannot flow text.
    InvalidViewportWidth {
        /// The rejected width.
        width: u16,
    },
    /// A measurement cache must be able to hold at least one entry.
    InvalidCacheCapacity,
}

impl fmt::Display for MessageListStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMessageId { message_id } => {
                write!(formatter, "duplicate message id {message_id:?}")
            }
            Self::UnknownMessageId { message_id } => {
                write!(formatter, "unknown message id {message_id:?}")
            }
            Self::InvalidInsertIndex { index, len } => {
                write!(formatter, "insert index {index} is past the end ({len})")
            }
            Self::MissingMeasurement { key } => {
                write!(
                    formatter,
                    "no measurement for message {:?}",
                    key.message_id()
                )
            }
            Self::MissingActiveMeasurement { message_id } => {
                write!(
                    formatter,
                    "committed message {message_id:?} has no active measurement"
                )
            }
            Self::StaleStateRevision { expected, actual } => {
                write!(
                    formatter,
                    "stale state revision: expected {expected}, list is at {actual}"
                )
            }
            Self::StateRevisionOverflow { revision } => {
                write!(formatter, "state revision {revision} cannot advance")
            }
            Self::MeasurementIdentityMismatch { message_id } => {
                write!(
                    formatter,
                    "measurement key no longer describes message {message_id:?}"
                )
            }
            Self::RowArithmeticOverflow => write!(formatter, "row arithmetic overflow"),
            Self::CoordinateOverflow { value, target } => {
                write!(formatter, "row coordinate {value} does not fit {target}")
            }
            Self::InvalidAnchorRow {
                message_id,
                requested,
                measured_rows,
            } => write!(
                formatter,
                "anchor row {} is outside message {message_id:?} ({} rows)",
                requested.get(),
                measured_rows.get()
            ),
            Self::InvalidResizeConfig {
                message_index,
                message_id,
                new_width,
            } => write!(
                formatter,
                "rebuilt config for message {message_id:?} at index {message_index} \
                 does not describe width {new_width}"
            ),
            Self::InvalidViewportWidth { width } => {
                write!(formatter, "invalid viewport width {width}")
            }
            Self::InvalidCacheCapacity => {
                write!(formatter, "measurement cache capacity must be non-zero")
            }
        }
    }
}

impl Error for MessageListStateError {}

/// Why the reference composite adapter could not measure a message.
///
/// Generic over the text flow failure so the adapter never has to inspect,
/// downcast, or stringify what the flow reported.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageCompositeMeasureError<TextFlowFailure> {
    /// One textual child failed to flow.
    TextFlowFailed {
        /// Position of the child in the config's ordered list.
        child_index: usize,
        /// What the flow reported.
        source: TextFlowFailure,
    },
    /// A child's config disagreed with the shell it sits in.
    InvalidCompositeConfig {
        /// Position of the offending child.
        child_index: usize,
    },
    /// Summing child and structural rows exceeded the row counter.
    RowArithmeticOverflow,
    /// The parts summed to zero rows, which is not a renderable message.
    MessageRows(MessageRowsError),
}

impl<TextFlowFailure: fmt::Display> fmt::Display for MessageCompositeMeasureError<TextFlowFailure> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextFlowFailed {
                child_index,
                source,
            } => write!(
                formatter,
                "text flow for child {child_index} failed: {source}"
            ),
            Self::InvalidCompositeConfig { child_index } => write!(
                formatter,
                "child {child_index} does not fit the shell it is measured in"
            ),
            Self::RowArithmeticOverflow => write!(formatter, "composite row overflow"),
            Self::MessageRows(source) => write!(formatter, "composite rows rejected: {source}"),
        }
    }
}

impl<TextFlowFailure> Error for MessageCompositeMeasureError<TextFlowFailure>
where
    TextFlowFailure: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::TextFlowFailed { source, .. } => Some(source),
            Self::MessageRows(source) => Some(source),
            Self::InvalidCompositeConfig { .. } | Self::RowArithmeticOverflow => None,
        }
    }
}

/// Why a measuring mutation could not commit.
///
/// Failure and cancellation stay separate from the first version: a cancelled
/// measurement is a caller decision to stop, not an error to report upward, and
/// collapsing the two would force callers to guess which one happened.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageListMeasureError<Failure, Cancellation> {
    /// The mutation failed before any measurement ran.
    State(MessageListStateError),
    /// A resize could not rebuild one entry's config.
    ConfigRebuildFailed {
        /// Position of the entry in the committed order.
        message_index: usize,
        /// The entry's identity.
        message_id: MessageId,
        /// What the caller reported.
        source: Failure,
    },
    /// A resize's config rebuild was cancelled.
    ConfigRebuildCancelled {
        /// Position of the entry in the committed order.
        message_index: usize,
        /// The entry's identity.
        message_id: MessageId,
        /// What the caller reported.
        source: Cancellation,
    },
    /// A measurement callback reported failure.
    MeasurementFailed {
        /// The key that was being measured.
        key: Box<MessageMeasureKey>,
        /// What the caller reported.
        source: Failure,
    },
    /// A measurement callback reported cancellation.
    Cancelled {
        /// The key that was being measured.
        key: Box<MessageMeasureKey>,
        /// What the caller reported.
        source: Cancellation,
    },
}

impl<Failure, Cancellation> From<MessageListStateError>
    for MessageListMeasureError<Failure, Cancellation>
{
    fn from(error: MessageListStateError) -> Self {
        Self::State(error)
    }
}

impl<Failure, Cancellation> fmt::Display for MessageListMeasureError<Failure, Cancellation>
where
    Failure: fmt::Display,
    Cancellation: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(source) => source.fmt(formatter),
            Self::ConfigRebuildFailed {
                message_index,
                message_id,
                source,
            } => write!(
                formatter,
                "config rebuild for message {message_id:?} at index {message_index} failed: {source}"
            ),
            Self::ConfigRebuildCancelled {
                message_index,
                message_id,
                source,
            } => write!(
                formatter,
                "config rebuild for message {message_id:?} at index {message_index} \
                 was cancelled: {source}"
            ),
            Self::MeasurementFailed { key, source } => write!(
                formatter,
                "measuring message {:?} failed: {source}",
                key.message_id()
            ),
            Self::Cancelled { key, source } => write!(
                formatter,
                "measuring message {:?} was cancelled: {source}",
                key.message_id()
            ),
        }
    }
}

impl<Failure, Cancellation> Error for MessageListMeasureError<Failure, Cancellation>
where
    Failure: Error + 'static,
    Cancellation: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(source) => Some(source),
            Self::ConfigRebuildFailed { source, .. } | Self::MeasurementFailed { source, .. } => {
                Some(source)
            }
            Self::ConfigRebuildCancelled { source, .. } | Self::Cancelled { source, .. } => {
                Some(source)
            }
        }
    }
}

/// Why rendering the visible slices could not produce an element.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageListRenderError<RenderFailure> {
    /// The state could not produce a visible range to render.
    State(MessageListStateError),
    /// The caller's render closure failed for one slice.
    RenderFailed {
        /// The message being rendered.
        message_id: MessageId,
        /// The key the geometry was measured under.
        key: MessageMeasureKeyHandle,
        /// The message-local rows the slice covers.
        message_rows: core::ops::Range<u64>,
        /// What the closure reported.
        source: RenderFailure,
    },
}

impl<RenderFailure> From<MessageListStateError> for MessageListRenderError<RenderFailure> {
    fn from(error: MessageListStateError) -> Self {
        Self::State(error)
    }
}

impl<RenderFailure: fmt::Display> fmt::Display for MessageListRenderError<RenderFailure> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(source) => source.fmt(formatter),
            Self::RenderFailed {
                message_id,
                message_rows,
                source,
                ..
            } => write!(
                formatter,
                "rendering message {message_id:?} rows {}..{} failed: {source}",
                message_rows.start, message_rows.end
            ),
        }
    }
}

impl<RenderFailure> Error for MessageListRenderError<RenderFailure>
where
    RenderFailure: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(source) => Some(source),
            Self::RenderFailed { source, .. } => Some(source),
        }
    }
}
