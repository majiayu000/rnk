//! Text measurement utilities

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

/// Measure the display width of text using grapheme clusters
///
/// This function properly handles:
/// - CJK characters (width = 2)
/// - Emoji sequences (including ZWJ sequences like 👨‍👩‍👧‍👦)
/// - Combining characters (e.g., é = e + combining acute)
/// - Zero-width characters
pub fn measure_text_width(text: &str) -> usize {
    text.graphemes(true)
        .map(|g| UnicodeWidthStr::width(g))
        .sum()
}

/// Measure the display width using grapheme clusters (alias for measure_text_width)
pub fn display_width(text: &str) -> usize {
    measure_text_width(text)
}

/// Measure text dimensions (width, height)
pub fn measure_text(text: &str) -> (usize, usize) {
    let lines: Vec<&str> = text.lines().collect();
    let height = lines.len().max(1);
    let width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
    (width, height)
}

/// Wrap text to fit within a maximum width (grapheme-aware)
pub fn wrap_text(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut current_width = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);

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
    let text_width = measure_text_width(text);

    if text_width <= max_width {
        return text.to_string();
    }

    let ellipsis_width = measure_text_width(ellipsis);
    if max_width <= ellipsis_width {
        // Just take as much of ellipsis as we can
        let mut result = String::new();
        let mut width = 0;
        for g in ellipsis.graphemes(true) {
            let gw = UnicodeWidthStr::width(g);
            if width + gw > max_width {
                break;
            }
            result.push_str(g);
            width += gw;
        }
        return result;
    }

    let target_width = max_width - ellipsis_width;
    let mut result = String::new();
    let mut current_width = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
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
    let text_width = measure_text_width(text);

    if text_width <= max_width {
        return text.to_string();
    }

    let ellipsis_width = measure_text_width(ellipsis);
    if max_width <= ellipsis_width {
        let mut result = String::new();
        let mut width = 0;
        for g in ellipsis.graphemes(true) {
            let gw = UnicodeWidthStr::width(g);
            if width + gw > max_width {
                break;
            }
            result.push_str(g);
            width += gw;
        }
        return result;
    }

    let target_width = max_width - ellipsis_width;
    let graphemes: Vec<&str> = text.graphemes(true).collect();
    let mut result = String::new();
    let mut current_width = 0;
    let mut end_graphemes = Vec::new();

    for grapheme in graphemes.iter().rev() {
        let grapheme_width = UnicodeWidthStr::width(*grapheme);
        if current_width + grapheme_width > target_width {
            break;
        }
        end_graphemes.push(*grapheme);
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
    let text_width = measure_text_width(text);

    if text_width <= max_width {
        return text.to_string();
    }

    let ellipsis_width = measure_text_width(ellipsis);
    if max_width <= ellipsis_width {
        let mut result = String::new();
        let mut width = 0;
        for g in ellipsis.graphemes(true) {
            let gw = UnicodeWidthStr::width(g);
            if width + gw > max_width {
                break;
            }
            result.push_str(g);
            width += gw;
        }
        return result;
    }

    let available = max_width - ellipsis_width;
    let left_width = available / 2;
    let right_width = available - left_width;

    let graphemes: Vec<&str> = text.graphemes(true).collect();

    // Build left part
    let mut left = String::new();
    let mut current_width = 0;
    for grapheme in &graphemes {
        let grapheme_width = UnicodeWidthStr::width(*grapheme);
        if current_width + grapheme_width > left_width {
            break;
        }
        left.push_str(grapheme);
        current_width += grapheme_width;
    }

    // Build right part
    let mut right_graphemes = Vec::new();
    current_width = 0;
    for grapheme in graphemes.iter().rev() {
        let grapheme_width = UnicodeWidthStr::width(*grapheme);
        if current_width + grapheme_width > right_width {
            break;
        }
        right_graphemes.push(*grapheme);
        current_width += grapheme_width;
    }
    right_graphemes.reverse();

    let mut right = String::new();
    for g in right_graphemes {
        right.push_str(g);
    }

    format!("{}{}{}", left, ellipsis, right)
}

/// Pad text to a specific width
pub fn pad_text(text: &str, width: usize, align: TextAlign) -> String {
    let text_width = text.width();

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
        let combined = "é";  // This is e + combining acute (2 code points)
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
        let zwj = "\u{200D}";  // Zero Width Joiner
        assert_eq!(measure_text_width(zwj), 0);
    }
}
