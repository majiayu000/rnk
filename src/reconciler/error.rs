//! Typed failures produced while validating or planning reconciliation.

use std::any::TypeId;
use std::error::Error;
use std::fmt;

use super::identity::SiblingIdentity;

/// Which canonical user-key domain caused an identity diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKeyKind {
    /// An exact `Props.key` string. Diagnostics expose only its compatibility
    /// token, never the string contents.
    Exact,
    /// An opaque token supplied through `VNode::with_key`/`NodeKey`.
    Opaque,
}

impl fmt::Display for IdentityKeyKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Exact => "exact key",
            Self::Opaque => "opaque key token",
        })
    }
}

/// A deterministic, pre-mutation reconciliation validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcilePlanError {
    KeyTypeMismatch {
        parent_scope: String,
        index: usize,
        key_type: TypeId,
        vnode_type: TypeId,
    },
    KeyMetadataMismatch {
        parent_scope: String,
        index: usize,
        expected_token: u64,
        actual_token: u64,
    },
    DuplicateSiblingKey {
        parent_scope: String,
        key_kind: IdentityKeyKind,
        token: u64,
        first_index: usize,
        second_index: usize,
    },
    KeyTokenCollision {
        parent_scope: String,
        token: u64,
        first_index: usize,
        second_index: usize,
    },
    DuplicateFinalIdentity {
        parent_scope: String,
        identity: String,
        first_index: usize,
        second_index: usize,
    },
    MissingFinalIdentity {
        parent_scope: String,
        identity: String,
    },
    DuplicateFinalIdentitySource {
        parent_scope: String,
        identity: String,
    },
    ExtraPlannedIdentity {
        parent_scope: String,
        identity: String,
    },
    DuplicateParentPlan {
        parent_scope: String,
    },
    MissingParentPlan {
        parent_scope: String,
    },
    ExtraParentPlan {
        parent_scope: String,
    },
    PlannedChildrenMismatch {
        parent_scope: String,
    },
    DuplicatePlannedIdentity {
        identity: String,
    },
    CompositeIdentityCollision {
        identity: SiblingIdentity,
        first_scope: String,
        second_scope: String,
    },
    MissingExistingNodeId {
        identity: String,
    },
    DuplicateExistingIdentityUse {
        identity: String,
    },
    DuplicateExistingNodeIdUse {
        first_identity: String,
        second_identity: String,
    },
    PreviousTreeMismatch,
    CommittedTreeMismatch {
        reason: &'static str,
    },
}

impl fmt::Display for ReconcilePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyTypeMismatch {
                parent_scope,
                index,
                key_type,
                vnode_type,
            } => write!(
                formatter,
                "key type metadata mismatch under {parent_scope} at sibling {index}: \
                 key type {key_type:?}, vnode type {vnode_type:?}"
            ),
            Self::KeyMetadataMismatch {
                parent_scope,
                index,
                expected_token,
                actual_token,
            } => write!(
                formatter,
                "exact key metadata mismatch under {parent_scope} at sibling {index}: \
                 expected token {expected_token:#018x}, got {actual_token:#018x}"
            ),
            Self::DuplicateSiblingKey {
                parent_scope,
                key_kind,
                token,
                first_index,
                second_index,
            } => write!(
                formatter,
                "duplicate sibling {key_kind} {token:#018x} under {parent_scope} \
                 at indices {first_index} and {second_index}"
            ),
            Self::KeyTokenCollision {
                parent_scope,
                token,
                first_index,
                second_index,
            } => write!(
                formatter,
                "distinct sibling key sources project to token {token:#018x} under \
                 {parent_scope} at indices {first_index} and {second_index}"
            ),
            Self::DuplicateFinalIdentity {
                parent_scope,
                identity,
                first_index,
                second_index,
            } => write!(
                formatter,
                "final child identity {identity} is duplicated under {parent_scope} \
                 at indices {first_index} and {second_index}"
            ),
            Self::MissingFinalIdentity {
                parent_scope,
                identity,
            } => write!(
                formatter,
                "final child identity {identity} under {parent_scope} has no survivor or create"
            ),
            Self::DuplicateFinalIdentitySource {
                parent_scope,
                identity,
            } => write!(
                formatter,
                "final child identity {identity} under {parent_scope} has multiple sources"
            ),
            Self::ExtraPlannedIdentity {
                parent_scope,
                identity,
            } => write!(
                formatter,
                "planned child identity {identity} under {parent_scope} is absent from final order"
            ),
            Self::DuplicateParentPlan { parent_scope } => {
                write!(
                    formatter,
                    "parent {parent_scope} has more than one final-order plan"
                )
            }
            Self::MissingParentPlan { parent_scope } => {
                write!(
                    formatter,
                    "planned node {parent_scope} has no final-order plan"
                )
            }
            Self::ExtraParentPlan { parent_scope } => {
                write!(
                    formatter,
                    "final-order plan for {parent_scope} has no planned node"
                )
            }
            Self::PlannedChildrenMismatch { parent_scope } => write!(
                formatter,
                "planned children for {parent_scope} differ from its exact final order"
            ),
            Self::DuplicatePlannedIdentity { identity } => {
                write!(
                    formatter,
                    "target plan contains scoped identity {identity} more than once"
                )
            }
            Self::CompositeIdentityCollision {
                identity,
                first_scope,
                second_scope,
            } => write!(
                formatter,
                "scoped identities {first_scope} and {second_scope} collide at compatibility \
                 projection {identity:?}"
            ),
            Self::MissingExistingNodeId { identity } => {
                write!(
                    formatter,
                    "committed identity {identity} has no layout node"
                )
            }
            Self::DuplicateExistingIdentityUse { identity } => write!(
                formatter,
                "committed identity {identity} is consumed more than once by one plan"
            ),
            Self::DuplicateExistingNodeIdUse {
                first_identity,
                second_identity,
            } => write!(
                formatter,
                "committed identities {first_identity} and {second_identity} resolve to one layout node"
            ),
            Self::PreviousTreeMismatch => formatter.write_str(
                "caller-provided previous VNode does not match the engine's committed tree",
            ),
            Self::CommittedTreeMismatch { reason } => {
                write!(formatter, "committed layout tree is inconsistent: {reason}")
            }
        }
    }
}

impl Error for ReconcilePlanError {}

#[cfg(test)]
pub(crate) mod property_tests {
    use std::collections::HashSet;

    use crate::core::{Dimension, Element, Props, VNode};
    use crate::layout::{Layout, LayoutEngine};
    use crate::reconciler::{ScopedNodeIdentity, plan_diff, try_diff};

    use super::ReconcilePlanError;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SiblingCase {
        Unkeyed,
        KeyA,
        KeyB,
    }

    fn bounded_lists() -> Vec<Vec<SiblingCase>> {
        let alphabet = [SiblingCase::Unkeyed, SiblingCase::KeyA, SiblingCase::KeyB];
        let mut lists = Vec::new();
        for length in 0usize..=3 {
            for mut encoded in 0..alphabet.len().pow(length as u32) {
                let mut list = Vec::with_capacity(length);
                for _ in 0..length {
                    list.push(alphabet[encoded % alphabet.len()]);
                    encoded /= alphabet.len();
                }
                lists.push(list);
            }
        }
        lists
    }

    fn has_unique_keys(list: &[SiblingCase]) -> bool {
        [SiblingCase::KeyA, SiblingCase::KeyB]
            .iter()
            .all(|key| list.iter().filter(|item| **item == *key).count() <= 1)
    }

    fn vnode(list: &[SiblingCase]) -> VNode {
        VNode::box_node().children(list.iter().map(|item| match item {
            SiblingCase::Unkeyed => VNode::box_node(),
            SiblingCase::KeyA => VNode::box_node().with_props(Props::new().key("a")),
            SiblingCase::KeyB => VNode::text("bbb").with_props(Props::new().key("b")),
        }))
    }

    fn element(list: &[SiblingCase]) -> Element {
        let mut root = Element::box_element();
        for item in list {
            let (mut child, width) = match item {
                SiblingCase::Unkeyed => (Element::box_element(), 1.0),
                SiblingCase::KeyA => (Element::box_element().with_key("a"), 2.0),
                SiblingCase::KeyB => (Element::text("bbb").with_key("b"), 3.0),
            };
            child.style.width = Dimension::Points(width);
            child.style.height = Dimension::Points(1.0);
            root.add_child(child);
        }
        root
    }

    fn tuple(layout: Layout) -> (f32, f32, f32, f32) {
        (layout.x, layout.y, layout.width, layout.height)
    }

    fn assert_layout_parity(node: &Element, left: &LayoutEngine, right: &LayoutEngine) {
        assert_eq!(
            left.get_layout(node.id).map(tuple),
            right.get_layout(node.id).map(tuple),
            "layout mismatch at {:?}",
            node.id
        );
        for child in &node.children {
            assert_layout_parity(child, left, right);
        }
    }

    fn assert_apply_parity(before: &[SiblingCase], after: &[SiblingCase]) {
        let old = element(before);
        let target = element(after);
        let mut incremental = LayoutEngine::new();
        let (previous, _) = incremental.compute_element_incremental(&old, None, 40, 8);
        let (_, outcome) = incremental
            .try_compute_element_incremental_checked(&target, Some(&previous), 40, 8)
            .unwrap_or_else(|error| {
                panic!("valid bounded case failed to apply: {before:?} -> {after:?}: {error}")
            });
        assert!(
            outcome.used_reconciler && !outcome.fallback_full_rebuild,
            "valid bounded case did not stay on the incremental path: {before:?} -> {after:?}: \
             {outcome:?}"
        );
        let mut rebuilt = LayoutEngine::new();
        rebuilt.compute_element_incremental(&target, None, 40, 8);
        assert_layout_parity(&target, &incremental, &rebuilt);
    }

    pub(crate) fn run_bounded_mixed_key_property() {
        let lists = bounded_lists();
        assert_eq!(lists.len(), 40);
        let mut checked_pairs = 0usize;
        for before in &lists {
            for after in &lists {
                checked_pairs += 1;
                let old = vnode(before);
                let new = vnode(after);
                if !has_unique_keys(before) || !has_unique_keys(after) {
                    assert!(matches!(
                        try_diff(&old, &new),
                        Err(ReconcilePlanError::DuplicateSiblingKey { .. })
                    ));
                    continue;
                }
                let first = plan_diff(&old, &new).unwrap_or_else(|error| {
                    panic!("valid bounded plan failed: {before:?} -> {after:?}: {error}")
                });
                let second = plan_diff(&old, &new).unwrap_or_else(|error| {
                    panic!("repeat bounded plan failed: {before:?} -> {after:?}: {error}")
                });
                assert_eq!(format!("{first:?}"), format!("{second:?}"));
                first.validate_final_orders().unwrap_or_else(|error| {
                    panic!("final order invalid: {before:?} -> {after:?}: {error}")
                });
                let final_children: Vec<_> = first
                    .root
                    .children
                    .iter()
                    .map(|child| child.identity.clone())
                    .collect();
                assert_eq!(
                    final_children.iter().cloned().collect::<HashSet<_>>().len(),
                    final_children.len()
                );
                let root_order = first
                    .parents
                    .iter()
                    .find(|parent| parent.parent == ScopedNodeIdentity::Root)
                    .map(|parent| &parent.final_children);
                assert_eq!(root_order, Some(&final_children));
                assert_apply_parity(before, after);
            }
        }
        assert_eq!(checked_pairs, 1_600);
    }
}
