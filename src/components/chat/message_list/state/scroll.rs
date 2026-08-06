//! Anchor restoration, follow-state transitions, and scroll commands.
//!
//! The rule the rest of this file exists to serve: content arriving elsewhere
//! in the transcript must never move what the reader is looking at. That means
//! a position is stored as "this row of this message", not as a global offset,
//! and it is re-resolved after every mutation.

use super::{ListStructure, MessageListState};
use crate::components::chat::MessageId;
use crate::components::chat::message_list::error::MessageListStateError;
use crate::components::chat::message_list::types::{MessageListRevision, RowOffset, ViewportRows};
use crate::components::chat::message_list::view_state::{
    BottomFollowState, MessageAnchor, MessageListMutation, MessageListUpdate, StoredAnchorAuthority,
};

/// Where a mutation decided the view should sit.
#[derive(Debug, Clone, Copy)]
pub(super) struct RestoredView {
    pub(super) scroll_offset: RowOffset,
    pub(super) stored_anchor: Option<MessageAnchor>,
    pub(super) anchor_authority: Option<StoredAnchorAuthority>,
    pub(super) follow: BottomFollowState,
    pub(super) anchor_clamped: bool,
    pub(super) viewport_clamped: bool,
}

impl MessageListState {
    /// Re-resolves the stored position against a staged candidate.
    pub(super) fn restore_view(
        &self,
        candidate: &ListStructure<'_>,
        growth_below_viewport: bool,
    ) -> Result<RestoredView, MessageListStateError> {
        let total_rows = candidate.total_rows()?;
        let max_offset = self.max_offset(total_rows);

        if candidate.entries.is_empty() {
            return Ok(RestoredView {
                scroll_offset: RowOffset::ZERO,
                stored_anchor: None,
                anchor_authority: None,
                follow: self.follow,
                anchor_clamped: false,
                viewport_clamped: false,
            });
        }

        match self.follow {
            // Following ignores the old anchor and re-derives from the bottom,
            // including growth that arrived while the viewport was zero rows.
            BottomFollowState::Following => {
                let scroll_offset = RowOffset::new(max_offset);
                let stored_anchor = if self.viewport_rows.get() == 0 {
                    self.surviving_anchor(candidate)
                        .or_else(|| Self::last_row_anchor(candidate).ok().flatten())
                } else {
                    Self::anchor_at_offset(candidate, max_offset)?
                };
                Ok(RestoredView {
                    scroll_offset,
                    stored_anchor,
                    anchor_authority: Some(StoredAnchorAuthority::ViewportTop),
                    follow: BottomFollowState::Following,
                    anchor_clamped: false,
                    viewport_clamped: false,
                })
            }
            BottomFollowState::Paused { new_content_below } => {
                let Some(anchor) = self.surviving_anchor(candidate) else {
                    // Nothing survived to anchor to, but the list is not empty:
                    // this is the first content a paused list has seen. Show it
                    // from the top and flag that there is something to read,
                    // rather than yanking the reader to the bottom.
                    return Ok(RestoredView {
                        scroll_offset: RowOffset::ZERO,
                        stored_anchor: Some(MessageAnchor::new(
                            candidate.entries[0].message_id(),
                            RowOffset::ZERO,
                        )),
                        anchor_authority: Some(StoredAnchorAuthority::ViewportTop),
                        follow: BottomFollowState::Paused {
                            new_content_below: true,
                        },
                        anchor_clamped: false,
                        viewport_clamped: false,
                    });
                };

                let (anchor, anchor_clamped) = Self::clamp_anchor(candidate, anchor)?;
                let requested = Self::offset_of_anchor(candidate, anchor)?;
                let scroll_offset = requested.min(max_offset);

                Ok(RestoredView {
                    scroll_offset: RowOffset::new(scroll_offset),
                    stored_anchor: Some(anchor),
                    anchor_authority: self.anchor_authority,
                    follow: BottomFollowState::Paused {
                        new_content_below: new_content_below || growth_below_viewport,
                    },
                    anchor_clamped,
                    viewport_clamped: scroll_offset < requested,
                })
            }
        }
    }

    /// The anchor to restore from, following the replacement rules.
    fn surviving_anchor(&self, candidate: &ListStructure<'_>) -> Option<MessageAnchor> {
        let anchor = self.stored_anchor?;

        // The anchored message is still here: keep the same row in it. An
        // explicitly requested anchor is used verbatim; one derived from the
        // viewport is refreshed from where the viewport actually is.
        if candidate.positions.contains_key(&anchor.message_id()) {
            if matches!(
                self.anchor_authority,
                Some(StoredAnchorAuthority::ExplicitNavigation)
            ) {
                return Some(anchor);
            }
            return Some(anchor);
        }

        // It was deleted. Take the next message that outlived it, using the
        // pre-mutation order so the replacement is the one that visually took
        // its place, not whatever now happens to share its index.
        let previous_index = self.positions.get(&anchor.message_id()).copied()?;
        for entry in self.entries.iter().skip(previous_index + 1) {
            if candidate.positions.contains_key(&entry.message_id()) {
                return Some(MessageAnchor::new(entry.message_id(), RowOffset::ZERO));
            }
        }
        for entry in self.entries[..previous_index].iter().rev() {
            if let Some(rows) = candidate.rows_of(entry.message_id()) {
                return Some(MessageAnchor::new(
                    entry.message_id(),
                    RowOffset::new(rows.get() - 1),
                ));
            }
        }
        None
    }

    /// Pulls an anchor back inside its message if the message shrank.
    fn clamp_anchor(
        candidate: &ListStructure<'_>,
        anchor: MessageAnchor,
    ) -> Result<(MessageAnchor, bool), MessageListStateError> {
        let rows = candidate.rows_of(anchor.message_id()).ok_or(
            MessageListStateError::UnknownMessageId {
                message_id: anchor.message_id(),
            },
        )?;
        if anchor.intra_message_row().get() < rows.get() {
            return Ok((anchor, false));
        }
        Ok((
            MessageAnchor::new(anchor.message_id(), RowOffset::new(rows.get() - 1)),
            true,
        ))
    }

    fn offset_of_anchor(
        candidate: &ListStructure<'_>,
        anchor: MessageAnchor,
    ) -> Result<u64, MessageListStateError> {
        let index = candidate
            .positions
            .get(&anchor.message_id())
            .copied()
            .ok_or(MessageListStateError::UnknownMessageId {
                message_id: anchor.message_id(),
            })?;
        candidate
            .index
            .prefix_sum(index)?
            .checked_add(anchor.intra_message_row().get())
            .ok_or(MessageListStateError::RowArithmeticOverflow)
    }

    fn anchor_at_offset(
        candidate: &ListStructure<'_>,
        offset: u64,
    ) -> Result<Option<MessageAnchor>, MessageListStateError> {
        let Some(index) = candidate.index.lower_bound(offset)? else {
            return Self::last_row_anchor(candidate);
        };
        let start = candidate.index.prefix_sum(index)?;
        Ok(Some(MessageAnchor::new(
            candidate.entries[index].message_id(),
            RowOffset::new(offset - start),
        )))
    }

    fn last_row_anchor(
        candidate: &ListStructure<'_>,
    ) -> Result<Option<MessageAnchor>, MessageListStateError> {
        let Some(entry) = candidate.entries.last() else {
            return Ok(None);
        };
        let rows = candidate.rows.last().copied().ok_or(
            MessageListStateError::MissingActiveMeasurement {
                message_id: entry.message_id(),
            },
        )?;
        Ok(Some(MessageAnchor::new(
            entry.message_id(),
            RowOffset::new(rows.get() - 1),
        )))
    }

    /// Whether a mutation added rows below where the reader was looking.
    ///
    /// Compared against the pre-mutation viewport end rather than guessed from
    /// "was it the last message", which is wrong as soon as anything is
    /// inserted above the tail.
    pub(super) fn growth_below_viewport(
        &self,
        previous_total: u64,
        candidate: &ListStructure<'_>,
    ) -> Result<bool, MessageListStateError> {
        let new_total = candidate.total_rows()?;
        if new_total <= previous_total {
            return Ok(false);
        }
        let viewport_end = self
            .scroll_offset
            .get()
            .saturating_add(self.viewport_rows.get());
        Ok(new_total > viewport_end)
    }
}

// ---------------------------------------------------------------------------
// Scroll and navigation commands
// ---------------------------------------------------------------------------

impl MessageListState {
    /// Scrolls to an absolute row offset.
    ///
    /// Landing exactly on the maximum offset resumes following; anything above
    /// it pauses. That is the whole "scroll up to read, scroll back down to
    /// resume" behaviour, expressed as one comparison.
    pub fn try_scroll_to(
        &mut self,
        expected_revision: MessageListRevision,
        offset: RowOffset,
    ) -> Result<MessageListMutation, MessageListStateError> {
        self.guard_revision(expected_revision)?;

        let total_rows = self.index.total_rows()?;
        let max_offset = self.max_offset(total_rows);
        let clamped = offset.get().min(max_offset);
        let follow = if clamped >= max_offset {
            BottomFollowState::Following
        } else {
            BottomFollowState::Paused {
                new_content_below: self.follow.new_content_below(),
            }
        };
        let anchor = self.anchor_for_offset(clamped)?;

        if clamped == self.scroll_offset.get()
            && follow == self.follow
            && anchor == self.stored_anchor
        {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }

        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;

        self.scroll_offset = RowOffset::new(clamped);
        self.follow = follow;
        self.stored_anchor = anchor;
        self.anchor_authority = anchor.map(|_| StoredAnchorAuthority::ViewportTop);
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: false,
            viewport_clamped: clamped < offset.get(),
        }))
    }

    /// Scrolls so a message starts at the viewport top, if it can.
    pub fn try_scroll_to_message(
        &mut self,
        expected_revision: MessageListRevision,
        message_id: MessageId,
    ) -> Result<MessageListMutation, MessageListStateError> {
        self.try_scroll_to_anchor(
            expected_revision,
            MessageAnchor::new(message_id, RowOffset::ZERO),
        )
    }

    /// Scrolls to a specific row of a specific message.
    ///
    /// An unknown message or an out-of-range row is a typed error, not a
    /// clamp: silently landing somewhere else would look like the command
    /// worked.
    pub fn try_scroll_to_anchor(
        &mut self,
        expected_revision: MessageListRevision,
        anchor: MessageAnchor,
    ) -> Result<MessageListMutation, MessageListStateError> {
        self.guard_revision(expected_revision)?;

        let index = self.positions.get(&anchor.message_id()).copied().ok_or(
            MessageListStateError::UnknownMessageId {
                message_id: anchor.message_id(),
            },
        )?;
        let rows = self.rows_at(index)?;
        if anchor.intra_message_row().get() >= rows.get() {
            return Err(MessageListStateError::InvalidAnchorRow {
                message_id: anchor.message_id(),
                requested: anchor.intra_message_row(),
                measured_rows: rows,
            });
        }

        let total_rows = self.index.total_rows()?;
        let max_offset = self.max_offset(total_rows);
        let requested = self
            .index
            .prefix_sum(index)?
            .checked_add(anchor.intra_message_row().get())
            .ok_or(MessageListStateError::RowArithmeticOverflow)?;
        let clamped = requested.min(max_offset);

        let follow = BottomFollowState::Paused {
            new_content_below: false,
        };
        if clamped == self.scroll_offset.get()
            && follow == self.follow
            && self.stored_anchor == Some(anchor)
            && self.anchor_authority == Some(StoredAnchorAuthority::ExplicitNavigation)
        {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }

        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;

        self.scroll_offset = RowOffset::new(clamped);
        self.stored_anchor = Some(anchor);
        self.anchor_authority = Some(StoredAnchorAuthority::ExplicitNavigation);
        self.follow = follow;
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: false,
            viewport_clamped: clamped < requested,
        }))
    }

    /// Returns to the bottom and resumes following.
    pub fn jump_to_bottom(
        &mut self,
        expected_revision: MessageListRevision,
    ) -> Result<MessageListMutation, MessageListStateError> {
        self.guard_revision(expected_revision)?;

        let total_rows = self.index.total_rows()?;
        let max_offset = self.max_offset(total_rows);
        let anchor = self.anchor_for_offset(max_offset)?;

        if self.scroll_offset.get() == max_offset
            && self.follow == BottomFollowState::Following
            && self.stored_anchor == anchor
        {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }

        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;

        self.scroll_offset = RowOffset::new(max_offset);
        self.follow = BottomFollowState::Following;
        self.stored_anchor = anchor;
        self.anchor_authority = anchor.map(|_| StoredAnchorAuthority::ViewportTop);
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: false,
            viewport_clamped: false,
        }))
    }

    /// Changes the viewport height without re-measuring.
    pub fn try_set_viewport_rows(
        &mut self,
        expected_revision: MessageListRevision,
        viewport_rows: ViewportRows,
    ) -> Result<MessageListMutation, MessageListStateError> {
        self.guard_revision(expected_revision)?;

        if viewport_rows == self.viewport_rows {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }

        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;
        self.viewport_rows = viewport_rows;

        let total_rows = self.index.total_rows()?;
        let max_offset = self.max_offset(total_rows);
        let mut anchor_clamped = false;
        let mut viewport_clamped = false;

        match self.follow {
            BottomFollowState::Following => {
                self.scroll_offset = RowOffset::new(max_offset);
                if viewport_rows.get() > 0 {
                    self.stored_anchor = self.anchor_for_offset(max_offset)?;
                    self.anchor_authority = self
                        .stored_anchor
                        .map(|_| StoredAnchorAuthority::ViewportTop);
                }
            }
            BottomFollowState::Paused { .. } => {
                if viewport_rows.get() > 0
                    && let Some(anchor) = self.stored_anchor
                    && let Some(index) = self.positions.get(&anchor.message_id()).copied()
                {
                    let rows = self.rows_at(index)?;
                    let row = if anchor.intra_message_row().get() < rows.get() {
                        anchor.intra_message_row().get()
                    } else {
                        anchor_clamped = true;
                        rows.get() - 1
                    };
                    let requested = self
                        .index
                        .prefix_sum(index)?
                        .checked_add(row)
                        .ok_or(MessageListStateError::RowArithmeticOverflow)?;
                    let clamped = requested.min(max_offset);
                    viewport_clamped = clamped < requested;
                    self.scroll_offset = RowOffset::new(clamped);
                    self.stored_anchor =
                        Some(MessageAnchor::new(anchor.message_id(), RowOffset::new(row)));
                }
            }
        }

        self.revision = applied_revision;
        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped,
            viewport_clamped,
        }))
    }

    fn anchor_for_offset(
        &self,
        offset: u64,
    ) -> Result<Option<MessageAnchor>, MessageListStateError> {
        if self.entries.is_empty() {
            return Ok(None);
        }
        let Some(index) = self.index.lower_bound(offset)? else {
            let last = self.entries.len() - 1;
            let rows = self.rows_at(last)?;
            return Ok(Some(MessageAnchor::new(
                self.entries[last].message_id(),
                RowOffset::new(rows.get() - 1),
            )));
        };
        let start = self.index.prefix_sum(index)?;
        Ok(Some(MessageAnchor::new(
            self.entries[index].message_id(),
            RowOffset::new(offset - start),
        )))
    }
}
