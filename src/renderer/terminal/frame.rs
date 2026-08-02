//! Prepared terminal frame planning and publication.

use std::io::{self, Write};

use crossterm::Command;

use super::{
    DisableMouseCapture, EnableMouseCapture, LineDiffOp, Terminal, ansi, fullscreen_line_diff,
};

/// Terminal bytes and mirror state staged for one whole application frame.
pub(crate) struct PreparedTerminalFrame {
    bytes: Vec<u8>,
    previous_lines: Vec<String>,
    last_output: String,
    mouse_enabled: bool,
    inline_lines_rendered: usize,
}

impl Terminal {
    /// Plan mouse, static-content, and dynamic-frame output without writing or
    /// changing the terminal's committed mirror state.
    pub(crate) fn prepare_frame(
        &self,
        static_lines: &[String],
        dynamic_output: &str,
        mouse_enabled: bool,
    ) -> PreparedTerminalFrame {
        let mut bytes = String::new();
        append_mouse_transition(&mut bytes, self.mouse_enabled, mouse_enabled);

        let mut previous_lines = self.previous_lines.clone();
        let mut last_output = self.last_output.clone();
        let mut inline_lines_rendered = self.inline_lines_rendered;

        if !static_lines.is_empty() {
            append_clear(
                &mut bytes,
                self.alternate_screen,
                &previous_lines,
                inline_lines_rendered,
            );
            for line in static_lines {
                bytes.push_str(line);
                bytes.push_str(ansi::erase_end_of_line());
                bytes.push('\n');
            }
            previous_lines.clear();
            last_output.clear();
            inline_lines_rendered = 0;
        }

        if dynamic_output != last_output || previous_lines.is_empty() {
            if self.alternate_screen {
                append_fullscreen_render(&mut bytes, &previous_lines, dynamic_output);
            } else {
                append_inline_render(
                    &mut bytes,
                    &previous_lines,
                    inline_lines_rendered,
                    dynamic_output,
                );
            }
            previous_lines = dynamic_output.lines().map(str::to_owned).collect();
            inline_lines_rendered = previous_lines.len();
            last_output = dynamic_output.to_owned();
        }

        PreparedTerminalFrame {
            bytes: bytes.into_bytes(),
            previous_lines,
            last_output,
            mouse_enabled,
            inline_lines_rendered,
        }
    }

    /// Publish a prepared frame to stdout and then swap the terminal mirror.
    pub(crate) fn commit_prepared(&mut self, prepared: PreparedTerminalFrame) -> io::Result<()> {
        let mut output = std::io::stdout();
        self.commit_prepared_with_writer(prepared, &mut output)
    }

    /// Publish through an injected writer. Terminal state is exchanged only
    /// after both the write and flush have succeeded.
    pub(crate) fn commit_prepared_with_writer(
        &mut self,
        prepared: PreparedTerminalFrame,
        writer: &mut impl Write,
    ) -> io::Result<()> {
        writer.write_all(&prepared.bytes)?;
        writer.flush()?;
        self.previous_lines = prepared.previous_lines;
        self.last_output = prepared.last_output;
        self.mouse_enabled = prepared.mouse_enabled;
        self.inline_lines_rendered = prepared.inline_lines_rendered;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn committed_frame_state(&self) -> (Vec<String>, String, bool, usize) {
        (
            self.previous_lines.clone(),
            self.last_output.clone(),
            self.mouse_enabled,
            self.inline_lines_rendered,
        )
    }
}

impl PreparedTerminalFrame {
    #[cfg(test)]
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn append_mouse_transition(bytes: &mut String, current: bool, target: bool) {
    if current == target {
        return;
    }
    let result = if target {
        EnableMouseCapture.write_ansi(bytes)
    } else {
        DisableMouseCapture.write_ansi(bytes)
    };
    result.expect("writing ANSI into String cannot fail");
}

fn append_clear(
    bytes: &mut String,
    alternate_screen: bool,
    previous_lines: &[String],
    inline_lines_rendered: usize,
) {
    if previous_lines.is_empty() && inline_lines_rendered == 0 {
        return;
    }
    if alternate_screen {
        bytes.push_str(&ansi::cursor_to(0, 0));
        for row in 0..previous_lines.len() {
            bytes.push_str(&ansi::cursor_to(row as u16, 0));
            bytes.push_str(ansi::erase_line());
        }
        return;
    }

    let line_count = inline_lines_rendered.max(previous_lines.len());
    if line_count > 1 {
        bytes.push_str(&ansi::cursor_up(line_count as u16 - 1));
    }
    for _ in 0..line_count {
        bytes.push_str(&ansi::cursor_to_column(0));
        bytes.push_str(ansi::erase_line());
        bytes.push('\n');
    }
    bytes.push_str(&ansi::cursor_up(line_count as u16));
}

fn append_fullscreen_render(bytes: &mut String, previous_lines: &[String], output: &str) {
    bytes.push_str(&ansi::cursor_to(0, 0));
    let new_lines: Vec<&str> = output.lines().collect();
    for op in fullscreen_line_diff(previous_lines, &new_lines) {
        match op {
            LineDiffOp::Rewrite { row, line } => {
                bytes.push_str(&ansi::cursor_to(row as u16, 0));
                bytes.push_str(ansi::erase_line());
                bytes.push_str(line);
            }
            LineDiffOp::Clear { row } => {
                bytes.push_str(&ansi::cursor_to(row as u16, 0));
                bytes.push_str(ansi::erase_line());
            }
        }
    }
}

fn append_inline_render(
    bytes: &mut String,
    previous_lines: &[String],
    lines_on_screen: usize,
    output: &str,
) {
    let new_lines: Vec<&str> = output.lines().collect();
    let new_count = new_lines.len();
    if lines_on_screen > 0 {
        if lines_on_screen > 1 {
            bytes.push_str(&ansi::cursor_up(lines_on_screen as u16 - 1));
        }
        bytes.push_str(&ansi::cursor_to_column(0));
    }

    let max_lines = lines_on_screen.max(new_count);
    for (index, new_line) in new_lines.iter().enumerate() {
        if previous_lines.get(index).map(String::as_str) != Some(*new_line) {
            bytes.push_str(ansi::erase_line());
            bytes.push_str(new_line);
        }
        if index < max_lines.saturating_sub(1) {
            bytes.push_str("\r\n");
        }
    }
    for index in new_count..max_lines {
        bytes.push_str(ansi::erase_line());
        if index < max_lines.saturating_sub(1) {
            bytes.push_str("\r\n");
        }
    }
    if new_count < lines_on_screen {
        bytes.push_str(&ansi::cursor_up((lines_on_screen - new_count) as u16));
    }
    bytes.push_str(&ansi::cursor_to_column(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FaultWriter {
        bytes: Vec<u8>,
        fail_flush: bool,
    }

    impl Write for FaultWriter {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(input);
            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::Error::other("injected terminal flush failure"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn failed_terminal_commit_preserves_mirror_state() {
        let mut terminal = Terminal::new();
        terminal.previous_lines = vec!["old".to_owned()];
        terminal.last_output = "old".to_owned();
        terminal.inline_lines_rendered = 1;
        let before = terminal.committed_frame_state();
        let prepared = terminal.prepare_frame(&[], "new", true);
        let mut writer = FaultWriter {
            fail_flush: true,
            ..FaultWriter::default()
        };

        assert!(
            terminal
                .commit_prepared_with_writer(prepared, &mut writer)
                .is_err()
        );
        assert_eq!(terminal.committed_frame_state(), before);
    }

    #[test]
    fn mouse_and_mixed_content_are_emitted_only_by_commit() {
        let mut terminal = Terminal::new();
        terminal.previous_lines = vec!["old".to_owned()];
        terminal.last_output = "old".to_owned();
        terminal.inline_lines_rendered = 1;
        let before = terminal.committed_frame_state();

        let prepared = terminal.prepare_frame(&["static".to_owned()], "dynamic", true);
        assert_eq!(terminal.committed_frame_state(), before);
        assert!(String::from_utf8_lossy(prepared.bytes()).contains("?1000h"));

        let mut writer = FaultWriter::default();
        terminal
            .commit_prepared_with_writer(prepared, &mut writer)
            .expect("prepared terminal frame commits");
        let emitted = String::from_utf8(writer.bytes).expect("ANSI is UTF-8");
        assert!(emitted.contains("static"));
        assert!(emitted.contains("dynamic"));
        assert!(terminal.is_mouse_enabled());
        assert_eq!(terminal.last_output, "dynamic");
        terminal.mouse_enabled = false;
    }
}
