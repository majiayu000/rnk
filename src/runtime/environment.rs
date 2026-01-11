//! Environment detection utilities
//!
//! Provides detection for:
//! - CI environments (GitHub Actions, GitLab CI, etc.)
//! - TTY (terminal) vs non-TTY (pipe, redirect)
//! - Terminal capabilities

use std::io::IsTerminal;

/// Environment information
#[derive(Debug, Clone)]
pub struct Environment {
    /// Whether running in a CI environment
    pub is_ci: bool,
    /// Whether stdout is a TTY
    pub is_tty: bool,
    /// Whether stdin is a TTY
    pub stdin_is_tty: bool,
    /// Whether stderr is a TTY
    pub stderr_is_tty: bool,
    /// Terminal width (if available)
    pub width: Option<u16>,
    /// Terminal height (if available)
    pub height: Option<u16>,
}

impl Environment {
    /// Detect the current environment
    pub fn detect() -> Self {
        let (width, height) = crossterm::terminal::size().ok().unzip();

        Self {
            is_ci: is_ci(),
            is_tty: is_tty(),
            stdin_is_tty: std::io::stdin().is_terminal(),
            stderr_is_tty: std::io::stderr().is_terminal(),
            width,
            height,
        }
    }

    /// Check if colors should be disabled
    pub fn should_disable_colors(&self) -> bool {
        !self.is_tty
            || std::env::var("NO_COLOR").is_ok()
            || std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false)
    }

    /// Check if interactive features should be disabled
    pub fn should_disable_interactive(&self) -> bool {
        self.is_ci || !self.stdin_is_tty
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::detect()
    }
}

/// Check if running in a CI environment
///
/// Detects common CI providers:
/// - Generic CI flag
/// - GitHub Actions
/// - GitLab CI
/// - Jenkins
/// - Travis CI
/// - CircleCI
/// - Azure Pipelines
/// - Bitbucket Pipelines
/// - And others
///
/// # Example
///
/// ```
/// use tink::runtime::is_ci;
///
/// if is_ci() {
///     println!("Running in CI, disabling interactive features");
/// }
/// ```
pub fn is_ci() -> bool {
    // Generic CI flag (used by many CI systems)
    std::env::var("CI").is_ok()
        // GitHub Actions
        || std::env::var("GITHUB_ACTIONS").is_ok()
        // GitLab CI
        || std::env::var("GITLAB_CI").is_ok()
        // Jenkins
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("BUILD_NUMBER").is_ok()
        // Travis CI
        || std::env::var("TRAVIS").is_ok()
        // CircleCI
        || std::env::var("CIRCLECI").is_ok()
        // Azure Pipelines
        || std::env::var("TF_BUILD").is_ok()
        // Bitbucket Pipelines
        || std::env::var("BITBUCKET_BUILD_NUMBER").is_ok()
        // AppVeyor
        || std::env::var("APPVEYOR").is_ok()
        // Drone CI
        || std::env::var("DRONE").is_ok()
        // Buildkite
        || std::env::var("BUILDKITE").is_ok()
        // TeamCity
        || std::env::var("TEAMCITY_VERSION").is_ok()
        // Codeship
        || std::env::var("CI_NAME").map(|v| v == "codeship").unwrap_or(false)
}

/// Check if stdout is a TTY (terminal)
///
/// Returns `false` when output is being piped or redirected.
///
/// # Example
///
/// ```
/// use tink::runtime::is_tty;
///
/// if !is_tty() {
///     println!("Output is being redirected, using plain text");
/// }
/// ```
pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Get the terminal size, with fallback values
///
/// Returns (80, 24) as fallback when size cannot be determined.
pub fn terminal_size() -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((80, 24))
}

/// Get the TERM environment variable
pub fn term_program() -> Option<String> {
    std::env::var("TERM_PROGRAM").ok()
}

/// Check if running in a specific terminal emulator
pub fn is_terminal_emulator(name: &str) -> bool {
    term_program().map(|t| t.to_lowercase().contains(&name.to_lowercase())).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_detect() {
        let env = Environment::detect();
        // Just ensure it doesn't panic
        let _ = env.is_ci;
        let _ = env.is_tty;
    }

    #[test]
    fn test_is_ci_returns_bool() {
        // This test just ensures the function runs
        let _ = is_ci();
    }

    #[test]
    fn test_is_tty_returns_bool() {
        let _ = is_tty();
    }

    #[test]
    fn test_terminal_size_has_fallback() {
        let (w, h) = terminal_size();
        assert!(w > 0);
        assert!(h > 0);
    }

    #[test]
    fn test_should_disable_colors() {
        let env = Environment {
            is_ci: false,
            is_tty: false, // Not a TTY
            stdin_is_tty: false,
            stderr_is_tty: false,
            width: None,
            height: None,
        };
        assert!(env.should_disable_colors());
    }
}
