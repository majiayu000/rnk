use rnk::components::{Box as RnkBox, Text};
use rnk::core::{Dimension, Display, Element, FlexDirection, Overflow, Props, VNode};
use rnk::layout::{
    CheckedIncrementalLayoutReport, LayoutEngine, LayoutSnapshot, SnapshotBuildStrategy,
};
use rnk::renderer::try_render_to_string_checked;
use rnk::testing::TestRenderer;

fn chat_target(messages: &[(&str, &str)], width: u16) -> Element {
    let mut root = RnkBox::new()
        .width(width)
        .flex_direction(FlexDirection::Column)
        .into_element()
        .with_key("root");
    for (key, text) in messages {
        root.add_child(Text::new(*text).into_element().with_key(*key));
    }
    root
}

fn full_snapshot(target: &Element, width: u16, height: u16) -> LayoutSnapshot {
    let engine = LayoutEngine::new();
    engine
        .prepare_element_incremental(target, None, width, height)
        .expect("full snapshot")
        .snapshot()
        .clone()
}

#[test]
fn full_incremental_and_recovered_are_semantically_equal() {
    let (target, incomplete_previous) = recovered_target();
    let full = LayoutEngine::new()
        .prepare_element_incremental(&target, None, 0, 0)
        .unwrap();
    assert_eq!(
        full.snapshot_report().strategy(),
        SnapshotBuildStrategy::InitialFull
    );

    let mut incremental = LayoutEngine::new();
    let first = incremental
        .prepare_element_incremental(&target, None, 0, 0)
        .unwrap();
    let (previous, _) = first.commit(&mut incremental);
    let incremental_target = recovered_target().0;
    let incremental_frame = incremental
        .prepare_element_incremental(&incremental_target, Some(&previous), 0, 0)
        .unwrap();
    assert_eq!(
        incremental_frame.snapshot_report().strategy(),
        SnapshotBuildStrategy::Incremental
    );

    let mut recovered = LayoutEngine::new();
    recovered.build_vnode_tree(&incomplete_previous).unwrap();
    let recovered_frame = recovered
        .prepare_element_incremental(&target, Some(&incomplete_previous), 0, 0)
        .unwrap();
    assert!(matches!(
        recovered_frame.report(),
        CheckedIncrementalLayoutReport::RecoveredFullRebuild { .. }
    ));
    assert_eq!(
        recovered_frame.snapshot_report().strategy(),
        SnapshotBuildStrategy::RecoveredFull
    );
    assert_eq!(full.snapshot(), incremental_frame.snapshot());
    assert_eq!(full.snapshot(), recovered_frame.snapshot());
    let full_cells: Vec<_> = full
        .snapshot()
        .nodes()
        .map(|node| {
            (
                node.identity().clone(),
                node.border_bounds(),
                node.content_bounds(),
                node.effective_clip(),
                node.scroll_transform(),
            )
        })
        .collect();
    let incremental_cells: Vec<_> = incremental_frame
        .snapshot()
        .nodes()
        .map(|node| {
            (
                node.identity().clone(),
                node.border_bounds(),
                node.content_bounds(),
                node.effective_clip(),
                node.scroll_transform(),
            )
        })
        .collect();
    let recovered_cells: Vec<_> = recovered_frame
        .snapshot()
        .nodes()
        .map(|node| {
            (
                node.identity().clone(),
                node.border_bounds(),
                node.content_bounds(),
                node.effective_clip(),
                node.scroll_transform(),
            )
        })
        .collect();
    assert_eq!(full_cells, incremental_cells);
    assert_eq!(full_cells, recovered_cells);
}

#[test]
fn chat_mutation_matrix_matches_full() {
    let frames = [
        vec![("a", "A")],
        vec![("a", "A🙂"), ("b", "B")],
        vec![("front", "前"), ("a", "A🙂"), ("b", "B")],
        vec![("b", "B changed"), ("front", "前"), ("a", "A🙂")],
        vec![("b", "B changed"), ("replacement", "e\u{301}")],
    ];
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    for frame in frames {
        let target = chat_target(&frame, 10);
        let prepared = engine
            .prepare_element_incremental(&target, previous.as_ref(), 16, 6)
            .unwrap();
        assert_eq!(prepared.snapshot(), &full_snapshot(&target, 16, 6));
        let (next, _) = prepared.commit(&mut engine);
        previous = Some(next);
    }

    let build_large = |entries: &[(Option<String>, String)]| {
        let mut root = Element::box_element().with_key("large-root");
        root.style.flex_direction = FlexDirection::Column;
        root.style.width = Dimension::Points(40.0);
        for (key, text) in entries {
            let mut child = Element::text(text.clone());
            if let Some(key) = key {
                child = child.with_key(key.clone());
            }
            root.add_child(child);
        }
        root
    };
    let mut entries: Vec<_> = (0..1_000)
        .map(|index| {
            let key = (index % 2 == 0).then(|| format!("message-{index}"));
            let payload = match index % 4 {
                0 => format!("ascii {index}"),
                1 => format!("中\nvariable row {index}"),
                2 => format!("👩‍💻 {index}"),
                _ => format!("e\u{301} {index}"),
            };
            (key, payload)
        })
        .collect();
    let mut large_engine = LayoutEngine::new();
    let mut large_previous = None;
    for operation in 0..6 {
        match operation {
            1 => entries.insert(0, (Some("front".to_owned()), "前🙂".to_owned())),
            2 => entries.insert(entries.len() / 2, (None, "middle 👩‍💻\ne\u{301}".to_owned())),
            3 => entries.push((Some("tail".to_owned()), "尾 中".to_owned())),
            4 => entries[501].1.push_str(" streamed🙂中e\u{301}"),
            5 => {
                entries.remove(1);
                let moved = entries.remove(entries.len() / 2);
                entries.insert(2, moved);
            }
            _ => {}
        }
        let target = build_large(&entries);
        let prepared = large_engine
            .prepare_element_incremental(&target, large_previous.as_ref(), 40, 2_000)
            .unwrap_or_else(|error| panic!("large mutation operation {operation}: {error}"));
        assert_eq!(
            prepared.snapshot(),
            &full_snapshot(&target, 40, 2_000),
            "large mutation operation {operation}"
        );
        let (next, _) = prepared.commit(&mut large_engine);
        large_previous = Some(next);
    }
}

#[test]
fn resize_round_trip_restores_semantic_snapshot() {
    let target = chat_target(&[("a", "one two three four five 世界🙂")], 30);
    let wide = full_snapshot(&target, 30, 10);
    let narrow = full_snapshot(&target, 8, 4);
    let wide_again = full_snapshot(&target, 30, 10);
    assert_ne!(wide, narrow);
    assert_eq!(wide, wide_again);
}

#[test]
fn cold_and_cached_text_flow_revisions_are_semantically_equal() {
    let target = chat_target(&[("text", "cache identity 世界")], 14);
    let mut cached = LayoutEngine::new();
    let first = cached
        .prepare_element_incremental(&target, None, 14, 4)
        .unwrap();
    let (previous, _) = first.commit(&mut cached);
    let fresh_aliases = chat_target(&[("text", "cache identity 世界")], 14);
    let cached_frame = cached
        .prepare_element_incremental(&fresh_aliases, Some(&previous), 14, 4)
        .unwrap();
    assert_eq!(
        cached_frame.snapshot(),
        &full_snapshot(&fresh_aliases, 14, 4)
    );

    let mut fractional = Element::text("fractional width rebind").with_key("fractional");
    fractional.style.position = rnk::core::Position::Absolute;
    fractional.style.left = Some(0.75);
    fractional.style.width = Dimension::Points(5.75);
    let fractional_cold = LayoutEngine::new()
        .prepare_element_incremental(&fractional, None, 20, 4)
        .unwrap();
    let node = fractional_cold.snapshot().root();
    assert_eq!(
        node.text_flow().unwrap().max_width(),
        node.content_bounds().width() as usize
    );
    assert!(fractional_cold.snapshot_report().cache_hits() > 0);
}

#[test]
fn display_none_prunes_only_snapshot_render_traversal() {
    let mut hidden = chat_target(&[("hidden-child", "not rendered")], 10);
    hidden.style.display = Display::None;
    let mut root = chat_target(&[("visible", "shown")], 10);
    root.add_child(hidden);
    let snapshot = full_snapshot(&root, 10, 4);
    assert_eq!(snapshot.nodes().len(), 2);
    assert_eq!(try_render_to_string_checked(&root, 10).unwrap(), "shown");
}

#[test]
fn nested_shared_edges_do_not_gain_overlap() {
    let mut root = Element::box_element().with_key("root");
    root.style.width = Dimension::Points(10.0);
    for key in ["left", "middle", "right"] {
        let mut child = Element::box_element().with_key(key);
        child.style.width = Dimension::Percent(100.0 / 3.0);
        root.add_child(child);
    }
    let snapshot = full_snapshot(&root, 10, 2);
    let children = snapshot.root().children();
    let nodes: Vec<_> = children
        .iter()
        .map(|index| snapshot.nodes().nth(index.as_usize()).unwrap())
        .collect();
    assert!(nodes[0].border_bounds().right() <= nodes[1].border_bounds().left());
    assert!(nodes[1].border_bounds().right() <= nodes[2].border_bounds().left());
}

#[test]
fn mixed_axis_overflow_clips_only_selected_axis() {
    let mut root = chat_target(&[("child", "abcdef")], 6);
    root.style.height = Dimension::Points(2.0);
    root.style.overflow_x = Overflow::Hidden;
    root.style.overflow_y = Overflow::Visible;
    let snapshot = full_snapshot(&root, 20, 8);
    let root_node = snapshot.root();
    assert_eq!(root_node.effective_clip().x().end(), 6);
    assert_eq!(root_node.effective_clip().y().end(), 8);
}

#[test]
fn string_snapshot_cell_contract_is_supplemental() {
    let target = chat_target(&[("text", "共享🙂")], 8);
    let rendered = try_render_to_string_checked(&target, 8).unwrap();
    let snapshot = full_snapshot(&target, 8, 4);
    assert_eq!(rendered, "共享🙂");
    assert_eq!(snapshot.root().border_bounds().width(), 8);
}

#[test]
fn snapshot_target_adapter_uses_gh59_order_and_gh60_lookup_contract() {
    let target = chat_target(&[("c", "C"), ("a", "A"), ("b", "B")], 12);
    let snapshot = full_snapshot(&target, 12, 4);
    let children = snapshot.root().children();
    assert_eq!(children.len(), 3);
    assert_eq!(
        children
            .iter()
            .map(|index| index.as_usize())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    for index in children {
        let node = snapshot.nodes().nth(index.as_usize()).unwrap();
        assert_eq!(node.parent().map(|parent| parent.as_usize()), Some(0));
        assert_eq!(snapshot.get(node.identity()), Some(node));
    }
    assert_eq!(
        try_render_to_string_checked(&target, 12).unwrap(),
        "C\nA\nB"
    );
}

#[test]
fn nested_mixed_axis_overflow_matches_all_strategies() {
    let mut inner = chat_target(&[("text", "abcdefgh\n第二行")], 9);
    inner.style.height = Dimension::Points(2.0);
    inner.style.overflow_x = Overflow::Visible;
    inner.style.overflow_y = Overflow::Hidden;
    let mut target = RnkBox::new().child(inner).into_element().with_key("outer");
    target.style.width = Dimension::Points(6.0);
    target.style.height = Dimension::Points(4.0);
    target.style.overflow_x = Overflow::Hidden;
    target.style.overflow_y = Overflow::Visible;

    let full = full_snapshot(&target, 20, 8);
    let mut engine = LayoutEngine::new();
    let initial = engine
        .prepare_element_incremental(&target, None, 20, 8)
        .unwrap();
    let (previous, _) = initial.commit(&mut engine);
    let fresh_aliases = target.clone();
    let incremental = engine
        .prepare_element_incremental(&fresh_aliases, Some(&previous), 20, 8)
        .unwrap();
    assert_eq!(incremental.snapshot(), &full);
    let child_index = full.root().children()[0];
    let child = full.nodes().nth(child_index.as_usize()).unwrap();
    assert_eq!(child.effective_clip().x().end(), 6);
    assert_eq!(child.effective_clip().y().end(), 2);
}

fn recovered_target() -> (Element, VNode) {
    let mut target = Element::root();
    target.add_child(Element::text("recovered").with_key("message"));
    let previous = VNode::root().child(
        VNode::text("recovered")
            .with_key("message")
            .with_props(Props::new().key("message")),
    );
    (target, previous)
}

#[test]
fn recovered_frame_uses_only_recovered_candidate_snapshot() {
    let (target, previous) = recovered_target();
    let mut engine = LayoutEngine::new();
    engine.build_vnode_tree(&previous).unwrap();
    let prepared = engine
        .prepare_element_incremental(&target, Some(&previous), 0, 0)
        .unwrap();
    assert!(matches!(
        prepared.report(),
        CheckedIncrementalLayoutReport::RecoveredFullRebuild { .. }
    ));
    assert_eq!(
        prepared.snapshot_report().strategy(),
        SnapshotBuildStrategy::RecoveredFull
    );
    assert_eq!(
        prepared.snapshot_report().work_counters().rebuild_count(),
        1
    );
    assert_eq!(
        prepared.snapshot_report().work_counters().mutated_nodes(),
        2
    );
    assert!(prepared.snapshot_report().cache_hits() > 0);
    assert_eq!(prepared.snapshot(), &full_snapshot(&target, 0, 0));
}

#[test]
fn reused_snapshot_accepts_target_exact_frame_aliases() {
    let first_target = chat_target(&[("a", "same")], 10);
    let mut engine = LayoutEngine::new();
    let first = engine
        .prepare_element_incremental(&first_target, None, 10, 4)
        .unwrap();
    let first_revision = first.prepared_snapshot().frame_revision();
    let first_snapshot = first.snapshot().clone();
    let (previous, _) = first.commit(&mut engine);

    let second_target = chat_target(&[("a", "same")], 10);
    let second = engine
        .prepare_element_incremental(&second_target, Some(&previous), 10, 4)
        .unwrap();
    assert_eq!(second.snapshot(), &first_snapshot);
    assert_ne!(second.prepared_snapshot().frame_revision(), first_revision);
    assert_eq!(
        try_render_to_string_checked(&second_target, 10).unwrap(),
        "same"
    );
}

#[test]
fn string_and_testing_output_parity_is_supplemental() {
    let target = chat_target(&[("text", "consumer parity")], 20);
    let checked = try_render_to_string_checked(&target, 20).unwrap();
    let testing = TestRenderer::new(20, 4)
        .try_render_to_plain_checked(&target)
        .unwrap();
    assert_eq!(checked, testing);
    assert_eq!(checked, "consumer parity");
}

#[test]
fn scroll_changes_descendant_projection_only() {
    let child = Text::new("0123456789").into_element().with_key("child");
    let mut base = RnkBox::new()
        .width(6)
        .child(child.clone())
        .into_element()
        .with_key("root");
    base.style.overflow_x = Overflow::Scroll;
    let before = full_snapshot(&base, 6, 2);
    base.scroll_offset_x = Some(2);
    let after = full_snapshot(&base, 6, 2);
    assert_eq!(before.root().border_bounds(), after.root().border_bounds());
    assert_eq!(
        before.root().content_bounds(),
        after.root().content_bounds()
    );
    assert_ne!(
        before.nodes().nth(1).unwrap().border_bounds(),
        after.nodes().nth(1).unwrap().border_bounds()
    );
}
