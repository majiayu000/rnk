//! Unit tests for the layout engine.
//!
//! Split out of `engine.rs` to keep that file under the size ceiling; the
//! test bodies are unchanged.

#[allow(unused_imports)]
use super::*;
use crate::components::{Line, Span, Text};
use crate::core::{Color, Element, Props, Style, TextWrap, VNode};
use crate::layout::{TextFlowError, TextFlowSource, TextFlowSourceKind};
use crate::reconciler::Patch;
use std::sync::Arc;

#[test]
fn test_layout_engine_creation() {
    let engine = LayoutEngine::new();
    assert!(engine.node_map.is_empty());
    assert!(engine.vnode_map.is_empty());
    assert!(!engine.has_tree());
}

#[test]
fn test_simple_layout() {
    let mut engine = LayoutEngine::new();

    let mut root = Element::root();
    root.add_child(Element::text("Hello"));

    engine.compute(&root, 80, 24);

    let layout = engine.get_layout(root.id);
    assert!(layout.is_some());
}

#[test]
fn test_text_measurement() {
    let mut engine = LayoutEngine::new();

    let root = Element::text("Hello World");
    engine.compute(&root, 80, 24);

    let layout = engine.get_layout(root.id);
    assert!(layout.is_some());

    let layout = layout.unwrap();
    // "Hello World" is 11 characters wide
    assert!(layout.width >= 11.0);
}

// ==================== VNode Layout Tests ====================

#[test]
fn test_vnode_layout() {
    let mut engine = LayoutEngine::new();

    let root = VNode::box_node()
        .child(VNode::text("Hello"))
        .child(VNode::text("World"));

    engine.compute_vnode(&root, 80, 24);

    assert!(engine.has_tree());
    let layout = engine.get_vnode_layout(root.key);
    assert!(layout.is_some());
}

#[test]
fn test_compute_element_incremental_maps_layouts() {
    let mut engine = LayoutEngine::new();

    let mut root = Element::root();
    let root_id = root.id;

    let mut left = Element::box_element();
    let left_id = left.id;
    let left_text = Element::text("L");
    let left_text_id = left_text.id;
    left.add_child(left_text);

    let mut right = Element::box_element();
    let right_id = right.id;
    let right_text = Element::text("R");
    let right_text_id = right_text.id;
    right.add_child(right_text);

    root.add_child(left);
    root.add_child(right);

    let (_vnode, outcome) = engine.compute_element_incremental(&root, None, 80, 24);
    assert!(!outcome.used_reconciler);
    assert!(engine.get_layout(root_id).is_some());
    assert!(engine.node_key_for_element(root_id).is_some());
    assert!(engine.get_layout(left_id).is_some());
    assert!(engine.node_key_for_element(left_id).is_some());
    assert!(engine.get_layout(left_text_id).is_some());
    assert!(engine.get_layout(right_id).is_some());
    assert!(engine.get_layout(right_text_id).is_some());
}

#[test]
fn test_compute_element_incremental_uses_reconciler_on_next_frame() {
    let mut engine = LayoutEngine::new();

    let mut first = Element::root();
    let mut box_a = Element::box_element();
    box_a.add_child(Element::text("A"));
    first.add_child(box_a);

    let (previous_vnode, first_outcome) = engine.compute_element_incremental(&first, None, 80, 24);
    assert!(!first_outcome.used_reconciler);

    let mut second = Element::root();
    let mut box_b = Element::box_element();
    box_b.add_child(Element::text("B"));
    second.add_child(box_b);
    let second_root_id = second.id;

    let (_current_vnode, second_outcome) =
        engine.compute_element_incremental(&second, Some(&previous_vnode), 80, 24);
    assert!(second_outcome.used_reconciler);
    assert!(engine.get_layout(second_root_id).is_some());
}

#[test]
fn incremental_wrap_modes_refresh_context_bidirectionally() {
    for truncate_mode in [
        TextWrap::Truncate,
        TextWrap::TruncateStart,
        TextWrap::TruncateMiddle,
        TextWrap::TruncateEnd,
    ] {
        let mut engine = LayoutEngine::new();

        let initial_text = Text::new("abcdefgh")
            .key("wrap-context")
            .wrap(TextWrap::Wrap)
            .into_element();
        let initial_text_id = initial_text.id;
        let initial = fixed_width_parent(initial_text);
        let (wrapped_vnode, initial_outcome) =
            engine.compute_element_incremental(&initial, None, 80, 10);
        assert!(!initial_outcome.used_reconciler);
        let initial_layout = engine
            .get_layout(initial_text_id)
            .expect("initial wrapped layout should be available");
        assert_eq!((initial_layout.width, initial_layout.height), (4.0, 2.0));

        let truncated_text = Text::new("abcdefgh")
            .key("wrap-context")
            .wrap(truncate_mode)
            .into_element();
        let truncated_text_id = truncated_text.id;
        let truncated = fixed_width_parent(truncated_text);
        let (truncated_vnode, truncate_outcome) =
            engine.compute_element_incremental(&truncated, Some(&wrapped_vnode), 80, 10);
        assert!(truncate_outcome.used_reconciler);
        assert_eq!(truncate_outcome.patch_count, 1);
        assert!(!truncate_outcome.fallback_full_rebuild);
        let incremental_truncated = engine
            .get_layout(truncated_text_id)
            .expect("incrementally truncated layout should be available");

        let mut rebuilt_truncated_engine = LayoutEngine::new();
        let (_rebuilt_vnode, rebuilt_outcome) =
            rebuilt_truncated_engine.compute_element_incremental(&truncated, None, 80, 10);
        assert!(!rebuilt_outcome.used_reconciler);
        let rebuilt_truncated = rebuilt_truncated_engine
            .get_layout(truncated_text_id)
            .expect("rebuilt truncated layout should be available");
        assert_eq!(
            (incremental_truncated.width, incremental_truncated.height),
            (rebuilt_truncated.width, rebuilt_truncated.height),
            "Wrap -> {truncate_mode:?} must match a full rebuild"
        );
        assert_eq!(
            (incremental_truncated.width, incremental_truncated.height),
            (4.0, 1.0),
            "Wrap -> {truncate_mode:?} must update in the same frame"
        );

        let wrapped_again_text = Text::new("abcdefgh")
            .key("wrap-context")
            .wrap(TextWrap::Wrap)
            .into_element();
        let wrapped_again_text_id = wrapped_again_text.id;
        let wrapped_again = fixed_width_parent(wrapped_again_text);
        let (_current_vnode, wrap_outcome) =
            engine.compute_element_incremental(&wrapped_again, Some(&truncated_vnode), 80, 10);
        assert!(wrap_outcome.used_reconciler);
        assert_eq!(wrap_outcome.patch_count, 1);
        assert!(!wrap_outcome.fallback_full_rebuild);
        let incremental_wrapped = engine
            .get_layout(wrapped_again_text_id)
            .expect("incrementally wrapped layout should be available");

        let mut rebuilt_wrapped_engine = LayoutEngine::new();
        let (_rebuilt_vnode, rebuilt_outcome) =
            rebuilt_wrapped_engine.compute_element_incremental(&wrapped_again, None, 80, 10);
        assert!(!rebuilt_outcome.used_reconciler);
        let rebuilt_wrapped = rebuilt_wrapped_engine
            .get_layout(wrapped_again_text_id)
            .expect("rebuilt wrapped layout should be available");
        assert_eq!(
            (incremental_wrapped.width, incremental_wrapped.height),
            (rebuilt_wrapped.width, rebuilt_wrapped.height),
            "{truncate_mode:?} -> Wrap must match a full rebuild"
        );
        assert_eq!(
            (incremental_wrapped.width, incremental_wrapped.height),
            (4.0, 2.0),
            "{truncate_mode:?} -> Wrap must update in the same frame"
        );
    }
}

#[test]
fn incremental_color_update_preserves_text_shrink_normalization() {
    let mut engine = LayoutEngine::new();

    let first_text = Element::text("abcdefgh").with_key("wrapped");
    let first_text_id = first_text.id;
    let mut first = Element::box_element();
    first.style.width = Dimension::Points(4.0);
    first.add_child(first_text);
    let (previous_vnode, first_outcome) = engine.compute_element_incremental(&first, None, 80, 10);
    assert!(!first_outcome.used_reconciler);
    let first_layout = engine
        .get_layout(first_text_id)
        .expect("initial text layout should be available");
    assert_eq!((first_layout.width, first_layout.height), (4.0, 2.0));

    let mut updated_text = Element::text("abcdefgh").with_key("wrapped");
    updated_text.style.color = Some(crate::core::Color::Red);
    let updated_text_id = updated_text.id;
    let mut updated = Element::box_element();
    updated.style.width = Dimension::Points(4.0);
    updated.add_child(updated_text);
    let (current_vnode, update_outcome) =
        engine.compute_element_incremental(&updated, Some(&previous_vnode), 80, 10);

    assert!(update_outcome.used_reconciler);
    assert_eq!(update_outcome.patch_count, 1);
    assert!(!update_outcome.fallback_full_rebuild);
    let updated_layout = engine
        .get_layout(updated_text_id)
        .expect("updated text layout should be available");
    assert_eq!(
        (updated_layout.width, updated_layout.height),
        (4.0, 2.0),
        "a color-only patch must not restore the text node's automatic min-width"
    );
    let mut rebuilt_engine = LayoutEngine::new();
    let (_rebuilt_vnode, rebuilt_outcome) =
        rebuilt_engine.compute_element_incremental(&updated, None, 80, 10);
    assert!(!rebuilt_outcome.used_reconciler);
    let rebuilt_layout = rebuilt_engine
        .get_layout(updated_text_id)
        .expect("full-build updated text layout should be available");
    assert_eq!(
        (updated_layout.width, updated_layout.height),
        (rebuilt_layout.width, rebuilt_layout.height),
        "incremental and full-build normalization must agree"
    );

    let mut repeated_text = Element::text("abcdefgh").with_key("wrapped");
    repeated_text.style.color = Some(crate::core::Color::Blue);
    let repeated_text_id = repeated_text.id;
    let mut repeated = Element::box_element();
    repeated.style.width = Dimension::Points(4.0);
    repeated.add_child(repeated_text);
    let (_next_vnode, repeated_outcome) =
        engine.compute_element_incremental(&repeated, Some(&current_vnode), 80, 10);
    assert!(repeated_outcome.used_reconciler);
    assert_eq!(repeated_outcome.patch_count, 1);
    let repeated_layout = engine
        .get_layout(repeated_text_id)
        .expect("repeated text layout should be available");
    assert_eq!(
        (repeated_layout.width, repeated_layout.height),
        (4.0, 2.0),
        "repeated style patches must retain text shrink normalization"
    );
}

#[test]
fn incremental_text_update_preserves_explicit_min_width() {
    let mut engine = LayoutEngine::new();

    let mut first_text = Element::text("abcdefgh").with_key("explicit-min");
    first_text.style.min_width = Dimension::Points(6.0);
    let first_text_id = first_text.id;
    let mut first = Element::box_element();
    first.style.width = Dimension::Points(4.0);
    first.add_child(first_text);
    let (previous_vnode, _) = engine.compute_element_incremental(&first, None, 80, 10);
    let initial = engine
        .get_layout(first_text_id)
        .expect("initial explicit-min text layout should be available");
    assert_eq!(initial.width, 6.0);

    let mut updated_text = Element::text("abcdefgh").with_key("explicit-min");
    updated_text.style.min_width = Dimension::Points(6.0);
    updated_text.style.color = Some(crate::core::Color::Green);
    let updated_text_id = updated_text.id;
    let mut updated = Element::box_element();
    updated.style.width = Dimension::Points(4.0);
    updated.add_child(updated_text);
    let (_current_vnode, outcome) =
        engine.compute_element_incremental(&updated, Some(&previous_vnode), 80, 10);

    assert!(outcome.used_reconciler);
    assert_eq!(outcome.patch_count, 1);
    let updated_layout = engine
        .get_layout(updated_text_id)
        .expect("updated explicit-min text layout should be available");
    assert_eq!(
        updated_layout.width, 6.0,
        "the automatic text override must not replace an explicit min-width"
    );
}

#[test]
fn non_text_style_updates_keep_automatic_min_width() {
    let mut engine = LayoutEngine::new();
    let root = VNode::box_node().with_key("container");
    let root_key = root.key;
    engine.compute_vnode(&root, 80, 24);

    let node_id = *engine
        .vnode_map
        .get(&root_key)
        .expect("box node should be present");
    assert_eq!(
        engine.taffy.style(node_id).unwrap().min_size.width,
        ::taffy::Dimension::Auto
    );

    let mut updated_style = Style::new();
    updated_style.color = Some(crate::core::Color::Magenta);
    let changed = engine.apply_patches(&[Patch::update(
        root_key,
        root.props,
        Props::with_style(updated_style),
    )]);
    assert!(changed);
    assert_eq!(
        engine.taffy.style(node_id).unwrap().min_size.width,
        ::taffy::Dimension::Auto,
        "non-text nodes must not receive the text-only shrink override"
    );
}

fn fixed_width_parent(child: Element) -> Element {
    let mut parent = Element::box_element();
    parent.style.width = Dimension::Points(4.0);
    parent.add_child(child);
    parent
}

fn element_min_width(engine: &LayoutEngine, element_id: ElementId) -> ::taffy::Dimension {
    let key = engine
        .node_key_for_element(element_id)
        .expect("element should have a node key");
    let node_id = *engine
        .vnode_map
        .get(&key)
        .expect("element key should map to a Taffy node");
    engine
        .taffy
        .style(node_id)
        .expect("Taffy node should have a style")
        .min_size
        .width
}

#[test]
fn incremental_empty_text_update_keeps_shrink_normalization() {
    let mut engine = LayoutEngine::new();
    let first = fixed_width_parent(Text::new("").key("empty").into_element());
    let (previous_vnode, _) = engine.compute_element_incremental(&first, None, 80, 10);

    let updated_text = Text::new("")
        .key("empty")
        .color(crate::core::Color::Red)
        .into_element();
    let updated_text_id = updated_text.id;
    let updated = fixed_width_parent(updated_text);
    let (_current_vnode, outcome) =
        engine.compute_element_incremental(&updated, Some(&previous_vnode), 80, 10);

    assert!(outcome.used_reconciler);
    assert_eq!(outcome.patch_count, 1);
    assert!(!outcome.fallback_full_rebuild);
    assert_eq!(
        element_min_width(&engine, updated_text_id),
        ::taffy::Dimension::Length(0.0)
    );
}

#[test]
fn incremental_structured_text_update_is_normalized_as_text() {
    let mut engine = LayoutEngine::new();
    let first_text = Text::spans(vec![Span::new("abcd"), Span::new("efgh").bold()])
        .key("structured")
        .into_element();
    assert!(first_text.spans.is_some());
    let first = fixed_width_parent(first_text);
    let (previous_vnode, _) = engine.compute_element_incremental(&first, None, 80, 10);

    let updated_text = Text::spans(vec![Span::new("abcd"), Span::new("efgh").bold()])
        .key("structured")
        .color(crate::core::Color::Cyan)
        .into_element();
    assert!(updated_text.spans.is_some());
    let updated_text_id = updated_text.id;
    let updated = fixed_width_parent(updated_text);
    let (_current_vnode, outcome) =
        engine.compute_element_incremental(&updated, Some(&previous_vnode), 80, 10);

    assert!(outcome.used_reconciler);
    assert_eq!(outcome.patch_count, 1);
    assert!(!outcome.fallback_full_rebuild);
    assert_eq!(
        element_min_width(&engine, updated_text_id),
        ::taffy::Dimension::Length(0.0)
    );
    let layout = engine
        .get_layout(updated_text_id)
        .expect("structured text layout should be available");
    assert_eq!((layout.width, layout.height), (4.0, 2.0));
}

#[test]
fn incremental_create_text_uses_shrink_normalization() {
    let mut engine = LayoutEngine::new();
    let mut first = Element::box_element();
    first.style.width = Dimension::Points(4.0);
    let (previous_vnode, _) = engine.compute_element_incremental(&first, None, 80, 10);

    let created_text = Element::text("abcdefgh").with_key("created");
    let created_text_id = created_text.id;
    let updated = fixed_width_parent(created_text);
    let (_current_vnode, outcome) =
        engine.compute_element_incremental(&updated, Some(&previous_vnode), 80, 10);

    assert!(outcome.used_reconciler);
    assert_eq!(outcome.patch_count, 1);
    assert!(!outcome.fallback_full_rebuild);
    assert_eq!(
        element_min_width(&engine, created_text_id),
        ::taffy::Dimension::Length(0.0)
    );
    let layout = engine
        .get_layout(created_text_id)
        .expect("created text layout should be available");
    assert_eq!((layout.width, layout.height), (4.0, 2.0));
}

#[test]
fn replacement_rebuilds_text_and_non_text_with_their_own_normalization() {
    let mut engine = LayoutEngine::new();
    let box_child = VNode::box_node().with_key("switch");
    let box_key = box_child.key;
    let root = VNode::box_node().child(box_child);
    engine.compute_vnode(&root, 4, 10);
    let original_box_id = *engine.vnode_map.get(&box_key).unwrap();
    assert_eq!(
        engine.taffy.style(original_box_id).unwrap().min_size.width,
        ::taffy::Dimension::Auto
    );

    let text_child = VNode::text("abcdefgh").with_key("switch");
    let text_key = text_child.key;
    assert!(engine.apply_patches(&[Patch::replace(box_key, text_child)]));
    let text_id = *engine.vnode_map.get(&text_key).unwrap();
    assert_ne!(text_id, original_box_id);
    assert_eq!(
        engine.taffy.style(text_id).unwrap().min_size.width,
        ::taffy::Dimension::Length(0.0)
    );

    let replacement_box = VNode::box_node().with_key("switch");
    let replacement_box_key = replacement_box.key;
    assert!(engine.apply_patches(&[Patch::replace(text_key, replacement_box)]));
    let replacement_box_id = *engine.vnode_map.get(&replacement_box_key).unwrap();
    assert_ne!(replacement_box_id, text_id);
    assert_eq!(
        engine
            .taffy
            .style(replacement_box_id)
            .unwrap()
            .min_size
            .width,
        ::taffy::Dimension::Auto
    );
}

#[test]
fn incremental_text_update_preserves_percentage_min_width() {
    let mut engine = LayoutEngine::new();
    let mut first_text = Element::text("abcdefgh").with_key("percentage-min");
    first_text.style.min_width = Dimension::Percent(50.0);
    let first = fixed_width_parent(first_text);
    let (previous_vnode, _) = engine.compute_element_incremental(&first, None, 80, 10);

    let mut updated_text = Element::text("abcdefgh").with_key("percentage-min");
    updated_text.style.min_width = Dimension::Percent(50.0);
    updated_text.style.color = Some(crate::core::Color::Yellow);
    let updated_text_id = updated_text.id;
    let updated = fixed_width_parent(updated_text);
    let (_current_vnode, outcome) =
        engine.compute_element_incremental(&updated, Some(&previous_vnode), 80, 10);

    assert!(outcome.used_reconciler);
    assert_eq!(outcome.patch_count, 1);
    assert!(!outcome.fallback_full_rebuild);
    assert_eq!(
        element_min_width(&engine, updated_text_id),
        ::taffy::Dimension::Percent(0.5)
    );
}

#[test]
fn test_incremental_layout_avoids_key_collision_across_branches() {
    let mut engine = LayoutEngine::new();

    let mut root = Element::root();

    let mut left = Element::box_element();
    let left_text = Element::text("left").with_key("item");
    let left_text_id = left_text.id;
    left.add_child(left_text);

    let mut right = Element::box_element();
    let right_text = Element::text("right").with_key("item");
    let right_text_id = right_text.id;
    right.add_child(right_text);

    root.add_child(left);
    root.add_child(right);

    let (_vnode, _outcome) = engine.compute_element_incremental(&root, None, 80, 24);

    assert!(engine.get_layout(left_text_id).is_some());
    assert!(engine.get_layout(right_text_id).is_some());
}

#[test]
fn test_incremental_layout_keyed_reorder_no_fallback() {
    let mut engine = LayoutEngine::new();

    let mut first = Element::root();
    first.add_child(Element::box_element().with_key("a"));
    first.add_child(Element::box_element().with_key("b"));
    let (previous_vnode, first_outcome) = engine.compute_element_incremental(&first, None, 80, 24);
    assert!(!first_outcome.used_reconciler);

    let mut second = Element::root();
    let second_a = Element::box_element().with_key("a");
    let second_a_id = second_a.id;
    let second_b = Element::box_element().with_key("b");
    let second_b_id = second_b.id;
    second.add_child(second_b);
    second.add_child(second_a);

    let (_current_vnode, second_outcome) =
        engine.compute_element_incremental(&second, Some(&previous_vnode), 80, 24);

    assert!(second_outcome.used_reconciler);
    assert!(!second_outcome.fallback_full_rebuild);
    assert!(engine.get_layout(second_a_id).is_some());
    assert!(engine.get_layout(second_b_id).is_some());
}

#[test]
fn test_vnode_text_measurement() {
    let mut engine = LayoutEngine::new();

    let root = VNode::text("Hello World");
    engine.compute_vnode(&root, 80, 24);

    let layout = engine.get_vnode_layout(root.key);
    assert!(layout.is_some());

    let layout = layout.unwrap();
    assert!(layout.width >= 11.0);
}

#[test]
fn test_apply_patches_update() {
    let mut engine = LayoutEngine::new();

    let root = VNode::box_node().child(VNode::text("Hello"));
    engine.compute_vnode(&root, 80, 24);

    // Create an update patch
    let mut new_style = Style::new();
    new_style.padding.top = 5.0;
    let new_props = Props::with_style(new_style);

    let patches = vec![Patch::update(root.key, Props::new(), new_props)];

    let changed = engine.apply_patches(&patches);
    assert!(changed);
}

#[test]
fn test_apply_patches_empty() {
    let mut engine = LayoutEngine::new();

    let root = VNode::box_node();
    engine.compute_vnode(&root, 80, 24);

    let changed = engine.apply_patches(&[]);
    assert!(!changed);
}

#[test]
fn test_apply_patches_create() {
    let mut engine = LayoutEngine::new();

    let root = VNode::box_node();
    engine.compute_vnode(&root, 80, 24);

    let new_child = VNode::text("New child");
    let patches = vec![Patch::create(new_child, root.key)];

    let changed = engine.apply_patches(&patches);
    assert!(changed);
}

#[test]
fn test_apply_patches_remove() {
    let mut engine = LayoutEngine::new();

    let child = VNode::text("Child");
    let child_key = child.key;
    let root = VNode::box_node().child(child);
    engine.compute_vnode(&root, 80, 24);

    let patches = vec![Patch::remove(child_key)];

    let changed = engine.apply_patches(&patches);
    assert!(changed);
    assert!(engine.get_vnode_layout(child_key).is_none());
}

#[test]
fn test_get_all_vnode_layouts() {
    let mut engine = LayoutEngine::new();

    let root = VNode::box_node()
        .child(VNode::text("A"))
        .child(VNode::text("B"));

    engine.compute_vnode(&root, 80, 24);

    let layouts = engine.get_all_vnode_layouts();
    assert_eq!(layouts.len(), 3); // root + 2 children
}

#[test]
fn test_node_count() {
    let mut engine = LayoutEngine::new();

    // Use unique keys to avoid collision
    let root = VNode::box_node()
        .child(VNode::text("A").with_key("a"))
        .child(VNode::box_node().child(VNode::text("B").with_key("b")));

    engine.compute_vnode(&root, 80, 24);

    // root + text "A" + inner box + text "B" = 4 nodes
    assert_eq!(engine.node_count(), 4);
}

#[test]
fn incremental_no_patch_refreshes_source_and_style() {
    let mut engine = LayoutEngine::new();
    let first = fixed_width_parent(Text::new("a\r\nb").key("flow").into_element());
    let first_result = engine.try_compute_element_incremental(&first, None, 80, 10);
    let (first_vnode, _) = first_result.unwrap();
    let source_update = fixed_width_parent(Text::new("a\nb").key("flow").into_element());
    let source_id = source_update.children.get(0).unwrap().id;
    let (source_vnode, source_outcome) = engine
        .try_compute_element_incremental(&source_update, Some(&first_vnode), 80, 10)
        .unwrap();
    assert_eq!(source_outcome.patch_count, 0);
    let source_flow = engine.current_text_flow(source_id).unwrap();
    assert_eq!(source_flow.cache_identity().input.source, "a\nb");
    let mut styled = Text::new("a\nb").key("flow").into_element();
    styled.spans.as_mut().unwrap()[0].spans[0].style.color = Some(Color::Red);
    let styled = fixed_width_parent(styled);
    let styled_id = styled.children.get(0).unwrap().id;
    let (_, style_outcome) = engine
        .try_compute_element_incremental(&styled, Some(&source_vnode), 80, 10)
        .unwrap();
    assert_eq!(style_outcome.patch_count, 0);
    let styled_flow = engine.current_text_flow(styled_id).unwrap();
    let color = styled_flow.logical_rows()[0].runs[0].style.color;
    assert_eq!(color, Some(Color::Red));
}

#[test]
fn plain_text_style_is_published() {
    let mut text = Element::text("plain");
    text.style.color = Some(Color::Blue);
    let id = text.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&text, 20, 4).unwrap();
    let flow = engine.current_text_flow(id).unwrap();
    let color = flow.logical_rows()[0].runs[0].style.color;
    assert_eq!(color, Some(Color::Blue));
}

#[test]
fn alignable_crlf_spans_keep_exact_source_domain() {
    let text = Text::new("a\r\nb\r\n").into_element();
    let id = text.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&text, 20, 4).unwrap();
    let flow = engine.current_text_flow(id).unwrap();
    assert_eq!(
        flow.cache_identity().input.source_kind,
        TextFlowSourceKind::Exact
    );
    let ranges: Vec<_> = flow
        .tokens()
        .iter()
        .filter_map(|token| token.source_range())
        .collect();
    assert_eq!(ranges, [0..1, 1..3, 3..4, 4..6]);
}

#[test]
fn known_dimensions_publish_final_width_flow() {
    let mut text = Element::text("abcdefgh");
    text.style.width = Dimension::Points(4.0);
    let id = text.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&text, 80, 10).unwrap();
    let flow = engine.current_text_flow(id).unwrap();
    assert_eq!(flow.cache_identity().options.max_width, 4);
    assert_eq!(flow.row_count(), 2);
}

#[test]
fn reconstructed_source_domain_uses_text_content_truth() {
    let mut text = Element::text("truth");
    text.spans = Some(vec![Line::raw("different")]);
    let id = text.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&text, 20, 4).unwrap();
    let flow = engine.current_text_flow(id).unwrap();
    assert_eq!(flow.cache_identity().input.source, "truth");
    assert!(flow.tokens().iter().all(|token| matches!(
        token.source,
        TextFlowSource::Source {
            kind: TextFlowSourceKind::Reconstructed,
            ..
        }
    )));
}

#[test]
fn text_flow_failure_is_atomic() {
    let text = Element::text("stable");
    let id = text.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&text, 20, 4).unwrap();
    let published = engine.current_text_flow(id).unwrap();
    let layout = engine.get_layout(id).unwrap();
    engine.set_text_flow_policy(0, "…", 1);
    let failure = engine.try_compute(&Element::text("new"), 20, 4);
    assert_eq!(failure, Err(TextFlowError::InvalidTabStop));
    let current = engine.current_text_flow(id).unwrap();
    assert!(Arc::ptr_eq(&published, &current));
    assert_eq!(engine.get_layout(id).unwrap().width, layout.width);
    engine.set_text_flow_policy(4, "…", 1);
    let cancelled = engine.try_compute_interruptible(&Element::text("cancel"), 20, 4, || true);
    assert_eq!(cancelled, Err(TextFlowError::Interrupted));
    let current = engine.current_text_flow(id).unwrap();
    assert!(Arc::ptr_eq(&published, &current));
}

#[test]
fn try_compute_entrypoints_return_text_flow_error() {
    let mut engine = LayoutEngine::new();
    engine.set_text_flow_policy(0, "..", 1);
    let direct = engine.try_compute(&Element::text("x"), 4, 4);
    assert_eq!(direct, Err(TextFlowError::InvalidTabStop));
    let vnode = engine.try_compute_vnode(&VNode::text("x"), 4, 4);
    assert_eq!(vnode, Err(TextFlowError::InvalidTabStop));
    let incremental = engine.try_compute_element_incremental(&Element::text("x"), None, 4, 4);
    assert!(matches!(incremental, Err(TextFlowError::InvalidTabStop)));
}
