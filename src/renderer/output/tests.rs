//! Unit tests for the output buffer, split out of `output.rs` to keep that
//! file within the module size limit.

use super::*;

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
fn source_controls_are_replaced() {
    // `Output` is public, so a caller can hand it raw source that never passed
    // through `flow_text`. A stored ESC would be emitted verbatim by the
    // encoder and executed by the terminal.
    let mut output = Output::new(80, 24);
    output.write(0, 0, "a\u{1b}[2Jb", &Style::default());

    assert_eq!(output.cell_at(1, 0).unwrap().ch, '␛');
    let rendered = output.render();
    assert!(
        !rendered.contains("\u{1b}[2J"),
        "screen-clear payload reached the terminal stream: {rendered:?}"
    );

    // DEL and C1 arrive through `write_char` instead.
    output.write_char(0, 1, '\u{7f}', &Style::default());
    output.write_char(1, 1, '\u{9b}', &Style::default());
    assert_eq!(output.cell_at(0, 1).unwrap().ch, '␡');
    assert_eq!(output.cell_at(1, 1).unwrap().ch, '\u{fffd}');
}

#[test]
fn a_source_nul_is_not_confused_with_a_wide_char_placeholder() {
    // `'\0'` marks the second cell of a wide grapheme, and `render_row` skips
    // those cells. Storing a source NUL as-is would silently drop it.
    let mut output = Output::new(80, 24);
    output.write(0, 0, "\u{0}", &Style::default());

    assert_eq!(output.cell_at(0, 0).unwrap().ch, '␀');
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
