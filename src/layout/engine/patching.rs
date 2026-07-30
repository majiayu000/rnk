//! Applying a batch of reconciler patches to the Taffy tree.
//!
//! Split out of `engine.rs`, which was at its size ceiling.

use std::collections::HashSet;

use taffy::NodeId;

use crate::core::{NodeKey, Props, VNode};
use crate::layout::TextFlowInput;
use crate::reconciler::Patch;

use super::patch_error::{PatchError, PatchFailure, PatchKind};
use super::text_flow_bridge::NodeContext;
use super::{LayoutEngine, normalized_taffy_style};

impl LayoutEngine {
    /// Apply a batch of patches, or none of them.
    ///
    /// Returns whether the tree changed. A rejected batch leaves this engine
    /// exactly as it was, so the caller can fall back to a full rebuild from
    /// the current tree; see [`try_apply_patches`](Self::try_apply_patches) for
    /// the reason it was rejected.
    pub fn apply_patches(&mut self, patches: &[Patch]) -> bool {
        self.try_apply_patches(patches).unwrap_or(false)
    }

    /// Apply a batch of patches transactionally.
    ///
    /// Every patch is applied to a staged copy. If any one of them fails, or
    /// layout does not converge afterwards, the copy is dropped and this engine
    /// is untouched — a batch used to be accepted whenever *any* patch in it
    /// succeeded, which left the tree describing neither the old VNode nor the
    /// new one, with nothing reporting it.
    pub fn try_apply_patches(&mut self, patches: &[Patch]) -> Result<bool, PatchError> {
        if patches.is_empty() {
            return Ok(false);
        }

        let mut candidate = self.staged_clone();
        candidate.apply_patches_only(patches)?;
        candidate.check_batch_postconditions(patches)?;
        candidate
            .run_layout_and_publish(&mut || false)
            .map_err(|_| {
                PatchError::new(
                    PatchKind::Update,
                    batch_key(patches),
                    PatchFailure::LayoutFailed,
                )
            })?;

        *self = candidate;
        Ok(true)
    }

    pub(super) fn apply_patches_only(&mut self, patches: &[Patch]) -> Result<(), PatchError> {
        for patch in patches {
            match patch {
                Patch::Create { node, parent, .. } => {
                    self.create_vnode(node, *parent)?;
                }
                Patch::Update { key, new_props, .. } => {
                    self.update_node_props(*key, new_props)?;
                }
                Patch::Remove { key } => {
                    self.remove_node(*key)?;
                }
                Patch::Replace { key, node, .. } => {
                    self.replace_node(*key, node)?;
                }
                Patch::Reorder { parent, order } => {
                    self.reorder_children(*parent, order)?;
                }
            }
        }

        Ok(())
    }

    /// Confirm the tree actually says what the batch asked for.
    ///
    /// Each helper reports its own failure, but only the batch as a whole can
    /// check that the results agree — that no mapping outlived its node, and
    /// that every stated child order is the order the tree now holds.
    pub(super) fn check_batch_postconditions(&self, patches: &[Patch]) -> Result<(), PatchError> {
        // No mapping may outlive its node. A stale entry would hand out a
        // NodeId that Taffy has already freed.
        if self
            .vnode_map
            .values()
            .any(|node_id| self.taffy.style(*node_id).is_err())
        {
            return Err(PatchError::new(
                PatchKind::Remove,
                batch_key(patches),
                PatchFailure::PostconditionViolated,
            ));
        }

        for patch in patches {
            let Patch::Reorder { parent, order } = patch else {
                continue;
            };
            let fail = || {
                PatchError::new(
                    PatchKind::Reorder,
                    *parent,
                    PatchFailure::PostconditionViolated,
                )
            };
            let &parent_id = self.vnode_map.get(&parent.identity()).ok_or_else(fail)?;
            let actual = self.taffy.children(parent_id).map_err(|_| fail())?;
            let expected: Option<Vec<NodeId>> = order
                .iter()
                .map(|key| self.vnode_map.get(&key.identity()).copied())
                .collect();
            if expected.as_deref() != Some(actual.as_slice()) {
                return Err(fail());
            }
        }

        Ok(())
    }

    /// Create a new node and add it to a parent
    fn create_vnode(&mut self, vnode: &VNode, parent_key: NodeKey) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Create, parent_key, failure);

        // Get parent node ID first (copy it to avoid borrow issues)
        let parent_node = *self
            .vnode_map
            .get(&parent_key.identity())
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;

        let new_node_id = self
            .build_vnode(vnode)
            .ok_or_else(|| fail(PatchFailure::BuildFailed))?;

        self.taffy
            .add_child(parent_node, new_node_id)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;

        Ok(())
    }

    /// Update a node's props/style
    fn update_node_props(&mut self, key: NodeKey, props: &Props) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Update, key, failure);

        let &node_id = self
            .vnode_map
            .get(&key.identity())
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;

        let is_text = self
            .taffy
            .get_node_context(node_id)
            .is_some_and(NodeContext::is_text);
        let new_style = normalized_taffy_style(&props.style, is_text);
        self.taffy
            .set_style(node_id, new_style)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;

        if is_text {
            let source = self
                .taffy
                .get_node_context(node_id)
                .and_then(NodeContext::input)
                .map(|input| input.source.clone())
                .unwrap_or_default();
            let input = TextFlowInput::plain(
                source,
                crate::layout::TextFlowSourceKind::Exact,
                props.style.clone(),
            );
            self.taffy
                .set_node_context(
                    node_id,
                    Some(NodeContext::new(Some(input), &self.text_flow_policy)),
                )
                .map_err(|_| fail(PatchFailure::TreeRejected))?;
        }

        Ok(())
    }

    /// Remove a node from the tree
    fn remove_node(&mut self, key: NodeKey) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Remove, key, failure);

        let &node_id = self
            .vnode_map
            .get(&key.identity())
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;

        // Walk the descendants before the node leaves the tree; afterwards
        // there is no way to reach them, and any mapping left behind would
        // point at a node Taffy has freed.
        let subtree = self.vnode_subtree_nodes(node_id);
        self.taffy
            .remove(node_id)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;
        self.purge_vnode_subtree(&subtree);
        Ok(())
    }

    /// Replace a node with a new one
    fn replace_node(&mut self, old_key: NodeKey, new_node: &VNode) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Replace, old_key, failure);

        let &old_node_id = self
            .vnode_map
            .get(&old_key.identity())
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;
        let parent_id = self
            .taffy
            .parent(old_node_id)
            .ok_or_else(|| fail(PatchFailure::MissingParent))?;

        let children: Vec<_> = self.taffy.children(parent_id).unwrap_or_default();
        let index = children.iter().position(|&id| id == old_node_id);

        let subtree = self.vnode_subtree_nodes(old_node_id);
        self.taffy
            .remove(old_node_id)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;
        self.purge_vnode_subtree(&subtree);

        let new_node_id = self
            .build_vnode(new_node)
            .ok_or_else(|| fail(PatchFailure::BuildFailed))?;

        match index {
            Some(idx) => self
                .taffy
                .insert_child_at_index(parent_id, idx, new_node_id)
                .map_err(|_| fail(PatchFailure::TreeRejected))?,
            None => self
                .taffy
                .add_child(parent_id, new_node_id)
                .map_err(|_| fail(PatchFailure::TreeRejected))?,
        }

        Ok(())
    }

    fn vnode_subtree_nodes(&self, root: NodeId) -> HashSet<NodeId> {
        let mut nodes = HashSet::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if nodes.insert(node) {
                pending.extend(
                    self.taffy
                        .children(node)
                        .expect("mapped VNode subtree must remain in the Taffy tree"),
                );
            }
        }
        nodes
    }

    fn purge_vnode_subtree(&mut self, subtree: &HashSet<NodeId>) {
        self.vnode_map.retain(|_, node| !subtree.contains(node));
        self.node_map.retain(|_, node| !subtree.contains(node));
        self.element_keys
            .retain(|element_id, _| self.node_map.contains_key(element_id));
        self.current_vnode_flows
            .retain(|key, _| self.vnode_map.contains_key(key));
        self.current_text_flows
            .retain(|element_id, _| self.node_map.contains_key(element_id));
    }

    /// Set a parent's Taffy children to exactly `order`.
    ///
    /// The patch names every child the parent should end up with, so the result
    /// is established rather than inferred. The previous version copied nodes
    /// between slots of the old child vector according to a move list, which
    /// could leave a node in two slots or drop one entirely, and said nothing
    /// about where a newly created sibling belonged.
    fn reorder_children(
        &mut self,
        parent_key: NodeKey,
        order: &[NodeKey],
    ) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Reorder, parent_key, failure);

        let &parent_id = self
            .vnode_map
            .get(&parent_key.identity())
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;

        let mut children = Vec::with_capacity(order.len());
        for key in order {
            // A key with no node means the plan disagrees with the tree.
            // Setting a partial order would silently drop children.
            let &node_id = self
                .vnode_map
                .get(&key.identity())
                .ok_or_else(|| fail(PatchFailure::UnknownNode))?;
            children.push(node_id);
        }

        self.taffy
            .set_children(parent_id, &children)
            .map_err(|_| fail(PatchFailure::TreeRejected))
    }
}

/// A key to attribute a whole-batch failure to.
///
/// Batch-level checks are not caused by one patch, so they borrow the first
/// patch's key rather than invent one.
fn batch_key(patches: &[Patch]) -> NodeKey {
    patches
        .first()
        .map(|patch| match patch {
            Patch::Create { parent, .. } | Patch::Reorder { parent, .. } => *parent,
            Patch::Update { key, .. } | Patch::Remove { key } | Patch::Replace { key, .. } => *key,
        })
        .unwrap_or_else(NodeKey::root)
}
