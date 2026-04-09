//! Text measurement utilities

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[inline]
fn grapheme_width(grapheme: &str) -> usize {
    UnicodeWidthStr::width(grapheme)
}

#[inline]
fn fits_within_width(text: &str, max_width: usize) -> bool {
    if let Some(width) = ascii_width_fast_path(text) {
        return width <= max_width;
    }

    let mut width = 0;
    for grapheme in text.graphemes(true) {
        width += grapheme_width(grapheme);
        if width > max_width {
            return false;
        }
    }
    true
}

fn take_prefix_by_width(text: &str, max_width: usize) -> String {
    if let Some(width) = ascii_width_fast_path(text) {
        if width <= max_width {
            return text.to_string();
        }
        return text[..max_width].to_string();
    }

    let mut result = String::new();
    let mut width = 0;
    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme);
        if width + grapheme_width > max_width {
            break;
        }
        result.push_str(grapheme);
        width += grapheme_width;
    }
    result
}

/// Measure the display width of text using grapheme clusters
///
/// This function properly handles:
/// - CJK characters (width = 2)
/// - Emoji sequences (including ZWJ sequences like 👨‍👩‍👧‍👦)
/// - Combining characters (e.g., é = e + combining acute)
/// - Zero-width characters
pub fn measure_text_width(text: &str) -> usize {
    if let Some(width) = ascii_width_fast_path(text) {
        return width;
    }

    text.graphemes(true).map(grapheme_width).sum()
}

/// Measure the display width using grapheme clusters (alias for measure_text_width)
pub fn display_width(text: &str) -> usize {
    measure_text_width(text)
}

/// Measure text dimensions (width, height)
pub fn measure_text(text: &str) -> (usize, usize) {
    if let Some(dimensions) = ascii_measure_text_dimensions_fast_path(text) {
        return dimensions;
    }

    let mut height = 0usize;
    let mut width = 0usize;

    for line in text.lines() {
        height += 1;
        width = width.max(measure_text_width(line));
    }

    if height == 0 {
        height = 1;
    }

    (width, height)
}

/// Wrap text to fit within a maximum width (grapheme-aware)
pub fn wrap_text(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    if text.is_empty() {
        return String::new();
    }

    if let Some(width) = ascii_width_fast_path(text) {
        if width <= max_width {
            return text.to_string();
        }

        let line_breaks = width.div_ceil(max_width).saturating_sub(1);
        let mut result = String::with_capacity(text.len() + line_breaks);
        let mut current_width = 0;
        for byte in text.bytes() {
            if current_width == max_width {
                result.push('\n');
                current_width = 0;
            }
            result.push(byte as char);
            current_width += 1;
        }
        return result;
    }

    let mut result = String::with_capacity(text.len());
    let mut current_width = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme);

        if grapheme == "\n" {
            result.push('\n');
            current_width = 0;
        } else if current_width + grapheme_width > max_width {
            result.push('\n');
            result.push_str(grapheme);
            current_width = grapheme_width;
        } else {
            result.push_str(grapheme);
            current_width += grapheme_width;
        }
    }

    result
}

/// Truncate text to fit within a maximum width (grapheme-aware)
pub fn truncate_text(text: &str, max_width: usize, ellipsis: &str) -> String {
    if let (Some(text_width), Some(ellipsis_width)) =
        (ascii_width_fast_path(text), ascii_width_fast_path(ellipsis))
    {
        if text_width <= max_width {
            return text.to_string();
        }

        if max_width <= ellipsis_width {
            return ellipsis[..max_width].to_string();
        }

        let target_width = max_width - ellipsis_width;
        let mut result = String::with_capacity(max_width);
        result.push_str(&text[..target_width]);
        result.push_str(ellipsis);
        return result;
    }

    if fits_within_width(text, max_width) {
        return text.to_string();
    }

    let ellipsis_width = measure_text_width(ellipsis);
    if max_width <= ellipsis_width {
        return take_prefix_by_width(ellipsis, max_width);
    }

    let target_width = max_width - ellipsis_width;
    let mut result = String::new();
    let mut current_width = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme);
        if current_width + grapheme_width > target_width {
            break;
        }
        result.push_str(grapheme);
        current_width += grapheme_width;
    }

    result.push_str(ellipsis);
    result
}

/// Truncate text from the start (grapheme-aware)
pub fn truncate_start(text: &str, max_width: usize, ellipsis: &str) -> String {
    if let (Some(text_width), Some(ellipsis_width)) =
        (ascii_width_fast_path(text), ascii_width_fast_path(ellipsis))
    {
        if text_width <= max_width {
            return text.to_string();
        }

        if max_width <= ellipsis_width {
            return ellipsis[..max_width].to_string();
        }

        let target_width = max_width - ellipsis_width;
        let start = text_width - target_width;
        let mut result = String::with_capacity(max_width);
        result.push_str(ellipsis);
        result.push_str(&text[start..]);
        return result;
    }

    if fits_within_width(text, max_width) {
        return text.to_string();
    }

    let ellipsis_width = measure_text_width(ellipsis);
    if max_width <= ellipsis_width {
        return take_prefix_by_width(ellipsis, max_width);
    }

    let target_width = max_width - ellipsis_width;
    let mut result = String::new();
    let mut current_width = 0;
    let mut end_graphemes = Vec::new();

    for grapheme in text.graphemes(true).rev() {
        let grapheme_width = grapheme_width(grapheme);
        if current_width + grapheme_width > target_width {
            break;
        }
        end_graphemes.push(grapheme);
        current_width += grapheme_width;
    }

    end_graphemes.reverse();
    result.push_str(ellipsis);
    for g in end_graphemes {
        result.push_str(g);
    }
    result
}

/// Truncate text from the middle (grapheme-aware)
pub fn truncate_middle(text: &str, max_width: usize, ellipsis: &str) -> String {
    if let (Some(text_width), Some(ellipsis_width)) =
        (ascii_width_fast_path(text), ascii_width_fast_path(ellipsis))
    {
        if text_width <= max_width {
            return text.to_string();
        }

        if max_width <= ellipsis_width {
            return ellipsis[..max_width].to_string();
        }

        let available = max_width - ellipsis_width;
        let left_width = available / 2;
        let right_width = available - left_width;
        let right_start = text_width - right_width;

        let mut result = String::with_capacity(max_width);
        result.push_str(&text[..left_width]);
        result.push_str(ellipsis);
        result.push_str(&text[right_start..]);
        return result;
    }

    if fits_within_width(text, max_width) {
        return text.to_string();
    }

    let ellipsis_width = measure_text_width(ellipsis);
    if max_width <= ellipsis_width {
        return take_prefix_by_width(ellipsis, max_width);
    }

    let available = max_width - ellipsis_width;
    let left_width = available / 2;
    let right_width = available - left_width;

    // Build left part
    let mut left = String::new();
    let mut current_width = 0;
    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme);
        if current_width + grapheme_width > left_width {
            break;
        }
        left.push_str(grapheme);
        current_width += grapheme_width;
    }

    // Build right part
    let mut right_graphemes = Vec::new();
    current_width = 0;
    for grapheme in text.graphemes(true).rev() {
        let grapheme_width = grapheme_width(grapheme);
        if current_width + grapheme_width > right_width {
            break;
        }
        right_graphemes.push(grapheme);
        current_width += grapheme_width;
    }
    right_graphemes.reverse();

    let mut right = String::new();
    for g in right_graphemes {
        right.push_str(g);
    }

    format!("{}{}{}", left, ellipsis, right)
}

/// Count wrapped lines for a text block without allocating wrapped content.
pub(crate) fn count_wrapped_lines_by_width(text: &str, max_width: usize) -> usize {
    if text.is_empty() {
        return 1;
    }

    if max_width == 0 {
        return 1;
    }

    if let Some(lines) = ascii_wrapped_line_count_fast_path(text, max_width) {
        return lines;
    }

    let mut lines = 1usize;
    let mut current_width = 0usize;
    let mut ends_with_newline = false;

    for grapheme in text.graphemes(true) {
        if grapheme.ends_with('\n') {
            lines += 1;
            current_width = 0;
            ends_with_newline = true;
            continue;
        }
        ends_with_newline = false;

        let grapheme_width = grapheme_width(grapheme);
        if current_width + grapheme_width > max_width {
            lines += 1;
            current_width = grapheme_width;
        } else {
            current_width += grapheme_width;
        }
    }

    // Match `str::lines()` semantics used by the previous implementation:
    // a trailing '\n' does not produce an extra final empty line.
    if ends_with_newline && lines > 1 {
        lines -= 1;
    }

    lines.max(1)
}

fn ascii_measure_text_dimensions_fast_path(text: &str) -> Option<(usize, usize)> {
    if text.is_empty() {
        return Some((0, 1));
    }

    let bytes = text.as_bytes();
    let mut max_width = 0usize;
    let mut current_width = 0usize;
    let mut height = 1usize;
    let mut ends_with_line_break = false;
    let mut i = 0usize;

    while i < bytes.len() {
        let byte = bytes[i];

        if byte == b'\n' {
            max_width = max_width.max(current_width);
            current_width = 0;
            height += 1;
            ends_with_line_break = true;
            i += 1;
            continue;
        }

        if byte == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            max_width = max_width.max(current_width);
            current_width = 0;
            height += 1;
            ends_with_line_break = true;
            i += 2;
            continue;
        }

        if !byte.is_ascii() || byte < 0x20 || byte == 0x7f {
            return None;
        }

        current_width += 1;
        ends_with_line_break = false;
        i += 1;
    }

    max_width = max_width.max(current_width);
    if ends_with_line_break && height > 1 {
        height -= 1;
    }

    Some((max_width, height.max(1)))
}

fn ascii_wrapped_line_count_fast_path(text: &str, max_width: usize) -> Option<usize> {
    if text.is_empty() {
        return Some(1);
    }

    let bytes = text.as_bytes();
    let mut lines = 1usize;
    let mut current_width = 0usize;
    let mut ends_with_line_break = false;
    let mut i = 0usize;

    while i < bytes.len() {
        let byte = bytes[i];

        if byte == b'\n' {
            lines += 1;
            current_width = 0;
            ends_with_line_break = true;
            i += 1;
            continue;
        }

        if byte == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            lines += 1;
            current_width = 0;
            ends_with_line_break = true;
            i += 2;
            continue;
        }

        if !byte.is_ascii() || byte < 0x20 || byte == 0x7f {
            return None;
        }

        if current_width == max_width {
            lines += 1;
            current_width = 1;
        } else {
            current_width += 1;
        }
        ends_with_line_break = false;
        i += 1;
    }

    if ends_with_line_break && lines > 1 {
        lines -= 1;
    }

    Some(lines.max(1))
}

fn ascii_width_fast_path(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();

    if !bytes.iter().all(|b| b.is_ascii() && *b > 0x1f && *b < 0x7f) {
        return None;
    }

    Some(bytes.len())
}

/// Pad text to a specific width
pub fn pad_text(text: &str, width: usize, align: TextAlign) -> String {
    let text_width = measure_text_width(text);

    if text_width >= width {
        return text.to_string();
    }

    let padding = width - text_width;

    match align {
        TextAlign::Left => format!("{}{}", text, " ".repeat(padding)),
        TextAlign::Right => format!("{}{}", " ".repeat(padding), text),
        TextAlign::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
    }
}

/// Text alignment
#[derive(Debug, Clone, Copy, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Right,
    Center,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measure_ascii() {
        assert_eq!(measure_text_width("hello"), 5);
        assert_eq!(measure_text_width("hello world"), 11);
    }

    #[test]
    fn test_measure_unicode() {
        // Chinese characters are typically 2 cells wide
        assert_eq!(measure_text_width("你好"), 4);
        assert_eq!(measure_text_width("Hello 世界"), 10);
    }

    #[test]
    fn test_measure_text_dimensions() {
        let (w, h) = measure_text("hello\nworld");
        assert_eq!(w, 5);
        assert_eq!(h, 2);
    }

    #[test]
    fn test_wrap_text() {
        let wrapped = wrap_text("hello world", 6);
        assert!(wrapped.contains('\n'));
    }

    #[test]
    fn test_truncate_text() {
        let truncated = truncate_text("hello world", 8, "...");
        assert_eq!(truncated, "hello...");
    }

    #[test]
    fn test_truncate_start() {
        let truncated = truncate_start("hello world", 8, "...");
        assert_eq!(truncated, "...world");
    }

    #[test]
    fn test_truncate_middle() {
        let truncated = truncate_middle("hello world", 9, "...");
        assert_eq!(truncated, "hel...rld");
    }

    #[test]
    fn test_pad_text() {
        assert_eq!(pad_text("hi", 5, TextAlign::Left), "hi   ");
        assert_eq!(pad_text("hi", 5, TextAlign::Right), "   hi");
        assert_eq!(pad_text("hi", 5, TextAlign::Center), " hi  ");
    }

    #[test]
    fn test_grapheme_clusters_emoji() {
        // Family emoji (ZWJ sequence) - should be treated as 1 grapheme with width 2
        let family = "👨‍👩‍👧‍👦";
        let graphemes: Vec<&str> = family.graphemes(true).collect();
        assert_eq!(graphemes.len(), 1, "Family emoji should be 1 grapheme");
        // Note: Width may vary by terminal, but grapheme count should be 1
    }

    #[test]
    fn test_grapheme_clusters_combining() {
        // e + combining acute accent = 1 grapheme
        let combined = "é"; // This is e + combining acute (2 code points)
        let graphemes: Vec<&str> = combined.graphemes(true).collect();
        // Note: The actual behavior depends on the string encoding
        // If it's precomposed (1 code point), it's 1 grapheme
        // If it's decomposed (2 code points), it should still be 1 grapheme
        assert!(graphemes.len() <= 2); // Either 1 or at most 2
    }

    #[test]
    fn test_truncate_preserves_graphemes() {
        // Truncating should not split grapheme clusters
        let text = "hello 你好";
        let truncated = truncate_text(text, 8, "…");
        // Should truncate cleanly without splitting Chinese characters
        assert!(measure_text_width(&truncated) <= 8);
    }

    #[test]
    fn test_zero_width_characters() {
        // Zero-width joiner should have width 0
        let zwj = "\u{200D}"; // Zero Width Joiner
        assert_eq!(measure_text_width(zwj), 0);
    }

    #[test]
    fn test_count_wrapped_lines_handles_crlf_like_lines() {
        let text = "line1\r\nline2\r\n";
        assert_eq!(
            count_wrapped_lines_by_width(text, 80),
            text.lines().count().max(1)
        );
    }

    #[test]
    fn test_measure_text_ascii_crlf() {
        assert_eq!(measure_text("ab\r\nc"), (2, 2));
    }

    #[test]
    fn test_measure_text_trailing_newline_semantics() {
        assert_eq!(measure_text("ab\n"), (2, 1));
        assert_eq!(measure_text("ab\r\n"), (2, 1));
    }

    #[test]
    fn test_count_wrapped_lines_ascii_with_wrap() {
        assert_eq!(count_wrapped_lines_by_width("abcdef", 3), 2);
    }
}
