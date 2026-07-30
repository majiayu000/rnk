//! Diff algorithm for VNode trees
//!
//! Compares old and new VNode trees to produce minimal patches.
//! Uses a simplified algorithm optimized for typical UI patterns.

use std::collections::HashMap;

use crate::core::{NodeKey, Props, VNode, VNodeType};
use crate::reconciler::SiblingIdentity;

/// A patch representing a change to apply to the tree
#[derive(Debug, Clone)]
pub enum Patch {
    /// Create a new node under a parent
    Create {
        key: NodeKey,
        parent: NodeKey,
        props: Props,
        node: VNode,
    },
    /// Update an existing node's props
    Update {
        key: NodeKey,
        old_props: Props,
        new_props: Props,
    },
    /// Remove a node
    Remove { key: NodeKey },
    /// Replace a node entirely (different type)
    Replace {
        key: NodeKey,
        new_props: Props,
        node: VNode,
    },
    /// Set a parent's children to exactly this order.
    ///
    /// The full target order, not a set of moves. A move list only describes
    /// where surviving nodes went, so it cannot say where a newly created
    /// sibling belongs, and applying it position by position can duplicate or
    /// drop a child. Carrying the whole order makes "Taffy order equals VNode
    /// order" something the apply step can simply establish and assert.
    Reorder {
        parent: NodeKey,
        order: Vec<NodeKey>,
    },
}

impl Patch {
    /// Create a "create node" patch
    pub fn create(node: VNode, parent: NodeKey) -> Self {
        Patch::Create {
            key: node.key,
            parent,
            props: node.props.clone(),
            node,
        }
    }

    /// Create an "update props" patch
    pub fn update(key: NodeKey, old_props: Props, new_props: Props) -> Self {
        Patch::Update {
            key,
            old_props,
            new_props,
        }
    }

    /// Create a "remove node" patch
    pub fn remove(key: NodeKey) -> Self {
        Patch::Remove { key }
    }

    /// Create a "replace node" patch
    pub fn replace(old_key: NodeKey, new_node: VNode) -> Self {
        Patch::Replace {
            key: old_key,
            new_props: new_node.props.clone(),
            node: new_node,
        }
    }

    /// Create a "set children order" patch
    pub fn reorder(parent: NodeKey, order: Vec<NodeKey>) -> Self {
        Patch::Reorder { parent, order }
    }
}

/// Diff two VNode trees and produce patches
///
/// This is the main entry point for the diff algorithm.
/// It compares the old and new trees and returns a list of
/// patches that transform the old tree into the new tree.
pub fn diff(old: &VNode, new: &VNode) -> Vec<Patch> {
    let mut patches = Vec::new();
    diff_node(old, new, &mut patches);
    patches
}

/// Diff a single node
fn diff_node(old: &VNode, new: &VNode, patches: &mut Vec<Patch>) {
    // If keys don't match, this is a replacement
    if !old.key.matches(&new.key) {
        patches.push(Patch::replace(old.key, new.clone()));
        return;
    }

    // If node types are different, replace
    if std::mem::discriminant(&old.node_type) != std::mem::discriminant(&new.node_type) {
        patches.push(Patch::replace(old.key, new.clone()));
        return;
    }

    // Check for text content changes
    if let (VNodeType::Text(old_text), VNodeType::Text(new_text)) = (&old.node_type, &new.node_type)
    {
        if old_text != new_text {
            patches.push(Patch::replace(old.key, new.clone()));
            return;
        }
    }

    // Check for props changes
    if old.props != new.props {
        patches.push(Patch::update(old.key, old.props.clone(), new.props.clone()));
    }

    // Diff children
    diff_children(&old.children, &new.children, old.key, patches);
}

/// Diff children lists, matching by [`SiblingIdentity`].
///
/// Surviving children are diffed in place, absent ones removed, unrecognised
/// ones created, and the parent's final child order stated outright whenever it
/// differs from the order the old children were in.
pub fn diff_children(
    old_children: &[VNode],
    new_children: &[VNode],
    parent_key: NodeKey,
    patches: &mut Vec<Patch>,
) {
    // First occurrence wins. A duplicate identity must never rebind the map
    // entry of an existing node, or which node a key refers to would depend on
    // sibling order.
    let mut old_by_identity: HashMap<SiblingIdentity, usize> = HashMap::new();
    for (old_idx, child) in old_children.iter().enumerate() {
        old_by_identity
            .entry(child.key.identity())
            .or_insert(old_idx);
    }

    let mut matched_old = vec![false; old_children.len()];
    // The keys the parent must end up holding, in order, as of the new tree.
    // A survivor's new key carries a fresh `index` but the same identity, which
    // is what the apply step resolves against.
    let mut final_order = Vec::with_capacity(new_children.len());

    // Creating a node appends it, so the order is already right only when the
    // survivors are an unbroken run from the front of the old list and every
    // create lands after all of them. Anything else has to be stated.
    let mut already_in_order = true;
    let mut next_untouched_old = 0usize;
    let mut seen_create = false;

    for new_child in new_children {
        let identity = new_child.key.identity();
        // An old node can be claimed once. Two new children sharing an identity
        // are not the same node, so the second is a create, not a second match.
        let survivor = old_by_identity
            .get(&identity)
            .copied()
            .filter(|&old_idx| !matched_old[old_idx]);

        match survivor {
            Some(old_idx) => {
                if seen_create || old_idx != next_untouched_old {
                    already_in_order = false;
                }
                next_untouched_old = old_idx + 1;
                matched_old[old_idx] = true;
                diff_node(&old_children[old_idx], new_child, patches);
                final_order.push(new_child.key);
            }
            None => {
                seen_create = true;
                patches.push(Patch::create(new_child.clone(), parent_key));
                final_order.push(new_child.key);
            }
        }
    }

    for (old_idx, matched) in matched_old.iter().enumerate() {
        if !matched {
            patches.push(Patch::remove(old_children[old_idx].key));
        }
    }

    if !already_in_order {
        patches.push(Patch::reorder(parent_key, final_order));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::VNode;

    fn lcs_length<T: PartialEq>(a: &[T], b: &[T]) -> usize {
        let m = a.len();
        let n = b.len();

        if m == 0 || n == 0 {
            return 0;
        }

        let mut dp = vec![vec![0; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        dp[m][n]
    }

    #[test]
    fn test_diff_identical_trees() {
        // Create new nodes with same structure but matching keys
        let old = VNode::box_node()
            .with_index(0)
            .child(VNode::text("Hello").with_index(0));
        let mut new = VNode::box_node().with_index(0);
        new.key = old.key; // Same key
        let mut text_child = VNode::text("Hello").with_index(0);
        text_child.key = old.children[0].key; // Same key
        new = new.child(text_child);
        new.children[0].key = old.children[0].key;

        let patches = diff(&old, &new);
        // Should have no patches for identical trees with same keys
        assert!(
            patches.is_empty()
                || patches
                    .iter()
                    .all(|p| matches!(p, Patch::Update { old_props, new_props, .. } if old_props == new_props))
        );
    }

    #[test]
    fn test_diff_text_change() {
        let old = VNode::text("Hello");
        let mut new = VNode::text("World");
        new.key = old.key; // Same key, different content

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Replace { .. }));
    }

    #[test]
    fn test_diff_props_change() {
        use crate::core::{Props, Style};

        let old = VNode::box_node();
        let mut new = VNode::box_node();
        new.key = old.key;

        let mut new_style = Style::new();
        new_style.padding.top = 10.0;
        new.props = Props::with_style(new_style);

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Update { .. }));
    }

    #[test]
    fn test_diff_add_child() {
        let old = VNode::box_node();
        let mut new = VNode::box_node();
        new.key = old.key;
        new = new.child(VNode::text("New child"));

        let patches = diff(&old, &new);
        assert!(patches.iter().any(|p| matches!(p, Patch::Create { .. })));
    }

    #[test]
    fn test_diff_remove_child() {
        let old = VNode::box_node().child(VNode::text("Child"));
        let mut new = VNode::box_node();
        new.key = old.key;

        let patches = diff(&old, &new);
        assert!(patches.iter().any(|p| matches!(p, Patch::Remove { .. })));
    }

    #[test]
    fn test_diff_replace_different_type() {
        let old = VNode::box_node();
        let new = VNode::text("Replaced");

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Replace { .. }));
    }

    #[test]
    fn test_diff_keyed_reorder() {
        let old = VNode::box_node()
            .child(VNode::text("A").with_key("a"))
            .child(VNode::text("B").with_key("b"))
            .child(VNode::text("C").with_key("c"));

        let mut new = VNode::box_node();
        new.key = old.key;
        new = new
            .child(VNode::text("C").with_key("c"))
            .child(VNode::text("A").with_key("a"))
            .child(VNode::text("B").with_key("b"));

        let patches = diff(&old, &new);

        // The previous assertion was `has_reorder || has_creates`, which held
        // even when every keyed child was destroyed and rebuilt — which is what
        // was happening.
        assert_eq!(
            final_order(&patches),
            Some(keys(&new)),
            "reorder must state the whole target order: {patches:?}"
        );
        assert!(
            !patches
                .iter()
                .any(|p| matches!(p, Patch::Create { .. } | Patch::Remove { .. })),
            "moving keyed children must preserve them, not rebuild them: {patches:?}"
        );
    }

    /// Keys of a node's children, in order.
    fn keys(parent: &VNode) -> Vec<NodeKey> {
        parent.children.iter().map(|child| child.key).collect()
    }

    /// The order stated by the single `Reorder` patch, if there is one.
    fn final_order(patches: &[Patch]) -> Option<Vec<NodeKey>> {
        let mut found = patches.iter().filter_map(|p| match p {
            Patch::Reorder { order, .. } => Some(order.clone()),
            _ => None,
        });
        let first = found.next();
        assert!(found.next().is_none(), "one Reorder per parent per frame");
        first
    }

    /// Build a parent whose children carry the given keys, reusing one parent
    /// key so the two frames describe the same node.
    fn parent_with(parent_key: NodeKey, child_keys: &[&str]) -> VNode {
        let mut parent = VNode::box_node();
        parent.key = parent_key;
        for key in child_keys {
            parent = parent.child(VNode::text("x").with_key(key));
        }
        parent
    }

    /// Identities in the order the patches leave them, given a starting order.
    fn apply_order(before: &[&str], after: &[&str]) -> Vec<Patch> {
        let parent_key = VNode::box_node().key;
        diff(
            &parent_with(parent_key, before),
            &parent_with(parent_key, after),
        )
    }

    #[test]
    fn keyed_children_keep_their_identity_through_every_edit() {
        // Front, middle and tail insert; delete; swap; and a multi-position
        // move. In each case the surviving keys must not be rebuilt.
        let cases: &[(&[&str], &[&str])] = &[
            (&["b", "c"], &["a", "b", "c"]),
            (&["a", "c"], &["a", "b", "c"]),
            (&["a", "b"], &["a", "b", "c"]),
            (&["a", "b", "c"], &["a", "c"]),
            (&["a", "b"], &["b", "a"]),
            (&["a", "b", "c", "d"], &["d", "b", "c", "a"]),
            (&["a", "b", "c"], &["c", "b", "a"]),
        ];

        for (before, after) in cases {
            let patches = apply_order(before, after);
            let survivors: Vec<&&str> = after.iter().filter(|k| before.contains(k)).collect();

            let rebuilt: Vec<_> = patches
                .iter()
                .filter(|p| matches!(p, Patch::Create { .. }))
                .collect();
            assert_eq!(
                rebuilt.len(),
                after.len() - survivors.len(),
                "{before:?} -> {after:?} created a node it should have kept: {patches:?}"
            );
        }
    }

    #[test]
    fn a_pure_append_needs_no_reorder() {
        // Creating a node appends it, so this order is already correct and
        // restating it would be pointless work every frame.
        let patches = apply_order(&["a", "b"], &["a", "b", "c"]);
        assert_eq!(final_order(&patches), None, "{patches:?}");
    }

    #[test]
    fn a_front_insert_states_the_order_because_appending_is_wrong() {
        let patches = apply_order(&["b", "c"], &["a", "b", "c"]);
        assert!(
            final_order(&patches).is_some(),
            "a create landing at the front must be positioned: {patches:?}"
        );
    }

    #[test]
    fn a_trailing_removal_needs_no_reorder() {
        let patches = apply_order(&["a", "b", "c"], &["a", "b"]);
        assert_eq!(final_order(&patches), None, "{patches:?}");
    }

    #[test]
    fn a_duplicate_key_does_not_rebind_an_existing_node() {
        // Two siblings sharing a key are not the same node. The first claims
        // the existing node; the second must be created, never silently
        // remapped onto it.
        let parent_key = VNode::box_node().key;
        let patches = diff(
            &parent_with(parent_key, &["a"]),
            &parent_with(parent_key, &["a", "a"]),
        );

        let creates = patches
            .iter()
            .filter(|p| matches!(p, Patch::Create { .. }))
            .count();
        assert_eq!(creates, 1, "{patches:?}");
        assert!(
            !patches.iter().any(|p| matches!(p, Patch::Remove { .. })),
            "the first occurrence must keep the existing node: {patches:?}"
        );
    }

    #[test]
    fn an_unkeyed_child_is_still_matched_by_position() {
        // Unchanged public semantics: without a key, position is identity.
        let parent_key = VNode::box_node().key;
        let mut before = VNode::box_node();
        before.key = parent_key;
        let before = before.child(VNode::text("x")).child(VNode::text("y"));

        let mut after = VNode::box_node();
        after.key = parent_key;
        let after = after.child(VNode::text("x")).child(VNode::text("y"));

        assert!(diff(&before, &after).is_empty());
    }

    #[test]
    fn test_patch_creation() {
        let node = VNode::text("Test");
        let parent = NodeKey::root();

        let patch = Patch::create(node.clone(), parent);
        assert!(matches!(patch, Patch::Create { parent: p, .. } if p == parent));
    }

    #[test]
    fn test_lcs_length() {
        let a = vec![1, 2, 3, 4, 5];
        let b = vec![2, 3, 5];
        assert_eq!(lcs_length(&a, &b), 3);

        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        assert_eq!(lcs_length(&a, &b), 0);

        let a: Vec<i32> = vec![];
        let b = vec![1, 2, 3];
        assert_eq!(lcs_length(&a, &b), 0);
    }
}
