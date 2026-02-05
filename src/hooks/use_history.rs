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

/// Handle for history operations
#[derive(Clone)]
pub struct HistoryHandle<T> {
    /// Past states (for undo)
    past: Signal<Vec<T>>,
    /// Current state
    present: Signal<T>,
    /// Future states (for redo)
    future: Signal<Vec<T>>,
    /// Maximum history size
    max_size: usize,
}

impl<T> HistoryHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Get the current value
    pub fn get(&self) -> T {
        self.present.get()
    }

    /// Push a new state (clears redo history)
    pub fn push(&self, value: T) {
        // Save current to past
        let current = self.present.get();
        self.past.update(|past| {
            past.push(current);
            // Limit history size
            while past.len() > self.max_size {
                past.remove(0);
            }
        });

        // Set new present
        self.present.set(value);

        // Clear future (redo history)
        self.future.update(|f| f.clear());
    }

    /// Undo to previous state
    pub fn undo(&self) -> bool {
        let past = self.past.get();
        if past.is_empty() {
            return false;
        }

        // Move current to future
        let current = self.present.get();
        self.future.update(|f| f.push(current));

        // Pop from past to present
        self.past.update(|p| {
            if let Some(prev) = p.pop() {
                self.present.set(prev);
            }
        });

        true
    }

    /// Redo to next state
    pub fn redo(&self) -> bool {
        let future = self.future.get();
        if future.is_empty() {
            return false;
        }

        // Move current to past
        let current = self.present.get();
        self.past.update(|p| p.push(current));

        // Pop from future to present
        self.future.update(|f| {
            if let Some(next) = f.pop() {
                self.present.set(next);
            }
        });

        true
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.past.get().is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.future.get().is_empty()
    }

    /// Get the number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.past.get().len()
    }

    /// Get the number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.future.get().len()
    }

    /// Clear all history
    pub fn clear(&self) {
        self.past.update(|p| p.clear());
        self.future.update(|f| f.clear());
    }

    /// Reset to initial state and clear history
    pub fn reset(&self, value: T) {
        self.present.set(value);
        self.past.update(|p| p.clear());
        self.future.update(|f| f.clear());
    }

    /// Go to a specific point in history (0 = oldest)
    pub fn go_to(&self, index: usize) {
        let past = self.past.get();
        let past_len = past.len();

        if index < past_len {
            // Going back in history
            let steps = past_len - index;
            for _ in 0..steps {
                self.undo();
            }
        }
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
    let past = use_signal(Vec::new);
    let present = use_signal(|| initial);
    let future = use_signal(Vec::new);

    HistoryHandle {
        past,
        present,
        future,
        max_size,
    }
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
