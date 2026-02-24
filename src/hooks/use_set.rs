//! use_set hook for Set state management
//!
//! Provides convenient methods for managing HashSet state.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn tags_editor() -> Element {
//!     let tags = use_set(vec!["rust", "terminal"]);
//!
//!     use_input(move |input, _| {
//!         if input == "a" {
//!             tags.insert("new-tag");
//!         } else if input == "r" {
//!             tags.remove(&"rust");
//!         }
//!     });
//!
//!     // Render tags...
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};
use std::collections::HashSet;
use std::hash::Hash;

/// Handle for set operations
#[derive(Clone)]
pub struct SetHandle<T> {
    signal: Signal<HashSet<T>>,
}

impl<T> SetHandle<T>
where
    T: Clone + Eq + Hash + Send + Sync + 'static,
{
    /// Get a clone of the current set
    pub fn get(&self) -> HashSet<T> {
        self.signal.get()
    }

    impl_collection_handle!(HashSet<T>);

    /// Insert an element
    pub fn insert(&self, value: T) -> bool {
        let mut inserted = false;
        self.signal.update(|s| {
            inserted = s.insert(value);
        });
        inserted
    }

    /// Remove an element
    pub fn remove(&self, value: &T) -> bool {
        let mut removed = false;
        self.signal.update(|s| {
            removed = s.remove(value);
        });
        removed
    }

    /// Check if the set contains an element
    pub fn contains(&self, value: &T) -> bool {
        self.signal.with(|s| s.contains(value))
    }

    /// Toggle an element (insert if absent, remove if present)
    pub fn toggle(&self, value: T) -> bool {
        let mut is_present = false;
        self.signal.update(|s| {
            if s.contains(&value) {
                s.remove(&value);
                is_present = false;
            } else {
                s.insert(value);
                is_present = true;
            }
        });
        is_present
    }

    /// Get all elements as a Vec
    pub fn to_vec(&self) -> Vec<T> {
        self.signal.with(|s| s.iter().cloned().collect())
    }

    /// Add multiple elements
    pub fn extend(&self, items: impl IntoIterator<Item = T>) {
        self.signal.update(|s| {
            s.extend(items);
        });
    }

    /// Retain only elements matching predicate
    pub fn retain<F>(&self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.signal.update(|s| s.retain(f));
    }
}

/// Create a set state with the given initial elements
pub fn use_set<T>(initial: Vec<T>) -> SetHandle<T>
where
    T: Clone + Eq + Hash + Send + Sync + 'static,
{
    let signal = use_signal(|| initial.into_iter().collect());
    SetHandle { signal }
}

/// Create an empty set state
pub fn use_set_empty<T>() -> SetHandle<T>
where
    T: Clone + Eq + Hash + Send + Sync + 'static,
{
    use_set(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_set_compiles() {
        fn _test() {
            let set = use_set(vec![1, 2, 3]);
            set.insert(4);
            set.remove(&1);
            let _ = set.contains(&2);
            let _ = set.len();
        }
    }

    #[test]
    fn test_set_toggle() {
        fn _test() {
            let set = use_set(vec!["a"]);
            set.toggle("a"); // removes
            set.toggle("b"); // inserts
        }
    }

    #[test]
    fn test_set_empty() {
        fn _test() {
            let set: SetHandle<i32> = use_set_empty();
            set.insert(1);
        }
    }
}
