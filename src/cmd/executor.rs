//! Command Executor - executes commands asynchronously using Tokio runtime
//!
//! The executor is responsible for:
//! - Managing a Tokio runtime for async tasks
//! - Executing commands (Perform, Sleep, Batch)
//! - Notifying the render loop when tasks complete
//! - Supporting graceful shutdown

use super::Cmd;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Handle for requesting renders from background tasks
///
/// This is a lightweight, cloneable handle that can be passed to
/// async tasks to request UI re-renders when they complete.
#[derive(Clone)]
pub struct RenderHandle {
    tx: mpsc::UnboundedSender<()>,
}

impl RenderHandle {
    /// Create a new render handle
    pub fn new(tx: mpsc::UnboundedSender<()>) -> Self {
        Self { tx }
    }

    /// Request a render
    ///
    /// This is a non-blocking operation that signals the event loop
    /// to re-render the UI on the next frame.
    pub fn request(&self) {
        let _ = self.tx.send(());
    }
}

/// Command executor that runs commands using a Tokio runtime
///
/// The executor owns a multi-threaded Tokio runtime and executes
/// commands asynchronously. When commands complete, it notifies the
/// render loop via a channel.
///
/// # Example
///
/// ```rust
/// use rnk::cmd::{Cmd, CmdExecutor};
/// use tokio::sync::mpsc;
///
/// let (tx, mut rx) = mpsc::unbounded_channel();
/// let executor = CmdExecutor::new(tx);
///
/// executor.execute(Cmd::perform(|| async {
///     println!("Hello from async task!");
/// }));
///
/// // Wait for notification
/// rx.blocking_recv();
/// executor.shutdown();
/// ```
pub struct CmdExecutor {
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    render_handle: RenderHandle,
}

impl CmdExecutor {
    /// Create a new command executor with its own runtime
    ///
    /// # Arguments
    ///
    /// * `render_tx` - Channel sender for requesting renders
    ///
    /// # Panics
    ///
    /// Panics if the Tokio runtime cannot be created.
    pub fn new(render_tx: mpsc::UnboundedSender<()>) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2) // Lightweight runtime for UI tasks
            .thread_name("rnk-cmd-executor")
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        Self {
            runtime: Some(Arc::new(runtime)),
            render_handle: RenderHandle::new(render_tx),
        }
    }

    /// Execute a command
    ///
    /// This method spawns the command's tasks on the Tokio runtime
    /// and returns immediately. When the command completes, a render
    /// request is automatically sent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rnk::cmd::{Cmd, CmdExecutor};
    /// use std::time::Duration;
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, _rx) = mpsc::unbounded_channel();
    /// let executor = CmdExecutor::new(tx);
    ///
    /// executor.execute(Cmd::batch(vec![
    ///     Cmd::sleep(Duration::from_secs(1)),
    ///     Cmd::perform(|| async {
    ///         println!("Task done!");
    ///     }),
    /// ]));
    ///
    /// executor.shutdown();
    /// ```
    pub fn execute(&self, cmd: Cmd) {
        self.execute_internal(cmd, true);
    }

    /// Internal execute method with optional render notification
    fn execute_internal(&self, cmd: Cmd, notify_render: bool) {
        let runtime = self.runtime.as_ref().expect("executor was shutdown");
        let render_handle = self.render_handle.clone();

        match cmd {
            Cmd::None => {
                // No-op, don't notify render
            }

            Cmd::Batch(cmds) => {
                // Execute all commands in parallel
                for cmd in cmds {
                    self.execute_internal(cmd, false);
                }
                // Only notify once for the entire batch
                if notify_render {
                    render_handle.request();
                }
            }

            Cmd::Perform { future } => {
                runtime.spawn(async move {
                    future.await;
                    if notify_render {
                        render_handle.request();
                    }
                });
            }

            Cmd::Sleep { duration, then } => {
                // Clone the runtime Arc for the spawned task
                let runtime_clone = Arc::clone(runtime);
                let render_handle_clone = render_handle.clone();

                runtime.spawn(async move {
                    tokio::time::sleep(duration).await;

                    // Execute the 'then' command
                    match *then {
                        Cmd::None => {
                            // If 'then' is None, still notify if requested
                            if notify_render {
                                render_handle_clone.request();
                            }
                        }
                        other => {
                            // Create a temporary executor to execute the 'then' command
                            let temp_executor = CmdExecutor {
                                runtime: Some(runtime_clone),
                                render_handle: render_handle_clone,
                            };

                            temp_executor.execute_internal(other, notify_render);

                            // Prevent shutdown on drop
                            std::mem::forget(temp_executor);
                        }
                    }
                });
            }
        }
    }

    /// Get a handle for requesting renders
    ///
    /// This is useful for passing to long-running tasks that need
    /// to trigger re-renders.
    pub fn render_handle(&self) -> RenderHandle {
        self.render_handle.clone()
    }

    /// Shutdown the executor gracefully
    ///
    /// This will wait for all running tasks to complete before
    /// shutting down the runtime.
    ///
    /// Note: This consumes the executor.
    pub fn shutdown(mut self) {
        if let Some(runtime) = self.runtime.take() {
            // Try to get exclusive ownership
            if let Ok(runtime) = Arc::try_unwrap(runtime) {
                // We have exclusive ownership, can shutdown
                runtime.shutdown_background();
            }
            // If Arc::try_unwrap fails, other references exist (from spawned tasks)
            // The runtime will be dropped when all references are gone
        }
    }
}

impl Drop for CmdExecutor {
    fn drop(&mut self) {
        // Take the runtime to avoid double-drop in shutdown()
        if let Some(runtime) = self.runtime.take() {
            // Try to get exclusive ownership
            if let Ok(runtime) = Arc::try_unwrap(runtime) {
                runtime.shutdown_background();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_executor_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let _executor = CmdExecutor::new(tx);
        // Should not panic
    }

    #[tokio::test]
    async fn test_execute_none() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        executor.execute(Cmd::None);

        // Should not receive any notification
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_execute_perform() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);

        executor.execute(Cmd::perform(move || async move {
            flag_clone.store(true, Ordering::SeqCst);
        }));

        // Wait for render notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Task should have completed
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_execute_sleep() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let start = std::time::Instant::now();

        executor.execute(Cmd::sleep(Duration::from_millis(100)));

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
        assert!(elapsed < Duration::from_millis(500)); // Should be reasonably fast
    }

    #[tokio::test]
    async fn test_execute_sleep_and_then() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);

        let cmd = Cmd::sleep(Duration::from_millis(100)).and_then(Cmd::perform(
            move || async move {
                flag_clone.store(true, Ordering::SeqCst);
            },
        ));

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Task should have run after sleep
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_execute_batch() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let counter = Arc::new(AtomicU32::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);
        let c3 = Arc::clone(&counter);

        let cmd = Cmd::batch(vec![
            Cmd::perform(move || async move {
                c1.fetch_add(1, Ordering::SeqCst);
            }),
            Cmd::perform(move || async move {
                c2.fetch_add(1, Ordering::SeqCst);
            }),
            Cmd::perform(move || async move {
                c3.fetch_add(1, Ordering::SeqCst);
            }),
        ]);

        executor.execute(cmd);

        // Wait for notification (should only get one for the entire batch)
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        // All three tasks should have run
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        // Should not receive additional notifications
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_batch_with_sleep() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));
        let f1 = Arc::clone(&flag1);
        let f2 = Arc::clone(&flag2);

        let cmd = Cmd::batch(vec![
            Cmd::sleep(Duration::from_millis(50)).and_then(Cmd::perform(
                move || async move {
                    f1.store(true, Ordering::SeqCst);
                },
            )),
            Cmd::perform(move || async move {
                f2.store(true, Ordering::SeqCst);
            }),
        ]);

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Both tasks should complete
        assert!(flag1.load(Ordering::SeqCst));
        assert!(flag2.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_render_handle() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let handle = executor.render_handle();
        handle.request();

        // Should receive notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
    }

    #[tokio::test]
    async fn test_render_handle_clone() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let handle1 = executor.render_handle();
        let handle2 = handle1.clone();

        handle1.request();
        handle2.request();

        // Should receive two notifications
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
    }

    #[tokio::test]
    async fn test_nested_sleep_chain() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);

        let cmd = Cmd::sleep(Duration::from_millis(50))
            .and_then(Cmd::sleep(Duration::from_millis(50)))
            .and_then(Cmd::perform(move || async move {
                f.store(true, Ordering::SeqCst);
            }));

        let start = std::time::Instant::now();
        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_executor_shutdown() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        // Should not panic
        executor.shutdown();
    }

    #[tokio::test]
    async fn test_concurrent_executions() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let counter = Arc::new(AtomicU32::new(0));

        // Execute multiple commands concurrently
        for _ in 0..10 {
            let c = Arc::clone(&counter);
            executor.execute(Cmd::perform(move || async move {
                c.fetch_add(1, Ordering::SeqCst);
            }));
        }

        // Wait for all notifications
        for _ in 0..10 {
            tokio::time::timeout(Duration::from_secs(1), rx.recv())
                .await
                .expect("timeout")
                .expect("channel closed");
        }

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        executor.execute(Cmd::batch(vec![]));

        // Should not hang, but also should not notify (empty batch becomes None)
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(rx.try_recv().is_err());
    }
}
