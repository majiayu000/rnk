//! Application runner
//!
//! This module provides the main application runner with support for:
//! - Inline mode (default, like Ink/Bubbletea)
//! - Fullscreen mode (alternate screen, like vim)
//! - Runtime mode switching
//! - Println for persistent output
//! - Cross-thread render requests

use crossterm::event::Event;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::Element;
use crate::hooks::context::{HookContext, with_hooks};
use crate::hooks::use_app::{AppContext, set_app_context};
use crate::hooks::use_input::{clear_input_handlers, dispatch_key_event};
use crate::hooks::use_mouse::{clear_mouse_handlers, dispatch_mouse_event, is_mouse_enabled};
use crate::layout::LayoutEngine;
use crate::renderer::{Output, Terminal};

// === Global state for cross-thread communication ===

/// Global render flag storage for cross-thread render requests
static GLOBAL_RENDER_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

/// Printable content that can be sent to println
#[derive(Clone)]
pub enum Printable {
    /// Plain text message
    Text(String),
    /// Rendered element (boxed to reduce enum size)
    Element(Box<Element>),
}

/// Trait for types that can be printed via println
pub trait IntoPrintable {
    /// Convert into a printable value
    fn into_printable(self) -> Printable;
}

impl IntoPrintable for String {
    fn into_printable(self) -> Printable {
        Printable::Text(self)
    }
}

impl IntoPrintable for &str {
    fn into_printable(self) -> Printable {
        Printable::Text(self.to_string())
    }
}

impl IntoPrintable for Element {
    fn into_printable(self) -> Printable {
        Printable::Element(Box::new(self))
    }
}

/// Global println message queue
static PRINTLN_QUEUE: std::sync::OnceLock<Mutex<Vec<Printable>>> = std::sync::OnceLock::new();

/// Global mode switch request
static MODE_SWITCH_REQUEST: std::sync::OnceLock<Mutex<Option<ModeSwitch>>> =
    std::sync::OnceLock::new();

/// Mode switch direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeSwitch {
    /// Switch to alternate screen (fullscreen)
    EnterAltScreen,
    /// Switch to inline mode
    ExitAltScreen,
}

/// Initialize global state (called by App::run)
fn init_global_state(render_flag: Arc<AtomicBool>) {
    let _ = GLOBAL_RENDER_FLAG.set(render_flag);
    let _ = PRINTLN_QUEUE.set(Mutex::new(Vec::new()));
    let _ = MODE_SWITCH_REQUEST.set(Mutex::new(None));
}

/// Request a re-render from any thread.
///
/// This is useful when state is updated from a background thread
/// and you need to notify the UI to refresh.
///
/// # Example
///
/// ```ignore
/// use std::thread;
/// use rnk::request_render;
///
/// // In a background thread
/// thread::spawn(|| {
///     // ... update some shared state ...
///     request_render(); // Notify rnk to re-render
/// });
/// ```
pub fn request_render() {
    if let Some(flag) = GLOBAL_RENDER_FLAG.get() {
        flag.store(true, Ordering::SeqCst);
    }
}

/// Check if a render has been requested and reset the flag.
fn take_render_request() -> bool {
    if let Some(flag) = GLOBAL_RENDER_FLAG.get() {
        flag.swap(false, Ordering::SeqCst)
    } else {
        false
    }
}

/// Print a message that persists above the UI (like Bubbletea's Println).
///
/// In inline mode, this clears the current UI, writes the message,
/// and the UI is re-rendered below it. The message stays in terminal history.
///
/// In fullscreen mode, this is a no-op (messages are ignored, like Bubbletea).
///
/// Supports both plain text and rendered elements:
///
/// # Examples
///
/// ```ignore
/// use rnk::println;
///
/// // Simple text
/// rnk::println("Task completed successfully!");
/// rnk::println(format!("Downloaded {} files", count));
///
/// // Complex components
/// let banner = Box::new()
///     .border_style(BorderStyle::Round)
///     .padding(1)
///     .child(Text::new("Welcome!").bold().into_element())
///     .into_element();
/// rnk::println(banner);
/// ```
pub fn println(message: impl IntoPrintable) {
    if let Some(queue) = PRINTLN_QUEUE.get() {
        if let Ok(mut q) = queue.lock() {
            q.push(message.into_printable());
        }
    }
    request_render();
}

/// Take all queued println messages
fn take_println_messages() -> Vec<Printable> {
    if let Some(queue) = PRINTLN_QUEUE.get() {
        if let Ok(mut q) = queue.lock() {
            std::mem::take(&mut *q)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Request to enter alternate screen mode (fullscreen).
///
/// This can be called from any thread. The mode switch happens
/// on the next render cycle.
///
/// # Example
///
/// ```ignore
/// use rnk::enter_alt_screen;
///
/// // Switch to fullscreen mode
/// enter_alt_screen();
/// ```
pub fn enter_alt_screen() {
    if let Some(req) = MODE_SWITCH_REQUEST.get() {
        if let Ok(mut r) = req.lock() {
            *r = Some(ModeSwitch::EnterAltScreen);
        }
    }
    request_render();
}

/// Request to exit alternate screen mode (return to inline).
///
/// This can be called from any thread. The mode switch happens
/// on the next render cycle.
///
/// # Example
///
/// ```ignore
/// use rnk::exit_alt_screen;
///
/// // Return to inline mode
/// exit_alt_screen();
/// ```
pub fn exit_alt_screen() {
    if let Some(req) = MODE_SWITCH_REQUEST.get() {
        if let Ok(mut r) = req.lock() {
            *r = Some(ModeSwitch::ExitAltScreen);
        }
    }
    request_render();
}

/// Take mode switch request if any
fn take_mode_switch_request() -> Option<ModeSwitch> {
    if let Some(req) = MODE_SWITCH_REQUEST.get() {
        if let Ok(mut r) = req.lock() {
            r.take()
        } else {
            None
        }
    } else {
        None
    }
}

/// Check if currently in alternate screen mode.
///
/// Returns `None` if no app is running.
pub fn is_alt_screen() -> Option<bool> {
    // This is set during render, so we need a global flag
    ALT_SCREEN_STATE
        .get()
        .map(|flag| flag.load(Ordering::SeqCst))
}

/// Global alt screen state flag
static ALT_SCREEN_STATE: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

/// A handle for requesting renders from any thread.
///
/// This is a cloneable, thread-safe handle that can be used to trigger
/// re-renders from background threads or async tasks.
///
/// # Example
///
/// ```ignore
/// use std::thread;
/// use rnk::render_handle;
///
/// let handle = render_handle().expect("App must be running");
///
/// thread::spawn(move || {
///     // ... do some work ...
///     handle.request_render();
/// });
/// ```
#[derive(Clone)]
pub struct RenderHandle {
    flag: Arc<AtomicBool>,
}

impl RenderHandle {
    /// Request a re-render
    pub fn request_render(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Print a message that persists above the UI
    pub fn println(&self, message: impl IntoPrintable) {
        println(message);
    }

    /// Request to enter fullscreen mode
    pub fn enter_alt_screen(&self) {
        enter_alt_screen();
    }

    /// Request to exit fullscreen mode
    pub fn exit_alt_screen(&self) {
        exit_alt_screen();
    }

    /// Check if currently in fullscreen mode
    pub fn is_alt_screen(&self) -> bool {
        is_alt_screen().unwrap_or(false)
    }
}

/// Get a render handle for cross-thread render requests.
///
/// Returns `None` if no app is currently running.
pub fn render_handle() -> Option<RenderHandle> {
    GLOBAL_RENDER_FLAG.get().map(|flag| RenderHandle {
        flag: Arc::clone(flag),
    })
}

// === Application Options ===

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

// === App Builder ===

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

// === Application State ===

/// Application state
pub struct App<F>
where
    F: Fn() -> Element,
{
    component: F,
    terminal: Terminal,
    layout_engine: LayoutEngine,
    hook_context: Rc<RefCell<HookContext>>,
    options: AppOptions,
    should_exit: Arc<AtomicBool>,
    needs_render: Arc<AtomicBool>,
    alt_screen_state: Arc<AtomicBool>,
    /// Lines of static content that have been committed
    static_lines: Vec<String>,
    /// Last known terminal width (for detecting width decreases)
    last_width: u16,
    /// Last known terminal height
    last_height: u16,
}

impl<F> App<F>
where
    F: Fn() -> Element,
{
    /// Create a new app with default options (inline mode)
    pub fn new(component: F) -> Self {
        Self::with_options(component, AppOptions::default())
    }

    /// Create a new app with custom options
    pub fn with_options(component: F, options: AppOptions) -> Self {
        let needs_render = Arc::new(AtomicBool::new(true));
        let alt_screen_state = Arc::new(AtomicBool::new(options.alternate_screen));
        let hook_context = Rc::new(RefCell::new(HookContext::new()));

        // Set up render callback
        let needs_render_clone = needs_render.clone();
        hook_context
            .borrow_mut()
            .set_render_callback(Rc::new(move || {
                needs_render_clone.store(true, Ordering::SeqCst);
            }));

        // Get initial terminal size
        let (initial_width, initial_height) = Terminal::size().unwrap_or((80, 24));

        Self {
            component,
            terminal: Terminal::new(),
            layout_engine: LayoutEngine::new(),
            hook_context,
            options,
            should_exit: Arc::new(AtomicBool::new(false)),
            needs_render,
            alt_screen_state,
            static_lines: Vec::new(),
            last_width: initial_width,
            last_height: initial_height,
        }
    }

    /// Run the application
    pub fn run(&mut self) -> std::io::Result<()> {
        use std::time::{Duration, Instant};

        // Initialize global state for cross-thread communication
        init_global_state(Arc::clone(&self.needs_render));
        let _ = ALT_SCREEN_STATE.set(Arc::clone(&self.alt_screen_state));

        // Enter terminal mode based on options
        if self.options.alternate_screen {
            self.terminal.enter()?;
            self.alt_screen_state.store(true, Ordering::SeqCst);
        } else {
            self.terminal.enter_inline()?;
            self.alt_screen_state.store(false, Ordering::SeqCst);
        }

        let frame_duration = Duration::from_millis(1000 / self.options.fps as u64);
        let mut last_render = Instant::now();

        // Initial render
        self.render_frame()?;

        loop {
            // Handle input
            if let Some(event) = Terminal::poll_event(Duration::from_millis(10))? {
                self.handle_event(event);
            }

            if self.should_exit.load(Ordering::SeqCst) {
                break;
            }

            // Check for external render requests (from other threads)
            if take_render_request() {
                self.needs_render.store(true, Ordering::SeqCst);
            }

            // Handle mode switch requests
            if let Some(mode_switch) = take_mode_switch_request() {
                self.handle_mode_switch(mode_switch)?;
            }

            // Handle println messages
            let messages = take_println_messages();
            if !messages.is_empty() {
                self.handle_println_messages(&messages)?;
            }

            // Throttle rendering - only render if needed and time elapsed
            let now = Instant::now();
            let time_elapsed = now.duration_since(last_render) >= frame_duration;
            let render_requested = self.needs_render.load(Ordering::SeqCst);

            if render_requested && time_elapsed {
                self.needs_render.store(false, Ordering::SeqCst);
                self.render_frame()?;
                last_render = now;
            }
        }

        // Clean up input handlers
        clear_input_handlers();

        // Exit terminal mode
        if self.terminal.is_alt_screen() {
            self.terminal.exit()?;
        } else {
            self.terminal.exit_inline()?;
        }

        Ok(())
    }

    /// Handle mode switch request
    fn handle_mode_switch(&mut self, mode_switch: ModeSwitch) -> std::io::Result<()> {
        match mode_switch {
            ModeSwitch::EnterAltScreen => {
                if !self.terminal.is_alt_screen() {
                    self.terminal.switch_to_alt_screen()?;
                    self.alt_screen_state.store(true, Ordering::SeqCst);
                    self.terminal.repaint();
                }
            }
            ModeSwitch::ExitAltScreen => {
                if self.terminal.is_alt_screen() {
                    self.terminal.switch_to_inline()?;
                    self.alt_screen_state.store(false, Ordering::SeqCst);
                    self.terminal.repaint();
                }
            }
        }
        Ok(())
    }

    /// Handle println messages (like Bubbletea's Println)
    fn handle_println_messages(&mut self, messages: &[Printable]) -> std::io::Result<()> {
        // Println only works in inline mode
        if self.terminal.is_alt_screen() {
            return Ok(());
        }

        // Get terminal width for rendering elements
        let (width, _) = Terminal::size().unwrap_or((80, 24));

        for message in messages {
            match message {
                Printable::Text(text) => {
                    // Simple text - print directly
                    self.terminal.println(text)?;
                }
                Printable::Element(element) => {
                    // Render element to string first
                    let rendered = self.render_element_to_string(element, width);
                    self.terminal.println(&rendered)?;
                }
            }
        }

        // Force repaint after println
        self.terminal.repaint();

        Ok(())
    }

    /// Render an element to a string (for println)
    fn render_element_to_string(&self, element: &Element, width: u16) -> String {
        // Create a temporary layout engine
        let mut engine = LayoutEngine::new();

        // Calculate actual height considering text wrapping
        let height = self.calculate_element_height(element, width, &mut engine);

        // Compute layout
        engine.compute(element, width, height.max(1000));

        // Get layout dimensions
        let layout = engine.get_layout(element.id).unwrap_or_default();
        let content_width = (layout.width as u16).max(1).min(width);
        let content_height = height.max(1);

        // Render to output buffer
        let mut output = Output::new(content_width, content_height);
        self.render_element_to_output(element, &engine, &mut output, 0.0, 0.0);

        output.render()
    }

    /// Calculate the actual height needed for an element, considering text wrapping
    fn calculate_element_height(
        &self,
        element: &Element,
        max_width: u16,
        _engine: &mut LayoutEngine,
    ) -> u16 {
        use crate::layout::measure::wrap_text;

        let mut height = 1u16;

        // Calculate available width for text
        let available_width = if element.style.has_border() {
            max_width.saturating_sub(2)
        } else {
            max_width
        };
        let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
        let available_width = available_width.saturating_sub(padding_h).max(1);

        // Check for multiline spans with wrapping
        if let Some(lines) = &element.spans {
            let mut total_lines = 0usize;
            for line in lines {
                // Reconstruct the full line text and calculate wrapped height
                let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
                let wrapped = wrap_text(&line_text, available_width as usize);
                total_lines += wrapped.len();
            }
            height = height.max(total_lines as u16);
        }

        // Check text_content with wrapping
        if let Some(text) = &element.text_content {
            let wrapped = wrap_text(text, available_width as usize);
            height = height.max(wrapped.len() as u16);
        }

        // Add border height
        if element.style.has_border() {
            height = height.saturating_add(2);
        }

        // Add padding height
        let padding_v = (element.style.padding.top + element.style.padding.bottom) as u16;
        height = height.saturating_add(padding_v);

        // Recursively check children and accumulate height for column layout
        if !element.children.is_empty() {
            let mut child_height_sum = 0u16;
            for child in &element.children {
                let child_height = self.calculate_element_height(child, max_width, _engine);
                child_height_sum = child_height_sum.saturating_add(child_height);
            }
            height = height.max(child_height_sum);
        }

        height
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) => {
                // Handle Ctrl+C
                if self.options.exit_on_ctrl_c && Terminal::is_ctrl_c(&Event::Key(key_event)) {
                    self.should_exit.store(true, Ordering::SeqCst);
                    return;
                }

                // Dispatch to input handlers
                dispatch_key_event(&key_event);

                // Request re-render after input
                self.needs_render.store(true, Ordering::SeqCst);
            }
            Event::Mouse(mouse_event) => {
                // Dispatch to mouse handlers
                dispatch_mouse_event(&mouse_event);

                // Request re-render after mouse event
                self.needs_render.store(true, Ordering::SeqCst);
            }
            Event::Resize(new_width, new_height) => {
                // Handle resize - clear screen if width decreased to prevent artifacts
                self.handle_resize(new_width, new_height);
                // Re-render on resize
                self.needs_render.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    /// Handle terminal resize events
    ///
    /// When terminal width decreases, clears the screen to prevent rendering artifacts.
    /// This follows the same pattern as Ink and Bubbletea.
    fn handle_resize(&mut self, new_width: u16, new_height: u16) {
        use crossterm::execute;
        use crossterm::terminal::{Clear, ClearType};
        use std::io::stdout;

        // If width decreased, clear the screen to prevent artifacts
        if new_width < self.last_width {
            let _ = execute!(stdout(), Clear(ClearType::All));
            self.terminal.repaint();
        }

        // Update tracked size
        self.last_width = new_width;
        self.last_height = new_height;
    }

    fn render_frame(&mut self) -> std::io::Result<()> {
        // Clear input and mouse handlers before render (they'll be re-registered)
        clear_input_handlers();
        clear_mouse_handlers();

        // Get terminal size
        let (width, height) = Terminal::size()?;

        // Set up app context for use_app hook
        set_app_context(Some(AppContext::new(self.should_exit.clone())));

        // Build element tree with hooks context
        let root = with_hooks(self.hook_context.clone(), || (self.component)());

        // Clear app context after render
        set_app_context(None);

        // Enable/disable mouse mode based on whether any component uses it
        if is_mouse_enabled() {
            self.terminal.enable_mouse()?;
        } else {
            self.terminal.disable_mouse()?;
        }

        // Extract and commit static content
        let new_static_lines = self.extract_static_content(&root, width);
        if !new_static_lines.is_empty() {
            self.commit_static_content(&new_static_lines)?;
        }

        // Filter out static elements from the tree for dynamic rendering
        let dynamic_root = self.filter_static_elements(&root);

        // Compute layout for dynamic content
        self.layout_engine.compute(&dynamic_root, width, height);

        // Get the actual content size from layout
        let root_layout = self
            .layout_engine
            .get_layout(dynamic_root.id)
            .unwrap_or_default();
        let content_width = (root_layout.width as u16).max(1).min(width);
        let content_height = (root_layout.height as u16).max(1).min(height);

        // Render to output buffer sized to content
        let mut output = Output::new(content_width, content_height);
        self.render_element(&dynamic_root, &mut output, 0.0, 0.0);

        // Write to terminal
        self.terminal.render(&output.render())
    }

    /// Extract static content from the element tree
    fn extract_static_content(&self, element: &Element, width: u16) -> Vec<String> {
        let mut lines = Vec::new();

        if element.style.is_static {
            // Render static element to get its content
            let mut engine = LayoutEngine::new();
            engine.compute(element, width, 100); // Use large height for static content

            let layout = engine.get_layout(element.id).unwrap_or_default();
            let mut output = Output::new(layout.width as u16, layout.height as u16);
            self.render_element_to_output(element, &engine, &mut output, 0.0, 0.0);

            let rendered = output.render();
            for line in rendered.lines() {
                lines.push(line.to_string());
            }
        }

        // Check children for static content
        for child in &element.children {
            lines.extend(self.extract_static_content(child, width));
        }

        lines
    }

    /// Commit static content to the terminal (write permanently)
    fn commit_static_content(&mut self, new_lines: &[String]) -> std::io::Result<()> {
        use std::io::{Write, stdout};

        let mut stdout = stdout();
        for line in new_lines {
            // Write the line and move to next line
            writeln!(stdout, "{}", line)?;
            self.static_lines.push(line.clone());
        }
        stdout.flush()?;

        Ok(())
    }

    /// Filter out static elements from the tree
    fn filter_static_elements(&self, element: &Element) -> Element {
        let mut new_element = element.clone();

        // Remove static children
        new_element.children = element
            .children
            .iter()
            .filter(|child| !child.style.is_static)
            .map(|child| self.filter_static_elements(child))
            .collect();

        new_element
    }

    /// Render element to output buffer (helper for static content)
    fn render_element_to_output(
        &self,
        element: &Element,
        engine: &LayoutEngine,
        output: &mut Output,
        offset_x: f32,
        offset_y: f32,
    ) {
        // Skip elements with display: none
        if element.style.display == crate::core::Display::None {
            return;
        }

        let layout = engine.get_layout(element.id).unwrap_or_default();

        let x = (offset_x + layout.x) as u16;
        let y = (offset_y + layout.y) as u16;
        let width = layout.width as u16;
        let height = layout.height as u16;

        if element.style.background_color.is_some() {
            output.fill_rect(x, y, width, height, ' ', &element.style);
        }

        if element.style.has_border() {
            self.render_border(element, output, x, y, width, height);
        }

        if let Some(text) = &element.text_content {
            let text_x = x
                + if element.style.has_border() { 1 } else { 0 }
                + element.style.padding.left as u16;
            let text_y = y
                + if element.style.has_border() { 1 } else { 0 }
                + element.style.padding.top as u16;
            output.write(text_x, text_y, text, &element.style);
        }

        let child_offset_x = offset_x + layout.x;
        let child_offset_y = offset_y + layout.y;

        for child in &element.children {
            self.render_element_to_output(child, engine, output, child_offset_x, child_offset_y);
        }
    }

    fn render_element(&self, element: &Element, output: &mut Output, offset_x: f32, offset_y: f32) {
        // Skip elements with display: none
        if element.style.display == crate::core::Display::None {
            return;
        }

        // Get layout for this element
        let layout = self
            .layout_engine
            .get_layout(element.id)
            .unwrap_or_default();

        let x = (offset_x + layout.x) as u16;
        let y = (offset_y + layout.y) as u16;
        let width = layout.width as u16;
        let height = layout.height as u16;

        // Render background if set
        if element.style.background_color.is_some() {
            output.fill_rect(x, y, width, height, ' ', &element.style);
        }

        // Render border if set
        if element.style.has_border() {
            self.render_border(element, output, x, y, width, height);
        }

        // Render text content (simple or rich text with spans)
        let text_x =
            x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
        let text_y =
            y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;

        if let Some(spans) = &element.spans {
            // Rich text with multiple spans
            self.render_spans(spans, output, text_x, text_y);
        } else if let Some(text) = &element.text_content {
            // Simple text
            output.write(text_x, text_y, text, &element.style);
        }

        // Render children - Taffy already includes border/padding in child positions
        let child_offset_x = offset_x + layout.x;
        let child_offset_y = offset_y + layout.y;

        for child in &element.children {
            self.render_element(child, output, child_offset_x, child_offset_y);
        }
    }

    fn render_border(
        &self,
        element: &Element,
        output: &mut Output,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) {
        let (tl, tr, bl, br, h, v) = element.style.border_style.chars();

        // Create base style for borders
        let mut base_style = element.style.clone();
        base_style.dim = element.style.border_dim;

        // Create per-side styles with their respective colors
        let mut top_style = base_style.clone();
        top_style.color = element.style.get_border_top_color();

        let mut right_style = base_style.clone();
        right_style.color = element.style.get_border_right_color();

        let mut bottom_style = base_style.clone();
        bottom_style.color = element.style.get_border_bottom_color();

        let mut left_style = base_style.clone();
        left_style.color = element.style.get_border_left_color();

        // Top border
        if element.style.border_top && height > 0 {
            // Top-left corner uses left color if no top color, or top color
            output.write_char(x, y, tl.chars().next().unwrap(), &top_style);
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, y, h.chars().next().unwrap(), &top_style);
            }
            if width > 1 {
                // Top-right corner uses top color
                output.write_char(x + width - 1, y, tr.chars().next().unwrap(), &top_style);
            }
        }

        // Bottom border
        if element.style.border_bottom && height > 1 {
            let bottom_y = y + height - 1;
            output.write_char(x, bottom_y, bl.chars().next().unwrap(), &bottom_style);
            for col in (x + 1)..(x + width - 1) {
                output.write_char(col, bottom_y, h.chars().next().unwrap(), &bottom_style);
            }
            if width > 1 {
                output.write_char(
                    x + width - 1,
                    bottom_y,
                    br.chars().next().unwrap(),
                    &bottom_style,
                );
            }
        }

        // Left border
        if element.style.border_left {
            for row in (y + 1)..(y + height - 1) {
                output.write_char(x, row, v.chars().next().unwrap(), &left_style);
            }
        }

        // Right border
        if element.style.border_right && width > 1 {
            for row in (y + 1)..(y + height - 1) {
                output.write_char(x + width - 1, row, v.chars().next().unwrap(), &right_style);
            }
        }
    }

    /// Render rich text spans
    fn render_spans(
        &self,
        lines: &[crate::components::text::Line],
        output: &mut Output,
        start_x: u16,
        start_y: u16,
    ) {
        for (line_idx, line) in lines.iter().enumerate() {
            let y = start_y + line_idx as u16;
            let mut x = start_x;

            for span in &line.spans {
                output.write(x, y, &span.content, &span.style);
                x += span.width() as u16;
            }
        }
    }

    /// Request exit
    pub fn exit(&self) {
        self.should_exit.store(true, Ordering::SeqCst);
    }
}

// === Public API Functions ===

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

    #[test]
    fn test_app_options_default() {
        let options = AppOptions::default();
        assert_eq!(options.fps, 60); // Changed from 30 to 60 (Bubbletea default)
        assert!(options.exit_on_ctrl_c);
        assert!(!options.alternate_screen); // Changed: inline mode by default
    }

    #[test]
    fn test_app_builder_defaults() {
        fn dummy() -> Element {
            crate::components::Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy);
        assert!(!builder.options().alternate_screen);
        assert_eq!(builder.options().fps, 60);
    }

    #[test]
    fn test_app_builder_fullscreen() {
        fn dummy() -> Element {
            crate::components::Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy).fullscreen();
        assert!(builder.options().alternate_screen);
    }

    #[test]
    fn test_app_builder_inline() {
        fn dummy() -> Element {
            crate::components::Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy).fullscreen().inline();
        assert!(!builder.options().alternate_screen);
    }

    #[test]
    fn test_app_builder_fps() {
        fn dummy() -> Element {
            crate::components::Text::new("test").into_element()
        }
        let builder = AppBuilder::new(dummy).fps(30);
        assert_eq!(builder.options().fps, 30);
    }

    #[test]
    fn test_mode_switch_enum() {
        assert_eq!(ModeSwitch::EnterAltScreen, ModeSwitch::EnterAltScreen);
        assert_ne!(ModeSwitch::EnterAltScreen, ModeSwitch::ExitAltScreen);
    }
}
