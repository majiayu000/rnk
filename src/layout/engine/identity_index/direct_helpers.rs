//! Focused helpers for direct raw-patch virtual-tree simulation.

use std::collections::{HashMap, HashSet};

use crate::core::{NodeKey, VNode};
use crate::reconciler::{
    PlannedNode, ReconcilePlanError, ScopedIdentityArena, ScopedNodeIdentity, SiblingIdentity,
    compatibility_token_for_exact, resolve_child_identity,
};

use super::VirtualNodeEntry;

/// Associates every raw scoped patch address with the same virtual node while
/// earlier operations change positional identities around it.
#[derive(Clone, Default)]
pub(super) struct VirtualAliases {
    paths: HashMap<SiblingIdentity, Vec<usize>>,
    stable_identities: HashMap<ScopedNodeIdentity, (Vec<usize>, NodeKey, NodeKey)>,
    positional_origins: Vec<(Vec<usize>, usize)>,
    batch_local_raw_paths: Vec<(SiblingIdentity, Vec<usize>)>,
}

impl VirtualAliases {
    pub(super) fn from_index(index: &[VirtualNodeEntry]) -> Self {
        let mut aliases = Self::default();
        aliases.register_current(index);
        aliases.positional_origins = index
            .iter()
            .filter(|entry| !entry.path.is_empty() && entry.legacy_key.user_key.is_none())
            .map(|entry| (entry.path.clone(), entry.legacy_key.index))
            .collect();
        aliases
    }

    pub(super) fn register_current(&mut self, index: &[VirtualNodeEntry]) {
        for entry in index {
            let existing_node = self
                .stable_identities
                .values()
                .any(|(path, _, _)| path == &entry.path);
            if existing_node {
                continue;
            }
            self.stable_identities.insert(
                entry.identity.clone(),
                (entry.path.clone(), entry.legacy_key, entry.raw_key),
            );
            self.paths
                .entry(
                    entry
                        .identity
                        .scoped_patch_address(entry.legacy_key)
                        .identity(),
                )
                .or_insert_with(|| entry.path.clone());
        }
    }

    pub(super) fn path_for(&self, address: SiblingIdentity) -> Option<&[usize]> {
        self.paths.get(&address).map(Vec::as_slice)
    }

    pub(super) fn matching_paths(&self, key: NodeKey, parent: &[usize]) -> Vec<Vec<usize>> {
        self.stable_identities
            .values()
            .filter(|(path, legacy, raw)| {
                path.len() == parent.len() + 1
                    && path.starts_with(parent)
                    && (legacy.identity() == key.identity() || raw.identity() == key.identity())
            })
            .map(|(path, _, _)| path.clone())
            .collect()
    }

    pub(super) fn batch_local_paths(&self, key: NodeKey, parent: &[usize]) -> Vec<Vec<usize>> {
        self.batch_local_raw_paths
            .iter()
            .filter(|(raw, path)| {
                *raw == key.identity() && path.len() == parent.len() + 1 && path.starts_with(parent)
            })
            .map(|(_, path)| path.clone())
            .collect()
    }

    pub(super) fn batch_local_paths_anywhere(&self, key: NodeKey) -> Vec<Vec<usize>> {
        self.batch_local_raw_paths
            .iter()
            .filter(|(raw, _)| *raw == key.identity())
            .map(|(_, path)| path.clone())
            .collect()
    }

    pub(super) fn batch_local_path_has_different_key(&self, path: &[usize], key: NodeKey) -> bool {
        self.batch_local_raw_paths
            .iter()
            .any(|(raw, candidate)| candidate == path && *raw != key.identity())
    }

    pub(super) fn positional_target_index(&self, path: &[usize]) -> Option<usize> {
        self.positional_origins
            .iter()
            .find_map(|(candidate, target_index)| (candidate == path).then_some(*target_index))
    }

    pub(super) fn batch_local_aliases_at(&self, path: &[usize]) -> Vec<SiblingIdentity> {
        self.batch_local_raw_paths
            .iter()
            .filter(|(_, candidate)| candidate == path)
            .map(|(raw, _)| *raw)
            .collect()
    }

    pub(super) fn register_new(
        &mut self,
        identity: ScopedNodeIdentity,
        legacy_key: NodeKey,
        raw_key: NodeKey,
        path: Vec<usize>,
    ) {
        self.paths
            .entry(identity.scoped_patch_address(legacy_key).identity())
            .or_insert_with(|| path.clone());
        self.stable_identities
            .entry(identity)
            .or_insert_with(|| (path.clone(), legacy_key, raw_key));
        self.batch_local_raw_paths.push((raw_key.identity(), path));
    }

    pub(super) fn register_new_subtree(
        &mut self,
        index: &[VirtualNodeEntry],
        root_path: &[usize],
        payload: &VNode,
    ) {
        for entry in index
            .iter()
            .filter(|entry| entry.path.starts_with(root_path))
        {
            let payload_node = vnode_at(payload, &entry.path[root_path.len()..]);
            self.register_new(
                entry.identity.clone(),
                entry.legacy_key,
                payload_node.key,
                entry.path.clone(),
            );
            if !entry.path.is_empty()
                && entry.legacy_key.user_key.is_none()
                && !self
                    .positional_origins
                    .iter()
                    .any(|(path, _)| path == &entry.path)
            {
                self.positional_origins
                    .push((entry.path.clone(), payload_node.key.index));
            }
        }
    }

    pub(super) fn addresses_at(&self, target: &[usize]) -> Vec<SiblingIdentity> {
        self.paths
            .iter()
            .filter(|(_, path)| path.as_slice() == target)
            .map(|(address, _)| *address)
            .collect()
    }

    pub(super) fn insert_child(&mut self, parent: &[usize], slot: usize) {
        for path in self.paths.values_mut() {
            if path.len() > parent.len() && path.starts_with(parent) && path[parent.len()] >= slot {
                path[parent.len()] += 1;
            }
        }
        for (path, _, _) in self.stable_identities.values_mut() {
            if path.len() > parent.len() && path.starts_with(parent) && path[parent.len()] >= slot {
                path[parent.len()] += 1;
            }
        }
        for (path, _) in &mut self.positional_origins {
            if path.len() > parent.len() && path.starts_with(parent) && path[parent.len()] >= slot {
                path[parent.len()] += 1;
            }
        }
        for (_, path) in &mut self.batch_local_raw_paths {
            if path.len() > parent.len() && path.starts_with(parent) && path[parent.len()] >= slot {
                path[parent.len()] += 1;
            }
        }
    }

    pub(super) fn remove_subtree(&mut self, removed: &[usize]) {
        let Some((&removed_slot, parent)) = removed.split_last() else {
            self.paths.clear();
            return;
        };
        self.paths.retain(|_, path| !path.starts_with(removed));
        self.stable_identities
            .retain(|_, (path, _, _)| !path.starts_with(removed));
        self.positional_origins
            .retain(|(path, _)| !path.starts_with(removed));
        self.batch_local_raw_paths
            .retain(|(_, path)| !path.starts_with(removed));
        for path in self.paths.values_mut() {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && path[parent.len()] > removed_slot
            {
                path[parent.len()] -= 1;
            }
        }
        for (path, _, _) in self.stable_identities.values_mut() {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && path[parent.len()] > removed_slot
            {
                path[parent.len()] -= 1;
            }
        }
        for (path, _) in &mut self.positional_origins {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && path[parent.len()] > removed_slot
            {
                path[parent.len()] -= 1;
            }
        }
        for (_, path) in &mut self.batch_local_raw_paths {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && path[parent.len()] > removed_slot
            {
                path[parent.len()] -= 1;
            }
        }
    }

    pub(super) fn replace_subtree(&mut self, replaced: &[usize]) {
        self.paths.retain(|_, path| !path.starts_with(replaced));
        self.stable_identities
            .retain(|_, (path, _, _)| !path.starts_with(replaced));
        self.positional_origins
            .retain(|(path, _)| !path.starts_with(replaced));
        self.batch_local_raw_paths
            .retain(|(_, path)| !path.starts_with(replaced));
    }

    pub(super) fn reorder_children(&mut self, parent: &[usize], old_order: &[usize]) {
        let new_slots: HashMap<_, _> = old_order
            .iter()
            .copied()
            .enumerate()
            .map(|(new_slot, old_slot)| (old_slot, new_slot))
            .collect();
        for path in self.paths.values_mut() {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && let Some(new_slot) = new_slots.get(&path[parent.len()])
            {
                path[parent.len()] = *new_slot;
            }
        }
        for (path, _, _) in self.stable_identities.values_mut() {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && let Some(new_slot) = new_slots.get(&path[parent.len()])
            {
                path[parent.len()] = *new_slot;
            }
        }
        for (path, _) in &mut self.positional_origins {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && let Some(new_slot) = new_slots.get(&path[parent.len()])
            {
                path[parent.len()] = *new_slot;
            }
        }
        for (_, path) in &mut self.batch_local_raw_paths {
            if path.len() > parent.len()
                && path.starts_with(parent)
                && let Some(new_slot) = new_slots.get(&path[parent.len()])
            {
                path[parent.len()] = *new_slot;
            }
        }
    }
}

pub(super) fn vnode_at<'a>(root: &'a VNode, path: &[usize]) -> &'a VNode {
    let mut node = root;
    for index in path {
        node = &node.children[*index];
    }
    node
}

pub(super) fn vnode_at_mut<'a>(root: &'a mut VNode, path: &[usize]) -> &'a mut VNode {
    let mut node = root;
    for index in path {
        node = &mut node.children[*index];
    }
    node
}

pub(super) fn normalize_child_indices(node: &mut VNode) {
    for (index, child) in node.children.iter_mut().enumerate() {
        child.key.index = index;
        normalize_child_indices(child);
    }
}

pub(super) fn canonical_payload_identity(
    index: &[VirtualNodeEntry],
    node: &VNode,
    actual_index: usize,
    parent: &ScopedNodeIdentity,
) -> Result<(ScopedNodeIdentity, NodeKey), ReconcilePlanError> {
    let mut arena = ScopedIdentityArena::seeded(index.iter().map(|entry| &entry.identity));
    resolve_child_identity(
        node,
        actual_index,
        parent,
        &compatibility_token_for_exact,
        &mut arena,
    )
    .map(|resolved| (resolved.scoped, resolved.legacy_key))
}

fn effective_child_key(node: &VNode, actual_index: usize) -> NodeKey {
    if let Some(exact) = node.props.key.as_deref() {
        NodeKey::with_key(exact, node.node_type.type_id(), actual_index)
    } else if let Some(token) = node.key.user_key {
        NodeKey {
            user_key: Some(token),
            type_id: node.node_type.type_id(),
            index: actual_index,
        }
    } else {
        NodeKey::new(node.node_type.type_id(), actual_index)
    }
}

pub(super) fn first_subtree_duplicate(node: &VNode) -> Option<NodeKey> {
    let mut siblings = HashSet::with_capacity(node.children.len());
    for (index, child) in node.children.iter().enumerate() {
        let key = effective_child_key(child, index);
        if key
            .user_key
            .is_some_and(|exact_token| !siblings.insert(exact_token))
        {
            return Some(key);
        }
        if let Some(duplicate) = first_subtree_duplicate(child) {
            return Some(duplicate);
        }
    }
    None
}

pub(super) fn sibling_collision(
    index: &[VirtualNodeEntry],
    parent: &ScopedNodeIdentity,
    excluded_path: Option<&[usize]>,
    key: NodeKey,
) -> Option<NodeKey> {
    key.user_key.and_then(|token| {
        index
            .iter()
            .find(|entry| {
                entry.identity.parent() == Some(parent)
                    && excluded_path != Some(entry.path.as_slice())
                    && entry.legacy_key.user_key == Some(token)
            })
            .map(|entry| entry.legacy_key)
    })
}

fn force_create_subtree(planned: &mut PlannedNode) {
    planned.action = crate::reconciler::PlannedNodeAction::Create;
    planned.old_identity = None;
    planned.mutations = Default::default();
    for child in &mut planned.children {
        force_create_subtree(child);
    }
}

pub(super) fn force_replace_subtree(
    planned: &mut PlannedNode,
    replacement: &ScopedNodeIdentity,
) -> bool {
    if &planned.identity == replacement {
        if planned.action != crate::reconciler::PlannedNodeAction::Create {
            planned.action = crate::reconciler::PlannedNodeAction::Replace;
            planned.mutations = Default::default();
            for child in &mut planned.children {
                force_create_subtree(child);
            }
        }
        return true;
    }
    planned
        .children
        .iter_mut()
        .any(|child| force_replace_subtree(child, replacement))
}
