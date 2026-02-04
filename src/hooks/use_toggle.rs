//! use_toggle hook for boolean state management
//!
//! Provides a simple way to manage boolean state with toggle functionality.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let (is_visible, toggle) = use_toggle(false);
//!
//!     use_input(move |input, _| {
//!         if input == " " {
//!             toggle.toggle();
//!         }
//!     });
//!
//!     if is_visible {
//!         Text::new("Visible!").into_element()
//!     } else {
//!         Text::new("Hidden").into_element()
//!     }
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};

/// Handle for toggling boolean state
#[derive(Clone)]
pub struct ToggleHandle {
    signal: Signal<bool>,
}

impl ToggleHandle {
    /// Toggle the value
    pub fn toggle(&self) {
        self.signal.update(|v| *v = !*v);
    }

    /// Set to true
    pub fn set_true(&self) {
        self.signal.set(true);
    }

    /// Set to false
    pub fn set_false(&self) {
        self.signal.set(false);
    }

    /// Set to a specific value
    pub fn set(&self, value: bool) {
        self.signal.set(value);
    }

    /// Get the current value
    pub fn get(&self) -> bool {
        self.signal.get()
    }
}

/// Create a toggleable boolean state
///
/// Returns a tuple of (current_value, toggle_handle)
pub fn use_toggle(initial: bool) -> (bool, ToggleHandle) {
    let signal = use_signal(|| initial);
    let handle = ToggleHandle {
        signal: signal.clone(),
    };
    (signal.get(), handle)
}

/// Create a toggle that starts as false
pub fn use_toggle_off() -> (bool, ToggleHandle) {
    use_toggle(false)
}

/// Create a toggle that starts as true
pub fn use_toggle_on() -> (bool, ToggleHandle) {
    use_toggle(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_toggle_compiles() {
        fn _test() {
            let (value, handle) = use_toggle(false);
            let _ = value;
            handle.toggle();
            handle.set_true();
            handle.set_false();
        }
    }
}
