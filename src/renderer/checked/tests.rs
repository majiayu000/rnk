use crate::core::{Display, Element, ElementType};
use crate::layout::{
    LayoutEngine, LayoutSnapshotError, SnapshotInvariantError, SnapshotTargetMismatchReason,
};
use crate::renderer::Output;

use super::{CheckedRenderError, SnapshotRenderError, try_render_element_tree_checked};

#[test]
fn missing_root_layout_is_typed_and_commits_no_output() {
    let element = Element::text("missing");
    let engine = LayoutEngine::new();
    let mut output = Output::new(20, 4);
    let before = output.render();

    let error = try_render_element_tree_checked(&element, &engine, &mut output, 0.0, 0.0)
        .expect_err("visible root requires layout");

    assert!(matches!(error, CheckedRenderError::Snapshot(
        SnapshotRenderError::Snapshot {
            source: LayoutSnapshotError::MissingIdentity { element_id }
        }
    ) if element_id == element.id));
    assert_eq!(output.render(), before);
}

#[test]
fn missing_descendant_layout_is_typed_before_projection() {
    let mut element = Element::box_element();
    let mut engine = LayoutEngine::new();
    engine.try_compute(&element, 20, 4).expect("root layout");
    let child = Element::text("missing child");
    let child_id = child.id;
    element.add_child(child);
    let mut output = Output::new(20, 4);

    let error = try_render_element_tree_checked(&element, &engine, &mut output, 0.0, 0.0)
        .expect_err("visible child requires layout");

    assert!(matches!(
        error,
        CheckedRenderError::Snapshot(SnapshotRenderError::Snapshot {
            source: LayoutSnapshotError::InvalidTree {
                source: SnapshotInvariantError::SnapshotTargetMismatch {
                    reason: SnapshotTargetMismatchReason::ChildOrder,
                    ..
                },
                ..
            }
        })
    ));
    assert!(engine.get_layout(child_id).is_none());
    assert_eq!(output.render(), Output::new(20, 4).render());
}

#[test]
fn invalid_mapped_layout_is_not_degraded_to_missing() {
    let element = Element::text("invalid backend");
    let mut engine = LayoutEngine::new();
    engine.try_compute(&element, 20, 4).expect("valid layout");
    engine.inject_test_required_layout_fault(element.id);
    let mut output = Output::new(20, 4);

    let error = try_render_element_tree_checked(&element, &engine, &mut output, 0.0, 0.0)
        .expect_err("invalid mapped node is a checked invariant failure");

    assert!(matches!(error, CheckedRenderError::Snapshot(
        SnapshotRenderError::Snapshot {
            source: LayoutSnapshotError::MissingIdentity { element_id }
        }
    ) if element_id == element.id));
    assert_eq!(output.render(), Output::new(20, 4).render());
}

#[test]
fn virtual_text_and_hidden_subtrees_are_filtered_before_lookup() {
    let mut virtual_text = Element::new(ElementType::VirtualText);
    virtual_text.text_content = Some("virtual".into());
    let mut hidden = Element::text("hidden");
    hidden.style.display = Display::None;
    let mut root = Element::box_element();
    let mut engine = LayoutEngine::new();
    engine.try_compute(&root, 20, 4).expect("root-only layout");
    root.add_child(virtual_text);
    root.add_child(hidden);
    let mut output = Output::new(20, 4);

    try_render_element_tree_checked(&root, &engine, &mut output, 0.0, 0.0)
        .expect("filtered children need no layout");
}
