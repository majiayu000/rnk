//! Diff algorithm for VNode trees
//!
//! Compares old and new VNode trees to produce minimal patches.
//! Uses a simplified algorithm optimized for typical UI patterns.

use crate::core::{NodeKey, Props, VNode};
use crate::reconciler::ReconcilePlanError;

use super::plan::{plan_child_patches, plan_diff};

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
///
/// # Errors
///
/// Returns [`ReconcilePlanError`] when either tree has invalid identity
/// metadata, duplicate sibling keys, or a non-bijective final-order plan.
pub fn try_diff(old: &VNode, new: &VNode) -> Result<Vec<Patch>, ReconcilePlanError> {
    plan_diff(old, new).map(|plan| plan.into_patches())
}

/// Legacy adapter for [`try_diff`].
///
/// # Panics
///
/// Panics when checked reconciliation planning fails. Use [`try_diff`] to
/// handle invalid identity metadata or sibling order explicitly.
pub fn diff(old: &VNode, new: &VNode) -> Vec<Patch> {
    try_diff(old, new).unwrap_or_else(|error| panic!("reconciliation planning failed: {error}"))
}

/// Checked child-list diff. No patches are returned when validation fails.
///
/// The existing signature supplies only a sibling-local `parent_key`, so its
/// patch addresses remain sibling-local too. Applying this partial diff to a
/// whole layout tree can therefore return a checked ambiguity error when the
/// same raw address exists under multiple scopes. Use [`try_diff`] when the
/// complete old/new trees are available; it emits scoped internal addresses
/// without guessing a parent scope.
///
/// # Errors
///
/// Returns [`ReconcilePlanError`] when either sibling list has invalid or
/// duplicate identity metadata, or cannot produce one bijective final order.
pub fn try_diff_children(
    old_children: &[VNode],
    new_children: &[VNode],
    parent_key: NodeKey,
) -> Result<Vec<Patch>, ReconcilePlanError> {
    plan_child_patches(old_children, new_children, parent_key)
}

/// Legacy child-list adapter.
///
/// The destination is extended only after the complete checked result exists.
/// Invalid input therefore fails loudly without leaving partial patches.
///
/// # Panics
///
/// Panics when [`try_diff_children`] would return an error. Use that checked
/// function to preserve the destination and handle the failure explicitly.
pub fn diff_children(
    old_children: &[VNode],
    new_children: &[VNode],
    parent_key: NodeKey,
    patches: &mut Vec<Patch>,
) {
    let checked = try_diff_children(old_children, new_children, parent_key)
        .unwrap_or_else(|error| panic!("child reconciliation planning failed: {error}"));
    patches.extend(checked);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::VNode;
    use crate::reconciler::SiblingIdentity;

    #[test]
    fn identical_tree_has_empty_deterministic_plan() {
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
        assert!(patches.is_empty(), "{patches:?}");
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
    fn same_key_incompatible_type_is_replace() {
        let old = VNode::box_node().child(VNode::box_node().with_key("stable"));
        let new = VNode::box_node().child(VNode::text("Replaced").with_key("stable"));

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Replace { .. }));
    }

    #[test]
    fn keyed_match_ignores_position_within_parent() {
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
    fn property_mixed_key_permutations_are_bijective_or_typed_error() {
        super::super::error::property_tests::run_bounded_mixed_key_property();
    }

    #[test]
    fn a_pure_append_needs_no_reorder() {
        // Creating a node appends it, so this order is already correct and
        // restating it would be pointless work every frame.
        let patches = apply_order(&["a", "b"], &["a", "b", "c"]);
        assert_eq!(final_order(&patches), None, "{patches:?}");
    }

    #[test]
    fn plan_contains_total_hole_free_final_order() {
        let patches = apply_order(&["b", "c"], &["a", "b", "c"]);
        let order = final_order(&patches).expect("front create needs exact target order");
        assert_eq!(order.len(), 3);
        assert_eq!(
            order.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
    }

    #[test]
    fn a_trailing_removal_needs_no_reorder() {
        let patches = apply_order(&["a", "b", "c"], &["a", "b"]);
        assert_eq!(final_order(&patches), None, "{patches:?}");
    }

    #[test]
    fn try_diff_children_duplicate_returns_error_without_patches() {
        let parent_key = VNode::box_node().key;
        let old = parent_with(parent_key, &["a"]);
        let new = parent_with(parent_key, &["a", "a"]);
        let failure = try_diff_children(&old.children, &new.children, parent_key)
            .expect_err("duplicate sibling key must fail");

        assert!(matches!(
            failure,
            ReconcilePlanError::DuplicateSiblingKey {
                first_index: 0,
                second_index: 1,
                ..
            }
        ));
    }

    #[test]
    fn mixed_keyed_unkeyed_keeps_positional_contract() {
        // Unchanged public semantics: without a key, position is identity.
        let parent_key = VNode::box_node().key;
        let mut before = VNode::box_node();
        before.key = parent_key;
        let before = before
            .child(VNode::text("a").with_key("a"))
            .child(VNode::text("x"))
            .child(VNode::text("b").with_key("b"));

        let mut after = VNode::box_node();
        after.key = parent_key;
        let after = after
            .child(VNode::text("b").with_key("b"))
            .child(VNode::text("x"))
            .child(VNode::text("a").with_key("a"));

        let patches = diff(&before, &after);
        assert!(
            patches
                .iter()
                .any(|patch| matches!(patch, Patch::Reorder { .. }))
        );
        assert!(patches.iter().all(|patch| !matches!(
            patch,
            Patch::Create { .. } | Patch::Remove { .. } | Patch::Replace { .. }
        )));
    }

    #[test]
    fn test_patch_creation() {
        let node = VNode::text("Test");
        let parent = NodeKey::root();

        let patch = Patch::create(node.clone(), parent);
        assert!(matches!(patch, Patch::Create { parent: p, .. } if p == parent));
    }

    #[test]
    fn vnode_key_metadata_decision_table() {
        let old = VNode::box_node().children([
            VNode::text("a").with_props(Props::new().key("a")),
            VNode::text("b").with_props(Props::new().key("b")),
        ]);
        let new = VNode::box_node().children([
            VNode::text("b").with_props(Props::new().key("b")),
            VNode::text("a").with_props(Props::new().key("a")),
        ]);

        let patches = try_diff(&old, &new).expect("props-only exact keys are valid");

        assert!(
            patches
                .iter()
                .all(|patch| !matches!(patch, Patch::Create { .. } | Patch::Remove { .. }))
        );
        assert!(
            patches
                .iter()
                .any(|patch| matches!(patch, Patch::Reorder { .. }))
        );

        let exact_with_metadata = VNode::text("x")
            .with_key("exact")
            .with_props(Props::new().key("exact"));
        let props_only = VNode::text("x").with_props(Props::new().key("exact"));
        assert!(
            try_diff(
                &VNode::box_node().child(exact_with_metadata),
                &VNode::box_node().child(props_only)
            )
            .expect("both exact metadata forms have one canonical source")
            .is_empty()
        );
        assert!(
            try_diff(
                &VNode::box_node().child(VNode::text("x").with_key("opaque")),
                &VNode::box_node().child(VNode::text("x").with_key("opaque"))
            )
            .expect("opaque metadata is valid")
            .is_empty()
        );
        assert!(
            try_diff(
                &VNode::box_node().child(VNode::text("x")),
                &VNode::box_node().child(VNode::text("x"))
            )
            .expect("missing key metadata is positional")
            .is_empty()
        );
    }

    #[test]
    fn mismatched_key_metadata_and_type_are_typed_errors() {
        let mismatched_token = VNode::text("x")
            .with_key("opaque")
            .with_props(Props::new().key("exact"));
        let token_error = try_diff(
            &VNode::box_node(),
            &VNode::box_node().child(mismatched_token),
        )
        .expect_err("mismatched exact/token metadata must fail");
        assert!(matches!(
            token_error,
            ReconcilePlanError::KeyMetadataMismatch { .. }
        ));

        let mut mismatched_type = VNode::text("x");
        mismatched_type.key.type_id = VNode::box_node().node_type.type_id();
        let type_error = try_diff(
            &VNode::box_node(),
            &VNode::box_node().child(mismatched_type),
        )
        .expect_err("mismatched key type metadata must fail");
        assert!(matches!(
            type_error,
            ReconcilePlanError::KeyTypeMismatch { .. }
        ));
    }

    #[test]
    fn opaque_token_collision_is_error() {
        let invalid = VNode::box_node()
            .child(VNode::box_node().with_key("duplicate"))
            .child(VNode::text("x").with_key("duplicate"));

        assert!(matches!(
            try_diff(&VNode::box_node(), &invalid),
            Err(ReconcilePlanError::DuplicateSiblingKey { .. })
        ));
    }

    #[test]
    fn raw_hash_collision_never_aliases_exact_keys() {
        let target = VNode::box_node().children([
            VNode::text("a").with_props(Props::new().key("a")),
            VNode::text("b").with_props(Props::new().key("b")),
        ]);

        let failure = super::super::plan::plan_diff_with_token_source(
            &VNode::box_node(),
            &target,
            &super::super::plan::constant_token,
        )
        .expect_err("distinct exact strings with one projection token must fail closed");

        assert!(matches!(
            failure,
            ReconcilePlanError::KeyTokenCollision {
                first_index: 0,
                second_index: 1,
                ..
            }
        ));
    }

    fn opaque_text(token: u64) -> VNode {
        let mut node = VNode::text("same");
        node.key.user_key = Some(token);
        node
    }

    #[test]
    fn exact_to_opaque_same_token_is_one_replace() {
        let old = VNode::box_node()
            .child(VNode::text("same").with_props(Props::new().key("exact-source")));
        let new = VNode::box_node().child(opaque_text(7));

        let plan = super::super::plan::plan_diff_with_token_source(
            &old,
            &new,
            &super::super::plan::constant_token,
        )
        .expect("one-to-one exact-to-opaque projection is unambiguous");
        let patches = plan.patches();

        assert_eq!(
            patches
                .iter()
                .filter(|patch| matches!(patch, Patch::Replace { .. }))
                .count(),
            1,
            "{patches:?}"
        );
        assert!(
            patches
                .iter()
                .all(|patch| !matches!(patch, Patch::Create { .. } | Patch::Remove { .. })),
            "a source-domain conversion must be directly applicable: {patches:?}"
        );
    }

    #[test]
    fn opaque_to_exact_same_token_is_one_replace() {
        let old = VNode::box_node().child(opaque_text(7));
        let new = VNode::box_node()
            .child(VNode::text("same").with_props(Props::new().key("exact-source")));

        let plan = super::super::plan::plan_diff_with_token_source(
            &old,
            &new,
            &super::super::plan::constant_token,
        )
        .expect("one-to-one opaque-to-exact projection is unambiguous");
        let patches = plan.patches();

        assert_eq!(
            patches
                .iter()
                .filter(|patch| matches!(patch, Patch::Replace { .. }))
                .count(),
            1,
            "{patches:?}"
        );
        assert!(
            patches
                .iter()
                .all(|patch| !matches!(patch, Patch::Create { .. } | Patch::Remove { .. })),
            "a source-domain conversion must be directly applicable: {patches:?}"
        );
    }

    #[test]
    fn distinct_exact_keys_with_one_cross_frame_token_are_typed_collision() {
        let old =
            VNode::box_node().child(VNode::text("same").with_props(Props::new().key("old-exact")));
        let new =
            VNode::box_node().child(VNode::text("same").with_props(Props::new().key("new-exact")));

        let failure = super::super::plan::plan_diff_with_token_source(
            &old,
            &new,
            &super::super::plan::constant_token,
        )
        .expect_err("distinct exact sources must not alias through one token");

        assert!(matches!(
            failure,
            ReconcilePlanError::KeyTokenCollision { token: 7, .. }
        ));
    }

    #[test]
    fn actual_child_position_overrides_stale_public_index() {
        let mut old_child = VNode::text("same");
        old_child.key.index = 99;
        let mut new_child = VNode::text("same");
        new_child.key.index = 42;

        let patches = try_diff(
            &VNode::box_node().child(old_child),
            &VNode::box_node().child(new_child),
        )
        .expect("stale public indices are compatibility metadata");

        assert!(patches.is_empty(), "{patches:?}");
    }

    #[test]
    fn empty_key_is_keyed_and_duplicate_is_error() {
        let single = VNode::box_node().child(VNode::text("x").with_props(Props::new().key("")));
        assert!(
            try_diff(&single, &single)
                .expect("empty exact key is valid")
                .is_empty()
        );
        let duplicate = VNode::box_node().children([
            VNode::text("a").with_props(Props::new().key("")),
            VNode::text("b").with_props(Props::new().key("")),
        ]);
        assert!(matches!(
            try_diff(&VNode::box_node(), &duplicate),
            Err(ReconcilePlanError::DuplicateSiblingKey {
                key_kind: crate::reconciler::IdentityKeyKind::Exact,
                ..
            })
        ));
    }

    #[test]
    fn discarded_plan_mutates_no_engine_state() {
        let old = VNode::box_node().child(VNode::text("old").with_key("stable"));
        let new = VNode::box_node().child(VNode::text("new").with_key("stable"));
        let mut engine = crate::layout::LayoutEngine::new();
        engine.compute_vnode(&old, 20, 4);
        let before_count = engine.node_count();
        let before_layout = engine.get_vnode_layout(old.children[0].key);

        let _discarded = try_diff(&old, &new).expect("planning succeeds");

        assert_eq!(engine.node_count(), before_count);
        assert_eq!(
            engine
                .get_vnode_layout(old.children[0].key)
                .map(|layout| (layout.width, layout.height)),
            before_layout.map(|layout| (layout.width, layout.height))
        );
    }

    #[test]
    fn try_diff_invalid_nested_metadata_returns_error_without_partial_patches() {
        let invalid = VNode::box_node().child(
            VNode::box_node().with_key("branch").child(
                VNode::text("x")
                    .with_key("opaque")
                    .with_props(Props::new().key("exact")),
            ),
        );
        assert!(matches!(
            try_diff(&VNode::box_node(), &invalid),
            Err(ReconcilePlanError::KeyMetadataMismatch { .. })
        ));
    }

    #[test]
    fn legacy_diff_fails_loudly_on_invalid_input() {
        let invalid = VNode::box_node().children([
            VNode::text("a").with_key("duplicate"),
            VNode::text("b").with_key("duplicate"),
        ]);
        assert!(std::panic::catch_unwind(|| diff(&VNode::box_node(), &invalid)).is_err());
    }

    #[test]
    fn legacy_diff_children_fails_loudly_without_mutating_destination() {
        let parent = VNode::box_node().key;
        let mut destination = vec![Patch::remove(NodeKey::root())];
        let invalid = [
            VNode::text("a").with_key("duplicate"),
            VNode::text("b").with_key("duplicate"),
        ];

        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            diff_children(&[], &invalid, parent, &mut destination);
        }));

        assert!(failure.is_err());
        assert_eq!(destination.len(), 1);
        assert!(matches!(destination[0], Patch::Remove { .. }));
    }

    #[test]
    fn try_diff_accepts_public_box_root() {
        assert!(
            try_diff(&VNode::box_node(), &VNode::box_node())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn try_diff_accepts_public_text_root() {
        assert!(
            try_diff(&VNode::text("x"), &VNode::text("x"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn try_diff_accepts_public_component_root() {
        struct Component;
        assert!(
            try_diff(
                &VNode::component::<Component>(),
                &VNode::component::<Component>()
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn legacy_diff_accepts_public_non_container_roots() {
        struct Component;
        assert!(diff(&VNode::text("x"), &VNode::text("x")).is_empty());
        assert!(
            diff(
                &VNode::component::<Component>(),
                &VNode::component::<Component>()
            )
            .is_empty()
        );
    }

    #[test]
    fn planned_children_and_parent_final_order_cannot_diverge() {
        let mut plan = super::super::plan::plan_initial_tree(
            &VNode::box_node().child(VNode::text("child").with_key("child")),
        )
        .expect("fixture has a valid plan");
        plan.root.children.clear();

        plan.validate_final_orders()
            .expect_err("the executable child tree and declared final order must agree");
    }

    #[test]
    fn old_and_new_projection_collision_is_rejected_by_the_pure_plan() {
        use std::any::TypeId;

        let old_plan = super::super::plan::plan_initial_tree(
            &VNode::box_node().child(VNode::text("old").with_key("old")),
        )
        .expect("old fixture has valid structural identities");
        let new_plan = super::super::plan::plan_initial_tree(
            &VNode::box_node().child(VNode::text("new").with_key("new")),
        )
        .expect("new fixture has valid structural identities");
        let root_projection = SiblingIdentity::Keyed {
            user_key: 1,
            type_id: TypeId::of::<u8>(),
        };
        let forced_projection = SiblingIdentity::Keyed {
            user_key: 7,
            type_id: TypeId::of::<u8>(),
        };

        let failure = new_plan
            .validate_composite_projection_union_with(&old_plan, &|planned| {
                if planned.identity == super::super::ScopedNodeIdentity::Root {
                    root_projection
                } else {
                    forced_projection
                }
            })
            .expect_err("old and new exact scopes must not share a transient patch address");
        assert!(matches!(
            failure,
            ReconcilePlanError::CompositeIdentityCollision { identity, .. }
                if identity == forced_projection
        ));
    }
}
