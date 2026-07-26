use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{CellPosition, GraphemeCell, GraphemeWriteOutcome, Output};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PendingZwj {
    lead: CellPosition,
    completion: CellPosition,
}

impl Output {
    pub(super) fn clear_pending_zwj(&mut self) {
        self.pending_zwj = None;
    }

    pub(super) fn has_pending_zwj_at(&self, x: i64, y: i64) -> bool {
        self.pending_zwj.is_some_and(|pending| {
            i64::from(pending.completion.x) == x && i64::from(pending.completion.y) == y
        })
    }

    pub(super) fn update_pending_zwj(
        &mut self,
        lead: CellPosition,
        completion_x: i64,
        completion_y: i64,
        owner_width: usize,
    ) {
        let Some(owner_end) = usize::from(lead.x).checked_add(owner_width) else {
            self.pending_zwj = None;
            return;
        };
        let (Ok(completion_x), Ok(completion_y)) =
            (u16::try_from(completion_x), u16::try_from(completion_y))
        else {
            self.pending_zwj = None;
            return;
        };
        let completion = CellPosition {
            x: completion_x,
            y: completion_y,
        };
        self.pending_zwj = (owner_end == usize::from(completion.x) && completion.y == lead.y)
            .then_some(PendingZwj { lead, completion });
    }

    pub(super) fn try_complete_pending_zwj(
        &mut self,
        x: i64,
        y: i64,
        safe_grapheme: &str,
        completion_width: usize,
    ) -> Option<GraphemeWriteOutcome> {
        let pending = self.pending_zwj?;
        if i64::from(pending.completion.x) != x || i64::from(pending.completion.y) != y {
            self.pending_zwj = None;
            return None;
        }

        let row_width = self.width as usize;
        let lead_idx = usize::from(pending.lead.y)
            .checked_mul(row_width)?
            .checked_add(usize::from(pending.lead.x))?;
        let (owner_width, current_suffix) = match self.grapheme_cells.get(lead_idx)? {
            GraphemeCell::Lead { width, suffix } if suffix.ends_with('\u{200d}') => {
                (*width, suffix.clone())
            }
            _ => {
                self.pending_zwj = None;
                return None;
            }
        };
        if usize::from(pending.lead.x).checked_add(owner_width)?
            != usize::from(pending.completion.x)
        {
            self.pending_zwj = None;
            return None;
        }

        let mut candidate = String::new();
        candidate.push(self.grid.get(lead_idx)?.ch);
        candidate.push_str(&current_suffix);
        candidate.push_str(safe_grapheme);
        let mut graphemes = candidate.graphemes(true);
        if graphemes.next()? != candidate || graphemes.next().is_some() {
            self.pending_zwj = None;
            return None;
        }

        let candidate_width = UnicodeWidthStr::width(candidate.as_str());
        if candidate_width == 0 {
            return Some(GraphemeWriteOutcome::Clipped);
        }
        let mut footprint = match self.prospective_footprint_for_width(
            i64::from(pending.lead.x),
            i64::from(pending.lead.y),
            candidate_width,
        ) {
            Some(footprint) => footprint,
            None => return Some(GraphemeWriteOutcome::Clipped),
        };
        let completion_cells = match self.in_bounds_completion_cells(x, y, completion_width) {
            Some(cells) if self.active_clips_contain_cells(&cells) => cells,
            _ => return Some(GraphemeWriteOutcome::Clipped),
        };
        for cell in completion_cells {
            for old in self.owner_footprint(cell) {
                if !footprint.old_cells.contains(&old) {
                    footprint.old_cells.push(old);
                }
            }
        }
        footprint
            .old_cells
            .sort_by_key(|position| (position.y, position.x));
        if !self.active_clips_contain_cells(&footprint.old_cells) {
            return Some(GraphemeWriteOutcome::Clipped);
        }

        let mut scalars = candidate.chars();
        let lead = scalars
            .next()
            .expect("a validated non-empty EGC must have a lead scalar");
        let suffix = scalars.collect();
        let mut styled_lead = self.grid[lead_idx].clone();
        styled_lead.ch = lead;
        self.pending_zwj = None;
        self.commit_styled_grapheme(&footprint, styled_lead, suffix, candidate_width);
        Some(GraphemeWriteOutcome::Committed(footprint))
    }

    fn in_bounds_completion_cells(
        &self,
        x: i64,
        y: i64,
        display_width: usize,
    ) -> Option<Vec<CellPosition>> {
        if x < 0 || x > i64::from(self.width) || y < 0 || y >= i64::from(self.height) {
            return None;
        }
        let mut cells = Vec::with_capacity(display_width);
        for offset in 0..display_width {
            let cell_x = x.checked_add(i64::try_from(offset).ok()?)?;
            if cell_x >= i64::from(self.width) {
                break;
            }
            cells.push(CellPosition {
                x: u16::try_from(cell_x).ok()?,
                y: u16::try_from(y).ok()?,
            });
        }
        Some(cells)
    }
}
