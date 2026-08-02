//! Canonical sibling-local and scoped VNode identity.

use std::any::TypeId;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::core::{NodeKey, Props, VNode};

use super::{IdentityKeyKind, ReconcilePlanError};

/// Compatibility identity exposed by the pre-GH59 public surface.
///
/// This remains sibling-local and hash-only. Reconciliation and layout
/// correctness use an internal parent-scoped identity instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SiblingIdentity {
    Keyed { user_key: u64, type_id: TypeId },
    Positional { type_id: TypeId, index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum CanonicalKey {
    Exact(Arc<str>),
    Opaque(u64),
}

impl CanonicalKey {
    pub(crate) fn diagnostic_kind(&self) -> IdentityKeyKind {
        match self {
            Self::Exact(_) => IdentityKeyKind::Exact,
            Self::Opaque(_) => IdentityKeyKind::Opaque,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SiblingMatchKey {
    Keyed(CanonicalKey),
    Positional(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ScopedIdentitySegment {
    Keyed { key: CanonicalKey, type_id: TypeId },
    Positional { type_id: TypeId, index: usize },
}

/// Exact identity of a node in one logical tree.
///
/// Parent links are shared and identities built during one operation are
/// interned by [`ScopedIdentityArena`]. Hashing is constant-time through the
/// cached hash. Equality uses pointer identity on that normal path, with an
/// iterative exact fallback so independently built arenas and forced hash
/// collisions preserve the logical `Eq`/`Hash` contract.
#[derive(Debug, Clone)]
pub(crate) enum ScopedNodeIdentity {
    Root,
    Child(Arc<ScopedIdentityNode>),
}

#[derive(Debug)]
pub(crate) struct ScopedIdentityNode {
    parent: ScopedNodeIdentity,
    segment: ScopedIdentitySegment,
    cached_hash: u64,
    depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ScopedIdentityArenaKey {
    parent: ScopedNodeIdentity,
    segment: ScopedIdentitySegment,
}

/// Invocation-local interner for exact scoped identities.
///
/// The arena is deliberately owned by one planning/apply operation. It can be
/// seeded from an engine's current identity keys, which lets surviving paths
/// retain their existing `Arc` handles without global or thread-local state.
#[derive(Debug, Default)]
pub(crate) struct ScopedIdentityArena {
    children: HashMap<ScopedIdentityArenaKey, ScopedNodeIdentity>,
    #[cfg(test)]
    intern_calls: usize,
}

impl ScopedIdentityArena {
    pub(crate) fn seeded<'a>(identities: impl IntoIterator<Item = &'a ScopedNodeIdentity>) -> Self {
        let mut arena = Self::default();
        for identity in identities {
            if let ScopedNodeIdentity::Child(node) = identity {
                arena.children.insert(
                    ScopedIdentityArenaKey {
                        parent: node.parent.clone(),
                        segment: node.segment.clone(),
                    },
                    identity.clone(),
                );
            }
        }
        arena
    }

    pub(crate) fn child(
        &mut self,
        parent: &ScopedNodeIdentity,
        segment: ScopedIdentitySegment,
    ) -> ScopedNodeIdentity {
        #[cfg(test)]
        {
            self.intern_calls += 1;
        }
        let key = ScopedIdentityArenaKey {
            parent: parent.clone(),
            segment,
        };
        if let Some(identity) = self.children.get(&key) {
            return identity.clone();
        }
        let identity = ScopedNodeIdentity::Child(Arc::new(ScopedIdentityNode {
            cached_hash: child_scope_hash(parent.cached_hash(), &key.segment),
            depth: parent.depth() + 1,
            parent: key.parent.clone(),
            segment: key.segment.clone(),
        }));
        self.children.insert(key, identity.clone());
        identity
    }
}

impl ScopedNodeIdentity {
    pub(crate) fn parent(&self) -> Option<&ScopedNodeIdentity> {
        match self {
            Self::Root => None,
            Self::Child(node) => Some(&node.parent),
        }
    }

    pub(crate) fn diagnostic(&self) -> String {
        format!("scope:{:016x}", self.cached_hash())
    }

    pub(crate) fn composite_identity(&self, legacy_key: NodeKey) -> SiblingIdentity {
        SiblingIdentity::Keyed {
            user_key: self.composite_user_key(legacy_key),
            type_id: TypeId::of::<ScopedCompositeIdentityMarker>(),
        }
    }

    fn composite_user_key(&self, legacy_key: NodeKey) -> u64 {
        let mut hasher = DefaultHasher::new();
        "rnk-scoped-node-identity-v1".hash(&mut hasher);
        self.cached_hash().hash(&mut hasher);
        legacy_key.type_id.hash(&mut hasher);
        hasher.finish()
    }

    pub(crate) fn scoped_patch_address(&self, legacy_key: NodeKey) -> NodeKey {
        NodeKey {
            user_key: Some(self.composite_user_key(legacy_key)),
            type_id: TypeId::of::<ScopedPatchAddressMarker>(),
            index: legacy_key.index,
        }
    }

    pub(crate) fn is_scoped_patch_address(key: NodeKey) -> bool {
        key.user_key.is_some() && key.type_id == TypeId::of::<ScopedPatchAddressMarker>()
    }

    fn cached_hash(&self) -> u64 {
        match self {
            Self::Root => ROOT_SCOPE_HASH,
            Self::Child(node) => node.cached_hash,
        }
    }

    fn depth(&self) -> usize {
        match self {
            Self::Root => 0,
            Self::Child(node) => node.depth,
        }
    }
}

pub(crate) fn insert_composite_projection(
    projections: &mut HashMap<SiblingIdentity, ScopedNodeIdentity>,
    identity: &ScopedNodeIdentity,
    legacy_key: NodeKey,
) -> Result<(), (SiblingIdentity, ScopedNodeIdentity)> {
    let projected = identity.composite_identity(legacy_key);
    if let Some(first_scope) = projections.get(&projected)
        && first_scope != identity
    {
        return Err((projected, first_scope.clone()));
    }
    projections.insert(projected, identity.clone());
    Ok(())
}

impl PartialEq for ScopedNodeIdentity {
    fn eq(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self, other);
        loop {
            match (left, right) {
                (Self::Root, Self::Root) => return true,
                (Self::Child(left_node), Self::Child(right_node)) => {
                    if Arc::ptr_eq(left_node, right_node) {
                        return true;
                    }
                    if left_node.cached_hash != right_node.cached_hash
                        || left_node.depth != right_node.depth
                        || left_node.segment != right_node.segment
                    {
                        return false;
                    }
                    left = &left_node.parent;
                    right = &right_node.parent;
                }
                _ => return false,
            }
        }
    }
}

impl Eq for ScopedNodeIdentity {}

impl Hash for ScopedNodeIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.cached_hash());
    }
}

struct ScopedPatchAddressMarker;
struct ScopedCompositeIdentityMarker;

const ROOT_SCOPE_HASH: u64 = 0x726e_6b2d_726f_6f74;

#[derive(Debug, Clone)]
pub(crate) struct ResolvedNodeIdentity {
    pub(crate) scoped: ScopedNodeIdentity,
    pub(crate) legacy_key: NodeKey,
    pub(crate) kind: ResolvedIdentityKind,
}

#[derive(Debug, Clone)]
pub(crate) enum ResolvedIdentityKind {
    Keyed {
        canonical_key: CanonicalKey,
        compatibility_token: u64,
    },
    Positional {
        index: usize,
    },
}

impl ResolvedNodeIdentity {
    pub(crate) fn match_key(&self) -> SiblingMatchKey {
        match &self.kind {
            ResolvedIdentityKind::Keyed { canonical_key, .. } => {
                SiblingMatchKey::Keyed(canonical_key.clone())
            }
            ResolvedIdentityKind::Positional { index } => SiblingMatchKey::Positional(*index),
        }
    }

    pub(crate) fn compatibility_token(&self) -> Option<u64> {
        self.canonical_projection().map(|(token, _)| token)
    }

    pub(crate) fn canonical_projection(&self) -> Option<(u64, &CanonicalKey)> {
        match &self.kind {
            ResolvedIdentityKind::Keyed {
                compatibility_token,
                canonical_key,
            } => Some((*compatibility_token, canonical_key)),
            ResolvedIdentityKind::Positional { .. } => None,
        }
    }

    pub(crate) fn canonical_key(&self) -> Option<&CanonicalKey> {
        self.canonical_projection().map(|(_, key)| key)
    }
}

pub(crate) fn resolve_child_identity(
    vnode: &VNode,
    actual_index: usize,
    parent: &ScopedNodeIdentity,
    token_source: &impl Fn(&str) -> u64,
    arena: &mut ScopedIdentityArena,
) -> Result<ResolvedNodeIdentity, ReconcilePlanError> {
    let vnode_type = vnode.node_type.type_id();
    if vnode.key.type_id != vnode_type {
        return Err(ReconcilePlanError::KeyTypeMismatch {
            parent_scope: parent.diagnostic(),
            index: actual_index,
            key_type: vnode.key.type_id,
            vnode_type,
        });
    }

    let (kind, segment, legacy_key) = match (&vnode.props.key, vnode.key.user_key) {
        (Some(exact), actual_token) => {
            let expected_token = token_source(exact);
            if let Some(actual_token) = actual_token
                && actual_token != expected_token
            {
                return Err(ReconcilePlanError::KeyMetadataMismatch {
                    parent_scope: parent.diagnostic(),
                    index: actual_index,
                    expected_token,
                    actual_token,
                });
            }
            let key = CanonicalKey::Exact(Arc::from(exact.as_str()));
            (
                ResolvedIdentityKind::Keyed {
                    canonical_key: key.clone(),
                    compatibility_token: expected_token,
                },
                ScopedIdentitySegment::Keyed {
                    key: key.clone(),
                    type_id: vnode_type,
                },
                NodeKey {
                    user_key: Some(expected_token),
                    type_id: vnode_type,
                    index: actual_index,
                },
            )
        }
        (None, Some(token)) => {
            let key = CanonicalKey::Opaque(token);
            (
                ResolvedIdentityKind::Keyed {
                    canonical_key: key.clone(),
                    compatibility_token: token,
                },
                ScopedIdentitySegment::Keyed {
                    key: key.clone(),
                    type_id: vnode_type,
                },
                NodeKey {
                    user_key: Some(token),
                    type_id: vnode_type,
                    index: actual_index,
                },
            )
        }
        (None, None) => (
            ResolvedIdentityKind::Positional {
                index: actual_index,
            },
            ScopedIdentitySegment::Positional {
                type_id: vnode_type,
                index: actual_index,
            },
            NodeKey::new(vnode_type, actual_index),
        ),
    };

    Ok(ResolvedNodeIdentity {
        scoped: arena.child(parent, segment),
        legacy_key,
        kind,
    })
}

pub(crate) fn compatibility_token_for_exact(value: &str) -> u64 {
    NodeKey::compatibility_token(value)
}

pub(crate) fn semantically_equal_vnode_in(
    left: &VNode,
    right: &VNode,
    arena: &mut ScopedIdentityArena,
) -> Result<bool, ReconcilePlanError> {
    semantically_equal_node(
        left,
        right,
        &ScopedNodeIdentity::Root,
        &ScopedNodeIdentity::Root,
        true,
        arena,
    )
}

fn semantically_equal_node(
    left: &VNode,
    right: &VNode,
    left_scope: &ScopedNodeIdentity,
    right_scope: &ScopedNodeIdentity,
    is_root: bool,
    arena: &mut ScopedIdentityArena,
) -> Result<bool, ReconcilePlanError> {
    if left.node_type != right.node_type
        || if is_root {
            !props_equal_ignoring_key(&left.props, &right.props)
        } else {
            !left.props.semantically_eq(&right.props)
        }
        || left.children.len() != right.children.len()
    {
        return Ok(false);
    }

    for (index, (left_child, right_child)) in left.children.iter().zip(&right.children).enumerate()
    {
        let left_identity = resolve_child_identity(
            left_child,
            index,
            left_scope,
            &compatibility_token_for_exact,
            arena,
        )?;
        let right_identity = resolve_child_identity(
            right_child,
            index,
            right_scope,
            &compatibility_token_for_exact,
            arena,
        )?;
        if left_identity.match_key() != right_identity.match_key()
            || !semantically_equal_node(
                left_child,
                right_child,
                &left_identity.scoped,
                &right_identity.scoped,
                false,
                arena,
            )?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn props_equal_ignoring_key(left: &Props, right: &Props) -> bool {
    left.semantically_eq_ignoring_key(right)
}

fn child_scope_hash(parent_hash: u64, segment: &ScopedIdentitySegment) -> u64 {
    let mut hasher = DefaultHasher::new();
    "rnk-scoped-node-identity-structural-v1".hash(&mut hasher);
    parent_hash.hash(&mut hasher);
    segment.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
