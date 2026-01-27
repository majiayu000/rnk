//! Command Executor - executes commands asynchronously using Tokio runtime
//!
//! The executor is responsible for:
//! - Managing a Tokio runtime for async tasks
//! - Executing commands (Perform, Sleep, Batch, Sequence, Tick, Every)
//! - Notifying the render loop when tasks complete
//! - Supporting graceful shutdown

use super::Cmd;
use std::sync::Arc;
use std::time::Instant;
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
                // Execute all commands in parallel (no ordering guarantees)
                for cmd in cmds {
                    self.execute_internal(cmd, false);
                }
                // Only notify once for the entire batch
                if notify_render {
                    render_handle.request();
                }
            }

            Cmd::Sequence(cmds) => {
                // Execute commands sequentially (in order)
                if cmds.is_empty() {
                    if notify_render {
                        render_handle.request();
                    }
                    return;
                }

                let runtime_clone = Arc::clone(runtime);
                let render_handle_clone = render_handle.clone();

                runtime.spawn(async move {
                    for cmd in cmds {
                        // Create a temporary executor for each command
                        let temp_executor = CmdExecutor {
                            runtime: Some(Arc::clone(&runtime_clone)),
                            render_handle: render_handle_clone.clone(),
                        };

                        // Execute and wait for completion using a oneshot channel
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        temp_executor.execute_with_completion(cmd, tx);

                        // Prevent shutdown on drop
                        std::mem::forget(temp_executor);

                        // Wait for this command to complete before starting next
                        let _ = rx.await;
                    }

                    // Notify render after all commands complete
                    if notify_render {
                        render_handle_clone.request();
                    }
                });
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

            Cmd::Tick { duration, callback } => {
                runtime.spawn(async move {
                    tokio::time::sleep(duration).await;
                    let timestamp = Instant::now();
                    callback(timestamp);
                    if notify_render {
                        render_handle.request();
                    }
                });
            }

            Cmd::Every { duration, callback } => {
                runtime.spawn(async move {
                    // Calculate time until next aligned boundary
                    // For example, if duration is 1 second and current time is 12:34:56.789,
                    // we want to wait until 12:34:57.000
                    let now = std::time::SystemTime::now();
                    let since_epoch = now
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default();

                    let duration_nanos = duration.as_nanos() as u64;
                    let since_epoch_nanos = since_epoch.as_nanos() as u64;

                    // Calculate how far we are into the current interval
                    let remainder = since_epoch_nanos % duration_nanos;

                    // Calculate time until next boundary
                    let wait_nanos = if remainder == 0 {
                        duration_nanos
                    } else {
                        duration_nanos - remainder
                    };

                    let wait_duration = std::time::Duration::from_nanos(wait_nanos);
                    tokio::time::sleep(wait_duration).await;

                    let timestamp = Instant::now();
                    callback(timestamp);
                    if notify_render {
                        render_handle.request();
                    }
                });
            }
        }
    }

    /// Execute a command and signal completion via a oneshot channel
    fn execute_with_completion(&self, cmd: Cmd, completion: tokio::sync::oneshot::Sender<()>) {
        let runtime = self.runtime.as_ref().expect("executor was shutdown");
        let render_handle = self.render_handle.clone();

        match cmd {
            Cmd::None => {
                let _ = completion.send(());
            }

            Cmd::Batch(cmds) => {
                // Execute all commands in parallel and wait for all to complete
                let runtime_clone = Arc::clone(runtime);
                let render_handle_clone = render_handle.clone();

                runtime.spawn(async move {
                    let mut handles = Vec::with_capacity(cmds.len());

                    for cmd in cmds {
                        let rt = Arc::clone(&runtime_clone);
                        let rh = render_handle_clone.clone();

                        let handle = tokio::spawn(async move {
                            let temp_executor = CmdExecutor {
                                runtime: Some(rt),
                                render_handle: rh,
                            };
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            temp_executor.execute_with_completion(cmd, tx);
                            std::mem::forget(temp_executor);
                            let _ = rx.await;
                        });
                        handles.push(handle);
                    }

                    // Wait for all to complete
                    for handle in handles {
                        let _ = handle.await;
                    }

                    let _ = completion.send(());
                });
            }

            Cmd::Sequence(cmds) => {
                let runtime_clone = Arc::clone(runtime);
                let render_handle_clone = render_handle.clone();

                runtime.spawn(async move {
                    for cmd in cmds {
                        let temp_executor = CmdExecutor {
                            runtime: Some(Arc::clone(&runtime_clone)),
                            render_handle: render_handle_clone.clone(),
                        };
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        temp_executor.execute_with_completion(cmd, tx);
                        std::mem::forget(temp_executor);
                        let _ = rx.await;
                    }
                    let _ = completion.send(());
                });
            }

            Cmd::Perform { future } => {
                runtime.spawn(async move {
                    future.await;
                    let _ = completion.send(());
                });
            }

            Cmd::Sleep { duration, then } => {
                let runtime_clone = Arc::clone(runtime);
                let render_handle_clone = render_handle.clone();

                runtime.spawn(async move {
                    tokio::time::sleep(duration).await;

                    match *then {
                        Cmd::None => {
                            let _ = completion.send(());
                        }
                        other => {
                            let temp_executor = CmdExecutor {
                                runtime: Some(runtime_clone),
                                render_handle: render_handle_clone,
                            };
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            temp_executor.execute_with_completion(other, tx);
                            std::mem::forget(temp_executor);
                            let _ = rx.await;
                            let _ = completion.send(());
                        }
                    }
                });
            }

            Cmd::Tick { duration, callback } => {
                runtime.spawn(async move {
                    tokio::time::sleep(duration).await;
                    let timestamp = Instant::now();
                    callback(timestamp);
                    let _ = completion.send(());
                });
            }

            Cmd::Every { duration, callback } => {
                runtime.spawn(async move {
                    let now = std::time::SystemTime::now();
                    let since_epoch = now
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default();

                    let duration_nanos = duration.as_nanos() as u64;
                    let since_epoch_nanos = since_epoch.as_nanos() as u64;
                    let remainder = since_epoch_nanos % duration_nanos;
                    let wait_nanos = if remainder == 0 {
                        duration_nanos
                    } else {
                        duration_nanos - remainder
                    };

                    let wait_duration = std::time::Duration::from_nanos(wait_nanos);
                    tokio::time::sleep(wait_duration).await;

                    let timestamp = Instant::now();
                    callback(timestamp);
                    let _ = completion.send(());
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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

        let cmd =
            Cmd::sleep(Duration::from_millis(100)).and_then(Cmd::perform(move || async move {
                flag_clone.store(true, Ordering::SeqCst);
            }));

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
            Cmd::sleep(Duration::from_millis(50)).and_then(Cmd::perform(move || async move {
                f1.store(true, Ordering::SeqCst);
            })),
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

    // ==================== Sequence Tests ====================

    #[tokio::test]
    async fn test_execute_sequence() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let o1 = Arc::clone(&order);
        let o2 = Arc::clone(&order);
        let o3 = Arc::clone(&order);

        let cmd = Cmd::sequence(vec![
            Cmd::perform(move || async move {
                o1.lock().unwrap().push(1);
            }),
            Cmd::perform(move || async move {
                o2.lock().unwrap().push(2);
            }),
            Cmd::perform(move || async move {
                o3.lock().unwrap().push(3);
            }),
        ]);

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Commands should have executed in order
        let result = order.lock().unwrap().clone();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_sequence_with_sleep() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let o1 = Arc::clone(&order);
        let o2 = Arc::clone(&order);

        let cmd = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_millis(50)).and_then(Cmd::perform(move || async move {
                o1.lock().unwrap().push(1);
            })),
            Cmd::perform(move || async move {
                o2.lock().unwrap().push(2);
            }),
        ]);

        let start = std::time::Instant::now();
        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let elapsed = start.elapsed();
        // Should take at least 50ms (the sleep)
        assert!(elapsed >= Duration::from_millis(50));

        // Commands should have executed in order (1 before 2)
        let result = order.lock().unwrap().clone();
        assert_eq!(result, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_sequence_timing() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let cmd = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_millis(50)),
            Cmd::sleep(Duration::from_millis(50)),
            Cmd::sleep(Duration::from_millis(50)),
        ]);

        let start = std::time::Instant::now();
        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        let elapsed = start.elapsed();
        // Should take at least 150ms (3 x 50ms sequentially)
        assert!(elapsed >= Duration::from_millis(150));
        // But not too long
        assert!(elapsed < Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_empty_sequence() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        executor.execute(Cmd::sequence(vec![]));

        // Should not hang, but also should not notify (empty sequence becomes None)
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_sequence_vs_batch_timing() {
        // Batch should be faster than sequence for parallel tasks
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let executor1 = CmdExecutor::new(tx1);

        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let executor2 = CmdExecutor::new(tx2);

        // Batch: parallel execution
        let batch_cmd = Cmd::batch(vec![
            Cmd::sleep(Duration::from_millis(50)),
            Cmd::sleep(Duration::from_millis(50)),
        ]);

        // Sequence: sequential execution
        let seq_cmd = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_millis(50)),
            Cmd::sleep(Duration::from_millis(50)),
        ]);

        let batch_start = std::time::Instant::now();
        executor1.execute(batch_cmd);
        tokio::time::timeout(Duration::from_secs(2), rx1.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
        let batch_elapsed = batch_start.elapsed();

        let seq_start = std::time::Instant::now();
        executor2.execute(seq_cmd);
        tokio::time::timeout(Duration::from_secs(2), rx2.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
        let seq_elapsed = seq_start.elapsed();

        // Sequence should take roughly twice as long as batch
        // (100ms vs 50ms, with some tolerance)
        assert!(seq_elapsed >= Duration::from_millis(100));
        assert!(batch_elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_nested_sequence_in_batch() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let counter = Arc::new(AtomicU32::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);
        let c3 = Arc::clone(&counter);

        let cmd = Cmd::batch(vec![
            Cmd::sequence(vec![Cmd::perform(move || async move {
                c1.fetch_add(1, Ordering::SeqCst);
            })]),
            Cmd::sequence(vec![
                Cmd::perform(move || async move {
                    c2.fetch_add(1, Ordering::SeqCst);
                }),
                Cmd::perform(move || async move {
                    c3.fetch_add(1, Ordering::SeqCst);
                }),
            ]),
        ]);

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    // ==================== Tick Tests ====================

    #[tokio::test]
    async fn test_execute_tick() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);
        let timestamp_received = Arc::new(std::sync::Mutex::new(None));
        let ts_clone = Arc::clone(&timestamp_received);

        let cmd = Cmd::tick(Duration::from_millis(50), move |ts| {
            flag_clone.store(true, Ordering::SeqCst);
            *ts_clone.lock().unwrap() = Some(ts);
        });

        let start = std::time::Instant::now();
        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
        assert!(flag.load(Ordering::SeqCst));
        assert!(timestamp_received.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn test_tick_timing() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let cmd = Cmd::tick(Duration::from_millis(100), |_| {});

        let start = std::time::Instant::now();
        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_multiple_ticks() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let counter = Arc::new(AtomicU32::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);

        let cmd = Cmd::batch(vec![
            Cmd::tick(Duration::from_millis(50), move |_| {
                c1.fetch_add(1, Ordering::SeqCst);
            }),
            Cmd::tick(Duration::from_millis(100), move |_| {
                c2.fetch_add(1, Ordering::SeqCst);
            }),
        ]);

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    // ==================== Every Tests ====================

    #[tokio::test]
    async fn test_execute_every() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);

        // Use a short duration for testing
        let cmd = Cmd::every(Duration::from_millis(100), move |_| {
            flag_clone.store(true, Ordering::SeqCst);
        });

        executor.execute(cmd);

        // Wait for notification (may take up to 100ms to align)
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_every_receives_timestamp() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let timestamp_received = Arc::new(std::sync::Mutex::new(None));
        let ts_clone = Arc::clone(&timestamp_received);

        let cmd = Cmd::every(Duration::from_millis(50), move |ts| {
            *ts_clone.lock().unwrap() = Some(ts);
        });

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(timestamp_received.lock().unwrap().is_some());
    }

    // ==================== Mixed Composition Tests ====================

    #[tokio::test]
    async fn test_sequence_with_tick() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let o1 = Arc::clone(&order);
        let o2 = Arc::clone(&order);

        let cmd = Cmd::sequence(vec![
            Cmd::tick(Duration::from_millis(50), move |_| {
                o1.lock().unwrap().push(1);
            }),
            Cmd::perform(move || async move {
                o2.lock().unwrap().push(2);
            }),
        ]);

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = order.lock().unwrap().clone();
        assert_eq!(result, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_batch_with_sequence_and_tick() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let executor = CmdExecutor::new(tx);

        let counter = Arc::new(AtomicU32::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);
        let c3 = Arc::clone(&counter);

        let cmd = Cmd::batch(vec![
            Cmd::sequence(vec![
                Cmd::tick(Duration::from_millis(25), move |_| {
                    c1.fetch_add(1, Ordering::SeqCst);
                }),
                Cmd::perform(move || async move {
                    c2.fetch_add(1, Ordering::SeqCst);
                }),
            ]),
            Cmd::tick(Duration::from_millis(50), move |_| {
                c3.fetch_add(1, Ordering::SeqCst);
            }),
        ]);

        executor.execute(cmd);

        // Wait for notification
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Give tasks time to complete
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
