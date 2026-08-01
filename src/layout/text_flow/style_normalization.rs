use std::{cmp::Ordering, ops::Range};

use super::{StyledTextRange, TextFlowDiagnostic, TextFlowError, TextFlowInput, TextFlowToken};
use crate::core::Style;

pub(super) const VALIDATION_POLL_INTERVAL: usize = 1_024;

pub(super) trait NormalizationObserver {
    fn grapheme_step(&mut self) -> Result<(), TextFlowError>;
    fn plan_endpoint_visit(&mut self) -> Result<(), TextFlowError>;
    fn style_range_advance(&mut self) -> Result<(), TextFlowError>;
    fn boundary_endpoint_visit(&mut self) -> Result<(), TextFlowError>;
    fn diagnostic_count_visit(&mut self) -> Result<(), TextFlowError>;
    fn diagnostic_bucket_preparation(&mut self) -> Result<(), TextFlowError>;
    fn diagnostic_projection(&mut self) -> Result<(), TextFlowError>;
    fn style_application(&mut self) -> Result<(), TextFlowError>;
}

pub(super) struct NoopNormalizationObserver;

impl NormalizationObserver for NoopNormalizationObserver {
    fn grapheme_step(&mut self) -> Result<(), TextFlowError> {
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

    fn diagnostic_bucket_preparation(&mut self) -> Result<(), TextFlowError> {
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

#[derive(Clone, Copy)]
pub(super) struct ValidatedStyledInput<'a> {
    input: &'a TextFlowInput,
}

pub(super) struct StyledNormalization {
    pub(super) styles: Vec<Style>,
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

fn pollable_sort_by<T: Copy>(
    values: &mut Vec<T>,
    compare: &dyn Fn(&T, &T) -> Ordering,
    poller: &mut ValidationPoller,
    interrupted_callback: &mut dyn FnMut() -> bool,
) -> Result<(), TextFlowError> {
    let len = values.len();
    if len < 2 {
        return Ok(());
    }

    let mut scratch = Vec::new();
    reserve(&mut scratch, len)?;
    let mut width = 1usize;
    while width < len {
        scratch.clear();
        let mut chunk_start = 0usize;
        while chunk_start < len {
            let middle = chunk_start.saturating_add(width).min(len);
            let chunk_end = middle.saturating_add(width).min(len);
            let mut left = chunk_start;
            let mut right = middle;
            while left < middle || right < chunk_end {
                poller.step(interrupted_callback)?;
                let take_left = if left == middle {
                    false
                } else if right == chunk_end {
                    true
                } else {
                    compare(&values[left], &values[right]) != Ordering::Greater
                };
                if take_left {
                    scratch.push(values[left]);
                    left += 1;
                } else {
                    scratch.push(values[right]);
                    right += 1;
                }
            }
            chunk_start = chunk_end;
        }
        std::mem::swap(values, &mut scratch);
        width = width.saturating_mul(2);
    }
    Ok(())
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

    let mut sorted_non_empty = Vec::new();
    reserve(&mut sorted_non_empty, input.styled_ranges.len())?;
    sorted_non_empty.extend(
        input
            .styled_ranges
            .iter()
            .enumerate()
            .filter(|(_, styled)| !styled.range.is_empty())
            .map(|(original_ordinal, styled)| PlannedRange {
                original_ordinal,
                styled,
            }),
    );
    sorted_non_empty
        .sort_unstable_by_key(|planned| (planned.styled.range.start, planned.original_ordinal));
    for pair in sorted_non_empty.windows(2) {
        if pair[0].styled.range.end > pair[1].styled.range.start {
            return Err(TextFlowError::OverlappingStyleRanges {
                first: pair[0].styled.range.clone(),
                second: pair[1].styled.range.clone(),
            });
        }
    }

    Ok(ValidatedStyledInput { input })
}

pub(super) fn build_styled_range_plan<'a>(
    validated: ValidatedStyledInput<'a>,
    interrupted_callback: &mut dyn FnMut() -> bool,
) -> Result<ValidatedStyledRanges<'a>, TextFlowError> {
    let input = validated.input;
    let mut poller = ValidationPoller::default();
    let mut sorted_non_empty = Vec::new();
    reserve(&mut sorted_non_empty, input.styled_ranges.len())?;
    for (original_ordinal, styled) in input.styled_ranges.iter().enumerate() {
        poller.step(interrupted_callback)?;
        if !styled.range.is_empty() {
            sorted_non_empty.push(PlannedRange {
                original_ordinal,
                styled,
            });
        }
    }
    pollable_sort_by(
        &mut sorted_non_empty,
        &|left, right| {
            (left.styled.range.start, left.original_ordinal)
                .cmp(&(right.styled.range.start, right.original_ordinal))
        },
        &mut poller,
        interrupted_callback,
    )?;
    let endpoint_count = checked_endpoint_count(input.styled_ranges.len())?;
    let mut endpoints = Vec::new();
    reserve(&mut endpoints, endpoint_count)?;
    for styled in &input.styled_ranges {
        poller.step(interrupted_callback)?;
        endpoints.push(EndpointEvent {
            boundary: styled.range.start,
        });
        poller.step(interrupted_callback)?;
        endpoints.push(EndpointEvent {
            boundary: styled.range.end,
        });
    }

    let mut sorted_endpoint_indices = Vec::new();
    reserve(&mut sorted_endpoint_indices, endpoint_count)?;
    for index in 0..endpoint_count {
        poller.step(interrupted_callback)?;
        sorted_endpoint_indices.push(index);
    }
    pollable_sort_by(
        &mut sorted_endpoint_indices,
        &|left, right| {
            (endpoints[*left].boundary, *left).cmp(&(endpoints[*right].boundary, *right))
        },
        &mut poller,
        interrupted_callback,
    )?;

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
    interrupted_callback: &mut dyn FnMut() -> bool,
    observer: &mut dyn NormalizationObserver,
) -> Result<StyledNormalization, TextFlowError> {
    if plan.endpoints.is_empty() {
        return Ok(StyledNormalization {
            styles: Vec::new(),
            diagnostics: Vec::new(),
        });
    }

    let mut styles = Vec::new();
    reserve(&mut styles, grapheme_ranges.len())?;
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
        styles.push(style);

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

    let mut diagnostics_by_grapheme = Vec::new();
    reserve(&mut diagnostics_by_grapheme, grapheme_ranges.len())?;
    for count in diagnostic_counts {
        interrupted(interrupted_callback)?;
        observer.diagnostic_bucket_preparation()?;
        let mut bucket = Vec::new();
        reserve(&mut bucket, count)?;
        diagnostics_by_grapheme.push(bucket);
    }

    for (event, target) in plan.endpoints.iter().zip(endpoint_targets) {
        interrupted(interrupted_callback)?;
        observer.plan_endpoint_visit()?;
        if let Some(grapheme_ordinal) = target {
            diagnostics_by_grapheme[grapheme_ordinal].push(
                TextFlowDiagnostic::StyleBoundaryNormalized {
                    boundary: event.boundary,
                    grapheme_range: grapheme_ranges[grapheme_ordinal].clone(),
                },
            );
        }
    }

    let mut diagnostics = Vec::new();
    reserve(&mut diagnostics, diagnostic_count)?;
    for bucket in diagnostics_by_grapheme {
        interrupted(interrupted_callback)?;
        for diagnostic in bucket {
            interrupted(interrupted_callback)?;
            observer.diagnostic_projection()?;
            diagnostics.push(diagnostic);
        }
    }

    Ok(StyledNormalization {
        styles,
        diagnostics,
    })
}

pub(super) fn apply_styles(
    targets: &mut [TextFlowToken],
    styles: Vec<Style>,
    interrupted_callback: &mut dyn FnMut() -> bool,
    observer: &mut dyn NormalizationObserver,
) -> Result<(), TextFlowError> {
    for (target, style) in targets.iter_mut().zip(styles) {
        interrupted(interrupted_callback)?;
        observer.style_application()?;
        target.style = style;
    }
    Ok(())
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct NormalizationOperations {
    pub(super) grapheme_steps: usize,
    pub(super) plan_endpoint_visits: usize,
    pub(super) style_range_advances: usize,
    pub(super) boundary_endpoint_visits: usize,
    pub(super) diagnostic_count_visits: usize,
    pub(super) diagnostic_bucket_preparations: usize,
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
            self.plan_endpoint_visits,
            self.style_range_advances,
            self.boundary_endpoint_visits,
            self.diagnostic_count_visits,
            self.diagnostic_bucket_preparations,
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

    fn diagnostic_bucket_preparation(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.diagnostic_bucket_preparations)
    }

    fn diagnostic_projection(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.diagnostic_projections)
    }

    fn style_application(&mut self) -> Result<(), TextFlowError> {
        Self::increment(&mut self.style_applications)
    }
}
