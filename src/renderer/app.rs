//! Application runner
//!
//! This module provides the main application runner.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cmd::{Cmd, CmdExecutor};
use crate::core::{Element, VNode};
use crate::hooks::use_mouse::is_mouse_enabled;
use crate::layout::LayoutEngine;
use crate::renderer::Terminal;
use crate::runtime::{RuntimeContext, set_current_runtime, with_runtime};
use tokio::sync::mpsc;

use super::builder::{AppOptions, CancelToken};
use super::filter::FilterChain;
use super::pipeline::RenderPipeline;
use super::registry::{AppRuntime, AppSink, RenderHandle, register_app};
use super::runtime::EventLoop;
use super::runtime_bridge::RuntimeBridge;
use super::static_content::StaticRenderer;
use super::terminal_controller::TerminalController;

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
    /// Previous VNode snapshot for incremental reconciliation.
    previous_vnode: Option<VNode>,
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
            static_renderer: StaticRenderer::new(),
            last_width: initial_width,
            last_height: initial_height,
            filter_chain,
            cancel_token,
            cmd_executor,
            cmd_render_rx: Some(cmd_render_rx),
            runtime_context,
            previous_vnode: None,
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
                RuntimeBridge::handle_exec_requests(&mut self.terminal, &self.runtime)?;
                RuntimeBridge::handle_terminal_commands(
                    &mut self.terminal,
                    &self.runtime,
                    &mut self.last_width,
                    &mut self.last_height,
                )?;
                RuntimeBridge::handle_mode_switch_request(&mut self.terminal, &self.runtime)?;
                RuntimeBridge::handle_println_messages(&mut self.terminal, &self.runtime)?;

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

    /// Handle terminal resize events
    fn handle_resize(&mut self, new_width: u16, new_height: u16) {
        TerminalController::handle_resize(
            &mut self.terminal,
            &mut self.last_width,
            &mut self.last_height,
            new_width,
            new_height,
        );
    }

    fn render_frame(&mut self) -> std::io::Result<()> {
        // Get terminal size
        let (width, height) = Terminal::size()?;

        // Build element tree under a unified runtime+hook lifecycle.
        let root = with_runtime(self.runtime_context.clone(), || (self.component)());

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
        let (new_static_lines, rendered) = self
            .try_prepare_frame(&root, width, height)
            .map_err(crate::renderer::DynamicFrameError::into_io)?;

        if !new_static_lines.is_empty() {
            self.static_renderer
                .commit_static_content(&new_static_lines, &mut self.terminal)?;
        }

        self.terminal.render(&rendered)
    }

    fn try_prepare_frame(
        &mut self,
        root: &Element,
        width: u16,
        height: u16,
    ) -> Result<(Vec<String>, String), crate::renderer::DynamicFrameError> {
        let new_static_lines = self
            .static_renderer
            .try_extract_static_content(root, width)?;
        let dynamic_root = self.static_renderer.filter_static_elements(root);
        let rendered = RenderPipeline::try_render_dynamic_frame_checked(
            &dynamic_root,
            width,
            height,
            &mut self.layout_engine,
            &self.runtime_context,
            &mut self.previous_vnode,
        )?;
        Ok((new_static_lines, rendered))
    }

    /// Request exit
    pub fn exit(&self) {
        self.should_exit.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::core::Element;
    use crate::layout::TextFlowError;
    use crate::renderer::registry::{is_alt_screen, lock_test_registry, render_handle};

    #[test]
    fn test_registry_cleanup_on_drop() {
        let _registry_guard = lock_test_registry();
        let runtime = AppRuntime::new(false);

        {
            let _guard = register_app(runtime);
            assert!(render_handle().is_some());
            assert_eq!(is_alt_screen(), Some(false));
        }

        assert!(render_handle().is_none());
        assert_eq!(is_alt_screen(), None);
    }

    #[test]
    fn test_exit_sets_should_exit_flag() {
        let app = App::new(|| Element::text("ok"));
        assert!(!app.should_exit.load(Ordering::SeqCst));

        app.exit();

        assert!(app.should_exit.load(Ordering::SeqCst));
    }

    #[test]
    fn test_exit_updates_runtime_context_exit_state() {
        let app = App::new(|| Element::text("ok"));
        assert!(!app.runtime_context.borrow().should_exit());

        app.exit();

        assert!(app.runtime_context.borrow().should_exit());
    }

    #[test]
    fn app_render_candidate_preserves_typed_error_source() {
        let mut app = App::new(|| Element::text("app"));
        app.layout_engine.set_text_flow_policy(0, "…", 1);
        let failure = app
            .try_prepare_frame(&Element::text("app"), 20, 4)
            .unwrap_err();
        assert!(matches!(
            failure,
            crate::renderer::DynamicFrameError::Incremental(
                crate::layout::IncrementalLayoutError::TextFlow(TextFlowError::InvalidTabStop)
            )
        ));
        let io_error = failure.into_io();
        let dynamic_error = io_error
            .get_ref()
            .and_then(|source| source.downcast_ref::<crate::renderer::DynamicFrameError>())
            .expect("io error must retain DynamicFrameError");
        assert!(matches!(
            dynamic_error
                .source()
                .and_then(|source| {
                    source.downcast_ref::<crate::layout::IncrementalLayoutError>()
                })
                .and_then(std::error::Error::source)
                .and_then(|source| source.downcast_ref::<TextFlowError>()),
            Some(TextFlowError::InvalidTabStop),
        ));
    }

    #[test]
    fn duplicate_key_reaches_app_io_error_without_frame_commit() {
        let mut app = App::new(|| Element::text("app"));
        let mut invalid = Element::root();
        invalid.add_child(Element::text("first").with_key("duplicate"));
        invalid.add_child(Element::text("second").with_key("duplicate"));

        let failure = app
            .try_prepare_frame(&invalid, 20, 4)
            .expect_err("duplicate sibling key must reach the App boundary");
        assert!(matches!(
            &failure,
            crate::renderer::DynamicFrameError::Incremental(
                crate::layout::IncrementalLayoutError::Identity(
                    crate::reconciler::ReconcilePlanError::DuplicateSiblingKey { .. }
                )
            )
        ));
        assert!(app.previous_vnode.is_none());
        assert!(!app.layout_engine.has_tree());
        assert!(
            app.runtime_context
                .borrow()
                .get_measurement_by_key_dims("duplicate")
                .is_none()
        );

        let io_error = failure.into_io();
        let dynamic_error = io_error
            .get_ref()
            .and_then(|source| source.downcast_ref::<crate::renderer::DynamicFrameError>())
            .expect("io error must retain DynamicFrameError");
        let incremental_error = dynamic_error
            .source()
            .and_then(|source| source.downcast_ref::<crate::layout::IncrementalLayoutError>())
            .expect("dynamic error must retain IncrementalLayoutError");
        assert!(matches!(
            incremental_error.source().and_then(|source| {
                source.downcast_ref::<crate::reconciler::ReconcilePlanError>()
            }),
            Some(crate::reconciler::ReconcilePlanError::DuplicateSiblingKey { .. })
        ));
    }
}
