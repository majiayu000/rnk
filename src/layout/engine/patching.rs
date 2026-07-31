//! Applying a batch of reconciler patches to the Taffy tree.
use super::patch_error::{PatchError, PatchFailure, PatchKind};
use super::text_flow_bridge::NodeContext;
use super::{LayoutEngine, normalized_taffy_style};
use crate::core::{NodeKey, Props, VNode};
use crate::layout::TextFlowInput;
use crate::reconciler::{
    Patch, ScopedIdentityArena, ScopedNodeIdentity, compatibility_token_for_exact,
    insert_composite_projection, plan_initial_tree, resolve_child_identity,
};
use std::collections::{HashMap, HashSet};
use taffy::NodeId;
impl LayoutEngine {
    /// Apply a batch of patches, or none of them.
    ///
    /// Returns whether the tree changed. A rejected batch leaves this engine
    /// exactly as it was, so the caller can fall back to a full rebuild from
    /// the current tree; see [`try_apply_patches`](Self::try_apply_patches) for
    /// the reason it was rejected.
    ///
    /// # Panics
    ///
    /// Panics when canonical identity validation fails or a raw compatibility
    /// address is missing or ambiguous. Use
    /// [`try_apply_patches`](Self::try_apply_patches) to handle those failures
    /// explicitly.
    pub fn apply_patches(&mut self, patches: &[Patch]) -> bool {
        match self.try_apply_patches(patches) {
            Ok(changed) => changed,
            Err(
                error @ PatchError {
                    failure:
                        PatchFailure::IdentityRejected
                        | PatchFailure::UnknownNode
                        | PatchFailure::AmbiguousNode,
                    ..
                },
            ) => panic!("patch identity validation failed: {error}"),
            Err(_) => false,
        }
    }

    /// Apply a batch of patches transactionally.
    ///
    /// Every patch is applied to a staged copy; any failure drops that copy and
    /// leaves this engine untouched.
    ///
    /// # Errors
    ///
    /// Returns [`PatchError`] when identity preflight, a tree mutation,
    /// postcondition validation, or final layout fails.
    pub fn try_apply_patches(&mut self, patches: &[Patch]) -> Result<bool, PatchError> {
        if patches.is_empty() {
            return Ok(false);
        }

        let mut arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
        self.preflight_patch_identities(patches, &mut arena)?;
        let mut candidate = self.staged_clone();
        candidate.apply_patches_only(patches, &mut arena)?;
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

        candidate.committed_vnode = None;
        *self = candidate;
        Ok(true)
    }
    pub(super) fn apply_patches_only(
        &mut self,
        patches: &[Patch],
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), PatchError> {
        for patch in patches {
            match patch {
                Patch::Create { node, parent, .. } => {
                    self.create_vnode(node, *parent, arena)?;
                }
                Patch::Update { key, new_props, .. } => {
                    self.update_node_props(*key, new_props)?;
                }
                Patch::Remove { key } => {
                    self.remove_node(*key)?;
                }
                Patch::Replace { key, node, .. } => {
                    self.replace_node(*key, node, arena)?;
                }
                Patch::Reorder { parent, order } => {
                    self.reorder_children(*parent, order)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn check_batch_postconditions(&self, patches: &[Patch]) -> Result<(), PatchError> {
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
            let parent_scope = self
                .resolve_patch_scope(*parent, PatchKind::Reorder)
                .map_err(|_| fail())?;
            let &parent_id = self.vnode_map.get(&parent_scope).ok_or_else(fail)?;
            let actual = self.taffy.children(parent_id).map_err(|_| fail())?;
            let expected: Option<Vec<NodeId>> = order
                .iter()
                .map(|key| {
                    let identity = self
                        .resolve_child_legacy_scope(&parent_scope, *key)
                        .ok()??;
                    self.vnode_map.get(&identity).copied()
                })
                .collect();
            if expected.as_deref() != Some(actual.as_slice()) {
                return Err(fail());
            }
        }
        Ok(())
    }

    fn resolve_patch_scope(
        &self,
        key: NodeKey,
        kind: PatchKind,
    ) -> Result<ScopedNodeIdentity, PatchError> {
        if ScopedNodeIdentity::is_scoped_patch_address(key) {
            let matches: Vec<_> = self
                .vnode_legacy_keys
                .iter()
                .filter(|(identity, legacy_key)| {
                    identity.scoped_patch_address(**legacy_key).identity() == key.identity()
                })
                .map(|(identity, _)| identity.clone())
                .collect();
            return match matches.as_slice() {
                [identity] => Ok(identity.clone()),
                [] => Err(PatchError::new(kind, key, PatchFailure::UnknownNode)),
                _ => Err(PatchError::new(kind, key, PatchFailure::AmbiguousNode)),
            };
        }
        match self.resolve_legacy_scope(key) {
            Ok(Some(identity)) => Ok(identity),
            Ok(None) => Err(PatchError::new(kind, key, PatchFailure::UnknownNode)),
            Err(_) => Err(PatchError::new(kind, key, PatchFailure::AmbiguousNode)),
        }
    }

    fn preflight_patch_identities(
        &self,
        patches: &[Patch],
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), PatchError> {
        let mut planned_identities = HashSet::new();
        let mut planned_keyed_tokens = HashSet::new();
        let mut prospective_projections = HashMap::new();
        for (identity, legacy_key) in &self.vnode_legacy_keys {
            insert_composite_projection(&mut prospective_projections, identity, *legacy_key)
                .map_err(|_| {
                    PatchError::new(
                        PatchKind::Update,
                        batch_key(patches),
                        PatchFailure::IdentityRejected,
                    )
                })?;
        }
        for patch in patches {
            let (kind, node) = match patch {
                Patch::Create { node, .. } => (PatchKind::Create, node),
                Patch::Replace { node, .. } => (PatchKind::Replace, node),
                _ => continue,
            };
            plan_initial_tree(node)
                .map_err(|_| PatchError::new(kind, node.key, PatchFailure::IdentityRejected))?;
            let (parent_scope, resolved, excluded, excluded_nodes) = match patch {
                Patch::Create { parent, .. } => {
                    let parent_scope = self.resolve_patch_scope(*parent, kind)?;
                    let resolved = resolve_child_identity(
                        node,
                        node.key.index,
                        &parent_scope,
                        &compatibility_token_for_exact,
                        arena,
                    )
                    .map_err(|_| PatchError::new(kind, node.key, PatchFailure::IdentityRejected))?;
                    (parent_scope, resolved, None, None)
                }
                Patch::Replace { key, .. } => {
                    let old_identity = self.resolve_patch_scope(*key, kind)?;
                    let old_node =
                        self.vnode_map.get(&old_identity).copied().ok_or_else(|| {
                            PatchError::new(kind, *key, PatchFailure::UnknownNode)
                        })?;
                    let parent_scope = old_identity
                        .parent()
                        .cloned()
                        .ok_or_else(|| PatchError::new(kind, *key, PatchFailure::MissingParent))?;
                    let parent_node = self
                        .taffy
                        .parent(old_node)
                        .ok_or_else(|| PatchError::new(kind, *key, PatchFailure::MissingParent))?;
                    let actual_index = self
                        .taffy
                        .children(parent_node)
                        .map_err(|_| PatchError::new(kind, *key, PatchFailure::TreeRejected))?
                        .iter()
                        .position(|candidate| *candidate == old_node)
                        .ok_or_else(|| {
                            PatchError::new(kind, *key, PatchFailure::PostconditionViolated)
                        })?;
                    let resolved = resolve_child_identity(
                        node,
                        actual_index,
                        &parent_scope,
                        &compatibility_token_for_exact,
                        arena,
                    )
                    .map_err(|_| PatchError::new(kind, node.key, PatchFailure::IdentityRejected))?;
                    (
                        parent_scope,
                        resolved,
                        Some(old_identity),
                        Some(self.vnode_subtree_nodes(old_node)),
                    )
                }
                _ => unreachable!("only create and replace reach identity preflight"),
            };
            self.preflight_new_sibling_identity(
                &parent_scope,
                &resolved.scoped,
                resolved.legacy_key,
                excluded.as_ref(),
                kind,
            )?;
            if let Some(excluded_nodes) = excluded_nodes {
                for (identity, legacy_key) in &self.vnode_legacy_keys {
                    if self
                        .vnode_map
                        .get(identity)
                        .is_some_and(|node_id| excluded_nodes.contains(node_id))
                    {
                        prospective_projections.remove(&identity.composite_identity(*legacy_key));
                    }
                }
            }
            let mut subtree_identities = Vec::new();
            Self::collect_scoped_subtree_identities(
                node,
                resolved.scoped.clone(),
                resolved.legacy_key,
                kind,
                &mut subtree_identities,
                arena,
            )?;
            for (identity, legacy_key) in subtree_identities {
                insert_composite_projection(&mut prospective_projections, &identity, legacy_key)
                    .map_err(|_| {
                        PatchError::new(kind, legacy_key, PatchFailure::IdentityRejected)
                    })?;
            }
            let duplicate_planned_identity = !planned_identities.insert(resolved.scoped.clone());
            let duplicate_planned_token = resolved
                .legacy_key
                .user_key
                .is_some_and(|token| !planned_keyed_tokens.insert((parent_scope.clone(), token)));
            if duplicate_planned_identity || duplicate_planned_token {
                return Err(PatchError::new(
                    kind,
                    resolved.legacy_key,
                    PatchFailure::IdentityRejected,
                ));
            }
        }
        Ok(())
    }

    fn collect_scoped_subtree_identities(
        vnode: &VNode,
        identity: ScopedNodeIdentity,
        legacy_key: NodeKey,
        kind: PatchKind,
        identities: &mut Vec<(ScopedNodeIdentity, NodeKey)>,
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), PatchError> {
        identities.push((identity.clone(), legacy_key));
        for (index, child) in vnode.children.iter().enumerate() {
            let resolved = resolve_child_identity(
                child,
                index,
                &identity,
                &compatibility_token_for_exact,
                arena,
            )
            .map_err(|_| PatchError::new(kind, child.key, PatchFailure::IdentityRejected))?;
            Self::collect_scoped_subtree_identities(
                child,
                resolved.scoped,
                resolved.legacy_key,
                kind,
                identities,
                arena,
            )?;
        }
        Ok(())
    }

    fn preflight_new_sibling_identity(
        &self,
        parent_scope: &ScopedNodeIdentity,
        new_identity: &ScopedNodeIdentity,
        new_key: NodeKey,
        excluded: Option<&ScopedNodeIdentity>,
        kind: PatchKind,
    ) -> Result<(), PatchError> {
        let collides = self.vnode_legacy_keys.iter().any(|(identity, key)| {
            identity.parent() == Some(parent_scope)
                && excluded != Some(identity)
                && match new_key.user_key {
                    Some(token) => key.user_key == Some(token),
                    None => identity == new_identity,
                }
        });
        if collides {
            return Err(PatchError::new(
                kind,
                new_key,
                PatchFailure::IdentityRejected,
            ));
        }
        Ok(())
    }

    fn build_vnode_scoped(
        &mut self,
        vnode: &VNode,
        identity: ScopedNodeIdentity,
        legacy_key: NodeKey,
        kind: PatchKind,
        arena: &mut ScopedIdentityArena,
    ) -> Result<NodeId, PatchError> {
        let fail = |failure| PatchError::new(kind, legacy_key, failure);
        if self.vnode_map.contains_key(&identity) {
            return Err(fail(PatchFailure::AmbiguousNode));
        }

        let mut child_nodes = Vec::with_capacity(vnode.children.len());
        for (index, child) in vnode.children.iter().enumerate() {
            let resolved = resolve_child_identity(
                child,
                index,
                &identity,
                &compatibility_token_for_exact,
                arena,
            )
            .map_err(|_| fail(PatchFailure::IdentityRejected))?;
            child_nodes.push(self.build_vnode_scoped(
                child,
                resolved.scoped,
                resolved.legacy_key,
                kind,
                arena,
            )?);
        }

        let style = normalized_taffy_style(&vnode.props.style, vnode.is_text());
        let context = NodeContext::new(
            super::text_flow_bridge::input_from_vnode(vnode),
            &self.text_flow_policy,
        );
        let node_id = if vnode.is_text() {
            if !child_nodes.is_empty() {
                return Err(fail(PatchFailure::BuildFailed));
            }
            self.taffy
                .new_leaf_with_context(style, context)
                .map_err(|_| fail(PatchFailure::BuildFailed))?
        } else {
            let node_id = self
                .taffy
                .new_with_children(style, &child_nodes)
                .map_err(|_| fail(PatchFailure::BuildFailed))?;
            self.taffy
                .set_node_context(node_id, Some(context))
                .map_err(|_| fail(PatchFailure::BuildFailed))?;
            node_id
        };
        self.vnode_map.insert(identity.clone(), node_id);
        self.vnode_legacy_keys.insert(identity, legacy_key);
        Ok(node_id)
    }

    fn create_vnode(
        &mut self,
        vnode: &VNode,
        parent_key: NodeKey,
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Create, parent_key, failure);

        let parent_scope = self.resolve_patch_scope(parent_key, PatchKind::Create)?;
        let parent_node = *self
            .vnode_map
            .get(&parent_scope)
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;
        let resolved = resolve_child_identity(
            vnode,
            vnode.key.index,
            &parent_scope,
            &compatibility_token_for_exact,
            arena,
        )
        .map_err(|_| fail(PatchFailure::IdentityRejected))?;
        if self.vnode_map.contains_key(&resolved.scoped) {
            return Err(fail(PatchFailure::AmbiguousNode));
        }
        let new_node_id = self.build_vnode_scoped(
            vnode,
            resolved.scoped,
            resolved.legacy_key,
            PatchKind::Create,
            arena,
        )?;

        self.taffy
            .add_child(parent_node, new_node_id)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;

        Ok(())
    }

    fn update_node_props(&mut self, key: NodeKey, props: &Props) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Update, key, failure);

        let identity = self.resolve_patch_scope(key, PatchKind::Update)?;
        let &node_id = self
            .vnode_map
            .get(&identity)
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

    fn remove_node(&mut self, key: NodeKey) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Remove, key, failure);

        let identity = self.resolve_patch_scope(key, PatchKind::Remove)?;
        let &node_id = self
            .vnode_map
            .get(&identity)
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;

        let subtree = self.vnode_subtree_nodes(node_id);
        for subtree_node in &subtree {
            self.taffy
                .remove(*subtree_node)
                .map_err(|_| fail(PatchFailure::TreeRejected))?;
        }
        self.purge_vnode_subtree(&subtree);
        Ok(())
    }

    fn replace_node(
        &mut self,
        old_key: NodeKey,
        new_node: &VNode,
        arena: &mut ScopedIdentityArena,
    ) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Replace, old_key, failure);

        let old_identity = self.resolve_patch_scope(old_key, PatchKind::Replace)?;
        let &old_node_id = self
            .vnode_map
            .get(&old_identity)
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;
        let parent_id = self
            .taffy
            .parent(old_node_id)
            .ok_or_else(|| fail(PatchFailure::MissingParent))?;
        let parent_scope = old_identity
            .parent()
            .cloned()
            .ok_or_else(|| fail(PatchFailure::MissingParent))?;

        let children = self
            .taffy
            .children(parent_id)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;
        let index = children
            .iter()
            .position(|&id| id == old_node_id)
            .ok_or_else(|| fail(PatchFailure::PostconditionViolated))?;

        let subtree = self.vnode_subtree_nodes(old_node_id);
        for subtree_node in &subtree {
            self.taffy
                .remove(*subtree_node)
                .map_err(|_| fail(PatchFailure::TreeRejected))?;
        }
        self.purge_vnode_subtree(&subtree);

        let resolved = resolve_child_identity(
            new_node,
            index,
            &parent_scope,
            &compatibility_token_for_exact,
            arena,
        )
        .map_err(|_| fail(PatchFailure::IdentityRejected))?;
        let new_node_id = self.build_vnode_scoped(
            new_node,
            resolved.scoped,
            resolved.legacy_key,
            PatchKind::Replace,
            arena,
        )?;

        self.taffy
            .insert_child_at_index(parent_id, index, new_node_id)
            .map_err(|_| fail(PatchFailure::TreeRejected))?;

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
        self.vnode_legacy_keys
            .retain(|identity, _| self.vnode_map.contains_key(identity));
        self.node_map.retain(|_, node| !subtree.contains(node));
        self.element_keys
            .retain(|element_id, _| self.node_map.contains_key(element_id));
        self.element_scopes
            .retain(|element_id, _| self.node_map.contains_key(element_id));
        self.current_vnode_flows
            .retain(|key, _| self.vnode_map.contains_key(key));
        self.current_text_flows
            .retain(|element_id, _| self.node_map.contains_key(element_id));
    }

    fn reorder_children(
        &mut self,
        parent_key: NodeKey,
        order: &[NodeKey],
    ) -> Result<(), PatchError> {
        let fail = |failure| PatchError::new(PatchKind::Reorder, parent_key, failure);

        let parent_scope = self.resolve_patch_scope(parent_key, PatchKind::Reorder)?;
        let &parent_id = self
            .vnode_map
            .get(&parent_scope)
            .ok_or_else(|| fail(PatchFailure::UnknownNode))?;

        let mut children = Vec::with_capacity(order.len());
        for key in order {
            let identity = self
                .resolve_child_legacy_scope(&parent_scope, *key)
                .map_err(|_| fail(PatchFailure::AmbiguousNode))?
                .ok_or_else(|| fail(PatchFailure::UnknownNode))?;
            let &node_id = self
                .vnode_map
                .get(&identity)
                .ok_or_else(|| fail(PatchFailure::UnknownNode))?;
            children.push(node_id);
        }

        self.taffy
            .set_children(parent_id, &children)
            .map_err(|_| fail(PatchFailure::TreeRejected))
    }
}

fn batch_key(patches: &[Patch]) -> NodeKey {
    patches
        .first()
        .map(|patch| match patch {
            Patch::Create { parent, .. } | Patch::Reorder { parent, .. } => *parent,
            Patch::Update { key, .. } | Patch::Remove { key } | Patch::Replace { key, .. } => *key,
        })
        .unwrap_or_else(NodeKey::root)
}

#[cfg(test)]
pub(super) mod contract_tests {
    use super::super::LayoutEngine;
    use crate::components::Text;
    use crate::core::{Dimension, Element, TextWrap, VNode};
    use crate::layout::{IncrementalLayoutError, LayoutLookupError, TextFlowError};
    use crate::reconciler::ReconcilePlanError;

    fn fixed_width_parent(child: Element) -> Element {
        let mut parent = Element::box_element();
        parent.style.width = Dimension::Points(4.0);
        parent.add_child(child);
        parent
    }

    pub(crate) fn incremental_wrap_modes_refresh_context_bidirectionally() {
        for truncate_mode in [
            TextWrap::Truncate,
            TextWrap::TruncateStart,
            TextWrap::TruncateMiddle,
            TextWrap::TruncateEnd,
        ] {
            let mut engine = LayoutEngine::new();
            let initial_text = Text::new("abcdefgh")
                .key("wrap-context")
                .wrap(TextWrap::Wrap)
                .into_element();
            let initial_id = initial_text.id;
            let initial = fixed_width_parent(initial_text);
            let (wrapped, first_outcome) =
                engine.compute_element_incremental(&initial, None, 80, 10);
            assert!(!first_outcome.used_reconciler);
            let initial_layout = engine.get_layout(initial_id).expect("wrapped layout");
            assert_eq!((initial_layout.width, initial_layout.height), (4.0, 2.0));

            let truncated_text = Text::new("abcdefgh")
                .key("wrap-context")
                .wrap(truncate_mode)
                .into_element();
            let truncated_id = truncated_text.id;
            let truncated = fixed_width_parent(truncated_text);
            let (truncated_vnode, outcome) =
                engine.compute_element_incremental(&truncated, Some(&wrapped), 80, 10);
            assert!(outcome.used_reconciler);
            assert_eq!(outcome.patch_count, 1);
            assert!(!outcome.fallback_full_rebuild);
            let incremental = engine.get_layout(truncated_id).expect("truncated layout");
            let mut rebuilt = LayoutEngine::new();
            rebuilt.compute_element_incremental(&truncated, None, 80, 10);
            let full = rebuilt.get_layout(truncated_id).expect("rebuilt layout");
            assert_eq!(
                (incremental.width, incremental.height),
                (full.width, full.height)
            );
            assert_eq!((incremental.width, incremental.height), (4.0, 1.0));

            let wrapped_text = Text::new("abcdefgh")
                .key("wrap-context")
                .wrap(TextWrap::Wrap)
                .into_element();
            let wrapped_id = wrapped_text.id;
            let wrapped_again = fixed_width_parent(wrapped_text);
            let (_, outcome) =
                engine.compute_element_incremental(&wrapped_again, Some(&truncated_vnode), 80, 10);
            assert!(outcome.used_reconciler);
            assert_eq!(outcome.patch_count, 1);
            assert!(!outcome.fallback_full_rebuild);
            let incremental = engine.get_layout(wrapped_id).expect("wrapped layout");
            let mut rebuilt = LayoutEngine::new();
            rebuilt.compute_element_incremental(&wrapped_again, None, 80, 10);
            let full = rebuilt
                .get_layout(wrapped_id)
                .expect("rebuilt wrapped layout");
            assert_eq!(
                (incremental.width, incremental.height),
                (full.width, full.height)
            );
            assert_eq!((incremental.width, incremental.height), (4.0, 2.0));
        }
    }

    pub(crate) fn duplicate_sibling_key_fails_before_mutation() {
        let mut engine = LayoutEngine::new();
        let stable = Element::box_element();
        let (previous, _) = engine.compute_element_incremental(&stable, None, 20, 4);
        let before_root = engine.root_node;
        let before_count = engine.node_count();
        let mut invalid = Element::box_element();
        invalid.add_child(Element::box_element().with_key("duplicate"));
        invalid.add_child(Element::box_element().with_key("duplicate"));
        let failure = engine
            .try_compute_element_incremental_checked(&invalid, Some(&previous), 20, 4)
            .expect_err("duplicate target is rejected");
        assert!(matches!(
            failure,
            IncrementalLayoutError::Identity(ReconcilePlanError::DuplicateSiblingKey { .. })
        ));
        assert_eq!(engine.root_node, before_root);
        assert_eq!(engine.node_count(), before_count);
    }

    pub(crate) fn raw_legacy_lookup_reports_typed_ambiguity() {
        let tree = VNode::box_node().children([
            VNode::box_node()
                .with_key("left")
                .child(VNode::text("a").with_key("shared")),
            VNode::box_node()
                .with_key("right")
                .child(VNode::text("b").with_key("shared")),
        ]);
        let raw = tree.children[0].children[0].key;
        let mut engine = LayoutEngine::new();
        engine.compute_vnode(&tree, 20, 4);
        assert!(matches!(
            engine.try_get_vnode_layout(raw),
            Err(LayoutLookupError::AmbiguousLegacyNodeKey {
                scoped_match_count: 2,
                ..
            })
        ));
    }

    pub(crate) fn textflow_and_identity_causes_remain_distinct() {
        let identity = IncrementalLayoutError::from(ReconcilePlanError::PreviousTreeMismatch);
        let text = IncrementalLayoutError::from(TextFlowError::InvalidTabStop);
        assert!(matches!(identity, IncrementalLayoutError::Identity(_)));
        assert!(matches!(text, IncrementalLayoutError::TextFlow(_)));
    }

    pub(crate) fn checked_layout_accepts_public_box_text_component_roots() {
        struct Component;
        for root in [
            VNode::box_node(),
            VNode::text("root"),
            VNode::component::<Component>(),
        ] {
            let mut engine = LayoutEngine::new();
            engine.compute_vnode(&root, 20, 4);
            assert!(engine.get_vnode_layout(root.key).is_some());
        }
    }
}
