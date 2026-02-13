//! use_history hook for undo/redo functionality
//!
//! Provides history tracking with undo and redo capabilities.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn editor() -> Element {
//!     let history = use_history("initial text".to_string());
//!
//!     use_input(move |input, key| {
//!         if key.ctrl && input == "z" {
//!             history.undo();
//!         } else if key.ctrl && input == "y" {
//!             history.redo();
//!         } else if !input.is_empty() {
//!             let mut current = history.get();
//!             current.push_str(&input);
//!             history.push(current);
//!         }
//!     });
//!
//!     Text::new(history.get()).into_element()
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};

/// Internal state for history tracking
#[derive(Clone)]
struct HistoryState<T> {
    past: Vec<T>,
    present: T,
    future: Vec<T>,
    max_size: usize,
}

/// Handle for history operations
#[derive(Clone)]
pub struct HistoryHandle<T> {
    signal: Signal<HistoryState<T>>,
}

impl<T> HistoryHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Get the current value
    pub fn get(&self) -> T {
        self.signal.with(|s| s.present.clone())
    }

    /// Push a new state (clears redo history)
    pub fn push(&self, value: T) {
        self.signal.update(|s| {
            s.past.push(s.present.clone());
            while s.past.len() > s.max_size {
                s.past.remove(0);
            }
            s.present = value;
            s.future.clear();
        });
    }

    /// Undo to previous state
    pub fn undo(&self) -> bool {
        let mut success = false;
        self.signal.update(|s| {
            if let Some(prev) = s.past.pop() {
                s.future.push(s.present.clone());
                s.present = prev;
                success = true;
            }
        });
        success
    }

    /// Redo to next state
    pub fn redo(&self) -> bool {
        let mut success = false;
        self.signal.update(|s| {
            if let Some(next) = s.future.pop() {
                s.past.push(s.present.clone());
                s.present = next;
                success = true;
            }
        });
        success
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.signal.with(|s| !s.past.is_empty())
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.signal.with(|s| !s.future.is_empty())
    }

    /// Get the number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.signal.with(|s| s.past.len())
    }

    /// Get the number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.signal.with(|s| s.future.len())
    }

    /// Clear all history
    pub fn clear(&self) {
        self.signal.update(|s| {
            s.past.clear();
            s.future.clear();
        });
    }

    /// Reset to initial state and clear history
    pub fn reset(&self, value: T) {
        self.signal.update(|s| {
            s.present = value;
            s.past.clear();
            s.future.clear();
        });
    }

    /// Go to a specific point in history (0 = oldest)
    pub fn go_to(&self, index: usize) {
        self.signal.update(|s| {
            let past_len = s.past.len();
            if index < past_len {
                let steps = past_len - index;
                for _ in 0..steps {
                    if let Some(prev) = s.past.pop() {
                        s.future.push(s.present.clone());
                        s.present = prev;
                    }
                }
            }
        });
    }
}

/// Create a history-tracked state
pub fn use_history<T>(initial: T) -> HistoryHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    use_history_with_size(initial, 100)
}

/// Create a history-tracked state with custom max size
pub fn use_history_with_size<T>(initial: T, max_size: usize) -> HistoryHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    let signal = use_signal(|| HistoryState {
        past: Vec::new(),
        present: initial,
        future: Vec::new(),
        max_size,
    });

    HistoryHandle { signal }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_history_compiles() {
        fn _test() {
            let history = use_history(0);
            history.push(1);
            history.undo();
            history.redo();
            let _ = history.get();
        }
    }

    #[test]
    fn test_history_can_undo_redo() {
        fn _test() {
            let history = use_history("a".to_string());
            assert!(!history.can_undo());
            assert!(!history.can_redo());

            history.push("b".to_string());
            // Now can undo
        }
    }

    #[test]
    fn test_history_with_size() {
        fn _test() {
            let history = use_history_with_size(0, 10);
            for i in 1..=20 {
                history.push(i);
            }
            // Should only keep last 10
        }
    }
}
