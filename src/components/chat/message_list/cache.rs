//! Bounded, deterministic reuse cache for message measurements.
//!
//! Measuring a message costs a text flow per textual child, so re-measuring
//! everything on each scroll is not viable. Keeping every measurement forever
//! is not either: a long session would grow without bound. The cache holds a
//! fixed number of recent measurements and evicts the least recently used.
//!
//! Eviction only affects what can be *reused*. The active height of every
//! committed message lives in the state's own vectors, so a cache smaller than
//! the transcript never causes a message to lose its height.

use std::collections::{BTreeMap, HashMap};

use super::key::MessageMeasureKeyHandle;
use super::types::MessageRows;

/// A fixed-capacity, least-recently-used measurement cache.
///
/// The recency order is kept in a side index rather than recomputed. Scanning
/// for the coldest entry would cost a pass over the whole cache on every
/// insert once it is full, which is a per-token cost during streaming — the
/// kind of hidden linear work this component exists to remove.
#[derive(Debug, Clone)]
pub(super) struct BoundedMeasurementCache {
    entries: HashMap<MessageMeasureKeyHandle, (MessageRows, u64)>,
    /// Use tick to key, ordered oldest first.
    recency: BTreeMap<u64, MessageMeasureKeyHandle>,
    capacity: usize,
    /// Monotonic tick used to order uses. Deterministic, unlike wall time.
    clock: u64,
}

impl BoundedMeasurementCache {
    /// Builds a cache. Capacity is validated by the caller.
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity.min(1024)),
            recency: BTreeMap::new(),
            capacity,
            clock: 0,
        }
    }

    /// Looks up a measurement, marking it as most recently used.
    ///
    /// Lookup is by full key equality, not by message id: a message whose
    /// content or width changed has a different key and must miss.
    pub(super) fn get(&mut self, key: &MessageMeasureKeyHandle) -> Option<MessageRows> {
        let tick = self.next_tick();
        let (rows, last_used) = self.entries.get_mut(key)?;
        let previous = core::mem::replace(last_used, tick);
        let rows = *rows;
        if let Some(handle) = self.recency.remove(&previous) {
            self.recency.insert(tick, handle);
        }
        Some(rows)
    }

    /// Stores a measurement, evicting the least recently used entry if full.
    pub(super) fn insert(&mut self, key: MessageMeasureKeyHandle, rows: MessageRows) {
        let tick = self.next_tick();
        if let Some((_, previous)) = self.entries.get(&key) {
            self.recency.remove(previous);
        } else if self.entries.len() >= self.capacity {
            self.evict_least_recently_used();
        }
        self.recency.insert(tick, key.clone());
        self.entries.insert(key, (rows, tick));
    }

    /// The number of measurements held.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    fn next_tick(&mut self) -> u64 {
        // Saturating rather than wrapping: a wrapped clock would make an old
        // entry look freshly used and evict a hot one instead.
        self.clock = self.clock.saturating_add(1);
        self.clock
    }

    fn evict_least_recently_used(&mut self) {
        if let Some((_, victim)) = self.recency.pop_first() {
            self.entries.remove(&victim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::chat::message_list::tests::support::entry_with_source;

    fn rows(value: u64) -> MessageRows {
        MessageRows::try_new(value).expect("non-zero")
    }

    #[test]
    fn a_stored_measurement_is_returned_for_the_same_key() {
        let mut cache = BoundedMeasurementCache::new(4);
        let key = entry_with_source(1, "hello").measure_key();

        assert_eq!(cache.get(&key), None);
        cache.insert(key.clone(), rows(3));
        assert_eq!(cache.get(&key), Some(rows(3)));
    }

    #[test]
    fn changed_content_is_a_different_key_and_misses() {
        let mut cache = BoundedMeasurementCache::new(4);
        let original = entry_with_source(1, "hello").measure_key();
        let edited = entry_with_source(1, "hello there").measure_key();

        cache.insert(original.clone(), rows(1));

        assert_eq!(cache.get(&original), Some(rows(1)));
        assert_eq!(
            cache.get(&edited),
            None,
            "a longer message reused the short message's height"
        );
    }

    #[test]
    fn the_least_recently_used_entry_is_evicted_first() {
        let mut cache = BoundedMeasurementCache::new(2);
        let first = entry_with_source(1, "one").measure_key();
        let second = entry_with_source(2, "two").measure_key();
        let third = entry_with_source(3, "three").measure_key();

        cache.insert(first.clone(), rows(1));
        cache.insert(second.clone(), rows(2));
        // Touch `first` so `second` becomes the coldest entry.
        assert_eq!(cache.get(&first), Some(rows(1)));
        cache.insert(third.clone(), rows(3));

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&first), Some(rows(1)));
        assert_eq!(cache.get(&third), Some(rows(3)));
        assert_eq!(cache.get(&second), None);
    }

    #[test]
    fn overwriting_a_key_does_not_evict() {
        let mut cache = BoundedMeasurementCache::new(2);
        let first = entry_with_source(1, "one").measure_key();
        let second = entry_with_source(2, "two").measure_key();

        cache.insert(first.clone(), rows(1));
        cache.insert(second.clone(), rows(2));
        cache.insert(first.clone(), rows(9));

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&first), Some(rows(9)));
        assert_eq!(cache.get(&second), Some(rows(2)));
    }
}
