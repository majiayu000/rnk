//! Accessibility hooks for screen reader support

use std::env;

/// Check if a screen reader is likely enabled
///
/// This checks common environment variables that indicate
/// accessibility tools are in use.
fn detect_screen_reader() -> bool {
    // Check common accessibility environment variables
    let indicators = [
        "SCREEN_READER",
        "ACCESSIBILITY_ENABLED",
        "ORCA_ENABLED",      // Linux Orca
        "NVDA_RUNNING",      // Windows NVDA
        "JAWS_RUNNING",      // Windows JAWS
        "VOICEOVER_RUNNING", // macOS VoiceOver
        "TERM_PROGRAM",      // May indicate accessible terminal
    ];

    for var in indicators {
        if let Ok(val) = env::var(var) {
            if var == "TERM_PROGRAM" {
                // Some terminals have built-in accessibility
                if val.to_lowercase().contains("accessibility") {
                    return true;
                }
            } else if !val.is_empty() && val != "0" && val.to_lowercase() != "false" {
                return true;
            }
        }
    }

    // Check if running in a known accessible terminal
    if let Ok(term) = env::var("TERM")
        && (term.contains("screen") || term.contains("tmux"))
    {
        // These often have accessibility features
        // but we can't be certain, so we don't return true
    }

    // Check macOS VoiceOver via defaults (if available)
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "com.apple.universalaccess", "voiceOverOnOffKey"])
            .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "1" {
                return true;
            }
        }
    }

    false
}

/// Hook to check if a screen reader is enabled
///
/// Returns true if accessibility tools are detected.
/// When a RuntimeContext is available, the result is cached there.
/// Otherwise falls back to direct detection.
///
/// # Example
///
/// ```ignore
/// let is_accessible = use_is_screen_reader_enabled();
///
/// if is_accessible {
///     // Provide more detailed text descriptions
///     // Avoid relying solely on colors
/// }
/// ```
pub fn use_is_screen_reader_enabled() -> bool {
    if let Some(ctx) = crate::runtime::current_runtime() {
        if ctx.borrow().is_screen_reader_initialized() {
            return ctx.borrow().is_screen_reader_enabled();
        }

        let detected = detect_screen_reader();
        ctx.borrow_mut().set_screen_reader_enabled(detected);
        detected
    } else {
        detect_screen_reader()
    }
}

/// Manually set screen reader status (for testing or override)
pub fn set_screen_reader_enabled(enabled: bool) {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().set_screen_reader_enabled(enabled);
    }
}

/// Clear cached screen reader status (forces re-detection on next call)
pub fn clear_screen_reader_cache() {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut()
            .set_screen_reader_enabled(detect_screen_reader());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeContext, set_current_runtime};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_screen_reader_detection() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(ctx.clone()));

        // Should return false in normal test environment
        let result = use_is_screen_reader_enabled();
        let _ = result;

        set_current_runtime(None);
    }

    #[test]
    fn test_manual_override() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(ctx.clone()));

        set_screen_reader_enabled(true);
        assert!(use_is_screen_reader_enabled());

        set_screen_reader_enabled(false);
        assert!(!use_is_screen_reader_enabled());

        set_current_runtime(None);
    }

    #[test]
    fn test_runtime_auto_initializes_on_first_read() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(ctx.clone()));

        assert!(!ctx.borrow().is_screen_reader_initialized());
        let _ = use_is_screen_reader_enabled();
        assert!(ctx.borrow().is_screen_reader_initialized());

        set_current_runtime(None);
    }

    #[test]
    fn test_caching() {
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        set_current_runtime(Some(ctx.clone()));

        clear_screen_reader_cache();

        let first = use_is_screen_reader_enabled();
        let second = use_is_screen_reader_enabled();

        assert_eq!(first, second);

        set_current_runtime(None);
    }

    #[test]
    fn test_without_runtime_falls_back() {
        set_current_runtime(None);
        // Should not panic, falls back to detect_screen_reader()
        let _ = use_is_screen_reader_enabled();
    }
}
