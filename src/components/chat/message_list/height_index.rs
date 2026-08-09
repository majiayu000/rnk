//! Prefix-row index over variable-height messages.
//!
//! Finding which message covers a given screen row, and updating one message's
//! height when it streams, both have to stay cheap as a transcript grows. A
//! plain prefix-sum array makes the lookup fast and the update `O(n)`; walking
//! the list makes the update fast and the lookup `O(n)`. A Fenwick tree is
//! `O(log n)` for both, which is what streaming into a 10k-message transcript
//! needs.
//!
//! This module deliberately knows nothing about messages, views, or blocks: it
//! indexes row counts by position and nothing else.

use super::error::MessageListStateError;
use super::types::MessageRows;

/// A Fenwick tree over per-message row counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FenwickRows {
    /// One-based tree; `tree[0]` is unused.
    tree: Vec<u64>,
    len: usize,
}

impl FenwickRows {
    /// Builds an index over the given heights in `O(n)`.
    pub(super) fn try_build(rows: &[MessageRows]) -> Result<Self, MessageListStateError> {
        let len = rows.len();
        let mut tree = vec![0_u64; len + 1];

        // Seed each node with its own value, then fold each node into its
        // parent once. This is the linear build; repeated point updates would
        // make it O(n log n) for no benefit.
        for (index, value) in rows.iter().enumerate() {
            tree[index + 1] = tree[index + 1]
                .checked_add(value.get())
                .ok_or(MessageListStateError::RowArithmeticOverflow)?;
        }
        for index in 1..=len {
            let parent = index + (index & index.wrapping_neg());
            if parent <= len {
                tree[parent] = tree[parent]
                    .checked_add(tree[index])
                    .ok_or(MessageListStateError::RowArithmeticOverflow)?;
            }
        }

        Ok(Self { tree, len })
    }

    /// The number of messages indexed.
    pub(super) const fn len(&self) -> usize {
        self.len
    }

    /// Total rows across `[0, index)`.
    pub(super) fn prefix_sum(&self, index: usize) -> Result<u64, MessageListStateError> {
        if index > self.len {
            return Err(MessageListStateError::RowArithmeticOverflow);
        }
        let mut total = 0_u64;
        let mut cursor = index;
        while cursor > 0 {
            total = total
                .checked_add(self.tree[cursor])
                .ok_or(MessageListStateError::RowArithmeticOverflow)?;
            cursor -= cursor & cursor.wrapping_neg();
        }
        Ok(total)
    }

    /// Total rows across every message.
    pub(super) fn total_rows(&self) -> Result<u64, MessageListStateError> {
        self.prefix_sum(self.len)
    }

    /// Adds `delta` to the message at `index`.
    ///
    /// Computed in full before anything is written. Writing as it goes would
    /// leave the tree half-updated if a later node overflowed, and a Fenwick
    /// tree with inconsistent partial sums reports positions that exist nowhere
    /// in the list.
    pub(super) fn checked_add_at(
        &mut self,
        index: usize,
        delta: i128,
    ) -> Result<(), MessageListStateError> {
        if index >= self.len {
            return Err(MessageListStateError::RowArithmeticOverflow);
        }

        // At most ceil(log2(len)) + 1 nodes are on the path to the root.
        let mut staged: Vec<(usize, u64)> = Vec::with_capacity(self.len.ilog2() as usize + 2);
        let mut cursor = index + 1;
        while cursor <= self.len {
            let updated = i128::from(self.tree[cursor])
                .checked_add(delta)
                .ok_or(MessageListStateError::RowArithmeticOverflow)?;
            let updated =
                u64::try_from(updated).map_err(|_| MessageListStateError::RowArithmeticOverflow)?;
            staged.push((cursor, updated));
            cursor += cursor & cursor.wrapping_neg();
        }

        for (node, value) in staged {
            self.tree[node] = value;
        }
        Ok(())
    }

    /// The index of the message containing global `row`.
    ///
    /// Returns the smallest `i` with `prefix_sum(i + 1) > row`. A row at or past
    /// the end returns `None` rather than clamping, so a caller cannot silently
    /// scroll past the transcript and land on the last message.
    pub(super) fn lower_bound(&self, row: u64) -> Result<Option<usize>, MessageListStateError> {
        if self.len == 0 {
            return Ok(None);
        }
        if row >= self.total_rows()? {
            return Ok(None);
        }

        // Descend the tree from the highest power of two that fits, taking each
        // branch whose subtree total still leaves `remaining` positive.
        let mut index = 0_usize;
        let mut remaining = row;
        let mut step = self.len.next_power_of_two();
        while step > 0 {
            let candidate = index + step;
            if candidate <= self.len && self.tree[candidate] <= remaining {
                remaining -= self.tree[candidate];
                index = candidate;
            }
            step /= 2;
        }
        Ok(Some(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[u64]) -> Vec<MessageRows> {
        values
            .iter()
            .map(|value| MessageRows::try_new(*value).expect("non-zero"))
            .collect()
    }

    /// The naive implementation the tree has to agree with.
    fn oracle_lower_bound(values: &[u64], row: u64) -> Option<usize> {
        let mut consumed = 0_u64;
        for (index, value) in values.iter().enumerate() {
            consumed += value;
            if row < consumed {
                return Some(index);
            }
        }
        None
    }

    #[test]
    fn prefix_sums_match_a_running_total() {
        let values = [3_u64, 5, 2, 9, 1];
        let index = FenwickRows::try_build(&rows(&values)).unwrap();

        let mut running = 0_u64;
        assert_eq!(index.prefix_sum(0).unwrap(), 0);
        for (position, value) in values.iter().enumerate() {
            running += value;
            assert_eq!(index.prefix_sum(position + 1).unwrap(), running);
        }
        assert_eq!(index.total_rows().unwrap(), running);
    }

    #[test]
    fn lower_bound_agrees_with_the_naive_scan() {
        let values = [3_u64, 5, 2, 9, 1];
        let index = FenwickRows::try_build(&rows(&values)).unwrap();
        let total: u64 = values.iter().sum();

        for row in 0..total + 3 {
            assert_eq!(
                index.lower_bound(row).unwrap(),
                oracle_lower_bound(&values, row),
                "row {row}"
            );
        }
    }

    #[test]
    fn a_point_update_moves_every_later_boundary() {
        let mut values = vec![3_u64, 5, 2];
        let mut index = FenwickRows::try_build(&rows(&values)).unwrap();

        index.checked_add_at(1, 4).unwrap();
        values[1] += 4;

        assert_eq!(index.total_rows().unwrap(), values.iter().sum::<u64>());
        for row in 0..values.iter().sum::<u64>() {
            assert_eq!(
                index.lower_bound(row).unwrap(),
                oracle_lower_bound(&values, row),
                "row {row}"
            );
        }
    }

    #[test]
    fn a_shrinking_update_is_applied_as_a_negative_delta() {
        let mut values = vec![3_u64, 8, 2];
        let mut index = FenwickRows::try_build(&rows(&values)).unwrap();

        index.checked_add_at(1, -6).unwrap();
        values[1] -= 6;

        assert_eq!(index.total_rows().unwrap(), 7);
        for row in 0..7 {
            assert_eq!(
                index.lower_bound(row).unwrap(),
                oracle_lower_bound(&values, row),
                "row {row}"
            );
        }
    }

    #[test]
    fn an_empty_index_has_no_rows_and_no_containing_message() {
        let index = FenwickRows::try_build(&[]).unwrap();
        assert_eq!(index.len(), 0);
        assert_eq!(index.total_rows().unwrap(), 0);
        assert_eq!(index.lower_bound(0).unwrap(), None);
    }

    #[test]
    fn a_row_past_the_end_has_no_containing_message() {
        let index = FenwickRows::try_build(&rows(&[2, 2])).unwrap();
        assert_eq!(index.lower_bound(3).unwrap(), Some(1));
        assert_eq!(index.lower_bound(4).unwrap(), None);
        assert_eq!(index.lower_bound(u64::MAX).unwrap(), None);
    }

    #[test]
    fn building_past_the_row_counter_fails_instead_of_wrapping() {
        let values = rows(&[u64::MAX, u64::MAX]);
        assert_eq!(
            FenwickRows::try_build(&values),
            Err(MessageListStateError::RowArithmeticOverflow)
        );
    }

    #[test]
    fn a_point_update_past_the_row_counter_fails() {
        let mut index = FenwickRows::try_build(&rows(&[u64::MAX])).unwrap();
        assert_eq!(
            index.checked_add_at(0, 1),
            Err(MessageListStateError::RowArithmeticOverflow)
        );
    }

    #[test]
    fn lookups_and_updates_stay_logarithmic() {
        // The tree must not degrade into a scan as the transcript grows: at
        // 4096 messages a linear lookup is what makes streaming stutter.
        let values = rows(&vec![1_u64; 4096]);
        let mut index = FenwickRows::try_build(&values).unwrap();
        let bound = (4096_usize.ilog2() + 2) as usize;

        assert!(count_lookup_steps(&index, 4095) <= bound);
        assert!(count_update_steps(&mut index, 0) <= bound);
    }

    fn count_lookup_steps(index: &FenwickRows, row: u64) -> usize {
        // Mirrors `lower_bound`'s descent so the count is the real one.
        let mut steps = 0;
        let mut step = index.len().next_power_of_two();
        while step > 0 {
            steps += 1;
            step /= 2;
        }
        let _ = index.lower_bound(row);
        steps
    }

    fn count_update_steps(index: &mut FenwickRows, position: usize) -> usize {
        let mut steps = 0;
        let mut cursor = position + 1;
        while cursor <= index.len() {
            steps += 1;
            cursor += cursor & cursor.wrapping_neg();
        }
        index.checked_add_at(position, 0).unwrap();
        steps
    }
}
