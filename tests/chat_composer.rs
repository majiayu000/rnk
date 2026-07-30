//! GH-64: the chat composer's editing, submission and mode contracts.
//!
//! The cases worth writing down are the ones where losing the user's text is
//! the failure: submitting, cancelling, failing to send, and being disabled
//! mid-edit. In each, the draft must still be there afterwards unless the
//! caller explicitly said otherwise.

use std::num::NonZeroUsize;

use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, ComposerProjection, handle_key,
};
use rnk::components::{InteractionMode, InteractionOutcome};
use rnk::hooks::Key;

fn enter() -> Key {
    Key {
        return_key: true,
        ..Key::default()
    }
}

fn shift_enter() -> Key {
    Key {
        return_key: true,
        shift: true,
        ..Key::default()
    }
}

fn escape() -> Key {
    Key {
        escape: true,
        ..Key::default()
    }
}

fn backspace() -> Key {
    Key {
        backspace: true,
        ..Key::default()
    }
}

/// Type `text` as committed input.
fn type_text(state: &mut ChatComposerState, keymap: &ChatComposerKeyMap, text: &str) {
    handle_key(state, keymap, text, &Key::default());
}

fn composer_with(text: &str) -> (ChatComposerState, ChatComposerKeyMap) {
    let mut state = ChatComposerState::new();
    let keymap = ChatComposerKeyMap::new();
    type_text(&mut state, &keymap, text);
    (state, keymap)
}

#[test]
fn enter_submits_and_shift_enter_breaks_the_line() {
    let (mut state, keymap) = composer_with("first");

    let outcome = handle_key(&mut state, &keymap, "", &shift_enter());
    assert!(matches!(outcome, InteractionOutcome::Changed(_)));
    type_text(&mut state, &keymap, "second");
    assert_eq!(state.text(), "first\nsecond");

    let outcome = handle_key(&mut state, &keymap, "", &enter());
    assert!(
        matches!(outcome, InteractionOutcome::Submitted(ref text) if text == "first\nsecond"),
        "{outcome:?}"
    );
}

#[test]
fn a_configured_newline_binding_replaces_the_default() {
    use rnk::components::keymap::{KeyBinding, KeyType, Modifiers};

    let keymap = ChatComposerKeyMap::new()
        .newline(vec![KeyBinding::new(KeyType::Char('j'), Modifiers::CTRL)]);
    let mut state = ChatComposerState::new();
    type_text(&mut state, &keymap, "line");

    let ctrl_j = Key {
        character: Some('j'),
        ctrl: true,
        ..Key::default()
    };
    handle_key(&mut state, &keymap, "", &ctrl_j);
    assert_eq!(state.text(), "line\n");

    // Shift+Enter is no longer bound to anything, so it does nothing at all —
    // it must not fall through to the plain-Enter submit binding.
    let outcome = handle_key(&mut state, &keymap, "", &shift_enter());
    assert!(
        matches!(outcome, InteractionOutcome::Ignored),
        "{outcome:?}"
    );

    // Plain Enter still submits.
    let outcome = handle_key(&mut state, &keymap, "", &enter());
    assert!(
        matches!(outcome, InteractionOutcome::Submitted(_)),
        "{outcome:?}"
    );
}

#[test]
fn submitting_does_not_clear_the_draft() {
    let (mut state, keymap) = composer_with("keep me");

    let outcome = handle_key(&mut state, &keymap, "", &enter());
    assert!(matches!(outcome, InteractionOutcome::Submitted(_)));
    assert_eq!(
        state.text(),
        "keep me",
        "the draft was cleared before the caller confirmed the send"
    );
    assert!(state.is_submitting());
}

#[test]
fn only_an_acknowledged_success_clears_the_draft() {
    let (mut state, keymap) = composer_with("hello");
    handle_key(&mut state, &keymap, "", &enter());

    let token = state.pending_submission().expect("staged").token();
    state.acknowledge_success(token).expect("token is current");

    assert_eq!(state.text(), "");
    assert!(!state.is_submitting());
}

#[test]
fn a_failed_send_keeps_every_character() {
    let draft = "a long message that would be painful to retype";
    let (mut state, keymap) = composer_with(draft);
    handle_key(&mut state, &keymap, "", &enter());

    let token = state.pending_submission().expect("staged").token();
    state.acknowledge_failure(token).expect("token is current");

    assert_eq!(state.text(), draft);
    assert!(
        !state.is_submitting(),
        "the composer must accept edits again"
    );

    // And it can be sent again.
    let outcome = handle_key(&mut state, &keymap, "", &enter());
    assert!(matches!(outcome, InteractionOutcome::Submitted(ref text) if text == draft));
}

#[test]
fn a_stale_token_cannot_clear_newer_text() {
    let (mut state, keymap) = composer_with("first");
    handle_key(&mut state, &keymap, "", &enter());
    let stale = state.pending_submission().expect("staged").token();

    state
        .acknowledge_success(stale)
        .expect("current at this point");
    type_text(&mut state, &keymap, "second");
    handle_key(&mut state, &keymap, "", &enter());

    assert!(
        state.acknowledge_success(stale).is_err(),
        "an acknowledgement from an earlier send cleared a later draft"
    );
    assert_eq!(state.text(), "second");
}

#[test]
fn blank_input_never_submits() {
    for draft in ["", "   ", "\n", " \n\t "] {
        let (mut state, keymap) = composer_with(draft);
        // Compare against what the composer actually holds: soft tabs mean the
        // stored draft is not byte-identical to what was typed.
        let before = state.text();

        let outcome = handle_key(&mut state, &keymap, "", &enter());

        assert!(
            matches!(outcome, InteractionOutcome::Ignored),
            "{draft:?} produced {outcome:?}"
        );
        assert!(!state.is_submitting());
        assert_eq!(state.text(), before, "a refused submit disturbed the draft");
    }
}

#[test]
fn escape_cancels_without_discarding_the_draft() {
    let (mut state, keymap) = composer_with("half-written thought");

    let outcome = handle_key(&mut state, &keymap, "", &escape());

    assert!(
        matches!(outcome, InteractionOutcome::Cancelled),
        "{outcome:?}"
    );
    assert_eq!(
        state.text(),
        "half-written thought",
        "Escape discarded the draft"
    );
}

#[test]
fn submitting_blocks_a_second_submit_and_further_edits() {
    let (mut state, keymap) = composer_with("in flight");
    handle_key(&mut state, &keymap, "", &enter());

    let again = handle_key(&mut state, &keymap, "", &enter());
    assert!(
        matches!(again, InteractionOutcome::Ignored),
        "a second Enter would send the same text twice: {again:?}"
    );

    type_text(&mut state, &keymap, " extra");
    handle_key(&mut state, &keymap, "", &backspace());
    assert_eq!(
        state.text(),
        "in flight",
        "the draft changed under a send that is already in flight"
    );
}

#[test]
fn disabled_ignores_everything_and_read_only_refuses_edits() {
    let (mut state, keymap) = composer_with("existing");

    state.set_mode(InteractionMode::Disabled);
    for key in [Key::default(), enter(), escape(), backspace()] {
        let outcome = handle_key(&mut state, &keymap, "x", &key);
        assert!(
            matches!(outcome, InteractionOutcome::Ignored),
            "{outcome:?}"
        );
    }
    assert_eq!(state.text(), "existing");

    state.set_mode(InteractionMode::ReadOnly);
    type_text(&mut state, &keymap, " more");
    handle_key(&mut state, &keymap, "", &backspace());
    assert_eq!(state.text(), "existing", "read-only accepted an edit");

    let outcome = handle_key(&mut state, &keymap, "", &enter());
    assert!(
        matches!(outcome, InteractionOutcome::Ignored),
        "read-only submitted a value: {outcome:?}"
    );
    // Escape still reports cancellation, which read-only permits.
    assert!(matches!(
        handle_key(&mut state, &keymap, "", &escape()),
        InteractionOutcome::Cancelled
    ));
}

#[test]
fn a_paste_is_inserted_whole_with_normalised_line_endings() {
    let (mut state, keymap) = composer_with("");
    type_text(&mut state, &keymap, "one\r\ntwo\rthree\nfour");

    assert_eq!(
        state.text(),
        "one\ntwo\nthree\nfour",
        "CRLF and lone CR must both become one break"
    );
}

#[test]
fn a_multi_cluster_paste_keeps_its_order_and_content() {
    let pasted = "héllo 👨‍👩‍👧 世界 🇯🇵";
    let (state, _keymap) = composer_with(pasted);

    assert_eq!(state.text(), pasted);
}

#[test]
fn a_multi_scalar_keystroke_is_one_edit() {
    // An IME commit or a ZWJ emoji arrives as several scalars at once.
    let (mut state, keymap) = composer_with("");
    let before = state.revision();

    type_text(
        &mut state,
        &keymap,
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    );

    assert_eq!(
        state.revision().get(),
        before.get() + 1,
        "one committed input advanced the generation more than once"
    );
}

#[test]
fn backspace_over_a_cluster_removes_all_of_it() {
    let (mut state, keymap) = composer_with("a👨‍👩‍👧");
    handle_key(&mut state, &keymap, "", &backspace());

    assert_eq!(state.text(), "a", "the cluster was split");
}

#[test]
fn control_scalars_never_enter_the_draft() {
    // A draft carrying an ESC would send it onward to whatever reads the text.
    let (state, _keymap) = composer_with("before\u{1b}[2Jafter\u{7}");

    assert_eq!(state.text(), "before[2Jafter");
}

#[test]
fn the_generation_advances_once_per_change_and_not_at_all_otherwise() {
    let (mut state, keymap) = composer_with("text");

    let unchanged = state.revision();
    handle_key(&mut state, &keymap, "", &escape());
    assert_eq!(state.revision(), unchanged, "Cancel changed the generation");

    handle_key(&mut state, &keymap, "", &Key::default());
    assert_eq!(
        state.revision(),
        unchanged,
        "an empty keystroke changed the generation"
    );

    type_text(&mut state, &keymap, "!");
    assert_eq!(state.revision().get(), unchanged.get() + 1);
}

#[test]
fn height_follows_wrapped_visual_rows_not_logical_lines() {
    // One logical line that wraps to several rows. Sizing from logical lines
    // would show one row and hide the rest.
    let (state, _) = composer_with("aaaa bbbb cccc dddd eeee ffff");

    let narrow = ComposerProjection::build(&state, 10);
    let wide = ComposerProjection::build(&state, 60);

    assert!(
        narrow.row_count() > 1,
        "a wrapped line reported a single row: {:?}",
        narrow.rows()
    );
    assert_eq!(wide.row_count(), 1);
    assert_eq!(narrow.visible_rows(), narrow.row_count());
}

#[test]
fn height_stays_between_one_and_the_cap() {
    let mut state = ChatComposerState::new().with_max_visible_lines(NonZeroUsize::new(3).unwrap());
    let keymap = ChatComposerKeyMap::new();

    let empty = ComposerProjection::build(&state, 40);
    assert_eq!(empty.visible_rows(), 1, "an empty composer needs one row");

    type_text(&mut state, &keymap, "one\ntwo\nthree\nfour\nfive");
    let full = ComposerProjection::build(&state, 40);

    assert_eq!(full.row_count(), 5);
    assert_eq!(full.visible_rows(), 3, "the cap was exceeded");
}

#[test]
fn a_projection_goes_stale_when_the_state_or_width_changes() {
    let (mut state, keymap) = composer_with("text");
    let projection = ComposerProjection::build(&state, 40);

    assert!(projection.is_current_for(&state, 40));
    assert!(
        !projection.is_current_for(&state, 20),
        "a projection from another width was accepted"
    );

    type_text(&mut state, &keymap, "!");
    assert!(
        !projection.is_current_for(&state, 40),
        "a projection from before an edit was accepted"
    );
}

#[test]
fn resize_reflows_the_same_content() {
    let (state, _) = composer_with("alpha beta gamma delta epsilon");

    for width in [8u16, 16, 32, 64] {
        let projection = ComposerProjection::build(&state, width);
        let joined: String = projection.rows().concat().split_whitespace().collect();
        let expected: String = state.text().split_whitespace().collect();

        assert_eq!(joined, expected, "width {width} lost content");
    }
}

#[test]
fn the_cursor_tracks_its_wrapped_position() {
    let (mut state, keymap) = composer_with("aaaa bbbb cccc");
    let projection = ComposerProjection::build(&state, 10);

    // "aaaa bbbb" / "cccc" — the cursor is at the end, on the second row.
    assert_eq!(projection.cursor_row(), 1);
    assert_eq!(projection.cursor_column(), 4);

    // At the start of the draft it is at the origin.
    handle_key(
        &mut state,
        &keymap,
        "",
        &Key {
            home: true,
            ..Key::default()
        },
    );
    let projection = ComposerProjection::build(&state, 10);
    assert_eq!(
        (projection.cursor_row(), projection.cursor_column()),
        (0, 0)
    );
}

#[test]
fn the_cursor_column_counts_cells_not_clusters() {
    // Two CJK characters are two clusters but four columns; placing the cursor
    // by cluster index would leave it two cells short.
    let (state, _) = composer_with("世界");
    let projection = ComposerProjection::build(&state, 20);

    assert_eq!(projection.cursor_column(), 4);
}

#[test]
fn the_visible_window_follows_the_cursor() {
    let mut state = ChatComposerState::new().with_max_visible_lines(NonZeroUsize::new(2).unwrap());
    let keymap = ChatComposerKeyMap::new();
    type_text(&mut state, &keymap, "one\ntwo\nthree\nfour");

    let projection = ComposerProjection::build(&state, 40);

    assert_eq!(projection.row_count(), 4);
    assert_eq!(projection.visible_rows(), 2);
    assert_eq!(
        projection.visible_slice(),
        ["three", "four"],
        "the window must show where the cursor is, not the top of the draft"
    );
}
