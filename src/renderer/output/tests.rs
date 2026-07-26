use super::*;

fn assert_no_source_controls(text: &str) {
    assert!(
        !text
            .chars()
            .any(|ch| { matches!(ch, '\0'..='\u{001f}' | '\u{007f}' | '\u{0080}'..='\u{009f}') }),
        "rendered source text contains a terminal control: {text:?}"
    );
}

#[test]
fn test_output_creation() {
    let output = Output::new(80, 24);
    assert_eq!(output.width, 80);
    assert_eq!(output.height, 24);
}

#[test]
fn test_write_text() {
    let mut output = Output::new(80, 24);
    output.write(0, 0, "Hello", &Style::default());

    assert_eq!(output.cell_at(0, 0).unwrap().ch, 'H');
    assert_eq!(output.cell_at(4, 0).unwrap().ch, 'o');
}

#[test]
fn test_styled_output() {
    let mut output = Output::new(80, 24);
    let style = Style {
        color: Some(Color::Green),
        bold: true,
        ..Style::default()
    };

    output.write(0, 0, "Test", &style);

    let rendered = output.render();
    assert!(rendered.contains("\x1b["));
}

#[test]
fn test_wide_char_placeholder() {
    let mut output = Output::new(80, 24);
    output.write(0, 0, "你好", &Style::default());

    // '你' at position 0, placeholder at position 1
    assert_eq!(output.cell_at(0, 0).unwrap().ch, '你');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, '\0');
    // '好' at position 2, placeholder at position 3
    assert_eq!(output.cell_at(2, 0).unwrap().ch, '好');
    assert_eq!(output.cell_at(3, 0).unwrap().ch, '\0');
}

#[test]
fn test_overwrite_wide_char_placeholder() {
    let mut output = Output::new(80, 24);
    // Write a wide char first
    output.write(0, 0, "你", &Style::default());
    assert_eq!(output.cell_at(0, 0).unwrap().ch, '你');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, '\0');

    // Overwrite the placeholder with a narrow char
    output.write_char(1, 0, 'X', &Style::default());

    // The wide char should be replaced with space (broken)
    assert_eq!(output.cell_at(0, 0).unwrap().ch, ' ');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, 'X');
}

#[test]
fn test_write_overwrite_wide_char_placeholder() {
    let mut output = Output::new(80, 24);
    output.write(0, 0, "你", &Style::default());

    // Overwrite placeholder through `write()` path to keep behavior aligned with `write_char()`.
    output.write(1, 0, "XY", &Style::default());

    assert_eq!(output.cell_at(0, 0).unwrap().ch, ' ');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, 'X');
    assert_eq!(output.cell_at(2, 0).unwrap().ch, 'Y');
}

#[test]
fn test_overwrite_wide_char_first_half() {
    let mut output = Output::new(80, 24);
    // Write a wide char first
    output.write(0, 0, "你", &Style::default());
    assert_eq!(output.cell_at(0, 0).unwrap().ch, '你');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, '\0');

    // Overwrite the first half with a narrow char
    output.write_char(0, 0, 'X', &Style::default());

    // The wide char's placeholder should be cleared
    assert_eq!(output.cell_at(0, 0).unwrap().ch, 'X');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, ' ');
}

#[test]
fn test_wide_char_render_no_duplicate() {
    let mut output = Output::new(80, 24);
    output.write(0, 0, "你好世界", &Style::default());

    let rendered = output.render();
    // Should contain exactly these 4 chars, no placeholders visible
    assert_eq!(rendered, "你好世界");
}

#[test]
fn test_raw_mode_line_endings() {
    // Raw mode requires CRLF line endings, not just LF
    let mut output = Output::new(40, 5);
    output.write(0, 0, "Line 1", &Style::default());
    output.write(0, 1, "Line 2", &Style::default());
    output.write(0, 2, "Line 3", &Style::default());

    let rendered = output.render();

    // Must use CRLF for raw mode compatibility
    assert!(
        rendered.contains("\r\n"),
        "Output must use CRLF line endings for raw mode"
    );

    // Count that we don't have standalone LF (without CR before it)
    let lines: Vec<&str> = rendered.split("\r\n").collect();
    assert!(lines.len() >= 3, "Should have at least 3 lines");

    // Verify no standalone LF within lines
    for line in &lines {
        assert!(
            !line.contains('\n'),
            "Should not have standalone LF within lines"
        );
    }
}

#[test]
fn test_line_alignment_in_output() {
    // Test that multi-line output will render with correct alignment
    let mut output = Output::new(20, 3);
    output.write(0, 0, "AAAA", &Style::default());
    output.write(0, 1, "BBBB", &Style::default());
    output.write(0, 2, "CCCC", &Style::default());

    let rendered = output.render();
    let lines: Vec<&str> = rendered.split("\r\n").collect();

    assert_eq!(lines[0], "AAAA");
    assert_eq!(lines[1], "BBBB");
    assert_eq!(lines[2], "CCCC");
}

#[test]
fn test_wide_char_at_boundary() {
    // Wide char at end of buffer should be replaced with space
    let mut output = Output::new(5, 1);
    output.write(3, 0, "你", &Style::default());

    // Position 3 should be a space, position 4 is at boundary
    assert_eq!(output.cell_at(3, 0).unwrap().ch, '你');
    assert_eq!(output.cell_at(4, 0).unwrap().ch, '\0');

    // Now test when wide char would extend past buffer
    let mut output2 = Output::new(5, 1);
    output2.write(4, 0, "你", &Style::default());

    // Should write a space instead since wide char won't fit
    assert_eq!(output2.cell_at(4, 0).unwrap().ch, ' ');
}

#[test]
fn test_wide_char_at_exact_boundary() {
    // Test when wide char is at the last valid position
    let mut output = Output::new(4, 1);
    output.write(2, 0, "你", &Style::default());

    // Wide char at position 2-3 should fit exactly
    assert_eq!(output.cell_at(2, 0).unwrap().ch, '你');
    assert_eq!(output.cell_at(3, 0).unwrap().ch, '\0');
}

#[test]
fn test_dirty_tracking_initial_state() {
    let output = Output::new(80, 24);
    assert!(!output.is_dirty());
    assert!(!output.is_row_dirty(0));
}

#[test]
fn test_dirty_tracking_after_write() {
    let mut output = Output::new(80, 24);
    output.write(0, 5, "Hello", &Style::default());

    assert!(output.is_dirty());
    assert!(output.is_row_dirty(5));
    assert!(!output.is_row_dirty(0));
    assert!(!output.is_row_dirty(6));
}

#[test]
fn test_dirty_tracking_after_write_char() {
    let mut output = Output::new(80, 24);
    output.write_char(10, 3, 'X', &Style::default());

    assert!(output.is_dirty());
    assert!(output.is_row_dirty(3));
    assert!(!output.is_row_dirty(2));
}

#[test]
fn test_dirty_tracking_clear() {
    let mut output = Output::new(80, 24);
    output.write(0, 0, "Test", &Style::default());
    output.write(0, 5, "Test", &Style::default());

    assert!(output.is_dirty());
    assert!(output.is_row_dirty(0));
    assert!(output.is_row_dirty(5));

    output.clear_dirty();

    assert!(!output.is_dirty());
    assert!(!output.is_row_dirty(0));
    assert!(!output.is_row_dirty(5));
}

#[test]
fn test_dirty_row_indices() {
    let mut output = Output::new(80, 24);
    output.write(0, 1, "A", &Style::default());
    output.write(0, 3, "B", &Style::default());
    output.write(0, 7, "C", &Style::default());

    let dirty: Vec<usize> = output.dirty_row_indices().collect();
    assert_eq!(dirty, vec![1, 3, 7]);
}

#[test]
fn test_render_dirty_rows() {
    let mut output = Output::new(80, 24);
    output.write(0, 0, "Line 0", &Style::default());
    output.write(0, 2, "Line 2", &Style::default());

    let dirty_rows = output.render_dirty_rows();
    assert_eq!(dirty_rows.len(), 2);
    assert_eq!(dirty_rows[0].0, 0);
    assert_eq!(dirty_rows[0].1, "Line 0");
    assert_eq!(dirty_rows[1].0, 2);
    assert_eq!(dirty_rows[1].1, "Line 2");
}

#[test]
fn test_render_after_clear_dirty_preserves_content() {
    let mut output = Output::new(10, 2);
    output.write(0, 0, "A", &Style::default());
    output.clear_dirty();
    assert_eq!(output.render(), "A");
}

#[test]
fn test_render_sparse_dirty_rows_preserves_line_gaps() {
    let mut output = Output::new(10, 4);
    output.write(0, 2, "C", &Style::default());
    assert_eq!(output.render(), "\r\n\r\nC");
}

#[test]
fn source_controls_are_replaced() {
    let mut output = Output::new(80, 1);
    output.write(
        0,
        0,
        "ok\x1b[2J\x1b[3A\x1b]0;pwn\x07\x00\x08\x7f\u{0085}\tZ",
        &Style::default(),
    );

    let rendered = output.render();
    assert_eq!(rendered, "ok␛[2J␛[3A␛]0;pwn␇␀␈␡�  Z");
    assert_no_source_controls(&rendered);
}

#[test]
fn terminal_encoder_rejects_payload_sequences() {
    let mut output = Output::new(80, 1);
    let style = Style {
        color: Some(Color::Green),
        bold: true,
        ..Style::default()
    };
    output.write(0, 0, "\x1b]0;owned\x07\x1b[2J", &style);

    let rendered = output.render();
    assert_eq!(
        rendered, "\x1b[1;32m␛]0;owned␇␛[2J\x1b[0m",
        "only structured Style may produce terminal escapes"
    );

    let source_text = rendered
        .strip_prefix("\x1b[1;32m")
        .and_then(|text| text.strip_suffix("\x1b[0m"))
        .expect("structured style encoder must wrap the rendered source");
    assert_no_source_controls(source_text);
}

#[test]
fn every_non_structural_c0_has_its_control_picture() {
    for code_point in 0_u32..=0x1f {
        let source = char::from_u32(code_point).expect("C0 must be a valid Unicode scalar");
        if matches!(source, '\n' | '\r' | '\t') {
            continue;
        }

        let mut output = Output::new(2, 1);
        output.write_char(0, 0, source, &Style::default());

        let expected =
            char::from_u32(0x2400 + code_point).expect("control picture must be valid Unicode");
        assert_eq!(
            output.cell_at(0, 0).map(|cell| cell.ch),
            Some(expected),
            "unexpected replacement for U+{code_point:04X}"
        );
        assert_no_source_controls(&output.render());
    }
}

#[test]
fn del_and_every_c1_are_replaced() {
    let mut del_output = Output::new(2, 1);
    del_output.write_char(0, 0, '\u{007f}', &Style::default());
    assert_eq!(del_output.render(), "␡");

    for code_point in 0x80_u32..=0x9f {
        let source = char::from_u32(code_point).expect("C1 must be a valid Unicode scalar");
        let mut output = Output::new(2, 1);
        output.write_char(0, 0, source, &Style::default());

        assert_eq!(
            output.cell_at(0, 0).map(|cell| cell.ch),
            Some('\u{fffd}'),
            "unexpected replacement for U+{code_point:04X}"
        );
        assert_no_source_controls(&output.render());
    }
}

#[test]
fn breaks_and_tabs_are_structured_before_cell_storage() {
    let mut text_output = Output::new(20, 1);
    text_output.write(0, 0, "ab\tZ\r\nignored", &Style::default());
    assert_eq!(text_output.render(), "ab  Z");

    let mut lf_output = Output::new(20, 1);
    lf_output.write(0, 0, "ab\nignored", &Style::default());
    assert_eq!(lf_output.render(), "ab");

    let mut char_output = Output::new(8, 1);
    let style = Style {
        background_color: Some(Color::Red),
        ..Style::default()
    };
    char_output.write_char(2, 0, '\t', &style);
    char_output.write_char(4, 0, 'Z', &Style::default());
    assert_eq!(char_output.cell_at(2, 0).map(|cell| cell.ch), Some(' '));
    assert_eq!(char_output.cell_at(3, 0).map(|cell| cell.ch), Some(' '));
    assert_eq!(
        char_output.cell_at(2, 0).and_then(|cell| cell.bg),
        Some(Color::Red)
    );
    assert_eq!(
        char_output.cell_at(3, 0).and_then(|cell| cell.bg),
        Some(Color::Red)
    );

    let mut break_output = Output::new(4, 1);
    break_output.write_char(0, 0, '\n', &Style::default());
    break_output.write_char(0, 0, '\r', &Style::default());
    assert!(break_output.is_dirty());
    assert_eq!(break_output.render(), "");
}

#[test]
fn clipped_controls_and_tabs_do_not_mutate_hidden_cells() {
    let mut text_output = Output::new(8, 1);
    text_output.write(0, 0, "abcdefgh", &Style::default());
    text_output.clear_dirty();
    text_output.clip(ClipRegion {
        x1: 4,
        y1: 0,
        x2: 8,
        y2: 1,
    });
    text_output.write(0, 0, "\x1b\tZ", &Style::default());
    text_output.unclip();

    assert_eq!(text_output.render(), "abcdZfgh");
    assert!(text_output.is_dirty());
    assert_no_source_controls(&text_output.render());

    let mut char_output = Output::new(8, 1);
    char_output.write(0, 0, "abcdefgh", &Style::default());
    char_output.clear_dirty();
    char_output.clip(ClipRegion {
        x1: 4,
        y1: 0,
        x2: 8,
        y2: 1,
    });
    char_output.write_char(0, 0, '\x1b', &Style::default());
    char_output.write_char(1, 0, '\t', &Style::default());
    char_output.write_char(4, 0, '\u{0085}', &Style::default());
    char_output.unclip();

    assert_eq!(char_output.render(), "abcd�fgh");
    assert!(char_output.is_dirty());
    assert_no_source_controls(&char_output.render());
}

#[test]
fn tabs_and_control_replacements_are_safe_at_right_edge() {
    let mut text_tab = Output::new(6, 1);
    text_tab.write(0, 0, "XXXXXX", &Style::default());
    text_tab.clear_dirty();
    text_tab.write(5, 0, "\t", &Style::default());
    assert_eq!(text_tab.cell_at(4, 0).map(|cell| cell.ch), Some('X'));
    assert_eq!(text_tab.cell_at(5, 0).map(|cell| cell.ch), Some(' '));
    assert!(text_tab.is_dirty());
    assert_no_source_controls(&text_tab.render());

    let mut char_tab = Output::new(6, 1);
    char_tab.write(0, 0, "XXXXXX", &Style::default());
    char_tab.clear_dirty();
    char_tab.write_char(5, 0, '\t', &Style::default());
    assert_eq!(char_tab.cell_at(4, 0).map(|cell| cell.ch), Some('X'));
    assert_eq!(char_tab.cell_at(5, 0).map(|cell| cell.ch), Some(' '));
    assert!(char_tab.is_dirty());
    assert_no_source_controls(&char_tab.render());

    let mut text_control = Output::new(1, 1);
    text_control.write(0, 0, "\x1bX", &Style::default());
    assert_eq!(text_control.render(), "␛");
    assert_no_source_controls(&text_control.render());

    let mut char_control = Output::new(1, 1);
    char_control.write_char(0, 0, '\u{009f}', &Style::default());
    assert_eq!(char_control.render(), "�");
    assert_no_source_controls(&char_control.render());
}

#[test]
fn out_of_bounds_and_clipped_writes_preserve_dirty_contract() {
    let mut output = Output::new(2, 1);
    output.write(0, 1, "\x1b", &Style::default());
    output.write_char(2, 0, '\x1b', &Style::default());
    assert!(!output.is_dirty());
    assert_eq!(output.render(), "");

    output.write(2, 0, "\x1b", &Style::default());
    assert!(output.is_dirty());
    assert_eq!(output.render(), "");

    output.clear_dirty();
    output.clip(ClipRegion {
        x1: 1,
        y1: 0,
        x2: 2,
        y2: 1,
    });
    output.write_char(0, 0, '\x1b', &Style::default());
    output.unclip();
    assert!(output.is_dirty());
    assert_eq!(output.render(), "");
    assert_no_source_controls(&output.render());
}

#[test]
fn ordinary_unicode_and_structured_style_are_unchanged() {
    let mut output = Output::new(20, 1);
    let style = Style {
        underline: true,
        ..Style::default()
    };
    output.write(0, 0, "hé🙂界", &style);

    assert_eq!(output.render(), "\x1b[4mhé🙂界\x1b[0m");
}

#[test]
fn whole_egc_writes_are_atomic_at_bounds_and_nested_clips() {
    let mut bounded = Output::new(4, 1);
    bounded.write(0, 0, "abcd", &Style::default());
    bounded.write(3, 0, "你", &Style::default());
    assert_eq!(bounded.render(), "abcd");

    let mut clipped = Output::new(4, 1);
    clipped.write(0, 0, "abcd", &Style::default());
    clipped.clip(ClipRegion {
        x1: 1,
        y1: 0,
        x2: 4,
        y2: 1,
    });
    clipped.clip(ClipRegion {
        x1: 1,
        y1: 0,
        x2: 2,
        y2: 1,
    });
    clipped.write(1, 0, "你", &Style::default());
    clipped.unclip();
    clipped.unclip();
    assert_eq!(clipped.render(), "abcd");
}

#[test]
fn combining_and_zwj_suffixes_preserve_grapheme_order() {
    let mut output = Output::new(8, 1);
    output.write(0, 0, "e\u{301}\u{323}", &Style::default());
    output.write(1, 0, "👩\u{200d}💻", &Style::default());

    assert_eq!(output.render(), "e\u{301}\u{323}👩\u{200d}💻");
}

#[test]
fn zero_width_attachments_follow_the_existing_lead_in_order() {
    let mut output = Output::new(4, 1);
    output.write_char(0, 0, 'e', &Style::default());
    output.write_char(1, 0, '\u{301}', &Style::default());
    output.write_char(1, 0, '\u{323}', &Style::default());
    assert_eq!(output.render(), "e\u{301}\u{323}");

    let mut no_lead = Output::new(2, 1);
    no_lead.write_char(0, 0, '\u{301}', &Style::default());
    assert_eq!(no_lead.render(), "");
}

#[test]
fn zero_width_prospective_matches_actual_owner_and_clip_states() {
    let mut no_owner = Output::new(2, 1);
    assert_eq!(
        no_owner.prospective_grapheme_write_footprint(1, 0, "\u{301}"),
        None
    );
    assert_eq!(
        no_owner.write_grapheme(1, 0, "\u{301}", &Style::default()),
        GraphemeWriteOutcome::Clipped
    );

    let mut valid_owner = Output::new(2, 1);
    valid_owner.write(0, 0, "e", &Style::default());
    valid_owner.clear_dirty();
    let expected = GraphemeWriteFootprint {
        target_cells: Vec::new(),
        old_cells: vec![CellPosition { x: 0, y: 0 }],
    };
    assert_eq!(
        valid_owner.prospective_grapheme_write_footprint(1, 0, "\u{301}"),
        Some(expected.clone())
    );
    assert_eq!(
        valid_owner.write_grapheme(1, 0, "\u{301}", &Style::default()),
        GraphemeWriteOutcome::Committed(expected)
    );
    assert_eq!(valid_owner.render(), "e\u{301}");
    assert_eq!(
        valid_owner.dirty_cell_positions().collect::<Vec<_>>(),
        vec![CellPosition { x: 0, y: 0 }]
    );

    let mut clipped_owner = Output::new(3, 1);
    clipped_owner.write(0, 0, "你", &Style::default());
    clipped_owner.clear_dirty();
    clipped_owner.clip(ClipRegion {
        x1: 1,
        y1: 0,
        x2: 2,
        y2: 1,
    });
    assert_eq!(
        clipped_owner.prospective_grapheme_write_footprint(2, 0, "\u{301}"),
        None
    );
    assert_eq!(
        clipped_owner.write_grapheme(2, 0, "\u{301}", &Style::default()),
        GraphemeWriteOutcome::Clipped
    );
    clipped_owner.unclip();
    assert_eq!(clipped_owner.render(), "你");
    assert_eq!(clipped_owner.dirty_cell_positions().count(), 0);
}

#[test]
fn repaint_clears_complete_old_and_target_footprints() {
    let mut wide_over_narrow = Output::new(4, 1);
    wide_over_narrow.write(0, 0, "ABC", &Style::default());
    wide_over_narrow.clear_dirty();
    wide_over_narrow.write(0, 0, "你", &Style::default());
    assert_eq!(wide_over_narrow.render(), "你C");
    assert_eq!(
        wide_over_narrow.dirty_cell_positions().collect::<Vec<_>>(),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );

    let mut narrow_over_wide = Output::new(4, 1);
    narrow_over_wide.write(0, 0, "你", &Style::default());
    narrow_over_wide.clear_dirty();
    narrow_over_wide.write(1, 0, "X", &Style::default());
    assert_eq!(narrow_over_wide.render(), " X");
    assert_eq!(
        narrow_over_wide.dirty_cell_positions().collect::<Vec<_>>(),
        vec![CellPosition { x: 0, y: 0 }, CellPosition { x: 1, y: 0 }]
    );

    let mut wide_over_wide = Output::new(5, 1);
    wide_over_wide.write(1, 0, "你", &Style::default());
    wide_over_wide.clear_dirty();
    wide_over_wide.write(2, 0, "界", &Style::default());
    assert_eq!(wide_over_wide.render(), "  界");
    assert_eq!(
        wide_over_wide.dirty_cell_positions().collect::<Vec<_>>(),
        vec![
            CellPosition { x: 1, y: 0 },
            CellPosition { x: 2, y: 0 },
            CellPosition { x: 3, y: 0 },
        ]
    );
}

#[test]
fn active_clips_report_grapheme_visibility() {
    let mut output = Output::new(5, 2);
    output.write(0, 0, "abc", &Style::default());
    output.clear_dirty();
    output.clip(ClipRegion {
        x1: 0,
        y1: 0,
        x2: 4,
        y2: 2,
    });
    output.clip(ClipRegion {
        x1: 1,
        y1: 0,
        x2: 4,
        y2: 1,
    });

    let before_grid = output.grid.iter().map(|cell| cell.ch).collect::<Vec<_>>();
    let before_metadata = output.grapheme_cells.clone();
    let before_dirty = output.dirty_cells.clone();
    assert!(output.active_clips_contain_grapheme(1, 0, 2));
    assert!(!output.active_clips_contain_grapheme(3, 0, 2));
    assert!(!output.active_clips_contain_grapheme(1, 1, 2));
    assert!(!output.active_clips_contain_grapheme(-1, 0, 2));
    assert!(!output.active_clips_contain_grapheme(4, 0, 2));
    assert_eq!(
        output.grid.iter().map(|cell| cell.ch).collect::<Vec<_>>(),
        before_grid
    );
    assert_eq!(output.grapheme_cells, before_metadata);
    assert_eq!(output.dirty_cells, before_dirty);
    assert_eq!(output.clip_depth(), 2);
}

#[test]
fn staged_snapshot_and_write_footprint_are_isolated() {
    let mut source = Output::new(6, 2);
    source.write(1, 0, "你", &Style::default());
    source.clear_dirty();
    source.clip(ClipRegion {
        x1: 0,
        y1: 0,
        x2: 6,
        y2: 2,
    });
    source.clip(ClipRegion {
        x1: 2,
        y1: 0,
        x2: 3,
        y2: 1,
    });

    let mut staged = source.staged_snapshot();
    assert_eq!((staged.width, staged.height), (6, 2));
    assert_eq!(staged.clip_depth(), 2);
    assert_eq!(staged.cell_at(1, 0).map(|cell| cell.ch), Some('你'));
    assert_eq!(staged.cell_at(2, 0).map(|cell| cell.ch), Some('\0'));
    assert!(
        staged
            .prospective_grapheme_write_footprint(2, 0, "X")
            .is_none(),
        "the old wide lead outside the inner clip makes the repaint atomic miss"
    );

    staged.unclip();
    let footprint = staged
        .prospective_grapheme_write_footprint(2, 0, "X")
        .expect("the outer clip contains both target and prior wide footprint");
    assert_eq!(footprint.target_cells, vec![CellPosition { x: 2, y: 0 }]);
    assert_eq!(
        footprint.old_cells,
        vec![CellPosition { x: 1, y: 0 }, CellPosition { x: 2, y: 0 }]
    );
    assert!(matches!(
        staged.write_grapheme(2, 0, "X", &Style::default()),
        GraphemeWriteOutcome::Committed(_)
    ));
    assert!(matches!(
        staged.write_grapheme(4, 0, "\x1b", &Style::default()),
        GraphemeWriteOutcome::Committed(_)
    ));
    staged.unclip();

    assert_eq!(source.cell_at(1, 0).map(|cell| cell.ch), Some('你'));
    assert_eq!(source.cell_at(2, 0).map(|cell| cell.ch), Some('\0'));
    assert_eq!(source.cell_at(4, 0).map(|cell| cell.ch), Some(' '));
    assert_eq!(source.clip_depth(), 2);
    assert_eq!(source.dirty_cell_positions().count(), 0);

    assert_eq!(staged.cell_at(1, 0).map(|cell| cell.ch), Some(' '));
    assert_eq!(staged.cell_at(2, 0).map(|cell| cell.ch), Some('X'));
    assert_eq!(staged.cell_at(4, 0).map(|cell| cell.ch), Some('␛'));
    assert_eq!(
        staged.dirty_cell_positions().collect::<Vec<_>>(),
        vec![
            CellPosition { x: 1, y: 0 },
            CellPosition { x: 2, y: 0 },
            CellPosition { x: 4, y: 0 },
        ]
    );
    assert_no_source_controls(&staged.render());

    let mut receiver = Output::new(1, 1);
    receiver.commit_staged(staged);
    assert_eq!((receiver.width, receiver.height), (6, 2));
    assert_eq!(receiver.render(), "  X ␛");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "Output::unclip called with an empty clip stack")]
fn test_unclip_panics_when_stack_is_empty_in_debug() {
    let mut output = Output::new(10, 5);
    output.unclip();
}

#[test]
fn test_clip_depth_tracks_push_and_pop() {
    let mut output = Output::new(10, 5);
    assert_eq!(output.clip_depth(), 0);

    output.clip(ClipRegion {
        x1: 0,
        y1: 0,
        x2: 5,
        y2: 5,
    });
    assert_eq!(output.clip_depth(), 1);

    output.unclip();
    assert_eq!(output.clip_depth(), 0);
}

#[test]
#[should_panic(expected = "Output::render called with an unbalanced clip stack")]
fn test_render_panics_with_active_clip_stack() {
    let mut output = Output::new(10, 5);
    output.clip(ClipRegion {
        x1: 0,
        y1: 0,
        x2: 5,
        y2: 5,
    });
    let _ = output.render();
}
