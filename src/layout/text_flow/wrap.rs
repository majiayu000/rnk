use std::ops::Range;

use super::{
    TextFlowError, TextFlowOptions, TextFlowRow, TextFlowToken, TokenClass, place_row,
    token_width_at,
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
            if interrupted() {
                return Err(TextFlowError::Interrupted);
            }
            cursor += 1;
        }
        let word: Vec<usize> = (word_start..cursor).collect();
        let pending_width = sequence_width(
            tokens,
            &pending,
            current_width,
            options.tab_stop,
            interrupted,
        )?;
        let after_pending = current_width
            .checked_add(pending_width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        let word_width =
            sequence_width(tokens, &word, after_pending, options.tab_stop, interrupted)?;
        let combined = after_pending
            .checked_add(word_width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if combined <= options.max_width {
            current.append(&mut pending);
            current.extend(word);
            current_width = combined;
            continue;
        }
        append_wrapped(
            tokens,
            &pending,
            &mut current,
            &mut current_width,
            options,
            rows,
            interrupted,
        )?;
        pending.clear();
        let word_width =
            sequence_width(tokens, &word, current_width, options.tab_stop, interrupted)?;
        let with_word = current_width
            .checked_add(word_width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if with_word <= options.max_width {
            current.extend(word);
            current_width = with_word;
            continue;
        }
        let fresh_width = sequence_width(tokens, &word, 0, options.tab_stop, interrupted)?;
        if fresh_width <= options.max_width {
            if !current.is_empty() {
                place_row_interruptible(tokens, &current, options.tab_stop, rows, interrupted)?;
                current.clear();
            }
            current.extend(word);
            current_width = fresh_width;
            continue;
        }
        append_wrapped(
            tokens,
            &word,
            &mut current,
            &mut current_width,
            options,
            rows,
            interrupted,
        )?;
    }
    append_wrapped(
        tokens,
        &pending,
        &mut current,
        &mut current_width,
        options,
        rows,
        interrupted,
    )?;
    place_row_interruptible(tokens, &current, options.tab_stop, rows, interrupted)
}

fn append_wrapped(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    current: &mut Vec<usize>,
    current_width: &mut usize,
    options: &TextFlowOptions,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    for index in indices {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let width = token_width_at(&mut tokens[*index], *current_width, options.tab_stop)?;
        let combined = current_width
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if combined > options.max_width && !current.is_empty() {
            place_row_interruptible(tokens, current, options.tab_stop, rows, interrupted)?;
            current.clear();
            *current_width = 0;
        }
        current.push(*index);
        let width = token_width_at(&mut tokens[*index], *current_width, options.tab_stop)?;
        *current_width = current_width
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
    }
    Ok(())
}

fn sequence_width(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    start: usize,
    tab_stop: usize,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<usize, TextFlowError> {
    let mut column = start;
    for index in indices {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let width = token_width_at(&mut tokens[*index], column, tab_stop)?;
        column = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
    }
    Ok(column - start)
}

fn place_row_interruptible(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    tab_stop: usize,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    if interrupted() {
        return Err(TextFlowError::Interrupted);
    }
    place_row(tokens, indices, tab_stop, rows)
}
