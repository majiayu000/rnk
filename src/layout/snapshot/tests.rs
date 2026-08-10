use std::hash::Hasher;

use super::*;
use crate::components::{Box as RnkBox, Text};
use crate::core::{Dimension, FlexDirection, Overflow};
use crate::layout::LayoutEngine;
use crate::reconciler::ScopedNodeIdentity;

#[derive(Default)]
struct RecordingHasher(Vec<u8>);

impl Hasher for RecordingHasher {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

fn target() -> crate::core::Element {
    RnkBox::new()
        .width(10)
        .flex_direction(FlexDirection::Column)
        .child(Text::new("first").into_element().with_key("first"))
        .child(Text::new("second").into_element().with_key("second"))
        .into_element()
        .with_key("root")
}

#[test]
fn semantic_identity_and_final_order() {
    let target = target();
    let engine = LayoutEngine::new();
    let frame = engine
        .prepare_element_incremental(&target, None, 10, 4)
        .unwrap();
    let snapshot = frame.snapshot();
    let children = snapshot.root().children();
    assert_eq!(children.len(), 2);
    let first = snapshot.nodes().nth(children[0].as_usize()).unwrap();
    let second = snapshot.nodes().nth(children[1].as_usize()).unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_eq!(first.parent(), Some(SnapshotNodeIndex::checked(0)));
    assert_eq!(second.parent(), Some(SnapshotNodeIndex::checked(0)));

    let hostile = "secret/path/99\u{1b}[31m\u{7f}\u{85}";
    let hostile_target = RnkBox::new()
        .child(Text::new("hostile").into_element().with_key(hostile))
        .into_element();
    let hostile_frame = LayoutEngine::new()
        .prepare_element_incremental(&hostile_target, None, 10, 2)
        .unwrap();
    let hostile_index = hostile_frame.snapshot().root().children()[0];
    let identity = hostile_frame
        .snapshot()
        .nodes()
        .nth(hostile_index.as_usize())
        .unwrap()
        .identity()
        .clone();
    for rendered in [
        format!("{identity:?}"),
        format!("{identity}"),
        identity.diagnostic(),
    ] {
        assert!(!rendered.contains(hostile));
        assert!(
            !rendered
                .chars()
                .any(|character| character == '\u{1b}' || character.is_control())
        );
        assert!(rendered.len() <= 32);
    }
    let mut first_hash = RecordingHasher::default();
    identity.hash(&mut first_hash);
    let mut second_hash = RecordingHasher::default();
    identity.clone().hash(&mut second_hash);
    assert_eq!(first_hash.0, second_hash.0);
    assert!(
        !first_hash
            .0
            .windows(hostile.len())
            .any(|window| window == hostile.as_bytes())
    );
}

#[test]
fn mixed_axis_overflow_clips_only_selected_axis() {
    let mut target = target();
    target.style.width = Dimension::Points(6.0);
    target.style.height = Dimension::Points(2.0);
    target.style.overflow_x = Overflow::Hidden;
    target.style.overflow_y = Overflow::Visible;
    let engine = LayoutEngine::new();
    let frame = engine
        .prepare_element_incremental(&target, None, 20, 8)
        .unwrap();
    assert_eq!(frame.snapshot().root().effective_clip().x().end(), 6);
    assert_eq!(frame.snapshot().root().effective_clip().y().end(), 8);
}

#[test]
fn producer_report_does_not_change_semantic_equality() {
    let first_target = target();
    let mut engine = LayoutEngine::new();
    let first = engine
        .prepare_element_incremental(&first_target, None, 10, 4)
        .unwrap();
    let first_snapshot = first.snapshot().clone();
    let (previous, _) = first.commit(&mut engine);
    let second_target = target();
    let second = engine
        .prepare_element_incremental(&second_target, Some(&previous), 10, 4)
        .unwrap();
    assert_eq!(&first_snapshot, second.snapshot());
    assert_ne!(
        SnapshotBuildStrategy::InitialFull,
        second.snapshot_report().strategy()
    );
}

#[test]
fn cancelled_builder_is_hidden_and_published_snapshot_is_immutable() {
    let target = target();
    let mut engine = LayoutEngine::new();
    let initial = engine
        .prepare_element_incremental(&target, None, 10, 4)
        .unwrap();
    let (previous, _) = initial.commit(&mut engine);
    let (published, report) = engine.try_snapshot(&target).unwrap();
    let next_target = RnkBox::new()
        .child(
            Text::new("replacement")
                .into_element()
                .with_key("replacement"),
        )
        .into_element()
        .with_key("root");
    let cancelled = engine
        .prepare_element_incremental(&next_target, Some(&previous), 10, 4)
        .unwrap();
    drop(cancelled);
    let (after_drop, after_report) = engine.try_snapshot(&target).unwrap();
    assert_eq!(published.snapshot(), after_drop.snapshot());
    assert_eq!(published.frame_revision(), after_drop.frame_revision());
    assert_eq!(report, after_report);
    assert!(engine.try_snapshot(&next_target).is_err());

    let mut poisoned = LayoutSnapshotBuilder::new(4, 2, 1);
    poisoned
        .add_work(SnapshotWorkCounters::from_fields(u64::MAX, 0, 0, 0, 0))
        .unwrap();
    let first = poisoned
        .add_work(SnapshotWorkCounters::from_fields(1, 0, 0, 0, 0))
        .expect_err("counter overflow permanently poisons the builder");
    let identity = SnapshotIdentity::from_scoped(ScopedNodeIdentity::Root);
    let rect = CellRect::viewport(4, 2);
    let continued = poisoned
        .push_ordered(CheckedSnapshotNodeInput {
            element_id: target.id,
            identity,
            parent: None,
            border_bounds: rect,
            content_bounds: rect,
            text_origin: rect.origin(),
            effective_clip: AxisClip::from_rect(rect),
            scroll_transform: CellVector::checked(0, 0),
            text_flow: None,
        })
        .expect_err("ignored first error cannot resume construction");
    assert_eq!(first.into_parts(), continued.into_parts());
    let terminal = poisoned
        .finish(SnapshotBuildStrategy::InitialFull, 0, None)
        .expect_err("poisoned builder cannot publish");
    assert!(matches!(
        terminal.into_parts().0,
        LayoutSnapshotError::WorkCounters {
            source: SnapshotCounterError::Overflow {
                field: SnapshotWorkCounterField::VisitedNodes,
                lhs: u64::MAX,
                rhs: 1,
            }
        }
    ));
}

#[test]
fn layout_alias_variants_are_reached_through_checked_production_seams() {
    let target = target();
    let frame = LayoutEngine::new()
        .prepare_element_incremental(&target, None, 10, 4)
        .unwrap();
    let prepared = frame.prepared_snapshot();
    let snapshot = prepared.snapshot();
    let root = snapshot.root();
    let first = snapshot.node(root.children()[0]);
    let second = snapshot.node(root.children()[1]);
    let first_element_id = target.children.iter().next().unwrap().id;

    let missing_id = crate::core::Element::text("not in frame").id;
    assert!(matches!(
        prepared.node_for_element(missing_id),
        Err(LayoutAliasError::MissingFrameAlias { element_id, .. }) if element_id == missing_id
    ));
    assert!(matches!(
        prepared.resolve_exact_alias(first_element_id, second.identity()),
        Err(LayoutAliasError::AliasIdentityMismatch {
            element_id,
            expected_identity,
            actual_identity,
        }) if element_id == first_element_id
            && expected_identity == *second.identity()
            && actual_identity == *first.identity()
    ));

    let other = RnkBox::new()
        .child(Text::new("other").into_element().with_key("other"))
        .into_element();
    let other_frame = LayoutEngine::new()
        .prepare_element_incremental(&other, None, 10, 4)
        .unwrap();
    let absent = other_frame
        .snapshot()
        .node(other_frame.snapshot().root().children()[0])
        .identity()
        .clone();
    assert!(matches!(
        prepared.resolve_exact_alias(first_element_id, &absent),
        Err(LayoutAliasError::AliasTargetMissing { element_id, identity })
            if element_id == first_element_id && identity == absent
    ));

    let mut builder = LayoutSnapshotBuilder::new(10, 4, 1);
    let root_index = builder
        .push_ordered(CheckedSnapshotNodeInput {
            element_id: target.id,
            identity: root.identity().clone(),
            parent: None,
            border_bounds: root.border_bounds(),
            content_bounds: root.content_bounds(),
            text_origin: root.text_origin(),
            effective_clip: root.effective_clip(),
            scroll_transform: root.scroll_transform(),
            text_flow: root.text_flow().cloned(),
        })
        .unwrap();
    let duplicate = builder
        .push_ordered(CheckedSnapshotNodeInput {
            element_id: target.id,
            identity: first.identity().clone(),
            parent: Some(root_index),
            border_bounds: first.border_bounds(),
            content_bounds: first.content_bounds(),
            text_origin: first.text_origin(),
            effective_clip: first.effective_clip(),
            scroll_transform: first.scroll_transform(),
            text_flow: first.text_flow().cloned(),
        })
        .expect_err("duplicate frame alias must poison the real checked builder");
    assert!(matches!(
        duplicate.source_error(),
        LayoutSnapshotError::Alias {
            source: LayoutAliasError::DuplicateFrameAlias { element_id, .. },
        } if *element_id == target.id
    ));
}
