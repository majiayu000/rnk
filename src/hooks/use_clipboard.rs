//! use_clipboard hook for clipboard operations
//!
//! Provides clipboard read/write functionality using system commands.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let clipboard = use_clipboard();
//!
//!     use_input(move |input, key| {
//!         if key.ctrl && input == "c" {
//!             clipboard.write("Copied text!");
//!         } else if key.ctrl && input == "v" {
//!             if let Some(text) = clipboard.read() {
//!                 println!("Pasted: {}", text);
//!             }
//!         }
//!     });
//!
//!     // ...
//! }
//! ```

use std::process::Command;

/// Handle for clipboard operations
#[derive(Clone, Copy)]
pub struct ClipboardHandle;

impl ClipboardHandle {
    /// Read text from clipboard
    pub fn read(&self) -> Option<String> {
        read_clipboard()
    }

    /// Write text to clipboard
    pub fn write(&self, text: &str) -> bool {
        write_clipboard(text)
    }

    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        is_clipboard_available()
    }

    /// Clear the clipboard
    pub fn clear(&self) -> bool {
        write_clipboard("")
    }
}

/// Create a clipboard handle
pub fn use_clipboard() -> ClipboardHandle {
    ClipboardHandle
}

/// Read text from system clipboard
pub fn read_clipboard() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("pbpaste")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
    }

    #[cfg(target_os = "linux")]
    {
        // Try xclip first, then xsel
        Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .or_else(|| {
                Command::new("xsel")
                    .args(["--clipboard", "--output"])
                    .output()
                    .ok()
                    .and_then(|output| {
                        if output.status.success() {
                            String::from_utf8(output.stdout).ok()
                        } else {
                            None
                        }
                    })
            })
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("powershell")
            .args(["-command", "Get-Clipboard"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Write text to system clipboard
pub fn write_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            })
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            })
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = text.replace("\"", "`\"");
        Command::new("powershell")
            .args(["-command", &format!("Set-Clipboard -Value \"{}\"", escaped)])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = text;
        false
    }
}

/// Check if clipboard is available on this system
pub fn is_clipboard_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        Command::new("which")
            .arg("pbcopy")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("which")
            .arg("xclip")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || Command::new("which")
                .arg("xsel")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        true // PowerShell is always available on Windows
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_handle() {
        let clipboard = use_clipboard();
        let _ = clipboard.is_available();
    }

    #[test]
    fn test_is_clipboard_available() {
        // Just check it doesn't panic
        let _ = is_clipboard_available();
    }
}
