//! Terminal hyperlink support
//!
//! Provides clickable hyperlinks in supported terminals using OSC 8 escape sequences.
//! Falls back gracefully in unsupported terminals.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for hyperlink support detection
static HYPERLINKS_SUPPORTED: AtomicBool = AtomicBool::new(true);
static HYPERLINKS_CHECKED: AtomicBool = AtomicBool::new(false);

/// Check if the terminal supports hyperlinks
pub fn supports_hyperlinks() -> bool {
    if !HYPERLINKS_CHECKED.load(Ordering::SeqCst) {
        let supported = detect_hyperlink_support();
        HYPERLINKS_SUPPORTED.store(supported, Ordering::SeqCst);
        HYPERLINKS_CHECKED.store(true, Ordering::SeqCst);
    }
    HYPERLINKS_SUPPORTED.load(Ordering::SeqCst)
}

/// Force enable/disable hyperlink support
pub fn set_hyperlinks_supported(supported: bool) {
    HYPERLINKS_SUPPORTED.store(supported, Ordering::SeqCst);
    HYPERLINKS_CHECKED.store(true, Ordering::SeqCst);
}

/// Detect if the terminal supports hyperlinks
fn detect_hyperlink_support() -> bool {
    // Check TERM_PROGRAM for known supporting terminals
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        let term_lower = term_program.to_lowercase();
        if term_lower.contains("iterm")
            || term_lower.contains("hyper")
            || term_lower.contains("wezterm")
            || term_lower.contains("kitty")
            || term_lower.contains("alacritty")
        {
            return true;
        }
    }

    // Check for Windows Terminal
    if std::env::var("WT_SESSION").is_ok() {
        return true;
    }

    // Check VTE version (GNOME Terminal, etc.)
    if let Ok(vte_version) = std::env::var("VTE_VERSION") {
        if let Ok(version) = vte_version.parse::<u32>() {
            // VTE 0.50.0 and later support hyperlinks
            if version >= 5000 {
                return true;
            }
        }
    }

    // Check COLORTERM for modern terminal indicators
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        if colorterm == "truecolor" || colorterm == "24bit" {
            // Modern terminals with truecolor often support hyperlinks
            return true;
        }
    }

    // Check for Konsole
    if std::env::var("KONSOLE_VERSION").is_ok() {
        return true;
    }

    // Default to false for unknown terminals
    false
}

/// A terminal hyperlink
#[derive(Debug, Clone)]
pub struct Hyperlink {
    /// The URL to link to
    url: String,
    /// The display text
    text: String,
    /// Optional ID for the link (for grouping)
    id: Option<String>,
}

impl Hyperlink {
    /// Create a new hyperlink
    pub fn new(url: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            text: text.into(),
            id: None,
        }
    }

    /// Create a hyperlink where the URL is also the display text
    pub fn url(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            text: url.clone(),
            url,
            id: None,
        }
    }

    /// Set an ID for the link (useful for multi-line links)
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Get the URL
    pub fn get_url(&self) -> &str {
        &self.url
    }

    /// Get the display text
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Render the hyperlink as an ANSI escape sequence
    ///
    /// If hyperlinks are not supported, returns just the text.
    pub fn render(&self) -> String {
        if supports_hyperlinks() {
            self.render_osc8()
        } else {
            self.render_fallback()
        }
    }

    /// Force render as OSC 8 hyperlink (ignoring detection)
    pub fn render_osc8(&self) -> String {
        let id_param = match &self.id {
            Some(id) => format!("id={}", id),
            None => String::new(),
        };

        // OSC 8 ; params ; URI ST text OSC 8 ; ; ST
        format!(
            "\x1b]8;{};{}\x1b\\{}\x1b]8;;\x1b\\",
            id_param, self.url, self.text
        )
    }

    /// Render fallback (just text, or text with URL)
    pub fn render_fallback(&self) -> String {
        if self.text == self.url {
            self.text.clone()
        } else {
            format!("{} ({})", self.text, self.url)
        }
    }

    /// Render with custom fallback format
    pub fn render_with_fallback<F>(&self, fallback: F) -> String
    where
        F: FnOnce(&str, &str) -> String,
    {
        if supports_hyperlinks() {
            self.render_osc8()
        } else {
            fallback(&self.text, &self.url)
        }
    }
}

/// Builder for creating styled hyperlinks
#[derive(Debug, Clone)]
pub struct HyperlinkBuilder {
    hyperlink: Hyperlink,
    color: Option<crate::core::Color>,
    underline: bool,
    bold: bool,
}

impl HyperlinkBuilder {
    /// Create a new hyperlink builder
    pub fn new(url: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            hyperlink: Hyperlink::new(url, text),
            color: Some(crate::core::Color::Blue),
            underline: true,
            bold: false,
        }
    }

    /// Set the link ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.hyperlink.id = Some(id.into());
        self
    }

    /// Set the color
    pub fn color(mut self, color: crate::core::Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Remove color
    pub fn no_color(mut self) -> Self {
        self.color = None;
        self
    }

    /// Set underline
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Set bold
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Render the styled hyperlink
    pub fn render(&self) -> String {
        let mut result = String::new();

        // Apply styles
        if self.bold {
            result.push_str("\x1b[1m");
        }
        if self.underline {
            result.push_str("\x1b[4m");
        }
        if let Some(color) = &self.color {
            result.push_str(&color.to_ansi_fg());
        }

        // Add the hyperlink
        result.push_str(&self.hyperlink.render());

        // Reset styles
        result.push_str("\x1b[0m");

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_hyperlink_creation() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::new("https://example.com", "Example");
        assert_eq!(link.get_url(), "https://example.com");
        assert_eq!(link.get_text(), "Example");
    }

    #[test]
    fn test_hyperlink_url() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::url("https://example.com");
        assert_eq!(link.get_url(), "https://example.com");
        assert_eq!(link.get_text(), "https://example.com");
    }

    #[test]
    fn test_hyperlink_with_id() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::new("https://example.com", "Example").with_id("link1");
        assert_eq!(link.id, Some("link1".to_string()));
    }

    #[test]
    fn test_hyperlink_render_osc8() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::new("https://example.com", "Example");
        let rendered = link.render_osc8();

        assert!(rendered.contains("\x1b]8;"));
        assert!(rendered.contains("https://example.com"));
        assert!(rendered.contains("Example"));
        assert!(rendered.contains("\x1b]8;;\x1b\\"));
    }

    #[test]
    fn test_hyperlink_render_osc8_with_id() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::new("https://example.com", "Example").with_id("mylink");
        let rendered = link.render_osc8();

        assert!(rendered.contains("id=mylink"));
    }

    #[test]
    fn test_hyperlink_render_fallback() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::new("https://example.com", "Example");
        let fallback = link.render_fallback();

        assert_eq!(fallback, "Example (https://example.com)");
    }

    #[test]
    fn test_hyperlink_render_fallback_same_text() {
        let _guard = test_lock().lock().unwrap();
        let link = Hyperlink::url("https://example.com");
        let fallback = link.render_fallback();

        assert_eq!(fallback, "https://example.com");
    }

    #[test]
    fn test_hyperlink_builder() {
        let _guard = test_lock().lock().unwrap();
        let builder = HyperlinkBuilder::new("https://example.com", "Example")
            .color(crate::core::Color::Cyan)
            .underline(true)
            .bold(true);

        let rendered = builder.render();
        assert!(rendered.contains("\x1b[1m")); // Bold
        assert!(rendered.contains("\x1b[4m")); // Underline
        assert!(rendered.contains("\x1b[0m")); // Reset
    }

    #[test]
    fn test_hyperlink_builder_no_style() {
        let _guard = test_lock().lock().unwrap();
        let builder = HyperlinkBuilder::new("https://example.com", "Example")
            .no_color()
            .underline(false)
            .bold(false);

        let rendered = builder.render();
        // Should still have reset at end
        assert!(rendered.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_set_hyperlinks_supported() {
        let _guard = test_lock().lock().unwrap();
        set_hyperlinks_supported(true);
        assert!(supports_hyperlinks());

        set_hyperlinks_supported(false);
        assert!(!supports_hyperlinks());

        // Reset for other tests
        HYPERLINKS_CHECKED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_render_with_fallback() {
        let _guard = test_lock().lock().unwrap();
        set_hyperlinks_supported(false);

        let link = Hyperlink::new("https://example.com", "Example");
        let result = link.render_with_fallback(|text, url| format!("[{}]({})", text, url));

        assert_eq!(result, "[Example](https://example.com)");

        // Reset
        HYPERLINKS_CHECKED.store(false, Ordering::SeqCst);
    }
}
