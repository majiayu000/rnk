//! Application builder and options
//!
//! This module provides configuration types for the application runner.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::Element;

use super::app::App;
use super::filter::{EventFilter, FilterChain, FilterResult};
use super::frame_rate::FrameRateConfig;

/// A token for cancelling the application from external code.
///
/// This allows external code (e.g., another thread, signal handler, or
/// async task) to request that the application exit gracefully.
///
/// # Example
///
/// ```rust
/// use rnk::renderer::CancelToken;
/// use std::thread;
/// use std::time::Duration;
///
/// let token = CancelToken::new();
/// let token_clone = token.clone();
///
/// // Spawn a thread that will cancel after 5 seconds
/// thread::spawn(move || {
///     thread::sleep(Duration::from_secs(5));
///     token_clone.cancel();
/// });
///
/// // The app will exit when token.cancel() is called
/// // render(my_app).with_cancel_token(token).run()?;
/// ```
#[derive(Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    /// Create a new cancel token
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel the application
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Get the internal flag (for integration with App)
    pub(crate) fn flag(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}

/// Application options for configuring the renderer
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// Target frames per second (default: 60)
    pub fps: u32,
    /// Exit on Ctrl+C (default: true)
    pub exit_on_ctrl_c: bool,
    /// Use alternate screen / fullscreen mode (default: false = inline mode)
    ///
    /// - `false` (default): Inline mode, like Ink and Bubbletea's default.
    ///   Output appears at current cursor position and persists in terminal history.
    ///
    /// - `true`: Fullscreen mode, like vim or Bubbletea's `WithAltScreen()`.
    ///   Uses alternate screen buffer, content is cleared on exit.
    pub alternate_screen: bool,
    /// Enable adaptive frame rate (default: false)
    pub adaptive_fps: bool,
    /// Minimum FPS when adaptive is enabled (default: 10)
    pub min_fps: u32,
    /// Maximum FPS when adaptive is enabled (default: 120)
    pub max_fps: u32,
    /// Collect frame rate statistics (default: false)
    pub collect_frame_stats: bool,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            fps: 60, // Bubbletea default
            exit_on_ctrl_c: true,
            alternate_screen: false, // Inline mode by default (like Ink/Bubbletea)
            adaptive_fps: false,
            min_fps: 10,
            max_fps: 120,
            collect_frame_stats: false,
        }
    }
}

impl AppOptions {
    /// Convert to FrameRateConfig
    pub fn to_frame_rate_config(&self) -> FrameRateConfig {
        let mut config = FrameRateConfig::new(self.fps);
        if self.adaptive_fps {
            config = config.adaptive(self.min_fps, self.max_fps);
        }
        if self.collect_frame_stats {
            config = config.with_stats();
        }
        config
    }
}

/// Builder for configuring and running an application.
///
/// This provides a fluent API similar to Bubbletea's `WithXxx()` options.
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// // Inline mode (default)
/// render(my_app).run()?;
///
/// // Fullscreen mode
/// render(my_app).fullscreen().run()?;
///
/// // Custom configuration
/// render(my_app)
///     .fullscreen()
///     .fps(30)
///     .exit_on_ctrl_c(false)
///     .run()?;
/// ```
pub struct AppBuilder<F>
where
    F: Fn() -> Element,
{
    component: F,
    options: AppOptions,
    filter_chain: FilterChain,
    cancel_token: Option<CancelToken>,
}

impl<F> AppBuilder<F>
where
    F: Fn() -> Element,
{
    /// Create a new app builder with default options (inline mode)
    pub fn new(component: F) -> Self {
        Self {
            component,
            options: AppOptions::default(),
            filter_chain: FilterChain::new(),
            cancel_token: None,
        }
    }

    /// Use fullscreen mode (alternate screen buffer).
    ///
    /// Like Bubbletea's `WithAltScreen()`.
    pub fn fullscreen(mut self) -> Self {
        self.options.alternate_screen = true;
        self
    }

    /// Use inline mode (default).
    ///
    /// Output appears at current cursor position and persists in terminal history.
    pub fn inline(mut self) -> Self {
        self.options.alternate_screen = false;
        self
    }

    /// Set the target frames per second.
    ///
    /// Default is 60 FPS.
    pub fn fps(mut self, fps: u32) -> Self {
        self.options.fps = fps;
        self
    }

    /// Set whether to exit on Ctrl+C.
    ///
    /// Default is `true`.
    pub fn exit_on_ctrl_c(mut self, exit: bool) -> Self {
        self.options.exit_on_ctrl_c = exit;
        self
    }

    /// Enable adaptive frame rate.
    ///
    /// When enabled, the frame rate will automatically adjust based on
    /// render performance. If rendering takes too long, FPS will decrease.
    /// If rendering is fast, FPS will increase back toward the target.
    ///
    /// # Arguments
    ///
    /// * `min_fps` - Minimum FPS (default: 10)
    /// * `max_fps` - Maximum FPS (default: 120)
    ///
    /// # Example
    ///
    /// ```ignore
    /// render(my_app)
    ///     .fps(60)
    ///     .adaptive_fps(15, 90)
    ///     .run()?;
    /// ```
    pub fn adaptive_fps(mut self, min_fps: u32, max_fps: u32) -> Self {
        self.options.adaptive_fps = true;
        self.options.min_fps = min_fps.clamp(1, 120);
        self.options.max_fps = max_fps.clamp(self.options.min_fps, 120);
        self
    }

    /// Enable frame rate statistics collection.
    ///
    /// When enabled, frame rate statistics can be accessed via the
    /// `use_frame_rate()` hook.
    ///
    /// # Example
    ///
    /// ```ignore
    /// render(my_app)
    ///     .collect_frame_stats()
    ///     .run()?;
    /// ```
    pub fn collect_frame_stats(mut self) -> Self {
        self.options.collect_frame_stats = true;
        self
    }

    /// Add an event filter to the filter chain.
    ///
    /// Filters are applied in priority order (higher priority first).
    /// Each filter can pass, replace, or block events.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rnk::prelude::*;
    /// use rnk::renderer::FilterResult;
    /// use crossterm::event::{Event, KeyCode};
    ///
    /// render(my_app)
    ///     .with_filter("block-escape", |event| {
    ///         // Block Escape key
    ///         if let Event::Key(key) = &event {
    ///             if key.code == KeyCode::Esc {
    ///                 return FilterResult::Block;
    ///             }
    ///         }
    ///         FilterResult::Pass(event)
    ///     })
    ///     .run()?;
    /// ```
    pub fn with_filter<G>(mut self, name: &str, filter: G) -> Self
    where
        G: Fn(crossterm::event::Event) -> FilterResult + Send + Sync + 'static,
    {
        self.filter_chain.add(EventFilter::new(name, filter));
        self
    }

    /// Add an event filter with priority.
    ///
    /// Higher priority filters run first.
    ///
    /// # Example
    ///
    /// ```ignore
    /// render(my_app)
    ///     .with_filter_priority("high-priority", 100, |event| {
    ///         FilterResult::Pass(event)
    ///     })
    ///     .with_filter_priority("low-priority", 0, |event| {
    ///         FilterResult::Pass(event)
    ///     })
    ///     .run()?;
    /// ```
    pub fn with_filter_priority<G>(mut self, name: &str, priority: i32, filter: G) -> Self
    where
        G: Fn(crossterm::event::Event) -> FilterResult + Send + Sync + 'static,
    {
        self.filter_chain
            .add(EventFilter::with_priority(name, priority, filter));
        self
    }

    /// Set a cancel token for external cancellation.
    ///
    /// This allows external code to cancel the application by calling
    /// `token.cancel()`. The application will exit gracefully on the
    /// next event loop iteration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rnk::prelude::*;
    /// use rnk::renderer::CancelToken;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let token = CancelToken::new();
    /// let token_clone = token.clone();
    ///
    /// // Cancel after 10 seconds
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(10));
    ///     token_clone.cancel();
    /// });
    ///
    /// render(my_app)
    ///     .with_cancel_token(token)
    ///     .run()?;
    /// ```
    pub fn with_cancel_token(mut self, token: CancelToken) -> Self {
        self.cancel_token = Some(token);
        self
    }

    /// Get the current options
    pub fn options(&self) -> &AppOptions {
        &self.options
    }

    /// Run the application
    pub fn run(self) -> std::io::Result<()> {
        App::with_full_config(
            self.component,
            self.options,
            self.filter_chain,
            self.cancel_token,
        )
        .run()
    }
}

/// Create an app builder for configuring and running a component.
///
/// This is the main entry point for running an rnk application.
/// Returns an `AppBuilder` that allows fluent configuration.
///
/// # Default Behavior
///
/// By default, the app runs in **inline mode** (like Ink and Bubbletea):
/// - Output appears at the current cursor position
/// - Content persists in terminal history after exit
/// - Supports `println()` for persistent messages
///
/// # Examples
///
/// ```ignore
/// use rnk::prelude::*;
///
/// // Inline mode (default)
/// render(my_app).run()?;
///
/// // Fullscreen mode
/// render(my_app).fullscreen().run()?;
///
/// // Custom configuration
/// render(my_app)
///     .fullscreen()
///     .fps(30)
///     .exit_on_ctrl_c(false)
///     .run()?;
/// ```
pub fn render<F>(component: F) -> AppBuilder<F>
where
    F: Fn() -> Element,
{
    AppBuilder::new(component)
}

/// Run a component in inline mode (convenience function).
///
/// This is equivalent to `render(component).run()`.
pub fn render_inline<F>(component: F) -> std::io::Result<()>
where
    F: Fn() -> Element,
{
    render(component).inline().run()
}

/// Run a component in fullscreen mode (convenience function).
///
/// This is equivalent to `render(component).fullscreen().run()`.
pub fn render_fullscreen<F>(component: F) -> std::io::Result<()>
where
    F: Fn() -> Element,
{
    render(component).fullscreen().run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;

    #[test]
    fn test_app_options_default() {
        let options = AppOptions::default();
        assert_eq!(options.fps, 60);
        assert!(options.exit_on_ctrl_c);
        assert!(!options.alternate_screen);
    }

    #[test]
    fn test_app_builder_defaults() {
        fn dummy() -> Element {
            Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy);
        assert!(!builder.options().alternate_screen);
        assert_eq!(builder.options().fps, 60);
    }

    #[test]
    fn test_app_builder_fullscreen() {
        fn dummy() -> Element {
            Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy).fullscreen();
        assert!(builder.options().alternate_screen);
    }

    #[test]
    fn test_app_builder_inline() {
        fn dummy() -> Element {
            Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy).fullscreen().inline();
        assert!(!builder.options().alternate_screen);
    }

    #[test]
    fn test_app_builder_fps() {
        fn dummy() -> Element {
            Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy).fps(30);
        assert_eq!(builder.options().fps, 30);
    }

    #[test]
    fn test_cancel_token_creation() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancel_token_cancel() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancel_token_clone() {
        let token = CancelToken::new();
        let token2 = token.clone();

        assert!(!token.is_cancelled());
        assert!(!token2.is_cancelled());

        token.cancel();

        assert!(token.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_cancel_token_default() {
        let token = CancelToken::default();
        assert!(!token.is_cancelled());
    }
}
