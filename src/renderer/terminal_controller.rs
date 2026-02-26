//! Terminal control helpers extracted from App orchestration.

use std::sync::Arc;

use super::Terminal;
use super::registry::{AppRuntime, ModeSwitch, Printable};
use super::render_to_string::render_to_string;

/// Handles terminal mode switching, terminal commands, and inline println rendering.
pub(crate) struct TerminalController;

impl TerminalController {
    pub(crate) fn handle_mode_switch(
        terminal: &mut Terminal,
        runtime: &Arc<AppRuntime>,
        mode_switch: ModeSwitch,
    ) -> std::io::Result<()> {
        match mode_switch {
            ModeSwitch::EnterAltScreen => {
                if !terminal.is_alt_screen() {
                    terminal.switch_to_alt_screen()?;
                    runtime.set_alt_screen_state(true);
                    terminal.repaint();
                }
            }
            ModeSwitch::ExitAltScreen => {
                if terminal.is_alt_screen() {
                    terminal.switch_to_inline()?;
                    runtime.set_alt_screen_state(false);
                    terminal.repaint();
                }
            }
        }
        Ok(())
    }

    pub(crate) fn handle_terminal_cmd(
        terminal: &mut Terminal,
        runtime: &Arc<AppRuntime>,
        cmd: crate::cmd::TerminalCmd,
        last_width: &mut u16,
        last_height: &mut u16,
    ) -> std::io::Result<()> {
        use crate::cmd::TerminalCmd;
        use crossterm::{cursor, execute, terminal as ct};
        use std::io::stdout;

        match cmd {
            TerminalCmd::ClearScreen => {
                if terminal.is_alt_screen() {
                    execute!(
                        stdout(),
                        ct::Clear(ct::ClearType::All),
                        cursor::MoveTo(0, 0)
                    )?;
                } else {
                    terminal.clear()?;
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
                // This triggers a resize check on next render.
                *last_width = 0;
                *last_height = 0;
            }
            TerminalCmd::EnterAltScreen => {
                if !terminal.is_alt_screen() {
                    terminal.switch_to_alt_screen()?;
                    runtime.set_alt_screen_state(true);
                    terminal.repaint();
                }
            }
            TerminalCmd::ExitAltScreen => {
                if terminal.is_alt_screen() {
                    terminal.switch_to_inline()?;
                    runtime.set_alt_screen_state(false);
                    terminal.repaint();
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

    pub(crate) fn handle_println_messages(
        terminal: &mut Terminal,
        messages: &[Printable],
    ) -> std::io::Result<()> {
        // Println only works in inline mode.
        if terminal.is_alt_screen() {
            return Ok(());
        }

        // Get terminal width for rendering elements.
        let (width, _) = Terminal::size().unwrap_or((80, 24));

        for message in messages {
            match message {
                Printable::Text(text) => {
                    terminal.println(text)?;
                }
                Printable::Element(element) => {
                    let rendered = render_to_string(element, width);
                    terminal.println(&rendered)?;
                }
            }
        }

        terminal.repaint();
        Ok(())
    }

    pub(crate) fn handle_resize(
        terminal: &mut Terminal,
        last_width: &mut u16,
        last_height: &mut u16,
        new_width: u16,
        new_height: u16,
    ) {
        use crossterm::cursor::MoveTo;
        use crossterm::execute;
        use crossterm::terminal::{Clear, ClearType};
        use std::io::stdout;

        if new_width != *last_width || new_height != *last_height {
            if terminal.is_alt_screen() {
                let _ = execute!(stdout(), MoveTo(0, 0), Clear(ClearType::All));
            }
            // Inline mode: repaint only to avoid clearing scrollback.
            terminal.repaint();
        }

        *last_width = new_width;
        *last_height = new_height;
    }
}
