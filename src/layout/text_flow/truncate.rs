use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use super::{
    TextFlowError, TextFlowOptions, TextFlowPlacement, TextFlowRow, TextFlowSource, TextFlowToken,
    TokenClass, classify_grapheme, grapheme_width, place_row,
};
use crate::core::TextWrap;

/// Constant-size composition of a token sequence's column transform.
///
/// Before the first tab the sequence adds a fixed width. A tab then aligns to
/// the next stop, after which the remaining width is independent of the
/// starting column modulo whole tab stops.
#[derive(Clone, Copy, Default)]
struct SequenceMetrics {
    width_before_first_tab: usize,
    width_after_first_tab: Option<usize>,
}

impl SequenceMetrics {
    fn prepend(&mut self, token: &TextFlowToken, tab_stop: usize) -> Result<(), TextFlowError> {
        if token.class == TokenClass::Tab {
            let tail_width = self.final_column(0, tab_stop)?;
            self.width_before_first_tab = 0;
            self.width_after_first_tab = Some(tail_width);
        } else {
            self.width_before_first_tab = token
                .display_width
                .checked_add(self.width_before_first_tab)
                .ok_or(TextFlowError::ArithmeticOverflow)?;
        }
        Ok(())
    }

    fn final_column(self, start_column: usize, tab_stop: usize) -> Result<usize, TextFlowError> {
        let before_tab = start_column
            .checked_add(self.width_before_first_tab)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        let Some(tail_width) = self.width_after_first_tab else {
            return Ok(before_tab);
        };
        let tab_width = tab_stop - before_tab % tab_stop;
        before_tab
            .checked_add(tab_width)
            .and_then(|column| column.checked_add(tail_width))
            .ok_or(TextFlowError::ArithmeticOverflow)
    }
}

pub(super) fn truncate_line(
    tokens: &mut Vec<TextFlowToken>,
    range: Range<usize>,
    options: &TextFlowOptions,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    let source_metrics = metrics_for_range(tokens, range.clone(), options.tab_stop, interrupted)?;
    if source_metrics.final_column(0, options.tab_stop)? <= options.max_width {
        let source: Vec<_> = range.collect();
        return place_row(tokens, &source, options.tab_stop, rows);
    }

    let row = rows.len();
    for token in &mut tokens[range.clone()] {
        token.placement = TextFlowPlacement::Truncated { row };
    }

    let (ellipsis, complete_ellipsis) = append_ellipsis(tokens, &range, options, interrupted)?;
    if !complete_ellipsis {
        return place_row(tokens, &ellipsis, options.tab_stop, rows);
    }
    let placed = match options.text_wrap {
        TextWrap::Truncate | TextWrap::TruncateEnd => {
            truncate_end(tokens, range, &ellipsis, options, interrupted)?
        }
        TextWrap::TruncateStart => truncate_start(tokens, range, &ellipsis, options, interrupted)?,
        TextWrap::TruncateMiddle => {
            truncate_middle(tokens, range, &ellipsis, options, interrupted)?
        }
        TextWrap::Wrap => unreachable!("truncate_line is only used for truncate modes"),
    };
    place_row(tokens, &placed, options.tab_stop, rows)
}

fn append_ellipsis(
    tokens: &mut Vec<TextFlowToken>,
    source_range: &Range<usize>,
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(Vec<usize>, bool), TextFlowError> {
    let style = tokens[source_range.start].style.clone();
    let mut candidates = Vec::new();
    for grapheme in options.ellipsis.graphemes(true) {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let (safe_text, class) = classify_grapheme(grapheme);
        let display_width = if class == TokenClass::Tab {
            0
        } else {
            grapheme_width(&safe_text)
        };
        candidates.push(TextFlowToken {
            source: TextFlowSource::Synthetic,
            safe_text,
            style: style.clone(),
            display_width,
            placement: TextFlowPlacement::Omitted { row: 0 },
            class,
        });
    }

    let candidate_indices: Vec<_> = (0..candidates.len()).collect();
    let configured = metrics_for_indices(
        &candidates,
        &candidate_indices,
        options.tab_stop,
        interrupted,
    )?;
    let complete = configured.final_column(0, options.tab_stop)? <= options.max_width;
    let selected = if complete {
        candidates.len()
    } else {
        let mut selected = 0;
        let mut column = 0;
        for candidate in &candidates {
            if interrupted() {
                return Err(TextFlowError::Interrupted);
            }
            let next = token_end_column(candidate, column, options.tab_stop)?;
            if next > options.max_width {
                break;
            }
            selected += 1;
            column = next;
        }
        selected
    };
    let start = tokens.len();
    tokens.extend(candidates.into_iter().take(selected));
    Ok(((start..tokens.len()).collect(), complete))
}

fn truncate_end(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<usize>, TextFlowError> {
    let ellipsis_metrics = metrics_for_indices(tokens, ellipsis, options.tab_stop, interrupted)?;
    let keep = maximum_prefix(
        tokens,
        range.clone(),
        ellipsis_metrics,
        options,
        interrupted,
    )?;
    let mut placed: Vec<_> = (range.start..keep).collect();
    placed.extend_from_slice(ellipsis);
    Ok(placed)
}

fn truncate_start(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<usize>, TextFlowError> {
    let ellipsis_metrics = metrics_for_indices(tokens, ellipsis, options.tab_stop, interrupted)?;
    let suffix_column = ellipsis_metrics.final_column(0, options.tab_stop)?;
    let keep = minimum_suffix(
        tokens,
        range.clone(),
        suffix_column,
        options.max_width,
        options.tab_stop,
        interrupted,
    )?;
    let mut placed = ellipsis.to_vec();
    placed.extend(keep..range.end);
    Ok(placed)
}

fn truncate_middle(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<usize>, TextFlowError> {
    let ellipsis_metrics = metrics_for_indices(tokens, ellipsis, options.tab_stop, interrupted)?;
    let ellipsis_at_zero = ellipsis_metrics.final_column(0, options.tab_stop)?;
    let available = options.max_width.saturating_sub(ellipsis_at_zero);
    let left_budget = available / 2;
    let right_budget = available - left_budget;
    let left_ellipsis_limit = ellipsis_at_zero
        .saturating_add(left_budget)
        .min(options.max_width);
    let (left_end, left_column) = prefix_within(
        tokens,
        range.clone(),
        ellipsis_metrics,
        left_ellipsis_limit,
        options.tab_stop,
        interrupted,
    )?;
    let suffix_column = ellipsis_metrics.final_column(left_column, options.tab_stop)?;
    let suffix_limit = suffix_column
        .saturating_add(right_budget)
        .min(options.max_width);
    let right_start = minimum_suffix(
        tokens,
        left_end..range.end,
        suffix_column,
        suffix_limit,
        options.tab_stop,
        interrupted,
    )?;
    let mut placed: Vec<_> = (range.start..left_end).collect();
    placed.extend_from_slice(ellipsis);
    placed.extend(right_start..range.end);
    Ok(placed)
}

fn maximum_prefix(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    suffix: SequenceMetrics,
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut keep = range.start;
    let mut column = 0;
    for index in range {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let next = token_end_column(&tokens[index], column, options.tab_stop)?;
        if suffix.final_column(next, options.tab_stop)? > options.max_width {
            break;
        }
        keep = index + 1;
        column = next;
    }
    Ok(keep)
}

fn minimum_suffix(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    start_column: usize,
    column_limit: usize,
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut keep = range.end;
    let mut metrics = SequenceMetrics::default();
    for index in range.rev() {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let mut candidate = metrics;
        candidate.prepend(&tokens[index], tab_stop)?;
        if candidate.final_column(start_column, tab_stop)? > column_limit {
            break;
        }
        keep = index;
        metrics = candidate;
    }
    Ok(keep)
}

fn prefix_within(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    suffix: SequenceMetrics,
    suffix_column_limit: usize,
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(usize, usize), TextFlowError> {
    let mut keep = range.start;
    let mut column = 0;
    for index in range {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let next = token_end_column(&tokens[index], column, tab_stop)?;
        if suffix.final_column(next, tab_stop)? > suffix_column_limit {
            break;
        }
        keep = index + 1;
        column = next;
    }
    Ok((keep, column))
}

fn metrics_for_range(
    tokens: &[TextFlowToken],
    range: Range<usize>,
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<SequenceMetrics, TextFlowError> {
    let mut metrics = SequenceMetrics::default();
    for index in range.rev() {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        metrics.prepend(&tokens[index], tab_stop)?;
    }
    Ok(metrics)
}

fn metrics_for_indices(
    tokens: &[TextFlowToken],
    indices: &[usize],
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<SequenceMetrics, TextFlowError> {
    let mut metrics = SequenceMetrics::default();
    for index in indices.iter().rev() {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        metrics.prepend(&tokens[*index], tab_stop)?;
    }
    Ok(metrics)
}

fn token_end_column(
    token: &TextFlowToken,
    column: usize,
    tab_stop: usize,
) -> Result<usize, TextFlowError> {
    let width = if token.class == TokenClass::Tab {
        tab_stop - column % tab_stop
    } else {
        token.display_width
    };
    column
        .checked_add(width)
        .ok_or(TextFlowError::ArithmeticOverflow)
}
