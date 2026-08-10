//! GH-61 closed snapshot, alias, output, and transaction error ledger.

use std::error::Error;

use rnk::core::{Dimension, Element, Position, Props, VNode};
use rnk::layout::{
    ArithmeticOperation, Axis, CellOutputError, Edge, GeometryField, LayoutAliasError,
    LayoutEngine, LayoutSnapshotError, SnapshotInvariantError, SnapshotTargetMismatchReason,
    TransactionalLayoutError,
};
use rnk::renderer::{
    CheckedRenderError, SnapshotRenderError, TransactionalFrameError, try_render_to_string_checked,
};

fn valid_frame() -> (Element, rnk::layout::PreparedLayoutFrame) {
    let element = Element::text("identity").with_key("identity");
    let prepared = LayoutEngine::new()
        .prepare_element_incremental(&element, None, 20, 4)
        .unwrap();
    (element, prepared)
}

fn non_finite_target() -> Element {
    let mut root = Element::root();
    let mut child = Element::text("invalid").with_key("message");
    child.style.padding.left = f32::NAN;
    root.add_child(child);
    root
}

#[test]
fn negative_and_overflow_cells_are_not_clamped_to_success() {
    let mut negative = Element::box_element().with_key("negative");
    negative.style.position = Position::Absolute;
    negative.style.left = Some(-0.5);
    negative.style.width = Dimension::Points(1.0);
    let mut root = Element::root();
    root.add_child(negative);
    let prepared = LayoutEngine::new()
        .prepare_element_incremental(&root, None, 20, 4)
        .unwrap();
    assert_eq!(
        prepared
            .snapshot()
            .nodes()
            .nth(1)
            .unwrap()
            .border_bounds()
            .left(),
        -1
    );

    let mut overflow = Element::box_element();
    overflow.style.width = Dimension::Points(f32::MAX);
    assert!(matches!(
        LayoutEngine::new().prepare_element_incremental(&overflow, None, u16::MAX, 4),
        Err(TransactionalLayoutError::Snapshot(
            LayoutSnapshotError::CellCoordinateOverflow { .. }
                | LayoutSnapshotError::EdgeArithmeticOverflow { .. }
        ))
    ));
}

#[test]
fn initial_snapshot_failure_never_enters_incremental_recovery() {
    let engine = LayoutEngine::new();
    let error = match engine.prepare_element_incremental(&non_finite_target(), None, 20, 4) {
        Err(error) => error,
        Ok(_) => panic!("snapshot unexpectedly succeeded"),
    };
    assert!(matches!(
        error,
        TransactionalLayoutError::Snapshot(LayoutSnapshotError::NonFiniteGeometry { .. })
    ));
    assert!(!engine.has_tree());
}

#[test]
fn recovered_snapshot_or_render_failure_preserves_both_causes() {
    let target = non_finite_target();
    let mut props = Props::new().key("message");
    props.style.padding.left = f32::NAN;
    let previous =
        VNode::root().child(VNode::text("invalid").with_key("message").with_props(props));
    let mut engine = LayoutEngine::new();
    engine.build_vnode_tree(&previous).unwrap();
    let error = match engine.prepare_element_incremental(&target, Some(&previous), 0, 0) {
        Err(error) => error,
        Ok(_) => panic!("recovered candidate snapshot unexpectedly succeeded"),
    };
    let TransactionalLayoutError::RecoveredSnapshot(source) = error else {
        panic!("expected recovered snapshot aggregation, got {error}");
    };
    assert!(matches!(
        source.snapshot_failure(),
        LayoutSnapshotError::NonFiniteGeometry { .. }
    ));
    assert!(source.incremental_failure().patch_index.is_none());
    assert!(source.source().is_some());
}

#[test]
fn snapshot_failure_publishes_nothing() {
    let stable = Element::text("stable").with_key("stable");
    let mut engine = LayoutEngine::new();
    let initial = engine
        .prepare_element_incremental(&stable, None, 20, 4)
        .unwrap();
    let (previous, _) = initial.commit(&mut engine);
    let before = engine.get_all_layouts();

    let invalid = non_finite_target();
    assert!(
        engine
            .prepare_element_incremental(&invalid, Some(&previous), 20, 4)
            .is_err()
    );
    assert_eq!(engine.get_all_layouts().len(), before.len());
    assert!(engine.get_layout(stable.id).is_some());
    assert!(engine.get_layout(invalid.id).is_none());
}

#[test]
fn every_snapshot_failure_variant_preserves_payload_and_source_chain() {
    let (element, prepared) = valid_frame();
    let identity = prepared.snapshot().root().identity().clone();
    let rect = prepared.snapshot().root().border_bounds();
    let invariant = SnapshotInvariantError::SnapshotTargetMismatch {
        identity: identity.clone(),
        reason: SnapshotTargetMismatchReason::ChildOrder,
    };
    let variants = [
        LayoutSnapshotError::NonFiniteGeometry {
            identity: identity.clone(),
            field: GeometryField::X,
            value_bits: f32::NAN.to_bits(),
        },
        LayoutSnapshotError::NegativeExtent {
            identity: identity.clone(),
            axis: Axis::X,
            value_bits: (-1.0_f32).to_bits(),
        },
        LayoutSnapshotError::EdgeArithmeticOverflow {
            identity: identity.clone(),
            operation: ArithmeticOperation::Add,
            lhs_bits: f64::MAX.to_bits(),
            rhs_bits: f64::MAX.to_bits(),
        },
        LayoutSnapshotError::CellCoordinateOverflow {
            identity: identity.clone(),
            edge: Edge::Right,
            rounded_bits: f64::MAX.to_bits(),
        },
        LayoutSnapshotError::ReversedContentBounds {
            identity: identity.clone(),
            border_bounds: rect,
            attempted_content_bounds: rect,
        },
        LayoutSnapshotError::MissingIdentity {
            element_id: element.id,
        },
        LayoutSnapshotError::DuplicateIdentity {
            identity: identity.clone(),
        },
        LayoutSnapshotError::MissingLayout {
            identity: identity.clone(),
        },
        LayoutSnapshotError::MissingTextFlowRevision {
            identity: identity.clone(),
        },
        LayoutSnapshotError::TextFlowRevision {
            identity: identity.clone(),
            source: rnk::layout::TextFlowError::InvalidTabStop,
        },
        LayoutSnapshotError::InvalidTree {
            identity: Some(identity.clone()),
            source: invariant.clone(),
        },
    ];
    for (index, error) in variants.into_iter().enumerate() {
        assert!(!error.to_string().is_empty(), "variant {index}");
        if matches!(
            error,
            LayoutSnapshotError::InvalidTree { .. } | LayoutSnapshotError::TextFlowRevision { .. }
        ) {
            assert!(error.source().is_some());
        } else {
            assert!(error.source().is_none());
        }
    }

    let output = SnapshotRenderError::Output {
        identity,
        source: CellOutputError::CoordinateOutOfRange {
            axis: Axis::Y,
            value: i32::MAX,
        },
    };
    assert!(output.source().is_some());
}

#[test]
fn every_layout_alias_variant_preserves_payload_and_source() {
    let (element, first) = valid_frame();
    let (_, second) = valid_frame();
    let identity = first.snapshot().root().identity().clone();
    let variants = [
        LayoutAliasError::MissingFrameAlias {
            element_id: element.id,
            frame_revision: first.prepared_snapshot().frame_revision(),
        },
        LayoutAliasError::DuplicateFrameAlias {
            element_id: element.id,
            first_identity: identity.clone(),
            second_identity: identity.clone(),
        },
        LayoutAliasError::AliasTargetMissing {
            element_id: element.id,
            identity: identity.clone(),
        },
        LayoutAliasError::StaleFrameAlias {
            element_id: element.id,
            expected_frame_revision: first.prepared_snapshot().frame_revision(),
            actual_frame_revision: second.prepared_snapshot().frame_revision(),
        },
        LayoutAliasError::AliasIdentityMismatch {
            element_id: element.id,
            expected_identity: identity.clone(),
            actual_identity: identity,
        },
    ];
    for error in variants {
        assert!(!error.to_string().is_empty());
        assert!(error.source().is_none());
    }
}

#[test]
fn gh60_frame_wrapper_routes_snapshot_failures_without_fictitious_initial_variant() {
    let checked = try_render_to_string_checked(&non_finite_target(), 20)
        .expect_err("checked string must preserve snapshot build failure");
    assert!(matches!(
        checked,
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::Snapshot(_))
    ));
    let frame = match checked {
        CheckedRenderError::LayoutBuild(source) => TransactionalFrameError::Transaction(source),
        other => TransactionalFrameError::Render(other),
    };
    assert!(matches!(
        frame,
        TransactionalFrameError::Transaction(TransactionalLayoutError::Snapshot(_))
    ));
    assert!(frame.source().is_some());
}
