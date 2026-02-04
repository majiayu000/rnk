//! Markdown rendering component
//!
//! Renders basic Markdown text with terminal styling.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Markdown rendering component
#[derive(Debug, Clone)]
pub struct Markdown {
    /// Raw markdown content
    content: String,
    /// Code block color
    code_color: Color,
    /// Heading color
    heading_color: Color,
    /// Link color
    link_color: Color,
    /// Quote color
    quote_color: Color,
    /// Maximum width
    width: Option<u16>,
    /// Key for reconciliation
    key: Option<String>,
}

impl Markdown {
    /// Create a new markdown renderer
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            code_color: Color::Yellow,
            heading_color: Color::Cyan,
            link_color: Color::Blue,
            quote_color: Color::BrightBlack,
            width: None,
            key: None,
        }
    }

    /// Set code color
    pub fn code_color(mut self, color: Color) -> Self {
        self.code_color = color;
        self
    }

    /// Set heading color
    pub fn heading_color(mut self, color: Color) -> Self {
        self.heading_color = color;
        self
    }

    /// Set link color
    pub fn link_color(mut self, color: Color) -> Self {
        self.link_color = color;
        self
    }

    /// Set quote color
    pub fn quote_color(mut self, color: Color) -> Self {
        self.quote_color = color;
        self
    }

    /// Set maximum width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        let lines = self.parse_markdown();

        let mut container = TinkBox::new()
            .flex_direction(FlexDirection::Column)
            .children(lines);

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }

    fn parse_markdown(&self) -> Vec<Element> {
        let mut elements = Vec::new();
        let mut in_code_block = false;
        let mut code_block_content = Vec::new();

        for line in self.content.lines() {
            // Code block handling
            if line.starts_with("```") {
                if in_code_block {
                    // End code block
                    elements.push(self.render_code_block(&code_block_content));
                    code_block_content.clear();
                    in_code_block = false;
                } else {
                    // Start code block
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_block_content.push(line.to_string());
                continue;
            }

            // Parse line
            elements.push(self.parse_line(line));
        }

        // Handle unclosed code block
        if in_code_block && !code_block_content.is_empty() {
            elements.push(self.render_code_block(&code_block_content));
        }

        elements
    }

    fn parse_line(&self, line: &str) -> Element {
        let trimmed = line.trim();

        // Empty line
        if trimmed.is_empty() {
            return TinkBox::new().height(1).into_element();
        }

        // Headings
        if let Some(heading) = self.parse_heading(trimmed) {
            return heading;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            return self.render_hr();
        }

        // Blockquote
        if let Some(rest) = trimmed.strip_prefix('>') {
            return self.render_blockquote(rest.trim_start());
        }

        // Unordered list
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            return self.render_list_item(&trimmed[2..], false, 0);
        }

        // Ordered list
        if let Some(rest) = self.parse_ordered_list(trimmed) {
            return self.render_list_item(rest, true, 0);
        }

        // Regular paragraph with inline formatting
        self.render_inline(trimmed)
    }

    fn parse_heading(&self, line: &str) -> Option<Element> {
        let level = line.chars().take_while(|&c| c == '#').count();
        if level > 0 && level <= 6 && line.chars().nth(level) == Some(' ') {
            let text = &line[level + 1..];
            return Some(self.render_heading(text, level));
        }
        None
    }

    fn parse_ordered_list<'a>(&self, line: &'a str) -> Option<&'a str> {
        let mut chars = line.chars().peekable();
        let mut num_len = 0;

        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                num_len += 1;
                chars.next();
            } else {
                break;
            }
        }

        if num_len > 0 && chars.next() == Some('.') && chars.next() == Some(' ') {
            return Some(&line[num_len + 2..]);
        }

        None
    }

    fn render_heading(&self, text: &str, level: usize) -> Element {
        let mut heading = Text::new(text.to_string())
            .color(self.heading_color)
            .bold();

        if level == 1 {
            heading = heading.underline();
        }

        TinkBox::new()
            .margin_top(if level <= 2 { 1.0 } else { 0.0 })
            .margin_bottom(0.0)
            .child(heading.into_element())
            .into_element()
    }

    fn render_code_block(&self, lines: &[String]) -> Element {
        let mut children = Vec::new();

        for line in lines {
            children.push(
                Text::new(format!("  {}", line))
                    .color(self.code_color)
                    .into_element(),
            );
        }

        TinkBox::new()
            .flex_direction(FlexDirection::Column)
            .margin_top(0.0)
            .margin_bottom(0.0)
            .children(children)
            .into_element()
    }

    fn render_blockquote(&self, text: &str) -> Element {
        TinkBox::new()
            .child(
                Text::new(format!("│ {}", text))
                    .color(self.quote_color)
                    .italic()
                    .into_element(),
            )
            .into_element()
    }

    fn render_list_item(&self, text: &str, ordered: bool, _indent: usize) -> Element {
        let bullet = if ordered { "  " } else { "• " };
        self.render_inline(&format!("{}{}", bullet, text))
    }

    fn render_hr(&self) -> Element {
        let width = self.width.unwrap_or(40) as usize;
        let line = "─".repeat(width);
        Text::new(line).dim().into_element()
    }

    fn render_inline(&self, text: &str) -> Element {
        // Simple inline parsing - handles **bold**, *italic*, `code`, [links](url)
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        let mut segments: Vec<(String, TextStyle)> = Vec::new();
        let mut current_text = String::new();
        let current_style = TextStyle::Normal;

        while let Some(c) = chars.next() {
            match c {
                '`' => {
                    // Inline code
                    if !current_text.is_empty() {
                        segments.push((current_text.clone(), current_style));
                        current_text.clear();
                    }
                    let mut code = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == '`' {
                            chars.next();
                            break;
                        }
                        code.push(chars.next().unwrap());
                    }
                    segments.push((code, TextStyle::Code));
                }
                '*' => {
                    if chars.peek() == Some(&'*') {
                        // Bold
                        chars.next();
                        if !current_text.is_empty() {
                            segments.push((current_text.clone(), current_style));
                            current_text.clear();
                        }
                        let mut bold_text = String::new();
                        while let Some(&next) = chars.peek() {
                            if next == '*' {
                                chars.next();
                                if chars.peek() == Some(&'*') {
                                    chars.next();
                                    break;
                                }
                                bold_text.push('*');
                            } else {
                                bold_text.push(chars.next().unwrap());
                            }
                        }
                        segments.push((bold_text, TextStyle::Bold));
                    } else {
                        // Italic
                        if !current_text.is_empty() {
                            segments.push((current_text.clone(), current_style));
                            current_text.clear();
                        }
                        let mut italic_text = String::new();
                        while let Some(&next) = chars.peek() {
                            if next == '*' {
                                chars.next();
                                break;
                            }
                            italic_text.push(chars.next().unwrap());
                        }
                        segments.push((italic_text, TextStyle::Italic));
                    }
                }
                '[' => {
                    // Link
                    if !current_text.is_empty() {
                        segments.push((current_text.clone(), current_style));
                        current_text.clear();
                    }
                    let mut link_text = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == ']' {
                            chars.next();
                            break;
                        }
                        link_text.push(chars.next().unwrap());
                    }
                    // Skip (url)
                    if chars.peek() == Some(&'(') {
                        chars.next();
                        while let Some(&next) = chars.peek() {
                            if next == ')' {
                                chars.next();
                                break;
                            }
                            chars.next();
                        }
                    }
                    segments.push((link_text, TextStyle::Link));
                }
                _ => {
                    current_text.push(c);
                }
            }
        }

        if !current_text.is_empty() {
            segments.push((current_text, current_style));
        }

        // Build result string with ANSI codes
        for (text, style) in &segments {
            match style {
                TextStyle::Normal => result.push_str(text),
                TextStyle::Bold => result.push_str(&format!("\x1b[1m{}\x1b[0m", text)),
                TextStyle::Italic => result.push_str(&format!("\x1b[3m{}\x1b[0m", text)),
                TextStyle::Code => result.push_str(&format!("\x1b[33m{}\x1b[0m", text)),
                TextStyle::Link => result.push_str(&format!("\x1b[34m\x1b[4m{}\x1b[0m", text)),
            }
        }

        Text::new(result).into_element()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TextStyle {
    Normal,
    Bold,
    Italic,
    Code,
    Link,
}

impl Default for Markdown {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_creation() {
        let md = Markdown::new("# Hello\n\nWorld");
        assert!(md.content.contains("Hello"));
    }

    #[test]
    fn test_empty_markdown() {
        let md = Markdown::new("");
        let _ = md.into_element();
    }

    #[test]
    fn test_heading_parsing() {
        let md = Markdown::new("# H1\n## H2\n### H3");
        let _ = md.into_element();
    }

    #[test]
    fn test_code_block() {
        let md = Markdown::new("```\ncode here\n```");
        let _ = md.into_element();
    }

    #[test]
    fn test_inline_formatting() {
        let md = Markdown::new("**bold** and *italic* and `code`");
        let _ = md.into_element();
    }

    #[test]
    fn test_lists() {
        let md = Markdown::new("- item 1\n- item 2\n\n1. first\n2. second");
        let _ = md.into_element();
    }

    #[test]
    fn test_blockquote() {
        let md = Markdown::new("> This is a quote");
        let _ = md.into_element();
    }
}
