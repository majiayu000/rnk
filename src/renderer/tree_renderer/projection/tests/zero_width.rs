use super::*;

#[test]
fn projection_zero_width_only_attaches_to_the_same_flow_sequence() {
    let mut attached = Element::text("A\u{200b}");
    attached.style.width = 2.into();
    attached.style.height = 1.into();
    let (_, attached_output, attached_projection) = layout_and_project(&attached, 2, 1);
    let base = source_record(&attached_projection, 0..1);
    let zero = source_record(&attached_projection, 1..4);
    assert_eq!(attached_output.render(), "A\u{200b}");
    assert_eq!(
        attached_projection.reverse.get(&FrameCell { x: 0, y: 0 }),
        Some(&base.origin())
    );
    assert_eq!(
        zero.frame,
        FrameDisposition::NonCell(NonCellDisposition::ZeroWidth)
    );

    for (wrap, width, expected, ellipsis_x) in [
        (TextWrap::TruncateStart, 3, "…\u{200b}XY", 0),
        (TextWrap::TruncateMiddle, 4, "a…\u{200b}XY", 1),
    ] {
        let input =
            TextFlowInput::plain("abc\u{200b}XY", TextFlowSourceKind::Exact, Style::default());
        let flow = TextFlow::try_build(&input, &TextFlowOptions::new(width, wrap)).unwrap();
        let mut staged = StagedFrame::new(&Output::new(width as u16, 1), Default::default());
        let element_id = ElementId::new();
        staged.project_flow(element_id, &flow, 0, 0).unwrap();
        let (output, projection) = staged.finish().unwrap();
        assert_eq!(output.render(), expected);
        assert_eq!(
            source_record(&projection, 3..6).frame,
            FrameDisposition::NonCell(NonCellDisposition::ZeroWidth)
        );
        let synthetic = projection
            .forward
            .iter()
            .find(|record| record.source == TextFlowSource::Synthetic)
            .unwrap();
        assert_eq!(
            projection.reverse.get(&FrameCell {
                x: ellipsis_x,
                y: 0
            }),
            Some(&synthetic.origin())
        );
        validate_round_trip(&projection).unwrap();
    }

    let mut nonzero = Element::text("\u{301}");
    nonzero.style.position = Position::Absolute;
    nonzero.style.left = Some(1.0);
    nonzero.style.top = Some(0.0);
    nonzero.style.width = 1.into();
    nonzero.style.height = 1.into();
    let mut nonzero_engine = LayoutEngine::new();
    nonzero_engine.try_compute(&nonzero, 3, 1).unwrap();
    let mut preexisting = Output::new(3, 1);
    preexisting.write(0, 0, "P", &Style::default());
    let nonzero_projection =
        try_render_tree(&nonzero, &nonzero_engine, &mut preexisting, 0.0, 0.0).unwrap();
    assert_eq!(preexisting.render(), "P");
    assert!(nonzero_projection.reverse.is_empty());

    let mut background = Element::text("\u{301}");
    background.style.position = Position::Absolute;
    background.style.left = Some(1.0);
    background.style.top = Some(0.0);
    background.style.width = 2.into();
    background.style.height = 1.into();
    background.style.padding.left = 1.0;
    background.style.background_color = Some(Color::Blue);
    let (_, background_output, background_projection) = layout_and_project(&background, 4, 1);
    assert!(!background_output.render().contains('\u{301}'));
    assert!(background_projection.reverse.is_empty());

    let mut sibling_zero = Element::text("\u{301}");
    sibling_zero.style.position = Position::Absolute;
    sibling_zero.style.left = Some(1.0);
    sibling_zero.style.top = Some(0.0);
    sibling_zero.style.width = 1.into();
    sibling_zero.style.height = 1.into();
    let sibling_tree = Box::new()
        .width(3)
        .height(1)
        .children([
            {
                let mut text = Element::text("S");
                text.style.position = Position::Absolute;
                text.style.left = Some(0.0);
                text.style.top = Some(0.0);
                text.style.width = 1.into();
                text.style.height = 1.into();
                text
            },
            sibling_zero,
        ])
        .into_element();
    let (_, sibling_output, sibling_projection) = layout_and_project(&sibling_tree, 3, 1);
    assert_eq!(sibling_output.render(), "S");
    assert_eq!(sibling_projection.reverse.len(), 1);
}

#[test]
fn synthetic_ellipsis_projection_failure_commits_neither_cells_nor_projection() {
    let mut tree = Element::text("abc\u{200b}XY");
    tree.style.width = 3.into();
    tree.style.height = 1.into();
    tree.style.text_wrap = TextWrap::TruncateStart;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&tree, 3, 1).unwrap();

    let mut output = Output::new(3, 1);
    output.write(0, 0, "old", &Style::default());
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
            ..ProjectionOptions::default()
        },
    );

    assert_eq!(failure, Err(ProjectionError::InjectedFailure));
    assert_eq!((output.width, output.height), (3, 1));
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
