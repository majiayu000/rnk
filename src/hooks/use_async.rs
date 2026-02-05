//! use_async hook for async operations
//!
//! Provides state management for async operations with loading/error states.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let data = use_async(|| async {
//!         // Simulate API call
//!         tokio::time::sleep(Duration::from_secs(1)).await;
//!         Ok::<_, String>("Data loaded!".to_string())
//!     });
//!
//!     match data.state() {
//!         AsyncState::Idle => Text::new("Click to load").into_element(),
//!         AsyncState::Loading => Text::new("Loading...").into_element(),
//!         AsyncState::Success(value) => Text::new(value).into_element(),
//!         AsyncState::Error(err) => Text::new(format!("Error: {}", err)).into_element(),
//!     }
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};

/// Async operation state
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AsyncState<T, E> {
    /// Not started
    Idle,
    /// In progress
    Loading,
    /// Completed successfully
    Success(T),
    /// Failed with error
    Error(E),
}

impl<T, E> AsyncState<T, E> {
    /// Check if idle
    pub fn is_idle(&self) -> bool {
        matches!(self, AsyncState::Idle)
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        matches!(self, AsyncState::Loading)
    }

    /// Check if successful
    pub fn is_success(&self) -> bool {
        matches!(self, AsyncState::Success(_))
    }

    /// Check if error
    pub fn is_error(&self) -> bool {
        matches!(self, AsyncState::Error(_))
    }

    /// Get the success value if present
    pub fn value(&self) -> Option<&T> {
        match self {
            AsyncState::Success(v) => Some(v),
            _ => None,
        }
    }

    /// Get the error if present
    pub fn error(&self) -> Option<&E> {
        match self {
            AsyncState::Error(e) => Some(e),
            _ => None,
        }
    }
}

impl<T: Default, E> Default for AsyncState<T, E> {
    fn default() -> Self {
        AsyncState::Idle
    }
}

/// Handle for async operations
#[derive(Clone)]
pub struct AsyncHandle<T, E> {
    state: Signal<AsyncState<T, E>>,
}

impl<T, E> AsyncHandle<T, E>
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Get the current state
    pub fn state(&self) -> AsyncState<T, E> {
        self.state.get()
    }

    /// Check if idle
    pub fn is_idle(&self) -> bool {
        self.state.get().is_idle()
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.state.get().is_loading()
    }

    /// Check if successful
    pub fn is_success(&self) -> bool {
        self.state.get().is_success()
    }

    /// Check if error
    pub fn is_error(&self) -> bool {
        self.state.get().is_error()
    }

    /// Set to loading state
    pub fn set_loading(&self) {
        self.state.set(AsyncState::Loading);
    }

    /// Set to success state
    pub fn set_success(&self, value: T) {
        self.state.set(AsyncState::Success(value));
    }

    /// Set to error state
    pub fn set_error(&self, error: E) {
        self.state.set(AsyncState::Error(error));
    }

    /// Reset to idle state
    pub fn reset(&self) {
        self.state.set(AsyncState::Idle);
    }
}

/// Create an async state handle
pub fn use_async_state<T, E>() -> AsyncHandle<T, E>
where
    T: Clone + Send + Sync + Default + 'static,
    E: Clone + Send + Sync + 'static,
{
    let state = use_signal(|| AsyncState::Idle);
    AsyncHandle { state }
}

/// Create an async state handle with initial value
pub fn use_async_state_with<T, E>(initial: AsyncState<T, E>) -> AsyncHandle<T, E>
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    let state = use_signal(|| initial);
    AsyncHandle { state }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_state() {
        let state: AsyncState<String, String> = AsyncState::Idle;
        assert!(state.is_idle());
        assert!(!state.is_loading());

        let state: AsyncState<String, String> = AsyncState::Loading;
        assert!(state.is_loading());

        let state: AsyncState<String, String> = AsyncState::Success("data".to_string());
        assert!(state.is_success());
        assert_eq!(state.value(), Some(&"data".to_string()));

        let state: AsyncState<String, String> = AsyncState::Error("error".to_string());
        assert!(state.is_error());
        assert_eq!(state.error(), Some(&"error".to_string()));
    }

    #[test]
    fn test_use_async_state_compiles() {
        fn _test() {
            let handle: AsyncHandle<String, String> = use_async_state();
            handle.set_loading();
            handle.set_success("done".to_string());
            handle.reset();
        }
    }
}
