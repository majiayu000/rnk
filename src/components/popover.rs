//! Popover component for contextual overlays
//!
//! Provides a popover/dropdown component for displaying contextual content.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Popover;
//!
//! fn app() -> Element {
//!     let popover = Popover::new("Click me")
//!         .content("This is the popover content")
//!         .position(PopoverPosition::Bottom);
//!
//!     popover.into_element()
//! }
//! ```

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};

/// Position for the popover relative to the trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverPosition {
    /// Above the trigger
    Top,
    /// Below the trigger
    #[default]
    Bottom,
    /// To the left of the trigger
    Left,
    /// To the right of the trigger
    Right,
}

/// Arrow style for the popover
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverArrow {
    /// No arrow
    #[default]
    None,
    /// Simple arrow
    Simple,
    /// Unicode arrow
    Unicode,
}

impl PopoverArrow {
    /// Get the arrow character for a position
    pub fn char_for_position(&self, position: PopoverPosition) -> &'static str {
        match self {
            PopoverArrow::None => "",
            PopoverArrow::Simple => match position {
                PopoverPosition::Top => "v",
                PopoverPosition::Bottom => "^",
                PopoverPosition::Left => ">",
                PopoverPosition::Right => "<",
            },
            PopoverArrow::Unicode => match position {
                PopoverPosition::Top => "▼",
                PopoverPosition::Bottom => "▲",
                PopoverPosition::Left => "▶",
                PopoverPosition::Right => "◀",
            },
        }
    }
}

/// Border style for the popover
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverBorder {
    /// No border
    None,
    /// Single line border
    #[default]
    Single,
    /// Double line border
    Double,
    /// Rounded corners
    Rounded,
}

impl PopoverBorder {
    /// Get border characters (top-left, top-right, bottom-left, bottom-right, horizontal, vertical)
    pub fn chars(&self) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str)> {
        match self {
            PopoverBorder::None => None,
            PopoverBorder::Single => Some(("┌", "┐", "└", "┘", "─", "│")),
            PopoverBorder::Double => Some(("╔", "╗", "╚", "╝", "═", "║")),
            PopoverBorder::Rounded => Some(("╭", "╮", "╰", "╯", "─", "│")),
        }
    }
}

/// Popover style configuration
#[derive(Debug, Clone)]
pub struct PopoverStyle {
    /// Border style
    pub border: PopoverBorder,
    /// Arrow style
    pub arrow: PopoverArrow,
    /// Background color
    pub background: Option<Color>,
    /// Foreground color
    pub foreground: Option<Color>,
    /// Border color
    pub border_color: Option<Color>,
    /// Padding
    pub padding: usize,
    /// Maximum width
    pub max_width: Option<usize>,
}

impl Default for PopoverStyle {
    fn default() -> Self {
        Self {
            border: PopoverBorder::Single,
            arrow: PopoverArrow::None,
            background: None,
            foreground: None,
            border_color: None,
            padding: 1,
            max_width: Some(40),
        }
    }
}

impl PopoverStyle {
    /// Create a new popover style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set border style
    pub fn border(mut self, border: PopoverBorder) -> Self {
        self.border = border;
        self
    }

    /// Set arrow style
    pub fn arrow(mut self, arrow: PopoverArrow) -> Self {
        self.arrow = arrow;
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set foreground color
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set padding
    pub fn padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }

    /// Set maximum width
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Minimal style (no border)
    pub fn minimal() -> Self {
        Self::new().border(PopoverBorder::None).padding(0)
    }

    /// Tooltip style (rounded border with arrow)
    pub fn tooltip() -> Self {
        Self::new()
            .border(PopoverBorder::Rounded)
            .arrow(PopoverArrow::Unicode)
    }

    /// Menu style (single border, no arrow)
    pub fn menu() -> Self {
        Self::new()
            .border(PopoverBorder::Single)
            .arrow(PopoverArrow::None)
    }
}

/// A popover component
#[derive(Debug, Clone)]
pub struct Popover {
    /// Trigger text/label
    trigger: String,
    /// Content to display in the popover
    content: String,
    /// Position relative to trigger
    position: PopoverPosition,
    /// Whether the popover is open
    open: bool,
    /// Style configuration
    style: PopoverStyle,
}

impl Popover {
    /// Create a new popover with a trigger
    pub fn new(trigger: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            content: String::new(),
            position: PopoverPosition::Bottom,
            open: false,
            style: PopoverStyle::default(),
        }
    }

    /// Set the popover content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Set the position
    pub fn position(mut self, position: PopoverPosition) -> Self {
        self.position = position;
        self
    }

    /// Set whether the popover is open
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the style
    pub fn style(mut self, style: PopoverStyle) -> Self {
        self.style = style;
        self
    }

    /// Render the popover content as a string
    fn render_content(&self) -> String {
        let mut result = String::new();

        // Apply max width
        let content = if let Some(max_width) = self.style.max_width {
            if self.content.len() > max_width {
                let mut truncated = self.content.chars().take(max_width - 3).collect::<String>();
                truncated.push_str("...");
                truncated
            } else {
                self.content.clone()
            }
        } else {
            self.content.clone()
        };

        // Add arrow if needed (for top position, arrow goes at bottom)
        if self.position == PopoverPosition::Top {
            let arrow = self.style.arrow.char_for_position(self.position);
            if !arrow.is_empty() {
                result.push_str(arrow);
                result.push('\n');
            }
        }

        // Render with border
        if let Some((tl, tr, bl, br, h, v)) = self.style.border.chars() {
            let padding = " ".repeat(self.style.padding);
            let inner_width = content.len() + self.style.padding * 2;

            // Top border
            result.push_str(tl);
            result.push_str(&h.repeat(inner_width));
            result.push_str(tr);
            result.push('\n');

            // Content
            result.push_str(v);
            result.push_str(&padding);
            result.push_str(&content);
            result.push_str(&padding);
            result.push_str(v);
            result.push('\n');

            // Bottom border
            result.push_str(bl);
            result.push_str(&h.repeat(inner_width));
            result.push_str(br);
        } else {
            // No border
            let padding = " ".repeat(self.style.padding);
            result.push_str(&padding);
            result.push_str(&content);
            result.push_str(&padding);
        }

        // Add arrow if needed (for bottom position, arrow goes at top)
        if self.position == PopoverPosition::Bottom {
            let arrow = self.style.arrow.char_for_position(self.position);
            if !arrow.is_empty() {
                result.push('\n');
                result.push_str(arrow);
            }
        }

        result
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let direction = match self.position {
            PopoverPosition::Top => FlexDirection::ColumnReverse,
            PopoverPosition::Bottom => FlexDirection::Column,
            PopoverPosition::Left => FlexDirection::RowReverse,
            PopoverPosition::Right => FlexDirection::Row,
        };

        let mut container = Box::new().flex_direction(direction);

        // Add trigger
        container = container.child(Text::new(&self.trigger).into_element());

        // Add content if open
        if self.open {
            let content_text = self.render_content();
            let mut content_element = Text::new(content_text);

            if let Some(fg) = self.style.foreground {
                content_element = content_element.color(fg);
            }
            if let Some(bg) = self.style.background {
                content_element = content_element.background(bg);
            }

            container = container.child(content_element.into_element());
        }

        container.into_element()
    }
}

impl Default for Popover {
    fn default() -> Self {
        Self::new("")
    }
}

/// Create a popover
pub fn popover(trigger: impl Into<String>) -> Popover {
    Popover::new(trigger)
}

/// Create a popover with content
pub fn popover_with_content(trigger: impl Into<String>, content: impl Into<String>) -> Popover {
    Popover::new(trigger).content(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popover_creation() {
        let p = Popover::new("Click me");
        assert_eq!(p.trigger, "Click me");
        assert!(!p.open);
    }

    #[test]
    fn test_popover_content() {
        let p = Popover::new("Trigger").content("Content");
        assert_eq!(p.content, "Content");
    }

    #[test]
    fn test_popover_position() {
        let p = Popover::new("T").position(PopoverPosition::Top);
        assert_eq!(p.position, PopoverPosition::Top);
    }

    #[test]
    fn test_popover_open() {
        let p = Popover::new("T").open(true);
        assert!(p.open);
    }

    #[test]
    fn test_popover_style() {
        let style = PopoverStyle::tooltip();
        let p = Popover::new("T").style(style);
        assert_eq!(p.style.border, PopoverBorder::Rounded);
    }

    #[test]
    fn test_popover_render_content() {
        let p = Popover::new("T")
            .content("Hello")
            .style(PopoverStyle::minimal());
        let rendered = p.render_content();
        assert!(rendered.contains("Hello"));
    }

    #[test]
    fn test_popover_render_with_border() {
        let p = Popover::new("T")
            .content("Test")
            .style(PopoverStyle::new().border(PopoverBorder::Single));
        let rendered = p.render_content();
        assert!(rendered.contains("┌"));
        assert!(rendered.contains("┘"));
    }

    #[test]
    fn test_popover_into_element() {
        let p = Popover::new("Trigger").content("Content").open(true);
        let _ = p.into_element();
    }

    #[test]
    fn test_popover_arrow() {
        assert_eq!(PopoverArrow::Unicode.char_for_position(PopoverPosition::Top), "▼");
        assert_eq!(PopoverArrow::Unicode.char_for_position(PopoverPosition::Bottom), "▲");
        assert_eq!(PopoverArrow::Simple.char_for_position(PopoverPosition::Left), ">");
        assert_eq!(PopoverArrow::None.char_for_position(PopoverPosition::Right), "");
    }

    #[test]
    fn test_popover_border_chars() {
        assert!(PopoverBorder::None.chars().is_none());
        assert!(PopoverBorder::Single.chars().is_some());
        assert!(PopoverBorder::Double.chars().is_some());
        assert!(PopoverBorder::Rounded.chars().is_some());
    }

    #[test]
    fn test_popover_style_builder() {
        let style = PopoverStyle::new()
            .border(PopoverBorder::Rounded)
            .arrow(PopoverArrow::Unicode)
            .padding(2)
            .max_width(50);

        assert_eq!(style.border, PopoverBorder::Rounded);
        assert_eq!(style.arrow, PopoverArrow::Unicode);
        assert_eq!(style.padding, 2);
        assert_eq!(style.max_width, Some(50));
    }

    #[test]
    fn test_popover_helpers() {
        let p1 = popover("Test");
        assert_eq!(p1.trigger, "Test");

        let p2 = popover_with_content("Trigger", "Content");
        assert_eq!(p2.trigger, "Trigger");
        assert_eq!(p2.content, "Content");
    }
}
