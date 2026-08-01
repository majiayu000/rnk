use std::ops::Range;

use super::{StyledTextRange, TextFlowDiagnostic, TextFlowError, TextFlowInput, TextFlowToken};

pub(super) const VALIDATION_POLL_INTERVAL: usize = 1_024;

pub(super) trait NormalizationObserver {
    fn grapheme_step(&mut self) -> Result<(), TextFlowError>;
    fn plan_construction_step(&mut self) -> Result<(), TextFlowError>;
    fn plan_endpoint_visit(&mut self) -> Result<(), TextFlowError>;
    fn style_range_advance(&mut self) -> Result<(), TextFlowError>;
    fn boundary_endpoint_visit(&mut self) -> Result<(), TextFlowError>;
    fn diagnostic_count_visit(&mut self) -> Result<(), TextFlowError>;
    fn diagnostic_offset_preparation(&mut self) -> Result<(), TextFlowError>;
    fn diagnostic_projection(&mut self) -> Result<(), TextFlowError>;
    fn style_application(&mut self) -> Result<(), TextFlowError>;
}

pub(super) struct NoopNormalizationObserver;

impl NormalizationObserver for NoopNormalizationObserver {
    fn grapheme_step(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn plan_construction_step(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn plan_endpoint_visit(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn style_range_advance(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn boundary_endpoint_visit(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn diagnostic_count_visit(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn diagnostic_offset_preparation(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn diagnostic_projection(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }

    fn style_application(&mut self) -> Result<(), TextFlowError> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct PlannedRange<'a> {
    original_ordinal: usize,
    start_event_index: usize,
    end_event_index: usize,
    styled: &'a StyledTextRange,
}

#[derive(Clone, Copy)]
struct EndpointEvent {
    boundary: usize,
}

pub(super) struct ValidatedStyledRanges<'a> {
    input: &'a TextFlowInput,
    sorted_non_empty: Vec<PlannedRange<'a>>,
    endpoints: Vec<EndpointEvent>,
    sorted_endpoint_indices: Vec<usize>,
}

pub(super) struct ValidatedStyledInput<'a> {
    input: &'a TextFlowInput,
    endpoint_count: usize,
    sorted_all: Vec<PlannedRange<'a>>,
    sorted_non_empty: Vec<PlannedRange<'a>>,
}

pub(super) struct StyledNormalization {
    pub(super) diagnostics: Vec<TextFlowDiagnostic>,
}

pub(super) fn reserve<T>(values: &mut Vec<T>, additional: usize) -> Result<(), TextFlowError> {
    values
        .try_reserve_exact(additional)
        .map_err(|_| TextFlowError::ArithmeticOverflow)
}

pub(super) fn checked_add(left: usize, right: usize) -> Result<usize, TextFlowError> {
    left.checked_add(right)
        .ok_or(TextFlowError::ArithmeticOverflow)
}

pub(super) fn checked_endpoint_count(range_count: usize) -> Result<usize, TextFlowError> {
    range_count
        .checked_mul(2)
        .ok_or(TextFlowError::ArithmeticOverflow)
}

fn interrupted(interrupted: &mut dyn FnMut() -> bool) -> Result<(), TextFlowError> {
    if interrupted() {
        Err(TextFlowError::Interrupted)
    } else {
        Ok(())
    }
}

#[derive(Default)]
struct ValidationPoller {
    steps: usize,
}

impl ValidationPoller {
    fn step(
        &mut self,
        interrupted_callback: &mut dyn FnMut() -> bool,
    ) -> Result<(), TextFlowError> {
        self.steps += 1;
        if self.steps == VALIDATION_POLL_INTERVAL {
            self.steps = 0;
            interrupted(interrupted_callback)
        } else {
            Ok(())
        }
    }
}

pub(super) fn validate_styled_ranges(
    input: &TextFlowInput,
) -> Result<ValidatedStyledInput<'_>, TextFlowError> {
    for styled in &input.styled_ranges {
        let range = &styled.range;
        if range.start > range.end
            || range.end > input.source.len()
            || !input.source.is_char_boundary(range.start)
            || !input.source.is_char_boundary(range.end)
        {
            return Err(TextFlowError::InvalidStyleRange {
                range: range.clone(),
            });
        }
    }

    let endpoint_count = checked_endpoint_count(input.styled_ranges.len())?;
    let mut sorted_all = Vec::new();
    reserve(&mut sorted_all, input.styled_ranges.len())?;
    sorted_all.extend(
        input
            .styled_ranges
            .iter()
            .enumerate()
            .map(|(original_ordinal, styled)| {
                let start_event_index = original_ordinal * 2;
                PlannedRange {
                    original_ordinal,
                    start_event_index,
                    end_event_index: start_event_index + 1,
                    styled,
                }
            }),
    );
    sorted_all
        .sort_unstable_by_key(|planned| (planned.styled.range.start, planned.original_ordinal));

    let mut sorted_non_empty = Vec::new();
    reserve(&mut sorted_non_empty, sorted_all.len())?;
    sorted_non_empty.extend(
        sorted_all
            .iter()
            .copied()
            .filter(|planned| !planned.styled.range.is_empty()),
    );
    for pair in sorted_non_empty.windows(2) {
        if pair[0].styled.range.end > pair[1].styled.range.start {
            return Err(TextFlowError::OverlappingStyleRanges {
                first: pair[0].styled.range.clone(),
                second: pair[1].styled.range.clone(),
            });
        }
    }

    Ok(ValidatedStyledInput {
        input,
        endpoint_count,
        sorted_all,
        sorted_non_empty,
    })
}

#[derive(Clone, Copy)]
enum EndpointStream {
    Start,
    NonEmptyEnd,
    EmptyEnd,
}

fn next_endpoint_candidate(
    sorted_all: &[PlannedRange<'_>],
    cursor: &mut usize,
    stream: EndpointStream,
    poller: &mut ValidationPoller,
    interrupted_callback: &mut dyn FnMut() -> bool,
) -> Result<Option<(usize, usize)>, TextFlowError> {
    while let Some(planned) = sorted_all.get(*cursor) {
        poller.step(interrupted_callback)?;
        // `cursor < sorted_all.len()` and the endpoint-count check bounds len to usize::MAX / 2.
        *cursor += 1;
        let is_empty = planned.styled.range.is_empty();
        let matches = match stream {
            EndpointStream::Start => true,
            EndpointStream::NonEmptyEnd => !is_empty,
            EndpointStream::EmptyEnd => is_empty,
        };
        if !matches {
            continue;
        }
        let (boundary, event_index) = match stream {
            EndpointStream::Start => (planned.styled.range.start, planned.start_event_index),
            EndpointStream::NonEmptyEnd | EndpointStream::EmptyEnd => {
                (planned.styled.range.end, planned.end_event_index)
            }
        };
        return Ok(Some((boundary, event_index)));
    }
    Ok(None)
}

pub(super) fn build_styled_range_plan<'a>(
    validated: ValidatedStyledInput<'a>,
    interrupted_callback: &mut dyn FnMut() -> bool,
    observer: &mut dyn NormalizationObserver,
) -> Result<ValidatedStyledRanges<'a>, TextFlowError> {
    let ValidatedStyledInput {
        input,
        endpoint_count,
        sorted_all,
        sorted_non_empty,
    } = validated;
    let mut poller = ValidationPoller::default();
    let mut endpoints = Vec::new();
    reserve(&mut endpoints, endpoint_count)?;
    for styled in &input.styled_ranges {
        poller.step(interrupted_callback)?;
        endpoints.push(EndpointEvent {
            boundary: styled.range.start,
        });
        poller.step(interrupted_callback)?;
        observer.plan_construction_step()?;
        endpoints.push(EndpointEvent {
            boundary: styled.range.end,
        });
    }

    let mut sorted_endpoint_indices = Vec::new();
    reserve(&mut sorted_endpoint_indices, endpoint_count)?;
    let streams = [
        EndpointStream::Start,
        EndpointStream::NonEmptyEnd,
        EndpointStream::EmptyEnd,
    ];
    let mut cursors = [0usize; 3];
    let mut candidates = [None; 3];
    for stream_index in 0..streams.len() {
        candidates[stream_index] = next_endpoint_candidate(
            &sorted_all,
            &mut cursors[stream_index],
            streams[stream_index],
            &mut poller,
            interrupted_callback,
        )?;
    }
    while sorted_endpoint_indices.len() < endpoint_count {
        let stream_index = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| candidate.map(|value| (index, value)))
            .min_by_key(|(_, value)| *value)
            .map(|(index, _)| index)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        let (_, event_index) = candidates[stream_index]
            .take()
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        poller.step(interrupted_callback)?;
        observer.plan_construction_step()?;
        sorted_endpoint_indices.push(event_index);
        let stream = streams[stream_index];
        candidates[stream_index] = next_endpoint_candidate(
            &sorted_all,
            &mut cursors[stream_index],
            stream,
            &mut poller,
            interrupted_callback,
        )?;
    }

    Ok(ValidatedStyledRanges {
        input,
        sorted_non_empty,
        endpoints,
        sorted_endpoint_indices,
    })
}

pub(super) fn normalize_source(
    plan: &ValidatedStyledRanges<'_>,
    grapheme_ranges: &[Range<usize>],
    targets: &mut [TextFlowToken],
    interrupted_callback: &mut dyn FnMut() -> bool,
    observer: &mut dyn NormalizationObserver,
) -> Result<StyledNormalization, TextFlowError> {
    if targets.len() != grapheme_ranges.len() {
        return Err(TextFlowError::ArithmeticOverflow);
    }
    if plan.endpoints.is_empty() {
        return Ok(StyledNormalization {
            diagnostics: Vec::new(),
        });
    }

    let mut endpoint_targets = Vec::new();
    reserve(&mut endpoint_targets, plan.endpoints.len())?;
    for _ in &plan.endpoints {
        interrupted(interrupted_callback)?;
        endpoint_targets.push(None);
    }

    let mut style_cursor = 0usize;
    let mut endpoint_cursor = 0usize;
    for (grapheme_ordinal, range) in grapheme_ranges.iter().enumerate() {
        interrupted(interrupted_callback)?;
        observer.grapheme_step()?;

        while let Some(candidate) = plan.sorted_non_empty.get(style_cursor)
            && candidate.styled.range.end <= range.start
        {
            interrupted(interrupted_callback)?;
            observer.style_range_advance()?;
            style_cursor += 1;
        }
        let style = plan
            .sorted_non_empty
            .get(style_cursor)
            .filter(|candidate| {
                candidate.styled.range.start <= range.start
                    && range.start < candidate.styled.range.end
            })
            .map_or_else(
                || plan.input.default_style.clone(),
                |candidate| candidate.styled.style.clone(),
            );
        interrupted(interrupted_callback)?;
        observer.style_application()?;
        targets[grapheme_ordinal].style = style;

        while let Some(index) = plan.sorted_endpoint_indices.get(endpoint_cursor).copied() {
            let boundary = plan.endpoints[index].boundary;
            if boundary > range.start {
                break;
            }
            interrupted(interrupted_callback)?;
            observer.boundary_endpoint_visit()?;
            endpoint_cursor += 1;
        }
        while let Some(index) = plan.sorted_endpoint_indices.get(endpoint_cursor).copied() {
            let boundary = plan.endpoints[index].boundary;
            if boundary >= range.end {
                break;
            }
            interrupted(interrupted_callback)?;
            observer.boundary_endpoint_visit()?;
            endpoint_targets[index] = Some(grapheme_ordinal);
            endpoint_cursor += 1;
        }
    }
    while plan.sorted_endpoint_indices.get(endpoint_cursor).is_some() {
        interrupted(interrupted_callback)?;
        observer.boundary_endpoint_visit()?;
        endpoint_cursor += 1;
    }

    let mut diagnostic_counts = Vec::new();
    reserve(&mut diagnostic_counts, grapheme_ranges.len())?;
    for _ in grapheme_ranges {
        interrupted(interrupted_callback)?;
        diagnostic_counts.push(0usize);
    }
    let mut diagnostic_count = 0usize;
    for target in &endpoint_targets {
        interrupted(interrupted_callback)?;
        observer.diagnostic_count_visit()?;
        if let Some(grapheme_ordinal) = *target {
            diagnostic_counts[grapheme_ordinal] =
                checked_add(diagnostic_counts[grapheme_ordinal], 1)?;
            diagnostic_count = checked_add(diagnostic_count, 1)?;
        }
    }

    let mut next_offset = 0usize;
    for count in &mut diagnostic_counts {
        interrupted(interrupted_callback)?;
        observer.diagnostic_offset_preparation()?;
        let diagnostic_count_for_grapheme = *count;
        *count = next_offset;
        next_offset = checked_add(next_offset, diagnostic_count_for_grapheme)?;
    }
    let mut ordered_event_indices = Vec::new();
    reserve(&mut ordered_event_indices, diagnostic_count)?;
    for _ in 0..diagnostic_count {
        interrupted(interrupted_callback)?;
        ordered_event_indices.push(0usize);
    }
    for (event_index, target) in endpoint_targets.iter().copied().enumerate() {
        interrupted(interrupted_callback)?;
        observer.plan_endpoint_visit()?;
        if let Some(grapheme_ordinal) = target {
            let offset = diagnostic_counts
                .get_mut(grapheme_ordinal)
                .ok_or(TextFlowError::ArithmeticOverflow)?;
            let slot = ordered_event_indices
                .get_mut(*offset)
                .ok_or(TextFlowError::ArithmeticOverflow)?;
            *slot = event_index;
            *offset = checked_add(*offset, 1)?;
        }
    }

    let mut diagnostics = Vec::new();
    reserve(&mut diagnostics, diagnostic_count)?;
    for event_index in ordered_event_indices {
        interrupted(interrupted_callback)?;
        observer.diagnostic_projection()?;
        let grapheme_ordinal = endpoint_targets
            .get(event_index)
            .copied()
            .flatten()
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        diagnostics.push(TextFlowDiagnostic::StyleBoundaryNormalized {
            boundary: plan.endpoints[event_index].boundary,
            grapheme_range: grapheme_ranges[grapheme_ordinal].clone(),
        });
    }

    Ok(StyledNormalization { diagnostics })
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct NormalizationOperations {
    pub(super) grapheme_steps: usize,
    pub(super) plan_construction_steps: usize,
    pub(super) plan_endpoint_visits: usize,
    pub(super) style_range_advances: usize,
    pub(super) boundary_endpoint_visits: usize,
    pub(super) diagnostic_count_visits: usize,
    pub(super) diagnostic_offset_preparations: usize,
    pub(super) diagnostic_projections: usize,
    pub(super) style_applications: usize,
}

#[cfg(test)]
impl NormalizationOperations {
    fn increment(value: &mut usize) -> Result<(), TextFlowError> {
        *value = checked_add(*value, 1)?;
        Ok(())
    }

    pub(super) fn total(&self) -> Result<usize, TextFlowError> {
        let mut total = 0usize;
        for value in [
            self.grapheme_steps,
            self.plan_construction_steps,
            self.plan_endpoint_visits,
            self.style_range_advances,
            self.boundary_endpoint_visits,
            self.diagnostic_count_visits,
            self.diagnostic_offset_preparations,
            self.diagnostic_projections,
            self.style_applications,
        ] {
            total = checked_add(total, value)?;
        }
        Ok(total)
    }
}

#[cfg(test)]
impl NormalizationObserver for NormalizationOperations {
    fn grapheme_step(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.grapheme_steps)
    }

    fn plan_construction_step(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.plan_construction_steps)
    }

    fn plan_endpoint_visit(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.plan_endpoint_visits)
    }

    fn style_range_advance(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.style_range_advances)
    }

    fn boundary_endpoint_visit(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.boundary_endpoint_visits)
    }

    fn diagnostic_count_visit(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.diagnostic_count_visits)
    }

    fn diagnostic_offset_preparation(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.diagnostic_offset_preparations)
    }

    fn diagnostic_projection(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.diagnostic_projections)
    }

    fn style_application(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.style_applications)
    }
}
