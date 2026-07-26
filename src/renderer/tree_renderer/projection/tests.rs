use std::collections::HashMap;
use std::ops::Range;

use crate::components::{Box, Text};
use crate::core::{BorderStyle, Color, Element, ElementId, Overflow, Position, Style, TextWrap};
use crate::layout::LayoutEngine;
use crate::layout::text_flow::{TextFlowPlacement, TextFlowSource, TextFlowSourceKind};
use crate::renderer::Output;
use crate::renderer::output::ClipRegion;

use super::*;

fn layout_and_project(
    element: &Element,
    width: u16,
    height: u16,
) -> (LayoutEngine, Output, RenderProjection) {
    let mut engine = LayoutEngine::new();
    engine.try_compute(element, width, height).unwrap();
    let mut output = Output::new(width, height);
    let projection = try_render_tree(element, &engine, &mut output, 0.0, 0.0).unwrap();
    (engine, output, projection)
}

fn source_record(projection: &RenderProjection, range: Range<usize>) -> &ForwardProjection {
    projection
        .forward
        .iter()
        .find(|record| {
            matches!(
                &record.source,
                TextFlowSource::Source {
                    range: candidate,
                    ..
                } if *candidate == range
            )
        })
        .unwrap()
}

fn visible_cells(record: &ForwardProjection) -> &[FrameCell] {
    let FrameDisposition::Cells { visible, .. } = &record.frame else {
        panic!("expected a cell disposition, got {:?}", record.frame);
    };
    visible
}

fn clipped_cells(record: &ForwardProjection) -> &[SignedCell] {
    let FrameDisposition::Cells { clipped, .. } = &record.frame else {
        panic!("expected a cell disposition, got {:?}", record.frame);
    };
    clipped
}

#[test]
fn projection_source_cell_round_trip_records_visible_clipped_and_synthetic_cells() {
    let source = "A\t界e\u{301}👩\u{200d}💻\u{1b}\n\u{301}";
    let mut element = Element::text(source);
    element.style.width = 20.into();
    element.style.height = 2.into();
    let (_, output, projection) = layout_and_project(&element, 20, 2);

    let tab_start = source.find('\t').unwrap();
    let tab = source_record(&projection, tab_start..tab_start + 1);
    assert_eq!(
        visible_cells(tab),
        &[
            FrameCell { x: 1, y: 0 },
            FrameCell { x: 2, y: 0 },
            FrameCell { x: 3, y: 0 },
        ]
    );
    for cell in visible_cells(tab) {
        assert_eq!(
            projection.reverse.get(cell),
            Some(&CellOrigin::Source {
                id: tab.id,
                range: tab_start..tab_start + 1,
            })
        );
    }

    let wide_start = source.find('界').unwrap();
    let wide = source_record(&projection, wide_start..wide_start + '界'.len_utf8());
    assert_eq!(
        visible_cells(wide),
        &[FrameCell { x: 4, y: 0 }, FrameCell { x: 5, y: 0 }]
    );
    assert_eq!(
        projection.reverse.get(&FrameCell { x: 5, y: 0 }),
        Some(&wide.origin())
    );

    let combining_start = source.find("e\u{301}").unwrap();
    let combining = source_record(&projection, combining_start..combining_start + 3);
    assert_eq!(combining.text, "e\u{301}");
    assert_eq!(visible_cells(combining), &[FrameCell { x: 6, y: 0 }]);

    let zwj_start = source.find("👩\u{200d}💻").unwrap();
    let zwj = source_record(&projection, zwj_start..zwj_start + "👩\u{200d}💻".len());
    assert_eq!(zwj.text, "👩\u{200d}💻");
    assert_eq!(
        visible_cells(zwj),
        &[FrameCell { x: 7, y: 0 }, FrameCell { x: 8, y: 0 }]
    );

    let control_start = source.find('\u{1b}').unwrap();
    let control = source_record(&projection, control_start..control_start + 1);
    assert_eq!(control.text, "␛");
    assert!(matches!(
        control.logical,
        TextFlowPlacement::SanitizedControl { .. }
    ));
    assert!(!output.render().contains('\u{1b}'));

    let break_start = source.find('\n').unwrap();
    let hard_break = source_record(&projection, break_start..break_start + 1);
    assert_eq!(
        hard_break.frame,
        FrameDisposition::NonCell(NonCellDisposition::HardBreak)
    );
    let zero_start = break_start + 1;
    let zero_width = source_record(&projection, zero_start..source.len());
    assert_eq!(
        zero_width.frame,
        FrameDisposition::NonCell(NonCellDisposition::ZeroWidth)
    );

    let mut truncated = Element::text("abcdef");
    truncated.style.width = 4.into();
    truncated.style.height = 1.into();
    truncated.style.text_wrap = TextWrap::TruncateEnd;
    let mut truncate_engine = LayoutEngine::new();
    truncate_engine.set_text_flow_policy(4, "..", 1);
    truncate_engine.try_compute(&truncated, 4, 1).unwrap();
    let mut truncate_output = Output::new(4, 1);
    let truncate_projection =
        try_render_tree(&truncated, &truncate_engine, &mut truncate_output, 0.0, 0.0).unwrap();
    let synthetic = truncate_projection
        .forward
        .iter()
        .filter(|record| record.source == TextFlowSource::Synthetic)
        .collect::<Vec<_>>();
    assert_eq!(synthetic.len(), 2);
    assert_ne!(synthetic[0].id, synthetic[1].id);
    assert!(synthetic.iter().all(|record| {
        visible_cells(record).iter().all(|cell| {
            matches!(
                truncate_projection.reverse.get(cell),
                Some(CellOrigin::Synthetic { id }) if *id == record.id
            )
        })
    }));
    assert!(truncate_projection.forward.iter().any(|record| {
        record.frame == FrameDisposition::NonCell(NonCellDisposition::Truncated)
    }));

    let mut omitted = Element::text("x");
    omitted.style.width = 0.into();
    omitted.style.height = 1.into();
    let (_, omitted_output, omitted_projection) = layout_and_project(&omitted, 1, 1);
    assert_eq!(
        omitted_projection.forward[0].frame,
        FrameDisposition::NonCell(NonCellDisposition::Omitted)
    );
    assert!(omitted_projection.reverse.is_empty());
    assert_eq!(omitted_output.render(), "");

    let mut clipped_wide = Element::text("界");
    clipped_wide.style.width = 1.into();
    clipped_wide.style.height = 1.into();
    clipped_wide.style.overflow_x = Overflow::Hidden;
    let (_, clipped_output, clipped_projection) = layout_and_project(&clipped_wide, 1, 1);
    let clipped = &clipped_projection.forward[0];
    assert!(visible_cells(clipped).is_empty());
    assert_eq!(
        clipped_cells(clipped),
        &[SignedCell { x: 0, y: 0 }, SignedCell { x: 1, y: 0 }]
    );
    assert!(clipped_projection.reverse.is_empty());
    assert_eq!(clipped_output.render(), "");

    let mut gapped = clipped_projection.clone();
    let FrameDisposition::Cells { clipped, .. } = &mut gapped.forward[0].frame else {
        panic!("wide source must have clipped cells");
    };
    clipped[1].x = 2;
    assert_eq!(
        validate_round_trip(&gapped),
        Err(ProjectionError::MalformedProjection(
            "token cells contain a gap"
        ))
    );

    let mut malformed = projection.clone();
    malformed.reverse.remove(&FrameCell { x: 1, y: 0 });
    assert_eq!(
        validate_round_trip(&malformed),
        Err(ProjectionError::WriterOutcomeMismatch)
    );
    assert_eq!(projection.stats.committed_replacements, 1);
}

#[test]
fn projection_signed_coordinates_axis_clips_and_nested_active_clips_are_exact() {
    let mut vertical = Element::text("a\nb");
    vertical.style.width = 1.into();
    vertical.style.height = 1.into();
    vertical.style.overflow_y = Overflow::Hidden;
    let (_, _, vertical_projection) = layout_and_project(&vertical, 2, 2);
    assert_eq!(
        visible_cells(source_record(&vertical_projection, 0..1)),
        &[FrameCell { x: 0, y: 0 }]
    );
    assert_eq!(
        clipped_cells(source_record(&vertical_projection, 2..3)),
        &[SignedCell { x: 0, y: 1 }]
    );

    let mut horizontal = Element::text("界\nz");
    horizontal.style.width = 1.into();
    horizontal.style.height = 2.into();
    horizontal.style.overflow_x = Overflow::Hidden;
    let (_, _, horizontal_projection) = layout_and_project(&horizontal, 2, 2);
    assert_eq!(
        clipped_cells(source_record(&horizontal_projection, 0..3)),
        &[SignedCell { x: 0, y: 0 }, SignedCell { x: 1, y: 0 }]
    );
    assert_eq!(
        visible_cells(source_record(&horizontal_projection, 4..5)),
        &[FrameCell { x: 0, y: 1 }]
    );

    let mut left = Element::text("ab");
    left.style.width = 2.into();
    left.style.height = 1.into();
    left.style.overflow_x = Overflow::Scroll;
    left.scroll_offset_x = Some(1);
    let (_, _, left_projection) = layout_and_project(&left, 2, 1);
    assert_eq!(
        clipped_cells(source_record(&left_projection, 0..1)),
        &[SignedCell { x: -1, y: 0 }]
    );
    assert_eq!(
        visible_cells(source_record(&left_projection, 1..2)),
        &[FrameCell { x: 0, y: 0 }]
    );

    let mut up = Element::text("a\nb");
    up.style.width = 1.into();
    up.style.height = 2.into();
    up.style.overflow_y = Overflow::Scroll;
    up.scroll_offset_y = Some(1);
    let (_, _, up_projection) = layout_and_project(&up, 1, 2);
    assert_eq!(
        clipped_cells(source_record(&up_projection, 0..1)),
        &[SignedCell { x: 0, y: -1 }]
    );
    assert_eq!(
        visible_cells(source_record(&up_projection, 2..3)),
        &[FrameCell { x: 0, y: 0 }]
    );

    let mut child = Element::text("ab");
    child.style.width = 2.into();
    child.style.height = 1.into();
    child.style.flex_shrink = 0.0;
    let nested = Box::new()
        .width(1)
        .height(1)
        .overflow_x(Overflow::Hidden)
        .child(child)
        .into_element();
    let mut engine = LayoutEngine::new();
    engine.try_compute(&nested, 2, 1).unwrap();
    let child_id = nested.children.iter().next().unwrap().id;
    let mut active = Output::new(2, 1);
    active.clip(ClipRegion {
        x1: 0,
        y1: 0,
        x2: 2,
        y2: 1,
    });
    let nested_projection = try_render_tree(&nested, &engine, &mut active, 0.0, 0.0).unwrap();
    assert_eq!(active.clip_depth(), 1);
    assert_eq!(
        visible_cells(nested_projection.forward_for(child_id, 0).unwrap()),
        &[FrameCell { x: 0, y: 0 }]
    );
    assert_eq!(
        clipped_cells(nested_projection.forward_for(child_id, 1).unwrap()),
        &[SignedCell { x: 1, y: 0 }]
    );
    active.unclip();
}

#[test]
fn projection_later_paint_replaces_old_wide_ownership_deterministically() {
    fn absolute_text(content: &str, width: u16) -> Element {
        let mut element = Element::text(content);
        element.style.position = Position::Absolute;
        element.style.left = Some(0.0);
        element.style.top = Some(0.0);
        element.style.width = width.into();
        element.style.height = 1.into();
        element
    }

    let wide = absolute_text("界", 2);
    let wide_id = wide.id;
    let mut middle = Box::new()
        .width(1)
        .height(1)
        .position_absolute()
        .left(0.0)
        .top(0.0)
        .border_style(BorderStyle::Single)
        .into_element();
    middle.style.background_color = Some(Color::Blue);
    let final_text = absolute_text("Z", 1);
    let final_id = final_text.id;
    let tree = Box::new()
        .width(3)
        .height(1)
        .children([wide, middle, final_text])
        .into_element();

    let (_, output, projection) = layout_and_project(&tree, 3, 1);
    let FrameDisposition::Cells {
        visible, replaced, ..
    } = &projection.forward_for(wide_id, 0).unwrap().frame
    else {
        panic!("wide source must have a cell disposition");
    };
    assert!(visible.is_empty());
    assert_eq!(
        replaced,
        &[FrameCell { x: 0, y: 0 }, FrameCell { x: 1, y: 0 }]
    );
    assert_eq!(
        projection.reverse.get(&FrameCell { x: 0, y: 0 }),
        Some(&projection.forward_for(final_id, 0).unwrap().origin())
    );
    assert!(!projection.reverse.contains_key(&FrameCell { x: 1, y: 0 }));
    assert_eq!(output.cell_at(0, 0).unwrap().ch, 'Z');
    assert_eq!(output.cell_at(1, 0).unwrap().ch, '┐');
}

#[test]
fn projection_failure_commits_neither_cells_nor_projection() {
    let tree = Box::new()
        .width(8)
        .height(2)
        .child(Text::new("first").into_element())
        .into_element();
    let mut engine = LayoutEngine::new();
    engine.try_compute(&tree, 8, 2).unwrap();

    let mut output = Output::new(8, 2);
    output.write(0, 0, "e\u{301}界", &Style::default());
    let before_render = output.render();
    let before_dirty = output.dirty_cell_positions().collect::<Vec<_>>();
    let before_footprint = output.prospective_grapheme_write_footprint(1, 0, "X");

    let failure = try_render_tree_with_options(
        &tree,
        &engine,
        &mut output,
        0.0,
        0.0,
        ProjectionOptions {
            fail_after_writes: Some(1),
        },
    );

    assert_eq!(failure, Err(ProjectionError::InjectedFailure));
    assert_eq!((output.width, output.height), (8, 2));
    assert_eq!(output.render(), before_render);
    assert_eq!(
        output.dirty_cell_positions().collect::<Vec<_>>(),
        before_dirty
    );
    assert_eq!(
        output.prospective_grapheme_write_footprint(1, 0, "X"),
        before_footprint
    );
    assert_eq!(output.clip_depth(), 0);
}

#[test]
fn projection_round_trip_validation_is_linear() {
    fn projection_with_cells(count: usize) -> RenderProjection {
        let element_id = ElementId::new();
        let mut forward = Vec::with_capacity(count);
        let mut reverse = HashMap::with_capacity(count);
        for index in 0..count {
            let id = ProjectionId {
                element_id,
                token_index: index,
            };
            let cell = FrameCell {
                x: u16::try_from(index).unwrap(),
                y: 0,
            };
            let source = TextFlowSource::Source {
                range: index..index + 1,
                kind: TextFlowSourceKind::Exact,
            };
            let record = ForwardProjection {
                id,
                source,
                logical: TextFlowPlacement::Positioned {
                    row: 0,
                    column: index,
                },
                text: "x".to_string(),
                display_width: 1,
                frame: FrameDisposition::Cells {
                    visible: vec![cell],
                    clipped: Vec::new(),
                    replaced: Vec::new(),
                },
            };
            reverse.insert(cell, record.origin());
            forward.push(record);
        }
        RenderProjection {
            forward,
            reverse,
            stats: ProjectionStats::default(),
        }
    }

    for count in [2_000, 10_000] {
        let projection = projection_with_cells(count);
        assert_eq!(validate_round_trip(&projection), Ok(count * 3));
    }
}
