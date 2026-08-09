//! How the draft occupies rows at a given width.
//!
//! The composer grows with its content, so its height is a function of *visual*
//! rows — what the renderer paints after wrapping — not of logical lines. One
//! pasted paragraph is a single logical line and many visual rows; sizing from
//! logical lines would show one row and hide the rest.
//!
//! Rows come from the shared [`TextFlow`], the same flow the renderer draws, so
//! the reserved height and the painted height cannot disagree.

use super::state::{ChatComposerState, ComposerRevision};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::core::{Style, TextWrap};
use crate::layout::{TextFlow, TextFlowInput, TextFlowOptions, TextFlowSourceKind};

/// The draft's visual shape at one width.
///
/// Carries the revision it was built from, so a caller can tell that the state
/// has moved on and the geometry no longer describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerProjection {
    revision: ComposerRevision,
    width: u16,
    rows: Vec<String>,
    visible_rows: usize,
    cursor_row: usize,
    cursor_column: usize,
}

impl ComposerProjection {
    /// Projects `state` at `width` columns.
    pub fn build(state: &ChatComposerState, width: u16) -> Self {
        let rows = flow_rows(&state.text(), width);
        // At least one row: an empty composer still needs somewhere to put the
        // cursor. At most the cap, so a long draft scrolls instead of pushing
        // the transcript off the screen.
        let visible_rows = rows.len().clamp(1, state.max_visible_lines().get());
        let (cursor_row, cursor_column) = cursor_position(state, width);

        Self {
            revision: state.revision(),
            width,
            rows,
            visible_rows,
            cursor_row,
            cursor_column,
        }
    }

    /// Visual row the cursor sits on, counted from the first row of the draft.
    pub const fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Cursor column, in terminal cells.
    ///
    /// Cells, not clusters: a CJK character occupies two columns, and placing
    /// the cursor by cluster index would put it in the wrong place after one.
    pub const fn cursor_column(&self) -> usize {
        self.cursor_column
    }

    /// First visual row to show so the cursor stays on screen.
    ///
    /// The draft can be taller than the cap, so the window follows the cursor
    /// rather than pinning to the top and scrolling it out of sight.
    pub fn scroll_offset(&self) -> usize {
        self.cursor_row
            .saturating_add(1)
            .saturating_sub(self.visible_rows)
    }

    /// The rows to paint, honouring the cap and the cursor.
    pub fn visible_slice(&self) -> &[String] {
        let start = self.scroll_offset().min(self.rows.len());
        let end = (start + self.visible_rows).min(self.rows.len());
        &self.rows[start..end]
    }

    /// The revision this projection was built from.
    pub const fn revision(&self) -> ComposerRevision {
        self.revision
    }

    /// The width it was built at.
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// Every visual row of the draft, wrapped.
    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    /// Total visual rows the draft occupies.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Rows the composer should be given, between one and its cap.
    pub const fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    /// Whether this projection still describes `state`.
    ///
    /// A geometry action — moving up or down a *visual* row — needs a current
    /// projection. Acting on a stale one would move the cursor to a row that no
    /// longer exists.
    pub fn is_current_for(&self, state: &ChatComposerState, width: u16) -> bool {
        self.revision == state.revision() && self.width == width
    }
}

/// Display width of one grapheme cluster, in cells.
fn grapheme_width(cluster: &str) -> usize {
    UnicodeWidthStr::width(cluster)
}

/// Where the cursor lands once the draft is wrapped.
///
/// Greedy wrapping places a word on the current row exactly when it fits, and
/// that decision depends only on the text before it. So flowing the text up to
/// the cursor wraps identically to the same prefix of the whole draft, and the
/// prefix's last row gives the cursor's row and column directly.
fn cursor_position(state: &ChatComposerState, width: u16) -> (usize, usize) {
    let text = state.text();
    let cursor = state.text_state().cursor();

    let mut prefix = String::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            prefix.push('\n');
        }
        if index < cursor.row {
            prefix.push_str(line);
            continue;
        }
        if index == cursor.row {
            let clusters: Vec<&str> = line.graphemes(true).collect();
            for cluster in clusters.iter().take(cursor.col) {
                prefix.push_str(cluster);
            }
        }
        break;
    }

    let rows = flow_rows(&prefix, width);
    let row = rows.len().saturating_sub(1);
    let column = rows
        .last()
        .map_or(0, |last| last.graphemes(true).map(grapheme_width).sum());

    (row, column)
}

/// Wrap `text` to `width`, using the flow the renderer will use.
fn flow_rows(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    let input = TextFlowInput::plain(text.to_owned(), TextFlowSourceKind::Exact, Style::default());
    let options = TextFlowOptions::new(width as usize, TextWrap::Wrap);

    match TextFlow::try_build(&input, &options) {
        Ok(flow) => flow.rows().to_vec(),
        // The flow could not be built. One row keeps the composer usable
        // rather than collapsing it to nothing.
        Err(_) => vec![String::new()],
    }
}
