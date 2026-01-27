//! Application event loop runtime
//!
//! This module handles the main event loop, event processing, and
//! integration with the Command system (CmdExecutor).

use crossterm::event::Event;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::hooks::use_input::{clear_input_handlers, dispatch_key_event};
use crate::hooks::use_mouse::dispatch_mouse_event;
use crate::renderer::Terminal;

use super::registry::{AppRuntime, AppSink};

/// Event loop state and execution
pub(crate) struct EventLoop {
    runtime: Arc<AppRuntime>,
    should_exit: Arc<AtomicBool>,
    fps: u32,
    exit_on_ctrl_c: bool,
}

impl EventLoop {
    pub(crate) fn new(
        runtime: Arc<AppRuntime>,
        should_exit: Arc<AtomicBool>,
        fps: u32,
        exit_on_ctrl_c: bool,
    ) -> Self {
        Self {
            runtime,
            should_exit,
            fps,
            exit_on_ctrl_c,
        }
    }

    /// Run the event loop
    ///
    /// Returns when should_exit is set or an error occurs
    pub(crate) fn run<F>(&mut self, mut on_render: F) -> std::io::Result<()>
    where
        F: FnMut() -> std::io::Result<()>,
    {
        let frame_duration = Duration::from_millis(1000 / self.fps as u64);
        let mut last_render = Instant::now();

        // Initial render
        on_render()?;

        loop {
            // Handle input events
            if let Some(event) = Terminal::poll_event(Duration::from_millis(10))? {
                self.handle_event(event);
            }

            // Check exit condition
            if self.should_exit.load(Ordering::SeqCst) {
                break;
            }

            // Check if render is needed
            let now = Instant::now();
            let time_elapsed = now.duration_since(last_render) >= frame_duration;
            let render_requested = self.runtime.render_requested();

            if render_requested && time_elapsed {
                self.runtime.clear_render_request();
                on_render()?;
                last_render = now;
            }
        }

        // Clean up input handlers
        clear_input_handlers();

        Ok(())
    }

    /// Handle terminal event
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) => {
                // Handle Ctrl+C
                if self.exit_on_ctrl_c && Terminal::is_ctrl_c(&Event::Key(key_event)) {
                    self.should_exit.store(true, Ordering::SeqCst);
                    return;
                }

                // Dispatch to input handlers
                dispatch_key_event(&key_event);

                // Request re-render after input
                self.runtime.request_render();
            }
            Event::Mouse(mouse_event) => {
                // Dispatch to mouse handlers
                dispatch_mouse_event(&mouse_event);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::registry::{AppRuntime, AppSink, ModeSwitch, Printable};

    #[test]
    fn test_event_loop_creation() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let event_loop = EventLoop::new(runtime, should_exit, 60, true);

        assert_eq!(event_loop.fps, 60);
        assert!(event_loop.exit_on_ctrl_c);
    }

    #[test]
    fn test_event_loop_mode_switch() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let _event_loop = EventLoop::new(runtime.clone(), should_exit, 60, true);

        runtime.enter_alt_screen();
        let switch = runtime.take_mode_switch_request();
        assert_eq!(switch, Some(ModeSwitch::EnterAltScreen));
    }

    #[test]
    fn test_event_loop_println_messages() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let _event_loop = EventLoop::new(runtime.clone(), should_exit, 60, true);

        runtime.println(Printable::Text("test".to_string()));
        let messages = runtime.take_println_messages();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_event_loop_exit_flag() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let _event_loop = EventLoop::new(runtime, should_exit.clone(), 60, true);

        assert!(!should_exit.load(Ordering::SeqCst));
        should_exit.store(true, Ordering::SeqCst);
        assert!(should_exit.load(Ordering::SeqCst));
    }
}
