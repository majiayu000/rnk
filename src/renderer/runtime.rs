//! Application event loop runtime
//!
//! This module handles the main event loop, event processing, and
//! integration with the Command system (CmdExecutor).

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::hooks::use_input::dispatch_key_event;
use crate::hooks::use_mouse::dispatch_mouse_event;
use crate::renderer::Terminal;

use super::filter::FilterChain;
use super::frame_rate::FrameRateController;
use super::registry::{AppRuntime, AppSink};

/// Event loop state and execution
pub(crate) struct EventLoop {
    runtime: Arc<AppRuntime>,
    should_exit: Arc<AtomicBool>,
    frame_rate: FrameRateController,
    exit_on_ctrl_c: bool,
    /// Event filter chain
    filter_chain: FilterChain,
    /// External cancel token flag
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Render notifications from background tasks
    render_rx: Option<mpsc::UnboundedReceiver<()>>,
}

impl EventLoop {
    pub(crate) fn with_filters(
        runtime: Arc<AppRuntime>,
        should_exit: Arc<AtomicBool>,
        frame_rate: FrameRateController,
        exit_on_ctrl_c: bool,
        filter_chain: FilterChain,
    ) -> Self {
        Self {
            runtime,
            should_exit,
            frame_rate,
            exit_on_ctrl_c,
            filter_chain,
            cancel_flag: None,
            render_rx: None,
        }
    }

    pub(crate) fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
    }

    pub(crate) fn with_render_rx(mut self, rx: mpsc::UnboundedReceiver<()>) -> Self {
        self.render_rx = Some(rx);
        self
    }

    /// Run the event loop
    ///
    /// Returns when should_exit is set or an error occurs
    pub(crate) fn run<F>(&mut self, mut on_render: F) -> std::io::Result<()>
    where
        F: FnMut() -> std::io::Result<()>,
    {
        // Initial render
        let start = Instant::now();
        on_render()?;
        self.frame_rate.record_frame(start.elapsed());

        loop {
            // Handle input events
            if let Some(event) = Terminal::poll_event(Duration::from_millis(10))? {
                // Apply event filters
                if let Some(filtered_event) = self.filter_chain.apply(event) {
                    self.handle_event(filtered_event);
                }
                // If filter returned None, the event was blocked
            }

            // Check exit condition
            if self.should_exit.load(Ordering::SeqCst) {
                break;
            }

            // Check external cancel token
            if let Some(ref cancel_flag) = self.cancel_flag {
                if cancel_flag.load(Ordering::SeqCst) {
                    self.should_exit.store(true, Ordering::SeqCst);
                    break;
                }
            }

            // Check suspend condition
            if self.runtime.suspend_requested() {
                // Return a special marker - the App will handle the actual suspend
                return Ok(());
            }

            // Drain render notifications from background tasks
            self.drain_render_notifications();

            // Check if render is needed
            let render_requested = self.runtime.render_requested();
            let time_elapsed = self.frame_rate.should_render();

            if render_requested && time_elapsed {
                self.runtime.clear_render_request();
                let start = Instant::now();
                on_render()?;
                self.frame_rate.record_frame(start.elapsed());
            }
        }

        Ok(())
    }

    /// Handle terminal event
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) => {
                // Ignore key release/repeat events to avoid duplicate actions.
                if key_event.kind != KeyEventKind::Press {
                    return;
                }

                // Handle Ctrl+C
                if self.exit_on_ctrl_c && Terminal::is_ctrl_c(&Event::Key(key_event)) {
                    self.should_exit.store(true, Ordering::SeqCst);
                    return;
                }

                // Handle Ctrl+Z (suspend) on Unix
                #[cfg(unix)]
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('z')
                {
                    self.runtime.request_suspend();
                    return;
                }

                // Dispatch to input handlers
                dispatch_key_event(&key_event);

                // Record user activity for idle detection
                crate::hooks::record_activity();

                // Request re-render after input
                self.runtime.request_render();
            }
            Event::Mouse(mouse_event) => {
                // Dispatch to mouse handlers
                dispatch_mouse_event(&mouse_event);

                // Record user activity for idle detection
                crate::hooks::record_activity();

                // Request re-render after mouse event
                self.runtime.request_render();
            }
            Event::Resize(_new_width, _new_height) => {
                // Resize is handled by the App itself
                // Just request re-render
                self.runtime.request_render();
            }
            _ => {}
        }
    }

    fn drain_render_notifications(&mut self) {
        if let Some(rx) = self.render_rx.as_mut() {
            let mut requested = false;
            while rx.try_recv().is_ok() {
                requested = true;
            }
            if requested {
                self.runtime.request_render();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::use_input::register_input_handler;
    use crate::hooks::use_mouse::register_mouse_handler;
    use crate::renderer::frame_rate::FrameRateConfig;
    use crate::renderer::registry::{AppRuntime, AppSink, ModeSwitch, Printable};
    use crossterm::event::{KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
    use std::sync::atomic::AtomicBool;

    fn create_event_loop(runtime: Arc<AppRuntime>, should_exit: Arc<AtomicBool>) -> EventLoop {
        let frame_rate = FrameRateController::new(FrameRateConfig::new(60));
        EventLoop::with_filters(runtime, should_exit, frame_rate, true, FilterChain::new())
    }

    #[test]
    fn test_event_loop_creation() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let event_loop = create_event_loop(runtime, should_exit);

        assert_eq!(event_loop.frame_rate.current_fps(), 60);
        assert!(event_loop.exit_on_ctrl_c);
    }

    #[test]
    fn test_event_loop_mode_switch() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let _event_loop = create_event_loop(runtime.clone(), should_exit);

        runtime.enter_alt_screen();
        let switch = runtime.take_mode_switch_request();
        assert_eq!(switch, Some(ModeSwitch::EnterAltScreen));
    }

    #[test]
    fn test_event_loop_println_messages() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let _event_loop = create_event_loop(runtime.clone(), should_exit);

        runtime.println(Printable::Text("test".to_string()));
        let messages = runtime.take_println_messages();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_event_loop_exit_flag() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let _event_loop = create_event_loop(runtime, should_exit.clone());

        assert!(!should_exit.load(Ordering::SeqCst));
        should_exit.store(true, Ordering::SeqCst);
        assert!(should_exit.load(Ordering::SeqCst));
    }

    #[test]
    fn test_event_loop_key_dispatch_requests_render() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let runtime = AppRuntime::new(false);
        runtime.clear_render_request();
        let should_exit = Arc::new(AtomicBool::new(false));
        let mut event_loop = create_event_loop(runtime.clone(), should_exit);

        let rt_ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(rt_ctx.clone()));

        let hit = Arc::new(AtomicBool::new(false));
        let hit_clone = hit.clone();
        register_input_handler(move |input, _| {
            if input == "x" {
                hit_clone.store(true, Ordering::SeqCst);
            }
        });

        let event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        event_loop.handle_event(event);

        assert!(hit.load(Ordering::SeqCst));
        assert!(runtime.render_requested());

        set_current_runtime(None);
    }

    #[test]
    fn test_event_loop_ignores_key_release_events() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let runtime = AppRuntime::new(false);
        runtime.clear_render_request();
        let should_exit = Arc::new(AtomicBool::new(false));
        let mut event_loop = create_event_loop(runtime.clone(), should_exit);

        let rt_ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(rt_ctx.clone()));

        let hit = Arc::new(AtomicBool::new(false));
        let hit_clone = hit.clone();
        register_input_handler(move |_input, _| {
            hit_clone.store(true, Ordering::SeqCst);
        });

        let mut key_event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        key_event.kind = KeyEventKind::Release;
        event_loop.handle_event(Event::Key(key_event));

        assert!(!hit.load(Ordering::SeqCst));
        assert!(!runtime.render_requested());

        set_current_runtime(None);
    }

    #[test]
    fn test_event_loop_mouse_dispatch_requests_render() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let runtime = AppRuntime::new(false);
        runtime.clear_render_request();
        let should_exit = Arc::new(AtomicBool::new(false));
        let mut event_loop = create_event_loop(runtime.clone(), should_exit);

        let rt_ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(rt_ctx.clone()));

        let hit = Arc::new(AtomicBool::new(false));
        let hit_clone = hit.clone();
        register_mouse_handler(move |_mouse| {
            hit_clone.store(true, Ordering::SeqCst);
        });

        let mouse_event = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        event_loop.handle_event(Event::Mouse(mouse_event));

        assert!(hit.load(Ordering::SeqCst));
        assert!(runtime.render_requested());

        set_current_runtime(None);
    }

    #[test]
    fn test_event_loop_render_rx_requests_render() {
        let runtime = AppRuntime::new(false);
        runtime.clear_render_request();
        let should_exit = Arc::new(AtomicBool::new(false));
        let frame_rate = FrameRateController::new(FrameRateConfig::new(60));
        let (tx, rx) = mpsc::unbounded_channel();
        let mut event_loop = EventLoop::with_filters(
            runtime.clone(),
            should_exit,
            frame_rate,
            true,
            FilterChain::new(),
        )
        .with_render_rx(rx);

        tx.send(()).unwrap();
        event_loop.drain_render_notifications();

        assert!(runtime.render_requested());
    }
}
