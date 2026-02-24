//! use_map hook for key-value state management
//!
//! Provides convenient methods for managing HashMap state.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn settings_app() -> Element {
//!     let settings = use_map(vec![
//!         ("theme", "dark"),
//!         ("font_size", "14"),
//!     ]);
//!
//!     use_input(move |input, _| {
//!         if input == "t" {
//!             let current = settings.get(&"theme").unwrap_or("light");
//!             let new_theme = if current == "dark" { "light" } else { "dark" };
//!             settings.insert("theme", new_theme);
//!         }
//!     });
//!
//!     // Render settings...
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};
use std::collections::HashMap;
use std::hash::Hash;

/// Handle for map operations
#[derive(Clone)]
pub struct MapHandle<K, V> {
    signal: Signal<HashMap<K, V>>,
}

impl<K, V> MapHandle<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Get a clone of the current map
    pub fn get_all(&self) -> HashMap<K, V> {
        self.signal.get()
    }

    impl_collection_handle!(HashMap<K, V>);

    /// Get a value by key
    pub fn get(&self, key: &K) -> Option<V> {
        self.signal.with(|m| m.get(key).cloned())
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut result = None;
        self.signal.update(|m| {
            result = m.insert(key, value);
        });
        result
    }

    /// Remove a key-value pair
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut result = None;
        self.signal.update(|m| {
            result = m.remove(key);
        });
        result
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &K) -> bool {
        self.signal.with(|m| m.contains_key(key))
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<K> {
        self.signal.with(|m| m.keys().cloned().collect())
    }

    /// Get all values
    pub fn values(&self) -> Vec<V> {
        self.signal.with(|m| m.values().cloned().collect())
    }

    /// Update a value if the key exists
    pub fn update_value<F>(&self, key: &K, f: F)
    where
        F: FnOnce(&mut V),
    {
        self.signal.update(|m| {
            if let Some(v) = m.get_mut(key) {
                f(v);
            }
        });
    }

    /// Get or insert a default value
    pub fn get_or_insert(&self, key: K, default: V) -> V {
        let mut result = default.clone();
        self.signal.update(|m| {
            result = m.entry(key).or_insert(default).clone();
        });
        result
    }

    /// Merge another map into this one
    pub fn merge(&self, other: HashMap<K, V>) {
        self.signal.update(|m| {
            m.extend(other);
        });
    }
}

/// Create a map state with the given initial entries
pub fn use_map<K, V>(initial: Vec<(K, V)>) -> MapHandle<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    let signal = use_signal(|| initial.into_iter().collect());
    MapHandle { signal }
}

/// Create an empty map state
pub fn use_map_empty<K, V>() -> MapHandle<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    use_map(Vec::new())
}

/// Create a map state from a HashMap
pub fn use_map_from<K, V>(map: HashMap<K, V>) -> MapHandle<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    let signal = use_signal(|| map);
    MapHandle { signal }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_map_compiles() {
        fn _test() {
            let map = use_map(vec![("a", 1), ("b", 2)]);
            map.insert("c", 3);
            map.remove(&"a");
            let _ = map.get(&"b");
            let _ = map.len();
            let _ = map.is_empty();
            let _ = map.contains_key(&"c");
        }
    }

    #[test]
    fn test_use_map_empty_compiles() {
        fn _test() {
            let map: MapHandle<String, i32> = use_map_empty();
            map.insert("key".to_string(), 42);
        }
    }

    #[test]
    fn test_map_operations_compile() {
        fn _test() {
            let map = use_map(vec![("x", 10)]);
            let _ = map.keys();
            let _ = map.values();
            map.update_value(&"x", |v| *v += 1);
            let _ = map.get_or_insert("y", 20);
            map.clear();
        }
    }
}
