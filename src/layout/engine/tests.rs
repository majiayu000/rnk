//! Unit tests for the layout engine.
//!
//! Split out of `engine.rs` to keep that file under the size ceiling; the
//! test bodies are unchanged.

#[allow(unused_imports)]
use super::*;
use crate::core::{Element, Props, Style, VNode};
use crate::reconciler::Patch;

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
