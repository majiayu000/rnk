//! Single source of truth for how a text block occupies terminal rows.
//!
//! Measurement and rendering must agree on where lines break. Before this
//! module they did not: layout counted wrapped rows with one algorithm while
//! [`Output::write`](crate::renderer::output::Output::write) stopped at the
//! first newline or at the right edge, so any content past that point was
//! silently dropped even though layout had reserved height for it.
//!
//! Both sides now call [`flow_text`] and consume the same rows.

use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use crate::core::TextWrap;

use super::measure::grapheme_width;

/// Columns between tab stops, matching the terminal default.
const TAB_STOP: usize = 8;

/// The rows a text block occupies at a given content width.
///
/// Row order matches source order. A block that is present but empty produces
/// exactly one empty row, so its height is still determinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextFlow {
    rows: Vec<String>,
}

impl TextFlow {
    /// Rows in source order.
    pub(crate) fn rows(&self) -> &[String] {
        &self.rows
    }

    /// Number of terminal rows this block occupies.
    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Width in cells of the widest row.
    pub(crate) fn max_row_width(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row_width(row))
            .max()
            .unwrap_or(0)
    }
}

fn row_width(row: &str) -> usize {
    row.graphemes(true).map(grapheme_width).sum()
}

/// Split `text` into logical lines on hard breaks.
///
/// `\r\n` counts once. A trailing break does not produce a final empty row,
/// which preserves the existing visible-line contract.
fn logical_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&text[start..i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                lines.push(&text[start..i]);
                // CRLF is one break, not two.
                i += if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = i;
            }
            _ => i += 1,
        }
    }

    if start < bytes.len() {
        lines.push(&text[start..]);
    } else if lines.is_empty() {
        lines.push("");
    }

    lines
}

/// Map a control scalar to a terminal-safe stand-in, or `None` if it is text.
///
/// Source text is untrusted. A terminal executes any escape sequence it
/// receives, so an `ESC [ 2 J` embedded in a rendered message would clear the
/// user's screen, and other C0/C1 bytes can move the cursor or start an OSC
/// command. Each control becomes exactly one printable cell so that the
/// substitution cannot shift anything else on the row.
///
/// C0 (which includes ESC) has a dedicated Unicode control picture per scalar;
/// DEL has `␡`. C1 has no picture block, so it falls back to the replacement
/// character.
pub(crate) fn sanitize_control(ch: char) -> Option<char> {
    match ch {
        '\u{0}'..='\u{1f}' => char::from_u32(0x2400 + ch as u32),
        '\u{7f}' => Some('\u{2421}'),
        '\u{80}'..='\u{9f}' => Some('\u{fffd}'),
        _ => None,
    }
}

/// Expand tabs and neutralise controls in one already-hard-break-free line.
///
/// Borrows unchanged when the line is plain, which is the common case.
fn sanitize_line(line: &str) -> Cow<'_, str> {
    if !line.chars().any(|ch| sanitize_control(ch).is_some()) {
        return Cow::Borrowed(line);
    }

    let mut out = String::with_capacity(line.len());
    let mut width = 0usize;

    for ch in line.chars() {
        if ch == '\t' {
            // Advance to the next tab stop, never zero columns.
            let advance = TAB_STOP - (width % TAB_STOP);
            for _ in 0..advance {
                out.push(' ');
            }
            width += advance;
        } else if let Some(safe) = sanitize_control(ch) {
            out.push(safe);
            width += 1;
        } else {
            width += ch.width().unwrap_or(0);
            out.push(ch);
        }
    }

    Cow::Owned(out)
}

/// Widest row `text` occupies when nothing forces a break: its max-content width.
///
/// This must come from the same flow the renderer draws. Measuring the raw
/// string instead sizes the node from characters that never reach a cell — a
/// tab is one scalar but up to eight columns, so the node came out too narrow
/// and layout then wrapped text the renderer drew on a single row.
pub(crate) fn intrinsic_width(text: &str) -> usize {
    flow_text(text, usize::MAX, TextWrap::Wrap).max_row_width()
}

/// Lay `text` out into rows at `max_width` cells under `wrap`.
///
/// `max_width == 0` yields one empty row per logical line: there is nowhere to
/// place a cell, but the row count stays determinate.
pub(crate) fn flow_text(text: &str, max_width: usize, wrap: TextWrap) -> TextFlow {
    // Hard breaks are consumed first, then tabs and controls, so that only
    // printable graphemes reach the wrapper and the renderer.
    let lines: Vec<Cow<'_, str>> = logical_lines(text).into_iter().map(sanitize_line).collect();

    if max_width == 0 {
        return TextFlow {
            rows: vec![String::new(); lines.len()],
        };
    }

    let mut rows = Vec::with_capacity(lines.len());
    for line in &lines {
        match wrap {
            TextWrap::Wrap => wrap_line(line, max_width, &mut rows),
            _ => rows.push(fit_single_row(line, max_width)),
        }
    }

    TextFlow { rows }
}

/// Greedily fill rows, preferring to break at whitespace.
///
/// A word longer than `max_width` is broken mid-word rather than overflowing:
/// a terminal has no horizontal scroll to escape to, and dropping the tail
/// would lose content.
fn wrap_line(line: &str, max_width: usize, rows: &mut Vec<String>) {
    if line.is_empty() {
        rows.push(String::new());
        return;
    }

    let mut current = String::new();
    let mut current_width = 0usize;
    // Trailing whitespace already placed on the current row. It is only
    // materialised once a non-space follows, so a break at a space does not
    // leave the space dangling at the edge.
    let mut pending_space = String::new();
    let mut pending_space_width = 0usize;

    for word in split_keeping_whitespace(line) {
        if word.chars().all(char::is_whitespace) {
            pending_space.push_str(word);
            pending_space_width += row_width(word);
            continue;
        }

        let word_width = row_width(word);

        if current_width + pending_space_width + word_width <= max_width {
            current.push_str(&pending_space);
            current_width += pending_space_width;
            pending_space.clear();
            pending_space_width = 0;
            current.push_str(word);
            current_width += word_width;
            continue;
        }

        // Word does not fit after the pending space; start a new row.
        if !current.is_empty() {
            rows.push(std::mem::take(&mut current));
            current_width = 0;
        }
        pending_space.clear();
        pending_space_width = 0;

        if word_width <= max_width {
            current.push_str(word);
            current_width = word_width;
            continue;
        }

        // Longer than a full row: break it across rows on grapheme boundaries.
        for grapheme in word.graphemes(true) {
            let g_width = grapheme_width(grapheme);
            if current_width + g_width > max_width && !current.is_empty() {
                rows.push(std::mem::take(&mut current));
                current_width = 0;
            }
            current.push_str(grapheme);
            current_width += g_width;
        }
    }

    // Whitespace at the very end of the line was never followed by a word, so
    // it was never flushed. Keep it: it is content, and dropping it would
    // change strings that deliberately pad to a fixed width.
    current.push_str(&pending_space);

    rows.push(current);
}

/// Split into alternating runs of whitespace and non-whitespace, keeping both.
fn split_keeping_whitespace(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_space: Option<bool> = None;

    for (idx, ch) in line.char_indices() {
        let is_space = ch.is_whitespace();
        match in_space {
            Some(prev) if prev == is_space => {}
            Some(_) => {
                parts.push(&line[start..idx]);
                start = idx;
            }
            None => {}
        }
        in_space = Some(is_space);
    }

    if start < line.len() {
        parts.push(&line[start..]);
    }

    parts
}

/// Take as many leading graphemes as fit in `max_width` cells.
///
/// A wide grapheme that would straddle the edge is dropped rather than split,
/// so no half-character is ever emitted.
fn fit_single_row(line: &str, max_width: usize) -> String {
    let mut row = String::new();
    let mut width = 0usize;

    for grapheme in line.graphemes(true) {
        let g_width = grapheme_width(grapheme);
        if width + g_width > max_width {
            break;
        }
        row.push_str(grapheme);
        width += g_width;
    }

    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str, width: usize, wrap: TextWrap) -> Vec<String> {
        flow_text(text, width, wrap).rows().to_vec()
    }

    #[test]
    fn wrapped_text_keeps_every_word() {
        assert_eq!(
            rows("aaaa bbbb cccc dddd", 10, TextWrap::Wrap),
            vec!["aaaa bbbb", "cccc dddd"]
        );
    }

    #[test]
    fn measure_and_render_agree_on_row_count() {
        let flow = flow_text("aaaa bbbb cccc dddd", 10, TextWrap::Wrap);
        assert_eq!(flow.row_count(), flow.rows().len());
        assert_eq!(flow.row_count(), 2);
    }

    #[test]
    fn word_longer_than_row_is_broken_not_dropped() {
        assert_eq!(
            rows("abcdefghijkl", 6, TextWrap::Wrap),
            vec!["abcdef", "ghijkl"]
        );
    }

    #[test]
    fn hard_breaks_split_rows_and_crlf_counts_once() {
        assert_eq!(
            rows("a\r\nb\nc\rd", 10, TextWrap::Wrap),
            vec!["a", "b", "c", "d"]
        );
    }

    #[test]
    fn consecutive_hard_breaks_keep_the_blank_row() {
        assert_eq!(rows("a\n\nb", 10, TextWrap::Wrap), vec!["a", "", "b"]);
    }

    #[test]
    fn trailing_whitespace_survives() {
        // Fixed-width padding relies on this; dropping it silently changes
        // rendered output.
        assert_eq!(rows("ab ", 10, TextWrap::Wrap), vec!["ab "]);
        assert_eq!(rows("a  ", 10, TextWrap::Wrap), vec!["a  "]);
    }

    #[test]
    fn trailing_break_does_not_add_a_final_row() {
        assert_eq!(rows("a\nb\n", 10, TextWrap::Wrap), vec!["a", "b"]);
    }

    #[test]
    fn empty_text_is_one_empty_row() {
        assert_eq!(rows("", 10, TextWrap::Wrap), vec![""]);
        assert_eq!(flow_text("", 10, TextWrap::Wrap).row_count(), 1);
    }

    #[test]
    fn zero_width_keeps_row_count_but_places_no_cells() {
        let flow = flow_text("a\nb", 0, TextWrap::Wrap);
        assert_eq!(flow.row_count(), 2);
        assert_eq!(flow.max_row_width(), 0);
    }

    #[test]
    fn wide_graphemes_are_never_split_across_rows() {
        // Each CJK ideograph is two cells, so three fit in a six-cell row.
        assert_eq!(rows("你好世界", 6, TextWrap::Wrap), vec!["你好世", "界"]);
    }

    #[test]
    fn a_wide_grapheme_straddling_the_edge_is_not_half_written() {
        // Width 5 cannot hold a third two-cell grapheme.
        let flow = flow_text("你好世", 5, TextWrap::Wrap);
        assert!(flow.rows().iter().all(|row| row_width(row) <= 5));
        assert_eq!(flow.rows().concat(), "你好世");
    }

    #[test]
    fn truncate_keeps_one_row_per_logical_line() {
        assert_eq!(
            rows("aaaa bbbb cccc", 6, TextWrap::Truncate),
            vec!["aaaa b"]
        );
        assert_eq!(rows("ab\ncd", 6, TextWrap::Truncate), vec!["ab", "cd"]);
    }

    #[test]
    fn rows_exceed_the_limit_only_for_a_single_oversized_grapheme() {
        // A two-cell grapheme has nowhere to go at width 1. Splitting it would
        // emit half a character and dropping it would lose content, so it keeps
        // its own row and the renderer clips at the terminal edge.
        for text in ["aaaa bbbb cccc dddd", "abcdefghijkl", "你好世界 hello"] {
            for width in 1..=12 {
                let flow = flow_text(text, width, TextWrap::Wrap);
                for row in flow.rows() {
                    if row_width(row) <= width {
                        continue;
                    }
                    assert_eq!(
                        row.graphemes(true).count(),
                        1,
                        "text {text:?} at width {width} overflowed with a multi-grapheme row {row:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn control_scalars_never_survive_the_flow() {
        // An ESC reaching the terminal is executed, so this exact input would
        // otherwise clear the user's screen mid-render.
        let flow = flow_text("hi\u{1b}[2Jgone", 40, TextWrap::Wrap);
        let out = flow.rows().concat();
        assert!(!out.contains('\u{1b}'), "ESC survived in {out:?}");
        assert_eq!(out, "hi␛[2Jgone");
    }

    #[test]
    fn each_control_class_gets_its_own_safe_stand_in() {
        assert_eq!(sanitize_control('\u{1b}'), Some('␛'));
        assert_eq!(sanitize_control('\u{0}'), Some('␀'));
        assert_eq!(sanitize_control('\u{7}'), Some('␇'));
        assert_eq!(sanitize_control('\u{7f}'), Some('␡'));
        assert_eq!(sanitize_control('\u{9b}'), Some('\u{fffd}'));
        // Ordinary text, including wide and combining scalars, is untouched.
        assert_eq!(sanitize_control('a'), None);
        assert_eq!(sanitize_control('世'), None);
        assert_eq!(sanitize_control('\u{301}'), None);
    }

    #[test]
    fn a_control_occupies_exactly_one_cell_so_the_row_does_not_shift() {
        // Substitution must be width-preserving, or every later column on the
        // row would be off by the difference.
        let flow = flow_text("ab\u{7}cd", 40, TextWrap::Wrap);
        assert_eq!(flow.max_row_width(), 5);
    }

    #[test]
    fn tabs_expand_to_the_next_tab_stop() {
        assert_eq!(rows("a\tb", 40, TextWrap::Wrap), vec!["a       b"]);
        // Already on a stop: a full run, not zero columns.
        assert_eq!(
            rows("12345678\tx", 40, TextWrap::Wrap),
            vec!["12345678        x"]
        );
        // Stops are measured in cells, so a wide grapheme counts as two.
        assert_eq!(rows("世界\tx", 40, TextWrap::Wrap), vec!["世界    x"]);
    }

    #[test]
    fn tab_expansion_happens_before_wrapping() {
        // The expanded spaces are what has to fit, not the single tab scalar.
        assert_eq!(rows("a\tb", 8, TextWrap::Wrap), vec!["a", "b"]);
    }

    #[test]
    fn plain_text_is_not_reallocated() {
        assert!(matches!(sanitize_line("plain 世界 text"), Cow::Borrowed(_)));
        assert!(matches!(sanitize_line("has\ttab"), Cow::Owned(_)));
    }

    #[test]
    fn hard_breaks_are_consumed_before_sanitizing() {
        // LF and CR are C0, but they are structure, not payload: they must
        // still split rows rather than turn into ␊ / ␍ pictures.
        assert_eq!(
            rows("a\r\nb\nc\rd", 10, TextWrap::Wrap),
            vec!["a", "b", "c", "d"]
        );
    }

    #[test]
    fn wrapping_preserves_all_non_whitespace_content() {
        let text = "the quick brown fox jumps over the lazy dog";
        for width in 3..=20 {
            let flow = flow_text(text, width, TextWrap::Wrap);
            let flowed: String = flow.rows().concat().split_whitespace().collect();
            let original: String = text.split_whitespace().collect();
            assert_eq!(flowed, original, "content lost at width {width}");
        }
    }
}
