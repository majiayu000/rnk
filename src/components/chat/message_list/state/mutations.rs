//! Constructor and measuring mutations.
//!
//! Each of these validates, stages a full candidate, measures whatever the
//! cache cannot supply, and commits once. Anything that fails leaves the
//! committed state byte-for-byte unchanged, so a caller that retries after a
//! cancelled measurement is retrying against exactly what it saw before.

use std::collections::HashMap;

use super::{Candidate, ListStructure, MessageListState};
use crate::components::chat::MessageId;
use crate::components::chat::message_list::cache::BoundedMeasurementCache;
use crate::components::chat::message_list::error::{
    MessageListMeasureError, MessageListStateError,
};
use crate::components::chat::message_list::key::{MessageListEntry, MessageMeasureKeyHandle};
use crate::components::chat::message_list::types::{
    MessageListRevision, MessageRows, RowOffset, ViewportRows,
};
use crate::components::chat::message_list::view_state::{
    BottomFollowState, MessageListMutation, MessageListUpdate, MessageMeasureOutcome,
    MessageMeasureRequest, MessageResizeConfigOutcome, MessageResizeConfigRequest,
    StoredAnchorAuthority,
};

/// Measures one staged order, reusing cached heights where the key matches.
fn measure_all<F, C, M>(
    entries: &[MessageListEntry],
    cache: &mut BoundedMeasurementCache,
    mut measure: M,
) -> Result<(Vec<MessageRows>, Vec<MessageMeasureKeyHandle>), MessageListMeasureError<F, C>>
where
    M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
{
    let mut rows = Vec::with_capacity(entries.len());
    let mut keys = Vec::with_capacity(entries.len());

    for entry in entries {
        let key = entry.measure_key();
        let measured = match cache.get(&key) {
            Some(cached) => cached,
            None => match measure(MessageMeasureRequest {
                entry,
                key: key.as_key(),
            }) {
                MessageMeasureOutcome::Measured(measured) => measured,
                MessageMeasureOutcome::Missing => {
                    return Err(MessageListMeasureError::State(
                        MessageListStateError::MissingMeasurement {
                            key: Box::new(key.as_key().clone()),
                        },
                    ));
                }
                MessageMeasureOutcome::Failed(source) => {
                    return Err(MessageListMeasureError::MeasurementFailed {
                        key: Box::new(key.as_key().clone()),
                        source,
                    });
                }
                MessageMeasureOutcome::Cancelled(source) => {
                    return Err(MessageListMeasureError::Cancelled {
                        key: Box::new(key.as_key().clone()),
                        source,
                    });
                }
            },
        };
        rows.push(measured);
        keys.push(key);
    }

    Ok((rows, keys))
}

fn reject_duplicates<F, C>(
    entries: &[MessageListEntry],
) -> Result<(), MessageListMeasureError<F, C>> {
    let mut seen = HashMap::with_capacity(entries.len());
    for entry in entries {
        if seen.insert(entry.message_id(), ()).is_some() {
            return Err(MessageListStateError::DuplicateMessageId {
                message_id: entry.message_id(),
            }
            .into());
        }
    }
    Ok(())
}

impl MessageListState {
    /// Builds a list, measuring every message once.
    ///
    /// Nothing is published unless every entry measures: a partially built list
    /// would report heights for some messages and not others, and there is no
    /// correct thing to render for the rest.
    pub fn try_new<F, C, M>(
        entries: &[MessageListEntry],
        width: u16,
        viewport_rows: ViewportRows,
        measurement_cache_capacity: usize,
        measure: M,
    ) -> Result<Self, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        if width == 0 {
            return Err(MessageListStateError::InvalidViewportWidth { width }.into());
        }
        if measurement_cache_capacity == 0 {
            return Err(MessageListStateError::InvalidCacheCapacity.into());
        }
        reject_duplicates(entries)?;

        let mut measurements = BoundedMeasurementCache::new(measurement_cache_capacity);
        let owned: Vec<MessageListEntry> = entries.to_vec();
        let (rows, active_keys) = measure_all(&owned, &mut measurements, measure)?;
        for (key, measured) in active_keys.iter().zip(&rows) {
            measurements.insert(key.clone(), *measured);
        }

        let candidate = Candidate::try_build(owned, rows, active_keys)?;
        let total_rows = candidate.total_rows()?;
        let max_offset = total_rows.saturating_sub(viewport_rows.get());

        let (scroll_offset, stored_anchor) = if candidate.entries.is_empty() {
            (0, None)
        } else {
            let anchor = if viewport_rows.get() == 0 {
                let last = candidate.entries.len() - 1;
                Some(
                    crate::components::chat::message_list::view_state::MessageAnchor::new(
                        candidate.entries[last].message_id(),
                        RowOffset::new(candidate.rows[last].get() - 1),
                    ),
                )
            } else {
                let index = candidate.index.lower_bound(max_offset)?.unwrap_or(0);
                let start = candidate.index.prefix_sum(index)?;
                Some(
                    crate::components::chat::message_list::view_state::MessageAnchor::new(
                        candidate.entries[index].message_id(),
                        RowOffset::new(max_offset - start),
                    ),
                )
            };
            (max_offset, anchor)
        };

        Ok(Self {
            entries: candidate.entries,
            positions: candidate.positions,
            rows: candidate.rows,
            active_keys: candidate.active_keys,
            index: candidate.index,
            measurements,
            width,
            viewport_rows,
            scroll_offset: RowOffset::new(scroll_offset),
            stored_anchor,
            anchor_authority: stored_anchor.map(|_| StoredAnchorAuthority::ViewportTop),
            follow: BottomFollowState::Following,
            revision: MessageListRevision::INITIAL,
        })
    }

    /// Replaces every message.
    pub fn try_replace_all<F, C, M>(
        &mut self,
        expected_revision: MessageListRevision,
        entries: &[MessageListEntry],
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        self.guard_revision(expected_revision)?;
        reject_duplicates(entries)?;

        if entries == self.entries.as_slice() {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }
        self.commit_measured(entries.to_vec(), measure)
    }

    /// Adds messages at the end.
    pub fn try_append<F, C, M>(
        &mut self,
        expected_revision: MessageListRevision,
        entries: &[MessageListEntry],
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        self.guard_revision(expected_revision)?;
        if entries.is_empty() {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }
        let mut staged = self.entries.clone();
        staged.extend_from_slice(entries);
        reject_duplicates(&staged)?;
        self.commit_measured(staged, measure)
    }

    /// Adds messages at the start, as when loading older history.
    pub fn try_prepend<F, C, M>(
        &mut self,
        expected_revision: MessageListRevision,
        entries: &[MessageListEntry],
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        self.guard_revision(expected_revision)?;
        if entries.is_empty() {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }
        let mut staged = entries.to_vec();
        staged.extend_from_slice(&self.entries);
        reject_duplicates(&staged)?;
        self.commit_measured(staged, measure)
    }

    /// Inserts one message at a position.
    ///
    /// The index is checked before anything is cloned or measured, so an
    /// out-of-range insert costs nothing and leaves no trace.
    pub fn try_insert<F, C, M>(
        &mut self,
        expected_revision: MessageListRevision,
        index: usize,
        entry: MessageListEntry,
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        self.guard_revision(expected_revision)?;
        if index > self.entries.len() {
            return Err(MessageListStateError::InvalidInsertIndex {
                index,
                len: self.entries.len(),
            }
            .into());
        }

        let mut staged = self.entries.clone();
        staged.insert(index, entry);
        reject_duplicates(&staged)?;
        self.commit_measured(staged, measure)
    }

    /// Replaces one message in place, as when streaming content into it.
    ///
    /// An entry equal to the committed one is a no-op: streaming that produces
    /// no change must not re-measure or advance the revision.
    pub fn try_update<F, C, M>(
        &mut self,
        expected_revision: MessageListRevision,
        entry: MessageListEntry,
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        self.guard_revision(expected_revision)?;

        let index = self.positions.get(&entry.message_id()).copied().ok_or(
            MessageListStateError::UnknownMessageId {
                message_id: entry.message_id(),
            },
        )?;
        if self.entries[index] == entry {
            return Ok(MessageListMutation::NoChange {
                revision: self.revision,
            });
        }

        self.update_in_place(index, entry, measure)
    }

    /// Re-measures one message and moves only the boundaries below it.
    ///
    /// Streaming rewrites the same message many times a second. Rebuilding the
    /// whole index each time would make the cost of one token proportional to
    /// the length of the transcript; a point update keeps it logarithmic.
    fn update_in_place<F, C, M>(
        &mut self,
        index: usize,
        entry: MessageListEntry,
        mut measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;
        let previous_total = self.index.total_rows()?;

        let key = entry.measure_key();
        let measured = match self.measurements.get(&key) {
            Some(cached) => cached,
            None => match measure(MessageMeasureRequest {
                entry: &entry,
                key: key.as_key(),
            }) {
                MessageMeasureOutcome::Measured(measured) => measured,
                MessageMeasureOutcome::Missing => {
                    return Err(MessageListStateError::MissingMeasurement {
                        key: Box::new(key.as_key().clone()),
                    }
                    .into());
                }
                MessageMeasureOutcome::Failed(source) => {
                    return Err(MessageListMeasureError::MeasurementFailed {
                        key: Box::new(key.as_key().clone()),
                        source,
                    });
                }
                MessageMeasureOutcome::Cancelled(source) => {
                    return Err(MessageListMeasureError::Cancelled {
                        key: Box::new(key.as_key().clone()),
                        source,
                    });
                }
            },
        };

        // Everything that can fail has now happened. From here the update is
        // applied to the committed vectors directly: cloning them would make
        // one streamed token cost as much as the whole transcript, which is the
        // opposite of what the row index is for. The index's own point update
        // computes its whole path before writing, so it cannot leave the tree
        // half-changed.
        let delta = i128::from(measured.get()) - i128::from(self.rows[index].get());
        if delta != 0 {
            self.index.checked_add_at(index, delta)?;
        }
        self.entries[index] = entry;
        self.rows[index] = measured;
        self.active_keys[index] = key.clone();

        let growth_below = {
            let structure = self.structure();
            let new_total = structure.total_rows()?;
            new_total > previous_total
                && new_total
                    > self
                        .scroll_offset
                        .get()
                        .saturating_add(self.viewport_rows.get())
        };
        let restored = {
            let structure = ListStructure {
                entries: &self.entries,
                positions: &self.positions,
                rows: &self.rows,
                index: &self.index,
            };
            self.restore_view(&structure, growth_below)?
        };

        self.measurements.insert(key, measured);
        self.scroll_offset = restored.scroll_offset;
        self.stored_anchor = restored.stored_anchor;
        self.anchor_authority = restored.anchor_authority;
        self.follow = restored.follow;
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: restored.anchor_clamped,
            viewport_clamped: restored.viewport_clamped,
        }))
    }

    /// Removes one message.
    pub fn try_remove(
        &mut self,
        expected_revision: MessageListRevision,
        message_id: MessageId,
    ) -> Result<MessageListMutation, MessageListStateError> {
        self.guard_revision(expected_revision)?;

        let index = self
            .positions
            .get(&message_id)
            .copied()
            .ok_or(MessageListStateError::UnknownMessageId { message_id })?;

        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;

        let mut entries = self.entries.clone();
        let mut rows = self.rows.clone();
        let mut keys = self.active_keys.clone();
        entries.remove(index);
        rows.remove(index);
        keys.remove(index);

        let candidate = Candidate::try_build(entries, rows, keys)?;
        let restored = self.restore_view(&candidate.structure(), false)?;

        self.commit(candidate, restored.into_parts());
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: restored.anchor_clamped,
            viewport_clamped: restored.viewport_clamped,
        }))
    }

    /// Re-lays out every message at a new width and viewport height.
    ///
    /// Width and viewport height change together on a terminal resize, but only
    /// a width change invalidates measurements. A height-only change skips both
    /// callbacks entirely.
    pub fn try_resize<F, C, R, M>(
        &mut self,
        expected_revision: MessageListRevision,
        width: u16,
        viewport_rows: ViewportRows,
        mut rebuild_config: R,
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        R: FnMut(MessageResizeConfigRequest<'_>) -> MessageResizeConfigOutcome<F, C>,
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        self.guard_revision(expected_revision)?;
        if width == 0 {
            return Err(MessageListStateError::InvalidViewportWidth { width }.into());
        }

        if width == self.width {
            if viewport_rows == self.viewport_rows {
                return Ok(MessageListMutation::NoChange {
                    revision: self.revision,
                });
            }
            return Ok(self.try_set_viewport_rows(expected_revision, viewport_rows)?);
        }

        // The revision must be able to advance before any callback runs, so a
        // saturated counter does not leave the caller having done the work of a
        // full re-measure for nothing.
        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;

        let mut staged = Vec::with_capacity(self.entries.len());
        for (message_index, entry) in self.entries.iter().enumerate() {
            let old_key = self.active_keys[message_index].clone();
            let outcome = rebuild_config(MessageResizeConfigRequest {
                message_index,
                old_entry: entry,
                old_key: old_key.as_key(),
                new_width: width,
                new_viewport_rows: viewport_rows,
            });
            let config = match outcome {
                MessageResizeConfigOutcome::Rebuilt(config) => config,
                MessageResizeConfigOutcome::Failed(source) => {
                    return Err(MessageListMeasureError::ConfigRebuildFailed {
                        message_index,
                        message_id: entry.message_id(),
                        source,
                    });
                }
                MessageResizeConfigOutcome::Cancelled(source) => {
                    return Err(MessageListMeasureError::ConfigRebuildCancelled {
                        message_index,
                        message_id: entry.message_id(),
                        source,
                    });
                }
            };
            if config.shell().outer_width() != width {
                return Err(MessageListStateError::InvalidResizeConfig {
                    message_index,
                    message_id: entry.message_id(),
                    new_width: width,
                }
                .into());
            }
            staged.push(MessageListEntry::new(
                entry.message_id(),
                entry.content_revision(),
                entry.variant(),
                entry.expansion(),
                config,
            ));
        }

        let mut measurements = self.measurements.clone();
        let (rows, active_keys) = measure_all(&staged, &mut measurements, measure)?;
        for (key, measured) in active_keys.iter().zip(&rows) {
            measurements.insert(key.clone(), *measured);
        }

        let candidate = Candidate::try_build(staged, rows, active_keys)?;
        let previous_viewport = self.viewport_rows;
        self.viewport_rows = viewport_rows;
        let restored = self.restore_view(&candidate.structure(), false);
        let restored = match restored {
            Ok(restored) => restored,
            Err(error) => {
                self.viewport_rows = previous_viewport;
                return Err(error.into());
            }
        };

        self.measurements = measurements;
        self.width = width;
        self.commit(candidate, restored.into_parts());
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: restored.anchor_clamped,
            viewport_clamped: restored.viewport_clamped,
        }))
    }

    fn commit_measured<F, C, M>(
        &mut self,
        staged: Vec<MessageListEntry>,
        measure: M,
    ) -> Result<MessageListMutation, MessageListMeasureError<F, C>>
    where
        M: FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<F, C>,
    {
        let previous_revision = self.revision;
        let applied_revision = self.revision.checked_next()?;
        let previous_total = self.index.total_rows()?;

        let mut measurements = self.measurements.clone();
        let (rows, active_keys) = measure_all(&staged, &mut measurements, measure)?;
        for (key, measured) in active_keys.iter().zip(&rows) {
            measurements.insert(key.clone(), *measured);
        }

        let candidate = Candidate::try_build(staged, rows, active_keys)?;
        let growth_below = self.growth_below_viewport(previous_total, &candidate.structure())?;
        let restored = self.restore_view(&candidate.structure(), growth_below)?;

        self.measurements = measurements;
        self.commit(candidate, restored.into_parts());
        self.revision = applied_revision;

        Ok(MessageListMutation::Applied(MessageListUpdate {
            previous_revision,
            applied_revision,
            anchor_clamped: restored.anchor_clamped,
            viewport_clamped: restored.viewport_clamped,
        }))
    }

    fn commit(&mut self, candidate: Candidate, view: CommittedView) {
        self.entries = candidate.entries;
        self.positions = candidate.positions;
        self.rows = candidate.rows;
        self.active_keys = candidate.active_keys;
        self.index = candidate.index;
        self.scroll_offset = view.scroll_offset;
        self.stored_anchor = view.stored_anchor;
        self.anchor_authority = view.anchor_authority;
        self.follow = view.follow;

        debug_assert_eq!(self.entries.len(), self.rows.len());
        debug_assert_eq!(self.entries.len(), self.active_keys.len());
        debug_assert_eq!(self.entries.len(), self.index.len());
    }
}

/// The view fields a commit installs.
pub(super) struct CommittedView {
    scroll_offset: RowOffset,
    stored_anchor: Option<crate::components::chat::message_list::view_state::MessageAnchor>,
    anchor_authority: Option<StoredAnchorAuthority>,
    follow: BottomFollowState,
}

impl super::scroll::RestoredView {
    fn into_parts(self) -> CommittedView {
        CommittedView {
            scroll_offset: self.scroll_offset,
            stored_anchor: self.stored_anchor,
            anchor_authority: self.anchor_authority,
            follow: self.follow,
        }
    }
}
