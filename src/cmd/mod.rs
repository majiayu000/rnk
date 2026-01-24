//! Command System for managing side effects
//!
//! The Command system provides a declarative way to describe side effects
//! (async operations, timers, I/O) without executing them immediately.
//! This enables better control, testability, and predictability.
//!
//! # Example
//!
//! ```rust
//! use rnk::cmd::{Cmd, CmdExecutor};
//! use std::time::Duration;
//! use tokio::sync::mpsc;
//!
//! // Create executor
//! let (tx, mut rx) = mpsc::unbounded_channel();
//! let executor = CmdExecutor::new(tx);
//!
//! // Create and execute commands
//! let cmd = Cmd::batch(vec![
//!     Cmd::sleep(Duration::from_secs(1)),
//!     Cmd::perform(|| async {
//!         println!("Task completed!");
//!     }),
//! ]);
//!
//! executor.execute(cmd);
//! ```

mod executor;

pub use executor::{CmdExecutor, RenderHandle};

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// A command represents a side effect to be executed.
///
/// Commands are descriptions of side effects, not the execution itself.
/// This allows for better control, composition, and testing.
#[derive(Default)]
pub enum Cmd {
    /// No-op command that does nothing
    #[default]
    None,

    /// Execute multiple commands in parallel
    Batch(Vec<Cmd>),

    /// Execute an async task
    Perform {
        /// The future to execute
        future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    },

    /// Sleep for a duration, then execute another command
    Sleep {
        /// Duration to sleep
        duration: Duration,
        /// Command to execute after sleeping
        then: Box<Cmd>,
    },
}

impl Cmd {
    /// Create a no-op command
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::none();
    /// ```
    pub fn none() -> Self {
        Cmd::None
    }

    /// Create a batch command that executes multiple commands in parallel
    ///
    /// Empty batches and single-item batches are optimized to avoid nesting.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::batch(vec![
    ///     Cmd::none(),
    ///     Cmd::none(),
    /// ]);
    /// ```
    pub fn batch(cmds: impl IntoIterator<Item = Cmd>) -> Self {
        let mut cmds: Vec<Cmd> = cmds
            .into_iter()
            .filter(|cmd| !matches!(cmd, Cmd::None))
            .collect();

        match cmds.len() {
            0 => Cmd::None,
            1 => cmds.pop().unwrap(),
            _ => Cmd::Batch(cmds),
        }
    }

    /// Create a command that executes an async function
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::perform(async {
    ///     println!("Hello from async!");
    /// });
    /// ```
    pub fn perform<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Cmd::Perform {
            future: Box::pin(async move { f().await }),
        }
    }

    /// Create a command that sleeps for a duration
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    /// use std::time::Duration;
    ///
    /// let cmd = Cmd::sleep(Duration::from_secs(1));
    /// ```
    pub fn sleep(duration: Duration) -> Self {
        Cmd::Sleep {
            duration,
            then: Box::new(Cmd::None),
        }
    }

    /// Chain this command with another command
    ///
    /// The next command will execute after this one completes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    /// use std::time::Duration;
    ///
    /// let cmd = Cmd::sleep(Duration::from_secs(1))
    ///     .and_then(Cmd::perform(async {
    ///         println!("After 1 second");
    ///     }));
    /// ```
    pub fn and_then(self, next: Cmd) -> Self {
        match self {
            Cmd::None => next,
            Cmd::Sleep { duration, then } => {
                let chained = then.and_then(next);
                Cmd::Sleep {
                    duration,
                    then: Box::new(chained),
                }
            }
            other => Cmd::batch(vec![other, next]),
        }
    }

    /// Check if this command is a no-op
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    ///
    /// assert!(Cmd::none().is_none());
    /// assert!(!Cmd::perform(async {}).is_none());
    /// ```
    pub fn is_none(&self) -> bool {
        matches!(self, Cmd::None)
    }

    /// Map over a command, transforming it
    ///
    /// This is useful for wrapping commands with additional behavior.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    ///
    /// let cmd = Cmd::none().map(|c| {
    ///     Cmd::batch(vec![
    ///         Cmd::perform(async { println!("Before"); }),
    ///         c,
    ///         Cmd::perform(async { println!("After"); }),
    ///     ])
    /// });
    /// ```
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }
}

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "Cmd::None"),
            Cmd::Batch(cmds) => f.debug_tuple("Cmd::Batch").field(cmds).finish(),
            Cmd::Perform { .. } => write!(f, "Cmd::Perform {{ ... }}"),
            Cmd::Sleep { duration, then } => f
                .debug_struct("Cmd::Sleep")
                .field("duration", duration)
                .field("then", then)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_none() {
        let cmd = Cmd::none();
        assert!(cmd.is_none());
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn test_cmd_default() {
        let cmd = Cmd::default();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_batch_empty() {
        let cmd = Cmd::batch(vec![]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_batch_single() {
        let cmd = Cmd::batch(vec![Cmd::none()]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_batch_filters_none() {
        let cmd = Cmd::batch(vec![Cmd::none(), Cmd::none(), Cmd::none()]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_batch_single_non_none() {
        let cmd = Cmd::batch(vec![
            Cmd::none(),
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::none(),
        ]);
        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_cmd_batch_multiple() {
        let cmd = Cmd::batch(vec![
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::sleep(Duration::from_secs(2)),
        ]);
        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
        }
    }

    #[test]
    fn test_cmd_perform() {
        let cmd = Cmd::perform(|| async {
            println!("test");
        });
        assert!(matches!(cmd, Cmd::Perform { .. }));
        assert!(!cmd.is_none());
    }

    #[test]
    fn test_cmd_sleep() {
        let duration = Duration::from_secs(1);
        let cmd = Cmd::sleep(duration);

        assert!(matches!(cmd, Cmd::Sleep { .. }));

        if let Cmd::Sleep {
            duration: d,
            then: t,
        } = cmd
        {
            assert_eq!(d, duration);
            assert!(t.is_none());
        }
    }

    #[test]
    fn test_cmd_and_then_none() {
        let cmd = Cmd::none().and_then(Cmd::sleep(Duration::from_secs(1)));
        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_cmd_and_then_sleep() {
        let cmd = Cmd::sleep(Duration::from_secs(1))
            .and_then(Cmd::sleep(Duration::from_secs(2)));

        assert!(matches!(cmd, Cmd::Sleep { .. }));

        if let Cmd::Sleep { duration, then } = cmd {
            assert_eq!(duration, Duration::from_secs(1));
            assert!(matches!(*then, Cmd::Sleep { .. }));

            if let Cmd::Sleep { duration, .. } = *then {
                assert_eq!(duration, Duration::from_secs(2));
            }
        }
    }

    #[test]
    fn test_cmd_and_then_perform() {
        let cmd =
            Cmd::perform(|| async {}).and_then(Cmd::perform(|| async {}));

        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
        }
    }

    #[test]
    fn test_cmd_and_then_chain() {
        let cmd = Cmd::sleep(Duration::from_secs(1))
            .and_then(Cmd::sleep(Duration::from_secs(2)))
            .and_then(Cmd::sleep(Duration::from_secs(3)));

        // Should create nested Sleep commands
        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_cmd_map() {
        let cmd = Cmd::none().map(|_| Cmd::sleep(Duration::from_secs(1)));
        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_cmd_map_wrap() {
        let cmd = Cmd::perform(|| async {}).map(|c| {
            Cmd::batch(vec![
                Cmd::perform(|| async { println!("before"); }),
                c,
                Cmd::perform(|| async { println!("after"); }),
            ])
        });

        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            assert_eq!(cmds.len(), 3);
        }
    }

    #[test]
    fn test_cmd_debug() {
        let cmd = Cmd::none();
        let debug_str = format!("{:?}", cmd);
        assert_eq!(debug_str, "Cmd::None");

        let cmd = Cmd::batch(vec![Cmd::none(), Cmd::none()]);
        let debug_str = format!("{:?}", cmd);
        assert_eq!(debug_str, "Cmd::None");

        let cmd = Cmd::sleep(Duration::from_secs(1));
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Cmd::Sleep"));

        let cmd = Cmd::perform(|| async {});
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Cmd::Perform"));
    }

    #[test]
    fn test_cmd_nested_batch() {
        let cmd = Cmd::batch(vec![
            Cmd::batch(vec![Cmd::sleep(Duration::from_secs(1))]),
            Cmd::batch(vec![Cmd::sleep(Duration::from_secs(2))]),
        ]);

        // Nested batches should be flattened by smart construction
        assert!(matches!(cmd, Cmd::Batch(_)));
    }

    #[test]
    fn test_cmd_complex_composition() {
        let cmd = Cmd::batch(vec![
            Cmd::sleep(Duration::from_secs(1))
                .and_then(Cmd::perform(|| async {})),
            Cmd::perform(|| async {}).and_then(Cmd::sleep(Duration::from_secs(2))),
            Cmd::none(),
        ]);

        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            // Should filter out Cmd::None
            assert_eq!(cmds.len(), 2);
        }
    }
}
