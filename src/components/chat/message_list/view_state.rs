//! Anchors, follow state, and the values the list reports to callers.

use super::key::{MessageListEntry, MessageMeasureKey, MessageMeasureKeyHandle};
use super::types::{
    MessageCompositeMeasureConfig, MessageListRevision, MessageRows, RowOffset, ViewportRows,
};
use crate::components::chat::MessageId;

/// A position in the list, expressed as a message and a row inside it.
///
/// Anchoring to a row within a message, rather than to a global offset, is what
/// keeps the view still when messages above it are inserted, removed, or change
/// height while the reader is looking at something further down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageAnchor {
    message_id: MessageId,
    intra_message_row: RowOffset,
}

impl MessageAnchor {
    /// Builds an anchor.
    pub const fn new(message_id: MessageId, intra_message_row: RowOffset) -> Self {
        Self {
            message_id,
            intra_message_row,
        }
    }

    /// The anchored message.
    pub const fn message_id(&self) -> MessageId {
        self.message_id
    }

    /// The row within that message.
    pub const fn intra_message_row(&self) -> RowOffset {
        self.intra_message_row
    }
}

/// Whether the list is tracking the bottom of the transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BottomFollowState {
    /// New content pulls the view down.
    Following,
    /// The reader scrolled away, and stays where they are.
    Paused {
        /// Whether content arrived below the viewport since pausing.
        new_content_below: bool,
    },
}

impl BottomFollowState {
    /// Whether content arrived below the viewport while paused.
    pub const fn new_content_below(self) -> bool {
        matches!(
            self,
            Self::Paused {
                new_content_below: true
            }
        )
    }
}

/// Which decision put the stored anchor where it is.
///
/// An anchor the reader asked for by name must survive mutations verbatim,
/// while one derived from wherever the viewport happened to sit is refreshed
/// from the current top. Conflating the two makes explicit navigation drift a
/// row at a time as content arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StoredAnchorAuthority {
    ViewportTop,
    ExplicitNavigation,
}

/// The part of one message that falls inside the viewport.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleMessageSlice {
    /// The message being shown.
    pub message_id: MessageId,
    /// Its position in the list.
    pub message_index: usize,
    /// The key its height was measured under.
    pub measure_key: MessageMeasureKeyHandle,
    /// Rows of the message that are visible, message-local and half-open.
    pub message_rows: core::ops::Range<u64>,
    /// Where those rows land in the viewport, half-open.
    pub viewport_rows: core::ops::Range<u64>,
}

/// Everything the renderer needs for one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleMessageRange {
    /// Total rows across every message.
    pub total_rows: u64,
    /// The current scroll position.
    pub scroll_offset: RowOffset,
    /// The visible parts, in list order.
    pub slices: Vec<VisibleMessageSlice>,
}

/// What a mutation changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageListUpdate {
    /// The revision before the mutation.
    pub previous_revision: MessageListRevision,
    /// The revision after it.
    pub applied_revision: MessageListRevision,
    /// Whether a valid anchor had to move because its message shrank.
    pub anchor_clamped: bool,
    /// Whether the anchor could not be placed at the viewport top.
    pub viewport_clamped: bool,
}

/// The result of a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageListMutation {
    /// Something observable changed.
    Applied(MessageListUpdate),
    /// Nothing changed, so the revision did not advance.
    NoChange {
        /// The unchanged revision.
        revision: MessageListRevision,
    },
}

/// A read-only snapshot of the list's scroll state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageListObservation {
    /// The current revision.
    pub revision: MessageListRevision,
    /// Whether the list is following the bottom.
    pub follow_state: BottomFollowState,
    /// Where the view is anchored, if anywhere.
    pub stored_anchor: Option<MessageAnchor>,
    /// Whether content arrived below the viewport while paused.
    pub new_content_below: bool,
}

/// What the list asks a caller to measure.
#[derive(Debug, Clone, Copy)]
pub struct MessageMeasureRequest<'a> {
    /// The entry being measured.
    pub entry: &'a MessageListEntry,
    /// The key it will be cached under.
    pub key: &'a MessageMeasureKey,
}

/// A caller's answer to a measurement request.
///
/// Failure and cancellation are separate variants rather than one error type:
/// a cancelled measurement is the caller deciding to stop, and treating it as a
/// failure would report an error for something nothing went wrong in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageMeasureOutcome<Failure, Cancellation> {
    /// The message measures this many rows.
    Measured(MessageRows),
    /// The caller has no measurement for this key.
    Missing,
    /// Measuring failed.
    Failed(Failure),
    /// Measuring was cancelled.
    Cancelled(Cancellation),
}

/// What the list asks a caller to rebuild during a resize.
#[derive(Debug, Clone, Copy)]
pub struct MessageResizeConfigRequest<'a> {
    /// The entry's position in the committed order.
    pub message_index: usize,
    /// The entry as committed before the resize.
    pub old_entry: &'a MessageListEntry,
    /// The key it was measured under before the resize.
    pub old_key: &'a MessageMeasureKey,
    /// The width to rebuild for.
    pub new_width: u16,
    /// The viewport height to rebuild for.
    pub new_viewport_rows: ViewportRows,
}

/// A caller's answer to a resize rebuild request.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageResizeConfigOutcome<Failure, Cancellation> {
    /// The rebuilt config for the new width.
    Rebuilt(MessageCompositeMeasureConfig),
    /// Rebuilding failed.
    Failed(Failure),
    /// Rebuilding was cancelled.
    Cancelled(Cancellation),
}
