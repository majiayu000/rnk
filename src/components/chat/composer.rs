//! A multi-line chat input with grapheme-safe editing and explicit submission.
//!
//! Two behaviours are deliberate and easy to get wrong:
//!
//! **Submitting does not clear the draft.** [`handle_key`] stages a submission
//! and returns the text; the draft is cleared only when the caller calls
//! [`ChatComposerState::acknowledge_success`]. Clearing on submit destroys the
//! user's text at exactly the moment a failed send makes it hardest to
//! reproduce.
//!
//! **Escape cancels the interaction, not the draft.** It reports
//! [`InteractionOutcome::Cancelled`] and leaves the text alone, so a stray key
//! cannot discard a long message.
//!
//! ```rust
//! use rnk::components::chat::{ChatComposerKeyMap, ChatComposerState, handle_key};
//! use rnk::components::InteractionOutcome;
//! use rnk::hooks::Key;
//!
//! let mut state = ChatComposerState::new();
//! let keymap = ChatComposerKeyMap::new();
//!
//! handle_key(&mut state, &keymap, "hello", &Key::default());
//!
//! let mut enter = Key::default();
//! enter.return_key = true;
//! let outcome = handle_key(&mut state, &keymap, "", &enter);
//!
//! // The text came back, and it is still in the draft.
//! assert!(matches!(outcome, InteractionOutcome::Submitted(ref text) if text == "hello"));
//! assert_eq!(state.text(), "hello");
//!
//! // Only an acknowledged send clears it.
//! let token = state.pending_submission().expect("staged").token();
//! state.acknowledge_success(token)?;
//! assert_eq!(state.text(), "");
//! # Ok::<(), rnk::components::chat::ComposerError>(())
//! ```

mod keymap;
mod projection;
mod state;

pub use keymap::{ChatComposerKeyMap, ComposerAction};
pub use projection::ComposerProjection;
pub use state::{
    ChatComposerState, ComposerError, ComposerRevision, PendingSubmission, SubmissionToken,
};

use crate::components::interaction::{InteractionMode, InteractionOutcome};
use crate::hooks::Key;

/// Apply one keystroke to the composer.
///
/// `input` carries committed text — one grapheme from a keypress, or a whole
/// bracketed paste. It is inserted as a unit, so a paste is one edit rather
/// than a sequence of them.
pub fn handle_key(
    state: &mut ChatComposerState,
    keymap: &ChatComposerKeyMap,
    input: &str,
    key: &Key,
) -> InteractionOutcome<String> {
    if matches!(state.mode(), InteractionMode::Disabled) {
        return InteractionOutcome::Ignored;
    }

    match keymap.action(key) {
        Some(ComposerAction::Cancel) => return InteractionOutcome::Cancelled,
        Some(ComposerAction::Submit) => return submit(state),
        Some(action) => return apply(state, action),
        None => {}
    }

    if input.is_empty() {
        return InteractionOutcome::Ignored;
    }

    // Committed text. Control scalars are dropped here rather than stored:
    // they are not content, and a draft carrying them would send them on.
    let text: String = input
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\r' || *ch == '\t')
        .collect();
    if text.is_empty() {
        return InteractionOutcome::Ignored;
    }

    // CRLF and lone CR both mean one line break. Normalising on the way in
    // keeps the draft's own text consistent regardless of what the terminal
    // sent.
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

    match state.edit(|text_state| {
        text_state.insert_string(&normalized);
        true
    }) {
        Ok(true) => InteractionOutcome::Changed(state.text()),
        Ok(false) => InteractionOutcome::Ignored,
        Err(_) => InteractionOutcome::Ignored,
    }
}

fn submit(state: &mut ChatComposerState) -> InteractionOutcome<String> {
    match state.stage_submission() {
        // Blank or whitespace-only: nothing to send, and the draft is left
        // exactly as it is rather than cleared.
        Ok(None) => InteractionOutcome::Ignored,
        Ok(Some(_)) => InteractionOutcome::Submitted(state.text()),
        Err(_) => InteractionOutcome::Ignored,
    }
}

fn apply(state: &mut ChatComposerState, action: ComposerAction) -> InteractionOutcome<String> {
    let changed = state.edit(|text| match action {
        ComposerAction::InsertNewline => {
            text.insert_char('\n');
            true
        }
        ComposerAction::DeleteBefore => {
            text.delete_before_cursor();
            true
        }
        ComposerAction::DeleteAfter => {
            text.delete_after_cursor();
            true
        }
        ComposerAction::DeleteWordBefore => {
            text.delete_word_before();
            true
        }
        ComposerAction::Clear => {
            text.clear();
            true
        }
        ComposerAction::MoveLeft => {
            text.move_left();
            false
        }
        ComposerAction::MoveRight => {
            text.move_right();
            false
        }
        ComposerAction::MoveUp => {
            text.move_up();
            false
        }
        ComposerAction::MoveDown => {
            text.move_down();
            false
        }
        ComposerAction::MoveLineStart => {
            text.move_to_line_start();
            false
        }
        ComposerAction::MoveLineEnd => {
            text.move_to_line_end();
            false
        }
        ComposerAction::MoveWordLeft => {
            text.move_word_left();
            false
        }
        ComposerAction::MoveWordRight => {
            text.move_word_right();
            false
        }
        ComposerAction::Submit | ComposerAction::Cancel => false,
    });

    match changed {
        // Movement is recognised even when the cursor was already at the edge;
        // reporting `Ignored` would let the key fall through to something else.
        Ok(true) => InteractionOutcome::Changed(state.text()),
        Ok(false) => InteractionOutcome::Handled,
        Err(_) => InteractionOutcome::Ignored,
    }
}
