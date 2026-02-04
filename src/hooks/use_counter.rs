//! use_counter hook for numeric state management
//!
//! Provides convenient methods for incrementing, decrementing, and
//! resetting numeric values.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let counter = use_counter(0);
//!
//!     use_input(move |_, key| {
//!         if key.up_arrow {
//!             counter.increment();
//!         } else if key.down_arrow {
//!             counter.decrement();
//!         } else if key.return_key {
//!             counter.reset();
//!         }
//!     });
//!
//!     Text::new(format!("Count: {}", counter.get())).into_element()
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};
use std::ops::{AddAssign, SubAssign};

/// Handle for counter operations
#[derive(Clone)]
pub struct CounterHandle<T> {
    signal: Signal<T>,
    initial: T,
}

impl<T> CounterHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Get the current value
    pub fn get(&self) -> T {
        self.signal.get()
    }

    /// Set to a specific value
    pub fn set(&self, value: T) {
        self.signal.set(value);
    }

    /// Reset to initial value
    pub fn reset(&self) {
        self.signal.set(self.initial.clone());
    }
}

impl<T> CounterHandle<T>
where
    T: Clone + Send + Sync + AddAssign + From<u8> + 'static,
{
    /// Increment by 1
    pub fn increment(&self) {
        self.signal.update(|v| *v += T::from(1));
    }

    /// Increment by a specific amount
    pub fn increment_by(&self, amount: T) {
        self.signal.update(|v| *v += amount);
    }
}

impl<T> CounterHandle<T>
where
    T: Clone + Send + Sync + SubAssign + From<u8> + 'static,
{
    /// Decrement by 1
    pub fn decrement(&self) {
        self.signal.update(|v| *v -= T::from(1));
    }

    /// Decrement by a specific amount
    pub fn decrement_by(&self, amount: T) {
        self.signal.update(|v| *v -= amount);
    }
}

/// Create a counter with the given initial value
pub fn use_counter<T>(initial: T) -> CounterHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    let signal = use_signal(|| initial.clone());
    CounterHandle { signal, initial }
}

/// Create a counter starting at 0
pub fn use_counter_zero() -> CounterHandle<i32> {
    use_counter(0i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_counter_compiles() {
        fn _test() {
            let counter = use_counter(0i32);
            counter.increment();
            counter.decrement();
            counter.reset();
            let _ = counter.get();
        }
    }

    #[test]
    fn test_use_counter_zero_compiles() {
        fn _test() {
            let counter = use_counter_zero();
            counter.increment();
        }
    }
}
