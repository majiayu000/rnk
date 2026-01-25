//! Integration tests for the Command system
//!
//! These tests verify the complete integration of:
//! - Cmd types (None, Batch, Perform, Sleep)
//! - CmdExecutor (async execution with Tokio)
//! - use_cmd Hook (dependency tracking)
//! - Predefined tasks (file I/O, process spawning)

use rnk::cmd::{Cmd, CmdExecutor, HttpRequest};
use rnk::hooks::{HookContext, use_cmd, with_hooks};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Test 1: Complete command execution lifecycle
///
/// Verifies: Cmd creation → Executor execution → Render notification
#[tokio::test]
async fn test_complete_command_lifecycle() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = Arc::clone(&executed);

    let cmd = Cmd::perform(move || async move {
        executed_clone.store(true, Ordering::SeqCst);
    });

    executor.execute(cmd);

    // Wait for render notification
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout waiting for render notification")
        .expect("channel closed");

    // Verify execution
    assert!(executed.load(Ordering::SeqCst));

    executor.shutdown();
}

/// Test 2: Batch command execution with multiple tasks
///
/// Verifies: Multiple commands execute in parallel, single notification
#[tokio::test]
async fn test_batch_command_execution() {
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
            tokio::time::sleep(Duration::from_millis(10)).await;
            c2.fetch_add(1, Ordering::SeqCst);
        }),
        Cmd::perform(move || async move {
            c3.fetch_add(1, Ordering::SeqCst);
        }),
    ]);

    executor.execute(cmd);

    // Wait for single notification
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    // Give tasks time to complete
    tokio::time::sleep(Duration::from_millis(50)).await;

    // All three tasks should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 3);

    // Should not receive additional notifications
    tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap_err(); // Should timeout

    executor.shutdown();
}

/// Test 3: Sleep and chained commands
///
/// Verifies: Sleep timing + command chaining
#[tokio::test]
async fn test_sleep_chain_execution() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let executed = Arc::new(AtomicBool::new(false));
    let e = Arc::clone(&executed);

    let start = std::time::Instant::now();

    let cmd = Cmd::sleep(Duration::from_millis(100))
        .and_then(Cmd::sleep(Duration::from_millis(100)))
        .and_then(Cmd::perform(move || async move {
            e.store(true, Ordering::SeqCst);
        }));

    executor.execute(cmd);

    // Wait for notification
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    let elapsed = start.elapsed();

    // Should take at least 200ms (two sleeps)
    assert!(elapsed >= Duration::from_millis(200));
    assert!(elapsed < Duration::from_millis(500)); // But not too long

    // Verify final command executed
    assert!(executed.load(Ordering::SeqCst));

    executor.shutdown();
}

/// Test 4: File I/O operations
///
/// Verifies: File write + read operations
#[tokio::test]
async fn test_file_io_integration() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let temp_file = std::env::temp_dir().join("rnk_integration_test.txt");
    let write_result = Arc::new(Mutex::new(None));
    let read_result = Arc::new(Mutex::new(None));

    let wr = Arc::clone(&write_result);
    let temp_file_clone = temp_file.clone();

    // Step 1: Write file
    let write_cmd = Cmd::write_file(temp_file.clone(), "integration test content", move |res| {
        *wr.lock().unwrap() = Some(res);
    });

    executor.execute(write_cmd);

    // Wait for write completion
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    // Verify write succeeded
    assert!(write_result.lock().unwrap().as_ref().unwrap().is_ok());

    // Step 2: Read file
    let rr = Arc::clone(&read_result);
    let read_cmd = Cmd::read_file(temp_file_clone, move |res| {
        *rr.lock().unwrap() = Some(res);
    });

    executor.execute(read_cmd);

    // Wait for read completion
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    // Verify read succeeded and content matches
    let read_res = read_result.lock().unwrap().take().unwrap();
    assert!(read_res.is_ok());
    assert_eq!(read_res.unwrap(), "integration test content");

    // Cleanup
    let _ = std::fs::remove_file(temp_file);

    executor.shutdown();
}

/// Test 5: Process spawning
///
/// Verifies: Process execution with output capture
#[tokio::test]
async fn test_process_spawn_integration() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let result = Arc::new(Mutex::new(None));
    let r = Arc::clone(&result);

    let cmd = Cmd::spawn(
        "echo",
        vec!["integration".to_string(), "test".to_string()],
        move |res| {
            *r.lock().unwrap() = Some(res);
        },
    );

    executor.execute(cmd);

    // Wait for completion
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    // Verify result
    let output = result.lock().unwrap().take().unwrap();
    assert!(output.is_ok());

    let output = output.unwrap();
    assert!(output.success);
    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("integration"));
    assert!(output.stdout.contains("test"));

    executor.shutdown();
}

/// Test 6: use_cmd Hook integration
///
/// Verifies: Hook dependency tracking + command queueing
#[test]
fn test_use_cmd_hook_integration() {
    let ctx = std::rc::Rc::new(std::cell::RefCell::new(HookContext::new()));
    let execution_count = Arc::new(AtomicU32::new(0));

    // First render - should execute
    {
        let count = Arc::clone(&execution_count);
        with_hooks(ctx.clone(), move || {
            use_cmd(42, move |val| {
                assert_eq!(val, 42);
                count.fetch_add(1, Ordering::SeqCst);
                Cmd::none()
            });
        });
    }

    assert_eq!(execution_count.load(Ordering::SeqCst), 1);
    let cmds = ctx.borrow_mut().take_cmds();
    assert_eq!(cmds.len(), 1);

    // Second render - same deps, should not execute
    {
        let count = Arc::clone(&execution_count);
        with_hooks(ctx.clone(), move || {
            use_cmd(42, move |_| {
                count.fetch_add(1, Ordering::SeqCst);
                Cmd::none()
            });
        });
    }

    assert_eq!(execution_count.load(Ordering::SeqCst), 1); // Still 1
    let cmds = ctx.borrow_mut().take_cmds();
    assert_eq!(cmds.len(), 0); // No new commands

    // Third render - different deps, should execute
    {
        let count = Arc::clone(&execution_count);
        with_hooks(ctx.clone(), move || {
            use_cmd(99, move |val| {
                assert_eq!(val, 99);
                count.fetch_add(1, Ordering::SeqCst);
                Cmd::none()
            });
        });
    }

    assert_eq!(execution_count.load(Ordering::SeqCst), 2);
    let cmds = ctx.borrow_mut().take_cmds();
    assert_eq!(cmds.len(), 1);
}

/// Test 7: Concurrent command execution
///
/// Verifies: Multiple concurrent commands execute correctly
#[tokio::test]
async fn test_concurrent_command_execution() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let counter = Arc::new(AtomicU32::new(0));

    // Execute 20 commands concurrently
    for _ in 0..20 {
        let c = Arc::clone(&counter);
        executor.execute(Cmd::perform(move || async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            c.fetch_add(1, Ordering::SeqCst);
        }));
    }

    // Wait for all 20 notifications
    for _ in 0..20 {
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
    }

    // Give tasks time to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // All 20 should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 20);

    executor.shutdown();
}

/// Test 8: Error handling in commands
///
/// Verifies: Errors are properly propagated to callbacks
#[tokio::test]
async fn test_error_handling_integration() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    // Test 1: Non-existent file read
    let result = Arc::new(Mutex::new(None));
    let r = Arc::clone(&result);

    let cmd = Cmd::read_file("/nonexistent/path/file.txt", move |res| {
        *r.lock().unwrap() = Some(res);
    });

    executor.execute(cmd);

    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    let res = result.lock().unwrap().take().unwrap();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Failed to read file"));

    // Test 2: Non-existent command
    let result2 = Arc::new(Mutex::new(None));
    let r2 = Arc::clone(&result2);

    let cmd2 = Cmd::spawn("nonexistent_command_xyz_123", vec![], move |res| {
        *r2.lock().unwrap() = Some(res);
    });

    executor.execute(cmd2);

    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    let res2 = result2.lock().unwrap().take().unwrap();
    assert!(res2.is_err());

    executor.shutdown();
}

/// Test 9: Complex scenario - Timer with file write
///
/// Verifies: Real-world scenario combining multiple features
#[tokio::test]
async fn test_complex_scenario_timer_file_write() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);

    let temp_file = std::env::temp_dir().join("rnk_timer_test.txt");
    let temp_file_for_verify = temp_file.clone();
    let result = Arc::new(Mutex::new(None));
    let r = Arc::clone(&result);

    let start = std::time::Instant::now();

    // Wait 100ms then write file
    let cmd = Cmd::delay(Duration::from_millis(100), move || {
        let temp = temp_file.clone();
        let res = r.clone();
        async move {
            match tokio::fs::write(&temp, "delayed write").await {
                Ok(_) => *res.lock().unwrap() = Some(Ok(())),
                Err(e) => *res.lock().unwrap() = Some(Err(format!("{}", e))),
            }
        }
    });

    executor.execute(cmd);

    // Wait for completion
    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    let elapsed = start.elapsed();

    // Should have waited at least 100ms
    assert!(elapsed >= Duration::from_millis(100));

    // Verify file was written
    assert!(result.lock().unwrap().is_some());
    assert!(result.lock().unwrap().as_ref().unwrap().is_ok());

    let contents = std::fs::read_to_string(&temp_file_for_verify).unwrap();
    assert_eq!(contents, "delayed write");

    // Cleanup
    let _ = std::fs::remove_file(temp_file_for_verify);

    executor.shutdown();
}

/// Test 10: HttpRequest builder integration
///
/// Verifies: HttpRequest builder creates correct structure
#[test]
fn test_http_request_builder_integration() {
    let request = HttpRequest::get("https://api.github.com/users/octocat")
        .header("Authorization", "Bearer token123")
        .header("Content-Type", "application/json");

    assert_eq!(request.url, "https://api.github.com/users/octocat");
    assert_eq!(request.method, "GET");
    assert_eq!(request.headers.len(), 2);
    assert_eq!(
        request.headers[0],
        ("Authorization".to_string(), "Bearer token123".to_string())
    );
    assert_eq!(
        request.headers[1],
        ("Content-Type".to_string(), "application/json".to_string())
    );
    assert!(request.body.is_none());

    let post_request = HttpRequest::post("https://api.example.com/data")
        .header("Content-Type", "application/json")
        .body(r#"{"key": "value"}"#);

    assert_eq!(post_request.method, "POST");
    assert!(post_request.body.is_some());
    assert_eq!(post_request.body.unwrap(), r#"{"key": "value"}"#);
}
