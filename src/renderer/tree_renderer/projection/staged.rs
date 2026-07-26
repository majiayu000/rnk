use std::collections::HashMap;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::core::{ElementId, Style};
use crate::layout::text_flow::{TextFlow, TextFlowPlacement, TextFlowRun};
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
    writes: usize,
    options: ProjectionOptions,
}

impl StagedFrame {
    pub(in crate::renderer::tree_renderer) fn new(
        output: &Output,
        options: ProjectionOptions,
    ) -> Self {
        let snapshot = output.staged_snapshot();
        Self {
            initial_clip_depth: snapshot.clip_depth(),
            output: snapshot,
            projection: ProjectionBuilder::default(),
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
        let mut runs = vec![None; flow.tokens().len()];
        for run in flow.logical_rows().iter().flat_map(|row| row.runs.iter()) {
            runs[run.token_index] = Some(run);
        }

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
            if let Some(run) = runs[token_index] {
                self.project_run(id, run, origin_x, origin_y)?;
            }
        }
        Ok(())
    }

    fn project_run(
        &mut self,
        id: ProjectionId,
        run: &TextFlowRun,
        origin_x: i64,
        origin_y: i64,
    ) -> Result<(), ProjectionError> {
        let base_x = origin_x
            .checked_add(
                i64::try_from(run.column).map_err(|_| ProjectionError::CoordinateOverflow)?,
            )
            .ok_or(ProjectionError::CoordinateOverflow)?;
        let y = origin_y
            .checked_add(i64::try_from(run.row).map_err(|_| ProjectionError::CoordinateOverflow)?)
            .ok_or(ProjectionError::CoordinateOverflow)?;
        let mut fragment_offset = 0_i64;
        let mut measured_width = 0_usize;

        for grapheme in run.text.graphemes(true) {
            let width = UnicodeWidthStr::width(grapheme);
            let x = base_x
                .checked_add(fragment_offset)
                .ok_or(ProjectionError::CoordinateOverflow)?;
            if width == 0 {
                self.write_zero_width(x, y, grapheme)?;
            } else {
                self.project_visible_grapheme(id, x, y, grapheme, width, &run.style)?;
            }
            measured_width = measured_width
                .checked_add(width)
                .ok_or(ProjectionError::CoordinateOverflow)?;
            fragment_offset = fragment_offset
                .checked_add(i64::try_from(width).map_err(|_| ProjectionError::CoordinateOverflow)?)
                .ok_or(ProjectionError::CoordinateOverflow)?;
        }
        if measured_width != run.width {
            return Err(ProjectionError::MalformedFlow(
                "run text width differs from published width",
            ));
        }
        Ok(())
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

    fn write_zero_width(&mut self, x: i64, y: i64, grapheme: &str) -> Result<(), ProjectionError> {
        self.checkpoint()?;
        let _ = self
            .output
            .write_grapheme(x, y, grapheme, &Style::default());
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
        for row_offset in 0..i64::from(height) {
            let row = y
                .checked_add(row_offset)
                .ok_or(ProjectionError::CoordinateOverflow)?;
            for column_offset in 0..i64::from(width) {
                let column = x
                    .checked_add(column_offset)
                    .ok_or(ProjectionError::CoordinateOverflow)?;
                self.paint_grapheme(column, row, " ", style)?;
            }
        }
        Ok(())
    }

    pub(in crate::renderer::tree_renderer) fn clip(&mut self, region: ClipRegion) {
        self.output.clip(region);
    }

    pub(in crate::renderer::tree_renderer) fn unclip(&mut self) {
        self.output.unclip();
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
