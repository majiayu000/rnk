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
        "ORCA_ENABLED",           // Linux Orca
        "NVDA_RUNNING",           // Windows NVDA
        "JAWS_RUNNING",           // Windows JAWS
        "VOICEOVER_RUNNING",      // macOS VoiceOver
        "TERM_PROGRAM",           // May indicate accessible terminal
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
    if let Ok(term) = env::var("TERM") {
        if term.contains("screen") || term.contains("tmux") {
            // These often have accessibility features
            // but we can't be certain, so we don't return true
        }
    }

    // Check macOS VoiceOver via defaults (if available)
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "com.apple.universalaccess", "voiceOverOnOffKey"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim() == "1" {
                    return true;
                }
            }
        }
    }

    false
}

// Thread-local cache for screen reader status
thread_local! {
    static SCREEN_READER_ENABLED: std::cell::Cell<Option<bool>> = std::cell::Cell::new(None);
}

/// Hook to check if a screen reader is enabled
///
/// Returns true if accessibility tools are detected.
/// The result is cached for performance.
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
    SCREEN_READER_ENABLED.with(|cached| {
        if let Some(value) = cached.get() {
            value
        } else {
            let detected = detect_screen_reader();
            cached.set(Some(detected));
            detected
        }
    })
}

/// Manually set screen reader status (for testing or override)
pub fn set_screen_reader_enabled(enabled: bool) {
    SCREEN_READER_ENABLED.with(|cached| {
        cached.set(Some(enabled));
    });
}

/// Clear cached screen reader status (forces re-detection)
pub fn clear_screen_reader_cache() {
    SCREEN_READER_ENABLED.with(|cached| {
        cached.set(None);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_reader_detection() {
        // Clear cache first
        clear_screen_reader_cache();

        // Should return false in normal test environment
        let result = use_is_screen_reader_enabled();
        // Result depends on environment, just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_manual_override() {
        set_screen_reader_enabled(true);
        assert!(use_is_screen_reader_enabled());

        set_screen_reader_enabled(false);
        assert!(!use_is_screen_reader_enabled());

        // Clean up
        clear_screen_reader_cache();
    }

    #[test]
    fn test_caching() {
        clear_screen_reader_cache();

        // First call detects
        let first = use_is_screen_reader_enabled();
        // Second call uses cache
        let second = use_is_screen_reader_enabled();

        assert_eq!(first, second);
    }
}
