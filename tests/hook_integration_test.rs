//! Integration tests for hooks that depend on runtime wiring.

use rnk::cmd::{Cmd, CmdExecutor};
use rnk::hooks::{HookContext, use_cmd_once, use_frame_rate, with_hooks};
use rnk::renderer::{FrameRateConfig, FrameRateController};
use rnk::runtime::{RuntimeContext, set_current_runtime};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_use_cmd_queues_and_executes() {
    let ctx = Arc::new(std::sync::RwLock::new(HookContext::new()));
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let _element = with_hooks(ctx.clone(), || {
        use_cmd_once(move |_| {
            let executed_inner = executed_clone.clone();
            Cmd::perform(move || async move {
                executed_inner.store(true, Ordering::SeqCst);
            })
        });
        // Return a minimal element to satisfy the render
        rnk::components::Text::new("ok").into_element()
    });

    let cmds = ctx.write().unwrap().take_cmds();
    assert!(!cmds.is_empty());

    let (tx, mut rx) = mpsc::unbounded_channel();
    let executor = CmdExecutor::new(tx);
    executor.execute(Cmd::batch(cmds));

    tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout waiting for render notification")
        .expect("channel closed");

    assert!(executed.load(Ordering::SeqCst));

    executor.shutdown();
}

#[test]
fn test_use_frame_rate_with_controller_stats() {
    let mut controller = FrameRateController::new(FrameRateConfig::new(60).with_stats());
    std::thread::sleep(Duration::from_millis(10));
    controller.record_frame(Duration::from_millis(1));

    let shared = controller.shared_stats().expect("stats should be enabled");
    let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
    ctx.borrow_mut().set_frame_rate_stats(Some(shared));

    set_current_runtime(Some(ctx));
    let stats = use_frame_rate().expect("expected stats");
    set_current_runtime(None);

    assert!(stats.total_frames >= 1);
    assert!(stats.current_fps > 0.0);
}
