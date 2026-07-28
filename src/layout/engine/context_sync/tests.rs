use std::collections::HashMap;
use std::sync::Arc;

use super::super::*;
use crate::core::{Color, Element, Overflow, TextWrap};
use crate::layout::engine::text_flow_bridge::input_from_element;
use crate::layout::{TextFlowInput, TextFlowSourceKind};

#[test]
fn identical_context_sync_keeps_text_leaf_and_root_clean_and_reuses_flow() {
    let text = Element::text("stable").with_key("leaf");
    let text_id = text.id;
    let input = input_from_element(&text).expect("text element must produce input");
    let mut root = Element::root();
    root.add_child(text);

    let mut engine = LayoutEngine::new();
    let (previous, initial) = engine
        .try_compute_element_incremental(&root, None, 20, 4)
        .unwrap();
    assert_eq!(initial.patch_count, 0);

    let key = engine
        .node_key_for_element(text_id)
        .expect("text element must keep its stable node key");
    let leaf = engine.node_map[&text_id];
    let root_node = engine.root_node.expect("computed tree must keep its root");
    let initial_flow = engine
        .current_text_flow(text_id)
        .expect("computed text must publish a flow");
    let initial_layout = engine.get_layout(text_id).unwrap();
    assert!(!engine.taffy.dirty(leaf).unwrap());
    assert!(!engine.taffy.dirty(root_node).unwrap());

    engine.sync_text_contexts(&HashMap::from([(key, input)]));

    assert!(!engine.taffy.dirty(leaf).unwrap());
    assert!(!engine.taffy.dirty(root_node).unwrap());
    engine.run_layout_and_publish(&mut || false).unwrap();
    assert!(Arc::ptr_eq(
        &initial_flow,
        &engine.current_text_flow(text_id).unwrap()
    ));

    let (_, repeated) = engine
        .try_compute_element_incremental(&root, Some(&previous), 20, 4)
        .unwrap();
    assert!(repeated.used_reconciler);
    assert_eq!(repeated.patch_count, 0);
    assert!(!repeated.fallback_full_rebuild);
    assert_eq!(engine.node_map[&text_id], leaf);
    assert_eq!(engine.root_node, Some(root_node));
    assert_eq!(
        engine.get_layout(text_id).unwrap().width,
        initial_layout.width
    );
    assert!(Arc::ptr_eq(
        &initial_flow,
        &engine.current_text_flow(text_id).unwrap()
    ));
}

struct DirtyFixture {
    engine: LayoutEngine,
    root: taffy::NodeId,
    left_branch: taffy::NodeId,
    left_leaf: taffy::NodeId,
    right_branch: taffy::NodeId,
    right_leaf: taffy::NodeId,
    left_key: crate::core::NodeKey,
    right_key: crate::core::NodeKey,
    left_id: crate::core::ElementId,
    right_id: crate::core::ElementId,
    left_input: TextFlowInput,
    right_input: TextFlowInput,
    left_flow: Arc<TextFlow>,
    right_flow: Arc<TextFlow>,
}

impl DirtyFixture {
    fn new() -> Self {
        let left_text = Element::text("left").with_key("left");
        let left_id = left_text.id;
        let left_input = input_from_element(&left_text).unwrap();
        let mut left_branch_element = Element::box_element().with_key("left-branch");
        let left_branch_id = left_branch_element.id;
        left_branch_element.add_child(left_text);

        let right_text = Element::text("right").with_key("right");
        let right_id = right_text.id;
        let right_input = input_from_element(&right_text).unwrap();
        let mut right_branch_element = Element::box_element().with_key("right-branch");
        let right_branch_id = right_branch_element.id;
        right_branch_element.add_child(right_text);

        let mut root_element = Element::root();
        root_element.add_child(left_branch_element);
        root_element.add_child(right_branch_element);

        let mut engine = LayoutEngine::new();
        engine
            .try_compute_element_incremental(&root_element, None, 20, 4)
            .unwrap();
        let root = engine.root_node.unwrap();
        let left_branch = engine.node_map[&left_branch_id];
        let left_leaf = engine.node_map[&left_id];
        let right_branch = engine.node_map[&right_branch_id];
        let right_leaf = engine.node_map[&right_id];
        let left_key = engine.node_key_for_element(left_id).unwrap();
        let right_key = engine.node_key_for_element(right_id).unwrap();
        let left_flow = engine.current_text_flow(left_id).unwrap();
        let right_flow = engine.current_text_flow(right_id).unwrap();

        Self {
            engine,
            root,
            left_branch,
            left_leaf,
            right_branch,
            right_leaf,
            left_key,
            right_key,
            left_id,
            right_id,
            left_input,
            right_input,
            left_flow,
            right_flow,
        }
    }

    fn sync(&mut self, left_input: TextFlowInput) {
        self.engine.sync_text_contexts(&HashMap::from([
            (self.left_key, left_input),
            (self.right_key, self.right_input.clone()),
        ]));
    }

    fn assert_only_left_path_dirty(&self) {
        assert!(self.engine.taffy.dirty(self.left_leaf).unwrap());
        assert!(self.engine.taffy.dirty(self.left_branch).unwrap());
        assert!(self.engine.taffy.dirty(self.root).unwrap());
        assert!(!self.engine.taffy.dirty(self.right_leaf).unwrap());
        assert!(!self.engine.taffy.dirty(self.right_branch).unwrap());
    }

    fn assert_all_text_paths_dirty(&self) {
        for node in [
            self.root,
            self.left_branch,
            self.left_leaf,
            self.right_branch,
            self.right_leaf,
        ] {
            assert!(self.engine.taffy.dirty(node).unwrap());
        }
    }

    fn publish_and_assert_flow_changes(&mut self, left_changed: bool, right_changed: bool) {
        self.engine.run_layout_and_publish(&mut || false).unwrap();
        assert_eq!(
            !Arc::ptr_eq(
                &self.left_flow,
                &self.engine.current_text_flow(self.left_id).unwrap()
            ),
            left_changed
        );
        assert_eq!(
            !Arc::ptr_eq(
                &self.right_flow,
                &self.engine.current_text_flow(self.right_id).unwrap()
            ),
            right_changed
        );
    }
}

#[test]
fn source_style_wrap_and_overflow_changes_dirty_only_the_affected_text_path() {
    for case in ["source", "style", "wrap", "overflow"] {
        let mut fixture = DirtyFixture::new();
        let mut changed = fixture.left_input.clone();
        match case {
            "source" => {
                changed = TextFlowInput::plain(
                    "changed",
                    TextFlowSourceKind::Exact,
                    changed.default_style,
                );
            }
            "style" => changed.default_style.color = Some(Color::Red),
            "wrap" => changed.default_style.text_wrap = TextWrap::TruncateMiddle,
            "overflow" => changed.default_style.overflow_x = Overflow::Hidden,
            _ => unreachable!(),
        }
        fixture.sync(changed);
        fixture.assert_only_left_path_dirty();
        fixture.publish_and_assert_flow_changes(true, false);
    }
}

#[test]
fn tab_ellipsis_and_width_policy_changes_dirty_every_text_path() {
    for (tab_stop, ellipsis, revision) in [(8, "…", 1), (4, "..", 1), (4, "…", 2)] {
        let mut fixture = DirtyFixture::new();
        fixture
            .engine
            .set_text_flow_policy(tab_stop, ellipsis, revision);
        fixture.sync(fixture.left_input.clone());
        fixture.assert_all_text_paths_dirty();
        fixture.publish_and_assert_flow_changes(true, true);
    }
}
