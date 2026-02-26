//! Output buffer for terminal rendering

use crate::core::{Color, Style};
use std::fmt::Write as FmtWrite;
use unicode_width::UnicodeWidthChar;

/// A styled character in the output grid
#[derive(Debug, Clone, Default)]
pub struct StyledChar {
    pub ch: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub inverse: bool,
}

impl StyledChar {
    pub fn new(ch: char) -> Self {
        Self {
            ch,
            ..Default::default()
        }
    }

    pub fn with_style(ch: char, style: &Style) -> Self {
        Self {
            ch,
            fg: style.color,
            bg: style.background_color,
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            strikethrough: style.strikethrough,
            dim: style.dim,
            inverse: style.inverse,
        }
    }

    /// Check if this char has any styling
    pub fn has_style(&self) -> bool {
        self.fg.is_some()
            || self.bg.is_some()
            || self.bold
            || self.italic
            || self.underline
            || self.strikethrough
            || self.dim
            || self.inverse
    }

    /// Check if two styled chars have the same style
    pub fn same_style(&self, other: &Self) -> bool {
        self.fg == other.fg
            && self.bg == other.bg
            && self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.strikethrough == other.strikethrough
            && self.dim == other.dim
            && self.inverse == other.inverse
    }
}

/// Clip region for overflow handling
#[derive(Debug, Clone)]
pub struct ClipRegion {
    pub x1: u16,
    pub y1: u16,
    pub x2: u16,
    pub y2: u16,
}

impl ClipRegion {
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x1 && x < self.x2 && y >= self.y1 && y < self.y2
    }
}

/// Output buffer that collects rendered content
pub struct Output {
    pub width: u16,
    pub height: u16,
    /// Flat grid storage for better cache locality (row-major order)
    grid: Vec<StyledChar>,
    clip_stack: Vec<ClipRegion>,
    /// Tracks which rows have been modified since last clear_dirty()
    dirty_rows: Vec<bool>,
    /// Quick check if any row is dirty
    any_dirty: bool,
}

impl Output {
    /// Create a new output buffer
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        let grid = vec![StyledChar::new(' '); size];
        Self {
            width,
            height,
            grid,
            clip_stack: Vec::new(),
            dirty_rows: vec![false; height as usize],
            any_dirty: false,
        }
    }

    /// Calculate flat index from (col, row) coordinates
    #[inline]
    fn index(&self, col: usize, row: usize) -> usize {
        row * (self.width as usize) + col
    }

    /// Get a reference to a cell at (col, row)
    #[inline]
    fn get(&self, col: usize, row: usize) -> Option<&StyledChar> {
        if col < self.width as usize && row < self.height as usize {
            Some(&self.grid[self.index(col, row)])
        } else {
            None
        }
    }

    /// Set a cell at (col, row)
    #[inline]
    fn set(&mut self, col: usize, row: usize, value: StyledChar) {
        if col < self.width as usize && row < self.height as usize {
            let idx = self.index(col, row);
            self.grid[idx] = value;
        }
    }

    /// Get an iterator over a row
    fn row_iter(&self, row: usize) -> impl Iterator<Item = &StyledChar> {
        let start = row * (self.width as usize);
        let end = start + (self.width as usize);
        self.grid[start..end].iter()
    }

    /// Get a reference to a cell at (col, row) - public for testing
    #[cfg(test)]
    pub fn cell_at(&self, col: usize, row: usize) -> Option<&StyledChar> {
        self.get(col, row)
    }

    /// Check if any row has been modified
    pub fn is_dirty(&self) -> bool {
        self.any_dirty
    }

    /// Check if a specific row has been modified
    pub fn is_row_dirty(&self, row: usize) -> bool {
        self.dirty_rows.get(row).copied().unwrap_or(false)
    }

    /// Clear all dirty flags
    pub fn clear_dirty(&mut self) {
        self.dirty_rows.fill(false);
        self.any_dirty = false;
    }

    /// Get indices of all dirty rows
    pub fn dirty_row_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.dirty_rows
            .iter()
            .enumerate()
            .filter_map(|(i, &dirty)| if dirty { Some(i) } else { None })
    }

    /// Render only the dirty rows, returning (row_index, rendered_line) pairs
    pub fn render_dirty_rows(&self) -> Vec<(usize, String)> {
        self.assert_no_active_clips("render_dirty_rows");
        self.dirty_row_indices()
            .map(|row_idx| {
                let line = self.render_row(row_idx);
                (row_idx, line)
            })
            .collect()
    }

    /// Render a single row to a string with ANSI codes
    fn render_row(&self, row_idx: usize) -> String {
        if row_idx >= self.height as usize {
            return String::new();
        }

        let mut last_content_idx = 0;
        for (i, cell) in self.row_iter(row_idx).enumerate() {
            if cell.ch != '\0' && (cell.ch != ' ' || cell.has_style()) {
                last_content_idx = i + 1;
            }
        }

        let mut line = String::new();
        let mut current_style: Option<StyledChar> = None;

        for (i, cell) in self.row_iter(row_idx).enumerate() {
            if i >= last_content_idx {
                break;
            }

            if cell.ch == '\0' {
                continue;
            }

            let need_style_change = match &current_style {
                None => cell.has_style(),
                Some(prev) => !cell.same_style(prev),
            };

            if need_style_change {
                if current_style.is_some() {
                    line.push_str("\x1b[0m");
                }
                self.apply_style(&mut line, cell);
                current_style = Some(cell.clone());
            }

            line.push(cell.ch);
        }

        if current_style.is_some() {
            line.push_str("\x1b[0m");
        }

        line
    }

    /// Mark a row as dirty
    #[inline]
    fn mark_dirty(&mut self, row: usize) {
        if row < self.dirty_rows.len() {
            self.dirty_rows[row] = true;
            self.any_dirty = true;
        }
    }

    /// Write text at position with style
    pub fn write(&mut self, x: u16, y: u16, text: &str, style: &Style) {
        let mut col = x as usize;
        let row = y as usize;

        if row >= self.height as usize {
            return;
        }

        // Mark row as dirty before any modifications
        self.mark_dirty(row);

        let width = self.width as usize;

        for ch in text.chars() {
            if ch == '\n' {
                break;
            }

            if col >= width {
                break;
            }

            // Check clip region
            let char_width = ch.width().unwrap_or(1);
            if let Some(clip) = self.clip_stack.last()
                && !clip.contains(col as u16, row as u16)
            {
                col += char_width;
                continue;
            }

            self.write_char_at(col, row, ch, style);
            col += char_width;
        }
    }

    /// Write a single character at position
    pub fn write_char(&mut self, x: u16, y: u16, ch: char, style: &Style) {
        let col = x as usize;
        let row = y as usize;

        if row >= self.height as usize || col >= self.width as usize {
            return;
        }

        // Mark row as dirty before any modifications
        self.mark_dirty(row);

        // Check clip region
        if let Some(clip) = self.clip_stack.last()
            && !clip.contains(x, y)
        {
            return;
        }

        self.write_char_at(col, row, ch, style);
    }

    /// Core character placement logic handling wide-char boundaries and placeholders
    fn write_char_at(&mut self, col: usize, row: usize, ch: char, style: &Style) {
        let width = self.width as usize;
        let char_width = ch.width().unwrap_or(1);

        // Handle wide character at buffer boundary - skip if it won't fit
        if char_width == 2 && col + 1 >= width {
            self.set(col, row, StyledChar::with_style(' ', style));
            return;
        }

        // Handle overwriting wide character's second half (placeholder)
        if let Some(cell) = self.get(col, row) {
            if cell.ch == '\0' && col > 0 {
                self.set(col - 1, row, StyledChar::new(' '));
            }
        }

        // Handle overwriting wide character's first half
        if let Some(cell) = self.get(col, row) {
            let old_char_width = cell.ch.width().unwrap_or(1);
            if old_char_width == 2 && col + 1 < width {
                self.set(col + 1, row, StyledChar::new(' '));
            }
        }

        self.set(col, row, StyledChar::with_style(ch, style));

        // For wide characters (width=2), mark the next cell as a placeholder
        if char_width == 2 && col + 1 < width {
            if let Some(next_cell) = self.get(col + 1, row) {
                if next_cell.ch != '\0' {
                    let next_char_width = next_cell.ch.width().unwrap_or(1);
                    if next_char_width == 2 && col + 2 < width {
                        self.set(col + 2, row, StyledChar::new(' '));
                    }
                }
            }
            self.set(col + 1, row, StyledChar::new('\0'));
        }
    }

    /// Fill a rectangle with a character
    pub fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, ch: char, style: &Style) {
        for row in y..(y + height).min(self.height) {
            for col in x..(x + width).min(self.width) {
                self.write_char(col, row, ch, style);
            }
        }
    }

    /// Push a clip region
    pub fn clip(&mut self, region: ClipRegion) {
        assert!(
            region.x1 <= region.x2 && region.y1 <= region.y2,
            "Invalid clip region: min > max"
        );
        self.clip_stack.push(region);
    }

    /// Pop the current clip region
    pub fn unclip(&mut self) {
        assert!(
            self.clip_stack.pop().is_some(),
            "Output::unclip called with an empty clip stack"
        );
    }

    /// Return current clip stack depth.
    ///
    /// A non-zero depth after a render pass usually means clip push/pop calls
    /// are unbalanced in the renderer.
    pub(crate) fn clip_depth(&self) -> usize {
        self.clip_stack.len()
    }

    fn assert_no_active_clips(&self, method: &str) {
        debug_assert!(
            self.clip_stack.is_empty(),
            "Output::{} called with an unbalanced clip stack (depth={})",
            method,
            self.clip_stack.len()
        );
    }

    /// Convert the buffer to a string with ANSI codes
    pub fn render(&self) -> String {
        self.assert_no_active_clips("render");
        let mut lines: Vec<String> = if self.is_dirty() {
            let dirty_rows = self.render_dirty_rows();
            if dirty_rows.is_empty() {
                Vec::new()
            } else {
                let mut sparse =
                    vec![String::new(); dirty_rows.last().map(|(row, _)| row + 1).unwrap_or(0)];
                for (row, line) in dirty_rows {
                    sparse[row] = line;
                }
                sparse
            }
        } else {
            // Preserve previous behavior when dirty flags were externally reset.
            (0..self.height as usize)
                .map(|row_idx| self.render_row(row_idx))
                .collect()
        };

        // Remove trailing empty lines
        while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }

        lines.join("\r\n")
    }

    /// Convert the buffer to a string, preserving all lines (including empty trailing lines)
    ///
    /// This is useful for inline mode rendering where cursor positioning depends on
    /// consistent line counts between frames. Use `render()` for normal rendering
    /// that strips trailing empty lines.
    pub fn render_fixed_height(&self) -> String {
        self.assert_no_active_clips("render_fixed_height");
        let lines: Vec<String> = (0..self.height as usize)
            .map(|row_idx| self.render_row(row_idx))
            .collect();

        // NOTE: Unlike render(), we do NOT strip trailing empty lines here
        // This preserves the exact line count for fixed-height layouts

        lines.join("\r\n")
    }

    fn apply_style(&self, result: &mut String, cell: &StyledChar) {
        let mut codes: Vec<u8> = Vec::new();

        if cell.bold {
            codes.push(1);
        }
        if cell.dim {
            codes.push(2);
        }
        if cell.italic {
            codes.push(3);
        }
        if cell.underline {
            codes.push(4);
        }
        if cell.inverse {
            codes.push(7);
        }
        if cell.strikethrough {
            codes.push(9);
        }

        if let Some(fg) = cell.fg {
            self.color_to_ansi(fg, false, &mut codes);
        }

        if let Some(bg) = cell.bg {
            self.color_to_ansi(bg, true, &mut codes);
        }

        if !codes.is_empty() {
            result.push_str("\x1b[");
            for (i, code) in codes.iter().enumerate() {
                if i > 0 {
                    result.push(';');
                }
                let _ = write!(result, "{}", code);
            }
            result.push('m');
        }
    }

    fn color_to_ansi(&self, color: Color, background: bool, codes: &mut Vec<u8>) {
        color.push_ansi_codes(background, codes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_creation() {
        let output = Output::new(80, 24);
        assert_eq!(output.width, 80);
        assert_eq!(output.height, 24);
    }

    #[test]
    fn test_write_text() {
        let mut output = Output::new(80, 24);
        output.write(0, 0, "Hello", &Style::default());

        assert_eq!(output.cell_at(0, 0).unwrap().ch, 'H');
        assert_eq!(output.cell_at(4, 0).unwrap().ch, 'o');
    }

    #[test]
    fn test_styled_output() {
        let mut output = Output::new(80, 24);
        let mut style = Style::default();
        style.color = Some(Color::Green);
        style.bold = true;

        output.write(0, 0, "Test", &style);

        let rendered = output.render();
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn test_wide_char_placeholder() {
        let mut output = Output::new(80, 24);
        output.write(0, 0, "你好", &Style::default());

        // '你' at position 0, placeholder at position 1
        assert_eq!(output.cell_at(0, 0).unwrap().ch, '你');
        assert_eq!(output.cell_at(1, 0).unwrap().ch, '\0');
        // '好' at position 2, placeholder at position 3
        assert_eq!(output.cell_at(2, 0).unwrap().ch, '好');
        assert_eq!(output.cell_at(3, 0).unwrap().ch, '\0');
    }

    #[test]
    fn test_overwrite_wide_char_placeholder() {
        let mut output = Output::new(80, 24);
        // Write a wide char first
        output.write(0, 0, "你", &Style::default());
        assert_eq!(output.cell_at(0, 0).unwrap().ch, '你');
        assert_eq!(output.cell_at(1, 0).unwrap().ch, '\0');

        // Overwrite the placeholder with a narrow char
        output.write_char(1, 0, 'X', &Style::default());

        // The wide char should be replaced with space (broken)
        assert_eq!(output.cell_at(0, 0).unwrap().ch, ' ');
        assert_eq!(output.cell_at(1, 0).unwrap().ch, 'X');
    }

    #[test]
    fn test_overwrite_wide_char_first_half() {
        let mut output = Output::new(80, 24);
        // Write a wide char first
        output.write(0, 0, "你", &Style::default());
        assert_eq!(output.cell_at(0, 0).unwrap().ch, '你');
        assert_eq!(output.cell_at(1, 0).unwrap().ch, '\0');

        // Overwrite the first half with a narrow char
        output.write_char(0, 0, 'X', &Style::default());

        // The wide char's placeholder should be cleared
        assert_eq!(output.cell_at(0, 0).unwrap().ch, 'X');
        assert_eq!(output.cell_at(1, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_wide_char_render_no_duplicate() {
        let mut output = Output::new(80, 24);
        output.write(0, 0, "你好世界", &Style::default());

        let rendered = output.render();
        // Should contain exactly these 4 chars, no placeholders visible
        assert_eq!(rendered, "你好世界");
    }

    #[test]
    fn test_raw_mode_line_endings() {
        // Raw mode requires CRLF line endings, not just LF
        let mut output = Output::new(40, 5);
        output.write(0, 0, "Line 1", &Style::default());
        output.write(0, 1, "Line 2", &Style::default());
        output.write(0, 2, "Line 3", &Style::default());

        let rendered = output.render();

        // Must use CRLF for raw mode compatibility
        assert!(
            rendered.contains("\r\n"),
            "Output must use CRLF line endings for raw mode"
        );

        // Count that we don't have standalone LF (without CR before it)
        let lines: Vec<&str> = rendered.split("\r\n").collect();
        assert!(lines.len() >= 3, "Should have at least 3 lines");

        // Verify no standalone LF within lines
        for line in &lines {
            assert!(
                !line.contains('\n'),
                "Should not have standalone LF within lines"
            );
        }
    }

    #[test]
    fn test_line_alignment_in_output() {
        // Test that multi-line output will render with correct alignment
        let mut output = Output::new(20, 3);
        output.write(0, 0, "AAAA", &Style::default());
        output.write(0, 1, "BBBB", &Style::default());
        output.write(0, 2, "CCCC", &Style::default());

        let rendered = output.render();
        let lines: Vec<&str> = rendered.split("\r\n").collect();

        assert_eq!(lines[0], "AAAA");
        assert_eq!(lines[1], "BBBB");
        assert_eq!(lines[2], "CCCC");
    }

    #[test]
    fn test_wide_char_at_boundary() {
        // Wide char at end of buffer should be replaced with space
        let mut output = Output::new(5, 1);
        output.write(3, 0, "你", &Style::default());

        // Position 3 should be a space, position 4 is at boundary
        assert_eq!(output.cell_at(3, 0).unwrap().ch, '你');
        assert_eq!(output.cell_at(4, 0).unwrap().ch, '\0');

        // Now test when wide char would extend past buffer
        let mut output2 = Output::new(5, 1);
        output2.write(4, 0, "你", &Style::default());

        // Should write a space instead since wide char won't fit
        assert_eq!(output2.cell_at(4, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_wide_char_at_exact_boundary() {
        // Test when wide char is at the last valid position
        let mut output = Output::new(4, 1);
        output.write(2, 0, "你", &Style::default());

        // Wide char at position 2-3 should fit exactly
        assert_eq!(output.cell_at(2, 0).unwrap().ch, '你');
        assert_eq!(output.cell_at(3, 0).unwrap().ch, '\0');
    }

    #[test]
    fn test_dirty_tracking_initial_state() {
        let output = Output::new(80, 24);
        assert!(!output.is_dirty());
        assert!(!output.is_row_dirty(0));
    }

    #[test]
    fn test_dirty_tracking_after_write() {
        let mut output = Output::new(80, 24);
        output.write(0, 5, "Hello", &Style::default());

        assert!(output.is_dirty());
        assert!(output.is_row_dirty(5));
        assert!(!output.is_row_dirty(0));
        assert!(!output.is_row_dirty(6));
    }

    #[test]
    fn test_dirty_tracking_after_write_char() {
        let mut output = Output::new(80, 24);
        output.write_char(10, 3, 'X', &Style::default());

        assert!(output.is_dirty());
        assert!(output.is_row_dirty(3));
        assert!(!output.is_row_dirty(2));
    }

    #[test]
    fn test_dirty_tracking_clear() {
        let mut output = Output::new(80, 24);
        output.write(0, 0, "Test", &Style::default());
        output.write(0, 5, "Test", &Style::default());

        assert!(output.is_dirty());
        assert!(output.is_row_dirty(0));
        assert!(output.is_row_dirty(5));

        output.clear_dirty();

        assert!(!output.is_dirty());
        assert!(!output.is_row_dirty(0));
        assert!(!output.is_row_dirty(5));
    }

    #[test]
    fn test_dirty_row_indices() {
        let mut output = Output::new(80, 24);
        output.write(0, 1, "A", &Style::default());
        output.write(0, 3, "B", &Style::default());
        output.write(0, 7, "C", &Style::default());

        let dirty: Vec<usize> = output.dirty_row_indices().collect();
        assert_eq!(dirty, vec![1, 3, 7]);
    }

    #[test]
    fn test_render_dirty_rows() {
        let mut output = Output::new(80, 24);
        output.write(0, 0, "Line 0", &Style::default());
        output.write(0, 2, "Line 2", &Style::default());

        let dirty_rows = output.render_dirty_rows();
        assert_eq!(dirty_rows.len(), 2);
        assert_eq!(dirty_rows[0].0, 0);
        assert_eq!(dirty_rows[0].1, "Line 0");
        assert_eq!(dirty_rows[1].0, 2);
        assert_eq!(dirty_rows[1].1, "Line 2");
    }

    #[test]
    fn test_render_after_clear_dirty_preserves_content() {
        let mut output = Output::new(10, 2);
        output.write(0, 0, "A", &Style::default());
        output.clear_dirty();
        assert_eq!(output.render(), "A");
    }

    #[test]
    fn test_render_sparse_dirty_rows_preserves_line_gaps() {
        let mut output = Output::new(10, 4);
        output.write(0, 2, "C", &Style::default());
        assert_eq!(output.render(), "\r\n\r\nC");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Output::unclip called with an empty clip stack")]
    fn test_unclip_panics_when_stack_is_empty_in_debug() {
        let mut output = Output::new(10, 5);
        output.unclip();
    }

    #[test]
    fn test_clip_depth_tracks_push_and_pop() {
        let mut output = Output::new(10, 5);
        assert_eq!(output.clip_depth(), 0);

        output.clip(ClipRegion {
            x1: 0,
            y1: 0,
            x2: 5,
            y2: 5,
        });
        assert_eq!(output.clip_depth(), 1);

        output.unclip();
        assert_eq!(output.clip_depth(), 0);
    }

    #[test]
    #[should_panic(expected = "Output::render called with an unbalanced clip stack")]
    fn test_render_panics_with_active_clip_stack() {
        let mut output = Output::new(10, 5);
        output.clip(ClipRegion {
            x1: 0,
            y1: 0,
            x2: 5,
            y2: 5,
        });
        let _ = output.render();
    }
}
