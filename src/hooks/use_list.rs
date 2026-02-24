//! use_list hook for list state management
//!
//! Provides convenient methods for managing list state including
//! push, pop, insert, remove, and more.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn todo_app() -> Element {
//!     let todos = use_list(vec!["Buy milk", "Walk dog"]);
//!
//!     use_input(move |input, key| {
//!         if input == "a" {
//!             todos.push("New item");
//!         } else if input == "d" && !todos.is_empty() {
//!             todos.pop();
//!         }
//!     });
//!
//!     // Render todos...
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};

/// Handle for list operations
#[derive(Clone)]
pub struct ListHandle<T> {
    signal: Signal<Vec<T>>,
}

impl<T> ListHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Get a clone of the current list
    pub fn get(&self) -> Vec<T> {
        self.signal.get()
    }

    impl_collection_handle!(Vec<T>);

    /// Push an item to the end
    pub fn push(&self, item: T) {
        self.signal.update(|v| v.push(item));
    }

    /// Pop an item from the end
    pub fn pop(&self) -> Option<T> {
        let mut result = None;
        self.signal.update(|v| {
            result = v.pop();
        });
        result
    }

    /// Insert an item at the given index
    pub fn insert(&self, index: usize, item: T) {
        self.signal.update(|v| {
            if index <= v.len() {
                v.insert(index, item);
            }
        });
    }

    /// Remove an item at the given index
    pub fn remove(&self, index: usize) -> Option<T> {
        let mut result = None;
        self.signal.update(|v| {
            if index < v.len() {
                result = Some(v.remove(index));
            }
        });
        result
    }

    /// Get an item at the given index
    pub fn get_at(&self, index: usize) -> Option<T> {
        self.signal.with(|v| v.get(index).cloned())
    }

    /// Update an item at the given index
    pub fn update_at(&self, index: usize, item: T) {
        self.signal.update(|v| {
            if index < v.len() {
                v[index] = item;
            }
        });
    }

    /// Swap two items
    pub fn swap(&self, a: usize, b: usize) {
        self.signal.update(|v| {
            if a < v.len() && b < v.len() {
                v.swap(a, b);
            }
        });
    }

    /// Move an item from one index to another
    pub fn move_item(&self, from: usize, to: usize) {
        self.signal.update(|v| {
            if from < v.len() && to < v.len() && from != to {
                let item = v.remove(from);
                v.insert(to, item);
            }
        });
    }

    /// Reverse the list
    pub fn reverse(&self) {
        self.signal.update(|v| v.reverse());
    }

    /// Get the first item
    pub fn first(&self) -> Option<T> {
        self.signal.with(|v| v.first().cloned())
    }

    /// Get the last item
    pub fn last(&self) -> Option<T> {
        self.signal.with(|v| v.last().cloned())
    }

    /// Apply a function to each item
    pub fn for_each<F>(&self, f: F)
    where
        F: FnMut(&T),
    {
        self.signal.with(|v| v.iter().for_each(f));
    }
}

impl<T> ListHandle<T>
where
    T: Clone + Send + Sync + PartialEq + 'static,
{
    /// Check if the list contains an item
    pub fn contains(&self, item: &T) -> bool {
        self.signal.with(|v| v.contains(item))
    }

    /// Find the index of an item
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.signal.with(|v| v.iter().position(|x| x == item))
    }

    /// Remove the first occurrence of an item
    pub fn remove_item(&self, item: &T) -> bool {
        let mut removed = false;
        self.signal.update(|v| {
            if let Some(pos) = v.iter().position(|x| x == item) {
                v.remove(pos);
                removed = true;
            }
        });
        removed
    }
}

/// Create a list state with the given initial items
pub fn use_list<T>(initial: Vec<T>) -> ListHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    let signal = use_signal(|| initial);
    ListHandle { signal }
}

/// Create an empty list state
pub fn use_list_empty<T>() -> ListHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    use_list(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_list_compiles() {
        fn _test() {
            let list = use_list(vec![1, 2, 3]);
            list.push(4);
            list.pop();
            list.insert(0, 0);
            list.remove(0);
            list.clear();
            let _ = list.len();
            let _ = list.is_empty();
        }
    }

    #[test]
    fn test_use_list_empty_compiles() {
        fn _test() {
            let list: ListHandle<String> = use_list_empty();
            list.push("hello".to_string());
        }
    }

    #[test]
    fn test_list_operations_compile() {
        fn _test() {
            let list = use_list(vec!["a", "b", "c"]);
            list.swap(0, 1);
            list.move_item(0, 2);
            list.reverse();
            let _ = list.first();
            let _ = list.last();
            let _ = list.contains(&"a");
            let _ = list.index_of(&"b");
            list.remove_item(&"c");
        }
    }
}
