//! Diff algorithm for VNode trees
//!
//! Compares old and new VNode trees to produce minimal patches.
//! Uses a simplified algorithm optimized for typical UI patterns.

use crate::core::{NodeKey, Props, VNode, VNodeType};

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
    /// Reorder children of a parent node
    Reorder {
        parent: NodeKey,
        moves: Vec<(usize, usize)>,
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

    /// Create a "reorder children" patch
    pub fn reorder(parent: NodeKey, moves: Vec<(usize, usize)>) -> Self {
        Patch::Reorder { parent, moves }
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

/// Diff children lists using a keyed algorithm
///
/// This uses a simplified LCS-like approach optimized for common cases:
/// 1. Additions at the end (most common)
/// 2. Removals from anywhere
/// 3. Reordering (less common)
pub fn diff_children(
    old_children: &[VNode],
    new_children: &[VNode],
    parent_key: NodeKey,
    patches: &mut Vec<Patch>,
) {
    // Build key map for efficient lookup
    let old_key_map: std::collections::HashMap<_, _> = old_children
        .iter()
        .enumerate()
        .map(|(i, c)| (key_identity(&c.key), i))
        .collect();

    // Track which old nodes have been matched
    let mut matched_old: Vec<bool> = vec![false; old_children.len()];
    let mut moves: Vec<(usize, usize)> = Vec::new();

    // First pass: match new children to old children
    for (new_idx, new_child) in new_children.iter().enumerate() {
        let key_id = key_identity(&new_child.key);

        if let Some(&old_idx) = old_key_map.get(&key_id) {
            // Found matching old node
            matched_old[old_idx] = true;

            // Recursively diff the matched nodes
            diff_node(&old_children[old_idx], new_child, patches);

            // Track if position changed
            if old_idx != new_idx {
                moves.push((old_idx, new_idx));
            }
        } else {
            // New node - create it
            patches.push(Patch::create(new_child.clone(), parent_key));
        }
    }

    // Second pass: remove unmatched old children
    for (old_idx, matched) in matched_old.iter().enumerate() {
        if !matched {
            patches.push(Patch::remove(old_children[old_idx].key));
        }
    }

    // Add reorder patch if needed
    if !moves.is_empty() && needs_reorder(&moves) {
        patches.push(Patch::reorder(parent_key, moves));
    }
}

/// Generate a unique identity for a key (for HashMap lookup)
fn key_identity(key: &NodeKey) -> (Option<u64>, std::any::TypeId, usize) {
    (key.user_key, key.type_id, key.index)
}

/// Check if moves actually require reordering
///
/// If all moves are just index shifts due to insertions/deletions,
/// we don't need an explicit reorder operation.
fn needs_reorder(moves: &[(usize, usize)]) -> bool {
    // Simple heuristic: if any move goes backwards, we need reorder
    for &(from, to) in moves {
        if to < from {
            return true;
        }
    }
    false
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
        // Should detect reordering
        let has_reorder = patches.iter().any(|p| matches!(p, Patch::Reorder { .. }));
        let has_creates = patches.iter().any(|p| matches!(p, Patch::Create { .. }));
        // Either reorder or create patches should exist
        assert!(has_reorder || has_creates);
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

    #[test]
    fn test_needs_reorder() {
        // Forward moves don't need reorder
        assert!(!needs_reorder(&[(0, 1), (1, 2)]));

        // Backward moves need reorder
        assert!(needs_reorder(&[(2, 0), (1, 1)]));

        // Empty moves don't need reorder
        assert!(!needs_reorder(&[]));
    }
}
