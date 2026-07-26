//! Output buffer for terminal rendering

use crate::core::{Color, Style};
use std::fmt::Write as FmtWrite;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const TAB_STOP: usize = 4;

mod cross_call_zwj;
mod zero_width;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceScalar {
    Break,
    Tab,
    Visible(char),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum GraphemeCell {
    #[default]
    Empty,
    Lead {
        width: usize,
        suffix: String,
    },
    Continuation {
        lead_col: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CellPosition {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphemeWriteFootprint {
    pub(crate) target_cells: Vec<CellPosition>,
    pub(crate) old_cells: Vec<CellPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GraphemeWriteOutcome {
    Committed(GraphemeWriteFootprint),
    Clipped,
}

fn sanitize_source_scalar(ch: char) -> SourceScalar {
    match ch {
        '\n' | '\r' => SourceScalar::Break,
        '\t' => SourceScalar::Tab,
        '\0'..='\u{001f}' => {
            let code_point = 0x2400 + u32::from(ch);
            SourceScalar::Visible(
                char::from_u32(code_point)
                    .expect("C0 control picture code points must be valid Unicode scalars"),
            )
        }
        '\u{007f}' => SourceScalar::Visible('\u{2421}'),
        '\u{0080}'..='\u{009f}' => SourceScalar::Visible('\u{fffd}'),
        _ => SourceScalar::Visible(ch),
    }
}

fn sanitize_grapheme(grapheme: &str) -> String {
    grapheme
        .chars()
        .map(|ch| match sanitize_source_scalar(ch) {
            SourceScalar::Visible(ch) => ch,
            SourceScalar::Break if ch == '\n' => '␊',
            SourceScalar::Break => '␍',
            SourceScalar::Tab => '␉',
        })
        .collect()
}

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
    grapheme_cells: Vec<GraphemeCell>,
    pending_zwj: Option<cross_call_zwj::PendingZwj>,
    clip_stack: Vec<ClipRegion>,
    /// Tracks which rows have been modified since last clear_dirty()
    dirty_rows: Vec<bool>,
    dirty_cells: Vec<bool>,
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
            grapheme_cells: vec![GraphemeCell::Empty; size],
            pending_zwj: None,
            clip_stack: Vec::new(),
            dirty_rows: vec![false; height as usize],
            dirty_cells: vec![false; size],
            any_dirty: false,
        }
    }

    /// Get a reference to a cell at (col, row)
    #[cfg(test)]
    #[inline]
    fn get(&self, col: usize, row: usize) -> Option<&StyledChar> {
        if col < self.width as usize && row < self.height as usize {
            let width = self.width as usize;
            Some(&self.grid[(row * width) + col])
        } else {
            None
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
        self.dirty_cells.fill(false);
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
            let metadata = &self.grapheme_cells[row_idx * self.width as usize + i];
            let has_suffix =
                matches!(metadata, GraphemeCell::Lead { suffix, .. } if !suffix.is_empty());
            if cell.ch != '\0' && (cell.ch != ' ' || cell.has_style() || has_suffix) {
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
            if let GraphemeCell::Lead { suffix, .. } =
                &self.grapheme_cells[row_idx * self.width as usize + i]
            {
                line.push_str(suffix);
            }
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

    fn mark_cell_dirty(&mut self, position: CellPosition) {
        let idx = position.y as usize * self.width as usize + position.x as usize;
        self.dirty_cells[idx] = true;
        self.mark_dirty(position.y as usize);
    }

    #[cfg(test)]
    pub(crate) fn dirty_cell_positions(&self) -> impl Iterator<Item = CellPosition> + '_ {
        let width = self.width as usize;
        self.dirty_cells
            .iter()
            .enumerate()
            .filter_map(move |(idx, dirty)| {
                dirty.then_some(CellPosition {
                    x: (idx % width) as u16,
                    y: (idx / width) as u16,
                })
            })
    }

    /// Create an isolated frame copy for transactional renderer staging.
    #[allow(dead_code)] // T5 consumes this crate-private handoff.
    pub(crate) fn staged_snapshot(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            grid: self.grid.clone(),
            grapheme_cells: self.grapheme_cells.clone(),
            pending_zwj: self.pending_zwj,
            clip_stack: self.clip_stack.clone(),
            dirty_rows: self.dirty_rows.clone(),
            dirty_cells: self.dirty_cells.clone(),
            any_dirty: self.any_dirty,
        }
    }

    /// Publish a fully validated staged frame in one replacement.
    #[allow(dead_code)] // T5 consumes this crate-private handoff.
    pub(crate) fn commit_staged(&mut self, staged: Self) {
        *self = staged;
    }

    /// Write text at position with style
    pub fn write(&mut self, x: u16, y: u16, text: &str, style: &Style) {
        let mut col = x as usize;
        let row = y as usize;

        if row >= self.height as usize {
            return;
        }

        // Preserve the existing dirty contract for every in-bounds write attempt.
        self.mark_dirty(row);

        let width = self.width as usize;

        for source_grapheme in text.graphemes(true) {
            match source_grapheme {
                "\n" | "\r" | "\r\n" => {
                    self.clear_pending_zwj();
                    break;
                }
                "\t" => {
                    self.clear_pending_zwj();
                    let spaces = TAB_STOP - (col % TAB_STOP);
                    for _ in 0..spaces {
                        if col >= width {
                            break;
                        }
                        let _ = self.write_grapheme(col as i64, row as i64, " ", style);
                        col += 1;
                    }
                }
                _ => {
                    let safe = sanitize_grapheme(source_grapheme);
                    let grapheme_width = UnicodeWidthStr::width(safe.as_str());
                    if grapheme_width > 0
                        && col >= width
                        && !self.has_pending_zwj_at(col as i64, row as i64)
                    {
                        self.clear_pending_zwj();
                        break;
                    }
                    let outcome = self.write_grapheme(col as i64, row as i64, &safe, style);
                    let merged_into_previous = matches!(
                        &outcome,
                        GraphemeWriteOutcome::Committed(footprint)
                            if footprint.target_cells.first().is_some_and(
                                |first| usize::from(first.x) < col
                            )
                    );
                    if !merged_into_previous {
                        col = col.saturating_add(grapheme_width);
                    }
                }
            }
        }
    }

    /// Write a single character at position
    pub fn write_char(&mut self, x: u16, y: u16, ch: char, style: &Style) {
        let col = x as usize;
        let row = y as usize;

        if row >= self.height as usize {
            return;
        }

        match sanitize_source_scalar(ch) {
            SourceScalar::Break => {
                self.clear_pending_zwj();
                if col < self.width as usize {
                    self.mark_dirty(row);
                }
            }
            SourceScalar::Tab => {
                self.clear_pending_zwj();
                if col >= self.width as usize {
                    return;
                }
                self.mark_dirty(row);
                let spaces = TAB_STOP - (col % TAB_STOP);
                for offset in 0..spaces {
                    let target_col = col + offset;
                    if target_col >= self.width as usize {
                        break;
                    }
                    let _ = self.write_grapheme(target_col as i64, row as i64, " ", style);
                }
            }
            SourceScalar::Visible(ch) => {
                let safe = sanitize_grapheme(&ch.to_string());
                let grapheme_width = UnicodeWidthStr::width(safe.as_str());
                if col >= self.width as usize
                    && !(grapheme_width == 0 && col == self.width as usize)
                    && !self.has_pending_zwj_at(col as i64, row as i64)
                {
                    self.clear_pending_zwj();
                    return;
                }
                self.mark_dirty(row);
                let _ = self.write_grapheme(col as i64, row as i64, &safe, style);
            }
        }
    }

    pub(crate) fn active_clips_contain_grapheme(
        &self,
        x: i64,
        y: i64,
        display_width: usize,
    ) -> bool {
        if display_width == 0 {
            return false;
        }
        self.target_cells(x, y, display_width)
            .is_some_and(|cells| self.active_clips_contain_cells(&cells))
    }

    pub(crate) fn prospective_grapheme_write_footprint(
        &self,
        x: i64,
        y: i64,
        grapheme: &str,
    ) -> Option<GraphemeWriteFootprint> {
        let safe = sanitize_grapheme(grapheme);
        let display_width = UnicodeWidthStr::width(safe.as_str());
        self.prospective_footprint_for_width(x, y, display_width)
    }

    pub(crate) fn write_grapheme(
        &mut self,
        x: i64,
        y: i64,
        grapheme: &str,
        style: &Style,
    ) -> GraphemeWriteOutcome {
        let safe = sanitize_grapheme(grapheme);
        let mut graphemes = safe.graphemes(true);
        let Some(safe_grapheme) = graphemes.next() else {
            self.clear_pending_zwj();
            return GraphemeWriteOutcome::Clipped;
        };
        if graphemes.next().is_some() {
            self.clear_pending_zwj();
            return GraphemeWriteOutcome::Clipped;
        }

        let display_width = UnicodeWidthStr::width(safe_grapheme);
        if display_width == 0 {
            return self.attach_zero_width(x, y, safe_grapheme);
        }
        if let Some(outcome) = self.try_complete_pending_zwj(x, y, safe_grapheme, display_width) {
            return outcome;
        }
        let Some(footprint) = self.prospective_grapheme_write_footprint(x, y, safe_grapheme) else {
            return GraphemeWriteOutcome::Clipped;
        };

        let mut scalars = safe_grapheme.chars();
        let lead = scalars
            .next()
            .expect("a non-empty grapheme must have a lead scalar");
        let suffix = scalars.collect();
        self.commit_styled_grapheme(
            &footprint,
            StyledChar::with_style(lead, style),
            suffix,
            display_width,
        );
        GraphemeWriteOutcome::Committed(footprint)
    }

    fn prospective_footprint_for_width(
        &self,
        x: i64,
        y: i64,
        display_width: usize,
    ) -> Option<GraphemeWriteFootprint> {
        if display_width == 0 {
            return self.zero_width_attachment_footprint(x, y);
        }
        let target_cells = self.target_cells(x, y, display_width)?;
        if !self.active_clips_contain_grapheme(x, y, display_width) {
            return None;
        }

        let mut old_cells = Vec::new();
        for target in &target_cells {
            for old in self.owner_footprint(*target) {
                if !old_cells.contains(&old) {
                    old_cells.push(old);
                }
            }
        }
        old_cells.sort_by_key(|position| (position.y, position.x));
        if !self.active_clips_contain_cells(&old_cells) {
            return None;
        }
        Some(GraphemeWriteFootprint {
            target_cells,
            old_cells,
        })
    }

    fn target_cells(&self, x: i64, y: i64, display_width: usize) -> Option<Vec<CellPosition>> {
        if y < 0 || y >= i64::from(self.height) || x < 0 {
            return None;
        }
        if display_width == 0 {
            return (x <= i64::from(self.width)).then(Vec::new);
        }

        let mut cells = Vec::with_capacity(display_width);
        for offset in 0..display_width {
            let offset = i64::try_from(offset).ok()?;
            let cell_x = x.checked_add(offset)?;
            if cell_x >= i64::from(self.width) {
                return None;
            }
            cells.push(CellPosition {
                x: u16::try_from(cell_x).ok()?,
                y: u16::try_from(y).ok()?,
            });
        }
        Some(cells)
    }

    fn active_clips_contain_cells(&self, cells: &[CellPosition]) -> bool {
        cells.iter().all(|position| {
            self.clip_stack
                .iter()
                .all(|clip| clip.contains(position.x, position.y))
        })
    }

    fn owner_footprint(&self, position: CellPosition) -> Vec<CellPosition> {
        let width = self.width as usize;
        let idx = position.y as usize * width + position.x as usize;
        let (lead_col, owner_width) = match &self.grapheme_cells[idx] {
            GraphemeCell::Empty => return Vec::new(),
            GraphemeCell::Lead { width, .. } => (position.x as usize, *width),
            GraphemeCell::Continuation { lead_col } => {
                let lead_idx = position.y as usize * width + *lead_col;
                match &self.grapheme_cells[lead_idx] {
                    GraphemeCell::Lead { width, .. } => (*lead_col, *width),
                    _ => return Vec::new(),
                }
            }
        };
        (lead_col..lead_col.saturating_add(owner_width))
            .filter(|col| *col < width)
            .map(|col| CellPosition {
                x: col as u16,
                y: position.y,
            })
            .collect()
    }

    fn commit_styled_grapheme(
        &mut self,
        footprint: &GraphemeWriteFootprint,
        lead: StyledChar,
        suffix: String,
        display_width: usize,
    ) {
        let width = self.width as usize;
        for position in &footprint.old_cells {
            let idx = position.y as usize * width + position.x as usize;
            self.grid[idx] = StyledChar::new(' ');
            self.grapheme_cells[idx] = GraphemeCell::Empty;
            self.mark_cell_dirty(*position);
        }

        let first = footprint
            .target_cells
            .first()
            .expect("a visible grapheme must own at least one target cell");
        for (offset, position) in footprint.target_cells.iter().enumerate() {
            let idx = position.y as usize * width + position.x as usize;
            if offset == 0 {
                self.grid[idx] = lead.clone();
                self.grapheme_cells[idx] = GraphemeCell::Lead {
                    width: display_width,
                    suffix: suffix.clone(),
                };
            } else {
                self.grid[idx] = StyledChar::new('\0');
                self.grapheme_cells[idx] = GraphemeCell::Continuation {
                    lead_col: first.x as usize,
                };
            }
            self.mark_cell_dirty(*position);
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
mod tests;
