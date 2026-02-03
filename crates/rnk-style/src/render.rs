//! Rendering logic for styled text

use crate::color::Color;
use crate::style::{Align, Style};
use unicode_width::UnicodeWidthStr;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const UNDERLINE: &str = "\x1b[4m";
const INVERSE: &str = "\x1b[7m";
const STRIKETHROUGH: &str = "\x1b[9m";

/// Render text with the given style
pub fn render(style: &Style, text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    // Calculate content width
    let content_width = if let Some(w) = style.width {
        w as usize
    } else {
        lines.iter().map(|l| l.width()).max().unwrap_or(0)
    };

    // Calculate total width including padding and border
    let border_width = if style.border_style.is_visible() { 2 } else { 0 };
    let inner_width = content_width + style.padding_left as usize + style.padding_right as usize;
    let _total_width = inner_width + border_width;

    // Build style codes
    let style_codes = build_style_codes(style);
    let border_codes = build_border_codes(style);

    let mut result = String::new();

    // Top margin
    for _ in 0..style.margin_top {
        result.push('\n');
    }

    // Get border characters
    let (tl, tr, bl, br, h, v) = style.border_style.chars();

    // Top border
    if style.border_style.is_visible() {
        add_margin_left(&mut result, style.margin_left);
        result.push_str(&border_codes);
        result.push_str(tl);
        for _ in 0..inner_width {
            result.push_str(h);
        }
        result.push_str(tr);
        result.push_str(RESET);
        result.push('\n');
    }

    // Top padding
    for _ in 0..style.padding_top {
        render_padding_line(&mut result, style, &border_codes, v, inner_width, &style_codes);
    }

    // Content lines
    let line_count = lines.len();
    let target_height = style.height.map(|h| h as usize).unwrap_or(line_count);

    for (i, line) in lines.iter().enumerate() {
        if i >= target_height {
            break;
        }
        render_content_line(&mut result, style, &border_codes, v, inner_width, &style_codes, line, content_width);
    }

    // Fill remaining height with empty lines
    for _ in line_count..target_height {
        render_padding_line(&mut result, style, &border_codes, v, inner_width, &style_codes);
    }

    // Bottom padding
    for _ in 0..style.padding_bottom {
        render_padding_line(&mut result, style, &border_codes, v, inner_width, &style_codes);
    }

    // Bottom border
    if style.border_style.is_visible() {
        add_margin_left(&mut result, style.margin_left);
        result.push_str(&border_codes);
        result.push_str(bl);
        for _ in 0..inner_width {
            result.push_str(h);
        }
        result.push_str(br);
        result.push_str(RESET);
        result.push('\n');
    }

    // Bottom margin
    for _ in 0..style.margin_bottom {
        result.push('\n');
    }

    // Remove trailing newline if present
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

fn build_style_codes(style: &Style) -> String {
    let mut codes = String::new();

    if style.fg != Color::Default {
        codes.push_str(&style.fg.fg_code());
    }
    if style.bg != Color::Default {
        codes.push_str(&style.bg.bg_code());
    }
    if style.bold {
        codes.push_str(BOLD);
    }
    if style.dim {
        codes.push_str(DIM);
    }
    if style.italic {
        codes.push_str(ITALIC);
    }
    if style.underline {
        codes.push_str(UNDERLINE);
    }
    if style.inverse {
        codes.push_str(INVERSE);
    }
    if style.strikethrough {
        codes.push_str(STRIKETHROUGH);
    }

    codes
}

fn build_border_codes(style: &Style) -> String {
    if style.border_fg != Color::Default {
        style.border_fg.fg_code()
    } else {
        String::new()
    }
}

fn add_margin_left(result: &mut String, margin: u16) {
    for _ in 0..margin {
        result.push(' ');
    }
}

fn render_padding_line(
    result: &mut String,
    style: &Style,
    border_codes: &str,
    v: &str,
    inner_width: usize,
    style_codes: &str,
) {
    add_margin_left(result, style.margin_left);

    if style.border_style.is_visible() {
        result.push_str(border_codes);
        result.push_str(v);
        result.push_str(RESET);
    }

    // Background fill
    result.push_str(style_codes);
    for _ in 0..inner_width {
        result.push(' ');
    }
    result.push_str(RESET);

    if style.border_style.is_visible() {
        result.push_str(border_codes);
        result.push_str(v);
        result.push_str(RESET);
    }

    result.push('\n');
}

fn render_content_line(
    result: &mut String,
    style: &Style,
    border_codes: &str,
    v: &str,
    _inner_width: usize,
    style_codes: &str,
    text: &str,
    content_width: usize,
) {
    add_margin_left(result, style.margin_left);

    if style.border_style.is_visible() {
        result.push_str(border_codes);
        result.push_str(v);
        result.push_str(RESET);
    }

    result.push_str(style_codes);

    // Left padding
    for _ in 0..style.padding_left {
        result.push(' ');
    }

    // Content with alignment
    let text_width = text.width();
    let padding_needed = content_width.saturating_sub(text_width);

    match style.align {
        Align::Left => {
            result.push_str(text);
            for _ in 0..padding_needed {
                result.push(' ');
            }
        }
        Align::Center => {
            let left_pad = padding_needed / 2;
            let right_pad = padding_needed - left_pad;
            for _ in 0..left_pad {
                result.push(' ');
            }
            result.push_str(text);
            for _ in 0..right_pad {
                result.push(' ');
            }
        }
        Align::Right => {
            for _ in 0..padding_needed {
                result.push(' ');
            }
            result.push_str(text);
        }
    }

    // Right padding
    for _ in 0..style.padding_right {
        result.push(' ');
    }

    result.push_str(RESET);

    if style.border_style.is_visible() {
        result.push_str(border_codes);
        result.push_str(v);
        result.push_str(RESET);
    }

    result.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::border::BorderStyle;

    #[test]
    fn test_simple_render() {
        let style = Style::new();
        let output = style.render("Hello");
        // Even without styles, we add RESET for safety
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_colored_render() {
        let style = Style::new().fg(Color::Red);
        let output = style.render("Hello");
        assert!(output.contains("\x1b[31m"));
        assert!(output.contains("Hello"));
        assert!(output.contains(RESET));
    }

    #[test]
    fn test_bold_render() {
        let style = Style::new().bold();
        let output = style.render("Hello");
        assert!(output.contains(BOLD));
    }

    #[test]
    fn test_padding_render() {
        let style = Style::new().padding(1, 2);
        let output = style.render("Hi");
        // Should have padding lines and spaces
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3); // 1 top padding + 1 content + 1 bottom padding
    }

    #[test]
    fn test_border_render() {
        let style = Style::new().border(BorderStyle::Rounded);
        let output = style.render("Hi");
        assert!(output.contains("╭"));
        assert!(output.contains("╯"));
        assert!(output.contains("│"));
    }

    #[test]
    fn test_multiline_render() {
        let style = Style::new();
        let output = style.render("Line 1\nLine 2");
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_alignment_center() {
        let style = Style::new().width(10).center();
        let output = style.render("Hi");
        // "Hi" is 2 chars, width is 10, so 4 spaces on each side
        assert!(output.contains("    Hi    "));
    }

    #[test]
    fn test_alignment_right() {
        let style = Style::new().width(10).right();
        let output = style.render("Hi");
        assert!(output.contains("        Hi"));
    }

    #[test]
    fn test_full_style() {
        let style = Style::new()
            .fg(Color::Cyan)
            .bg(Color::Black)
            .bold()
            .padding(1, 2)
            .border(BorderStyle::Rounded)
            .border_fg(Color::Magenta);

        let output = style.render("Test");
        assert!(output.contains("╭"));
        assert!(output.contains("Test"));
    }
}
