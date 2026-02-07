//! use_reducer hook for complex state management
//!
//! Provides Redux-style state management with actions and reducers.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! #[derive(Clone)]
//! struct State {
//!     count: i32,
//!     loading: bool,
//! }
//!
//! enum Action {
//!     Increment,
//!     Decrement,
//!     SetLoading(bool),
//! }
//!
//! fn reducer(state: &State, action: Action) -> State {
//!     match action {
//!         Action::Increment => State { count: state.count + 1, ..*state },
//!         Action::Decrement => State { count: state.count - 1, ..*state },
//!         Action::SetLoading(loading) => State { loading, ..*state },
//!     }
//! }
//!
//! fn app() -> Element {
//!     let (state, dispatch) = use_reducer(
//!         State { count: 0, loading: false },
//!         reducer,
//!     );
//!
//!     use_input(move |input, _| {
//!         if input == "+" {
//!             dispatch(Action::Increment);
//!         } else if input == "-" {
//!             dispatch(Action::Decrement);
//!         }
//!     });
//!
//!     Text::new(format!("Count: {}", state.count)).into_element()
//! }
//! ```

use crate::hooks::use_signal::use_signal;
use std::sync::Arc;

/// Handle for dispatching actions
#[derive(Clone)]
pub struct Dispatch<A> {
    dispatch_fn: Arc<dyn Fn(A) + Send + Sync>,
}

impl<A> Dispatch<A> {
    /// Dispatch an action
    pub fn dispatch(&self, action: A) {
        (self.dispatch_fn)(action);
    }
}

impl<A> std::ops::Deref for Dispatch<A> {
    type Target = dyn Fn(A) + Send + Sync;

    fn deref(&self) -> &Self::Target {
        &*self.dispatch_fn
    }
}

/// Create a reducer-based state
///
/// Returns the current state and a dispatch function for actions.
pub fn use_reducer<S, A, F>(initial: S, reducer: F) -> (S, Dispatch<A>)
where
    S: Clone + Send + Sync + 'static,
    A: 'static,
    F: Fn(&S, A) -> S + Send + Sync + 'static,
{
    let state = use_signal(|| initial);
    let reducer = Arc::new(reducer);

    let state_clone = state.clone();
    let dispatch_fn = Arc::new(move |action: A| {
        let current = state_clone.get();
        let new_state = reducer(&current, action);
        state_clone.set(new_state);
    });

    let dispatch = Dispatch { dispatch_fn };

    (state.get(), dispatch)
}

/// Create a reducer with lazy initial state
pub fn use_reducer_lazy<S, A, F, I>(init_fn: I, reducer: F) -> (S, Dispatch<A>)
where
    S: Clone + Send + Sync + 'static,
    A: 'static,
    F: Fn(&S, A) -> S + Send + Sync + 'static,
    I: FnOnce() -> S,
{
    use_reducer(init_fn(), reducer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Debug)]
    struct TestState {
        value: i32,
    }

    enum TestAction {
        Add(i32),
        Reset,
    }

    fn test_reducer(state: &TestState, action: TestAction) -> TestState {
        match action {
            TestAction::Add(n) => TestState {
                value: state.value + n,
            },
            TestAction::Reset => TestState { value: 0 },
        }
    }

    #[test]
    fn test_use_reducer_compiles() {
        fn _test() {
            let (state, dispatch) = use_reducer(TestState { value: 0 }, test_reducer);
            let _ = state.value;
            dispatch.dispatch(TestAction::Add(1));
            dispatch.dispatch(TestAction::Reset);
        }
        let _ = _test as fn();
    }

    #[test]
    fn test_use_reducer_lazy_compiles() {
        fn _test() {
            let (state, _dispatch) = use_reducer_lazy(|| TestState { value: 42 }, test_reducer);
            let _ = state.value;
        }
        let _ = _test as fn();
    }
}
