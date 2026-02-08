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
    let state = use_signal(|| Option::<S>::None);
    if state.with(|value| value.is_none()) {
        state.set_silent(Some(init_fn()));
    }

    let reducer = Arc::new(reducer);
    let state_clone = state.clone();
    let dispatch_fn = Arc::new(move |action: A| {
        let current = state_clone
            .get()
            .expect("use_reducer_lazy: state should be initialized before dispatch");
        let new_state = reducer(&current, action);
        state_clone.set(Some(new_state));
    });

    let dispatch = Dispatch { dispatch_fn };
    let current = state
        .get()
        .expect("use_reducer_lazy: state should be initialized before read");
    (current, dispatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, RwLock};

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
    fn test_use_reducer_updates_state() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));

        let (state, dispatch) =
            with_hooks(ctx.clone(), || use_reducer(TestState { value: 0 }, test_reducer));
        assert_eq!(state.value, 0);

        dispatch.dispatch(TestAction::Add(3));

        let (state, _) = with_hooks(ctx.clone(), || {
            use_reducer(TestState { value: 999 }, test_reducer)
        });
        assert_eq!(state.value, 3);
    }

    #[test]
    fn test_use_reducer_lazy_initializes_once_and_updates_state() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));
        let init_calls = Arc::new(AtomicUsize::new(0));

        let calls = init_calls.clone();
        let (state, dispatch) = with_hooks(ctx.clone(), || {
            use_reducer_lazy(
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    TestState { value: 42 }
                },
                test_reducer,
            )
        });
        assert_eq!(state.value, 42);
        assert_eq!(init_calls.load(Ordering::SeqCst), 1);

        dispatch.dispatch(TestAction::Reset);
        let (state, _) = with_hooks(ctx.clone(), || {
            use_reducer_lazy(
                || {
                    init_calls.fetch_add(1, Ordering::SeqCst);
                    TestState { value: 999 }
                },
                test_reducer,
            )
        });
        assert_eq!(state.value, 0);
        assert_eq!(init_calls.load(Ordering::SeqCst), 1);
    }
}
