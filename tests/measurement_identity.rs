//! GH-59 corrective regressions for scoped layout and measurement identity.

use std::panic::{AssertUnwindSafe, catch_unwind};

use rnk::core::VNode;
use rnk::layout::{LayoutEngine, LayoutLookupError};

fn repeated_leaf_tree() -> VNode {
    VNode::box_node().children([
        VNode::box_node()
            .with_key("left")
            .child(VNode::text("short").with_key("shared")),
        VNode::box_node()
            .with_key("right")
            .child(VNode::text("much longer").with_key("shared")),
    ])
}

#[test]
fn same_raw_key_in_distinct_parents_keeps_both_layout_entries() {
    let tree = repeated_leaf_tree();
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 10);

    assert_eq!(
        engine.get_all_vnode_layouts().len(),
        tree.node_count(),
        "a global sibling identity map must not overwrite another parent scope"
    );
}

#[test]
fn raw_legacy_lookup_fails_loudly_when_multiple_scopes_match() {
    let tree = repeated_leaf_tree();
    let raw_shared_key = tree.children[0].children[0].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 10);

    let result = catch_unwind(AssertUnwindSafe(|| engine.get_vnode_layout(raw_shared_key)));

    assert!(
        result.is_err(),
        "an unscoped raw key must not silently select one parent"
    );
}

#[test]
fn checked_raw_lookup_reports_the_match_count() {
    let tree = repeated_leaf_tree();
    let raw_shared_key = tree.children[0].children[0].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&tree, 40, 10);

    let failure = engine
        .try_get_vnode_layout(raw_shared_key)
        .expect_err("unscoped key must be ambiguous");

    assert!(matches!(
        failure,
        LayoutLookupError::AmbiguousLegacyNodeKey {
            scoped_match_count: 2,
            ..
        }
    ));
}
