//! Spinner component for loading animations
//!
//! Provides a customizable loading spinner with optional cancellation support.

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal;

/// A loading spinner with optional cancellation support
///
/// # Example
///
/// ```ignore
/// use rnk::components::Spinner;
///
/// let spinner = Spinner::new("Loading...");
/// // Do some work...
/// let was_cancelled = spinner.stop();
/// ```
pub struct Spinner {
    running: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    /// Create a new spinner with default settings
    ///
    /// # Arguments
    ///
    /// * `message` - The message to display next to the spinner
    pub fn new(message: impl Into<String>) -> Self {
        Self::builder().message(message).build()
    }

    /// Create a spinner builder for customization
    pub fn builder() -> SpinnerBuilder {
        SpinnerBuilder::default()
    }

    /// Stop the spinner and return whether it was cancelled
    pub fn stop(mut self) -> bool {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Check if the spinner was cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Builder for customizing spinner appearance and behavior
pub struct SpinnerBuilder {
    message: String,
    frames: Vec<&'static str>,
    interval: Duration,
    cancellable: bool,
    cancel_key: KeyCode,
}

impl Default for SpinnerBuilder {
    fn default() -> Self {
        Self {
            message: "Loading...".to_string(),
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            interval: Duration::from_millis(80),
            cancellable: true,
            cancel_key: KeyCode::Esc,
        }
    }
}

impl SpinnerBuilder {
    /// Set the spinner message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Set custom animation frames
    pub fn frames(mut self, frames: Vec<&'static str>) -> Self {
        self.frames = frames;
        self
    }

    /// Set the animation interval
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Enable or disable cancellation
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    /// Set the key used for cancellation (default: Esc)
    pub fn cancel_key(mut self, key: KeyCode) -> Self {
        self.cancel_key = key;
        self
    }

    /// Build and start the spinner
    pub fn build(self) -> Spinner {
        let running = Arc::new(AtomicBool::new(true));
        let cancelled = Arc::new(AtomicBool::new(false));
        let running_clone = running.clone();
        let cancelled_clone = cancelled.clone();

        let handle = std::thread::spawn(move || {
            let mut i = 0;

            // Enable raw mode for key detection if cancellable
            if self.cancellable {
                let _ = terminal::enable_raw_mode();
            }

            while running_clone.load(Ordering::Relaxed) {
                // Check for cancel key if cancellable
                if self.cancellable {
                    if event::poll(self.interval).unwrap_or(false) {
                        if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
                            if code == self.cancel_key {
                                cancelled_clone.store(true, Ordering::SeqCst);
                                running_clone.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                } else {
                    std::thread::sleep(self.interval);
                }

                // Render spinner frame
                let cancel_hint = if self.cancellable {
                    " \x1b[2m(ESC to cancel)\x1b[0m".to_string()
                } else {
                    String::new()
                };

                print!(
                    "\x1b[2K\r\x1b[33m{} {}{}\x1b[0m",
                    self.frames[i], self.message, cancel_hint
                );
                io::stdout().flush().unwrap();
                i = (i + 1) % self.frames.len();
            }

            if self.cancellable {
                let _ = terminal::disable_raw_mode();
            }

            // Clear the line
            print!("\x1b[2K\r");
            io::stdout().flush().unwrap();
        });

        Spinner {
            running,
            cancelled,
            handle: Some(handle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::new("Testing...");
        assert!(!spinner.is_cancelled());
        spinner.stop();
    }

    #[test]
    fn test_spinner_builder() {
        let spinner = Spinner::builder()
            .message("Custom message")
            .cancellable(false)
            .build();
        assert!(!spinner.is_cancelled());
        spinner.stop();
    }
}
