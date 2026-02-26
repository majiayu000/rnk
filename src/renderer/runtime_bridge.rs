//! Runtime queue bridge for App event-loop orchestration.

use std::sync::Arc;

use crate::cmd::run_exec_process;

use super::Terminal;
use super::registry::{AppRuntime, AppSink};
use super::terminal_controller::TerminalController;

/// Bridge that drains runtime/registry queues and delegates to terminal handlers.
pub(crate) struct RuntimeBridge;

impl RuntimeBridge {
    /// Drain and process queued exec requests.
    pub(crate) fn handle_exec_requests(
        terminal: &mut Terminal,
        runtime: &Arc<AppRuntime>,
    ) -> std::io::Result<()> {
        let exec_requests = runtime.take_exec_requests();
        for request in exec_requests {
            terminal.suspend()?;
            let result = run_exec_process(&request.config);
            terminal.resume()?;
            (request.callback)(result);
            runtime.request_render();
        }
        Ok(())
    }

    /// Drain and process queued terminal commands.
    pub(crate) fn handle_terminal_commands(
        terminal: &mut Terminal,
        runtime: &Arc<AppRuntime>,
        last_width: &mut u16,
        last_height: &mut u16,
    ) -> std::io::Result<()> {
        let terminal_cmds = runtime.take_terminal_cmds();
        for cmd in terminal_cmds {
            TerminalController::handle_terminal_cmd(
                terminal,
                runtime,
                cmd,
                last_width,
                last_height,
            )?;
        }
        Ok(())
    }

    /// Process one pending mode switch request.
    pub(crate) fn handle_mode_switch_request(
        terminal: &mut Terminal,
        runtime: &Arc<AppRuntime>,
    ) -> std::io::Result<()> {
        if let Some(mode_switch) = runtime.take_mode_switch_request() {
            TerminalController::handle_mode_switch(terminal, runtime, mode_switch)?;
        }
        Ok(())
    }

    /// Process queued println messages.
    pub(crate) fn handle_println_messages(
        terminal: &mut Terminal,
        runtime: &Arc<AppRuntime>,
    ) -> std::io::Result<()> {
        let messages = runtime.take_println_messages();
        if !messages.is_empty() {
            TerminalController::handle_println_messages(terminal, &messages)?;
        }
        Ok(())
    }
}
