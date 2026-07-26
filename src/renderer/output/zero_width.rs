use super::{CellPosition, GraphemeCell, GraphemeWriteFootprint, GraphemeWriteOutcome, Output};

impl Output {
    pub(super) fn zero_width_attachment_footprint(
        &self,
        x: i64,
        y: i64,
    ) -> Option<GraphemeWriteFootprint> {
        if y < 0 || y >= i64::from(self.height) || x <= 0 || x > i64::from(self.width) {
            return None;
        }
        let previous = CellPosition {
            x: u16::try_from(x - 1).ok()?,
            y: u16::try_from(y).ok()?,
        };
        let owner = self.owner_footprint(previous);
        if owner.is_empty() || !self.active_clips_contain_cells(&owner) {
            return None;
        }
        Some(GraphemeWriteFootprint {
            target_cells: Vec::new(),
            old_cells: owner,
        })
    }

    pub(super) fn attach_zero_width(&mut self, x: i64, y: i64, text: &str) -> GraphemeWriteOutcome {
        let Some(footprint) = self.zero_width_attachment_footprint(x, y) else {
            return GraphemeWriteOutcome::Clipped;
        };
        let lead = footprint.old_cells[0];
        let idx = lead.y as usize * self.width as usize + lead.x as usize;
        let GraphemeCell::Lead { suffix, .. } = &mut self.grapheme_cells[idx] else {
            return GraphemeWriteOutcome::Clipped;
        };
        suffix.push_str(text);
        for position in &footprint.old_cells {
            self.mark_cell_dirty(*position);
        }
        GraphemeWriteOutcome::Committed(footprint)
    }
}
