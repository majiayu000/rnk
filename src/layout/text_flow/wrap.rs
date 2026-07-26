use std::ops::Range;

use super::{
    TextFlowError, TextFlowOptions, TextFlowPlacement, TextFlowRow, TextFlowToken, TokenClass,
    place_row, token_width_at,
};

pub(super) fn wrap_line(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    options: &TextFlowOptions,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    let mut current = Vec::new();
    let mut current_width = 0;
    let mut pending = Vec::new();
    let mut cursor = range.start;
    while cursor < range.end {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        if matches!(
            tokens[cursor].class,
            TokenClass::Whitespace | TokenClass::Tab
        ) {
            pending.push(cursor);
            cursor += 1;
            continue;
        }
        let word_start = cursor;
        while cursor < range.end
            && !matches!(
                tokens[cursor].class,
                TokenClass::Whitespace | TokenClass::Tab
            )
        {
            cursor += 1;
        }
        let word: Vec<usize> = (word_start..cursor).collect();
        let pending_width = sequence_width(tokens, &pending, current_width, options.tab_stop)?;
        let after_pending = current_width
            .checked_add(pending_width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        let word_width = sequence_width(tokens, &word, after_pending, options.tab_stop)?;
        let combined = after_pending
            .checked_add(word_width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if combined <= options.max_width {
            current.append(&mut pending);
            current.extend(word);
            current_width = combined;
            continue;
        }
        let omitted_row = rows.len();
        if !current.is_empty() {
            place_row(tokens, &current, options.tab_stop, rows)?;
            current.clear();
            current_width = 0;
        }
        for index in pending.drain(..) {
            tokens[index].placement = TextFlowPlacement::Omitted { row: omitted_row };
        }
        let fresh_width = sequence_width(tokens, &word, 0, options.tab_stop)?;
        if fresh_width <= options.max_width {
            current.extend(word);
            current_width = fresh_width;
            continue;
        }
        for index in word {
            let width = token_width_at(&mut tokens[index], current_width, options.tab_stop)?;
            if current_width
                .checked_add(width)
                .ok_or(TextFlowError::ArithmeticOverflow)?
                > options.max_width
                && !current.is_empty()
            {
                place_row(tokens, &current, options.tab_stop, rows)?;
                current.clear();
                current_width = 0;
            }
            current.push(index);
            let width = token_width_at(&mut tokens[index], current_width, options.tab_stop)?;
            current_width = current_width
                .checked_add(width)
                .ok_or(TextFlowError::ArithmeticOverflow)?;
        }
    }
    current.append(&mut pending);
    place_row(tokens, &current, options.tab_stop, rows)
}

fn sequence_width(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    start: usize,
    tab_stop: usize,
) -> Result<usize, TextFlowError> {
    let mut column = start;
    for index in indices {
        let width = token_width_at(&mut tokens[*index], column, tab_stop)?;
        column = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
    }
    Ok(column - start)
}
