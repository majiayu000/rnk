use super::*;
use crate::reconciler::{ScopedIdentityArena, plan_initial_tree_in};

#[test]
fn raw_content_bounds_reversal_is_typed_before_cell_rect_construction() {
    let identity = SnapshotIdentity::from_scoped(ScopedNodeIdentity::Root);
    let border = CellRect::checked(0, 0, 10, 5).unwrap();
    let error = checked_content_bounds(&identity, border, 8.0, 1.0, 2.0, 4.0)
        .expect_err("raw reversed x edges must fail");
    let LayoutSnapshotError::ReversedContentBounds {
        attempted_content_bounds,
        border_bounds,
        ..
    } = error
    else {
        panic!("expected raw reversed content bounds")
    };
    assert_eq!(border_bounds, border);
    assert_eq!(
        (
            attempted_content_bounds.left(),
            attempted_content_bounds.right()
        ),
        (8, 2)
    );

    let (_, before) = checked_content_bounds(&identity, border, -8.0, 1.0, -2.0, 4.0)
        .expect("ordered before-border content clips to a valid empty rect");
    let (_, after) = checked_content_bounds(&identity, border, 12.0, 1.0, 18.0, 4.0)
        .expect("ordered after-border content clips to a valid empty rect");
    assert_eq!((before.left(), before.right()), (-2, -2));
    assert_eq!((after.left(), after.right()), (10, 10));
}

#[test]
fn snapshot_width_rebind_failure_is_typed_and_preserves_published_frame() {
    let mut target = Element::text("snapshot width rebind").with_key("text");
    target.style.width = crate::core::Dimension::Points(5.75);
    target.style.position = crate::core::Position::Absolute;
    target.style.left = Some(0.75);
    let mut engine = LayoutEngine::new();
    engine.try_compute(&target, 20, 4).unwrap();
    let (published, report) = engine.try_snapshot(&target).unwrap();

    engine.text_flow_policy.set(0, "…", 1);
    let mut arena = ScopedIdentityArena::seeded(engine.vnode_map.keys());
    let element_snapshot = ElementVNodeSnapshot::from_element(&target, &mut arena).unwrap();
    let plan = plan_initial_tree_in(&element_snapshot.vnode, &mut arena).unwrap();
    let evidence = SnapshotProducerEvidence::initial(Some(0), Some(0), Some(0));
    let failure = engine
        .try_build_snapshot_for(&target, &element_snapshot, &plan, &evidence)
        .expect_err("invalid rebind policy must fail inside snapshot production");
    let (source, attempt) = failure.into_parts();
    assert!(matches!(
        source,
        LayoutSnapshotError::TextFlowRevision {
            source: crate::layout::TextFlowError::InvalidTabStop,
            ..
        }
    ));
    assert_eq!(attempt.work_counters().snapshot_nodes(), 0);
    let (after, after_report) = engine.try_snapshot(&target).unwrap();
    assert_eq!(published.snapshot(), after.snapshot());
    assert_eq!(published.frame_revision(), after.frame_revision());
    assert_eq!(report, after_report);
}
