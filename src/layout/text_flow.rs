//! Canonical logical text flow.
//!
//! The complete preserved source is segmented exactly once. Source identity
//! stays separate from placement, so sanitized and synthetic output cannot
//! forge source ranges.

use std::{error::Error, fmt, ops::Range, sync::Arc};

use unicode_segmentation::UnicodeSegmentation;

use crate::core::{Overflow, Style, TextWrap};

use super::measure::grapheme_width;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFlowSourceKind {
    Exact,
    Canonical,
    Reconstructed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFlowSource {
    Source {
        range: Range<usize>,
        kind: TextFlowSourceKind,
    },
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFlowPlacement {
    Positioned { row: usize, column: usize },
    ZeroWidth { row: usize, column: usize },
    SanitizedControl { row: usize, column: usize },
    HardBreak { row: usize },
    Omitted,
    Truncated,
    Synthetic { row: usize, column: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StyledTextRange {
    pub range: Range<usize>,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextFlowInput {
    pub source: String,
    pub source_kind: TextFlowSourceKind,
    pub default_style: Style,
    pub styled_ranges: Vec<StyledTextRange>,
}

impl TextFlowInput {
    pub fn plain(
        source: impl Into<String>,
        source_kind: TextFlowSourceKind,
        default_style: Style,
    ) -> Self {
        Self {
            source: source.into(),
            source_kind,
            default_style,
            styled_ranges: Vec::new(),
        }
    }

    pub fn with_styled_ranges(mut self, styled_ranges: Vec<StyledTextRange>) -> Self {
        self.styled_ranges = styled_ranges;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnicodeWidthPolicy {
    pub revision: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFlowOptions {
    pub max_width: usize,
    pub text_wrap: TextWrap,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub tab_stop: usize,
    pub ellipsis: String,
    pub width_policy: UnicodeWidthPolicy,
}

impl TextFlowOptions {
    pub fn new(max_width: usize, text_wrap: TextWrap) -> Self {
        Self {
            max_width,
            text_wrap,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            tab_stop: 4,
            ellipsis: "…".to_string(),
            width_policy: UnicodeWidthPolicy { revision: 1 },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextFlowCacheIdentity {
    pub input: TextFlowInput,
    pub options: TextFlowOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFlowDiagnostic {
    StyleBoundaryNormalized {
        boundary: usize,
        grapheme_range: Range<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenClass {
    Content,
    Whitespace,
    Tab,
    HardBreak,
    SanitizedControl,
}

type Tokenization = (
    Vec<TextFlowToken>,
    Vec<TextFlowDiagnostic>,
    Vec<Range<usize>>,
);

#[derive(Debug, Clone, PartialEq)]
pub struct TextFlowToken {
    pub source: TextFlowSource,
    pub safe_text: String,
    pub style: Style,
    pub display_width: usize,
    pub placement: TextFlowPlacement,
    class: TokenClass,
}

impl TextFlowToken {
    pub fn source_range(&self) -> Option<Range<usize>> {
        match &self.source {
            TextFlowSource::Source { range, .. } => Some(range.clone()),
            TextFlowSource::Synthetic => None,
        }
    }

    pub fn safe_text(&self) -> &str {
        &self.safe_text
    }

    pub fn placement(&self) -> &TextFlowPlacement {
        &self.placement
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextFlowRun {
    pub token_index: usize,
    pub row: usize,
    pub column: usize,
    pub width: usize,
    pub text: String,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextFlowRow {
    pub index: usize,
    pub width: usize,
    pub text: String,
    pub runs: Vec<TextFlowRun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFlowPositionMapEntry {
    pub token_index: usize,
    pub row: usize,
    pub column: usize,
    pub width: usize,
    pub source: TextFlowSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextFlow {
    rows: Vec<String>,
    logical_rows: Vec<TextFlowRow>,
    tokens: Vec<TextFlowToken>,
    position_map: Vec<TextFlowPositionMapEntry>,
    diagnostics: Vec<TextFlowDiagnostic>,
    cache_identity: TextFlowCacheIdentity,
}

impl TextFlow {
    pub fn try_build(
        input: &TextFlowInput,
        options: &TextFlowOptions,
    ) -> Result<Self, TextFlowError> {
        Self::try_build_interruptible(input, options, || false)
    }

    pub fn try_build_interruptible(
        input: &TextFlowInput,
        options: &TextFlowOptions,
        mut interrupted: impl FnMut() -> bool,
    ) -> Result<Self, TextFlowError> {
        if options.tab_stop == 0 {
            return Err(TextFlowError::InvalidTabStop);
        }
        validate_styled_ranges(input)?;
        let (mut tokens, diagnostics, grapheme_ranges) = tokenize_source(input, &mut interrupted)?;
        let logical_rows = layout_tokens(&mut tokens, options, &mut interrupted)?;
        validate_source_coverage(&input.source, &tokens, &grapheme_ranges)?;
        let position_map = build_position_map(&tokens);
        let rows = logical_rows.iter().map(|row| row.text.clone()).collect();

        Ok(Self {
            rows,
            logical_rows,
            tokens,
            position_map,
            diagnostics,
            cache_identity: TextFlowCacheIdentity {
                input: input.clone(),
                options: options.clone(),
            },
        })
    }

    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    pub fn tokens(&self) -> &[TextFlowToken] {
        &self.tokens
    }

    pub fn logical_rows(&self) -> &[TextFlowRow] {
        &self.logical_rows
    }

    pub fn position_map(&self) -> &[TextFlowPositionMapEntry] {
        &self.position_map
    }

    pub fn diagnostics(&self) -> &[TextFlowDiagnostic] {
        &self.diagnostics
    }

    pub fn cache_identity(&self) -> &TextFlowCacheIdentity {
        &self.cache_identity
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn max_row_width(&self) -> usize {
        self.logical_rows
            .iter()
            .map(|row| row.width)
            .max()
            .unwrap_or(0)
    }
}

#[derive(Debug, Default)]
pub struct TextFlowCache {
    published: Option<Arc<TextFlow>>,
    build_count: usize,
}

impl TextFlowCache {
    pub fn get_or_compute(
        &mut self,
        input: &TextFlowInput,
        options: &TextFlowOptions,
    ) -> Result<Arc<TextFlow>, TextFlowError> {
        if let Some(flow) = &self.published
            && flow.cache_identity.input == *input
            && flow.cache_identity.options == *options
        {
            return Ok(Arc::clone(flow));
        }
        let completed = Arc::new(TextFlow::try_build(input, options)?);
        self.build_count = self
            .build_count
            .checked_add(1)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        self.published = Some(Arc::clone(&completed));
        Ok(completed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFlowError {
    InvalidTabStop,
    InvalidStyleRange {
        range: Range<usize>,
    },
    OverlappingStyleRanges {
        first: Range<usize>,
        second: Range<usize>,
    },
    FinalizedRangeNotGraphemeBoundary {
        range: Range<usize>,
    },
    IncompleteSourceCoverage {
        expected: usize,
        covered: usize,
    },
    ArithmeticOverflow,
    Interrupted,
}

impl fmt::Display for TextFlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTabStop => write!(f, "text flow tab stop must be greater than zero"),
            Self::InvalidStyleRange { range } => write!(f, "invalid styled range {range:?}"),
            Self::OverlappingStyleRanges { first, second } => {
                write!(f, "styled ranges overlap: {first:?} and {second:?}")
            }
            Self::FinalizedRangeNotGraphemeBoundary { range } => {
                write!(f, "finalized range is not one grapheme: {range:?}")
            }
            Self::IncompleteSourceCoverage { expected, covered } => {
                write!(f, "source map covers {covered} bytes, expected {expected}")
            }
            Self::ArithmeticOverflow => write!(f, "text flow dimensions overflowed"),
            Self::Interrupted => write!(f, "text flow construction was interrupted"),
        }
    }
}

impl Error for TextFlowError {}

fn validate_styled_ranges(input: &TextFlowInput) -> Result<(), TextFlowError> {
    for styled in &input.styled_ranges {
        let range = &styled.range;
        if range.start > range.end
            || range.end > input.source.len()
            || !input.source.is_char_boundary(range.start)
            || !input.source.is_char_boundary(range.end)
        {
            return Err(TextFlowError::InvalidStyleRange {
                range: (*range).clone(),
            });
        }
    }
    let mut ranges: Vec<_> = input
        .styled_ranges
        .iter()
        .filter(|styled| !styled.range.is_empty())
        .map(|styled| &styled.range)
        .collect();
    ranges.sort_by_key(|range| range.start);
    for pair in ranges.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(TextFlowError::OverlappingStyleRanges {
                first: pair[0].clone(),
                second: pair[1].clone(),
            });
        }
    }
    Ok(())
}

fn tokenize_source(
    input: &TextFlowInput,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Tokenization, TextFlowError> {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut grapheme_ranges = Vec::new();
    for (start, grapheme) in input.source.grapheme_indices(true) {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let range = start..start + grapheme.len();
        grapheme_ranges.push(range.clone());
        let style = input
            .styled_ranges
            .iter()
            .find(|styled| styled.range.start <= start && start < styled.range.end)
            .map_or_else(
                || input.default_style.clone(),
                |styled| styled.style.clone(),
            );
        for boundary in input
            .styled_ranges
            .iter()
            .flat_map(|styled| [styled.range.start, styled.range.end])
            .filter(|boundary| range.start < *boundary && *boundary < range.end)
        {
            diagnostics.push(TextFlowDiagnostic::StyleBoundaryNormalized {
                boundary,
                grapheme_range: range.clone(),
            });
        }
        let (safe_text, class) = classify_grapheme(grapheme);
        let display_width = if class == TokenClass::Tab {
            0
        } else {
            grapheme_width(&safe_text)
        };
        tokens.push(TextFlowToken {
            source: TextFlowSource::Source {
                range,
                kind: input.source_kind,
            },
            safe_text,
            style,
            display_width,
            placement: TextFlowPlacement::Omitted,
            class,
        });
    }
    Ok((tokens, diagnostics, grapheme_ranges))
}

#[cfg(test)]
fn validate_finalized_range(source: &str, range: &Range<usize>) -> Result<(), TextFlowError> {
    let is_whole_grapheme = source
        .grapheme_indices(true)
        .any(|(start, grapheme)| range == &(start..start + grapheme.len()));
    if !is_whole_grapheme {
        return Err(TextFlowError::FinalizedRangeNotGraphemeBoundary {
            range: range.clone(),
        });
    }
    Ok(())
}

fn classify_grapheme(grapheme: &str) -> (String, TokenClass) {
    if matches!(grapheme, "\n" | "\r" | "\r\n") {
        return (String::new(), TokenClass::HardBreak);
    }
    if grapheme == "\t" {
        return (String::new(), TokenClass::Tab);
    }
    let mut safe = String::new();
    let mut sanitized = false;
    for scalar in grapheme.chars() {
        match scalar {
            '\u{0000}'..='\u{001f}' => {
                safe.push(char::from_u32(0x2400 + scalar as u32).unwrap_or('\u{fffd}'));
                sanitized = true;
            }
            '\u{007f}' => {
                safe.push('␡');
                sanitized = true;
            }
            '\u{0080}'..='\u{009f}' => {
                safe.push('\u{fffd}');
                sanitized = true;
            }
            _ => safe.push(scalar),
        }
    }
    let class = if sanitized {
        TokenClass::SanitizedControl
    } else if grapheme.chars().all(char::is_whitespace) {
        TokenClass::Whitespace
    } else {
        TokenClass::Content
    };
    (safe, class)
}

fn layout_tokens(
    tokens: &mut [TextFlowToken],
    options: &TextFlowOptions,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Vec<TextFlowRow>, TextFlowError> {
    let mut rows = Vec::new();
    let mut line_start = 0;
    for index in 0..tokens.len() {
        if tokens[index].class != TokenClass::HardBreak {
            continue;
        }
        layout_line(tokens, line_start..index, options, &mut rows, interrupted)?;
        tokens[index].placement = TextFlowPlacement::HardBreak {
            row: rows.len().saturating_sub(1),
        };
        line_start = index + 1;
    }
    if line_start < tokens.len() || tokens.is_empty() {
        layout_line(
            tokens,
            line_start..tokens.len(),
            options,
            &mut rows,
            interrupted,
        )?;
    }
    Ok(rows)
}

fn layout_line(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    options: &TextFlowOptions,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    if options.max_width == 0 {
        for token in &mut tokens[range] {
            finalize_tab(token, options.tab_stop);
            token.placement = TextFlowPlacement::Omitted;
        }
        return place_row(tokens, &[], options.tab_stop, rows);
    }
    match options.text_wrap {
        TextWrap::Wrap => wrap_line(tokens, range, options, rows, interrupted),
        _ => truncate_line(tokens, range, options, rows, interrupted),
    }
}

fn wrap_line(
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
        if !current.is_empty() {
            place_row(tokens, &current, options.tab_stop, rows)?;
            current.clear();
            current_width = 0;
        }
        for index in pending.drain(..) {
            tokens[index].placement = TextFlowPlacement::Omitted;
        }
        let fresh_width = sequence_width(tokens, &word, 0, options.tab_stop)?;
        if fresh_width <= options.max_width {
            current.extend(word);
            current_width = fresh_width;
            continue;
        }
        for index in word {
            let width = token_width_at(&mut tokens[index], current_width, options.tab_stop);
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
            let width = token_width_at(&mut tokens[index], current_width, options.tab_stop);
            current_width = current_width
                .checked_add(width)
                .ok_or(TextFlowError::ArithmeticOverflow)?;
        }
    }
    current.append(&mut pending);
    place_row(tokens, &current, options.tab_stop, rows)
}

fn truncate_line(
    tokens: &mut [TextFlowToken],
    range: Range<usize>,
    options: &TextFlowOptions,
    rows: &mut Vec<TextFlowRow>,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<(), TextFlowError> {
    let mut placed = Vec::new();
    let mut width = 0usize;
    let mut truncating = false;
    for index in range {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        let token_width = token_width_at(&mut tokens[index], width, options.tab_stop);
        if !truncating
            && width
                .checked_add(token_width)
                .ok_or(TextFlowError::ArithmeticOverflow)?
                <= options.max_width
        {
            placed.push(index);
            width += token_width;
        } else {
            truncating = true;
            tokens[index].placement = TextFlowPlacement::Truncated;
        }
    }
    place_row(tokens, &placed, options.tab_stop, rows)
}

fn sequence_width(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    start: usize,
    tab_stop: usize,
) -> Result<usize, TextFlowError> {
    let mut column = start;
    for index in indices {
        let width = token_width_at(&mut tokens[*index], column, tab_stop);
        column = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
    }
    Ok(column - start)
}

fn token_width_at(token: &mut TextFlowToken, column: usize, tab_stop: usize) -> usize {
    if token.class == TokenClass::Tab {
        let width = tab_stop - column % tab_stop;
        finalize_tab(token, width);
        width
    } else {
        token.display_width
    }
}

fn finalize_tab(token: &mut TextFlowToken, width: usize) {
    if token.class == TokenClass::Tab {
        token.display_width = width;
        token.safe_text = " ".repeat(width);
    }
}

fn place_row(
    tokens: &mut [TextFlowToken],
    indices: &[usize],
    tab_stop: usize,
    rows: &mut Vec<TextFlowRow>,
) -> Result<(), TextFlowError> {
    let row = rows.len();
    let mut column = 0;
    let mut text = String::new();
    let mut runs = Vec::with_capacity(indices.len());
    for index in indices {
        let width = token_width_at(&mut tokens[*index], column, tab_stop);
        tokens[*index].placement = match (&tokens[*index].source, tokens[*index].class, width) {
            (TextFlowSource::Synthetic, _, _) => TextFlowPlacement::Synthetic { row, column },
            (_, TokenClass::SanitizedControl, _) => {
                TextFlowPlacement::SanitizedControl { row, column }
            }
            (_, _, 0) => TextFlowPlacement::ZeroWidth { row, column },
            _ => TextFlowPlacement::Positioned { row, column },
        };
        text.push_str(&tokens[*index].safe_text);
        runs.push(TextFlowRun {
            token_index: *index,
            row,
            column,
            width,
            text: tokens[*index].safe_text.clone(),
            style: tokens[*index].style.clone(),
        });
        column = column
            .checked_add(width)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
    }
    rows.push(TextFlowRow {
        index: row,
        width: column,
        text,
        runs,
    });
    Ok(())
}

fn validate_source_coverage(
    source: &str,
    tokens: &[TextFlowToken],
    grapheme_ranges: &[Range<usize>],
) -> Result<(), TextFlowError> {
    let mut covered = 0;
    let mut source_index = 0;
    for token in tokens {
        let TextFlowSource::Source { range, .. } = &token.source else {
            continue;
        };
        if range.start != covered {
            return Err(TextFlowError::IncompleteSourceCoverage {
                expected: source.len(),
                covered,
            });
        }
        if grapheme_ranges.get(source_index) != Some(range) {
            return Err(TextFlowError::FinalizedRangeNotGraphemeBoundary {
                range: range.clone(),
            });
        }
        source_index += 1;
        covered = range.end;
    }
    if covered != source.len() || source_index != grapheme_ranges.len() {
        return Err(TextFlowError::IncompleteSourceCoverage {
            expected: source.len(),
            covered,
        });
    }
    Ok(())
}

fn build_position_map(tokens: &[TextFlowToken]) -> Vec<TextFlowPositionMapEntry> {
    tokens
        .iter()
        .enumerate()
        .filter_map(|(token_index, token)| {
            let (row, column) = match token.placement {
                TextFlowPlacement::Positioned { row, column }
                | TextFlowPlacement::ZeroWidth { row, column }
                | TextFlowPlacement::SanitizedControl { row, column }
                | TextFlowPlacement::Synthetic { row, column } => (row, column),
                _ => return None,
            };
            Some(TextFlowPositionMapEntry {
                token_index,
                row,
                column,
                width: token.display_width,
                source: token.source.clone(),
            })
        })
        .collect()
}

/// Compatibility wrapper retained for PR #84 callers.
pub(crate) fn flow_text(text: &str, max_width: usize, wrap: TextWrap) -> TextFlow {
    let input = TextFlowInput::plain(text, TextFlowSourceKind::Exact, Style::new());
    TextFlow::try_build(&input, &TextFlowOptions::new(max_width, wrap))
        .unwrap_or_else(|error| panic!("canonical text flow failed: {error}"))
}

#[cfg(test)]
mod tests;
