//! The single definition of what makes two nodes the same node across frames.
//!
//! There used to be two. [`NodeKey`](crate::core::NodeKey) derives `Eq`/`Hash`
//! fieldwise, which includes `index`, and the child diff looked old nodes up by
//! that. `NodeKey::matches` meanwhile ignored `index` for keyed nodes. So a
//! keyed child that moved matched under one rule and missed under the other:
//! the diff saw an unknown node, created it, and removed the original — losing
//! the identity the key existed to preserve.
//!
//! Every identity decision now goes through [`SiblingIdentity`].

use std::any::TypeId;

/// Cross-frame identity of a node among its siblings.
///
/// A keyed node keeps its identity when it moves, so position is deliberately
/// absent. An unkeyed node has nothing but its position to be known by, so
/// position is all it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SiblingIdentity {
    /// Identified by the caller's key. Survives any move.
    Keyed { user_key: u64, type_id: TypeId },
    /// Identified by where it sits. Changes when it moves.
    Positional { type_id: TypeId, index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::NodeKey;

    #[test]
    fn a_keyed_node_keeps_its_identity_when_it_moves() {
        let type_id = TypeId::of::<u8>();
        let at_front = NodeKey::with_key("a", type_id, 0);
        let at_back = NodeKey::with_key("a", type_id, 7);

        assert_eq!(at_front.identity(), at_back.identity());
        assert!(at_front.matches(&at_back));
    }

    #[test]
    fn an_unkeyed_node_is_its_position() {
        let type_id = TypeId::of::<u8>();
        let first = NodeKey::new(type_id, 0);
        let second = NodeKey::new(type_id, 1);

        assert_ne!(first.identity(), second.identity());
        assert!(!first.matches(&second));
        assert_eq!(first.identity(), NodeKey::new(type_id, 0).identity());
    }

    #[test]
    fn a_key_never_matches_across_node_types() {
        let keyed = NodeKey::with_key("a", TypeId::of::<u8>(), 0);
        let other_type = NodeKey::with_key("a", TypeId::of::<u16>(), 0);

        assert_ne!(keyed.identity(), other_type.identity());
        assert!(!keyed.matches(&other_type));
    }

    #[test]
    fn keyed_and_unkeyed_are_never_the_same_node() {
        let type_id = TypeId::of::<u8>();
        assert!(!NodeKey::with_key("a", type_id, 0).matches(&NodeKey::new(type_id, 0)));
    }

    #[test]
    fn matching_is_exactly_identity_equality() {
        // The two must not drift apart again.
        let type_id = TypeId::of::<u8>();
        let keys = [
            NodeKey::with_key("a", type_id, 0),
            NodeKey::with_key("a", type_id, 3),
            NodeKey::with_key("b", type_id, 0),
            NodeKey::new(type_id, 0),
            NodeKey::new(type_id, 3),
            NodeKey::root(),
        ];

        for left in &keys {
            for right in &keys {
                assert_eq!(
                    left.matches(right),
                    left.identity() == right.identity(),
                    "{left:?} vs {right:?}"
                );
            }
        }
    }
}
