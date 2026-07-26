use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::core::{Display, Element, ElementId};
use crate::layout::LayoutEngine;
use crate::layout::text_flow::{
    TextFlow, TextFlowPlacement, TextFlowRun, TextFlowSource, TextFlowToken,
};
use crate::renderer::Output;

mod staged;

pub(super) use staged::StagedFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct FrameCell {
    pub(super) x: u16,
    pub(super) y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SignedCell {
    pub(super) x: i64,
    pub(super) y: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ProjectionId {
    pub(super) element_id: ElementId,
    pub(super) token_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CellOrigin {
    Source {
        id: ProjectionId,
        range: Range<usize>,
    },
    Synthetic {
        id: ProjectionId,
    },
}

impl CellOrigin {
    pub(super) fn id(&self) -> ProjectionId {
        match self {
            Self::Source { id, .. } | Self::Synthetic { id } => *id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NonCellDisposition {
    HardBreak,
    ZeroWidth,
    Omitted,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FrameDisposition {
    Cells {
        visible: Vec<FrameCell>,
        clipped: Vec<SignedCell>,
        replaced: Vec<FrameCell>,
    },
    NonCell(NonCellDisposition),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ForwardProjection {
    pub(super) id: ProjectionId,
    pub(super) source: TextFlowSource,
    pub(super) logical: TextFlowPlacement,
    pub(super) text: String,
    pub(super) display_width: usize,
    pub(super) frame: FrameDisposition,
}

impl ForwardProjection {
    pub(super) fn origin(&self) -> CellOrigin {
        match &self.source {
            TextFlowSource::Source { range, .. } => CellOrigin::Source {
                id: self.id,
                range: range.clone(),
            },
            TextFlowSource::Synthetic => CellOrigin::Synthetic { id: self.id },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ProjectionStats {
    pub(super) validation_visits: usize,
    pub(super) committed_replacements: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RenderProjection {
    pub(super) forward: Vec<ForwardProjection>,
    pub(super) reverse: HashMap<FrameCell, CellOrigin>,
    pub(super) stats: ProjectionStats,
}

impl RenderProjection {
    #[cfg(test)]
    pub(super) fn forward_for(
        &self,
        element_id: ElementId,
        token_index: usize,
    ) -> Option<&ForwardProjection> {
        self.forward.iter().find(|record| {
            record.id.element_id == element_id && record.id.token_index == token_index
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProjectionOptions {
    pub(super) fail_after_writes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProjectionError {
    MissingCurrentFlow(ElementId),
    NonFiniteCoordinate,
    CoordinateOverflow,
    MalformedFlow(&'static str),
    DuplicateForwardRecord(ProjectionId),
    DuplicateReverseCell(FrameCell),
    MalformedProjection(&'static str),
    WriterOutcomeMismatch,
    UnbalancedClipStack,
    InjectedFailure,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCurrentFlow(id) => {
                write!(formatter, "missing current TextFlow for element {id:?}")
            }
            Self::NonFiniteCoordinate => write!(formatter, "non-finite render coordinate"),
            Self::CoordinateOverflow => write!(formatter, "render coordinate overflow"),
            Self::MalformedFlow(reason) => {
                write!(formatter, "malformed current TextFlow: {reason}")
            }
            Self::DuplicateForwardRecord(id) => {
                write!(formatter, "duplicate projection record {id:?}")
            }
            Self::DuplicateReverseCell(cell) => {
                write!(formatter, "duplicate reverse projection cell {cell:?}")
            }
            Self::MalformedProjection(reason) => {
                write!(formatter, "malformed render projection: {reason}")
            }
            Self::WriterOutcomeMismatch => write!(formatter, "staged writer outcome mismatch"),
            Self::UnbalancedClipStack => write!(formatter, "unbalanced staged clip stack"),
            Self::InjectedFailure => write!(formatter, "injected staged projection failure"),
        }
    }
}

pub(super) fn try_render_tree(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) -> Result<RenderProjection, ProjectionError> {
    try_render_tree_with_options(
        element,
        layout_engine,
        output,
        offset_x,
        offset_y,
        ProjectionOptions::default(),
    )
}

pub(super) fn try_render_tree_with_options(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
    options: ProjectionOptions,
) -> Result<RenderProjection, ProjectionError> {
    validate_tree_flows(element, layout_engine)?;
    let mut staged = StagedFrame::new(output, options);
    super::render_element_tree_staged(
        element,
        layout_engine,
        &mut staged,
        offset_x,
        offset_y,
        None,
    )?;
    let (staged_output, mut projection) = staged.finish()?;
    output.commit_staged(staged_output);
    projection.stats.committed_replacements = 1;
    Ok(projection)
}

fn validate_tree_flows(
    element: &Element,
    layout_engine: &LayoutEngine,
) -> Result<(), ProjectionError> {
    if element.style.display == Display::None {
        return Ok(());
    }
    if element.spans.is_some() || element.text_content.is_some() {
        let flow = layout_engine
            .current_text_flow(element.id)
            .ok_or(ProjectionError::MissingCurrentFlow(element.id))?;
        validate_flow(&flow)?;
    }
    for child in &element.children {
        validate_tree_flows(child, layout_engine)?;
    }
    Ok(())
}

fn validate_flow(flow: &TextFlow) -> Result<(), ProjectionError> {
    let tokens = flow.tokens();
    let mut runs = vec![None; tokens.len()];
    for (row_index, row) in flow.logical_rows().iter().enumerate() {
        if row.index != row_index {
            return Err(ProjectionError::MalformedFlow("logical row index mismatch"));
        }
        for run in &row.runs {
            let slot = runs
                .get_mut(run.token_index)
                .ok_or(ProjectionError::MalformedFlow(
                    "run token index out of range",
                ))?;
            if slot.replace(run).is_some() {
                return Err(ProjectionError::MalformedFlow("duplicate run token index"));
            }
            if run.row != row.index {
                return Err(ProjectionError::MalformedFlow("run row mismatch"));
            }
        }
    }

    let mut source_covered = 0;
    let source = &flow.cache_identity().input.source;
    for (token_index, token) in tokens.iter().enumerate() {
        validate_source_token(token, source, &mut source_covered)?;
        validate_token_run(token, runs[token_index])?;
    }
    if source_covered != source.len() {
        return Err(ProjectionError::MalformedFlow("source coverage gap"));
    }
    validate_position_map(flow)?;
    Ok(())
}

fn validate_source_token(
    token: &TextFlowToken,
    source: &str,
    covered: &mut usize,
) -> Result<(), ProjectionError> {
    let TextFlowSource::Source { range, .. } = &token.source else {
        return Ok(());
    };
    if range.start != *covered || range.end > source.len() {
        return Err(ProjectionError::MalformedFlow(
            "source range gap or overlap",
        ));
    }
    let grapheme = source
        .get(range.clone())
        .ok_or(ProjectionError::MalformedFlow("source range boundary"))?;
    if grapheme.graphemes(true).count() != 1 {
        return Err(ProjectionError::MalformedFlow(
            "source range is not one complete EGC",
        ));
    }
    *covered = range.end;
    Ok(())
}

fn validate_token_run(
    token: &TextFlowToken,
    run: Option<&TextFlowRun>,
) -> Result<(), ProjectionError> {
    let expected_position = match token.placement() {
        TextFlowPlacement::Positioned { row, column }
        | TextFlowPlacement::ZeroWidth { row, column }
        | TextFlowPlacement::SanitizedControl { row, column }
        | TextFlowPlacement::Synthetic { row, column } => Some((*row, *column)),
        TextFlowPlacement::HardBreak { .. }
        | TextFlowPlacement::Omitted { .. }
        | TextFlowPlacement::Truncated { .. } => None,
    };
    match (expected_position, run) {
        (None, None) => Ok(()),
        (Some((row, column)), Some(run))
            if run.row == row
                && run.column == column
                && run.width == token.display_width
                && run.text == token.safe_text
                && run.style == token.style =>
        {
            Ok(())
        }
        (None, Some(_)) => Err(ProjectionError::MalformedFlow(
            "non-positioned token has a run",
        )),
        (Some(_), None) => Err(ProjectionError::MalformedFlow(
            "positioned token lacks a run",
        )),
        (Some(_), Some(_)) => Err(ProjectionError::MalformedFlow(
            "token and run geometry differ",
        )),
    }
}

fn validate_position_map(flow: &TextFlow) -> Result<(), ProjectionError> {
    if flow.position_map().len() != flow.tokens().len() {
        return Err(ProjectionError::MalformedFlow(
            "position map length mismatch",
        ));
    }
    let mut seen = vec![false; flow.tokens().len()];
    for entry in flow.position_map() {
        let token = flow
            .tokens()
            .get(entry.token_index)
            .ok_or(ProjectionError::MalformedFlow(
                "position map token out of range",
            ))?;
        if std::mem::replace(&mut seen[entry.token_index], true)
            || entry.placement != token.placement
            || entry.source != token.source
        {
            return Err(ProjectionError::MalformedFlow(
                "position map identity mismatch",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_round_trip(projection: &RenderProjection) -> Result<usize, ProjectionError> {
    let mut seen_forward = HashSet::with_capacity(projection.forward.len());
    let mut seen_cells = HashMap::with_capacity(projection.reverse.len());
    let mut occupied = 0;
    let mut visits = 0;

    for record in &projection.forward {
        visits += 1;
        if !seen_forward.insert(record.id) {
            return Err(ProjectionError::DuplicateForwardRecord(record.id));
        }
        let FrameDisposition::Cells { visible, .. } = &record.frame else {
            continue;
        };
        let FrameDisposition::Cells {
            clipped, replaced, ..
        } = &record.frame
        else {
            unreachable!("the cell disposition was matched above");
        };
        let expected = record.origin();
        let mut occupied_by_record =
            HashSet::with_capacity(visible.len() + clipped.len() + replaced.len());
        let mut row = None;
        let mut min_x = i64::MAX;
        let mut max_x = i64::MIN;
        for cell in visible {
            visits += 1;
            occupied += 1;
            let signed = SignedCell {
                x: i64::from(cell.x),
                y: i64::from(cell.y),
            };
            validate_projected_cell(
                signed,
                &mut occupied_by_record,
                &mut row,
                &mut min_x,
                &mut max_x,
            )?;
            if seen_cells.insert(*cell, record.id).is_some() {
                return Err(ProjectionError::DuplicateReverseCell(*cell));
            }
            if projection.reverse.get(cell) != Some(&expected) {
                return Err(ProjectionError::WriterOutcomeMismatch);
            }
        }
        for cell in clipped {
            visits += 1;
            validate_projected_cell(
                *cell,
                &mut occupied_by_record,
                &mut row,
                &mut min_x,
                &mut max_x,
            )?;
        }
        for cell in replaced {
            visits += 1;
            validate_projected_cell(
                SignedCell {
                    x: i64::from(cell.x),
                    y: i64::from(cell.y),
                },
                &mut occupied_by_record,
                &mut row,
                &mut min_x,
                &mut max_x,
            )?;
        }
        if occupied_by_record.len() != record.display_width {
            return Err(ProjectionError::MalformedProjection(
                "cell count differs from token width",
            ));
        }
        if record.display_width > 0 {
            let span = max_x
                .checked_sub(min_x)
                .and_then(|value| value.checked_add(1))
                .and_then(|value| usize::try_from(value).ok())
                .ok_or(ProjectionError::CoordinateOverflow)?;
            if span != record.display_width {
                return Err(ProjectionError::MalformedProjection(
                    "token cells contain a gap",
                ));
            }
        }
    }
    visits += projection.reverse.len();
    if occupied != projection.reverse.len() {
        return Err(ProjectionError::WriterOutcomeMismatch);
    }
    for cell in projection.reverse.keys() {
        if !seen_cells.contains_key(cell) {
            return Err(ProjectionError::WriterOutcomeMismatch);
        }
    }
    Ok(visits)
}

fn validate_projected_cell(
    cell: SignedCell,
    occupied: &mut HashSet<SignedCell>,
    row: &mut Option<i64>,
    min_x: &mut i64,
    max_x: &mut i64,
) -> Result<(), ProjectionError> {
    if row.is_some_and(|expected| expected != cell.y) {
        return Err(ProjectionError::MalformedProjection(
            "one token spans multiple frame rows",
        ));
    }
    *row = Some(cell.y);
    *min_x = (*min_x).min(cell.x);
    *max_x = (*max_x).max(cell.x);
    if !occupied.insert(cell) {
        return Err(ProjectionError::MalformedProjection("duplicate token cell"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
