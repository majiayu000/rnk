//! Application runner
//!
//! This module provides the main application runner.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cmd::{Cmd, CmdExecutor, ExecRequest, run_exec_process};
use crate::core::Element;
use crate::hooks::context::with_hooks;
use crate::hooks::use_app::{AppContext, set_app_context};
use crate::hooks::use_input::clear_input_handlers;
use crate::hooks::use_mouse::{clear_mouse_handlers, is_mouse_enabled};
use crate::hooks::{MeasureContext, clear_paste_handlers, set_measure_context};
use crate::layout::LayoutEngine;
use crate::renderer::{Output, Terminal};
use crate::runtime::{RuntimeContext, set_current_runtime};
use tokio::sync::mpsc;

use super::builder::{AppOptions, CancelToken};
use super::element_renderer::render_element;
use super::filter::FilterChain;
use super::registry::{AppRuntime, AppSink, ModeSwitch, Printable, RenderHandle, register_app};
use super::render_to_string::render_to_string;
use super::runtime::EventLoop;
use super::static_content::StaticRenderer;

/// Application state
pub struct App<F>
where
    F: Fn() -> Element,
{
    component: F,
    terminal: Terminal,
    layout_engine: LayoutEngine,
    options: AppOptions,
    should_exit: Arc<AtomicBool>,
    runtime: Arc<AppRuntime>,
    render_handle: RenderHandle,
    /// Static content renderer for inline mode
    static_renderer: StaticRenderer,
    /// Last known terminal width (for detecting width decreases)
    last_width: u16,
    /// Last known terminal height
    last_height: u16,
    /// Event filter chain
    filter_chain: FilterChain,
    /// External cancel token
    cancel_token: Option<CancelToken>,
    /// Command executor for use_cmd and other Cmds
    cmd_executor: CmdExecutor,
    /// Render notifications from CmdExecutor
    cmd_render_rx: Option<mpsc::UnboundedReceiver<()>>,
    /// Unified runtime context for input/mouse/focus/stats
    runtime_context: Rc<RefCell<RuntimeContext>>,
}

impl<F> App<F>
where
    F: Fn() -> Element,
{
    /// Create a new app with default options (inline mode)
    pub fn new(component: F) -> Self {
        Self::with_options(component, AppOptions::default())
    }

    /// Create a new app with custom options
    pub fn with_options(component: F, options: AppOptions) -> Self {
        Self::with_options_and_filters(component, options, FilterChain::new())
    }

    /// Create a new app with custom options and event filters
    pub fn with_options_and_filters(
        component: F,
        options: AppOptions,
        filter_chain: FilterChain,
    ) -> Self {
        Self::with_full_config(component, options, filter_chain, None)
    }

    /// Create a new app with full configuration
    pub fn with_full_config(
        component: F,
        options: AppOptions,
        filter_chain: FilterChain,
        cancel_token: Option<CancelToken>,
    ) -> Self {
        let runtime = AppRuntime::new(options.alternate_screen);
        let render_handle = RenderHandle::new(runtime.clone());
        let should_exit = Arc::new(AtomicBool::new(false));
        let runtime_context = Rc::new(RefCell::new(RuntimeContext::with_app_control(
            should_exit.clone(),
            render_handle.clone(),
        )));

        let (cmd_render_tx, cmd_render_rx) = mpsc::unbounded_channel();
        let cmd_executor = CmdExecutor::new(cmd_render_tx);

        // Set up render callback
        let runtime_clone = runtime.clone();
        runtime_context
            .borrow_mut()
            .set_render_callback(Arc::new(move || {
                runtime_clone.request_render();
            }));

        // Get initial terminal size
        let (initial_width, initial_height) = Terminal::size().unwrap_or((80, 24));

        Self {
            component,
            terminal: Terminal::new(),
            layout_engine: LayoutEngine::new(),
            options,
            should_exit,
            runtime,
            render_handle,
            static_renderer: StaticRenderer::new(),
            last_width: initial_width,
            last_height: initial_height,
            filter_chain,
            cancel_token,
            cmd_executor,
            cmd_render_rx: Some(cmd_render_rx),
            runtime_context,
        }
    }

    /// Run the application
    pub fn run(mut self) -> std::io::Result<()> {
        let _app_guard = register_app(self.runtime.clone());
        set_current_runtime(Some(self.runtime_context.clone()));
        struct CurrentRuntimeGuard;
        impl Drop for CurrentRuntimeGuard {
            fn drop(&mut self) {
                set_current_runtime(None);
            }
        }
        let _runtime_guard = CurrentRuntimeGuard;

        // Enter terminal mode based on options
        if self.options.alternate_screen {
            self.terminal.enter()?;
            self.runtime.set_alt_screen_state(true);
        } else {
            self.terminal.enter_inline()?;
            self.runtime.set_alt_screen_state(false);
        }

        // Take ownership of filter chain for the event loop
        let filter_chain = std::mem::take(&mut self.filter_chain);

        // Create frame rate controller and share stats with runtime context
        let frame_rate =
            super::frame_rate::FrameRateController::new(self.options.to_frame_rate_config());
        let shared_stats = frame_rate.shared_stats();
        self.runtime_context
            .borrow_mut()
            .set_frame_rate_stats(shared_stats);

        // Create event loop with filters
        let mut event_loop = EventLoop::with_filters(
            self.runtime.clone(),
            self.should_exit.clone(),
            frame_rate,
            self.options.exit_on_ctrl_c,
            filter_chain,
        );

        // Add cancel token if present
        if let Some(ref token) = self.cancel_token {
            event_loop = event_loop.with_cancel_flag(token.flag());
        }

        // Add command render notifications if present
        if let Some(rx) = self.cmd_render_rx.take() {
            event_loop = event_loop.with_render_rx(rx);
        }

        // Run event loop with render callback (handle suspend/resume)
        loop {
            event_loop.run(|| {
                // Handle exec requests first (they suspend the terminal)
                let exec_requests = self.runtime.take_exec_requests();
                for request in exec_requests {
                    self.handle_exec_request(request)?;
                }

                // Handle terminal control commands
                let terminal_cmds = crate::renderer::registry::take_terminal_cmds();
                for cmd in terminal_cmds {
                    self.handle_terminal_cmd(cmd)?;
                }

                // Handle mode switch requests (access runtime directly)
                if let Some(mode_switch) = self.runtime.take_mode_switch_request() {
                    self.handle_mode_switch(mode_switch)?;
                }

                // Handle println messages (access runtime directly)
                let messages = self.runtime.take_println_messages();
                if !messages.is_empty() {
                    self.handle_println_messages(&messages)?;
                }

                // Handle resize
                let (width, height) = Terminal::size()?;
                if width != self.last_width || height != self.last_height {
                    self.handle_resize(width, height);
                }

                // Render frame
                self.render_frame()
            })?;

            if self.runtime.take_suspend_request() {
                self.handle_suspend()?;
                continue;
            }

            break;
        }

        // Exit terminal mode
        if self.terminal.is_alt_screen() {
            self.terminal.exit()?;
        } else {
            self.terminal.exit_inline()?;
        }

        Ok(())
    }

    /// Handle mode switch request
    fn handle_mode_switch(&mut self, mode_switch: ModeSwitch) -> std::io::Result<()> {
        match mode_switch {
            ModeSwitch::EnterAltScreen => {
                if !self.terminal.is_alt_screen() {
                    self.terminal.switch_to_alt_screen()?;
                    self.runtime.set_alt_screen_state(true);
                    self.terminal.repaint();
                }
            }
            ModeSwitch::ExitAltScreen => {
                if self.terminal.is_alt_screen() {
                    self.terminal.switch_to_inline()?;
                    self.runtime.set_alt_screen_state(false);
                    self.terminal.repaint();
                }
            }
        }
        Ok(())
    }

    /// Handle exec request - suspend TUI, run external process, resume TUI
    fn handle_exec_request(&mut self, request: ExecRequest) -> std::io::Result<()> {
        // Suspend the terminal
        self.terminal.suspend()?;

        // Execute the external process
        let result = run_exec_process(&request.config);

        // Resume the terminal
        self.terminal.resume()?;

        // Call the callback with the result
        (request.callback)(result);

        // Request re-render
        self.runtime.request_render();

        Ok(())
    }

    /// Handle app suspend request (Ctrl+Z or use_app::suspend)
    #[cfg(unix)]
    fn handle_suspend(&mut self) -> std::io::Result<()> {
        self.terminal.suspend()?;
        crate::runtime::suspend_self()?;
        self.terminal.resume()?;
        self.runtime.request_render();
        Ok(())
    }

    /// Handle suspend on non-Unix (no-op)
    #[cfg(not(unix))]
    fn handle_suspend(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// Handle terminal control commands
    fn handle_terminal_cmd(&mut self, cmd: crate::cmd::TerminalCmd) -> std::io::Result<()> {
        use crate::cmd::TerminalCmd;
        use crossterm::{cursor, execute, terminal as ct};
        use std::io::stdout;

        match cmd {
            TerminalCmd::ClearScreen => {
                if self.terminal.is_alt_screen() {
                    execute!(
                        stdout(),
                        ct::Clear(ct::ClearType::All),
                        cursor::MoveTo(0, 0)
                    )?;
                } else {
                    self.terminal.clear()?;
                }
            }
            TerminalCmd::HideCursor => {
                execute!(stdout(), cursor::Hide)?;
            }
            TerminalCmd::ShowCursor => {
                execute!(stdout(), cursor::Show)?;
            }
            TerminalCmd::SetWindowTitle(title) => {
                execute!(stdout(), ct::SetTitle(&title))?;
            }
            TerminalCmd::WindowSize => {
                // This triggers a resize check on next render
                self.last_width = 0;
                self.last_height = 0;
            }
            TerminalCmd::EnterAltScreen => {
                if !self.terminal.is_alt_screen() {
                    self.terminal.switch_to_alt_screen()?;
                    self.runtime.set_alt_screen_state(true);
                    self.terminal.repaint();
                }
            }
            TerminalCmd::ExitAltScreen => {
                if self.terminal.is_alt_screen() {
                    self.terminal.switch_to_inline()?;
                    self.runtime.set_alt_screen_state(false);
                    self.terminal.repaint();
                }
            }
            TerminalCmd::EnableMouse => {
                crate::hooks::use_mouse::set_mouse_enabled(true);
                execute!(stdout(), crossterm::event::EnableMouseCapture)?;
            }
            TerminalCmd::DisableMouse => {
                crate::hooks::use_mouse::set_mouse_enabled(false);
                execute!(stdout(), crossterm::event::DisableMouseCapture)?;
            }
            TerminalCmd::EnableBracketedPaste => {
                execute!(stdout(), crossterm::event::EnableBracketedPaste)?;
            }
            TerminalCmd::DisableBracketedPaste => {
                execute!(stdout(), crossterm::event::DisableBracketedPaste)?;
            }
        }
        Ok(())
    }

    /// Handle println messages (like Bubbletea's Println)
    fn handle_println_messages(&mut self, messages: &[Printable]) -> std::io::Result<()> {
        // Println only works in inline mode
        if self.terminal.is_alt_screen() {
            return Ok(());
        }

        // Get terminal width for rendering elements
        let (width, _) = Terminal::size().unwrap_or((80, 24));

        for message in messages {
            match message {
                Printable::Text(text) => {
                    // Simple text - print directly
                    self.terminal.println(text)?;
                }
                Printable::Element(element) => {
                    // Render element to string first
                    let rendered = render_to_string(element, width);
                    self.terminal.println(&rendered)?;
                }
            }
        }

        // Force repaint after println
        self.terminal.repaint();

        Ok(())
    }

    /// Handle terminal resize events
    fn handle_resize(&mut self, new_width: u16, new_height: u16) {
        use crossterm::cursor::MoveTo;
        use crossterm::execute;
        use crossterm::terminal::{Clear, ClearType};
        use std::io::stdout;

        if new_width != self.last_width || new_height != self.last_height {
            if self.terminal.is_alt_screen() {
                // Fullscreen mode: clear entire screen
                let _ = execute!(stdout(), MoveTo(0, 0), Clear(ClearType::All));
            }
            // Inline mode: just repaint (clear_inline_content will handle cleanup)
            // Don't use Clear(ClearType::All) as it destroys scrollback content
            self.terminal.repaint();
        }

        self.last_width = new_width;
        self.last_height = new_height;
    }

    fn render_frame(&mut self) -> std::io::Result<()> {
        // Clear input and mouse handlers before render (they'll be re-registered)
        clear_input_handlers();
        clear_mouse_handlers();
        clear_paste_handlers();
        set_measure_context(None);

        // Clear runtime per-frame handler registrations.
        self.runtime_context.borrow_mut().prepare_render();

        // Get terminal size
        let (width, height) = Terminal::size()?;

        // Set up app context for use_app hook
        set_app_context(Some(AppContext::new(
            self.should_exit.clone(),
            self.render_handle.clone(),
        )));

        // Build element tree with hooks context
        let hook_context = self.runtime_context.borrow().hook_context();
        let root = with_hooks(hook_context, || (self.component)());

        // Clear app context after render
        set_app_context(None);

        // Execute any queued commands from hooks
        let cmds = self.runtime_context.borrow_mut().take_cmds();
        if !cmds.is_empty() {
            self.cmd_executor.execute(Cmd::batch(cmds));
        }

        // Enable/disable mouse mode based on whether any component uses it
        if is_mouse_enabled() {
            self.terminal.enable_mouse()?;
        } else {
            self.terminal.disable_mouse()?;
        }

        // Extract and commit static content
        let new_static_lines = self.static_renderer.extract_static_content(&root, width);
        if !new_static_lines.is_empty() {
            self.static_renderer
                .commit_static_content(&new_static_lines, &mut self.terminal)?;
        }

        // Filter out static elements from the tree for dynamic rendering
        let dynamic_root = self.static_renderer.filter_static_elements(&root);

        // Compute layout for dynamic content
        self.layout_engine.compute(&dynamic_root, width, height);

        // Update measure context with latest layouts
        let mut measure_ctx = MeasureContext::new();
        measure_ctx.set_layouts(self.layout_engine.get_all_layouts());
        set_measure_context(Some(measure_ctx));

        // Get the actual content size from layout
        let root_layout = self
            .layout_engine
            .get_layout(dynamic_root.id)
            .unwrap_or_default();
        let content_width = (root_layout.width as u16).max(1).min(width);
        let render_height = (root_layout.height as u16).max(1).min(height);

        // Render to output buffer
        let mut output = Output::new(content_width, render_height);
        render_element(&dynamic_root, &self.layout_engine, &mut output, 0.0, 0.0);

        // Write to terminal
        let rendered = output.render();
        self.terminal.render(&rendered)
    }

    /// Request exit
    pub fn exit(&self) {
        self.should_exit.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::registry::{is_alt_screen, render_handle};

    #[test]
    fn test_registry_cleanup_on_drop() {
        let runtime = AppRuntime::new(false);

        {
            let _guard = register_app(runtime);
            assert!(render_handle().is_some());
            assert_eq!(is_alt_screen(), Some(false));
        }

        assert!(render_handle().is_none());
        assert_eq!(is_alt_screen(), None);
    }
}
