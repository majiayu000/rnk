//! Signed fractional coordinates and failing-element identity (GH-132).
//!
//! Two failures shared one root cause: coordinate handling that discarded
//! information on the way from layout space to the cell grid. Truncation
//! toward zero erased the sign of a coordinate in `(-1.0, 0.0)`, and coordinate
//! errors raised below the tree walk carried no element, so every caller
//! reported the root.

use rnk::components::{Box, Text};
use rnk::core::{Element, Overflow, Style};
use rnk::layout::{LayoutEngine, LayoutSnapshotError, TransactionalLayoutError};
use rnk::renderer::{Output, TextCoordinateError, TextRenderError, try_render_to_string};
use rnk::testing::TestRenderer;

/// A child whose own painting fails, nested one level below the root.
fn tree_with_failing_grandchild(padding_left: f32) -> (Element, rnk::core::ElementId) {
    let mut failing = Text::new("child").into_element();
    failing.style.padding.left = padding_left;
    let failing_id = failing.id;

    let tree = Box::new()
        .width(20)
        .height(4)
        .child(Text::new("sibling").into_element())
        .child(Box::new().child(failing).into_element())
        .into_element();

    (tree, failing_id)
}

/// Render `text` with a raw horizontal padding that reaches the projector
/// unrounded. Taffy rounds layout positions, so padding is the shortest path
/// from a public API to a fractional coordinate.
fn render_with_fractional_padding(text: &str, padding_left: f32) -> String {
    let mut padded = Text::new(text).into_element();
    padded.style.padding.left = padding_left;
    let element = Box::new()
        .width(12)
        .height(2)
        .overflow_x(Overflow::Hidden)
        .child(padded)
        .into_element();

    TestRenderer::new(12, 2)
        .try_render_to_plain(&element)
        .unwrap_or_else(|error| panic!("padding {padding_left} failed to render: {error}"))
}

#[test]
fn negative_fractional_coordinates_stay_negative_and_clip() {
    // x = -0.5 lies in cell -1, which is off-screen to the left, so the first
    // column of text is clipped. Truncating toward zero would round it up to
    // cell 0 and paint the whole string at the viewport edge, silently moving
    // content the caller placed outside the frame back into it.
    assert_eq!(render_with_fractional_padding("abc", -0.5).trim_end(), "bc");
    assert_eq!(render_with_fractional_padding("abc", -0.9).trim_end(), "bc");

    // The same holds a whole cell further out.
    assert_eq!(render_with_fractional_padding("abc", -1.5).trim_end(), "c");
}

#[test]
fn positive_fractional_coordinates_keep_their_containing_cell() {
    // floor and truncation agree at and above zero, so this is unchanged.
    assert_eq!(render_with_fractional_padding("abc", 0.0).trim_end(), "abc");
    assert_eq!(render_with_fractional_padding("abc", 0.5).trim_end(), "abc");
    assert_eq!(render_with_fractional_padding("abc", 0.9).trim_end(), "abc");
    assert_eq!(
        render_with_fractional_padding("abc", 1.5).trim_end(),
        " abc"
    );
}

#[test]
fn nested_non_finite_coordinate_names_the_failing_child() {
    let (tree, failing_id) = tree_with_failing_grandchild(f32::NAN);

    let error = try_render_to_string(&tree, 20).expect_err("NaN padding must fail");
    match error {
        TextRenderError::Coordinate { element_id, source } => {
            assert_eq!(source, TextCoordinateError::NonFinite);
            assert_eq!(
                element_id, failing_id,
                "coordinate failure was attributed to an ancestor instead of the failing child"
            );
        }
        other => panic!("expected a coordinate failure, got {other:?}"),
    }
}

#[test]
fn nested_overflowing_coordinate_names_the_failing_child() {
    let (tree, failing_id) = tree_with_failing_grandchild(f32::MAX);

    let error = try_render_to_string(&tree, 20).expect_err("overflowing padding must fail");
    match error {
        TextRenderError::Coordinate { element_id, source } => {
            assert_eq!(source, TextCoordinateError::Overflow);
            assert_eq!(
                element_id, failing_id,
                "coordinate failure was attributed to an ancestor instead of the failing child"
            );
        }
        other => panic!("expected a coordinate failure, got {other:?}"),
    }
}

#[test]
fn test_renderer_reports_the_same_failing_child_as_the_string_api() {
    let (tree, failing_id) = tree_with_failing_grandchild(f32::NAN);
    let renderer = TestRenderer::new(20, 4);

    for error in [
        renderer
            .try_render_to_plain(&tree)
            .expect_err("plain rendering must fail"),
        renderer
            .try_render_to_ansi(&tree)
            .expect_err("ansi rendering must fail"),
    ] {
        match error {
            TextRenderError::Coordinate { element_id, .. } => assert_eq!(element_id, failing_id),
            other => panic!("expected a coordinate failure, got {other:?}"),
        }
    }
}

#[test]
fn a_failed_coordinate_commits_no_partial_frame() {
    let stable = Element::text("stable").with_key("stable");
    let mut engine = LayoutEngine::new();
    engine.try_compute(&stable, 20, 4).unwrap();
    let (published, report) = engine.try_snapshot(&stable).unwrap();

    let mut output = Output::new(20, 4);
    output.write(0, 0, "caller-owned", &Style::default());
    let before_render = output.render();
    let before_dirty = output.dirty_row_indices().collect::<Vec<_>>();
    let before_is_dirty = output.is_dirty();

    let (tree, _) = tree_with_failing_grandchild(f32::NAN);
    let error = match engine.prepare_element_incremental(&tree, None, 20, 4) {
        Err(error) => error,
        Ok(_) => panic!("NaN snapshot geometry must fail before publication"),
    };
    assert!(matches!(
        error,
        TransactionalLayoutError::Snapshot(LayoutSnapshotError::NonFiniteGeometry { .. })
    ));

    let (after, after_report) = engine.try_snapshot(&stable).unwrap();
    assert_eq!(published.snapshot(), after.snapshot());
    assert_eq!(published.frame_revision(), after.frame_revision());
    assert_eq!(report, after_report);

    assert_eq!(output.render(), before_render);
    assert_eq!(output.dirty_row_indices().collect::<Vec<_>>(), before_dirty);
    assert_eq!(output.is_dirty(), before_is_dirty);
}
