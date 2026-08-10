//! GH-61 closed snapshot, alias, output, and transaction error ledger.

use std::error::Error;

use rnk::core::{Dimension, Element, Position, Props, VNode};
use rnk::layout::{
    ArithmeticOperation, Axis, CellOutputError, Edge, GeometryField, IncrementalInvariantError,
    LayoutAliasError, LayoutEngine, LayoutSnapshotError, SnapshotCounterError,
    SnapshotInvariantError, SnapshotTargetMismatchReason, SnapshotWorkCounterField,
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
    let overflow =
        match LayoutEngine::new().prepare_element_incremental(&overflow, None, u16::MAX, 4) {
            Err(error) => error,
            Ok(_) => panic!("unrepresentable cell edge must fail"),
        };
    assert!(
        matches!(overflow, TransactionalLayoutError::Snapshot(source)
        if matches!(source,
            LayoutSnapshotError::CellCoordinateOverflow { .. }
                | LayoutSnapshotError::EdgeArithmeticOverflow { .. }))
    );
}

#[test]
fn initial_snapshot_failure_never_enters_incremental_recovery() {
    let engine = LayoutEngine::new();
    let error = match engine.prepare_element_incremental(&non_finite_target(), None, 20, 4) {
        Err(error) => error,
        Ok(_) => panic!("snapshot unexpectedly succeeded"),
    };
    let TransactionalLayoutError::Snapshot(source) = error else {
        panic!("expected source-compatible snapshot route")
    };
    assert!(matches!(
        source,
        LayoutSnapshotError::NonFiniteGeometry { .. }
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
    let work = source.snapshot_attempt_report().work_counters();
    assert_eq!(source.snapshot_attempt_report().operation_count(), 1);
    assert_eq!(work.snapshot_nodes(), 0);
    assert_eq!(work.rebuild_count(), 1);
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
    let (before_snapshot, before_report) = engine.try_snapshot(&stable).unwrap();

    let invalid = non_finite_target();
    assert!(
        engine
            .prepare_element_incremental(&invalid, Some(&previous), 20, 4)
            .is_err()
    );
    assert_eq!(engine.get_all_layouts().len(), before.len());
    assert!(engine.get_layout(stable.id).is_some());
    assert!(engine.get_layout(invalid.id).is_none());
    let (after_snapshot, after_report) = engine.try_snapshot(&stable).unwrap();
    assert_eq!(before_snapshot.snapshot(), after_snapshot.snapshot());
    assert_eq!(
        before_snapshot.frame_revision(),
        after_snapshot.frame_revision()
    );
    assert_eq!(before_report, after_report);
}

#[test]
fn every_snapshot_failure_variant_preserves_payload_and_source_chain() {
    let (element, prepared) = valid_frame();
    let identity = prepared.snapshot().root().identity().clone();
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
        LayoutSnapshotError::CellSpanOverflow {
            identity: identity.clone(),
            axis: Axis::X,
            start: i32::MIN,
            end: i32::MAX,
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
        LayoutSnapshotError::LayoutLookup {
            identity: identity.clone(),
            source: IncrementalInvariantError::InvalidMappedNode,
        },
        LayoutSnapshotError::MissingTextFlowRevision {
            identity: identity.clone(),
        },
        LayoutSnapshotError::TextFlowRevision {
            identity: identity.clone(),
            source: rnk::layout::TextFlowError::InvalidTabStop,
        },
        LayoutSnapshotError::WorkCounters {
            source: SnapshotCounterError::Overflow {
                field: SnapshotWorkCounterField::VisitedNodes,
                lhs: u64::MAX,
                rhs: 1,
            },
        },
        LayoutSnapshotError::CacheEvidenceOverflow,
        LayoutSnapshotError::Alias {
            source: LayoutAliasError::AliasTargetMissing {
                element_id: element.id,
                identity: identity.clone(),
            },
        },
        LayoutSnapshotError::InvalidTree {
            identity: Some(identity.clone()),
            source: invariant.clone(),
        },
    ];
    for (index, error) in variants.into_iter().enumerate() {
        assert!(!error.to_string().is_empty(), "variant {index}");
        let source_expected = match &error {
            LayoutSnapshotError::TextFlowRevision { .. }
            | LayoutSnapshotError::WorkCounters { .. }
            | LayoutSnapshotError::LayoutLookup { .. }
            | LayoutSnapshotError::Alias { .. }
            | LayoutSnapshotError::InvalidTree { .. } => true,
            LayoutSnapshotError::NonFiniteGeometry { .. }
            | LayoutSnapshotError::NegativeExtent { .. }
            | LayoutSnapshotError::EdgeArithmeticOverflow { .. }
            | LayoutSnapshotError::CellCoordinateOverflow { .. }
            | LayoutSnapshotError::CellSpanOverflow { .. }
            | LayoutSnapshotError::ReversedContentBounds { .. }
            | LayoutSnapshotError::MissingIdentity { .. }
            | LayoutSnapshotError::DuplicateIdentity { .. }
            | LayoutSnapshotError::MissingLayout { .. }
            | LayoutSnapshotError::MissingTextFlowRevision { .. }
            | LayoutSnapshotError::CacheEvidenceOverflow => false,
        };
        if source_expected {
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
