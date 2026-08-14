//! Composition tests: regions, resize and key routing.
//!
//! Row heights, anchoring and bottom-follow belong to `MessageListState` and are
//! tested there. What is checked here is that this shell keeps the transcript's
//! viewport agreeing with the layout — the disagreement being what puts a
//! composer under the transcript, or scrolls the transcript to a row nobody can
//! see.

use super::*;
use crate::components::chat::MessageId;
use crate::components::chat::message_list::tests::support::{entry_with_rows, measure_from_table};
use crate::components::chat::message_list::{MessageListEntry, ViewportRows};
use crate::hooks::Key;

const WIDTH: u16 = 40;
const STATUS_ROWS: u16 = 1;
const CACHE: usize = 64;

/// A transcript of `count` messages, each three rows tall.
fn transcript(count: u64, viewport_rows: u64) -> MessageListState {
    let entries: Vec<MessageListEntry> = (1..=count).map(|id| entry_with_rows(id, 3)).collect();
    let table: Vec<(u64, u64)> = (1..=count).map(|id| (id, 3)).collect();
    MessageListState::try_new(
        &entries,
        WIDTH,
        ViewportRows::new(viewport_rows),
        CACHE,
        measure_from_table(&table),
    )
    .expect("every message measures")
}

fn shell(height: u16) -> FullscreenChatShell {
    FullscreenChatShell::try_new(
        transcript(10, u64::from(height)),
        ChatComposerState::new(),
        WIDTH,
        height,
        STATUS_ROWS,
    )
    .expect("tall enough")
}

#[test]
fn the_transcript_viewport_matches_its_region_rather_than_the_terminal() {
    let shell = shell(24);

    let transcript_rows = shell.layout().transcript().rows();
    assert!(
        transcript_rows < 24,
        "the fixed regions must cost something"
    );
    assert_eq!(
        shell.transcript().viewport_rows(),
        ViewportRows::new(u64::from(transcript_rows)),
        "a transcript sized for the whole terminal scrolls to rows nobody can see"
    );
}

#[test]
fn the_composer_and_status_stay_pinned_across_every_workable_height() {
    for height in 3..=60u16 {
        let Ok(shell) = FullscreenChatShell::try_new(
            transcript(10, u64::from(height)),
            ChatComposerState::new(),
            WIDTH,
            height,
            STATUS_ROWS,
        ) else {
            continue;
        };
        let layout = shell.layout();
        assert!(!layout.has_overlap(), "overlap at height {height}");
        assert_eq!(layout.status().bottom(), height, "status left the bottom");
        assert_eq!(
            layout.composer().bottom(),
            layout.status().top(),
            "a gap opened between composer and status at height {height}"
        );
    }
}

#[test]
fn a_terminal_too_short_for_the_fixed_regions_is_refused() {
    let error = FullscreenChatShell::try_new(
        transcript(10, 1),
        ChatComposerState::new(),
        WIDTH,
        2,
        STATUS_ROWS,
    )
    .expect_err("two rows cannot hold composer, status and transcript");

    assert!(matches!(
        error,
        FullscreenShellError::Layout(FullscreenLayoutError::TooShort { .. })
    ));
}

#[test]
fn a_resize_moves_the_regions_and_the_transcript_viewport_together() {
    let mut shell = shell(24);
    let before = shell.layout().transcript().rows();

    shell.try_resize(WIDTH, 40).expect("taller terminal");

    let after = shell.layout().transcript().rows();
    assert!(
        after > before,
        "a taller terminal must show more transcript"
    );
    assert_eq!(
        shell.transcript().viewport_rows(),
        ViewportRows::new(u64::from(after)),
        "the viewport must follow the region, or the two disagree"
    );
    assert_eq!(shell.layout().status().bottom(), 40);
}

#[test]
fn a_resize_the_terminal_cannot_hold_leaves_the_old_layout_in_force() {
    let mut shell = shell(24);
    let before = shell.layout();

    let error = shell
        .try_resize(WIDTH, 1)
        .expect_err("one row cannot hold the fixed regions");

    assert!(matches!(error, FullscreenShellError::Layout(_)));
    // A partially applied resize is the failure mode: the layout and the
    // transcript viewport would disagree, and the transcript scrolls off screen.
    assert_eq!(shell.layout(), before);
    assert_eq!(
        shell.transcript().viewport_rows(),
        ViewportRows::new(u64::from(before.transcript().rows()))
    );
}

#[test]
fn replacing_a_transcript_cannot_publish_a_mismatched_viewport() {
    let mut shell = shell(24);
    let layout = shell.layout();
    let mut candidate = shell.transcript().clone();
    candidate
        .try_set_viewport_rows(candidate.revision(), ViewportRows::new(1))
        .expect("candidate can differ before publication");

    shell
        .try_replace_transcript(candidate)
        .expect("shell normalizes the candidate");

    assert_eq!(shell.layout(), layout);
    assert_layout_and_viewport_agree(&shell, "transcript replacement");
}

/// The invariant every mutating path has to preserve: the layout's transcript
/// region and the transcript's own viewport are the same number of rows.
///
/// They are two copies of one fact. Whenever they disagree the transcript
/// scrolls to positions the renderer never paints, and nothing reports it —
/// which is why this is asserted after *every* operation below rather than
/// documented once.
fn assert_layout_and_viewport_agree(shell: &FullscreenChatShell, after: &str) {
    assert_eq!(
        shell.transcript().viewport_rows(),
        ViewportRows::new(u64::from(shell.layout().transcript().rows())),
        "layout and viewport disagreed after {after}"
    );
}

#[test]
fn layout_and_viewport_agree_after_every_operation() {
    let mut shell = shell(24);
    assert_layout_and_viewport_agree(&shell, "construction");

    let keymap = ChatComposerKeyMap::new();
    let newline = Key {
        return_key: true,
        shift: true,
        ..Key::default()
    };

    for height in [40u16, 12, 3, 80, 5, 24] {
        // A refused resize must leave both untouched rather than half-applied.
        let before = shell.layout();
        if shell.try_resize(WIDTH, height).is_err() {
            assert_eq!(shell.layout(), before, "a refused resize moved the layout");
        }
        assert_layout_and_viewport_agree(&shell, &format!("a resize to {height}"));

        // Growing the draft re-lays out through the same path.
        let before = shell.layout();
        if shell.handle_key(&keymap, "", &newline).is_err() {
            assert_eq!(shell.layout(), before, "a refused reflow moved the layout");
        }
        assert_layout_and_viewport_agree(&shell, &format!("a newline at height {height}"));

        // So does appending to the transcript, which must not touch the regions.
        let id = u64::from(height) + 100;
        let mut transcript = shell.transcript().clone();
        let revision = transcript.revision();
        transcript
            .try_append(
                revision,
                &[entry_with_rows(id, 3)],
                measure_from_table(&[(id, 3)]),
            )
            .expect("appends");
        shell
            .try_replace_transcript(transcript)
            .expect("replacement preserves the shell viewport");
        assert_layout_and_viewport_agree(&shell, &format!("an append at height {height}"));
    }
}

#[test]
fn consecutive_resizes_never_leave_the_regions_overlapping() {
    let mut shell = shell(24);

    for height in [40u16, 3, 80, 5, 24, 100, 3] {
        if shell.try_resize(WIDTH, height).is_err() {
            continue;
        }
        let layout = shell.layout();
        assert!(!layout.has_overlap(), "overlap after resizing to {height}");
        assert_eq!(layout.height(), height);
        assert_eq!(
            shell.transcript().viewport_rows(),
            ViewportRows::new(u64::from(layout.transcript().rows())),
            "viewport drifted from the region after resizing to {height}"
        );
    }
}

#[test]
fn a_growing_draft_takes_rows_from_the_transcript_and_the_status_stays_put() {
    let mut shell = shell(24);
    let keymap = ChatComposerKeyMap::new();
    let status_before = shell.layout().status();
    let transcript_before = shell.layout().transcript().rows();

    let newline = Key {
        return_key: true,
        shift: true,
        ..Key::default()
    };
    for _ in 0..2 {
        shell
            .handle_key(&keymap, "", &newline)
            .expect("the terminal is tall enough for the draft");
    }

    assert!(
        shell.layout().composer().rows() > 1,
        "a multi-line draft must claim more rows"
    );
    assert!(shell.layout().transcript().rows() < transcript_before);
    assert_eq!(shell.layout().status(), status_before);
    assert!(!shell.layout().has_overlap());
    assert_eq!(
        shell.transcript().viewport_rows(),
        ViewportRows::new(u64::from(shell.layout().transcript().rows()))
    );
}

#[test]
fn an_open_overlay_captures_keys_before_any_other_region() {
    let mut shell = shell(24);
    let keymap = ChatComposerKeyMap::new();
    shell.set_overlay_open(true);

    let outcome = shell
        .handle_key(&keymap, "x", &Key::default())
        .expect("the overlay path performs no layout work");

    assert_eq!(outcome, FullscreenKeyOutcome::Overlay);
    // A modal that can be typed through is not modal.
    assert_eq!(shell.composer().text(), "");
}

#[test]
fn an_open_overlay_holds_focus_rather_than_letting_it_move_silently() {
    let mut shell = shell(24);
    shell.set_overlay_open(true);

    let outcome = shell.set_focus(FullscreenFocus::Transcript);

    assert_eq!(
        outcome,
        FullscreenFocusOutcome::HeldByOverlay(FullscreenFocus::Composer)
    );
    assert_eq!(shell.focus(), FullscreenFocus::Composer);
}

#[test]
fn closing_an_overlay_returns_the_keyboard_to_whoever_had_it() {
    let mut shell = shell(24);
    let keymap = ChatComposerKeyMap::new();
    assert_eq!(
        shell.set_focus(FullscreenFocus::Transcript),
        FullscreenFocusOutcome::Moved(FullscreenFocus::Transcript)
    );
    shell.set_overlay_open(true);
    shell.set_overlay_open(false);

    let outcome = shell
        .handle_key(&keymap, "x", &Key::default())
        .expect("no layout work on the transcript path");

    assert_eq!(
        outcome,
        FullscreenKeyOutcome::Unconsumed(FullscreenFocus::Transcript),
        "the overlay must not have quietly moved focus to the composer"
    );
}

#[test]
fn keys_reach_the_composer_only_when_it_holds_focus() {
    let mut shell = shell(24);
    let keymap = ChatComposerKeyMap::new();

    shell.set_focus(FullscreenFocus::Transcript);
    let ignored = shell
        .handle_key(&keymap, "a", &Key::default())
        .expect("no layout work");
    assert_eq!(
        ignored,
        FullscreenKeyOutcome::Unconsumed(FullscreenFocus::Transcript)
    );
    assert_eq!(shell.composer().text(), "");

    shell.set_focus(FullscreenFocus::Composer);
    let typed = shell
        .handle_key(&keymap, "a", &Key::default())
        .expect("tall enough");
    assert_eq!(typed, FullscreenKeyOutcome::Changed("a".to_owned()));
}

#[test]
fn every_key_outcome_names_the_region_that_saw_it() {
    let mut shell = shell(24);
    let keymap = ChatComposerKeyMap::new();

    shell
        .handle_key(&keymap, "hello", &Key::default())
        .expect("tall enough");
    let enter = Key {
        return_key: true,
        ..Key::default()
    };
    let submitted = shell.handle_key(&keymap, "", &enter).expect("tall enough");
    assert_eq!(
        submitted,
        FullscreenKeyOutcome::Submitted("hello".to_owned())
    );

    let escape = Key {
        escape: true,
        ..Key::default()
    };
    let cancelled = shell.handle_key(&keymap, "", &escape).expect("tall enough");
    assert_eq!(cancelled, FullscreenKeyOutcome::Cancelled);
    // Cancelling the interaction must not discard a long draft.
    assert_eq!(shell.composer().text(), "hello");
}

#[test]
fn moving_focus_where_it_already_rests_reports_no_change() {
    let mut shell = shell(24);
    assert_eq!(
        shell.set_focus(FullscreenFocus::Composer),
        FullscreenFocusOutcome::Unchanged(FullscreenFocus::Composer)
    );
}

/// Heights standing in for a Markdown paragraph, a code block, a thinking
/// disclosure and a tool result — the message kinds that are not one row.
const MIXED_HEIGHTS: [(u64, u64); 4] = [(1, 1), (2, 12), (3, 4), (4, 7)];

fn mixed_transcript(viewport_rows: u64) -> MessageListState {
    let entries: Vec<MessageListEntry> = MIXED_HEIGHTS
        .iter()
        .map(|(id, rows)| entry_with_rows(*id, *rows))
        .collect();
    MessageListState::try_new(
        &entries,
        WIDTH,
        ViewportRows::new(viewport_rows),
        CACHE,
        measure_from_table(&MIXED_HEIGHTS),
    )
    .expect("every message measures")
}

#[test]
fn messages_of_different_kinds_occupy_their_own_row_counts() {
    let shell = FullscreenChatShell::try_new(
        mixed_transcript(20),
        ChatComposerState::new(),
        WIDTH,
        24,
        STATUS_ROWS,
    )
    .expect("tall enough");

    // Item count would report 4. Rows are what the transcript is laid out in.
    assert_eq!(shell.transcript().total_rows().expect("measured"), 24);
    for (id, rows) in MIXED_HEIGHTS {
        assert_eq!(
            shell
                .transcript()
                .message_rows(MessageId::new(id))
                .expect("known message")
                .get(),
            rows
        );
    }
}

#[test]
fn streaming_into_a_message_grows_it_without_disturbing_the_regions() {
    let mut shell = FullscreenChatShell::try_new(
        mixed_transcript(20),
        ChatComposerState::new(),
        WIDTH,
        24,
        STATUS_ROWS,
    )
    .expect("tall enough");
    let layout_before = shell.layout();

    let grown = entry_with_rows(4, 9);
    let table = [(4u64, 9u64)];
    let mut transcript = shell.transcript().clone();
    let revision = transcript.revision();
    transcript
        .try_update(revision, grown, measure_from_table(&table))
        .expect("the streamed message re-measures");
    shell
        .try_replace_transcript(transcript)
        .expect("replacement preserves the shell viewport");

    assert_eq!(
        shell
            .transcript()
            .message_rows(MessageId::new(4))
            .expect("known message")
            .get(),
        9
    );
    // The transcript growing is not a layout event: the regions are sized by
    // the terminal, not by how much conversation there is.
    assert_eq!(shell.layout(), layout_before);
    assert!(!shell.layout().has_overlap());
}

#[test]
fn prepending_history_keeps_the_transcript_viewport_agreeing_with_its_region() {
    let mut shell = FullscreenChatShell::try_new(
        mixed_transcript(20),
        ChatComposerState::new(),
        WIDTH,
        24,
        STATUS_ROWS,
    )
    .expect("tall enough");
    let region_rows = shell.layout().transcript().rows();

    let older = [entry_with_rows(90, 6), entry_with_rows(91, 2)];
    let table = [(90u64, 6u64), (91, 2)];
    let mut transcript = shell.transcript().clone();
    let revision = transcript.revision();
    transcript
        .try_prepend(revision, &older, measure_from_table(&table))
        .expect("older history measures");
    shell
        .try_replace_transcript(transcript)
        .expect("replacement preserves the shell viewport");

    assert_eq!(shell.transcript().total_rows().expect("measured"), 32);
    assert_eq!(
        shell.transcript().viewport_rows(),
        ViewportRows::new(u64::from(region_rows)),
        "loading history must not change how tall the transcript region is"
    );
    assert_eq!(shell.layout().transcript().rows(), region_rows);
}

#[test]
fn a_resize_after_mixed_content_keeps_every_region_consistent() {
    let mut shell = FullscreenChatShell::try_new(
        mixed_transcript(20),
        ChatComposerState::new(),
        WIDTH,
        24,
        STATUS_ROWS,
    )
    .expect("tall enough");

    for height in [10u16, 40, 4, 24] {
        shell
            .try_resize(WIDTH, height)
            .unwrap_or_else(|error| panic!("resizing to {height} failed: {error}"));
        let layout = shell.layout();
        assert!(!layout.has_overlap(), "overlap at {height}");
        assert_eq!(layout.height(), height);
        assert_eq!(
            shell.transcript().viewport_rows(),
            ViewportRows::new(u64::from(layout.transcript().rows()))
        );
    }
}

#[test]
fn a_draft_that_outgrows_the_terminal_is_reported_rather_than_clipped() {
    // Exactly enough for a one-row composer, a status bar and one transcript row.
    let mut shell = FullscreenChatShell::try_new(
        transcript(3, 1),
        ChatComposerState::new(),
        WIDTH,
        3,
        STATUS_ROWS,
    )
    .expect("exactly enough");
    let keymap = ChatComposerKeyMap::new();
    let before = shell.layout();

    let newline = Key {
        return_key: true,
        shift: true,
        ..Key::default()
    };
    // The first break still projects to one row — a trailing empty line is not
    // a row until something follows it — so the layout is unchanged and fine.
    shell
        .handle_key(&keymap, "", &newline)
        .expect("still one composer row");
    assert_eq!(shell.layout(), before);

    let refused = shell
        .handle_key(&keymap, "", &newline)
        .expect_err("a second composer row leaves no transcript row");

    assert!(matches!(refused, FullscreenShellError::Layout(_)));
    // Both sides of the invariant roll back together, so drawing the next frame
    // cannot expose a draft that no longer fits the retained layout.
    assert_eq!(shell.layout(), before);
    assert_eq!(shell.composer().text(), "\n");
    assert!(!shell.layout().has_overlap());
}

#[test]
fn acknowledging_a_multiline_submission_reflows_the_shell_atomically() {
    let mut shell = shell(24);
    let keymap = ChatComposerKeyMap::new();
    let newline = Key {
        return_key: true,
        shift: true,
        ..Key::default()
    };
    shell
        .handle_key(&keymap, "first", &Key::default())
        .expect("first line fits");
    shell
        .handle_key(&keymap, "", &newline)
        .expect("newline fits");
    shell
        .handle_key(&keymap, "second", &Key::default())
        .expect("second line fits");
    let composer_rows_before = shell.layout().composer().rows();

    let submit = Key {
        return_key: true,
        ..Key::default()
    };
    assert!(matches!(
        shell.handle_key(&keymap, "", &submit),
        Ok(FullscreenKeyOutcome::Submitted(_))
    ));
    let token = shell
        .composer()
        .pending_submission()
        .expect("submission staged")
        .token();
    shell
        .acknowledge_submission_success(token)
        .expect("clearing a draft can only free rows");

    assert_eq!(shell.composer().text(), "");
    assert!(shell.layout().composer().rows() < composer_rows_before);
    assert_layout_and_viewport_agree(&shell, "submission acknowledgement");
}
