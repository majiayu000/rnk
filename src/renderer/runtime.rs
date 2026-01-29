//! Application event loop runtime
//!
//! This module handles the main event loop, event processing, and
//! integration with the Command system (CmdExecutor).

use crossterm::event::{Event, KeyCode, KeyModifiers};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::hooks::use_input::{clear_input_handlers, dispatch_key_event};
use crate::hooks::use_mouse::dispatch_mouse_event;
use crate::renderer::Terminal;

use super::filter::FilterChain;
use super::registry::{AppRuntime, AppSink};

/// Event loop state and execution
pub(crate) struct EventLoop {
    runtime: Arc<AppRuntime>,
    should_exit: Arc<AtomicBool>,
    fps: u32,
    exit_on_ctrl_c: bool,
    /// Flag indicating a suspend was requested
    suspend_requested: Arc<AtomicBool>,
    /// Event filter chain
    filter_chain: FilterChain,
    /// External cancel token flag
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl EventLoop {
    pub(crate) fn with_filters(
        runtime: Arc<AppRuntime>,
        should_exit: Arc<AtomicBool>,
        fps: u32,
        exit_on_ctrl_c: bool,
        filter_chain: FilterChain,
    ) -> Self {
        Self {
            runtime,
            should_exit,
            fps,
            exit_on_ctrl_c,
            suspend_requested: Arc::new(AtomicBool::new(false)),
            filter_chain,
            cancel_flag: None,
        }
    }

    pub(crate) fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
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
            if self.suspend_requested.swap(false, Ordering::SeqCst) {
                // Return a special marker - the App will handle the actual suspend
                return Ok(());
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

                // Handle Ctrl+Z (suspend) on Unix
                #[cfg(unix)]
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('z')
                {
                    self.suspend_requested.store(true, Ordering::SeqCst);
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

    fn create_event_loop(runtime: Arc<AppRuntime>, should_exit: Arc<AtomicBool>) -> EventLoop {
        EventLoop::with_filters(runtime, should_exit, 60, true, FilterChain::new())
    }

    #[test]
    fn test_event_loop_creation() {
        let runtime = AppRuntime::new(false);
        let should_exit = Arc::new(AtomicBool::new(false));
        let event_loop = create_event_loop(runtime, should_exit);

        assert_eq!(event_loop.fps, 60);
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
}
