use std::collections::HashMap;

use crate::core::{ElementId, Style};
use crate::layout::text_flow::{TextFlow, TextFlowPlacement, TextFlowToken};
use crate::renderer::Output;
use crate::renderer::output::{ClipRegion, GraphemeWriteOutcome};

use super::{
    CellOrigin, ForwardProjection, FrameCell, FrameDisposition, NonCellDisposition,
    ProjectionError, ProjectionId, ProjectionOptions, ProjectionStats, RenderProjection,
    SignedCell, validate_round_trip,
};

pub(in crate::renderer::tree_renderer) struct StagedFrame {
    output: Output,
    projection: ProjectionBuilder,
    initial_clip_depth: usize,
    fill_bounds: VisibleRect,
    fill_bounds_stack: Vec<VisibleRect>,
    #[cfg(test)]
    fill_candidate_visits: usize,
    writes: usize,
    options: ProjectionOptions,
}

impl StagedFrame {
    pub(in crate::renderer::tree_renderer) fn new(
        output: &Output,
        options: ProjectionOptions,
    ) -> Self {
        let snapshot = output.staged_snapshot();
        let fill_bounds = active_visible_bounds(&snapshot);
        Self {
            initial_clip_depth: snapshot.clip_depth(),
            output: snapshot,
            projection: ProjectionBuilder::default(),
            fill_bounds,
            fill_bounds_stack: Vec::new(),
            #[cfg(test)]
            fill_candidate_visits: 0,
            writes: 0,
            options,
        }
    }

    pub(in crate::renderer::tree_renderer) fn project_flow(
        &mut self,
        element_id: ElementId,
        flow: &TextFlow,
        origin_x: i64,
        origin_y: i64,
    ) -> Result<(), ProjectionError> {
        for (token_index, token) in flow.tokens().iter().enumerate() {
            let id = ProjectionId {
                element_id,
                token_index,
            };
            self.projection.add_record(ForwardProjection {
                id,
                source: token.source.clone(),
                logical: token.placement.clone(),
                text: token.safe_text.clone(),
                display_width: token.display_width,
                frame: initial_frame_disposition(&token.placement),
            })?;
            self.project_token(id, token, origin_x, origin_y)?;
        }
        Ok(())
    }

    fn project_token(
        &mut self,
        id: ProjectionId,
        token: &TextFlowToken,
        origin_x: i64,
        origin_y: i64,
    ) -> Result<(), ProjectionError> {
        let (row, column) = match token.placement {
            TextFlowPlacement::Positioned { row, column }
            | TextFlowPlacement::ZeroWidth { row, column }
            | TextFlowPlacement::SanitizedControl { row, column }
            | TextFlowPlacement::Synthetic { row, column } => (row, column),
            TextFlowPlacement::HardBreak { .. }
            | TextFlowPlacement::Omitted { .. }
            | TextFlowPlacement::Truncated { .. } => return Ok(()),
        };
        let base_x = origin_x
            .checked_add(i64::try_from(column).map_err(|_| ProjectionError::CoordinateOverflow)?)
            .ok_or(ProjectionError::CoordinateOverflow)?;
        let y = origin_y
            .checked_add(i64::try_from(row).map_err(|_| ProjectionError::CoordinateOverflow)?)
            .ok_or(ProjectionError::CoordinateOverflow)?;

        if token.display_width == 0 {
            return self.write_zero_width(id, row, column, base_x, y, &token.safe_text);
        }
        if is_published_space_expansion(token) {
            for offset in 0..token.display_width {
                let offset =
                    i64::try_from(offset).map_err(|_| ProjectionError::CoordinateOverflow)?;
                let x = base_x
                    .checked_add(offset)
                    .ok_or(ProjectionError::CoordinateOverflow)?;
                self.project_visible_grapheme(id, x, y, " ", 1, &token.style)?;
            }
            return Ok(());
        }
        self.project_visible_grapheme(
            id,
            base_x,
            y,
            &token.safe_text,
            token.display_width,
            &token.style,
        )
    }

    fn project_visible_grapheme(
        &mut self,
        id: ProjectionId,
        x: i64,
        y: i64,
        grapheme: &str,
        width: usize,
        style: &Style,
    ) -> Result<(), ProjectionError> {
        let signed_cells = signed_cells(x, y, width)?;
        let Some(expected) = self
            .output
            .prospective_grapheme_write_footprint(x, y, grapheme)
        else {
            self.projection.extend_clipped(id, signed_cells)?;
            return Ok(());
        };
        if expected.target_cells.len() != width {
            return Err(ProjectionError::WriterOutcomeMismatch);
        }

        self.checkpoint()?;
        let GraphemeWriteOutcome::Committed(actual) =
            self.output.write_grapheme(x, y, grapheme, style)
        else {
            return Err(ProjectionError::WriterOutcomeMismatch);
        };
        if actual != expected {
            return Err(ProjectionError::WriterOutcomeMismatch);
        }
        self.projection.retire(&actual.old_cells)?;
        self.projection.publish(id, &actual.target_cells)?;
        Ok(())
    }

    fn write_zero_width(
        &mut self,
        id: ProjectionId,
        row: usize,
        column: usize,
        x: i64,
        y: i64,
        grapheme: &str,
    ) -> Result<(), ProjectionError> {
        let Some(owner) = self
            .projection
            .preceding_sequence_owner(id, row, column, x, y)
        else {
            return Ok(());
        };
        self.checkpoint()?;
        let GraphemeWriteOutcome::Committed(actual) =
            self.output
                .write_grapheme(x, y, grapheme, &Style::default())
        else {
            return Err(ProjectionError::WriterOutcomeMismatch);
        };
        if !actual.target_cells.is_empty()
            || actual.old_cells.is_empty()
            || actual
                .old_cells
                .iter()
                .any(|position| self.projection.owner_at(position.x, position.y) != Some(owner))
        {
            return Err(ProjectionError::WriterOutcomeMismatch);
        }
        Ok(())
    }

    pub(in crate::renderer::tree_renderer) fn paint_grapheme(
        &mut self,
        x: i64,
        y: i64,
        grapheme: &str,
        style: &Style,
    ) -> Result<(), ProjectionError> {
        let Some(expected) = self
            .output
            .prospective_grapheme_write_footprint(x, y, grapheme)
        else {
            return Ok(());
        };
        self.checkpoint()?;
        let GraphemeWriteOutcome::Committed(actual) =
            self.output.write_grapheme(x, y, grapheme, style)
        else {
            return Err(ProjectionError::WriterOutcomeMismatch);
        };
        if actual != expected {
            return Err(ProjectionError::WriterOutcomeMismatch);
        }
        self.projection.retire(&actual.old_cells)
    }

    pub(in crate::renderer::tree_renderer) fn fill_rect(
        &mut self,
        x: i64,
        y: i64,
        width: u16,
        height: u16,
        style: &Style,
    ) -> Result<(), ProjectionError> {
        if x >= self.fill_bounds.x2 || y >= self.fill_bounds.y2 {
            return Ok(());
        }
        let Some(layout_rect) = VisibleRect::from_origin_size(x, y, width, height)? else {
            return Ok(());
        };
        let Some(fill) = layout_rect.intersection(self.fill_bounds) else {
            return Ok(());
        };
        for row in fill.y1..fill.y2 {
            for column in fill.x1..fill.x2 {
                #[cfg(test)]
                {
                    self.fill_candidate_visits = self
                        .fill_candidate_visits
                        .checked_add(1)
                        .ok_or(ProjectionError::CoordinateOverflow)?;
                }
                self.paint_grapheme(column, row, " ", style)?;
            }
        }
        Ok(())
    }

    pub(in crate::renderer::tree_renderer) fn clip(&mut self, region: ClipRegion) {
        self.fill_bounds_stack.push(self.fill_bounds);
        self.fill_bounds = self
            .fill_bounds
            .intersection(VisibleRect::from_clip(&region))
            .unwrap_or_default();
        self.output.clip(region);
    }

    pub(in crate::renderer::tree_renderer) fn unclip(&mut self) {
        self.output.unclip();
        self.fill_bounds = self
            .fill_bounds_stack
            .pop()
            .unwrap_or_else(|| active_visible_bounds(&self.output));
    }

    #[cfg(test)]
    pub(in crate::renderer::tree_renderer) fn fill_candidate_visits(&self) -> usize {
        self.fill_candidate_visits
    }

    fn checkpoint(&mut self) -> Result<(), ProjectionError> {
        if self.options.fail_after_writes == Some(self.writes) {
            return Err(ProjectionError::InjectedFailure);
        }
        self.writes = self
            .writes
            .checked_add(1)
            .ok_or(ProjectionError::CoordinateOverflow)?;
        Ok(())
    }

    pub(in crate::renderer::tree_renderer) fn finish(
        self,
    ) -> Result<(Output, RenderProjection), ProjectionError> {
        if self.output.clip_depth() != self.initial_clip_depth {
            return Err(ProjectionError::UnbalancedClipStack);
        }
        let mut projection = self.projection.finish();
        projection.stats.validation_visits = validate_round_trip(&projection)?;
        Ok((self.output, projection))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VisibleRect {
    x1: i64,
    y1: i64,
    x2: i64,
    y2: i64,
}

impl VisibleRect {
    fn viewport(output: &Output) -> Self {
        Self {
            x1: 0,
            y1: 0,
            x2: i64::from(output.width),
            y2: i64::from(output.height),
        }
    }

    fn from_clip(clip: &ClipRegion) -> Self {
        Self {
            x1: i64::from(clip.x1),
            y1: i64::from(clip.y1),
            x2: i64::from(clip.x2),
            y2: i64::from(clip.y2),
        }
    }

    fn from_origin_size(
        x: i64,
        y: i64,
        width: u16,
        height: u16,
    ) -> Result<Option<Self>, ProjectionError> {
        if width == 0 || height == 0 {
            return Ok(None);
        }
        let x2 = x
            .checked_add(i64::from(width))
            .ok_or(ProjectionError::CoordinateOverflow)?;
        let y2 = y
            .checked_add(i64::from(height))
            .ok_or(ProjectionError::CoordinateOverflow)?;
        Ok(Some(Self {
            x1: x,
            y1: y,
            x2,
            y2,
        }))
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let intersection = Self {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        };
        (intersection.x1 < intersection.x2 && intersection.y1 < intersection.y2)
            .then_some(intersection)
    }
}

fn active_visible_bounds(output: &Output) -> VisibleRect {
    let viewport = VisibleRect::viewport(output);
    if output.clip_depth() == 0 {
        return viewport;
    }

    let mut visible: Option<VisibleRect> = None;
    for y in viewport.y1..viewport.y2 {
        for x in viewport.x1..viewport.x2 {
            if !output.active_clips_contain_grapheme(x, y, 1) {
                continue;
            }
            visible = Some(match visible {
                Some(bounds) => VisibleRect {
                    x1: bounds.x1.min(x),
                    y1: bounds.y1.min(y),
                    x2: bounds.x2.max(x + 1),
                    y2: bounds.y2.max(y + 1),
                },
                None => VisibleRect {
                    x1: x,
                    y1: y,
                    x2: x + 1,
                    y2: y + 1,
                },
            });
        }
    }
    visible.unwrap_or_default()
}

fn is_published_space_expansion(token: &TextFlowToken) -> bool {
    token.safe_text.len() == token.display_width && token.safe_text.bytes().all(|byte| byte == b' ')
}

fn signed_cells(x: i64, y: i64, width: usize) -> Result<Vec<SignedCell>, ProjectionError> {
    let mut cells = Vec::with_capacity(width);
    for offset in 0..width {
        cells.push(SignedCell {
            x: x.checked_add(
                i64::try_from(offset).map_err(|_| ProjectionError::CoordinateOverflow)?,
            )
            .ok_or(ProjectionError::CoordinateOverflow)?,
            y,
        });
    }
    Ok(cells)
}

fn initial_frame_disposition(placement: &TextFlowPlacement) -> FrameDisposition {
    let non_cell = match placement {
        TextFlowPlacement::HardBreak { .. } => Some(NonCellDisposition::HardBreak),
        TextFlowPlacement::ZeroWidth { .. } => Some(NonCellDisposition::ZeroWidth),
        TextFlowPlacement::Omitted { .. } => Some(NonCellDisposition::Omitted),
        TextFlowPlacement::Truncated { .. } => Some(NonCellDisposition::Truncated),
        TextFlowPlacement::Positioned { .. }
        | TextFlowPlacement::SanitizedControl { .. }
        | TextFlowPlacement::Synthetic { .. } => None,
    };
    non_cell.map_or_else(
        || FrameDisposition::Cells {
            visible: Vec::new(),
            clipped: Vec::new(),
            replaced: Vec::new(),
        },
        FrameDisposition::NonCell,
    )
}

#[derive(Default)]
struct ProjectionBuilder {
    forward: Vec<ForwardProjection>,
    forward_index: HashMap<ProjectionId, usize>,
    reverse: HashMap<FrameCell, CellOrigin>,
    visible_index: HashMap<(ProjectionId, FrameCell), usize>,
}

impl ProjectionBuilder {
    fn add_record(&mut self, record: ForwardProjection) -> Result<(), ProjectionError> {
        let index = self.forward.len();
        if self.forward_index.insert(record.id, index).is_some() {
            return Err(ProjectionError::DuplicateForwardRecord(record.id));
        }
        self.forward.push(record);
        Ok(())
    }

    fn extend_clipped(
        &mut self,
        id: ProjectionId,
        cells: Vec<SignedCell>,
    ) -> Result<(), ProjectionError> {
        let FrameDisposition::Cells { clipped, .. } = &mut self.record_mut(id)?.frame else {
            return Err(ProjectionError::MalformedFlow(
                "non-cell token produced clipped cells",
            ));
        };
        clipped.extend(cells);
        Ok(())
    }

    fn publish(
        &mut self,
        id: ProjectionId,
        cells: &[crate::renderer::output::CellPosition],
    ) -> Result<(), ProjectionError> {
        let record = self.record_mut(id)?;
        let origin = record.origin();
        let FrameDisposition::Cells {
            visible: record_visible,
            ..
        } = &record.frame
        else {
            return Err(ProjectionError::MalformedFlow(
                "non-cell token produced visible cells",
            ));
        };
        let first_index = record_visible.len();
        let mut visible = Vec::with_capacity(cells.len());
        for position in cells {
            let cell = FrameCell {
                x: position.x,
                y: position.y,
            };
            if self.reverse.insert(cell, origin.clone()).is_some() {
                return Err(ProjectionError::DuplicateReverseCell(cell));
            }
            let visible_index = first_index + visible.len();
            if self
                .visible_index
                .insert((id, cell), visible_index)
                .is_some()
            {
                return Err(ProjectionError::DuplicateReverseCell(cell));
            }
            visible.push(cell);
        }
        let FrameDisposition::Cells {
            visible: record_visible,
            ..
        } = &mut self.record_mut(id)?.frame
        else {
            return Err(ProjectionError::MalformedFlow(
                "non-cell token produced visible cells",
            ));
        };
        record_visible.extend(visible);
        Ok(())
    }

    fn retire(
        &mut self,
        cells: &[crate::renderer::output::CellPosition],
    ) -> Result<(), ProjectionError> {
        for position in cells {
            let cell = FrameCell {
                x: position.x,
                y: position.y,
            };
            let Some(origin) = self.reverse.remove(&cell) else {
                continue;
            };
            let id = origin.id();
            let index = self
                .visible_index
                .remove(&(id, cell))
                .ok_or(ProjectionError::WriterOutcomeMismatch)?;
            let moved = {
                let FrameDisposition::Cells {
                    visible, replaced, ..
                } = &mut self.record_mut(id)?.frame
                else {
                    return Err(ProjectionError::MalformedFlow(
                        "non-cell token owned a reverse cell",
                    ));
                };
                visible.swap_remove(index);
                let moved = visible.get(index).copied();
                replaced.push(cell);
                moved
            };
            if let Some(moved) = moved {
                self.visible_index.insert((id, moved), index);
            }
        }
        Ok(())
    }

    fn preceding_sequence_owner(
        &self,
        id: ProjectionId,
        row: usize,
        column: usize,
        x: i64,
        y: i64,
    ) -> Option<ProjectionId> {
        let previous_x = x.checked_sub(1)?;
        let cell = FrameCell {
            x: u16::try_from(previous_x).ok()?,
            y: u16::try_from(y).ok()?,
        };
        let owner = self.reverse.get(&cell)?.id();
        if owner.element_id != id.element_id || owner.token_index >= id.token_index {
            return None;
        }
        let record = self
            .forward_index
            .get(&owner)
            .and_then(|index| self.forward.get(*index))?;
        let (owner_row, owner_column) = match record.logical {
            TextFlowPlacement::Positioned { row, column }
            | TextFlowPlacement::SanitizedControl { row, column }
            | TextFlowPlacement::Synthetic { row, column } => (row, column),
            TextFlowPlacement::ZeroWidth { .. }
            | TextFlowPlacement::HardBreak { .. }
            | TextFlowPlacement::Omitted { .. }
            | TextFlowPlacement::Truncated { .. } => return None,
        };
        let owner_end = owner_column.checked_add(record.display_width)?;
        (owner_row == row && owner_end == column).then_some(owner)
    }

    fn owner_at(&self, x: u16, y: u16) -> Option<ProjectionId> {
        self.reverse.get(&FrameCell { x, y }).map(CellOrigin::id)
    }

    fn record_mut(&mut self, id: ProjectionId) -> Result<&mut ForwardProjection, ProjectionError> {
        let index = self
            .forward_index
            .get(&id)
            .copied()
            .ok_or(ProjectionError::MalformedFlow(
                "projection identity missing",
            ))?;
        Ok(&mut self.forward[index])
    }

    fn finish(self) -> RenderProjection {
        RenderProjection {
            forward: self.forward,
            reverse: self.reverse,
            stats: ProjectionStats::default(),
        }
    }
}
