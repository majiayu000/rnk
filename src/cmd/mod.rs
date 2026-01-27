//! Command System for managing side effects
//!
//! The Command system provides a declarative way to describe side effects
//! (async operations, timers, I/O) without executing them immediately.
//! This enables better control, testability, and predictability.
//!
//! # Command Types
//!
//! - [`Cmd::none()`] - No-op command
//! - [`Cmd::perform()`] - Execute an async task
//! - [`Cmd::batch()`] - Execute multiple commands concurrently (no ordering)
//! - [`Cmd::sequence()`] - Execute multiple commands sequentially (in order)
//! - [`Cmd::sleep()`] - Sleep for a duration
//! - [`Cmd::tick()`] - Execute callback after a duration
//! - [`Cmd::every()`] - Execute callback aligned to system clock
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
//! // Create and execute commands concurrently
//! let cmd = Cmd::batch(vec![
//!     Cmd::sleep(Duration::from_secs(1)),
//!     Cmd::perform(|| async {
//!         println!("Task completed!");
//!     }),
//! ]);
//!
//! // Or execute commands sequentially
//! let cmd = Cmd::sequence(vec![
//!     Cmd::sleep(Duration::from_secs(1)),
//!     Cmd::perform(|| async {
//!         println!("After 1 second!");
//!     }),
//! ]);
//!
//! executor.execute(cmd);
//! ```

mod executor;
mod tasks;

pub use executor::{CmdExecutor, RenderHandle};
pub use tasks::{HttpRequest, HttpResponse, ProcessOutput};

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// A command represents a side effect to be executed.
///
/// Commands are descriptions of side effects, not the execution itself.
/// This allows for better control, composition, and testing.
#[derive(Default)]
pub enum Cmd {
    /// No-op command that does nothing
    #[default]
    None,

    /// Execute multiple commands concurrently (no ordering guarantees)
    Batch(Vec<Cmd>),

    /// Execute multiple commands sequentially (in order)
    Sequence(Vec<Cmd>),

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

    /// Timer tick - executes callback after duration with timestamp
    Tick {
        /// Duration to wait
        duration: Duration,
        /// Callback that receives the tick timestamp
        callback: Box<dyn FnOnce(Instant) + Send + 'static>,
    },

    /// System clock aligned tick - executes callback aligned to clock boundaries
    Every {
        /// Duration interval (aligned to system clock)
        duration: Duration,
        /// Callback that receives the tick timestamp
        callback: Box<dyn FnOnce(Instant) + Send + 'static>,
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

    /// Create a sequence command that executes multiple commands in order
    ///
    /// Unlike `batch`, which runs commands concurrently, `sequence` runs
    /// commands one at a time, waiting for each to complete before starting
    /// the next.
    ///
    /// Empty sequences and single-item sequences are optimized to avoid nesting.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    /// use std::time::Duration;
    ///
    /// let cmd = Cmd::sequence(vec![
    ///     Cmd::sleep(Duration::from_millis(100)),
    ///     Cmd::perform(|| async {
    ///         println!("After 100ms");
    ///     }),
    ///     Cmd::sleep(Duration::from_millis(100)),
    ///     Cmd::perform(|| async {
    ///         println!("After 200ms total");
    ///     }),
    /// ]);
    /// ```
    pub fn sequence(cmds: impl IntoIterator<Item = Cmd>) -> Self {
        let mut cmds: Vec<Cmd> = cmds
            .into_iter()
            .filter(|cmd| !matches!(cmd, Cmd::None))
            .collect();

        match cmds.len() {
            0 => Cmd::None,
            1 => cmds.pop().unwrap(),
            _ => Cmd::Sequence(cmds),
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

    /// Create a tick command that executes a callback after a duration
    ///
    /// The callback receives the timestamp when the tick occurred.
    /// Unlike `sleep`, `tick` is designed for timer-based updates where
    /// you need the exact time the tick fired.
    ///
    /// Note: `tick` sends a single message. To create a recurring timer,
    /// return another `tick` command from your callback.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    /// use std::time::Duration;
    /// use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
    ///
    /// let counter = Arc::new(AtomicU64::new(0));
    /// let counter_clone = counter.clone();
    ///
    /// let cmd = Cmd::tick(Duration::from_secs(1), move |_timestamp| {
    ///     counter_clone.fetch_add(1, Ordering::SeqCst);
    /// });
    /// ```
    pub fn tick<F>(duration: Duration, callback: F) -> Self
    where
        F: FnOnce(Instant) + Send + 'static,
    {
        Cmd::Tick {
            duration,
            callback: Box::new(callback),
        }
    }

    /// Create a command that ticks in sync with the system clock
    ///
    /// Unlike `tick`, which starts timing from when it's invoked, `every`
    /// aligns to system clock boundaries. For example, if you want to tick
    /// every second and the current time is 12:34:56.789, the first tick
    /// will occur at 12:34:57.000.
    ///
    /// This is useful for:
    /// - Displaying clocks that update on the second
    /// - Synchronizing multiple timers
    /// - Creating animations that align with wall clock time
    ///
    /// Note: `every` sends a single message. To create a recurring timer,
    /// return another `every` command from your callback.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::Cmd;
    /// use std::time::Duration;
    /// use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
    ///
    /// let counter = Arc::new(AtomicU64::new(0));
    /// let counter_clone = counter.clone();
    ///
    /// // Tick every second, aligned to system clock
    /// let cmd = Cmd::every(Duration::from_secs(1), move |_timestamp| {
    ///     counter_clone.fetch_add(1, Ordering::SeqCst);
    /// });
    /// ```
    pub fn every<F>(duration: Duration, callback: F) -> Self
    where
        F: FnOnce(Instant) + Send + 'static,
    {
        Cmd::Every {
            duration,
            callback: Box::new(callback),
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
            Cmd::Sequence(cmds) => f.debug_tuple("Cmd::Sequence").field(cmds).finish(),
            Cmd::Perform { .. } => write!(f, "Cmd::Perform {{ ... }}"),
            Cmd::Sleep { duration, then } => f
                .debug_struct("Cmd::Sleep")
                .field("duration", duration)
                .field("then", then)
                .finish(),
            Cmd::Tick { duration, .. } => f
                .debug_struct("Cmd::Tick")
                .field("duration", duration)
                .finish(),
            Cmd::Every { duration, .. } => f
                .debug_struct("Cmd::Every")
                .field("duration", duration)
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
        let cmd = Cmd::sleep(Duration::from_secs(1)).and_then(Cmd::sleep(Duration::from_secs(2)));

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
        let cmd = Cmd::perform(|| async {}).and_then(Cmd::perform(|| async {}));

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
                Cmd::perform(|| async {
                    println!("before");
                }),
                c,
                Cmd::perform(|| async {
                    println!("after");
                }),
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
            Cmd::sleep(Duration::from_secs(1)).and_then(Cmd::perform(|| async {})),
            Cmd::perform(|| async {}).and_then(Cmd::sleep(Duration::from_secs(2))),
            Cmd::none(),
        ]);

        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            // Should filter out Cmd::None
            assert_eq!(cmds.len(), 2);
        }
    }

    // ==================== Sequence Tests ====================

    #[test]
    fn test_cmd_sequence_empty() {
        let cmd = Cmd::sequence(vec![]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_sequence_single() {
        let cmd = Cmd::sequence(vec![Cmd::none()]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_sequence_filters_none() {
        let cmd = Cmd::sequence(vec![Cmd::none(), Cmd::none(), Cmd::none()]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cmd_sequence_single_non_none() {
        let cmd = Cmd::sequence(vec![
            Cmd::none(),
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::none(),
        ]);
        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_cmd_sequence_multiple() {
        let cmd = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::sleep(Duration::from_secs(2)),
        ]);
        assert!(matches!(cmd, Cmd::Sequence(_)));

        if let Cmd::Sequence(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
        }
    }

    #[test]
    fn test_cmd_sequence_preserves_order() {
        let cmd = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::sleep(Duration::from_secs(2)),
            Cmd::sleep(Duration::from_secs(3)),
        ]);

        if let Cmd::Sequence(cmds) = cmd {
            assert_eq!(cmds.len(), 3);
            // Verify order is preserved
            if let Cmd::Sleep { duration, .. } = &cmds[0] {
                assert_eq!(*duration, Duration::from_secs(1));
            }
            if let Cmd::Sleep { duration, .. } = &cmds[1] {
                assert_eq!(*duration, Duration::from_secs(2));
            }
            if let Cmd::Sleep { duration, .. } = &cmds[2] {
                assert_eq!(*duration, Duration::from_secs(3));
            }
        }
    }

    #[test]
    fn test_cmd_sequence_debug() {
        let cmd = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::sleep(Duration::from_secs(2)),
        ]);
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Cmd::Sequence"));
    }

    #[test]
    fn test_cmd_nested_sequence() {
        let cmd = Cmd::sequence(vec![
            Cmd::sequence(vec![Cmd::sleep(Duration::from_secs(1))]),
            Cmd::sequence(vec![Cmd::sleep(Duration::from_secs(2))]),
        ]);

        // Nested sequences should remain as Sequence
        assert!(matches!(cmd, Cmd::Sequence(_)));
    }

    #[test]
    fn test_cmd_sequence_with_batch() {
        let cmd = Cmd::sequence(vec![
            Cmd::batch(vec![
                Cmd::sleep(Duration::from_millis(100)),
                Cmd::sleep(Duration::from_millis(100)),
            ]),
            Cmd::perform(|| async {}),
        ]);

        assert!(matches!(cmd, Cmd::Sequence(_)));

        if let Cmd::Sequence(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
            assert!(matches!(cmds[0], Cmd::Batch(_)));
            assert!(matches!(cmds[1], Cmd::Perform { .. }));
        }
    }

    // ==================== Tick Tests ====================

    #[test]
    fn test_cmd_tick() {
        let duration = Duration::from_secs(1);
        let cmd = Cmd::tick(duration, |_| {});

        assert!(matches!(cmd, Cmd::Tick { .. }));

        if let Cmd::Tick {
            duration: d,
            callback: _,
        } = cmd
        {
            assert_eq!(d, duration);
        }
    }

    #[test]
    fn test_cmd_tick_debug() {
        let cmd = Cmd::tick(Duration::from_secs(1), |_| {});
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Cmd::Tick"));
        assert!(debug_str.contains("duration"));
    }

    #[test]
    fn test_cmd_tick_is_not_none() {
        let cmd = Cmd::tick(Duration::from_millis(100), |_| {});
        assert!(!cmd.is_none());
    }

    // ==================== Every Tests ====================

    #[test]
    fn test_cmd_every() {
        let duration = Duration::from_secs(1);
        let cmd = Cmd::every(duration, |_| {});

        assert!(matches!(cmd, Cmd::Every { .. }));

        if let Cmd::Every {
            duration: d,
            callback: _,
        } = cmd
        {
            assert_eq!(d, duration);
        }
    }

    #[test]
    fn test_cmd_every_debug() {
        let cmd = Cmd::every(Duration::from_secs(1), |_| {});
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Cmd::Every"));
        assert!(debug_str.contains("duration"));
    }

    #[test]
    fn test_cmd_every_is_not_none() {
        let cmd = Cmd::every(Duration::from_millis(100), |_| {});
        assert!(!cmd.is_none());
    }

    // ==================== Mixed Composition Tests ====================

    #[test]
    fn test_cmd_batch_with_tick() {
        let cmd = Cmd::batch(vec![
            Cmd::tick(Duration::from_millis(100), |_| {}),
            Cmd::tick(Duration::from_millis(200), |_| {}),
        ]);

        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
        }
    }

    #[test]
    fn test_cmd_sequence_with_tick() {
        let cmd = Cmd::sequence(vec![
            Cmd::tick(Duration::from_millis(100), |_| {}),
            Cmd::perform(|| async {}),
        ]);

        assert!(matches!(cmd, Cmd::Sequence(_)));

        if let Cmd::Sequence(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
        }
    }

    #[test]
    fn test_cmd_batch_with_every() {
        let cmd = Cmd::batch(vec![
            Cmd::every(Duration::from_secs(1), |_| {}),
            Cmd::every(Duration::from_secs(2), |_| {}),
        ]);

        assert!(matches!(cmd, Cmd::Batch(_)));

        if let Cmd::Batch(cmds) = cmd {
            assert_eq!(cmds.len(), 2);
        }
    }

    #[test]
    fn test_cmd_complex_mixed_composition() {
        let cmd = Cmd::sequence(vec![
            Cmd::batch(vec![
                Cmd::tick(Duration::from_millis(50), |_| {}),
                Cmd::perform(|| async {}),
            ]),
            Cmd::sleep(Duration::from_millis(100)),
            Cmd::every(Duration::from_secs(1), |_| {}),
        ]);

        assert!(matches!(cmd, Cmd::Sequence(_)));

        if let Cmd::Sequence(cmds) = cmd {
            assert_eq!(cmds.len(), 3);
            assert!(matches!(cmds[0], Cmd::Batch(_)));
            assert!(matches!(cmds[1], Cmd::Sleep { .. }));
            assert!(matches!(cmds[2], Cmd::Every { .. }));
        }
    }
}
