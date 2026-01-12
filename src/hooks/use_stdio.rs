//! Standard I/O hooks for accessing stdin, stdout, and stderr

use std::io::{self, IsTerminal, Stderr, Stdout, Write};

/// Handle for writing to stdout
#[derive(Clone, Copy)]
pub struct StdoutHandle;

impl StdoutHandle {
    /// Write a string to stdout
    pub fn write(&self, s: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        write!(stdout, "{}", s)?;
        stdout.flush()
    }

    /// Write a line to stdout
    pub fn writeln(&self, s: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        writeln!(stdout, "{}", s)?;
        stdout.flush()
    }

    /// Get raw stdout handle for advanced usage
    pub fn raw(&self) -> Stdout {
        io::stdout()
    }
}

/// Handle for writing to stderr
#[derive(Clone, Copy)]
pub struct StderrHandle;

impl StderrHandle {
    /// Write a string to stderr
    pub fn write(&self, s: &str) -> io::Result<()> {
        let mut stderr = io::stderr();
        write!(stderr, "{}", s)?;
        stderr.flush()
    }

    /// Write a line to stderr
    pub fn writeln(&self, s: &str) -> io::Result<()> {
        let mut stderr = io::stderr();
        writeln!(stderr, "{}", s)?;
        stderr.flush()
    }

    /// Get raw stderr handle for advanced usage
    pub fn raw(&self) -> Stderr {
        io::stderr()
    }
}

/// Handle for reading from stdin
#[derive(Clone, Copy)]
pub struct StdinHandle;

impl StdinHandle {
    /// Check if stdin is a TTY (terminal)
    ///
    /// Returns true when stdin is connected to a terminal,
    /// false when input is piped or redirected.
    pub fn is_tty(&self) -> bool {
        io::stdin().is_terminal()
    }

    /// Check if stdout is a TTY (terminal)
    pub fn stdout_is_tty(&self) -> bool {
        io::stdout().is_terminal()
    }

    /// Check if stderr is a TTY (terminal)
    pub fn stderr_is_tty(&self) -> bool {
        io::stderr().is_terminal()
    }

    /// Read a line from stdin (blocking)
    /// Note: This will block the event loop, use with caution
    pub fn read_line(&self) -> io::Result<String> {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        Ok(buffer)
    }
}

/// Hook to access stdout for writing
///
/// # Example
///
/// ```ignore
/// let stdout = use_stdout();
///
/// stdout.writeln("Debug output").ok();
/// ```
pub fn use_stdout() -> StdoutHandle {
    StdoutHandle
}

/// Hook to access stderr for writing
///
/// # Example
///
/// ```ignore
/// let stderr = use_stderr();
///
/// stderr.writeln("Error: something went wrong").ok();
/// ```
pub fn use_stderr() -> StderrHandle {
    StderrHandle
}

/// Hook to access stdin for reading
///
/// Note: For keyboard input, prefer use_input which integrates with the event loop.
/// This hook is mainly for direct stdin access when needed.
///
/// # Example
///
/// ```ignore
/// let stdin = use_stdin();
///
/// if stdin.is_tty() {
///     // Running in a terminal
/// }
/// ```
pub fn use_stdin() -> StdinHandle {
    StdinHandle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_handle() {
        let stdout = use_stdout();
        // Just verify we can get the handle
        let _ = stdout.raw();
    }

    #[test]
    fn test_stderr_handle() {
        let stderr = use_stderr();
        // Just verify we can get the handle
        let _ = stderr.raw();
    }

    #[test]
    fn test_stdin_handle() {
        let stdin = use_stdin();
        // Just verify the is_tty method works
        let _ = stdin.is_tty();
    }
}
