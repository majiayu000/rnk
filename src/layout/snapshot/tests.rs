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
        .finish(SnapshotBuildStrategy::InitialFull, 0, None, 0)
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
