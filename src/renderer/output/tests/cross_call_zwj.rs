use super::*;

fn write_two_scalar_sequence(output: &mut Output, style: &Style) {
    output.write_char(0, 0, '👩', style);
    output.write_char(2, 0, '\u{200d}', style);
    output.write_char(2, 0, '💻', style);
}

#[test]
fn public_write_char_merges_two_scalar_zwj_sequence() {
    let mut output = Output::new(6, 1);
    let style = Style::default();

    write_two_scalar_sequence(&mut output, &style);
    output.write_char(2, 0, 'X', &style);

    assert_eq!(output.render(), "👩\u{200d}💻X");
    assert_eq!(
        output.owner_footprint(CellPosition { x: 0, y: 0 }),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );
    assert_eq!(
        output.owner_footprint(CellPosition { x: 2, y: 0 }),
        vec![CellPosition { x: 2, y: 0 }]
    );
}

#[test]
fn public_write_char_merges_a_multi_zwj_family_into_one_owner() {
    let mut output = Output::new(8, 1);
    let style = Style::default();

    output.write_char(0, 0, '👨', &style);
    for member in ['👩', '👧', '👦'] {
        output.write_char(2, 0, '\u{200d}', &style);
        output.write_char(2, 0, member, &style);
    }
    output.write_char(2, 0, 'X', &style);

    assert_eq!(output.render(), "👨\u{200d}👩\u{200d}👧\u{200d}👦X");
    assert_eq!(
        output.owner_footprint(CellPosition { x: 0, y: 0 }),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );
    assert_eq!(
        output.owner_footprint(CellPosition { x: 2, y: 0 }),
        vec![CellPosition { x: 2, y: 0 }]
    );
}

#[test]
fn clipped_completion_mutates_neither_pending_owner_nor_candidate_cells() {
    for clip in [
        ClipRegion {
            x1: 0,
            y1: 0,
            x2: 2,
            y2: 1,
        },
        ClipRegion {
            x1: 2,
            y1: 0,
            x2: 4,
            y2: 1,
        },
    ] {
        let mut output = Output::new(6, 1);
        let style = Style::default();
        output.write_char(0, 0, '👩', &style);
        output.write_char(2, 0, '\u{200d}', &style);
        output.clear_dirty();

        output.clip(clip);
        output.write_char(2, 0, '💻', &style);
        output.unclip();

        assert_eq!(output.render(), "👩\u{200d}");
        assert_eq!(output.dirty_cell_positions().count(), 0);
        assert_eq!(
            output.owner_footprint(CellPosition { x: 0, y: 0 }),
            vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
        );
        assert!(
            output
                .owner_footprint(CellPosition { x: 2, y: 0 })
                .is_empty()
        );

        output.write_char(2, 0, '💻', &style);
        output.write_char(2, 0, 'X', &style);
        assert_eq!(output.render(), "👩\u{200d}💻X");
    }
}

#[test]
fn completion_clears_overwritten_owners_and_marks_the_full_union_dirty() {
    let mut output = Output::new(6, 1);
    let style = Style::default();
    output.write(2, 0, "界", &style);
    output.write_char(0, 0, '👩', &style);
    output.write_char(2, 0, '\u{200d}', &style);
    output.clear_dirty();

    output.write_char(2, 0, '💻', &style);

    assert_eq!(output.render(), "👩\u{200d}💻");
    assert_eq!(
        output.dirty_cell_positions().collect::<Vec<_>>(),
        vec![
            CellPosition { x: 0, y: 0 },
            CellPosition { x: 1, y: 0 },
            CellPosition { x: 2, y: 0 },
            CellPosition { x: 3, y: 0 },
        ]
    );
    assert!(
        output
            .owner_footprint(CellPosition { x: 2, y: 0 })
            .is_empty()
    );
    assert_eq!(output.cell_at(3, 0).unwrap().ch, ' ');

    output.write_char(2, 0, 'X', &style);
    assert_eq!(output.render(), "👩\u{200d}💻X");
}

#[test]
fn breaks_wrong_positions_and_sanitized_scalars_invalidate_pending_completion() {
    let style = Style::default();

    let mut broken = Output::new(8, 1);
    broken.write_char(0, 0, '👩', &style);
    broken.write_char(2, 0, '\u{200d}', &style);
    broken.write_char(2, 0, '\n', &style);
    broken.write_char(2, 0, '💻', &style);
    broken.write_char(2, 0, 'X', &style);
    assert_eq!(broken.render(), "👩\u{200d}X");

    let mut misplaced = Output::new(8, 1);
    misplaced.write_char(0, 0, '👩', &style);
    misplaced.write_char(2, 0, '\u{200d}', &style);
    misplaced.write_char(4, 0, 'Q', &style);
    misplaced.write_char(2, 0, '💻', &style);
    assert_eq!(
        misplaced.owner_footprint(CellPosition { x: 2, y: 0 }),
        vec![CellPosition { x: 2, y: 0 }, CellPosition { x: 3, y: 0 }]
    );
    assert_eq!(
        misplaced.owner_footprint(CellPosition { x: 4, y: 0 }),
        vec![CellPosition { x: 4, y: 0 }]
    );

    let mut sanitized = Output::new(8, 1);
    sanitized.write_char(0, 0, '👩', &style);
    sanitized.write_char(2, 0, '\u{200d}', &style);
    sanitized.write_char(2, 0, '\u{001b}', &style);
    assert_eq!(sanitized.render(), "👩\u{200d}␛");
    assert!(!sanitized.render().contains('\u{001b}'));
}

#[test]
fn write_char_cross_call_and_whole_egc_write_have_identical_ownership() {
    let style = Style {
        color: Some(Color::Cyan),
        bold: true,
        ..Style::default()
    };
    let mut scalar = Output::new(6, 1);
    write_two_scalar_sequence(&mut scalar, &style);
    scalar.write_char(2, 0, 'X', &style);

    let mut split_write = Output::new(6, 1);
    split_write.write(0, 0, "👩", &style);
    split_write.write(2, 0, "\u{200d}", &style);
    split_write.write(2, 0, "💻X", &style);

    let mut whole = Output::new(6, 1);
    whole.write(0, 0, "👩\u{200d}💻X", &style);

    assert_eq!(scalar.render(), whole.render());
    assert_eq!(split_write.render(), whole.render());
    assert_eq!(scalar.grapheme_cells, whole.grapheme_cells);
    assert_eq!(split_write.grapheme_cells, whole.grapheme_cells);
    assert_eq!(
        scalar.dirty_cell_positions().collect::<Vec<_>>(),
        whole.dirty_cell_positions().collect::<Vec<_>>()
    );
    for col in 0..3 {
        let scalar_cell = scalar.cell_at(col, 0).unwrap();
        let whole_cell = whole.cell_at(col, 0).unwrap();
        assert_eq!(scalar_cell.ch, whole_cell.ch);
        assert!(scalar_cell.same_style(whole_cell));
    }
}

#[test]
fn completion_at_the_right_boundary_reuses_the_existing_owner_cells() {
    let mut output = Output::new(2, 1);
    let style = Style::default();

    write_two_scalar_sequence(&mut output, &style);

    assert_eq!(output.render(), "👩\u{200d}💻");
    assert_eq!(
        output.owner_footprint(CellPosition { x: 0, y: 0 }),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );
}

#[test]
fn completion_recomputes_a_changed_candidate_width_before_publication() {
    let mut output = Output::new(5, 1);
    let style = Style::default();

    output.write_char(0, 0, '❤', &style);
    output.write_char(1, 0, '\u{fe0f}', &style);
    output.write_char(1, 0, '\u{200d}', &style);
    output.write_char(1, 0, '🔥', &style);
    output.write_char(2, 0, 'X', &style);

    assert_eq!(output.render(), "❤️\u{200d}🔥X");
    assert_eq!(
        output.owner_footprint(CellPosition { x: 0, y: 0 }),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );
    assert_eq!(
        output.owner_footprint(CellPosition { x: 2, y: 0 }),
        vec![CellPosition { x: 2, y: 0 }]
    );
}

#[test]
fn staged_snapshots_carry_pending_state_without_mutating_the_source_frame() {
    let mut source = Output::new(6, 1);
    let style = Style::default();
    source.write_char(0, 0, '👩', &style);
    source.write_char(2, 0, '\u{200d}', &style);
    source.clear_dirty();

    let mut staged = source.staged_snapshot();
    staged.write_char(2, 0, '💻', &style);

    assert_eq!(source.render(), "👩\u{200d}");
    assert_eq!(source.dirty_cell_positions().count(), 0);
    assert_eq!(staged.render(), "👩\u{200d}💻");
    assert_eq!(
        staged.dirty_cell_positions().collect::<Vec<_>>(),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );

    source.commit_staged(staged);
    source.write_char(2, 0, 'X', &style);
    assert_eq!(source.render(), "👩\u{200d}💻X");
}
