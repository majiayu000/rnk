//! Text component - Text rendering with styles
//!
//! Supports both single-style text and multi-style text via Spans.
//!
//! # Examples
//!
//! Single style:
//! ```ignore
//! Text::new("Hello World").color(Color::Green).bold()
//! ```
//!
//! Multiple styles (Spans):
//! ```ignore
//! Text::spans(vec![
//!     Span::new("Hello ").color(Color::White),
//!     Span::new("World").color(Color::Green).bold(),
//! ])
//! ```

use crate::core::{Element, ElementType, Style, Color, TextWrap};

/// A styled text fragment
///
/// Span represents a piece of text with its own styling.
/// Multiple Spans can be combined in a Text component to create
/// rich text with mixed styles on a single line.
#[derive(Debug, Clone)]
pub struct Span {
    /// The text content
    pub content: String,
    /// The style for this span
    pub style: Style,
}

impl Span {
    /// Create a new Span with content
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::new(),
        }
    }

    /// Create an unstyled Span (raw text)
    pub fn raw(content: impl Into<String>) -> Self {
        Self::new(content)
    }

    /// Create a styled Span
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self {
            content: content.into(),
            style,
        }
    }

    // === Style methods (chainable) ===

    /// Set text color
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    /// Set foreground color (alias for color)
    pub fn fg(self, color: Color) -> Self {
        self.color(color)
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }

    /// Set background color (alias)
    pub fn bg(self, color: Color) -> Self {
        self.background(color)
    }

    /// Set bold
    pub fn bold(mut self) -> Self {
        self.style.bold = true;
        self
    }

    /// Set italic
    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        self
    }

    /// Set underline
    pub fn underline(mut self) -> Self {
        self.style.underline = true;
        self
    }

    /// Set strikethrough
    pub fn strikethrough(mut self) -> Self {
        self.style.strikethrough = true;
        self
    }

    /// Set dim
    pub fn dim(mut self) -> Self {
        self.style.dim = true;
        self
    }

    /// Set inverse
    pub fn inverse(mut self) -> Self {
        self.style.inverse = true;
        self
    }

    /// Get the display width of this span
    pub fn width(&self) -> usize {
        use unicode_width::UnicodeWidthStr;
        self.content.width()
    }
}

impl<T: Into<String>> From<T> for Span {
    fn from(s: T) -> Self {
        Span::new(s)
    }
}

/// A line of text composed of multiple Spans
#[derive(Debug, Clone, Default)]
pub struct Line {
    /// The spans that make up this line
    pub spans: Vec<Span>,
}

impl Line {
    /// Create a new empty Line
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Create a Line from spans
    pub fn from_spans(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    /// Create a Line from a single string (raw text)
    pub fn raw(content: impl Into<String>) -> Self {
        Self {
            spans: vec![Span::new(content)],
        }
    }

    /// Add a span to this line
    pub fn span(mut self, span: impl Into<Span>) -> Self {
        self.spans.push(span.into());
        self
    }

    /// Get the display width of this line
    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| s.width()).sum()
    }

    /// Check if the line is empty
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.spans.iter().all(|s| s.content.is_empty())
    }
}

impl From<&str> for Line {
    fn from(s: &str) -> Self {
        Line::from_spans(vec![Span::new(s)])
    }
}

impl From<String> for Line {
    fn from(s: String) -> Self {
        Line::from_spans(vec![Span::new(s)])
    }
}

impl From<Span> for Line {
    fn from(s: Span) -> Self {
        Line::from_spans(vec![s])
    }
}

impl From<Vec<Span>> for Line {
    fn from(spans: Vec<Span>) -> Self {
        Line::from_spans(spans)
    }
}

/// Text component builder
///
/// Text can be created in two ways:
/// 1. Simple text with a single style: `Text::new("Hello").color(Color::Green)`
/// 2. Rich text with multiple spans: `Text::spans(vec![Span::new("Hello").bold(), Span::new(" World")])`
#[derive(Debug, Clone)]
pub struct Text {
    /// The lines of text (each line contains spans)
    lines: Vec<Line>,
    /// Default style applied to spans without explicit styling
    style: Style,
    /// Key for reconciliation
    key: Option<String>,
}

impl Text {
    /// Create a new Text with content (single style)
    pub fn new(content: impl Into<String>) -> Self {
        let content_str: String = content.into();
        let lines: Vec<Line> = content_str
            .lines()
            .map(Line::raw)
            .collect();

        Self {
            lines: if lines.is_empty() { vec![Line::raw("")] } else { lines },
            style: Style::new(),
            key: None,
        }
    }

    /// Create a new Text from multiple spans (rich text, single line)
    pub fn spans(spans: Vec<Span>) -> Self {
        Self {
            lines: vec![Line::from_spans(spans)],
            style: Style::new(),
            key: None,
        }
    }

    /// Create a new Text from a Line
    pub fn line(line: Line) -> Self {
        Self {
            lines: vec![line],
            style: Style::new(),
            key: None,
        }
    }

    /// Create a new Text from multiple Lines
    pub fn from_lines(lines: Vec<Line>) -> Self {
        Self {
            lines,
            style: Style::new(),
            key: None,
        }
    }

    /// Set key for reconciliation
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Get the lines
    pub fn get_lines(&self) -> &[Line] {
        &self.lines
    }

    // === Text styles (applied as default to all spans) ===

    /// Set text color
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        // Apply to existing spans that don't have a color
        for line in &mut self.lines {
            for span in &mut line.spans {
                if span.style.color.is_none() {
                    span.style.color = Some(color);
                }
            }
        }
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        for line in &mut self.lines {
            for span in &mut line.spans {
                if span.style.background_color.is_none() {
                    span.style.background_color = Some(color);
                }
            }
        }
        self
    }

    /// Alias for background
    pub fn bg(self, color: Color) -> Self {
        self.background(color)
    }

    /// Set bold
    pub fn bold(mut self) -> Self {
        self.style.bold = true;
        for line in &mut self.lines {
            for span in &mut line.spans {
                span.style.bold = true;
            }
        }
        self
    }

    /// Set italic
    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        for line in &mut self.lines {
            for span in &mut line.spans {
                span.style.italic = true;
            }
        }
        self
    }

    /// Set underline
    pub fn underline(mut self) -> Self {
        self.style.underline = true;
        for line in &mut self.lines {
            for span in &mut line.spans {
                span.style.underline = true;
            }
        }
        self
    }

    /// Set strikethrough
    pub fn strikethrough(mut self) -> Self {
        self.style.strikethrough = true;
        for line in &mut self.lines {
            for span in &mut line.spans {
                span.style.strikethrough = true;
            }
        }
        self
    }

    /// Set dim (less bright)
    pub fn dim(mut self) -> Self {
        self.style.dim = true;
        for line in &mut self.lines {
            for span in &mut line.spans {
                span.style.dim = true;
            }
        }
        self
    }

    /// Set inverse (swap foreground and background)
    pub fn inverse(mut self) -> Self {
        self.style.inverse = true;
        for line in &mut self.lines {
            for span in &mut line.spans {
                span.style.inverse = true;
            }
        }
        self
    }

    /// Set text wrap behavior
    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.style.text_wrap = wrap;
        self
    }

    // === Convenience methods ===

    /// Apply error style (red color)
    pub fn error(self) -> Self {
        self.color(Color::Red)
    }

    /// Apply success style (green color)
    pub fn success(self) -> Self {
        self.color(Color::Green)
    }

    /// Apply warning style (yellow color)
    pub fn warning(self) -> Self {
        self.color(Color::Yellow)
    }

    /// Apply info style (blue color)
    pub fn info(self) -> Self {
        self.color(Color::Blue)
    }

    /// Apply muted style (dim)
    pub fn muted(self) -> Self {
        self.dim()
    }

    /// Convert to Element
    ///
    /// For simple text (single span per line), uses text_content.
    /// For rich text (multiple spans), stores spans in the element.
    pub fn into_element(self) -> Element {
        let mut element = Element::new(ElementType::Text);
        element.style = self.style;
        element.key = self.key;

        // Check if this is simple text (single span per line, no mixed styles)
        let is_simple = self.lines.len() == 1
            && self.lines[0].spans.len() == 1;

        if is_simple {
            // Simple text: use text_content for backward compatibility
            element.text_content = Some(self.lines[0].spans[0].content.clone());
            // Copy span style to element style
            let span_style = &self.lines[0].spans[0].style;
            if span_style.color.is_some() {
                element.style.color = span_style.color;
            }
            if span_style.background_color.is_some() {
                element.style.background_color = span_style.background_color;
            }
            if span_style.bold {
                element.style.bold = true;
            }
            if span_style.italic {
                element.style.italic = true;
            }
            if span_style.underline {
                element.style.underline = true;
            }
            if span_style.strikethrough {
                element.style.strikethrough = true;
            }
            if span_style.dim {
                element.style.dim = true;
            }
            if span_style.inverse {
                element.style.inverse = true;
            }
        } else {
            // Rich text: store spans for rendering
            element.spans = Some(self.lines);
        }

        element
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_creation() {
        let element = Text::new("Hello").into_element();
        assert_eq!(element.get_text(), Some("Hello"));
    }

    #[test]
    fn test_text_styles() {
        let element = Text::new("Styled")
            .color(Color::Green)
            .bold()
            .underline()
            .into_element();

        assert_eq!(element.style.color, Some(Color::Green));
        assert!(element.style.bold);
        assert!(element.style.underline);
    }

    #[test]
    fn test_text_convenience_methods() {
        let error = Text::new("Error").error().into_element();
        assert_eq!(error.style.color, Some(Color::Red));

        let success = Text::new("Success").success().into_element();
        assert_eq!(success.style.color, Some(Color::Green));
    }

    #[test]
    fn test_span_creation() {
        let span = Span::new("Hello")
            .color(Color::Green)
            .bold();

        assert_eq!(span.content, "Hello");
        assert_eq!(span.style.color, Some(Color::Green));
        assert!(span.style.bold);
    }

    #[test]
    fn test_text_with_spans() {
        let text = Text::spans(vec![
            Span::new("Hello ").color(Color::White),
            Span::new("World").color(Color::Green).bold(),
        ]);

        assert_eq!(text.lines.len(), 1);
        assert_eq!(text.lines[0].spans.len(), 2);
        assert_eq!(text.lines[0].spans[0].content, "Hello ");
        assert_eq!(text.lines[0].spans[1].content, "World");
    }

    #[test]
    fn test_text_spans_element() {
        let element = Text::spans(vec![
            Span::new("Hello ").color(Color::White),
            Span::new("World").color(Color::Green),
        ]).into_element();

        // Should have spans, not simple text_content
        assert!(element.spans.is_some());
        let spans = element.spans.as_ref().unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].spans.len(), 2);
    }

    #[test]
    fn test_line_creation() {
        let line = Line::new()
            .span(Span::new("Part 1").color(Color::Red))
            .span(Span::new(" - "))
            .span(Span::new("Part 2").color(Color::Blue));

        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.width(), 15); // "Part 1" (6) + " - " (3) + "Part 2" (6)
    }

    #[test]
    fn test_multiline_text() {
        let text = Text::from_lines(vec![
            Line::from_spans(vec![
                Span::new("Line 1").bold(),
            ]),
            Line::from_spans(vec![
                Span::new("Line 2").italic(),
            ]),
        ]);

        assert_eq!(text.lines.len(), 2);
    }
}
