use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use super::{
    TextFlowError, TextFlowOptions, TextFlowPlacement, TextFlowRow, TextFlowSource, TextFlowToken,
    TokenClass, classify_grapheme, grapheme_width, place_row, token_width_at,
};
use crate::core::TextWrap;

pub(super) fn truncate_line(
    tokens: &mut Vec<TextFlowToken>,
    range: Range<usize>,
    options: &TextFlowOptions,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    let source: Vec<_> = range.clone().collect();
    if sequence_width(tokens, &source, options.tab_stop, interrupted)? <= options.max_width {
        return place_row(tokens, &source, options.tab_stop, rows);
    }

    let row = rows.len();
    for token in &mut tokens[range.clone()] {
        token.placement = TextFlowPlacement::Truncated { row };
    }

    let (ellipsis, ellipsis_fills_budget) = append_ellipsis(tokens, &range, options, interrupted)?;
    if ellipsis_fills_budget {
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
    let configured_width = sequence_width(
        &mut candidates,
        &candidate_indices,
        options.tab_stop,
        interrupted,
    )?;
    let mut selected = 0;
    let mut column = 0;
    for candidate in &mut candidates {
        let width = token_width_at(candidate, column, options.tab_stop)?;
        let next = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if next > options.max_width {
            break;
        }
        selected += 1;
        column = next;
    }
    let start = tokens.len();
    tokens.extend(candidates.into_iter().take(selected));
    Ok((
        (start..tokens.len()).collect(),
        configured_width >= options.max_width,
    ))
}

fn truncate_end(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<usize>, TextFlowError> {
    let keep = maximum_prefix(tokens, range.clone(), ellipsis, options, interrupted)?;
    let mut placed: Vec<_> = (range.start..keep).collect();
    placed.extend_from_slice(ellipsis);
    Ok(placed)
}

fn truncate_start(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<usize>, TextFlowError> {
    let keep = minimum_suffix(tokens, range.clone(), ellipsis, options, interrupted)?;
    let mut placed = ellipsis.to_vec();
    placed.extend(keep..range.end);
    Ok(placed)
}

fn truncate_middle(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<usize>, TextFlowError> {
    let ellipsis_width = sequence_width(tokens, ellipsis, options.tab_stop, interrupted)?;
    let available = options.max_width.saturating_sub(ellipsis_width);
    let left_budget = available / 2;
    let right_budget = available - left_budget;
    let left_end = prefix_within(
        tokens,
        range.clone(),
        left_budget,
        options.tab_stop,
        interrupted,
    )?;
    let right_start = suffix_within(
        tokens,
        left_end..range.end,
        right_budget,
        options.tab_stop,
        interrupted,
    )?;
    let mut placed: Vec<_> = (range.start..left_end).collect();
    placed.extend_from_slice(ellipsis);
    placed.extend(right_start..range.end);
    shrink_middle_to_fit(tokens, &mut placed, ellipsis, options, interrupted)?;
    Ok(placed)
}

fn maximum_prefix(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    suffix: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut low = range.start;
    let mut high = range.end;
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let mut indices: Vec<_> = (range.start..middle).collect();
        indices.extend_from_slice(suffix);
        if sequence_width(tokens, &indices, options.tab_stop, interrupted)? <= options.max_width {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(low)
}

fn minimum_suffix(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    prefix: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut low = range.start;
    let mut high = range.end;
    while low < high {
        let middle = low + (high - low) / 2;
        let mut indices = prefix.to_vec();
        indices.extend(middle..range.end);
        if sequence_width(tokens, &indices, options.tab_stop, interrupted)? <= options.max_width {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(low)
}

fn prefix_within(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    budget: usize,
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut column = 0;
    for index in range.clone() {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let width = token_width_at(&mut tokens[index], column, tab_stop)?;
        let next = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if next > budget {
            return Ok(index);
        }
        column = next;
    }
    Ok(range.end)
}

fn suffix_within(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    budget: usize,
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut low = range.start;
    let mut high = range.end;
    while low < high {
        let middle = low + (high - low) / 2;
        let indices: Vec<_> = (middle..range.end).collect();
        if sequence_width(tokens, &indices, tab_stop, interrupted)? <= budget {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(low)
}

fn shrink_middle_to_fit(
    tokens: &mut [TextFlowToken],
    placed: &mut Vec<usize>,
    ellipsis: &[usize],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    while sequence_width(tokens, placed, options.tab_stop, interrupted)? > options.max_width {
        let Some(position) = placed.iter().rposition(|index| !ellipsis.contains(index)) else {
            placed.pop();
            continue;
        };
        placed.remove(position);
    }
    Ok(())
}

fn sequence_width(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut column = 0usize;
    for index in indices {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let width = token_width_at(&mut tokens[*index], column, tab_stop)?;
        column = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
    }
    Ok(column)
}
