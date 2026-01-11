//! Terminal handling with ANSI escape codes (ink-style)

use crossterm::{
    cursor::{Hide, Show, MoveTo},
    event::{self, Event, KeyCode, KeyModifiers, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::io::{stdout, Write};
use std::time::Duration;

/// ANSI escape codes for terminal control
mod ansi {
    /// Move cursor to specific position (1-indexed)
    pub fn cursor_to(row: u16, col: u16) -> String {
        format!("\x1b[{};{}H", row + 1, col + 1)
    }

    /// Move cursor to column (0-indexed)
    pub fn cursor_to_column(col: u16) -> String {
        format!("\x1b[{}G", col + 1)
    }

    /// Move cursor up n lines
    pub fn cursor_up(n: u16) -> String {
        if n == 0 {
            String::new()
        } else {
            format!("\x1b[{}A", n)
        }
    }

    /// Move cursor down n lines
    pub fn cursor_down(n: u16) -> String {
        if n == 0 {
            String::new()
        } else {
            format!("\x1b[{}B", n)
        }
    }

    /// Erase from cursor to end of line
    pub fn erase_end_of_line() -> &'static str {
        "\x1b[K"
    }

    /// Erase entire line
    pub fn erase_line() -> &'static str {
        "\x1b[2K"
    }

    /// Erase n lines (moves up and clears each line)
    pub fn erase_lines(n: usize) -> String {
        if n == 0 {
            return String::new();
        }

        let mut result = String::new();
        for i in 0..n {
            if i > 0 {
                result.push_str("\x1b[1A"); // Move up
            }
            result.push_str("\x1b[2K"); // Erase line
            result.push_str("\x1b[G");  // Move to column 0
        }
        result
    }

    /// Hide cursor
    pub fn hide_cursor() -> &'static str {
        "\x1b[?25l"
    }

    /// Show cursor
    pub fn show_cursor() -> &'static str {
        "\x1b[?25h"
    }

    /// Save cursor position
    pub fn save_cursor() -> &'static str {
        "\x1b[s"
    }

    /// Restore cursor position
    pub fn restore_cursor() -> &'static str {
        "\x1b[u"
    }
}

/// Terminal abstraction with ink-style rendering
pub struct Terminal {
    /// Previous frame's lines for incremental rendering
    previous_lines: Vec<String>,
    /// Whether we're in alternate screen mode
    alternate_screen: bool,
    /// Whether cursor is hidden
    cursor_hidden: bool,
    /// Whether raw mode is enabled
    raw_mode: bool,
    /// Whether mouse mode is enabled
    mouse_enabled: bool,
}

impl Terminal {
    /// Create a new terminal instance
    pub fn new() -> Self {
        Self {
            previous_lines: Vec::new(),
            alternate_screen: false,
            cursor_hidden: false,
            raw_mode: false,
            mouse_enabled: false,
        }
    }

    /// Enter raw mode and alternate screen (fullscreen mode)
    pub fn enter(&mut self) -> std::io::Result<()> {
        enable_raw_mode()?;
        self.raw_mode = true;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        self.alternate_screen = true;
        self.cursor_hidden = true;
        Ok(())
    }

    /// Exit raw mode and alternate screen
    pub fn exit(&mut self) -> std::io::Result<()> {
        // Disable mouse capture first
        if self.mouse_enabled {
            execute!(stdout(), DisableMouseCapture)?;
            self.mouse_enabled = false;
        }
        if self.alternate_screen {
            execute!(stdout(), Show, LeaveAlternateScreen)?;
            self.alternate_screen = false;
            self.cursor_hidden = false;
        }
        if self.raw_mode {
            disable_raw_mode()?;
            self.raw_mode = false;
        }
        Ok(())
    }

    /// Enter inline mode (renders in current terminal position)
    pub fn enter_inline(&mut self) -> std::io::Result<()> {
        enable_raw_mode()?;
        self.raw_mode = true;

        // Hide cursor during rendering
        let mut stdout = stdout();
        write!(stdout, "{}", ansi::hide_cursor())?;
        stdout.flush()?;
        self.cursor_hidden = true;

        Ok(())
    }

    /// Exit inline mode
    pub fn exit_inline(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout();

        // Disable mouse capture first
        if self.mouse_enabled {
            execute!(stdout, DisableMouseCapture)?;
            self.mouse_enabled = false;
        }

        // Show cursor
        if self.cursor_hidden {
            write!(stdout, "{}", ansi::show_cursor())?;
            self.cursor_hidden = false;
        }

        // Move to the end of output and add newline
        let line_count = self.previous_lines.len();
        if line_count > 0 {
            // We're at the last line, just add a newline
            writeln!(stdout)?;
        }

        stdout.flush()?;

        if self.raw_mode {
            disable_raw_mode()?;
            self.raw_mode = false;
        }

        Ok(())
    }

    /// Render output to terminal (ink-style incremental rendering)
    pub fn render(&mut self, output: &str) -> std::io::Result<()> {
        if self.alternate_screen {
            self.render_fullscreen(output)
        } else {
            self.render_inline(output)
        }
    }

    /// Render in fullscreen/alternate screen mode
    fn render_fullscreen(&mut self, output: &str) -> std::io::Result<()> {
        let mut stdout = stdout();

        // Move to top-left
        execute!(stdout, MoveTo(0, 0))?;

        let new_lines: Vec<&str> = output.lines().collect();

        // Incremental update - only redraw changed lines
        for (i, new_line) in new_lines.iter().enumerate() {
            let old_line = self.previous_lines.get(i).map(|s| s.as_str());

            if old_line != Some(*new_line) {
                // Move to line and clear it, then write new content
                write!(stdout, "{}{}{}",
                    ansi::cursor_to(i as u16, 0),
                    ansi::erase_line(),
                    new_line
                )?;
            }
        }

        // Clear any extra lines from previous render
        if self.previous_lines.len() > new_lines.len() {
            for i in new_lines.len()..self.previous_lines.len() {
                write!(stdout, "{}{}",
                    ansi::cursor_to(i as u16, 0),
                    ansi::erase_line()
                )?;
            }
        }

        stdout.flush()?;

        // Store current lines for next comparison
        self.previous_lines = new_lines.iter().map(|s| s.to_string()).collect();

        Ok(())
    }

    /// Render in inline mode (like ink's default behavior)
    fn render_inline(&mut self, output: &str) -> std::io::Result<()> {
        let mut stdout = stdout();
        let new_lines: Vec<&str> = output.lines().collect();
        let prev_count = self.previous_lines.len();
        let new_count = new_lines.len();

        // If this is the first render, just write the output
        if prev_count == 0 {
            for (i, line) in new_lines.iter().enumerate() {
                // Move to column 0 and write line
                write!(stdout, "{}{}{}",
                    ansi::cursor_to_column(0),
                    ansi::erase_line(),
                    line
                )?;
                if i < new_count - 1 {
                    write!(stdout, "\r\n")?; // Use \r\n for raw mode
                }
            }
            stdout.flush()?;
            self.previous_lines = new_lines.iter().map(|s| s.to_string()).collect();
            return Ok(());
        }

        // Move cursor to the start of our output area
        if prev_count > 1 {
            write!(stdout, "{}", ansi::cursor_up(prev_count as u16 - 1))?;
        }

        // Render each line
        for (i, new_line) in new_lines.iter().enumerate() {
            // Always move to column 0, clear line, and write
            write!(stdout, "{}{}{}",
                ansi::cursor_to_column(0),
                ansi::erase_line(),
                new_line
            )?;

            if i < new_count - 1 {
                write!(stdout, "\r\n")?; // Use \r\n for raw mode
            }
        }

        // Clear extra lines if new output is shorter
        if new_count < prev_count {
            for _ in new_count..prev_count {
                write!(stdout, "\r\n{}{}", ansi::cursor_to_column(0), ansi::erase_line())?;
            }
            // Move back up to end of new content
            write!(stdout, "{}", ansi::cursor_up((prev_count - new_count) as u16))?;
        }

        stdout.flush()?;

        // Store current lines for next comparison
        self.previous_lines = new_lines.iter().map(|s| s.to_string()).collect();

        Ok(())
    }

    /// Clear the current output
    pub fn clear(&mut self) -> std::io::Result<()> {
        if self.previous_lines.is_empty() {
            return Ok(());
        }

        let mut stdout = stdout();
        let line_count = self.previous_lines.len();

        if self.alternate_screen {
            execute!(stdout, MoveTo(0, 0))?;
            for i in 0..line_count {
                write!(stdout, "{}{}",
                    ansi::cursor_to(i as u16, 0),
                    ansi::erase_line()
                )?;
            }
        } else {
            // Move up and clear each line
            if line_count > 1 {
                write!(stdout, "{}", ansi::cursor_up(line_count as u16 - 1))?;
            }
            for _ in 0..line_count {
                write!(stdout, "{}{}\n", ansi::cursor_to_column(0), ansi::erase_line())?;
            }
            // Move back up
            write!(stdout, "{}", ansi::cursor_up(line_count as u16))?;
        }

        stdout.flush()?;
        self.previous_lines.clear();

        Ok(())
    }

    /// Get terminal size
    pub fn size() -> std::io::Result<(u16, u16)> {
        crossterm::terminal::size()
    }

    /// Poll for input event
    pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
        if event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }

    /// Read input event (blocking)
    pub fn read_event() -> std::io::Result<Event> {
        event::read()
    }

    /// Check if Ctrl+C was pressed
    pub fn is_ctrl_c(event: &Event) -> bool {
        matches!(
            event,
            Event::Key(crossterm::event::KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL)
        )
    }

    /// Enable mouse capture
    pub fn enable_mouse(&mut self) -> std::io::Result<()> {
        if !self.mouse_enabled {
            execute!(stdout(), EnableMouseCapture)?;
            self.mouse_enabled = true;
        }
        Ok(())
    }

    /// Disable mouse capture
    pub fn disable_mouse(&mut self) -> std::io::Result<()> {
        if self.mouse_enabled {
            execute!(stdout(), DisableMouseCapture)?;
            self.mouse_enabled = false;
        }
        Ok(())
    }

    /// Check if mouse is enabled
    pub fn is_mouse_enabled(&self) -> bool {
        self.mouse_enabled
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Ensure we clean up on drop
        let _ = self.exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_size() {
        // This test may fail in CI environments without a terminal
        if let Ok((width, height)) = Terminal::size() {
            assert!(width > 0);
            assert!(height > 0);
        }
    }

    #[test]
    fn test_ansi_codes() {
        assert_eq!(ansi::cursor_to(0, 0), "\x1b[1;1H");
        assert_eq!(ansi::cursor_to(5, 10), "\x1b[6;11H");
        assert_eq!(ansi::cursor_up(3), "\x1b[3A");
        assert_eq!(ansi::erase_line(), "\x1b[2K");
    }
}
