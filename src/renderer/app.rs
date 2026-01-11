//! Application runner

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use crossterm::event::Event;

use crate::core::Element;
use crate::hooks::context::{HookContext, with_hooks};
use crate::hooks::use_app::{set_app_context, AppContext};
use crate::hooks::use_input::{clear_input_handlers, dispatch_key_event};
use crate::hooks::use_mouse::{dispatch_mouse_event, is_mouse_enabled, clear_mouse_handlers};
use crate::layout::LayoutEngine;
use crate::renderer::{Output, Terminal};

/// Application options
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// Target frames per second
    pub fps: u32,
    /// Exit on Ctrl+C
    pub exit_on_ctrl_c: bool,
    /// Use alternate screen
    pub alternate_screen: bool,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            fps: 30,
            exit_on_ctrl_c: true,
            alternate_screen: true,
        }
    }
}

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
    /// Create a new app with default options
    pub fn new(component: F) -> Self {
        Self::with_options(component, AppOptions::default())
    }

    /// Create a new app with custom options
    pub fn with_options(component: F, options: AppOptions) -> Self {
        let needs_render = Arc::new(AtomicBool::new(true));
        let hook_context = Rc::new(RefCell::new(HookContext::new()));

        // Set up render callback
        let needs_render_clone = needs_render.clone();
        hook_context.borrow_mut().set_render_callback(Rc::new(move || {
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
            static_lines: Vec::new(),
            last_width: initial_width,
            last_height: initial_height,
        }
    }

    /// Run the application
    pub fn run(&mut self) -> std::io::Result<()> {
        use std::time::Instant;

        // Enter terminal mode
        if self.options.alternate_screen {
            self.terminal.enter()?;
        } else {
            self.terminal.enter_inline()?;
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

            // Throttle rendering - only render if needed or time elapsed
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
        if self.options.alternate_screen {
            self.terminal.exit()?;
        } else {
            self.terminal.exit_inline()?;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) => {
                // Handle Ctrl+C
                if self.options.exit_on_ctrl_c && Terminal::is_ctrl_c(&Event::Key(key_event.clone())) {
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
        use crossterm::{execute, terminal::{Clear, ClearType}};
        use std::io::stdout;

        // If width decreased, clear the screen to prevent artifacts
        // This is necessary because content that was visible at the old width
        // may now extend past the right edge
        if new_width < self.last_width {
            let _ = execute!(stdout(), Clear(ClearType::All));
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
        let root = with_hooks(self.hook_context.clone(), || {
            (self.component)()
        });

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
        let root_layout = self.layout_engine.get_layout(dynamic_root.id).unwrap_or_default();
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
        use std::io::{stdout, Write};

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
        new_element.children = element.children
            .iter()
            .filter(|child| !child.style.is_static)
            .map(|child| self.filter_static_elements(child))
            .collect();

        new_element
    }

    /// Render element to output buffer (helper for static content)
    fn render_element_to_output(&self, element: &Element, engine: &LayoutEngine, output: &mut Output, offset_x: f32, offset_y: f32) {
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
            let text_x = x + if element.style.has_border() { 1 } else { 0 }
                + element.style.padding.left as u16;
            let text_y = y + if element.style.has_border() { 1 } else { 0 }
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
        let layout = self.layout_engine.get_layout(element.id).unwrap_or_default();

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
        let text_x = x + if element.style.has_border() { 1 } else { 0 }
            + element.style.padding.left as u16;
        let text_y = y + if element.style.has_border() { 1 } else { 0 }
            + element.style.padding.top as u16;

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

    fn render_border(&self, element: &Element, output: &mut Output, x: u16, y: u16, width: u16, height: u16) {
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
                output.write_char(x + width - 1, bottom_y, br.chars().next().unwrap(), &bottom_style);
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
    fn render_spans(&self, lines: &[crate::components::text::Line], output: &mut Output, start_x: u16, start_y: u16) {
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

/// Render a component to the terminal
pub fn render<F>(component: F) -> std::io::Result<()>
where
    F: Fn() -> Element,
{
    App::new(component).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_options_default() {
        let options = AppOptions::default();
        assert_eq!(options.fps, 30);
        assert!(options.exit_on_ctrl_c);
        assert!(options.alternate_screen);
    }
}
