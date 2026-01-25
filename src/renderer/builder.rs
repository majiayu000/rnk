//! Application builder and options
//!
//! This module provides configuration types for the application runner.

use crate::core::Element;

use super::app::App;

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
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            fps: 60, // Bubbletea default
            exit_on_ctrl_c: true,
            alternate_screen: false, // Inline mode by default (like Ink/Bubbletea)
        }
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

    /// Get the current options
    pub fn options(&self) -> &AppOptions {
        &self.options
    }

    /// Run the application
    pub fn run(self) -> std::io::Result<()> {
        App::with_options(self.component, self.options).run()
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
}
