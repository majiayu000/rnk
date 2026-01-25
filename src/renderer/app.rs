//! Application runner
//!
//! This module provides the main application runner.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::Element;
use crate::hooks::context::{HookContext, with_hooks};
use crate::hooks::use_app::{AppContext, set_app_context};
use crate::hooks::use_input::clear_input_handlers;
use crate::hooks::use_mouse::{clear_mouse_handlers, is_mouse_enabled};
use crate::layout::LayoutEngine;
use crate::renderer::{Output, Terminal};

use super::builder::AppOptions;
use super::element_renderer::render_element;
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
    hook_context: Rc<RefCell<HookContext>>,
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
        let runtime = AppRuntime::new(options.alternate_screen);
        let render_handle = RenderHandle::new(runtime.clone());
        let hook_context = Rc::new(RefCell::new(HookContext::new()));

        // Set up render callback
        let runtime_clone = runtime.clone();
        hook_context
            .borrow_mut()
            .set_render_callback(Rc::new(move || {
                runtime_clone.request_render();
            }));

        // Get initial terminal size
        let (initial_width, initial_height) = Terminal::size().unwrap_or((80, 24));

        Self {
            component,
            terminal: Terminal::new(),
            layout_engine: LayoutEngine::new(),
            hook_context,
            options,
            should_exit: Arc::new(AtomicBool::new(false)),
            runtime,
            render_handle,
            static_renderer: StaticRenderer::new(),
            last_width: initial_width,
            last_height: initial_height,
        }
    }

    /// Run the application
    pub fn run(&mut self) -> std::io::Result<()> {
        let _app_guard = register_app(self.runtime.clone());

        // Enter terminal mode based on options
        if self.options.alternate_screen {
            self.terminal.enter()?;
            self.runtime.set_alt_screen_state(true);
        } else {
            self.terminal.enter_inline()?;
            self.runtime.set_alt_screen_state(false);
        }

        // Create event loop
        let mut event_loop = EventLoop::new(
            self.runtime.clone(),
            self.should_exit.clone(),
            self.options.fps,
            self.options.exit_on_ctrl_c,
        );

        // Run event loop with render callback
        event_loop.run(|| {
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

        // Always clear on resize to prevent artifacts
        if new_width != self.last_width || new_height != self.last_height {
            let _ = execute!(stdout(), MoveTo(0, 0), Clear(ClearType::All));
            self.terminal.repaint();
        }

        self.last_width = new_width;
        self.last_height = new_height;
    }

    fn render_frame(&mut self) -> std::io::Result<()> {
        // Clear input and mouse handlers before render (they'll be re-registered)
        clear_input_handlers();
        clear_mouse_handlers();

        // Get terminal size
        let (width, height) = Terminal::size()?;

        // Set up app context for use_app hook
        set_app_context(Some(AppContext::new(
            self.should_exit.clone(),
            self.render_handle.clone(),
        )));

        // Build element tree with hooks context
        let root = with_hooks(self.hook_context.clone(), || (self.component)());

        // Clear app context after render
        set_app_context(None);

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
