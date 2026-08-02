//! GH-64: editing moves over grapheme clusters, not `char`s.
//!
//! Indexing by `char` let an edit land inside a cluster. Backspacing over
//! `e` + combining acute removed only the accent; over a ZWJ family emoji or a
//! regional-indicator flag it left a dangling joiner or a lone indicator —
//! text the user never typed and cannot repair by typing.

use rnk::components::textarea::TextAreaState;

/// Clusters that must never be split, each between two ASCII sentinels.
const CLUSTERS: &[(&str, &str)] = &[
    ("combining acute", "e\u{301}"),
    ("combining stack", "a\u{300}\u{301}\u{302}"),
    (
        "family emoji",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    ),
    ("flag", "\u{1F1EF}\u{1F1F5}"),
    ("skin tone", "\u{1F44D}\u{1F3FD}"),
    ("cjk", "世"),
    ("keycap", "1\u{FE0F}\u{20E3}"),
];

fn surrounded(cluster: &str) -> String {
    format!("a{cluster}b")
}

#[test]
fn backspace_removes_a_whole_cluster() {
    for (name, cluster) in CLUSTERS {
        let mut state = TextAreaState::with_content(&surrounded(cluster));
        state.move_to_end();
        state.move_left(); // now between the cluster and 'b'
        state.delete_before_cursor();

        assert_eq!(state.content(), "ab", "{name} was split by backspace");
    }
}

#[test]
fn delete_removes_a_whole_cluster() {
    for (name, cluster) in CLUSTERS {
        let mut state = TextAreaState::with_content(&surrounded(cluster));
        state.move_to_line_start();
        state.move_right(); // now between 'a' and the cluster
        state.delete_after_cursor();

        assert_eq!(state.content(), "ab", "{name} was split by delete");
    }
}

#[test]
fn horizontal_movement_steps_over_whole_clusters() {
    for (name, cluster) in CLUSTERS {
        let content = surrounded(cluster);
        let mut state = TextAreaState::with_content(&content);
        state.move_to_line_start();

        // a | cluster | b == three columns, so three steps reach the end.
        state.move_right();
        state.move_right();
        state.move_right();
        // Asserting the column alone proves nothing: a char-indexed cursor
        // would also report 3, just sitting inside the cluster. Deleting shows
        // where it actually is.
        state.delete_before_cursor();
        assert_eq!(
            state.content(),
            format!("a{cluster}"),
            "{name}: three steps right did not land at the end of the line"
        );

        let mut state = TextAreaState::with_content(&content);
        state.move_to_end();
        state.move_left();
        state.move_left();
        state.move_left();
        state.delete_after_cursor();
        assert_eq!(
            state.content(),
            format!("{cluster}b"),
            "{name}: three steps left did not land at the start of the line"
        );
    }
}

#[test]
fn a_combining_mark_joins_the_cluster_before_it() {
    // Typing an accent after a base letter grows the existing cluster. The
    // cursor must not advance past a cluster that grew rather than moved, or
    // the next keystroke lands in the wrong place.
    let mut state = TextAreaState::with_content("e");
    state.move_to_end();
    assert_eq!(state.cursor_col(), 1);

    state.insert_char('\u{301}');

    assert_eq!(state.content(), "e\u{301}");
    assert_eq!(
        state.cursor_col(),
        1,
        "the cursor advanced past a cluster that only grew"
    );
}

#[test]
fn typing_after_a_combining_mark_lands_after_the_cluster() {
    let mut state = TextAreaState::with_content("e");
    state.move_to_end();
    state.insert_char('\u{301}');
    state.insert_char('x');

    assert_eq!(state.content(), "e\u{301}x");
}

#[test]
fn pasted_multi_cluster_text_survives_intact() {
    let pasted = "héllo 👨‍👩‍👧 世界";
    let mut state = TextAreaState::new();
    state.insert_string(pasted);

    assert_eq!(state.content(), pasted);
}

#[test]
fn word_deletion_does_not_split_clusters() {
    let mut state = TextAreaState::with_content("one 👨‍👩‍👧 three");
    state.move_to_line_start();
    state.delete_word_after();

    assert!(
        state.content().starts_with("👨\u{200D}👩\u{200D}👧"),
        "word deletion damaged the following cluster: {:?}",
        state.content()
    );
}

#[test]
fn word_movement_treats_a_cluster_as_part_of_its_word() {
    // Decomposed, so the accents are separate `char`s but not separate
    // clusters — a char-indexed scan would stop in a different place.
    let mut state = TextAreaState::with_content("he\u{301}llo wo\u{308}rld");
    state.move_to_line_start();
    state.move_word_right();

    // "héllo" is five clusters, then the space.
    assert_eq!(state.cursor_col(), 6);
    state.delete_after_cursor();
    assert_eq!(state.content(), "he\u{301}llo o\u{308}rld");
}

#[test]
fn deleting_backwards_across_a_line_break_lands_on_a_cluster_boundary() {
    let mut state = TextAreaState::with_content("é\nx");
    state.move_to_end();
    state.move_to_line_start();
    state.delete_before_cursor();

    assert_eq!(state.content(), "éx");
    assert_eq!(
        state.cursor_col(),
        1,
        "the merge point must be a cluster boundary, not a char offset"
    );
}
