//! use_previous hook for tracking previous values
//!
//! Useful for comparing current and previous values, detecting changes,
//! or implementing undo functionality.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let count = use_signal(|| 0);
//!     let prev_count = use_previous(count.get());
//!
//!     // Show change direction
//!     let direction = if count.get() > prev_count.unwrap_or(0) {
//!         "↑"
//!     } else if count.get() < prev_count.unwrap_or(0) {
//!         "↓"
//!     } else {
//!         "="
//!     };
//!
//!     Text::new(format!("Count: {} {}", count.get(), direction)).into_element()
//! }
//! ```

use crate::hooks::use_signal::use_signal;

/// Track the previous value of a variable
///
/// Returns `None` on the first render, then returns the previous value
/// on subsequent renders.
pub fn use_previous<T>(value: T) -> Option<T>
where
    T: Clone + Send + Sync + 'static,
{
    let current = use_signal(|| None::<T>);
    let previous = use_signal(|| None::<T>);

    // Get the previous value before updating
    let result = previous.get();

    // Update: previous = current, current = new value
    previous.set(current.get());
    current.set(Some(value));

    result
}

/// Track whether a value has changed since last render
pub fn use_changed<T>(value: T) -> bool
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let prev = use_previous(value.clone());
    match prev {
        Some(p) => p != value,
        None => true, // First render counts as "changed"
    }
}

/// Track the first render
pub fn use_is_first_render() -> bool {
    let is_first = use_signal(|| true);
    let result = is_first.get();
    if result {
        is_first.set(false);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_previous_compiles() {
        fn _test() {
            let _prev = use_previous(42);
        }
    }

    #[test]
    fn test_use_changed_compiles() {
        fn _test() {
            let _changed = use_changed("test".to_string());
        }
    }

    #[test]
    fn test_use_is_first_render_compiles() {
        fn _test() {
            let _is_first = use_is_first_render();
        }
    }
}
