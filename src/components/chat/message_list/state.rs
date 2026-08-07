//! The message list's committed state and its read paths.
//!
//! Every mutation stages a complete candidate and commits it in one step. A
//! half-applied list — new order with old heights, or a fresh index against a
//! stale anchor — would put the viewport somewhere the renderer never paints,
//! and the reader would see the transcript jump for no reason they can act on.

use std::collections::HashMap;

use super::cache::BoundedMeasurementCache;
use super::error::MessageListStateError;
use super::height_index::FenwickRows;
use super::key::{MessageListEntry, MessageMeasureKeyHandle};
use super::types::{MessageListRevision, MessageRows, RowOffset, ViewportRows};
use super::view_state::{
    BottomFollowState, MessageAnchor, MessageListObservation, StoredAnchorAuthority,
    VisibleMessageRange, VisibleMessageSlice,
};
use crate::components::chat::MessageId;

mod mutations;
mod scroll;

/// A transcript's order, heights, and scroll position.
#[derive(Debug, Clone)]
pub struct MessageListState {
    pub(super) entries: Vec<MessageListEntry>,
    pub(super) positions: HashMap<MessageId, usize>,
    pub(super) rows: Vec<MessageRows>,
    pub(super) active_keys: Vec<MessageMeasureKeyHandle>,
    pub(super) index: FenwickRows,
    pub(super) measurements: BoundedMeasurementCache,
    pub(super) width: u16,
    pub(super) viewport_rows: ViewportRows,
    pub(super) scroll_offset: RowOffset,
    pub(super) stored_anchor: Option<MessageAnchor>,
    pub(super) anchor_authority: Option<StoredAnchorAuthority>,
    pub(super) follow: BottomFollowState,
    pub(super) revision: MessageListRevision,
}

/// The order, heights and index a view is resolved against.
///
/// Borrowed rather than owned so an in-place update can reuse the committed
/// vectors instead of cloning them. Cloning the whole transcript to move one
/// message's height is what makes streaming cost grow with history length.
pub(super) struct ListStructure<'a> {
    pub(super) entries: &'a [MessageListEntry],
    pub(super) positions: &'a HashMap<MessageId, usize>,
    pub(super) rows: &'a [MessageRows],
    pub(super) index: &'a FenwickRows,
}

impl ListStructure<'_> {
    pub(super) fn total_rows(&self) -> Result<u64, MessageListStateError> {
        self.index.total_rows()
    }

    pub(super) fn rows_of(&self, message_id: MessageId) -> Option<MessageRows> {
        self.positions
            .get(&message_id)
            .and_then(|index| self.rows.get(*index))
            .copied()
    }
}

/// The staged form of a mutation, before anything is committed.
pub(super) struct Candidate {
    pub(super) entries: Vec<MessageListEntry>,
    pub(super) positions: HashMap<MessageId, usize>,
    pub(super) rows: Vec<MessageRows>,
    pub(super) active_keys: Vec<MessageMeasureKeyHandle>,
    pub(super) index: FenwickRows,
}

impl MessageListState {
    /// The current revision.
    pub const fn revision(&self) -> MessageListRevision {
        self.revision
    }

    /// The width messages are laid out at.
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// The viewport height in rows.
    pub const fn viewport_rows(&self) -> ViewportRows {
        self.viewport_rows
    }

    /// The current scroll position.
    pub const fn scroll_offset(&self) -> RowOffset {
        self.scroll_offset
    }

    /// Whether the list is following the bottom.
    pub const fn follow_state(&self) -> BottomFollowState {
        self.follow
    }

    /// Where the view is anchored, if anywhere.
    pub const fn stored_anchor(&self) -> Option<MessageAnchor> {
        self.stored_anchor
    }

    /// The number of messages held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the list holds no messages.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The committed entries, in order.
    pub fn entries(&self) -> &[MessageListEntry] {
        &self.entries
    }

    /// A read-only snapshot of the scroll state.
    pub fn observation(&self) -> MessageListObservation {
        MessageListObservation {
            revision: self.revision,
            follow_state: self.follow,
            stored_anchor: self.stored_anchor,
            new_content_below: self.follow.new_content_below(),
        }
    }

    /// Total rows across every message.
    pub fn total_rows(&self) -> Result<u64, MessageListStateError> {
        self.index.total_rows()
    }

    /// The measured height of one message.
    pub fn message_rows(
        &self,
        message_id: MessageId,
    ) -> Result<MessageRows, MessageListStateError> {
        let index = self
            .positions
            .get(&message_id)
            .copied()
            .ok_or(MessageListStateError::UnknownMessageId { message_id })?;
        self.rows
            .get(index)
            .copied()
            .ok_or(MessageListStateError::MissingActiveMeasurement { message_id })
    }

    /// The parts of each message that fall inside the viewport.
    ///
    /// Measures nothing and changes nothing. Cost is `O(log n + k)` in the
    /// number of messages and visible slices: the rows above the viewport are
    /// never walked, which is what keeps scrolling flat as a transcript grows.
    pub fn visible_range(&self) -> Result<VisibleMessageRange, MessageListStateError> {
        let total_rows = self.index.total_rows()?;
        let scroll_offset = self.scroll_offset;

        if self.entries.is_empty() || self.viewport_rows.get() == 0 {
            return Ok(VisibleMessageRange {
                total_rows,
                scroll_offset,
                slices: Vec::new(),
            });
        }

        let viewport_end = scroll_offset
            .get()
            .checked_add(self.viewport_rows.get())
            .ok_or(MessageListStateError::RowArithmeticOverflow)?;

        let Some(first) = self.index.lower_bound(scroll_offset.get())? else {
            return Ok(VisibleMessageRange {
                total_rows,
                scroll_offset,
                slices: Vec::new(),
            });
        };

        let mut slices = Vec::new();
        let mut message_start = self.index.prefix_sum(first)?;

        for message_index in first..self.entries.len() {
            if message_start >= viewport_end {
                break;
            }
            let rows = self.rows_at(message_index)?;
            let message_end = message_start
                .checked_add(rows.get())
                .ok_or(MessageListStateError::RowArithmeticOverflow)?;

            let visible_start = message_start.max(scroll_offset.get());
            let visible_end = message_end.min(viewport_end);
            if visible_start < visible_end {
                slices.push(self.slice(
                    message_index,
                    message_start,
                    visible_start,
                    visible_end,
                    scroll_offset.get(),
                    rows,
                )?);
            }
            message_start = message_end;
        }

        Ok(VisibleMessageRange {
            total_rows,
            scroll_offset,
            slices,
        })
    }

    fn slice(
        &self,
        message_index: usize,
        message_start: u64,
        visible_start: u64,
        visible_end: u64,
        scroll_offset: u64,
        rows: MessageRows,
    ) -> Result<VisibleMessageSlice, MessageListStateError> {
        let entry = &self.entries[message_index];
        let local_start = visible_start - message_start;
        let local_end = visible_end - message_start;

        // Slot parity is a committed postcondition; a break here means the
        // heights and the order disagree, which must not be papered over by
        // treating the message as one row tall.
        if local_end > rows.get() {
            return Err(MessageListStateError::MissingActiveMeasurement {
                message_id: entry.message_id(),
            });
        }

        let key = self
            .active_keys
            .get(message_index)
            .ok_or(MessageListStateError::MissingActiveMeasurement {
                message_id: entry.message_id(),
            })?
            .clone();

        Ok(VisibleMessageSlice {
            message_id: entry.message_id(),
            message_index,
            measure_key: key,
            message_rows: local_start..local_end,
            viewport_rows: (visible_start - scroll_offset)..(visible_end - scroll_offset),
        })
    }

    pub(super) fn rows_at(&self, index: usize) -> Result<MessageRows, MessageListStateError> {
        self.rows.get(index).copied().ok_or_else(|| {
            MessageListStateError::MissingActiveMeasurement {
                message_id: self.entries[index].message_id(),
            }
        })
    }

    /// The largest scroll offset that keeps content in the viewport.
    pub(super) fn max_offset(&self, total_rows: u64) -> u64 {
        total_rows.saturating_sub(self.viewport_rows.get())
    }

    pub(super) fn structure(&self) -> ListStructure<'_> {
        ListStructure {
            entries: &self.entries,
            positions: &self.positions,
            rows: &self.rows,
            index: &self.index,
        }
    }

    pub(super) fn guard_revision(
        &self,
        expected: MessageListRevision,
    ) -> Result<(), MessageListStateError> {
        if expected == self.revision {
            return Ok(());
        }
        Err(MessageListStateError::StaleStateRevision {
            expected: expected.get(),
            actual: self.revision.get(),
        })
    }
}

impl Candidate {
    /// Stages order, positions, heights and keys, then builds the index.
    pub(super) fn try_build(
        entries: Vec<MessageListEntry>,
        rows: Vec<MessageRows>,
        active_keys: Vec<MessageMeasureKeyHandle>,
    ) -> Result<Self, MessageListStateError> {
        debug_assert_eq!(entries.len(), rows.len());
        debug_assert_eq!(entries.len(), active_keys.len());

        let mut positions = HashMap::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            if positions.insert(entry.message_id(), index).is_some() {
                return Err(MessageListStateError::DuplicateMessageId {
                    message_id: entry.message_id(),
                });
            }
        }
        let index = FenwickRows::try_build(&rows)?;

        Ok(Self {
            entries,
            positions,
            rows,
            active_keys,
            index,
        })
    }

    pub(super) fn structure(&self) -> ListStructure<'_> {
        ListStructure {
            entries: &self.entries,
            positions: &self.positions,
            rows: &self.rows,
            index: &self.index,
        }
    }

    pub(super) fn total_rows(&self) -> Result<u64, MessageListStateError> {
        self.index.total_rows()
    }
}
