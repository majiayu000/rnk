//! Typed message system for Cmd
//!
//! This module provides a typed message system inspired by Bubbletea's Msg pattern.
//! Commands can now return typed messages that get dispatched back to components.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::cmd::{TypedCmd, AppMsg};
//!
//! enum MyMsg {
//!     DataLoaded(Vec<String>),
//!     Error(String),
//! }
//!
//! // Create a command that returns a typed message
//! let cmd: TypedCmd<MyMsg> = TypedCmd::perform(|| async {
//!     match fetch_data().await {
//!         Ok(data) => MyMsg::DataLoaded(data),
//!         Err(e) => MyMsg::Error(e.to_string()),
//!     }
//! });
//! ```

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use super::{ExecConfig, ExecResult};

/// Application-level messages (similar to Bubbletea's Msg)
///
/// These are built-in message types that the framework handles automatically.
#[derive(Debug, Clone, Default)]
pub enum AppMsg {
    /// Window/terminal resize event
    WindowResize { width: u16, height: u16 },

    /// Keyboard input (raw key string)
    KeyInput(String),

    /// Timer tick with timestamp
    Tick(Instant),

    /// Focus changed to a new element
    FocusChanged(Option<String>),

    /// Blur event (element lost focus)
    Blur,

    /// No-op message (useful for conditional returns)
    #[default]
    None,
}

/// A boxed message that can hold any type
///
/// This allows components to define their own message types while
/// still being compatible with the framework's message dispatch system.
pub struct BoxedMsg(Box<dyn Any + Send + 'static>);

impl BoxedMsg {
    /// Create a new boxed message
    pub fn new<M: Any + Send + 'static>(msg: M) -> Self {
        BoxedMsg(Box::new(msg))
    }

    /// Try to downcast to a specific message type
    pub fn downcast<M: Any + 'static>(self) -> Result<M, Self> {
        match self.0.downcast::<M>() {
            Ok(msg) => Ok(*msg),
            Err(boxed) => Err(BoxedMsg(boxed)),
        }
    }

    /// Try to get a reference to the inner message
    pub fn downcast_ref<M: Any + 'static>(&self) -> Option<&M> {
        self.0.downcast_ref::<M>()
    }

    /// Check if this message is of a specific type
    pub fn is<M: Any + 'static>(&self) -> bool {
        self.0.is::<M>()
    }
}

impl std::fmt::Debug for BoxedMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxedMsg(...)")
    }
}

/// A typed command that produces a message of type `M`
///
/// This is the generic version of `Cmd` that allows commands to return
/// typed messages, enabling type-safe message passing between components.
///
/// # Type Parameter
///
/// - `M`: The message type this command produces. Must be `Send + 'static`.
///
/// # Example
///
/// ```rust,ignore
/// enum CounterMsg {
///     Increment,
///     Decrement,
///     Reset,
/// }
///
/// fn load_initial_count() -> TypedCmd<CounterMsg> {
///     TypedCmd::perform(|| async {
///         // Simulate loading from storage
///         CounterMsg::Reset
///     })
/// }
/// ```
#[derive(Default)]
pub enum TypedCmd<M = ()>
where
    M: Send + 'static,
{
    /// No-op command that produces no message
    #[default]
    None,

    /// Execute multiple commands concurrently
    Batch(Vec<TypedCmd<M>>),

    /// Execute multiple commands sequentially
    Sequence(Vec<TypedCmd<M>>),

    /// Execute an async task that produces a message
    Perform {
        /// The future that produces the message
        future: Pin<Box<dyn Future<Output = M> + Send + 'static>>,
    },

    /// Sleep for a duration, then produce a message
    Sleep {
        /// Duration to sleep
        duration: Duration,
        /// Message to produce after sleeping
        then: Box<TypedCmd<M>>,
    },

    /// Timer tick - produces a message after duration
    Tick {
        /// Duration to wait
        duration: Duration,
        /// Function that creates the message from the tick timestamp
        msg_fn: Box<dyn FnOnce(Instant) -> M + Send + 'static>,
    },

    /// System clock aligned tick
    Every {
        /// Duration interval (aligned to system clock)
        duration: Duration,
        /// Function that creates the message from the tick timestamp
        msg_fn: Box<dyn FnOnce(Instant) -> M + Send + 'static>,
    },

    /// Execute an external interactive process
    Exec {
        /// Configuration for the external process
        config: ExecConfig,
        /// Function that creates the message from the result
        msg_fn: Box<dyn FnOnce(ExecResult) -> M + Send + 'static>,
    },
}

impl<M> TypedCmd<M>
where
    M: Send + 'static,
{
    /// Create a no-op command
    pub fn none() -> Self {
        TypedCmd::None
    }

    /// Create a batch command that executes multiple commands concurrently
    pub fn batch(cmds: impl IntoIterator<Item = TypedCmd<M>>) -> Self {
        let mut cmds: Vec<TypedCmd<M>> = cmds
            .into_iter()
            .filter(|cmd| !matches!(cmd, TypedCmd::None))
            .collect();

        match cmds.len() {
            0 => TypedCmd::None,
            1 => cmds.pop().unwrap(),
            _ => TypedCmd::Batch(cmds),
        }
    }

    /// Create a sequence command that executes multiple commands in order
    pub fn sequence(cmds: impl IntoIterator<Item = TypedCmd<M>>) -> Self {
        let mut cmds: Vec<TypedCmd<M>> = cmds
            .into_iter()
            .filter(|cmd| !matches!(cmd, TypedCmd::None))
            .collect();

        match cmds.len() {
            0 => TypedCmd::None,
            1 => cmds.pop().unwrap(),
            _ => TypedCmd::Sequence(cmds),
        }
    }

    /// Create a command that executes an async function and produces a message
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// enum Msg {
    ///     DataLoaded(String),
    /// }
    ///
    /// let cmd = TypedCmd::perform(|| async {
    ///     let data = fetch_data().await;
    ///     Msg::DataLoaded(data)
    /// });
    /// ```
    pub fn perform<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = M> + Send + 'static,
    {
        TypedCmd::Perform {
            future: Box::pin(async move { f().await }),
        }
    }

    /// Create a command that sleeps for a duration
    pub fn sleep(duration: Duration) -> Self {
        TypedCmd::Sleep {
            duration,
            then: Box::new(TypedCmd::None),
        }
    }

    /// Create a tick command that produces a message after a duration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// enum Msg {
    ///     TimerTick(Instant),
    /// }
    ///
    /// let cmd = TypedCmd::tick(Duration::from_secs(1), |t| Msg::TimerTick(t));
    /// ```
    pub fn tick<F>(duration: Duration, msg_fn: F) -> Self
    where
        F: FnOnce(Instant) -> M + Send + 'static,
    {
        TypedCmd::Tick {
            duration,
            msg_fn: Box::new(msg_fn),
        }
    }

    /// Create a command that ticks in sync with the system clock
    pub fn every<F>(duration: Duration, msg_fn: F) -> Self
    where
        F: FnOnce(Instant) -> M + Send + 'static,
    {
        TypedCmd::Every {
            duration,
            msg_fn: Box::new(msg_fn),
        }
    }

    /// Execute an external interactive process
    pub fn exec<F>(config: ExecConfig, msg_fn: F) -> Self
    where
        F: FnOnce(ExecResult) -> M + Send + 'static,
    {
        TypedCmd::Exec {
            config,
            msg_fn: Box::new(msg_fn),
        }
    }

    /// Check if this command is a no-op
    pub fn is_none(&self) -> bool {
        matches!(self, TypedCmd::None)
    }

    /// Chain this command with another command
    pub fn and_then(self, next: TypedCmd<M>) -> Self {
        match self {
            TypedCmd::None => next,
            TypedCmd::Sleep { duration, then } => {
                let chained = then.and_then(next);
                TypedCmd::Sleep {
                    duration,
                    then: Box::new(chained),
                }
            }
            other => TypedCmd::batch(vec![other, next]),
        }
    }

    /// Map the message type to a different type
    ///
    /// This is useful for composing commands from child components
    /// into parent component message types.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// enum ChildMsg {
    ///     Clicked,
    /// }
    ///
    /// enum ParentMsg {
    ///     Child(ChildMsg),
    /// }
    ///
    /// let child_cmd: TypedCmd<ChildMsg> = TypedCmd::perform(|| async { ChildMsg::Clicked });
    /// let parent_cmd: TypedCmd<ParentMsg> = child_cmd.map(ParentMsg::Child);
    /// ```
    pub fn map<N, F>(self, f: F) -> TypedCmd<N>
    where
        N: Send + 'static,
        F: FnOnce(M) -> N + Send + 'static + Clone,
    {
        match self {
            TypedCmd::None => TypedCmd::None,
            TypedCmd::Batch(cmds) => {
                TypedCmd::Batch(cmds.into_iter().map(|c| c.map(f.clone())).collect())
            }
            TypedCmd::Sequence(cmds) => {
                TypedCmd::Sequence(cmds.into_iter().map(|c| c.map(f.clone())).collect())
            }
            TypedCmd::Perform { future } => TypedCmd::Perform {
                future: Box::pin(async move {
                    let msg = future.await;
                    f(msg)
                }),
            },
            TypedCmd::Sleep { duration, then } => TypedCmd::Sleep {
                duration,
                then: Box::new(then.map(f)),
            },
            TypedCmd::Tick { duration, msg_fn } => TypedCmd::Tick {
                duration,
                msg_fn: Box::new(move |t| f(msg_fn(t))),
            },
            TypedCmd::Every { duration, msg_fn } => TypedCmd::Every {
                duration,
                msg_fn: Box::new(move |t| f(msg_fn(t))),
            },
            TypedCmd::Exec { config, msg_fn } => TypedCmd::Exec {
                config,
                msg_fn: Box::new(move |r| f(msg_fn(r))),
            },
        }
    }
}

impl<M> std::fmt::Debug for TypedCmd<M>
where
    M: Send + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedCmd::None => write!(f, "TypedCmd::None"),
            TypedCmd::Batch(cmds) => f.debug_tuple("TypedCmd::Batch").field(cmds).finish(),
            TypedCmd::Sequence(cmds) => f.debug_tuple("TypedCmd::Sequence").field(cmds).finish(),
            TypedCmd::Perform { .. } => write!(f, "TypedCmd::Perform {{ ... }}"),
            TypedCmd::Sleep { duration, then } => f
                .debug_struct("TypedCmd::Sleep")
                .field("duration", duration)
                .field("then", then)
                .finish(),
            TypedCmd::Tick { duration, .. } => f
                .debug_struct("TypedCmd::Tick")
                .field("duration", duration)
                .finish(),
            TypedCmd::Every { duration, .. } => f
                .debug_struct("TypedCmd::Every")
                .field("duration", duration)
                .finish(),
            TypedCmd::Exec { config, .. } => f
                .debug_struct("TypedCmd::Exec")
                .field("config", config)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum TestMsg {
        Loaded(String),
        Tick(u64),
    }

    #[test]
    fn test_typed_cmd_none() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::none();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_typed_cmd_default() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::default();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_typed_cmd_batch_empty() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::batch(vec![]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_typed_cmd_batch_single() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::batch(vec![TypedCmd::none()]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_typed_cmd_batch_multiple() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::batch(vec![
            TypedCmd::sleep(Duration::from_secs(1)),
            TypedCmd::sleep(Duration::from_secs(2)),
        ]);
        assert!(matches!(cmd, TypedCmd::Batch(_)));
    }

    #[test]
    fn test_typed_cmd_sequence_empty() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::sequence(vec![]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_typed_cmd_sequence_multiple() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::sequence(vec![
            TypedCmd::sleep(Duration::from_secs(1)),
            TypedCmd::sleep(Duration::from_secs(2)),
        ]);
        assert!(matches!(cmd, TypedCmd::Sequence(_)));
    }

    #[test]
    fn test_typed_cmd_perform() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::perform(|| async { TestMsg::Loaded("data".into()) });
        assert!(matches!(cmd, TypedCmd::Perform { .. }));
    }

    #[test]
    fn test_typed_cmd_tick() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::tick(Duration::from_secs(1), |_| TestMsg::Tick(1));
        assert!(matches!(cmd, TypedCmd::Tick { .. }));
    }

    #[test]
    fn test_typed_cmd_every() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::every(Duration::from_secs(1), |_| TestMsg::Tick(1));
        assert!(matches!(cmd, TypedCmd::Every { .. }));
    }

    #[test]
    fn test_typed_cmd_and_then() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::sleep(Duration::from_secs(1))
            .and_then(TypedCmd::sleep(Duration::from_secs(2)));
        assert!(matches!(cmd, TypedCmd::Sleep { .. }));
    }

    #[test]
    fn test_typed_cmd_debug() {
        let cmd: TypedCmd<TestMsg> = TypedCmd::none();
        let debug_str = format!("{:?}", cmd);
        assert_eq!(debug_str, "TypedCmd::None");
    }

    #[test]
    fn test_app_msg_default() {
        let msg = AppMsg::default();
        assert!(matches!(msg, AppMsg::None));
    }

    #[test]
    fn test_boxed_msg() {
        let msg = BoxedMsg::new(TestMsg::Loaded("test".into()));
        assert!(msg.is::<TestMsg>());

        let result = msg.downcast::<TestMsg>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestMsg::Loaded("test".into()));
    }

    #[test]
    fn test_boxed_msg_wrong_type() {
        let msg = BoxedMsg::new(TestMsg::Loaded("test".into()));
        let result = msg.downcast::<String>();
        assert!(result.is_err());
    }

    #[test]
    fn test_boxed_msg_downcast_ref() {
        let msg = BoxedMsg::new(TestMsg::Tick(42));
        let ref_opt = msg.downcast_ref::<TestMsg>();
        assert!(ref_opt.is_some());
        assert_eq!(ref_opt.unwrap(), &TestMsg::Tick(42));
    }

    #[derive(Debug, PartialEq)]
    enum ParentMsg {
        Child(TestMsg),
    }

    #[test]
    fn test_typed_cmd_map() {
        let child_cmd: TypedCmd<TestMsg> =
            TypedCmd::perform(|| async { TestMsg::Loaded("data".into()) });

        let parent_cmd: TypedCmd<ParentMsg> = child_cmd.map(ParentMsg::Child);

        assert!(matches!(parent_cmd, TypedCmd::Perform { .. }));
    }

    #[test]
    fn test_typed_cmd_map_batch() {
        let child_cmd: TypedCmd<TestMsg> = TypedCmd::batch(vec![
            TypedCmd::sleep(Duration::from_secs(1)),
            TypedCmd::sleep(Duration::from_secs(2)),
        ]);

        let parent_cmd: TypedCmd<ParentMsg> = child_cmd.map(ParentMsg::Child);

        assert!(matches!(parent_cmd, TypedCmd::Batch(_)));
    }

    #[test]
    fn test_typed_cmd_map_none() {
        let child_cmd: TypedCmd<TestMsg> = TypedCmd::none();
        let parent_cmd: TypedCmd<ParentMsg> = child_cmd.map(ParentMsg::Child);
        assert!(parent_cmd.is_none());
    }
}
