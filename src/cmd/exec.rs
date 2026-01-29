//! External process execution types
//!
//! This module provides types for executing external interactive processes
//! (like vim, less, etc.) that require suspending the TUI.

use std::path::PathBuf;

/// Configuration for executing an external process
#[derive(Debug, Clone)]
pub struct ExecConfig {
    /// The command/program to execute
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
    /// Environment variables to set (key, value pairs)
    pub env: Vec<(String, String)>,
    /// Working directory for the process
    pub current_dir: Option<PathBuf>,
}

impl ExecConfig {
    /// Create a new exec config with just a command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: Vec::new(),
            current_dir: None,
        }
    }

    /// Add arguments to the command
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Add a single argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add environment variables
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env
            .extend(vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Add a single environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set the working directory
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }
}

/// Result of executing an external process
#[derive(Debug, Clone)]
pub struct ExecResult {
    /// Exit code of the process (None if terminated by signal)
    pub exit_code: Option<i32>,
    /// Whether the process completed successfully (exit code 0)
    pub success: bool,
    /// Error message if the process failed to start
    pub error: Option<String>,
}

impl ExecResult {
    /// Create a successful result
    pub fn success(exit_code: i32) -> Self {
        Self {
            exit_code: Some(exit_code),
            success: exit_code == 0,
            error: None,
        }
    }

    /// Create a result for a process terminated by signal
    pub fn terminated_by_signal() -> Self {
        Self {
            exit_code: None,
            success: false,
            error: Some("Process terminated by signal".to_string()),
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            exit_code: None,
            success: false,
            error: Some(message.into()),
        }
    }
}

/// Internal request for executing an external process
pub(crate) struct ExecRequest {
    pub config: ExecConfig,
    pub callback: Box<dyn FnOnce(ExecResult) + Send + 'static>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_config_new() {
        let config = ExecConfig::new("vim");
        assert_eq!(config.command, "vim");
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.current_dir.is_none());
    }

    #[test]
    fn test_exec_config_builder() {
        let config = ExecConfig::new("vim")
            .arg("file.txt")
            .args(["--clean", "-n"])
            .env("TERM", "xterm-256color")
            .envs([("FOO", "bar"), ("BAZ", "qux")])
            .current_dir("/tmp");

        assert_eq!(config.command, "vim");
        assert_eq!(config.args, vec!["file.txt", "--clean", "-n"]);
        assert_eq!(
            config.env,
            vec![
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("FOO".to_string(), "bar".to_string()),
                ("BAZ".to_string(), "qux".to_string()),
            ]
        );
        assert_eq!(config.current_dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_exec_result_success() {
        let result = ExecResult::success(0);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.success);
        assert!(result.error.is_none());

        let result = ExecResult::success(1);
        assert_eq!(result.exit_code, Some(1));
        assert!(!result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_exec_result_terminated() {
        let result = ExecResult::terminated_by_signal();
        assert!(result.exit_code.is_none());
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_exec_result_error() {
        let result = ExecResult::error("Command not found");
        assert!(result.exit_code.is_none());
        assert!(!result.success);
        assert_eq!(result.error, Some("Command not found".to_string()));
    }
}
