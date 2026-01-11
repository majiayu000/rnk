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
    grid: Vec<Vec<StyledChar>>,
    clip_stack: Vec<ClipRegion>,
}

impl Output {
    /// Create a new output buffer
    pub fn new(width: u16, height: u16) -> Self {
        let grid = vec![vec![StyledChar::new(' '); width as usize]; height as usize];
        Self {
            width,
            height,
            grid,
            clip_stack: Vec::new(),
        }
    }

    /// Write text at position with style
    pub fn write(&mut self, x: u16, y: u16, text: &str, style: &Style) {
        let mut col = x as usize;
        let row = y as usize;

        if row >= self.grid.len() {
            return;
        }

        for ch in text.chars() {
            if ch == '\n' {
                break;
            }

            if col >= self.grid[row].len() {
                break;
            }

            let char_width = ch.width().unwrap_or(1);

            // Handle wide character at buffer boundary - skip if it won't fit
            if char_width == 2 && col + 1 >= self.grid[row].len() {
                // Wide char would extend past buffer, write a space instead
                self.grid[row][col] = StyledChar::with_style(' ', style);
                col += 1;
                continue;
            }

            // Check clip region
            if let Some(clip) = self.clip_stack.last()
                && !clip.contains(col as u16, row as u16) {
                    col += char_width;
                    continue;
                }

            // Handle overwriting wide character's second half (placeholder)
            // If current position is a placeholder '\0', we're breaking a wide char
            if self.grid[row][col].ch == '\0' && col > 0 {
                // Clear the first half of the broken wide char
                self.grid[row][col - 1] = StyledChar::new(' ');
            }

            // Handle overwriting wide character's first half
            // If current position has a wide char, its placeholder will be orphaned
            let old_char_width = self.grid[row][col].ch.width().unwrap_or(1);
            if old_char_width == 2 && col + 1 < self.grid[row].len() {
                // Clear the orphaned placeholder
                self.grid[row][col + 1] = StyledChar::new(' ');
            }

            self.grid[row][col] = StyledChar::with_style(ch, style);

            // For wide characters (width=2), mark the next cell as a placeholder
            if char_width == 2 && col + 1 < self.grid[row].len() {
                // Check if we're about to overwrite another wide char's first half
                if self.grid[row][col + 1].ch == '\0' {
                    // This shouldn't happen as we just handled it, but be safe
                } else {
                    let next_char_width = self.grid[row][col + 1].ch.width().unwrap_or(1);
                    if next_char_width == 2 && col + 2 < self.grid[row].len() {
                        // Clear the placeholder of the wide char we're overwriting
                        self.grid[row][col + 2] = StyledChar::new(' ');
                    }
                }
                // Use a special marker for wide char continuation
                self.grid[row][col + 1] = StyledChar::new('\0');
            }

            col += char_width;
        }
    }

    /// Write a single character at position
    pub fn write_char(&mut self, x: u16, y: u16, ch: char, style: &Style) {
        let col = x as usize;
        let row = y as usize;

        if row >= self.grid.len() || col >= self.grid[row].len() {
            return;
        }

        let char_width = ch.width().unwrap_or(1);

        // Handle wide character at buffer boundary - skip if it won't fit
        if char_width == 2 && col + 1 >= self.grid[row].len() {
            // Wide char would extend past buffer, write a space instead
            self.grid[row][col] = StyledChar::with_style(' ', style);
            return;
        }

        // Check clip region
        if let Some(clip) = self.clip_stack.last()
            && !clip.contains(x, y) {
                return;
            }

        // Handle overwriting wide character's second half (placeholder)
        if self.grid[row][col].ch == '\0' && col > 0 {
            self.grid[row][col - 1] = StyledChar::new(' ');
        }

        // Handle overwriting wide character's first half
        let old_char_width = self.grid[row][col].ch.width().unwrap_or(1);
        if old_char_width == 2 && col + 1 < self.grid[row].len() {
            self.grid[row][col + 1] = StyledChar::new(' ');
        }

        self.grid[row][col] = StyledChar::with_style(ch, style);

        // For wide characters (width=2), mark the next cell as a placeholder
        if char_width == 2 && col + 1 < self.grid[row].len() {
            // Handle overwriting the next position's wide char if any
            let next_char_width = self.grid[row][col + 1].ch.width().unwrap_or(1);
            if next_char_width == 2 && col + 2 < self.grid[row].len() {
                self.grid[row][col + 2] = StyledChar::new(' ');
            }
            self.grid[row][col + 1] = StyledChar::new('\0');
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
        self.clip_stack.push(region);
    }

    /// Pop the current clip region
    pub fn unclip(&mut self) {
        self.clip_stack.pop();
    }

    /// Convert the buffer to a string with ANSI codes
    pub fn render(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        for row in self.grid.iter() {
            // First, find the last non-space, non-placeholder character
            // This determines where meaningful content ends
            let mut last_content_idx = 0;
            for (i, cell) in row.iter().enumerate() {
                // Consider any non-default-space character as content
                // A space with styling (color, bg, etc) is still content
                if cell.ch != '\0' && (cell.ch != ' ' || cell.has_style()) {
                    last_content_idx = i + 1;
                }
            }

            let mut line = String::new();
            let mut current_style: Option<StyledChar> = None;

            for (i, cell) in row.iter().enumerate() {
                // Stop at trailing whitespace (unstyled spaces at the end)
                if i >= last_content_idx {
                    break;
                }

                // Skip wide character continuation placeholders
                if cell.ch == '\0' {
                    continue;
                }

                // Check if we need to change style
                let need_style_change = match &current_style {
                    None => cell.has_style(),
                    Some(prev) => !cell.same_style(prev),
                };

                if need_style_change {
                    // Only reset if we had a previous style (not for first styled char)
                    if current_style.is_some() {
                        line.push_str("\x1b[0m");
                    }
                    self.apply_style(&mut line, cell);
                    current_style = Some(cell.clone());
                }

                line.push(cell.ch);
            }

            // Reset at end of line
            if current_style.is_some() {
                line.push_str("\x1b[0m");
            }

            lines.push(line);
        }

        // Remove trailing empty lines
        while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }

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
        let base = if background { 40 } else { 30 };

        match color {
            Color::Reset => {}
            Color::Black => codes.push(base),
            Color::Red => codes.push(base + 1),
            Color::Green => codes.push(base + 2),
            Color::Yellow => codes.push(base + 3),
            Color::Blue => codes.push(base + 4),
            Color::Magenta => codes.push(base + 5),
            Color::Cyan => codes.push(base + 6),
            Color::White => codes.push(base + 7),
            Color::BrightBlack => codes.push(base + 60),
            Color::BrightRed => codes.push(base + 61),
            Color::BrightGreen => codes.push(base + 62),
            Color::BrightYellow => codes.push(base + 63),
            Color::BrightBlue => codes.push(base + 64),
            Color::BrightMagenta => codes.push(base + 65),
            Color::BrightCyan => codes.push(base + 66),
            Color::BrightWhite => codes.push(base + 67),
            Color::Ansi256(n) => {
                codes.push(if background { 48 } else { 38 });
                codes.push(5);
                codes.push(n);
            }
            Color::Rgb(r, g, b) => {
                codes.push(if background { 48 } else { 38 });
                codes.push(2);
                codes.push(r);
                codes.push(g);
                codes.push(b);
            }
        }
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

        assert_eq!(output.grid[0][0].ch, 'H');
        assert_eq!(output.grid[0][4].ch, 'o');
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
        assert_eq!(output.grid[0][0].ch, '你');
        assert_eq!(output.grid[0][1].ch, '\0');
        // '好' at position 2, placeholder at position 3
        assert_eq!(output.grid[0][2].ch, '好');
        assert_eq!(output.grid[0][3].ch, '\0');
    }

    #[test]
    fn test_overwrite_wide_char_placeholder() {
        let mut output = Output::new(80, 24);
        // Write a wide char first
        output.write(0, 0, "你", &Style::default());
        assert_eq!(output.grid[0][0].ch, '你');
        assert_eq!(output.grid[0][1].ch, '\0');

        // Overwrite the placeholder with a narrow char
        output.write_char(1, 0, 'X', &Style::default());

        // The wide char should be replaced with space (broken)
        assert_eq!(output.grid[0][0].ch, ' ');
        assert_eq!(output.grid[0][1].ch, 'X');
    }

    #[test]
    fn test_overwrite_wide_char_first_half() {
        let mut output = Output::new(80, 24);
        // Write a wide char first
        output.write(0, 0, "你", &Style::default());
        assert_eq!(output.grid[0][0].ch, '你');
        assert_eq!(output.grid[0][1].ch, '\0');

        // Overwrite the first half with a narrow char
        output.write_char(0, 0, 'X', &Style::default());

        // The wide char's placeholder should be cleared
        assert_eq!(output.grid[0][0].ch, 'X');
        assert_eq!(output.grid[0][1].ch, ' ');
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
        assert_eq!(output.grid[0][3].ch, '你');
        assert_eq!(output.grid[0][4].ch, '\0');

        // Now test when wide char would extend past buffer
        let mut output2 = Output::new(5, 1);
        output2.write(4, 0, "你", &Style::default());

        // Should write a space instead since wide char won't fit
        assert_eq!(output2.grid[0][4].ch, ' ');
    }

    #[test]
    fn test_wide_char_at_exact_boundary() {
        // Test when wide char is at the last valid position
        let mut output = Output::new(4, 1);
        output.write(2, 0, "你", &Style::default());

        // Wide char at position 2-3 should fit exactly
        assert_eq!(output.grid[0][2].ch, '你');
        assert_eq!(output.grid[0][3].ch, '\0');
    }
}
